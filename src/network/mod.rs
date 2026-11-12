//! Network module for P2P synchronization
//!
//! Based on Section 15-17 of the specification.
//!
//! Handles:
//! - P2P peer discovery and connection
//! - Hook-based sync triggers for methodologies, blueprints, and findings
//! - Network status monitoring
//!
//! SYNC BEHAVIOR (operator rulings 2026-11-10: NOTIFY AND GATE, through the MARKETPLACE, never auto-sync):
//! - PUBLISH, never send content: a methodology, jurisdiction rule set, pipeline or tool is published as a small manifest (type, name, body hash,
//!   keywords) to THIS instance's marketplace listing and ONE notification is pushed on the shared marketplace topic to every connected peer —
//!   never a message per peer. The body never travels in a notification and nothing is applied automatically.
//! - The marketplace flows among CONNECTED peers (it is a catalog). "Paired" in this module is the operator-approved peer allow-list
//!   (`paired_peers.json`) — NOT device pairing — and it still gates every OTHER message kind (default-deny unchanged).
//! - THE DOOR: every inbound marketplace message is validated against an allow-list (`sanitize_marketplace_message`) — only methodology,
//!   jurisdiction_rule_set, pipeline and tool in strict shapes are accepted; everything else is refused and logged.
//! - A receiving instance lists what it accepted as OFFERS; an ADVISORY review (mechanical identical-content check, then a zero-shot
//!   recommendation) suggests download / ignore / review, and the OPERATOR's decision is what acts. Approved methodologies are fetched in chunks,
//!   hash re-verified and imported through the normal create path; jurisdiction files are only staged; tools and pipelines are approval-only.
//! - Blueprints: PRIVATE — never published, never synced.
//! - New container findings: no longer broadcast. Pipeline results: ON-DEMAND only (explicit share request).
//! - User data: NEVER sync (local only).
//! - Identity of an item is NAME + BODY HASH (`body_hash`), never a container id (ids are per store and collide).

use crate::config::NetworkConfig;
use crate::types::{ContainerID, OzoneError, OzoneResult};
use libp2p::{gossipsub, identify, mdns, ping, PeerId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};

#[allow(unused_imports)]
use tracing;

/// Sync priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncPriority {
    /// Immediate broadcast (methodologies, blueprints)
    High,
    /// Batch sync on next cycle (new findings)
    Normal,
    /// On-demand only (pipeline results)
    Low,
    /// Never sync (user data)
    Never,
}

/// Sync item to be queued
#[derive(Debug, Clone)]
pub struct SyncItem {
    pub container_id: ContainerID,
    pub item_type: SyncItemType,
    pub priority: SyncPriority,
    pub data: Vec<u8>,
    pub timestamp: u64,
}

/// Type of item being synced
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncItemType {
    Methodology,
    /// Kept for API compatibility only: blueprints are PRIVATE and are never announced (see `NetworkManager::on_items_announced`, which skips them).
    Blueprint,
    Container,
    PipelineResult,
    JurisdictionRuleSet,
    /// A pipeline (a built unit) — announced as a manifest; installing it is a reviewed, explicit step.
    Pipeline,
    /// A tool / MCP provider — announced as a manifest; installing it is a reviewed, explicit step (never auto-run).
    Tool,
}

impl SyncItemType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SyncItemType::Methodology => "methodology",
            SyncItemType::Blueprint => "blueprint",
            SyncItemType::Container => "container",
            SyncItemType::PipelineResult => "pipeline_result",
            SyncItemType::JurisdictionRuleSet => "jurisdiction_rule_set",
            SyncItemType::Pipeline => "pipeline",
            SyncItemType::Tool => "tool",
        }
    }
}

/// Stable identity hash of an item's body (FNV-1a 64). The SAME function is used by the announcing side and by the receiving side's local
/// index, so "same name + same hash" means "already have it", and "same name, different hash" is a CONFLICT to review — never an overwrite.
pub fn body_hash(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

/// The ONE shared topic every instance subscribes to. A marketplace publication is a SINGLE publish here — never a message per peer. The push
/// carries NO names or keywords (a shared topic reaches every subscriber); a connected peer that wants the details pulls the listing.
pub const MARKETPLACE_TOPIC: &str = "ozone/marketplace";

/// The message kinds of the marketplace exchange. They flow among CONNECTED peers (the marketplace is a catalog, not a trust relationship);
/// every other message kind still requires an operator-approved ("paired") peer — default-deny is unchanged for everything else. Note: "paired"
/// here is this module's approved-peer allow-list (`paired_peers.json`), NOT device pairing.
pub const MARKETPLACE_KINDS: [&str; 6] = ["marketplace_notify", "catalog_request", "catalog", "fetch_request", "fetch_chunk", "fetch_error"];

/// The `kind` of a message if it is a marketplace message.
pub fn marketplace_kind(data: &[u8]) -> Option<String> {
    let v = serde_json::from_slice::<serde_json::Value>(data).ok()?;
    let k = v.get("kind")?.as_str()?;
    MARKETPLACE_KINDS.contains(&k).then(|| k.to_string())
}

/// THE DOOR (operator 2026-11-10: "we don't accept anything not allowed"). The ONLY item kinds that may be offered, fetched or imported;
/// everything else — blueprints, user data, events, chat, any unknown kind — is refused at the door and never reaches the inbox.
pub const ALLOWED_ITEM_TYPES: [&str; 4] = ["methodology", "jurisdiction_rule_set", "pipeline", "tool"];

/// NO fixed size guards on peer manifests (operator 2026-11-11: no fixed limits). The name / keyword-count / chunk-size limits that stood here
/// (512 chars, 500 keywords, 2 x the chunk size) are gone: the transport already bounds one message (gossipsub's maximum message size), and
/// everything else the door checks is SHAPE, not size — an allowed item type, a name with no control characters, a 16-hex body hash,
/// alphanumeric listing versions, scope markers stripped from keyword lists.

fn is_hex16(h: &str) -> bool {
    h.len() == 16 && h.chars().all(|c| c.is_ascii_hexdigit())
}

fn clean_name(v: &serde_json::Value) -> Result<String, String> {
    let n = v.as_str().map(str::trim).unwrap_or("");
    if n.is_empty() {
        return Err("the item has no name".to_string());
    }
    if n.chars().any(|c| c.is_control()) {
        return Err("the item name is not acceptable (it contains control characters)".to_string());
    }
    Ok(n.to_string())
}

fn clean_item_type(v: &serde_json::Value) -> Result<String, String> {
    let t = v.as_str().unwrap_or("");
    if ALLOWED_ITEM_TYPES.contains(&t) {
        Ok(t.to_string())
    } else {
        Err(format!("item type '{t}' is not allowed"))
    }
}

/// Keywords from a peer: strings only, trimmed, scope MARKERS dropped (they can carry another instance's project / workspace ids), no empties.
fn clean_keywords(v: &serde_json::Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|k| k.as_str().map(str::trim))
                .filter(|k| !k.is_empty() && !k.chars().any(|c| c.is_control()) && !crate::shared_contracts::keyword_model::is_marker(k))
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

/// Validate + SANITIZE one manifest from a peer: allowed item type, a clean name, a 16-hex body hash (or none), clean keywords. Unknown fields
/// are dropped. Returns the cleaned manifest or the reason it is refused.
pub fn sanitize_manifest(m: &serde_json::Value) -> Result<serde_json::Value, String> {
    let item_type = clean_item_type(&m["item_type"])?;
    let name = clean_name(&m["name"])?;
    let hash = m["body_hash"].as_str().unwrap_or("");
    if !hash.is_empty() && !is_hex16(hash) {
        return Err("the body hash is not a 16-digit hex value".to_string());
    }
    Ok(serde_json::json!({
        "kind": "announce",
        "item_type": item_type,
        "name": name,
        "body_hash": hash,
        "keywords": clean_keywords(&m["keywords"]),
        "source_container_id": m["source_container_id"].as_u64(),
        "announced_at": m["announced_at"].as_u64(),
    }))
}

/// Validate + sanitize ONE inbound marketplace message. Only the kinds in `MARKETPLACE_KINDS` with well-formed, allowed content pass; the result
/// is a freshly built message (unknown fields dropped), never the peer's bytes. Err = the reason it is refused.
pub fn sanitize_marketplace_message(data: &[u8]) -> Result<serde_json::Value, String> {
    use serde_json::json;
    let v: serde_json::Value = serde_json::from_slice(data).map_err(|_| "not JSON".to_string())?;
    let kind = v["kind"].as_str().unwrap_or("");
    if !MARKETPLACE_KINDS.contains(&kind) {
        return Err(format!("message kind '{kind}' is not a marketplace kind"));
    }
    let version = |x: &serde_json::Value| x.as_str().filter(|s| s.len() <= 64 && s.chars().all(|c| c.is_ascii_alphanumeric())).map(String::from);
    match kind {
        "marketplace_notify" => {
            let listing_version = version(&v["listing_version"]).ok_or("the listing version is not acceptable")?;
            let item_type = v["item_type"].as_str().filter(|t| ALLOWED_ITEM_TYPES.contains(t)).map(String::from);
            Ok(json!({"kind": kind, "listing_version": listing_version, "count": v["count"].as_u64().unwrap_or(0), "item_type": item_type}))
        }
        "catalog_request" => Ok(json!({"kind": kind, "listing_version": version(&v["listing_version"])})),
        "catalog" => {
            let raw = v["items"].as_array().ok_or("a catalog has no items list")?;
            let items: Vec<serde_json::Value> = raw.iter().filter_map(|m| sanitize_manifest(m).ok()).collect();
            if items.is_empty() && !raw.is_empty() {
                return Err("a catalog with no acceptable item".to_string());
            }
            Ok(json!({"kind": kind, "items": items}))
        }
        "fetch_request" => {
            let hash = v["body_hash"].as_str().unwrap_or("");
            if !is_hex16(hash) {
                return Err("a fetch request needs a 16-digit hex body hash".to_string());
            }
            Ok(json!({"kind": kind, "item_type": clean_item_type(&v["item_type"])?, "name": clean_name(&v["name"])?, "body_hash": hash}))
        }
        "fetch_chunk" => {
            let hash = v["body_hash"].as_str().unwrap_or("");
            let (seq, total) = (v["seq"].as_u64(), v["total"].as_u64());
            let (Some(seq), Some(total)) = (seq, total) else { return Err("a chunk needs seq and total".to_string()) };
            if !is_hex16(hash) || total == 0 || seq >= total {
                return Err("a chunk has an invalid hash, total or sequence".to_string());
            }
            let data = v["data"].as_str().ok_or("a chunk has no data")?;
            Ok(json!({
                "kind": kind, "item_type": clean_item_type(&v["item_type"])?, "name": clean_name(&v["name"])?, "body_hash": hash,
                "seq": seq, "total": total, "data": data,
                "keywords": if seq == 0 { json!(clean_keywords(&v["keywords"])) } else { serde_json::Value::Null },
            }))
        }
        _ => {
            // fetch_error
            let hash = v["body_hash"].as_str().unwrap_or("");
            if !is_hex16(hash) {
                return Err("an error reply needs a 16-digit hex body hash".to_string());
            }
            let reason: String = v["reason"].as_str().unwrap_or("the peer could not serve it").chars().filter(|c| !c.is_control()).take(300).collect();
            Ok(json!({"kind": kind, "item_type": clean_item_type(&v["item_type"])?, "name": clean_name(&v["name"])?, "body_hash": hash, "reason": reason}))
        }
    }
}

/// Append one VALIDATED inbound marketplace message to `peer_marketplace_inbox.jsonl` (bookkeeping; nothing is applied). Durable: a failed append is retried in order until it succeeds
/// (src/durable_write.rs) — an accepted announcement is never lost to a transient disk error.
fn append_marketplace_inbox(path: &std::path::Path, from: &PeerId, clean: &serde_json::Value) {
    let line = serde_json::json!({"received_at": now(), "from_peer": from.to_string(), "envelope": clean}).to_string();
    crate::durable_write::append(path, line);
}

/// Record a REFUSED inbound marketplace message (who, why — never the content) in `peer_marketplace_rejected.jsonl`, so refusals are visible.
fn append_marketplace_rejected(path: &std::path::Path, from: &PeerId, reason: &str) {
    let line = serde_json::json!({"at": now(), "from_peer": from.to_string(), "reason": reason}).to_string();
    crate::durable_write::append(path, line);
}

/// One item handed to [`NetworkManager::on_items_announced`]: the manifest bytes (see [`announce_envelope`]) and where the body lives locally
/// (never sent).
pub struct AnnounceItem {
    pub container_id: ContainerID,
    pub item_type: SyncItemType,
    pub manifest: Vec<u8>,
    pub body_path: Option<String>,
}

/// A tool this system CREATED because a capability need was identified (`orchestrator/tool_creation.rs`) — a FINDING, published like a created
/// methodology (operator 2026-11-10: "we push on findings, not offers … that is automatic"). It is NOT a container-created event and NOT an
/// McpEntity (those are graph nodes built from tool RESULTS — user data — and are never announced; see `amt_loop::spawn_network_hook_bridge`).
/// `body_path` = the one-file body (`created_tools/<name>/tool.json`) a later reviewed fetch is answered from; `body_hash` = its `body_hash`.
#[derive(Debug, Clone)]
pub struct CreatedToolFinding {
    pub name: String,
    pub body_hash: String,
    pub keywords: Vec<String>,
    pub body_path: String,
}

impl CreatedToolFinding {
    /// The manifest + local body location handed to [`NetworkManager::on_items_announced`] (container id 0: a created tool has no container;
    /// a fetch is answered from the type|name|hash body index, never from the id).
    pub fn into_announce_item(self) -> AnnounceItem {
        let manifest = announce_envelope(&SyncItemType::Tool, &self.name, &self.body_hash, &self.keywords, 0);
        AnnounceItem { container_id: 0, item_type: SyncItemType::Tool, manifest, body_path: Some(self.body_path) }
    }
}

/// The created-tool channel: UNBOUNDED, single consumer (the marketplace bridge). A bounded broadcast buffer used to sit here and a lagging
/// bridge would drop findings; an unbounded queue never loses one. Findings are queued ONLY while a bridge holds the receiver (marketplace
/// publishing on) — with none attached, `publish_created_tool` is a no-op, exactly as before: creation never depends on publication.
struct CreatedToolChannel {
    tx: tokio::sync::mpsc::UnboundedSender<std::sync::Arc<CreatedToolFinding>>,
    rx: std::sync::Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<std::sync::Arc<CreatedToolFinding>>>>,
    attached: std::sync::atomic::AtomicBool,
}

fn created_tools_chan() -> &'static CreatedToolChannel {
    static CH: std::sync::OnceLock<CreatedToolChannel> = std::sync::OnceLock::new();
    CH.get_or_init(|| {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        CreatedToolChannel { tx, rx: std::sync::Mutex::new(Some(rx)), attached: std::sync::atomic::AtomicBool::new(false) }
    })
}

