//! Network module for P2P synchronization
//!
//! Based on Section 15-17 of the specification.
//!
//! Handles:
//! - P2P peer discovery and connection
//! - Hook-based sync triggers for methodologies, blueprints, and findings
//! - Network status monitoring
//!
//! SYNC BEHAVIOR:
//! - Methodologies: ALWAYS sync (no significance check)
//! - Blueprints: ALWAYS sync (no significance check)
//! - New Container Findings: ALWAYS sync (queue for batch)
//! - Pipeline Results: ON-DEMAND only (explicit share request)
//! - User Data: NEVER sync (local only)

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
    Blueprint,
    Container,
    PipelineResult,
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
/// no container is created, no model is called). Best-effort: any failure is
/// logged and swallowed so the swarm event loop can never be taken down by
/// disk trouble.
fn append_peer_sync_candidate(path: &std::path::Path, from: &PeerId, data: &[u8]) {
    use std::io::Write;
    let envelope = serde_json::from_slice::<serde_json::Value>(data)
        .unwrap_or_else(|_| serde_json::Value::String(String::from_utf8_lossy(data).to_string()));
    let line = serde_json::json!({
        "received_at": now(),
        "from_peer": from.to_string(),
        "envelope": envelope,
    })
    .to_string();
    let result = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut f| writeln!(f, "{}", line));
    if let Err(e) = result {
        tracing::warn!("P2P: failed to append peer_sync_candidates.jsonl at {:?}: {}", path, e);
    }
}

/// Persist the paired-peers store. Best-effort: a write failure is logged,
/// not propagated as an error up through `approve_peer`/`revoke_peer` — the
/// in-memory state (the actual source of truth for the running process)
/// still updates either way, so a transient disk issue degrades to "this
/// approval won't survive a restart" rather than failing the operator's
/// action outright.
fn save_paired_peers(path: &std::path::Path, peers: &HashMap<PeerId, u64>) {
    let entries: HashMap<String, u64> = peers.iter().map(|(pid, ts)| (pid.to_string(), *ts)).collect();
    match serde_json::to_string_pretty(&entries) {
        Ok(json) => {
            if let Err(e) = std::fs::write(path, json) {
                tracing::error!("P2P: failed to persist paired_peers.json at {:?}: {}", path, e);
            }
        }
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

        // Generate or load keypair
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        log::info!("Local peer ID: {}", local_peer_id);

        // Configure gossipsub
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(std::time::Duration::from_secs(1))
            .validation_mode(gossipsub::ValidationMode::Strict)
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
                        // `on_methodology_created`/etc.'s real call site
                        // (orchestrator::amt_loop::spawn_network_hook_bridge),
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
                            if paired_peers.read().await.contains_key(&propagation_source) {
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
        // Methodology hook - ALWAYS sync
        self.register_hook(
            "on_methodology_created",
            Arc::new(|item: SyncItem| {
                tracing::info!(
                    "Methodology created, queuing for immediate sync: {}",
                    item.container_id
                );
                Ok(())
            }),
        )
        .await;

        // Blueprint hook - ALWAYS sync
        self.register_hook(
            "on_blueprint_created",
            Arc::new(|item: SyncItem| {
                tracing::info!(
                    "Blueprint created, queuing for immediate sync: {}",
                    item.container_id
                );
                Ok(())
            }),
        )
        .await;

        // Container finding hook - ALWAYS sync (batched)
        self.register_hook(
            "on_container_created",
            Arc::new(|item: SyncItem| {
                tracing::debug!(
                    "Container created, queuing for batch sync: {}",
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

    /// Notify of new methodology (ALWAYS syncs)
    pub async fn on_methodology_created(
        &self,
        container_id: ContainerID,
        data: Vec<u8>,
    ) -> OzoneResult<()> {
        let item = SyncItem {
            container_id,
            item_type: SyncItemType::Methodology,
            priority: SyncPriority::High,
            data,
            timestamp: chrono::Utc::now().timestamp() as u64,
        };

        self.trigger_hooks("on_methodology_created", item).await
    }

    /// Notify of new blueprint (ALWAYS syncs)
    pub async fn on_blueprint_created(
        &self,
        container_id: ContainerID,
        data: Vec<u8>,
    ) -> OzoneResult<()> {
        let item = SyncItem {
            container_id,
            item_type: SyncItemType::Blueprint,
            priority: SyncPriority::High,
            data,
            timestamp: chrono::Utc::now().timestamp() as u64,
        };

        self.trigger_hooks("on_blueprint_created", item).await
    }

    /// Notify of new container finding (ALWAYS syncs, batched)
    pub async fn on_container_created(
        &self,
        container_id: ContainerID,
        data: Vec<u8>,
    ) -> OzoneResult<()> {
        let item = SyncItem {
            container_id,
            item_type: SyncItemType::Container,
            priority: SyncPriority::Normal,
            data,
            timestamp: chrono::Utc::now().timestamp() as u64,
        };

        self.trigger_hooks("on_container_created", item).await
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

    /// Process batch sync (called periodically)
    pub async fn process_batch_sync(&self) -> OzoneResult<()> {
        let items = {
            let mut queue = self.normal_queue.write().await;
            std::mem::take(&mut *queue)
        };

        if items.is_empty() {
            return Ok(());
        }

        tracing::info!("Processing batch sync of {} items", items.len());

        let peers = self.known_peers.read().await;

        // Group items and send in batches
        for item in &items {
            for (peer_id, _peer) in peers.iter() {
                if !self.is_peer_paired(peer_id).await {
                    tracing::debug!("P2P: skipping UNPAIRED peer {} for outbound batch sync", peer_id);
                    continue;
                }
                if let Err(e) = self.send_to_peer(peer_id, &item.data).await {
                    tracing::warn!("Failed to batch send to peer {}: {}", peer_id, e);
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