/// Publish a created-tool finding to whoever runs the marketplace bridge. A no-op when no bridge is attached (marketplace publishing off).
pub fn publish_created_tool(finding: CreatedToolFinding) {
    let ch = created_tools_chan();
    if ch.attached.load(std::sync::atomic::Ordering::SeqCst) {
        let _ = ch.tx.send(std::sync::Arc::new(finding));
    }
}

/// The bridge's receiver of created-tool findings. There is ONE consumer: the first caller gets the live receiver, a later caller gets `None`.
pub fn subscribe_created_tools() -> Option<tokio::sync::mpsc::UnboundedReceiver<std::sync::Arc<CreatedToolFinding>>> {
    let ch = created_tools_chan();
    let rx = ch.rx.lock().ok().and_then(|mut g| g.take());
    if rx.is_some() {
        ch.attached.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    rx
}

/// The announcement envelope: a MANIFEST only (no body). `source_container_id` is an opaque handle on the announcing store, kept so a later
/// reviewed fetch can ask for exactly this item; it is never used as an identity.
pub fn announce_envelope(item_type: &SyncItemType, name: &str, hash: &str, keywords: &[String], source_container_id: ContainerID) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "kind": "announce",
        "item_type": item_type.as_str(),
        "name": name,
        "body_hash": hash,
        "keywords": keywords,
        "source_container_id": source_container_id,
        "announced_at": chrono::Utc::now().timestamp(),
    }))
    .unwrap_or_default()
}

/// Hook function type
pub type HookFn = Arc<dyn Fn(SyncItem) -> OzoneResult<()> + Send + Sync>;

/// Network manager for P2P operations
pub struct NetworkManager {
    /// Sync queue for high priority items (immediate)
    high_priority_queue: Arc<RwLock<Vec<SyncItem>>>,

    /// Sync queue for normal priority items (batched)
    normal_queue: Arc<RwLock<Vec<SyncItem>>>,

    /// Registered hooks
    hooks: Arc<RwLock<HashMap<String, Vec<HookFn>>>>,

    /// Channel for sync notifications
    sync_sender: mpsc::Sender<SyncItem>,
    sync_receiver: Arc<RwLock<mpsc::Receiver<SyncItem>>>,

    /// libp2p swarm
    swarm: Arc<Mutex<libp2p::Swarm<OzoneBehaviour>>>,

    /// Network configuration
    config: NetworkConfig,

    /// Known peers by libp2p PeerId. NOT actually persistent despite the
    /// name (pre-existing doc comment was wrong, found 2026-10-20 while
    /// fixing the real P2P connectivity bugs this session) — purely
    /// in-memory, wiped on every restart. Real connection/session
    /// bookkeeping only; trust is tracked separately in `paired_peers`
    /// below, which genuinely is persisted.
    known_peers: Arc<RwLock<HashMap<PeerId, PeerInfo>>>,

    /// Explicitly operator-approved ("paired") peer ids — PeerId -> unix
    /// seconds when approved. 2026-10-20: real trust gate, added because
    /// `connect_peer` succeeding never implied any trust — ANY peer that
    /// completed a libp2p handshake was treated identically to an
    /// explicitly-approved one. This is the durable source of truth
    /// (`<data_dir>/paired_peers.json`); `known_peers` above is current
    /// connection state only and is NOT consulted for trust decisions.
    paired_peers: Arc<RwLock<HashMap<PeerId, u64>>>,

    /// Where `paired_peers` is persisted.
    paired_peers_path: std::path::PathBuf,

    /// Connection attempt tracking
    connection_attempts: HashMap<String, std::time::Instant>,

    /// Network statistics
    stats: Arc<Mutex<NetworkStats>>,

    /// Local libp2p peer ID
    local_peer_id: PeerId,
}

/// Peer information
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub peer_id: String,
    pub address: String,
    pub addresses: Vec<String>,
    pub last_seen: u64,
    pub latency_ms: u32,
    pub reputation: f32,
    pub contribution_count: u64,
    pub capabilities: Vec<String>,
}

/// Network statistics
#[derive(Debug, Clone, Default)]
pub struct NetworkStats {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

/// Get current unix timestamp
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Load the durable paired-peers store. Missing/unreadable/corrupt file is
/// treated as "no peers approved yet", not an error — a fresh install or a
/// deliberately-cleared file are both legitimate "start from zero trust"
/// states, consistent with this gate's whole purpose (default-deny).
fn load_paired_peers(path: &std::path::Path) -> HashMap<PeerId, u64> {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return HashMap::new();
    };
    let Ok(entries) = serde_json::from_str::<HashMap<String, u64>>(&raw) else {
        tracing::warn!("P2P: paired_peers.json at {:?} is unreadable/corrupt — starting with zero approved peers (default-deny)", path);
        return HashMap::new();
    };
    entries
        .into_iter()
        .filter_map(|(id_str, ts)| {
            use std::str::FromStr;
            match PeerId::from_str(&id_str) {
                Ok(pid) => Some((pid, ts)),
                Err(_) => {
                    tracing::warn!("P2P: ignoring unparseable peer id in paired_peers.json: {}", id_str);
                    None
                }
            }
        })
        .collect()
}

/// Append one received sync envelope from a PAIRED peer to
/// `peer_sync_candidates.jsonl` (guide §7.3 first version: bookkeeping only —
/// no container is created, no model is called). Durable: a failed append is
/// retried in order until it succeeds (src/durable_write.rs) off the swarm
/// event loop, so disk trouble can neither take the loop down nor lose the row.
fn append_peer_sync_candidate(path: &std::path::Path, from: &PeerId, data: &[u8]) {
    let envelope = serde_json::from_slice::<serde_json::Value>(data)
        .unwrap_or_else(|_| serde_json::Value::String(String::from_utf8_lossy(data).to_string()));
    let line = serde_json::json!({
        "received_at": now(),
        "from_peer": from.to_string(),
        "envelope": envelope,
    })
    .to_string();
    crate::durable_write::append(path, line);
}

/// Persist the paired-peers store (the operator-approved allow-list). Durable (src/durable_write.rs): one synchronous attempt; a
/// failure is retried until it succeeds, so an approval or revocation always reaches disk. The in-memory state (the source of truth
/// for the running process) updates immediately, and the operator's action is not failed by a transient disk error. Whole-file
/// replace: the newest allow-list supersedes an older queued one.
fn save_paired_peers(path: &std::path::Path, peers: &HashMap<PeerId, u64>) {
    let entries: HashMap<String, u64> = peers.iter().map(|(pid, ts)| (pid.to_string(), *ts)).collect();
    match serde_json::to_string_pretty(&entries) {
        Ok(json) => crate::durable_write::replace(path, json.into_bytes()),
        Err(e) => tracing::error!("P2P: failed to serialize paired_peers: {}", e),
    }
}

/// Combined network behaviour
#[derive(libp2p::swarm::NetworkBehaviour)]
struct OzoneBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
    ping: ping::Behaviour,
    identify: identify::Behaviour,
}

impl NetworkManager {
    pub async fn new(config: NetworkConfig, data_dir: std::path::PathBuf) -> OzoneResult<Self> {
        use libp2p::{
            gossipsub, identify, identity, mdns, noise, ping, SwarmBuilder, tcp, yamux,
        };

        let paired_peers_path = data_dir.join("paired_peers.json");
        let paired_peers = load_paired_peers(&paired_peers_path);

        // LOAD or generate-and-persist the node identity. The comment here used to say
        // "generate or load" but the code only ever generated, so the peer id changed on
        // EVERY restart and paired_peers.json (keyed by peer id) went stale — pairings could
        // not survive a restart. The key now lives at <data_dir>/p2p_identity.key (0600).
        let key_path = data_dir.join("p2p_identity.key");
        let local_key = match std::fs::read(&key_path)
            .ok()
            .and_then(|b| identity::Keypair::from_protobuf_encoding(&b).ok())
        {
            Some(k) => {
                tracing::info!("P2P: loaded persisted node identity from {:?}", key_path);
                k
            }
            None => {
                let k = identity::Keypair::generate_ed25519();
                match k.to_protobuf_encoding() {
                    Ok(bytes) => {
                        if let Some(dir) = key_path.parent() {
                            let _ = std::fs::create_dir_all(dir);
                        }
                        let write = std::fs::write(&key_path, &bytes);
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            let _ = std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600));
                        }
                        match write {
                            Ok(()) => tracing::info!("P2P: generated and persisted a new node identity at {:?}", key_path),
                            Err(e) => tracing::error!("P2P: could not persist node identity at {:?}: {} — peer id will change on restart", key_path, e),
                        }
                    }
                    Err(e) => tracing::error!("P2P: could not encode node identity: {} — peer id will change on restart", e),
                }
                k
            }
        };
        let local_peer_id = PeerId::from(local_key.public());

        // Publish the REAL peer id where the p2p sync pipeline reads it (<data_dir>/p2p/peer_id) — that pipeline used to invent a second,
        // fake "12D3KooW<clock nanoseconds>" identity (doctrine audit H1, 2026-11-04).
        {
            let p2p_dir = data_dir.join("p2p");
            // Durable (creates the directory itself; retried until written): the p2p sync pipeline reads this file.
            crate::durable_write::replace(p2p_dir.join("peer_id"), local_peer_id.to_string().into_bytes());
        }

        log::info!("Local peer ID: {}", local_peer_id);

        // Configure gossipsub
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(std::time::Duration::from_secs(1))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .max_transmit_size(GOSSIPSUB_MAX_TRANSMIT_BYTES)
            .build()
            .map_err(|e| OzoneError::NetworkError(format!("Gossipsub config error: {}", e)))?;

        let gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(local_key.clone()),
            gossipsub_config,
        )
        .map_err(|e| OzoneError::NetworkError(format!("Gossipsub init error: {}", e)))?;

        // Configure mDNS for local peer discovery
        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)
            .map_err(|e| OzoneError::NetworkError(format!("mDNS init error: {}", e)))?;

        // Build the swarm
        let swarm = SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )
            .map_err(|e| OzoneError::NetworkError(format!("TCP config error: {}", e)))?
            .with_behaviour(|_key| {
                Ok(OzoneBehaviour {
                    gossipsub,
                    mdns,
                    ping: ping::Behaviour::default(),
                    identify: identify::Behaviour::new(identify::Config::new(
                        "/ozone/1.0.0".into(),
                        _key.public(),
                    )),
                })
            })
            .map_err(|e| OzoneError::NetworkError(format!("Behaviour error: {}", e)))?
            // 2026-10-20 — found live, immediately after fixing the Swarm
            // event-loop gap above: libp2p's own default
            // `idle_connection_timeout` is 0 (confirmed in its own doc
            // comment, libp2p-swarm's `Config::with_idle_connection_timeout`),
            // meaning a connection with no actively-busy substream at a
            // given instant is torn down right away. First fix tried 120s,
            // reasoning `ping`'s own periodic keepalive traffic would reset
            // the idle timer — confirmed WRONG by direct observation: a
            // real local<->EU-VPS connection ran for exactly 120s, to the
            // second, then closed with `KeepAliveTimeout` regardless of
            // ping's ongoing traffic. In this libp2p version
            // `idle_connection_timeout` is the sole, non-overridable
            // authority — no behaviour's own activity resets it. Since the
            // operator explicitly wants to leave two real instances
            // connected (not just briefly testing), set to a full day
            // rather than re-guessing another short value that would just
            // fail the same way for a longer-but-still-wrong reason. A
            // peer that's genuinely gone is still detected independent of
            // this (TCP-level close / `ConnectionClosed` fires on a real
            // disconnect) — this setting only controls how long an IDLE
            // but still-live connection is kept.
            .with_swarm_config(|cfg| {
                cfg.with_idle_connection_timeout(std::time::Duration::from_secs(86400))
            })
            .build();

        let (sync_sender, sync_receiver) = mpsc::channel(1000);

        Ok(Self {
            high_priority_queue: Arc::new(RwLock::new(Vec::new())),
            normal_queue: Arc::new(RwLock::new(Vec::new())),
            hooks: Arc::new(RwLock::new(HashMap::new())),
            sync_sender,
            sync_receiver: Arc::new(RwLock::new(sync_receiver)),
            swarm: Arc::new(Mutex::new(swarm)),
            config,
            known_peers: Arc::new(RwLock::new(HashMap::new())),
            paired_peers: Arc::new(RwLock::new(paired_peers)),
            paired_peers_path,
            connection_attempts: HashMap::new(),
            stats: Arc::new(Mutex::new(NetworkStats::default())),
            local_peer_id,
        })
    }

    /// Initialize P2P network
    pub async fn initialize(&mut self) -> OzoneResult<()> {
        if !self.config.enable_p2p {
            tracing::info!("P2P networking disabled");
            return Ok(());
        }

        tracing::info!("Network initialized with peer ID: {}", self.local_peer_id);

        // Bind the libp2p transport so this instance can actually ACCEPT
        // inbound dials (2026-10-20, found live: `swarm.listen_on(...)` never
        // existed anywhere in this file — confirmed both this host's and a
        // paired EU VPS instance's real listening sockets (`ss -tlnp`) had
        // nothing on `p2p_port` despite both configs setting it to 9090.
        // Every `connect_peer` dial from either side was therefore always
        // going to fail (connection refused), regardless of firewall state
        // or whether a peer address was even correct — `peer_count` could
        // never become nonzero. Binds loopback-only, matching this
        // codebase's existing convention for the gRPC/HTTP server
        // (`[grpc] address = "127.0.0.1"` in config.toml) — real cross-host
        // reachability is meant to go through an SSH tunnel to this port,
        // the same pattern already used to pair a UI device to the HTTP
        // API (`ssh -L <port>:127.0.0.1:<port>`). Does not change firewall
        // or expose anything new to the public internet.
        {
            use libp2p::Multiaddr;
            use std::str::FromStr;
            let listen_addr_str = format!("/ip4/127.0.0.1/tcp/{}", self.config.p2p_port);
            match Multiaddr::from_str(&listen_addr_str) {
                Ok(listen_addr) => {
                    let mut swarm = self.swarm.lock().await;
                    match swarm.listen_on(listen_addr) {
                        Ok(_) => tracing::info!("P2P swarm listening on {}", listen_addr_str),
                        Err(e) => tracing::error!(
                            "P2P swarm failed to listen on {}: {} (peers will never be able to dial in)",
                            listen_addr_str, e
                        ),
                    }
                }
                Err(e) => tracing::error!("Invalid P2P listen multiaddr {}: {}", listen_addr_str, e),
            }
        }

        // Subscribe to our own direct-message topic (2026-10-20, found live
        // while verifying a real end-to-end sync test after the two fixes
        // below this one): `send_to_peer` publishes to gossipsub topic
        // `"direct/<recipient peer id>"`, but nothing anywhere in this file
        // ever called `gossipsub.subscribe(...)` for ANY topic — in gossipsub
        // a peer only receives messages on topics it has subscribed to, so
        // every `send_to_peer` call was always going to reach zero
        // subscribers even once the connection itself was real. Subscribing
        // to our own address is what makes an incoming direct message to us
        // actually deliverable at all.
        {
            let topic = libp2p::gossipsub::IdentTopic::new(format!("direct/{}", self.local_peer_id));
            let mut swarm = self.swarm.lock().await;
            match swarm.behaviour_mut().gossipsub.subscribe(&topic) {
                Ok(_) => tracing::info!("P2P: subscribed to own direct topic direct/{}", self.local_peer_id),
                Err(e) => tracing::error!("P2P: failed to subscribe to own direct topic: {}", e),
            }
        }

        // the shared marketplace topic: one publish reaches every connected instance (operator 2026-11-10: announcements go through the
        // marketplace, not to each user one by one)
        {
            let topic = libp2p::gossipsub::IdentTopic::new(MARKETPLACE_TOPIC);
            let mut swarm = self.swarm.lock().await;
            match swarm.behaviour_mut().gossipsub.subscribe(&topic) {
                Ok(_) => tracing::info!("P2P: subscribed to the shared marketplace topic {}", MARKETPLACE_TOPIC),
                Err(e) => tracing::error!("P2P: failed to subscribe to the marketplace topic: {}", e),
            }
        }

        // Drive the libp2p Swarm's own event loop (2026-10-20, found live
        // right after the listen_on fix above: `Swarm` is a `Stream` — none
        // of its pending work (dials, inbound handshakes, protocol
        // negotiation) ever progresses unless something polls it, and
        // nothing anywhere in this codebase did. Confirmed empirically: a
        // real connect_peer dial opened a raw TCP socket (visible in `ss`)
        // that then tore down within seconds (FIN-WAIT-2/CLOSE-WAIT)
        // because libp2p's own noise/yamux handshake had nowhere to make
        // progress — so `swarm.connected_peers()`/`is_connected()` could
        // never reflect a real connection, independent of the listen_on
        // gap. This was the deeper, more fundamental half of that finding.
        //
        // Spawned as its own task holding clones of only the two Arcs it
        // needs (not `self`) so it runs for the whole process lifetime.
        // Locking: `swarm` is also used by one-shot callers (connect_peer/
        // send_to_peer/broadcast) via their own brief `.lock().await` — an
        // unbounded `select_next_some().await` while holding the lock here
        // would starve them whenever the swarm is otherwise idle, so each
        // iteration polls with a short bounded timeout and releases the
        // lock in between.
        {
            let swarm = self.swarm.clone();
            let known_peers = self.known_peers.clone();
            let paired_peers = self.paired_peers.clone();
            // Bookkeeping-only receive ingest target (guide §7.3, first
            // version): sits beside paired_peers.json in the data dir.
            let candidates_path = self
                .paired_peers_path
                .parent()
                .map(|d| d.join("peer_sync_candidates.jsonl"))
                .unwrap_or_else(|| std::path::PathBuf::from("peer_sync_candidates.jsonl"));
            let inbox_path = self.paired_peers_path.parent().map(|d| d.join("peer_marketplace_inbox.jsonl")).unwrap_or_else(|| std::path::PathBuf::from("peer_marketplace_inbox.jsonl"));
            let rejected_path = self.paired_peers_path.parent().map(|d| d.join("peer_marketplace_rejected.jsonl")).unwrap_or_else(|| std::path::PathBuf::from("peer_marketplace_rejected.jsonl"));
            tokio::spawn(async move {
                use futures::StreamExt;
                loop {
                    let event = {
                        let mut s = swarm.lock().await;
                        match tokio::time::timeout(
                            std::time::Duration::from_millis(250),
                            s.select_next_some(),
                        )
                        .await
                        {
                            Ok(ev) => Some(ev),
                            Err(_) => None,
                        }
                    };
                    let Some(event) = event else { continue };
                    match event {
                        libp2p::swarm::SwarmEvent::ConnectionEstablished {
                            peer_id, endpoint, ..
                        } => {
                            let addr = endpoint.get_remote_address().to_string();
                            tracing::info!(
                                "P2P: connection established with peer {} ({})",
                                peer_id, addr
                            );
                            known_peers.write().await.insert(
                                peer_id,
                                PeerInfo {
                                    peer_id: peer_id.to_string(),
                                    address: addr.clone(),
                                    addresses: vec![addr],
                                    last_seen: now(),
                                    latency_ms: 0,
                                    reputation: 1.0,
                                    contribution_count: 0,
                                    capabilities: vec![],
                                },
                            );
                        }
                        libp2p::swarm::SwarmEvent::ConnectionClosed {
                            peer_id, cause, ..
                        } => {
                            tracing::info!(
                                "P2P: connection closed with peer {} ({:?})",
                                peer_id, cause
                            );
                            known_peers.write().await.remove(&peer_id);
                        }
                        libp2p::swarm::SwarmEvent::OutgoingConnectionError {
                            peer_id, error, ..
                        } => {
                            tracing::warn!(
                                "P2P: outgoing connection to {:?} failed: {:?}",
                                peer_id, error
                            );
                        }
                        libp2p::swarm::SwarmEvent::IncomingConnectionError { error, .. } => {
                            tracing::warn!("P2P: incoming connection failed: {:?}", error);
                        }
                        libp2p::swarm::SwarmEvent::NewListenAddr { address, .. } => {
                            tracing::info!("P2P: now listening on {}", address);
                        }
                        libp2p::swarm::SwarmEvent::Behaviour(OzoneBehaviourEvent::Mdns(
                            mdns::Event::Discovered(list),
                        )) => {
                            let mut s = swarm.lock().await;
                            for (peer_id, addr) in list {
                                tracing::info!("P2P: mDNS discovered peer {} at {}", peer_id, addr);
                                s.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                            }
                        }
                        libp2p::swarm::SwarmEvent::Behaviour(OzoneBehaviourEvent::Mdns(
                            mdns::Event::Expired(list),
                        )) => {
                            let mut s = swarm.lock().await;
                            for (peer_id, _addr) in list {
                                tracing::info!("P2P: mDNS peer expired {}", peer_id);
                                s.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                            }
                        }
                        // A received sync message (2026-10-20). Per
                        // the bridge's real call site
                        // (orchestrator::amt_loop::spawn_network_hook_bridge -> `on_items_announced`),
                        // the bytes are a JSON envelope — container_id,
                        // parent_id, container_type, source, scope_keywords
                        // — a lightweight "a container was created
                        // elsewhere" FINDING signal, not the container's
                        // full content (no full-content replication exists
                        // anywhere in this codebase; building that is a
                        // separate, larger feature, not attempted here).
                        //
                        // Trust gate (2026-10-20, same pass as the fix that
                        // made this arm reachable at all): a successful
                        // libp2p connection never implied trust — before
                        // this check, ANY connected peer's message was
                        // logged and treated as real signal identically to
                        // an explicitly-approved one. Now only a peer the
                        // operator has explicitly approved (`approve_peer`,
                        // `POST /network/approve-peer`) is treated as a
                        // real signal; anything else is logged at debug
                        // level and dropped — observable if you go looking
                        // (nothing is silently hidden), but never acted on
                        // or surfaced as trusted content.
                        libp2p::swarm::SwarmEvent::Behaviour(OzoneBehaviourEvent::Gossipsub(
                            gossipsub::Event::Message {
                                propagation_source,
                                message,
                                ..
                            },
                        )) => {
                            let summary = String::from_utf8_lossy(&message.data);
                            // MARKETPLACE messages (notify / catalog / fetch) are accepted from any CONNECTED peer and kept apart from the
                            // paired-only channel, so a stranger's catalog can never surface as a finding or be mistaken for a trusted message
                            if marketplace_kind(&message.data).is_some() {
                                // THE DOOR: validated + sanitized against the allow-list, or refused and logged — never the peer's raw bytes
                                match sanitize_marketplace_message(&message.data) {
                                    Ok(clean) => append_marketplace_inbox(&inbox_path, &propagation_source, &clean),
                                    Err(reason) => {
                                        tracing::debug!("P2P: refused a marketplace message from {}: {}", propagation_source, reason);
                                        append_marketplace_rejected(&rejected_path, &propagation_source, &reason);
                                    }
                                }
                            } else if paired_peers.read().await.contains_key(&propagation_source) {
                                tracing::info!(
                                    "P2P: received sync message from paired peer {}: {}",
                                    propagation_source, summary
                                );
                                append_peer_sync_candidate(
                                    &candidates_path,
                                    &propagation_source,
                                    &message.data,
                                );
                            } else {
                                tracing::debug!(
                                    "P2P: ignoring sync message from UNPAIRED peer {} (not approved — call approve_peer first): {}",
                                    propagation_source, summary
                                );
                            }
                        }
                        _ => {}
                    }
                }
            });
        }

        // Register default hooks
        self.register_default_hooks().await;

        // Reconnect to known peers from persistence
        let peers = self.known_peers.read().await;
        let addrs: Vec<String> = peers.values().map(|p| p.address.clone()).collect();
        drop(peers);

        for addr in addrs {
            if let Err(e) = self.connect_peer(&addr).await {
                tracing::warn!("Failed to reconnect to peer {}: {}", addr, e);
            }
        }

        Ok(())
    }

    /// Register default hooks for automatic sync
    async fn register_default_hooks(&self) {
        // Announcement hook - NOTIFY ONLY: a manifest (no body) is published to the marketplace and ONE push reaches the connected peers, who gate it.
        self.register_hook(
            "on_item_announced",
            Arc::new(|item: SyncItem| {
                tracing::info!(
                    "P2P: publishing {} {} to the marketplace (manifest only, no content)",
                    item.item_type.as_str(),
                    item.container_id
                );
                Ok(())
            }),
        )
        .await;

        // Pipeline result hook - ON-DEMAND only
        self.register_hook(
            "on_pipeline_completed",
            Arc::new(|item: SyncItem| {
                // Only sync if explicitly requested
                if item.priority != SyncPriority::Low {
                    tracing::debug!("Pipeline result shared: {}", item.container_id);
                }
                Ok(())
            }),
        )
        .await;
    }

    /// Register a hook for an event
    pub async fn register_hook(&self, event: &str, hook: HookFn) {
        let mut hooks = self.hooks.write().await;
        hooks
            .entry(event.to_string())
            .or_insert_with(Vec::new)
            .push(hook);
    }

    /// Trigger hooks for an event
    pub async fn trigger_hooks(&self, event: &str, item: SyncItem) -> OzoneResult<()> {
        let hooks = self.hooks.read().await;

        if let Some(event_hooks) = hooks.get(event) {
            for hook in event_hooks {
                hook(item.clone())?;
            }
        }

        // Also queue for sync based on priority
        match item.priority {
            SyncPriority::High => {
                self.high_priority_queue.write().await.push(item.clone());
                // Immediately trigger broadcast
                self.broadcast_high_priority().await?;
            }
            SyncPriority::Normal => {
                self.normal_queue.write().await.push(item);
            }
            SyncPriority::Low => {
                // Only queue if explicitly requested
            }
            SyncPriority::Never => {
                // Never sync
            }
        }

        Ok(())
    }

    /// PUBLISH a BATCH of allowed items (methodology, jurisdiction rule set, pipeline, tool) to MY marketplace listing and push ONE
    /// notification to every connected instance (a single publish on the shared marketplace topic), however many items the batch holds
    /// (operator 2026-11-10: "one push", event-driven coalescing — the bridge hands over every creation event that is already available, so
    /// there is no timer and no fixed window or size). The notification still names nothing (listing version + count only), so the door's
    /// strict shapes are unchanged: a connected peer pulls the listing and gates what it sees. Nothing is applied anywhere.
    pub async fn on_items_announced(&self, items: Vec<AnnounceItem>) -> OzoneResult<()> {
        let mut to_publish: Vec<(SyncItemType, serde_json::Value)> = Vec::new();
        let imported: serde_json::Value =
            std::fs::read_to_string(self.data_dir_of().join("peer_imported_items.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_else(|| serde_json::json!({}));
        let idx_path = self.data_dir_of().join("marketplace_published_bodies.json");
        let mut idx: serde_json::Map<String, serde_json::Value> = crate::durable_write::read_to_string(&idx_path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        let mut idx_changed = false;
        for it in items {
            if matches!(it.item_type, SyncItemType::Blueprint | SyncItemType::Container | SyncItemType::PipelineResult) {
                continue; // not a publishable kind
            }
            let Ok(m) = serde_json::from_slice::<serde_json::Value>(&it.manifest) else { continue };
            // an item we just IMPORTED from a peer is not re-published (it would only echo back to where it came from)
            let imported_key = format!("{}|{}", m["name"].as_str().unwrap_or(""), m["body_hash"].as_str().unwrap_or(""));
            if imported.get(&imported_key).is_some() {
                continue;
            }
            // LOCAL-ONLY index of where each published body lives (never sent): what a later fetch request is answered from
            if let Some(path) = it.body_path.as_deref() {
                idx.insert(format!("{}|{}|{}", it.item_type.as_str(), m["name"].as_str().unwrap_or(""), m["body_hash"].as_str().unwrap_or("")), serde_json::json!(path));
                idx_changed = true;
            }
            // observability hooks only; priority Never = nothing is queued for per-peer sending
            let item = SyncItem {
                container_id: it.container_id,
                item_type: it.item_type.clone(),
                priority: SyncPriority::Never,
                data: it.manifest,
                timestamp: chrono::Utc::now().timestamp() as u64,
            };
            self.trigger_hooks("on_item_announced", item).await?;
            to_publish.push((it.item_type, m));
        }
        if idx_changed {
            crate::durable_write::replace(&idx_path, serde_json::to_string_pretty(&idx).unwrap_or_default().into_bytes());
        }
        if to_publish.is_empty() {
            return Ok(());
        }
        let n_items = to_publish.len();
        match self.publish_batch_to_marketplace(&to_publish).await {
            Ok(n) => tracing::info!("P2P: marketplace updated with {} item(s); ONE notification pushed to {} connected peer(s)", n_items, n),
            Err(e) => tracing::debug!("P2P: marketplace updated locally with {} item(s); no push ({})", n_items, e),
        }
        Ok(())
    }

    fn data_dir_of(&self) -> std::path::PathBuf {
        self.paired_peers_path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| std::path::PathBuf::from("."))
    }

    /// Add every manifest of the batch to MY marketplace listing (`marketplace_published.json`, keyed by type + name, so a re-publish replaces;
    /// written ONCE) and push ONE notification on the shared topic. The notification names nothing — only the listing version, how many items
    /// the listing holds, and the item type when the whole batch is of one type (otherwise none; the receiver's door accepts both).
    pub async fn publish_batch_to_marketplace(&self, items: &[(SyncItemType, serde_json::Value)]) -> OzoneResult<usize> {
        let path = self.data_dir_of().join("marketplace_published.json");
        let mut listing: serde_json::Map<String, serde_json::Value> =
            crate::durable_write::read_to_string(&path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        for (item_type, manifest) in items {
            listing.insert(format!("{}|{}", item_type.as_str(), manifest["name"].as_str().unwrap_or("")), manifest.clone());
        }
        let version = body_hash(serde_json::to_string(&listing).unwrap_or_default().as_bytes());
        // Durable replace: the listing write is retried until it succeeds (and read back through durable_write above).
        crate::durable_write::replace(&path, serde_json::to_string_pretty(&listing).unwrap_or_default().into_bytes());
        let one_type = items.first().map(|(t, _)| t.as_str()).filter(|t0| items.iter().all(|(t, _)| t.as_str() == *t0));
        let notify = serde_json::to_vec(&serde_json::json!({
            "kind": "marketplace_notify",
            "listing_version": version,
            "count": listing.len(),
            "item_type": one_type,
        }))
        .unwrap_or_default();
        self.broadcast(MARKETPLACE_TOPIC, &notify).await
    }

    /// Notify of pipeline completion (ON-DEMAND only)
    pub async fn on_pipeline_completed(
        &self,
        container_id: ContainerID,
        data: Vec<u8>,
        share_requested: bool,
    ) -> OzoneResult<()> {
        if !share_requested {
            return Ok(());
        }

        let item = SyncItem {
            container_id,
            item_type: SyncItemType::PipelineResult,
            priority: SyncPriority::Low,
            data,
            timestamp: chrono::Utc::now().timestamp() as u64,
        };

        self.trigger_hooks("on_pipeline_completed", item).await
    }

    /// Broadcast high priority items immediately
    async fn broadcast_high_priority(&self) -> OzoneResult<()> {
        let items = {
            let mut queue = self.high_priority_queue.write().await;
            std::mem::take(&mut *queue)
        };

        if items.is_empty() {
            return Ok(());
        }

        let peers = self.known_peers.read().await;

        for item in &items {
            for (peer_id, _peer) in peers.iter() {
                // Outbound trust gate (default-deny): never send to a peer
                // the operator has not explicitly approved.
                if !self.is_peer_paired(peer_id).await {
                    tracing::debug!("P2P: skipping UNPAIRED peer {} for outbound sync", peer_id);
                    continue;
                }
                if let Err(e) = self.send_to_peer(peer_id, &item.data).await {
                    tracing::warn!("Failed to send to peer {}: {}", peer_id, e);
                }
            }
        }

        Ok(())
    }

    /// Connect to a peer using libp2p
    pub async fn connect_peer(&mut self, peer_addr: &str) -> OzoneResult<()> {
        use libp2p::Multiaddr;
        use std::str::FromStr;

        // Parse the multiaddr
        let multiaddr = Multiaddr::from_str(peer_addr)
            .map_err(|e| OzoneError::NetworkError(format!("Invalid peer address: {}", e)))?;

        // Extract peer ID from the address if present
        let peer_id = multiaddr.iter().find_map(|p| {
            if let libp2p::multiaddr::Protocol::P2p(hash) = p {
                PeerId::from_multihash(hash.into()).ok()
            } else {
                None
            }
        });

        // Get the swarm
        let mut swarm = self.swarm.lock().await;

        // Dial the peer
        match swarm.dial(multiaddr.clone()) {
            Ok(_) => {
                // Track the connection attempt
                self.connection_attempts
                    .insert(multiaddr.to_string(), std::time::Instant::now());

                // If we have peer_id, add to known peers
                if let Some(pid) = peer_id {
                    self.known_peers.write().await.insert(
                        pid,
                        PeerInfo {
                            peer_id: pid.to_string(),
                            address: multiaddr.to_string(),
                            addresses: vec![multiaddr.to_string()],
                            last_seen: now(),
                            latency_ms: 0,
                            reputation: 0.5,
                            contribution_count: 0,
                            capabilities: Vec::new(),
                        },
                    );
                }

                tracing::info!("Initiated connection to peer: {}", peer_addr);
                Ok(())
            }
            Err(e) => {
                tracing::warn!("Failed to dial peer {}: {}", peer_addr, e);
                Err(OzoneError::NetworkError(format!("Failed to connect: {}", e)))
            }
        }
    }

    /// Explicitly mark a peer as trusted (2026-10-20, the real trust gate —
    /// see `paired_peers`'s own doc comment on the struct). Operator-only,
    /// same loopback-only HTTP security boundary as `connect_peer`/the
    /// pairing routes — no additional auth layer here, consistent with the
    /// rest of this API. Idempotent (re-approving just refreshes the
    /// timestamp) and persisted immediately, not batched, since approvals
    /// are rare, deliberate actions, not hot-path traffic.
    pub async fn approve_peer(&self, peer_id: PeerId) -> OzoneResult<()> {
        self.paired_peers.write().await.insert(peer_id, now());
        save_paired_peers(&self.paired_peers_path, &*self.paired_peers.read().await);
        tracing::info!("P2P: peer {} explicitly approved (paired)", peer_id);
        Ok(())
    }

    /// Revoke a previously-approved peer. Added alongside `approve_peer`
    /// rather than later — an approve-only API with no way back out would
    /// be a real gap the moment a mistake or a compromised peer needs
    /// undoing.
    pub async fn revoke_peer(&self, peer_id: &PeerId) -> OzoneResult<()> {
        self.paired_peers.write().await.remove(peer_id);
        save_paired_peers(&self.paired_peers_path, &*self.paired_peers.read().await);
        tracing::info!("P2P: peer {} approval revoked (unpaired)", peer_id);
        Ok(())
    }

    /// Whether a peer has been explicitly approved — the one real trust
    /// check this module has. Anything gating on peer-sent content MUST
    /// call this rather than inferring trust from `known_peers`/a
    /// successful connection, which never implied trust (the original gap).
    pub async fn is_peer_paired(&self, peer_id: &PeerId) -> bool {
        self.paired_peers.read().await.contains_key(peer_id)
    }

    /// List of currently-approved peer ids, for a status/listing endpoint.
    pub async fn paired_peer_ids(&self) -> Vec<String> {
        self.paired_peers.read().await.keys().map(|p| p.to_string()).collect()
    }

    /// Send data to a specific peer using libp2p gossipsub
    pub async fn send_to_peer(&self, peer_id: &PeerId, data: &[u8]) -> OzoneResult<()> {
        // Outbound trust gate (default-deny), enforced here too so no caller
        // can bypass it: content only ever goes to an operator-approved peer.
        if !self.is_peer_paired(peer_id).await {
            tracing::debug!("P2P: refusing send to UNPAIRED peer {}", peer_id);
            return Err(OzoneError::NetworkError(format!(
                "Peer {} is not paired (approve it first)",
                peer_id
            )));
        }
        // Get the swarm
        let mut swarm = self.swarm.lock().await;

        // Check if peer is connected
        if !swarm.is_connected(peer_id) {
            return Err(OzoneError::NetworkError(format!(
                "Peer {} is not connected",
                peer_id
            )));
        }

        // Create a direct topic for this peer (for unicast)
        let topic_str = format!("direct/{}", peer_id);
        let topic = libp2p::gossipsub::IdentTopic::new(&topic_str);

        // Publish the message
        match swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic, data.to_vec())
        {
            Ok(message_id) => {
                tracing::debug!("Sent message {} to peer {}", message_id, peer_id);

                // Update stats
                let mut stats = self.stats.lock().await;
                stats.messages_sent += 1;
                stats.bytes_sent += data.len() as u64;

                Ok(())
            }
            Err(e) => {
                tracing::warn!("Failed to send to peer {}: {:?}", peer_id, e);
                Err(OzoneError::NetworkError(format!("Send failed: {:?}", e)))
            }
        }
    }

    /// Send a MARKETPLACE message to a CONNECTED peer (no approval needed — the marketplace is a catalog). Refuses anything that is not a
    /// marketplace message kind, so this can never be used to send other content to an unapproved peer.
    pub async fn send_to_connected_peer(&self, peer_id: &PeerId, data: &[u8]) -> OzoneResult<()> {
        if marketplace_kind(data).is_none() {
            return Err(OzoneError::NetworkError("send_to_connected_peer only carries marketplace messages".to_string()));
        }
        let mut swarm = self.swarm.lock().await;
        if !swarm.is_connected(peer_id) {
            return Err(OzoneError::NetworkError(format!("Peer {} is not connected", peer_id)));
        }
        let topic = libp2p::gossipsub::IdentTopic::new(format!("direct/{}", peer_id));
        match swarm.behaviour_mut().gossipsub.publish(topic, data.to_vec()) {
            Ok(_) => {
                let mut stats = self.stats.lock().await;
                stats.messages_sent += 1;
                stats.bytes_sent += data.len() as u64;
                Ok(())
            }
            Err(e) => Err(OzoneError::NetworkError(format!("Send failed: {:?}", e))),
        }
    }

    /// Broadcast data to all connected peers
    pub async fn broadcast(&self, topic: &str, data: &[u8]) -> OzoneResult<usize> {
        use libp2p::gossipsub::IdentTopic;

        let topic = IdentTopic::new(topic);
        let mut swarm = self.swarm.lock().await;

        match swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic, data.to_vec())
        {
            Ok(_) => {
                let peer_count = swarm.connected_peers().count();
                let mut stats = self.stats.lock().await;
                stats.messages_sent += peer_count as u64;
                stats.bytes_sent += (data.len() * peer_count) as u64;
                Ok(peer_count)
            }
            Err(e) => Err(OzoneError::NetworkError(format!("Broadcast failed: {:?}", e))),
        }
    }

    /// Get network status
    pub async fn get_status(&self) -> NetworkStatus {
        let peers = self.known_peers.read().await;
        let high_queue = self.high_priority_queue.read().await;
        let normal_queue = self.normal_queue.read().await;

        NetworkStatus {
            enabled: self.config.enable_p2p,
            peer_id: Some(self.local_peer_id.to_string()),
            connected_peers: peers.len(),
            pending_high_priority: high_queue.len(),
            pending_normal: normal_queue.len(),
        }
    }
}

/// Network status summary
#[derive(Debug, Clone)]
pub struct NetworkStatus {
    pub enabled: bool,
    pub peer_id: Option<String>,
    pub connected_peers: usize,
    pub pending_high_priority: usize,
    pub pending_normal: usize,
}

// ========================= PEER SYNC REVIEW (receive side) =========================

/// PEER SYNC REVIEW (guide §7.3 first version, 2026-10-23): drain
/// `peer_sync_candidates.jsonl` — the append-only log of envelopes received
/// from PAIRED peers — and surface each as a REAL finding container via
/// context_mirror (kind "finding" → CoordinationEvent → the choke point →
/// ripple → CoordinationFindingActor → a candidate the amt_loop can act on).
/// Bookkeeping only: NO model call, NO container replicated from the peer
/// (content stays the peer's; the finding cites it). Progress is an offset
/// file, so a crash resumes where it left off and nothing is re-surfaced.
pub fn spawn_peer_sync_review(
    zsei: std::sync::Arc<tokio::sync::RwLock<crate::zsei::ZSEI>>,
    data_dir: String,
) {
    tokio::spawn(async move {
        let log_path = std::path::PathBuf::from(&data_dir).join("peer_sync_candidates.jsonl");
        let offset_path = std::path::PathBuf::from(&data_dir).join("peer_sync_review.offset");
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(120));
        tick.tick().await; // consume the immediate tick: first pass after 2 min
        loop {
            tick.tick().await;
            let offset: u64 = crate::durable_write::read_to_string(&offset_path)
                .ok()
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
            let Ok(content) = std::fs::read_to_string(&log_path) else {
                continue; // nothing received yet — honest no-op
            };
            let byte_len = content.len() as u64;
            if byte_len <= offset {
                continue;
            }
            let fresh = &content[usize::try_from(offset).unwrap_or(0).min(content.len())..];
            let mut processed: u64 = 0;
            let mut surfaced = 0u64;
            for line in fresh.lines() {
                processed += line.len() as u64 + 1; // newline
                let Ok(row) = serde_json::from_str::<serde_json::Value>(line) else {
                    continue; // a malformed line is skipped, counted, not fatal
                };
                let from_peer = row
                    .get("from_peer")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown-peer");
                let envelope = row.get("envelope").cloned().unwrap_or(serde_json::json!({}));
                let ctype = envelope
                    .get("container_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                let cid = envelope.get("container_id").and_then(|v| v.as_u64()).unwrap_or(0);
                let req = crate::context_mirror::MirrorRequest {
                    kind: "finding".to_string(),
                    agent: "peer-sync".to_string(),
                    title: format!("P2P sync from peer: {ctype} #{cid}"),
                    body: format!(
                        "A PAIRED peer reported {ctype} #{cid}. This is a FINDING about the peer's activity, not replicated content — the peer's data stays the peer's."
                    ),
                    files: Vec::new(),
                    detail: Some(serde_json::json!({
                        "from_peer": from_peer,
                        "envelope": envelope,
                        "source": "peer-sync-review",
                    })),
                    scope: Some("global".to_string()),
                    workspace_id: None,
                    project_id: None,
                };
                {
                    let z = zsei.read().await;
                    let _ = crate::context_mirror::mirror(&z, &data_dir, &req).await;
                }
                surfaced += 1;
            }
            crate::durable_write::replace(&offset_path, (offset + processed).to_string().into_bytes());
            if surfaced > 0 {
                tracing::info!(surfaced, "peer sync review: surfaced paired-peer findings as containers (bookkeeping, no model calls)");
            }
        }
    });
}

// ========================= PEER DIALOGUE PRIMITIVES =========================

/// PEER DIALOGUE (2026-10-23, primitives-with-caps per the cross-instance
/// dialogue plan): the typed envelope two instances would exchange to hold
/// an ongoing conversation about a shared topic (e.g. self-improvement),
/// and the ONE send path every future caller must use. The caps and the
/// kill switch are enforced INSIDE the send path, so no caller — loop,
/// actor, or a future dialogue process — can bypass them.
///
/// Deliberately NOT auto-spawned: nothing in this pass starts a dialogue,
/// replies automatically, or shares content. What exists is the safe
/// substrate: typed turns, a shared-thread key, hop limiting, a daily
/// budget, a size cap, trace linkage, and the paired-only + env-gated
/// send path (dialogue rides the same trust gate as every other send).
pub const DIALOGUE_ENV: &str = "OZONE_PEER_DIALOGUE";
pub const DIALOGUE_MAGIC: &str = "ozone-dialogue/v1";
pub const DIALOGUE_MAX_TURNS_PER_DAY: u32 = 20;
pub const DIALOGUE_MAX_HOP: u32 = 3;
/// Size of ONE wire part, not a limit on a turn: a longer body is SPLIT into parts and sent whole (chunkable system — nothing is refused or cut
/// because of its size; the receiving model's window decides how the parts are read).
pub const DIALOGUE_MAX_BODY_CHARS: usize = 4_000;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PeerDialogueTurn {
    /// Shared topic key (both instances derive it from the same topic the
    /// same way — see new_thread_id) so turns group into one conversation.
    pub thread_id: String,
    pub turn: u32,
    /// Turn id this turn answers, when it is a reply.
    pub reply_to: Option<String>,
    /// Relay guard: a turn forwarded by an intermediate instance dies here.
    pub hop_count: u32,
    /// The sender's trace id — a conversation is joinable to both sides'
    /// capture stores through it.
    pub trace_id: Option<String>,
    /// Filled by the RECEIVER (the sender cannot know its own peer id as
    /// seen remotely); never trusted, only displayed/logged.
    #[serde(default)]
    pub from_peer: Option<String>,
    pub body: String,
    pub ts: u64,
    /// Multi-part turns (2026-11-04): this part's 1-based index and the total number of parts. `0/0` (the default, and what every
    /// pre-existing turn deserialises to) = the whole body in one message.
    #[serde(default)]
    pub part: u32,
    #[serde(default)]
    pub parts: u32,
}

/// The transport's maximum message (gossipsub's own default, now set explicitly in the config above): a PROTOCOL fact, not a content limit.
pub const GOSSIPSUB_MAX_TRANSMIT_BYTES: usize = 65_536;

/// Size of one fetched-body FRAGMENT on the wire — under half the transport maximum (leaving room for the JSON envelope and escaping). This is
/// transport fragmentation, NOT a context-window chunk and NOT a size limit: a body of any size is SPLIT into as many fragments as it needs and
/// re-joined by the receiver, never refused for size. (Window-derived chunking belongs to what a MODEL reads; this is what a SOCKET carries.)
pub const FETCH_CHUNK_BYTES: usize = 30_000;

/// Split `body` into chunks of at most `max_bytes`, on char boundaries (a body is never cut inside a character). Never returns zero chunks.
pub fn split_chunks(body: &str, max_bytes: usize) -> Vec<String> {
    let max = max_bytes.max(4);
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    for ch in body.chars() {
        if cur.len() + ch.len_utf8() > max {
            out.push(std::mem::take(&mut cur));
        }
        cur.push(ch);
    }
    if !cur.is_empty() || out.is_empty() {
        out.push(cur);
    }
    out
}

/// MARKETPLACE EXCHANGE (connected peers; operator 2026-11-10). Reads the marketplace INBOX (`peer_marketplace_inbox.jsonl`, written by the
/// swarm from any connected peer) and:
/// * a `marketplace_notify` whose listing version we have not seen → `catalog_request` to that peer;
/// * a `catalog_request` → answer with MY listing (`marketplace_published.json`) — manifests only, no bodies;
/// * a `fetch_request` → serve ONLY an item that is in my published index, only if its body still hashes to the requested hash, in chunks
///   (`fetch_chunk`, `FETCH_CHUNK_BYTES` each; blueprints and anything unpublished can never be served); otherwise a `fetch_error`.
/// `serve` (= `marketplace_publish`) turns on the catalog_request / fetch_request answers; `pull` (= `marketplace_accept_offers`) turns on the
/// catalog_request sent for a `marketplace_notify`. Either may be on without the other.
/// A `catalog` / `fetch_chunk` / `fetch_error` is only recorded here — the marketplace lists offers and `orchestrator::peer_import` handles
/// the chunks of requests WE made. Progress is an offset file; only COMPLETE lines are processed.
pub fn spawn_marketplace_exchange(network: std::sync::Arc<tokio::sync::RwLock<NetworkManager>>, data_dir: String, serve: bool, pull: bool) {
    tokio::spawn(async move {
        let dir = std::path::PathBuf::from(&data_dir);
        let log = dir.join("peer_marketplace_inbox.jsonl");
        let offset_path = dir.join("marketplace_exchange.offset");
        let versions_path = dir.join("peer_listing_versions.json");
        let published_path = dir.join("marketplace_published.json");
        let bodies_path = dir.join("marketplace_published_bodies.json");
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(30));
        tick.tick().await;
        loop {
            tick.tick().await;
            let offset: usize = crate::durable_write::read_to_string(&offset_path).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
            let Ok(content) = std::fs::read_to_string(&log) else { continue };
            if content.len() <= offset || !content.is_char_boundary(offset) {
                continue;
            }
            let fresh = &content[offset..];
            let upto = fresh.rfind('\n').map(|i| i + 1).unwrap_or(0); // complete lines only
            if upto == 0 {
                continue;
            }
            let mut versions: serde_json::Map<String, serde_json::Value> =
                crate::durable_write::read_to_string(&versions_path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
            for line in fresh[..upto].lines() {
                let Ok(row) = serde_json::from_str::<serde_json::Value>(line) else { continue };
                let env = &row["envelope"];
                let Some(from) = row["from_peer"].as_str() else { continue };
                let Ok(peer) = from.parse::<PeerId>() else { continue };
                match env["kind"].as_str() {
                    // PULL side (`marketplace_accept_offers`): a peer published something — ask for its listing
                    Some("marketplace_notify") if pull => {
                        let v = env["listing_version"].as_str().unwrap_or("");
                        if v.is_empty() || versions.get(from).and_then(|x| x.as_str()) == Some(v) {
                            continue;
                        }
                        let req = serde_json::json!({"kind": "catalog_request", "listing_version": v}).to_string();
                        if network.read().await.send_to_connected_peer(&peer, req.as_bytes()).await.is_ok() {
                            versions.insert(from.to_string(), serde_json::json!(v));
                        }
                    }
                    // SERVE side (`marketplace_publish`): hand our listing (manifests only) to a connected peer that asks for it
                    Some("catalog_request") if serve => {
                        let listing: serde_json::Map<String, serde_json::Value> =
                            std::fs::read_to_string(&published_path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
                        if listing.is_empty() {
                            continue;
                        }
                        let items: Vec<serde_json::Value> = listing.values().cloned().collect();
                        let msg = serde_json::json!({"kind": "catalog", "items": items}).to_string();
                        let _ = network.read().await.send_to_connected_peer(&peer, msg.as_bytes()).await;
                    }
                    // SERVE side: a body goes out only for an item WE published and only when its hash still matches (the peer's operator approved it)
                    Some("fetch_request") if serve => {
                        let (item_type, name, hash) = (env["item_type"].as_str().unwrap_or(""), env["name"].as_str().unwrap_or(""), env["body_hash"].as_str().unwrap_or(""));
                        let idx: serde_json::Map<String, serde_json::Value> =
                            std::fs::read_to_string(&bodies_path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
                        let body = idx
                            .get(&format!("{item_type}|{name}|{hash}"))
                            .and_then(|v| v.as_str())
                            .and_then(crate::orchestrator::tool_binding::read_object_file)
                            .filter(|b| body_hash(b.as_bytes()) == hash);
                        let net = network.read().await;
                        match body {
                            Some(body) => {
                                let listing: serde_json::Map<String, serde_json::Value> =
                                    std::fs::read_to_string(&published_path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
                                let keywords = listing.get(&format!("{item_type}|{name}")).and_then(|m| m.get("keywords")).cloned().unwrap_or_else(|| serde_json::json!([]));
                                let chunks = split_chunks(&body, FETCH_CHUNK_BYTES);
                                let total = chunks.len();
                                for (seq, data) in chunks.into_iter().enumerate() {
                                    let msg = serde_json::json!({
                                        "kind": "fetch_chunk", "item_type": item_type, "name": name, "body_hash": hash,
                                        "seq": seq, "total": total, "data": data,
                                        "keywords": if seq == 0 { keywords.clone() } else { serde_json::Value::Null },
                                    })
                                    .to_string();
                                    if net.send_to_connected_peer(&peer, msg.as_bytes()).await.is_err() {
                                        break;
                                    }
                                }
                            }
                            None => {
                                let msg = serde_json::json!({"kind": "fetch_error", "item_type": item_type, "name": name, "body_hash": hash,
                                    "reason": "not published here, or its content changed since it was announced"})
                                .to_string();
                                let _ = net.send_to_connected_peer(&peer, msg.as_bytes()).await;
                            }
                        }
                    }
                    _ => {}
                }
            }
            crate::durable_write::replace(&versions_path, serde_json::to_string_pretty(&versions).unwrap_or_default().into_bytes());
            crate::durable_write::replace(&offset_path, (offset + upto).to_string().into_bytes());
        }
    });
}

/// Derive the shared thread id for a topic: both instances hash the same
/// normalized topic to the same key without transmitting it in the clear.
pub fn new_thread_id(topic: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let normalized: String = topic
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let mut h = DefaultHasher::new();
    normalized.hash(&mut h);
    format!("{:016x}", h.finish())
}

fn dialogue_enabled() -> bool {
    std::env::var(DIALOGUE_ENV).as_deref() == Ok("1")
}

/// Day-key for the daily budget (UTC date).
fn dialogue_day() -> String {
    // Days since epoch — no chrono dependency here.
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() / 86_400)
        .unwrap_or(0)
        .to_string()
}

static DIALOGUE_BUDGET: std::sync::Mutex<Option<(String, u32)>> = std::sync::Mutex::new(None);

fn take_dialogue_budget() -> Result<(), String> {
    let mut g = DIALOGUE_BUDGET.lock().unwrap();
    let day = dialogue_day();
    match g.as_mut() {
        Some((d, n)) if *d == day => {
            if *n >= DIALOGUE_MAX_TURNS_PER_DAY {
                return Err(format!(
                    "peer dialogue daily budget exhausted ({DIALOGUE_MAX_TURNS_PER_DAY}/day) — the kill switch is the env var, the budget is the brake"
                ));
            }
            *n += 1;
            Ok(())
        }
        _ => {
            *g = Some((day, 1));
            Ok(())
        }
    }
}

impl NetworkManager {
    /// Send one dialogue turn to a PAIRED peer. Caps + kill switch are
    /// enforced here so no caller can bypass them: env-gated
    /// (OZONE_PEER_DIALOGUE=1), hop-limited, size-capped, daily-budgeted,
    /// and default-deny on pairing (send_to_peer's own gate).
    pub async fn send_dialogue_turn(
        &self,
        peer_id: &libp2p::PeerId,
        mut turn: PeerDialogueTurn,
    ) -> OzoneResult<()> {
        if !dialogue_enabled() {
            return Err(OzoneError::NetworkError(
                "peer dialogue disabled (set OZONE_PEER_DIALOGUE=1 to enable)".into(),
            ));
        }
        if turn.hop_count >= DIALOGUE_MAX_HOP {
            return Err(OzoneError::NetworkError(format!(
                "dialogue turn exceeded hop limit ({DIALOGUE_MAX_HOP}) — not relayed"
            )));
        }
        // ONE logical turn = ONE unit of the daily budget, however many wire parts it needs.
        take_dialogue_budget().map_err(OzoneError::NetworkError)?;
        turn.ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        // CHUNKED, NEVER REFUSED OR CUT (operator doctrine 2026-11-04: the system is graph-based and chunkable — any model, any context size):
        // a body longer than one wire part is split on character boundaries into numbered parts, each sent whole; the receiver reassembles by
        // (thread_id, turn, part/parts). The body is never truncated and never rejected for its size.
        let chars: Vec<char> = turn.body.chars().collect();
        let part_size = DIALOGUE_MAX_BODY_CHARS.max(1);
        let total_parts = if chars.is_empty() { 1 } else { (chars.len() + part_size - 1) / part_size };
        for idx in 0..total_parts {
            let mut part_turn = turn.clone();
            if total_parts > 1 {
                part_turn.body = chars[idx * part_size..((idx + 1) * part_size).min(chars.len())].iter().collect();
                part_turn.part = idx as u32 + 1;
                part_turn.parts = total_parts as u32;
            }
            let wire = serde_json::json!({
                "magic": DIALOGUE_MAGIC,
                "turn": part_turn,
            });
            let data = serde_json::to_vec(&wire)
                .map_err(|e| OzoneError::NetworkError(format!("dialogue serialize: {e}")))?;
            tracing::info!(
                thread = %part_turn.thread_id,
                turn = part_turn.turn,
                part = part_turn.part,
                parts = part_turn.parts,
                "peer dialogue: sending turn part (budgeted per turn, paired-only)"
            );
            self.send_to_peer(peer_id, &data).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod marketplace_tests {
    #[test]
    fn split_chunks_never_cuts_a_character_and_reassembles_exactly() {
        let body = "héllo wörld — ünïcode ✓ ".repeat(500);
        let chunks = super::split_chunks(&body, 97);
        assert!(chunks.iter().all(|c| c.len() <= 97) && chunks.len() > 1);
        assert_eq!(chunks.concat(), body, "reassembly is exact, nothing lost");
        assert_eq!(super::split_chunks("", 10), vec![String::new()], "never zero chunks");
        assert_eq!(super::split_chunks("short", 1000), vec!["short".to_string()]);
    }

    #[test]
    fn marketplace_kind_recognizes_only_marketplace_messages_and_body_hash_is_stable() {
        assert_eq!(super::marketplace_kind(br#"{"kind":"fetch_request"}"#).as_deref(), Some("fetch_request"));
        assert!(super::marketplace_kind(br#"{"kind":"announce"}"#).is_none(), "a plain announce is not a marketplace exchange message");
        assert!(super::marketplace_kind(b"not json").is_none());
        assert_eq!(super::body_hash(b"abc"), super::body_hash(b"abc"));
        assert_ne!(super::body_hash(b"abc"), super::body_hash(b"abd"));
    }

    #[test]
    fn the_door_refuses_everything_not_allowed_and_sanitizes_what_it_lets_in() {
        let ok = |v: serde_json::Value| super::sanitize_marketplace_message(v.to_string().as_bytes());
        // a good catalog: markers stripped from keywords, unknown fields dropped, a blueprint / unknown type item dropped
        let c = ok(serde_json::json!({"kind":"catalog","items":[
            {"item_type":"methodology","name":"API Design","body_hash":"00ff00ff00ff00ff","keywords":["api","ws:3","scope:global","design"],"evil":"x"},
            {"item_type":"blueprint","name":"Private plan","body_hash":"00ff00ff00ff00ff"},
            {"item_type":"tool","name":"bad hash","body_hash":"nothex"}]}))
        .unwrap();
        let items = c["items"].as_array().unwrap();
        assert_eq!(items.len(), 1, "blueprint and the bad-hash item are refused");
        assert_eq!(items[0]["keywords"], serde_json::json!(["api", "design"]), "scope markers never come in");
        assert!(items[0].get("evil").is_none(), "unknown fields are dropped");
        assert!(ok(serde_json::json!({"kind":"catalog","items":[{"item_type":"blueprint","name":"x"}]})).is_err(), "a catalog with only unacceptable items is refused");
        // kinds and shapes
        assert!(ok(serde_json::json!({"kind":"announce"})).is_err(), "not a marketplace kind");
        assert!(super::sanitize_marketplace_message(b"nope").is_err());
        assert!(ok(serde_json::json!({"kind":"fetch_request","item_type":"methodology","name":"X","body_hash":"zz"})).is_err());
        assert!(ok(serde_json::json!({"kind":"fetch_request","item_type":"blueprint","name":"X","body_hash":"00ff00ff00ff00ff"})).is_err());
        assert!(ok(serde_json::json!({"kind":"fetch_chunk","item_type":"methodology","name":"X","body_hash":"00ff00ff00ff00ff","seq":3,"total":3,"data":"a"})).is_err(), "seq must be < total");
        assert!(ok(serde_json::json!({"kind":"fetch_chunk","item_type":"methodology","name":"X","body_hash":"00ff00ff00ff00ff","seq":0,"total":2,"data":"a","keywords":["k","ws:1"]})).unwrap()["keywords"] == serde_json::json!(["k"]));
        assert!(ok(serde_json::json!({"kind":"marketplace_notify","listing_version":"not ok!"})).is_err());
        assert!(ok(serde_json::json!({"kind":"fetch_request","item_type":"methodology","name":"bad\u{0007}name","body_hash":"00ff00ff00ff00ff"})).is_err(), "control characters in a name");
    }
}
