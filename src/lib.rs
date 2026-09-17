//! Ozone Studio - Omnidirectional Zero-Shot Neural Engine
//!
//! A systems-first platform for omnidirectional, zero-shot data traversal,
//! abstraction, and context compilation.
//!
//! # Architecture
//!
//! Ozone Studio operates as a pipeline execution engine with:
//! - ZSEI (Zero-Shot Embedded Indexer) for knowledge fabric
//! - Pipeline system for composable, executable units
//! - Task management for tracking all computation
//! - Network layer for P2P sync of methodologies, blueprints, findings
//! - Consciousness system for AGI capabilities (enabled via config)
//! - UI layer (Electron) for user interaction
//!
//! # Core Principles
//!
//! - Structure before intelligence
//! - Compression before learning
//! - Traversal before generation
//! - Pipelines over monoliths
//! - Zero-shot discovery without task-specific training
//! - LLMs are clients, not the system core
//! - Context not copies
//! - Link not duplicate
//! - Integrity always

pub mod auth;
pub mod blueprints;
pub mod bootstrap;
pub mod config;
pub mod consciousness;
pub mod grpc;
pub mod integrity;
pub mod methodologies;
pub mod monitor;
pub mod network;
pub mod orchestrator;
pub mod pipeline;
pub mod task;
pub mod types;
pub mod zsei;

/// K-ALGORITHM shared contracts (single host inclusion point).
#[path = "../shared/contracts/mod.rs"]
pub mod shared_contracts;

/// K-ALGORITHM typed registry facade.
pub mod k_registry;

/// MCP TOOL registry — external tool connection surface.
pub mod mcp;

/// Shared-context mirroring — coordination state as real ZSEI containers
/// under the /SharedContext root (notes, decisions, handoffs, file claims).
pub mod context_mirror;

/// GRAPH EVENT ripple — the living-graph nervous system: every graph write
/// publishes a scoped event; websockets, monitor, and hooks subscribe.
pub mod graph_events;

/// QR DEVICE pairing — the phone as authenticator, multi-device onboarding.
pub mod pairing;

/// Hardware/OS-derived region detection for the jurisdiction gate (real
/// system signals only — timezone via tzdata's own zone.tab, locale via
/// POSIX naming — never a fabricated lookup or network geolocation call).
pub mod hardware_region;


// Re-exports
pub use config::OzoneConfig;
pub use types::*;

use bootstrap::BootstrapManager;
use std::sync::Arc;
use tokio::sync::RwLock;

use task::{RefinementConfig, TaskQueueConfig};

/// Real ISO-3166-1 alpha-2 codes of the EU's 27 member states — used only to
/// decide which national jurisdiction rulesets also relate to the "eu"
/// baseline scope during boot-time relationship-graph wiring, not asserted
/// as legal fact beyond that. Module-level (not inlined in the wiring block)
/// so `compute_jurisdiction_edges_to_add` and its tests share one real list.
pub const JURISDICTION_EU_MEMBER_CODES: &[&str] = &[
    "at", "be", "bg", "hr", "cy", "cz", "dk", "ee", "fi", "fr",
    "de", "gr", "hu", "ie", "it", "lv", "lt", "lu", "mt", "nl",
    "pl", "pt", "ro", "sk", "si", "es", "se",
];

/// Pure logic for the jurisdiction container self-heal registration —
/// extracted from the boot-time block in `OzoneRuntime::new` so its
/// idempotency guarantee is independently testable without a real ZSEI
/// store. Given the scope keywords already registered as children of
/// `JURISDICTION_ROOT_ID` and the full candidate list found on disk,
/// returns exactly the candidates that still need registering (empty when
/// every candidate is already covered — the "0 new on re-run" guarantee).
pub fn compute_new_jurisdiction_registrations(
    already_registered: &std::collections::HashSet<String>,
    candidates: &[(String, std::path::PathBuf)],
) -> Vec<(String, std::path::PathBuf)> {
    candidates
        .iter()
        .filter(|(scope, _)| !already_registered.contains(scope))
        .cloned()
        .collect()
}

/// Pure logic for the jurisdiction relationship-graph wiring — extracted
/// from the boot-time block in `OzoneRuntime::new` so both the edge shape
/// (T-J2) and its idempotency (T-J5) are independently testable. Given one
/// non-"global" scope's current relationships and the resolved global/eu
/// representative container ids, returns the NEW `Relation` edges this
/// scope should gain — empty when it already has them, which is exactly
/// what makes a second call with the first call's edges already merged in a
/// no-op (the idempotency guarantee the boot-time block relies on to be
/// safely re-run every startup).
pub fn compute_jurisdiction_edges_to_add(
    scope: &str,
    container_id: crate::types::ContainerID,
    existing_relationships: &[crate::types::container::Relation],
    global_id: Option<crate::types::ContainerID>,
    eu_id: Option<crate::types::ContainerID>,
) -> Vec<crate::types::container::Relation> {
    use crate::types::container::{DiscoveryMethod, Relation, RelationType};

    let mut new_edges = Vec::new();

    if let Some(gid) = global_id {
        if gid != container_id && !existing_relationships.iter().any(|r| r.target_id == gid) {
            new_edges.push(Relation {
                target_id: gid,
                relation_type: RelationType::RelatedTo,
                confidence: 0.9,
                discovered_via: DiscoveryMethod::Manual,
            });
        }
    }

    if scope != "eu" {
        if let Some(eid) = eu_id {
            if JURISDICTION_EU_MEMBER_CODES.contains(&scope)
                && !existing_relationships.iter().any(|r| r.target_id == eid)
            {
                new_edges.push(Relation {
                    target_id: eid,
                    relation_type: RelationType::RelatedTo,
                    confidence: 0.9,
                    discovered_via: DiscoveryMethod::Manual,
                });
            }
        }
    }

    new_edges
}

/// Result of the full AMT orchestration flow
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrchestrationOutput {
    pub success: bool,
    pub response_text: Option<String>,
    pub task_id: Option<u64>,
    pub blueprint_id: Option<u64>,
    pub stages_completed: Vec<serde_json::Value>,
    pub needs_clarification: bool,
    pub clarification_points: Vec<String>,
    pub model_used: Option<String>,
    pub total_tokens_used: Option<u32>,
    pub amt_summary: Option<serde_json::Value>,
    /// Was silently dropped here before — build_error_response's real
    /// message never made it past this struct even though it always existed
    /// inside OrchestrationResponse.error, so a failed /orchestrate call
    /// always returned error: null over HTTP with no way to know what broke.
    pub error: Option<String>,
    /// Full "thinking cycle" — one entry per real LLM call made this run
    /// (AMT-building passes, blueprint drafting, zero-shot simulation, step
    /// execution), each carrying the FULL raw response text. See
    /// orchestrator::ThinkingEntry.
    pub thinking_log: Vec<serde_json::Value>,
}

/// Main Ozone Studio runtime
pub struct OzoneRuntime {
    /// Configuration
    pub config: OzoneConfig,

    /// ZSEI instance
    pub zsei: Arc<RwLock<zsei::ZSEI>>,

    /// Pipeline registry
    pub pipeline_registry: Arc<RwLock<pipeline::PipelineRegistry>>,

    pub task_manager: Arc<RwLock<task::TaskManager>>,

    /// Authentication system
    pub auth: Arc<RwLock<auth::AuthSystem>>,

    /// Integrity monitor
    pub integrity: Arc<RwLock<integrity::IntegrityMonitor>>,

    /// Network manager for P2P sync
    pub network: Arc<RwLock<network::NetworkManager>>,

    /// Current session
    pub session: Arc<RwLock<Option<types::auth::Session>>>,

    /// Consciousness system (enabled/disabled via config.toml)
    pub consciousness: Option<Arc<RwLock<consciousness::ConsciousnessSystem>>>,
}

impl OzoneRuntime {
    /// Create a new Ozone runtime with the given configuration
    pub async fn new(config: OzoneConfig) -> Result<Self, OzoneError> {
        tracing::info!("Initializing Ozone Studio v{}", env!("CARGO_PKG_VERSION"));

        // Run bootstrap if first time setup
        if !config.general.setup_complete {
            tracing::info!("First-time setup detected, running bootstrap...");
            let bootstrap = BootstrapManager::new(&config);
            bootstrap.run()?;

            // Mark setup complete and save config
            // Note: We clone and modify since config is borrowed
            let mut updated_config = config.clone();
            updated_config.general.setup_complete = true;
            let config_path = std::path::Path::new("config.toml");
            if let Err(e) = updated_config.save(config_path) {
                tracing::warn!("Failed to save config after bootstrap: {}", e);
            }
        }

        // Voice pipeline (#10) reads OZONE_VOICE_* from its inherited
        // environment — export the configured voice settings once at boot
        // (and again on gRPC voice updates) so spawned children inherit them.
        for (k, v) in config.voice.to_pipeline_env() {
            std::env::set_var(&k, &v);
        }
        // Prompt pipeline (#9) reads OZONE_MODEL_* / OZONE_WIRE_PROTOCOL the
        // same way (serve mode + one-shot share this config source).
        for (k, v) in config.models.to_pipeline_env() {
            std::env::set_var(&k, &v);
        }
        // Web search pipeline (#56) — disabled by default; only forwards
        // which env var NAME holds a real search API key, never a
        // fabricated key or fabricated results (see that pipeline's own
        // doc comment).
        for (k, v) in config.web_search.to_pipeline_env() {
            std::env::set_var(&k, &v);
        }
        // K-ALGORITHM defaults (convergence/pairwise — the only families
        // with a live consumer today, orchestrator/amt.rs). The registry
        // ships with hardcoded defaults ("fast"/"default"); this applies
        // config.toml's choice on top before anything reads it.
        {
            let k = crate::k_registry::KAlgorithms::global();
            k.set_convergence_preset(&config.k_algorithms.convergence_preset);
            k.set_pairwise_preset(&config.k_algorithms.pairwise_preset);
        }

        // Initialize ZSEI
        let zsei = zsei::ZSEI::new(&config.zsei)?;

        let zsei_arc = Arc::new(RwLock::new(zsei));

        // --- Register artifacts as ZSEI containers (idempotent, runs every startup) ---
        {
            // Self-heal: installs that completed bootstrap before these index
            // files existed (bootstrap only runs once, gated on setup_complete)
            // would otherwise silently skip all methodology/blueprint
            // registration forever — same gap as the pipeline index, fixed the
            // same way, since Stage 3 (Gather Methodologies) and Stage 6
            // (Blueprint Assignment) both depend on these being registered.
            let methodology_index_path = std::path::PathBuf::from(&config.zsei.methodology_index_path);
            if !methodology_index_path.exists() {
                if let Some(parent) = methodology_index_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let index = crate::bootstrap::BootstrapManager::get_default_methodology_index();
                match serde_json::to_string_pretty(&index) {
                    Ok(content) => match std::fs::write(&methodology_index_path, content) {
                        Ok(()) => tracing::info!(
                            "Generated missing methodology index at {} (self-heal)",
                            methodology_index_path.display()
                        ),
                        Err(e) => tracing::warn!("Failed to write methodology index: {}", e),
                    },
                    Err(e) => tracing::warn!("Failed to serialize default methodology index: {}", e),
                }
            }

            let blueprint_index_path = std::path::PathBuf::from(&config.zsei.blueprint_index_path);
            if !blueprint_index_path.exists() {
                if let Some(parent) = blueprint_index_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let index = crate::bootstrap::BootstrapManager::get_default_blueprint_index();
                match serde_json::to_string_pretty(&index) {
                    Ok(content) => match std::fs::write(&blueprint_index_path, content) {
                        Ok(()) => tracing::info!(
                            "Generated missing blueprint index at {} (self-heal)",
                            blueprint_index_path.display()
                        ),
                        Err(e) => tracing::warn!("Failed to write blueprint index: {}", e),
                    },
                    Err(e) => tracing::warn!("Failed to serialize default blueprint index: {}", e),
                }
            }

            let methodology_store = crate::methodologies::store::MethodologyStore::new(
                zsei_arc.clone(),
                methodology_index_path,
            );
            if let Err(e) = methodology_store.register_all().await {
                tracing::warn!("Methodology ZSEI registration: {}", e);
            }

            let blueprint_store = crate::blueprints::store::BlueprintStore::new(
                zsei_arc.clone(),
                blueprint_index_path,
            );
            if let Err(e) = blueprint_store.register_all().await {
                tracing::warn!("Blueprint ZSEI registration: {}", e);
            }

            let pipeline_store = crate::pipeline::PipelineStore::new(zsei_arc.clone());
            if let Err(e) = pipeline_store.register_all().await {
                tracing::warn!("Pipeline ZSEI registration: {}", e);
            }

            // Self-heal the remaining ZSEI structural root containers
            // (Modality, Consciousness spheres, External, runtime graph
            // roots, etc.) — bootstrap.rs used to write these as
            // local/<id>.json files, a path the real mmap-backed
            // ContainerStorage engine never reads, so they were always
            // invisible to GetContainer and their ids sat unprotected until
            // the id-allocator fix above. METHODOLOGY_ROOT_ID /
            // BLUEPRINT_ROOT_ID / PIPELINE_ROOT_ID are skipped — the stores
            // just above already self-heal those with real child_ids this
            // generic pass doesn't know how to compute. ROOT_CONTAINER_ID
            // (0) is skipped too — ContainerStorage::ensure_root already
            // self-heals it at the storage layer on every ZSEI::new.
            {
                use crate::types::container::{
                    Container, Context, GlobalState, LocalState, Metadata, Modality,
                    BLUEPRINT_ROOT_ID, METHODOLOGY_ROOT_ID, PIPELINE_ROOT_ID, ROOT_CONTAINER_ID,
                };
                let mut zsei = zsei_arc.write().await;
                for (id, name, mat_path, container_type) in
                    crate::bootstrap::BootstrapManager::structural_root_specs()
                {
                    if id == ROOT_CONTAINER_ID
                        || id == METHODOLOGY_ROOT_ID
                        || id == BLUEPRINT_ROOT_ID
                        || id == PIPELINE_ROOT_ID
                    {
                        continue;
                    }
                    let needs_repair = match zsei.get_container(id).await {
                        Ok(Some(existing)) => existing.local_state.metadata.container_type != container_type,
                        Ok(None) => true,
                        Err(e) => {
                            tracing::warn!("Structural root {} lookup failed: {}", id, e);
                            false
                        }
                    };
                    if !needs_repair {
                        continue;
                    }
                    let root = Container {
                        global_state: GlobalState {
                            container_id: id,
                            parent_id: 0,
                            child_ids: vec![],
                            child_count: 0,
                            version: 1,
                        },
                        local_state: LocalState {
                            metadata: Metadata {
                                container_type,
                                modality: Modality::Unknown,
                                created_at: crate::bootstrap::BootstrapManager::now(),
                                updated_at: crate::bootstrap::BootstrapManager::now(),
                                provenance: "bootstrap".to_string(),
                                permissions: 0,
                                owner_id: 0,
                                name: Some(name.to_string()),
                                materialized_path: Some(mat_path.to_string()),
                            },
                            context: Context {
                                keywords: vec![name.to_lowercase()],
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                    };
                    if let Err(e) = zsei.store_container(root).await {
                        tracing::warn!("Failed to self-heal structural root {} ({}): {}", id, name, e);
                    }
                }
            }
        }

        // --- Register jurisdiction content as real ZSEI containers (idempotent,
        // runs every startup) --- Found live 2026-09-15: the structural
        // JurisdictionRoot container self-heals above, but nothing ever
        // actually read zsei_data/jurisdiction/*.json (global.json,
        // national/*.json) and registered them as real, queryable
        // JurisdictionRuleSet containers — the ORIGINAL global.json
        // container was a one-off manual creation earlier in this project's
        // history, not a repeatable mechanism, so a fresh instance (or this
        // dev instance after any restart that didn't happen to still hold
        // the old in-memory/cache state) had zero real jurisdiction
        // containers despite the content files genuinely being on disk —
        // confirmed live via GetContainer(JURISDICTION_ROOT_ID) showing
        // child_count: 0. Mirrors MethodologyStore::register_all's pattern
        // (see src/methodologies/store.rs): read real content files, check
        // for an existing container by scope keyword before creating
        // (idempotent), never fabricate content — this only registers files
        // that already exist on disk.
        {
            use crate::types::container::{
                Container, Context, GlobalState, IntegrityData, LocalState, Metadata, Modality,
                StoragePointers, TraversalHints, ContainerType, JURISDICTION_ROOT_ID,
            };

            let jurisdiction_dir = std::path::PathBuf::from(&config.general.data_dir).join("jurisdiction");

            // Self-heal the copy itself, not just the container registration —
            // confirmed live 2026-09-15: bootstrap's copy_jurisdiction_content()
            // only ever runs once, gated on setup_complete, so a data_dir that
            // was set up before newer jurisdiction/national/*.json files
            // existed (or one reached via a different CWD than earlier boots —
            // this exact scenario, when the host was launched from the repo
            // root instead of target/release, so a naive relative "assets/"
            // path would silently resolve to nothing — confirmed live,
            // target/release/assets/ genuinely does not exist) silently never
            // receives them, same class of gap the methodology/blueprint
            // index self-heal above already solves. Uses the same
            // CWD-independent resolver bootstrap.rs already relies on, not a
            // relative path. Mirrors assets/jurisdiction/ into
            // <data_dir>/jurisdiction/, additive only — never deletes a file
            // that's only in the data dir (e.g. one a human added directly).
            let real_assets_jurisdiction_dir =
                crate::bootstrap::BootstrapManager::resolve_assets_dir().join("jurisdiction");
            if let Ok(entries) = std::fs::read_dir(&real_assets_jurisdiction_dir) {
                let _ = std::fs::create_dir_all(&jurisdiction_dir);
                for entry in entries.flatten() {
                    let src_path = entry.path();
                    let file_name = entry.file_name();
                    if src_path.is_dir() {
                        let dst_subdir = jurisdiction_dir.join(&file_name);
                        let _ = std::fs::create_dir_all(&dst_subdir);
                        if let Ok(sub_entries) = std::fs::read_dir(&src_path) {
                            for sub in sub_entries.flatten() {
                                let dst = dst_subdir.join(sub.file_name());
                                if !dst.exists() {
                                    if let Err(e) = std::fs::copy(sub.path(), &dst) {
                                        tracing::warn!(path = %dst.display(), error = %e, "Failed to self-heal jurisdiction content copy");
                                    }
                                }
                            }
                        }
                    } else {
                        let dst = jurisdiction_dir.join(&file_name);
                        if !dst.exists() {
                            if let Err(e) = std::fs::copy(&src_path, &dst) {
                                tracing::warn!(path = %dst.display(), error = %e, "Failed to self-heal jurisdiction content copy");
                            }
                        }
                    }
                }
            }

            // scope keyword -> path relative to data_dir, e.g. "global" ->
            // "jurisdiction/global.json", "us" -> "jurisdiction/national/us.json"
            let mut candidates: Vec<(String, std::path::PathBuf)> = Vec::new();
            let global_file = jurisdiction_dir.join("global.json");
            if global_file.exists() {
                candidates.push(("global".to_string(), std::path::PathBuf::from("jurisdiction/global.json")));
            }
            let national_dir = jurisdiction_dir.join("national");
            if let Ok(entries) = std::fs::read_dir(&national_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("json") {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            candidates.push((
                                stem.to_lowercase(),
                                std::path::PathBuf::from("jurisdiction/national")
                                    .join(format!("{}.json", stem)),
                            ));
                        }
                    }
                }
            }

            if !candidates.is_empty() {
                let mut zsei = zsei_arc.write().await;

                // Existing registered scopes, so re-running this on every
                // boot never creates duplicates.
                let mut already_registered: std::collections::HashSet<String> =
                    std::collections::HashSet::new();
                if let Ok(Some(root)) = zsei.get_container(JURISDICTION_ROOT_ID).await {
                    for child_id in &root.global_state.child_ids {
                        if let Ok(Some(child)) = zsei.get_container(*child_id).await {
                            for kw in &child.local_state.context.keywords {
                                already_registered.insert(kw.clone());
                            }
                        }
                    }
                }

                let to_register = compute_new_jurisdiction_registrations(&already_registered, &candidates);
                let mut registered = 0usize;
                for (scope, rel_path) in to_register {
                    let container = Container {
                        global_state: GlobalState {
                            container_id: 0, // overwritten by CreateContainer
                            parent_id: JURISDICTION_ROOT_ID,
                            child_ids: vec![],
                            child_count: 0,
                            version: 1,
                        },
                        local_state: LocalState {
                            metadata: Metadata {
                                container_type: ContainerType::JurisdictionRuleSet,
                                modality: Modality::Unknown,
                                created_at: crate::bootstrap::BootstrapManager::now(),
                                updated_at: crate::bootstrap::BootstrapManager::now(),
                                provenance: "bootstrap".to_string(),
                                permissions: 0,
                                owner_id: 0,
                                name: Some(format!("Jurisdiction: {}", scope)),
                                materialized_path: Some(format!("/Jurisdiction/{}", scope)),
                            },
                            context: Context {
                                categories: vec![],
                                methodologies: vec![],
                                keywords: vec![scope.clone()],
                                topics: vec!["jurisdiction".to_string()],
                                relationships: vec![],
                                learned_associations: vec![],
                                embedding: None,
                            },
                            storage: StoragePointers {
                                db_shard_id: None,
                                vector_index_ref: None,
                                object_store_path: Some(
                                    rel_path.to_string_lossy().replace('\\', "/"),
                                ),
                                compression_type: crate::types::container::CompressionType::None,
                            },
                            hints: TraversalHints::default(),
                            integrity: IntegrityData::default(),
                            file_context: None,
                            code_context: None,
                            text_context: None,
                            external_ref: None,
                        },
                    };
                    match zsei
                        .query(crate::types::zsei::ZSEIQuery::CreateContainer {
                            parent_id: JURISDICTION_ROOT_ID,
                            container,
                        })
                        .await
                    {
                        Ok(_) => registered += 1,
                        Err(e) => tracing::warn!(
                            scope = %scope,
                            error = %e,
                            "Failed to register jurisdiction container"
                        ),
                    }
                }
                tracing::info!(
                    "Jurisdiction store: {} new registrations ({} scopes already registered)",
                    registered,
                    already_registered.len()
                );
            }
        }

        // --- Wire real jurisdiction relationship edges (graph, not just a
        // flat list) --- Found live 2026-09-15: traversal.rs's 6 modes never
        // read Context.relationships (structural_traversal just fixed above
        // in the same pass to follow Relation edges), but nothing ever
        // POPULATED jurisdiction relationships either — every
        // JurisdictionRuleSet container created above starts with
        // `relationships: vec![]`. This wires the real structural fact that
        // a national ruleset is layered on top of (not a replacement for) a
        // broader baseline: every non-global scope gets a RelatedTo edge to
        // "global", and any EU member state additionally gets a RelatedTo
        // edge to "eu" (the EU baseline is itself layered on "global" the
        // same way). This is what makes jurisdiction an actual graphed meta
        // workspace instead of a flat list only connected by shared
        // keywords. Idempotent (skips a relation that already exists), runs
        // every boot so it self-heals for content added after this code
        // first ran.
        {
            use crate::types::ContainerID;
            use crate::types::container::JURISDICTION_ROOT_ID;

            let mut zsei = zsei_arc.write().await;
            if let Ok(Some(root)) = zsei.get_container(JURISDICTION_ROOT_ID).await {
                let mut scope_to_id: std::collections::HashMap<String, ContainerID> =
                    std::collections::HashMap::new();
                for child_id in &root.global_state.child_ids {
                    if let Ok(Some(child)) = zsei.get_container(*child_id).await {
                        if let Some(kw) = child.local_state.context.keywords.first() {
                            scope_to_id.insert(kw.clone(), *child_id);
                        }
                    }
                }

                let global_id = scope_to_id.get("global").copied();
                let eu_id = scope_to_id.get("eu").copied();
                let scopes: Vec<(String, ContainerID)> =
                    scope_to_id.iter().map(|(s, id)| (s.clone(), *id)).collect();

                let mut wired = 0usize;
                for (scope, container_id) in scopes {
                    if scope.as_str() == "global" {
                        continue;
                    }
                    let container = match zsei.get_container(container_id).await {
                        Ok(Some(c)) => c,
                        _ => continue,
                    };

                    let mut relationships = container.local_state.context.relationships.clone();
                    let new_edges = compute_jurisdiction_edges_to_add(
                        &scope,
                        container_id,
                        &relationships,
                        global_id,
                        eu_id,
                    );
                    let changed = !new_edges.is_empty();
                    relationships.extend(new_edges);

                    if changed {
                        let mut new_context = container.local_state.context.clone();
                        new_context.relationships = relationships;
                        if let Err(e) = zsei
                            .query(crate::types::zsei::ZSEIQuery::UpdateContainer {
                                container_id,
                                updates: crate::types::zsei::ContainerUpdate {
                                    context: Some(new_context),
                                    ..Default::default()
                                },
                            })
                            .await
                        {
                            tracing::warn!(scope = %scope, error = %e, "Failed to wire jurisdiction relationship edge");
                        } else {
                            wired += 1;
                        }
                    }
                }
                if wired > 0 {
                    tracing::info!("Jurisdiction graph: wired {} relationship edges", wired);
                }
            }
        }

        // Wire ZSEI into ConsciousnessStore (so experiences persist to ZSEI)
        {
            if let Ok(mut store) = crate::consciousness::CONSCIOUSNESS_STORE.lock() {
                let consciousness_store: Arc<dyn crate::orchestrator::StoreAccess> =
                    Arc::new(crate::orchestrator::ZseiStoreAdapter { zsei: zsei_arc.clone() });
                store.set_store(consciousness_store);
            }
        }

        // Initialize pipeline registry
        let pipeline_registry = pipeline::PipelineRegistry::new(&config.pipelines)?;

        // Give every spawned pipeline subprocess a real way to make its own
        // internal LLM calls (keyword/topic extraction, etc.) without any
        // host auth bypass: point at pipeline 9's own binary, which already
        // does complete standalone LLM dispatch (BitNet/OpenRouter) with no
        // host callback. A pipeline that needs this spawns this binary
        // itself, exactly like the orchestrator spawns pipelines — never via
        // the user-session-gated /pipeline/execute HTTP route. Command
        // inherits the full parent environment by default (no env_clear
        // anywhere in executor.rs), so setting this once here reaches every
        // spawned pipeline without touching invoke_pipeline itself.
        //
        // MUST run after PipelineRegistry::new() above, not before it —
        // confirmed live this session as a real bug: get_pipeline_info(9)
        // prefers the runtime registry (loaded from index.json, correctly
        // says category "general") but falls back to the compile-time
        // PIPELINE_INFO table (which labels pipeline 9 "core" — a stale
        // logical grouping that every OTHER caller of get_pipeline_info,
        // e.g. pipeline/executor.rs's get_builtin_info, only ever reads
        // AFTER this same registry init, so the mismatch was invisible until
        // this specific lookup ran too early and used the wrong category).
        // Left unset (never a guessed/fake path) if pipeline 9 isn't built
        // on this instance — callers must treat "not set" as "no real
        // executor available", same as today's stub fallback.
        if let Some(info) = crate::pipeline::registry::get_pipeline_info(9) {
            let name = info.folder_name.as_str();
            let assets_pipeline_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("assets/pipelines")
                .join(info.category)
                .join(name);
            let candidates = [
                assets_pipeline_dir.join("target/release").join(name),
                assets_pipeline_dir.join("target/debug").join(name),
            ];
            if let Some(found) = candidates.iter().find(|c| c.exists()) {
                std::env::set_var("OZONE_PROMPT_PIPELINE_PATH", found);
            } else {
                tracing::warn!(
                    "Pipeline 9 (prompt) binary not found — OZONE_PROMPT_PIPELINE_PATH left \
                     unset; pipelines needing an internal LLM call will report unavailable"
                );
            }
        }

        // Initialize task manager
        let refinement_config = RefinementConfig {
            interval_secs: config.tasks.refinement_interval_secs,
            enabled: config.tasks.refinement_enabled,
            ..RefinementConfig::default()
        };
        let task_manager = task::TaskManager::new(TaskQueueConfig::default(), refinement_config)?;

        // Initialize auth system
        let auth = auth::AuthSystem::new(&config.auth)?;

        // Initialize integrity monitor
        let integrity = integrity::IntegrityMonitor::new(&config.integrity)?;

        // Initialize network manager
        let mut network = network::NetworkManager::new(config.network.clone()).await?;
        network.initialize().await?;

        // Initialize consciousness if enabled in config
        let consciousness = if config.consciousness.enabled {
            tracing::info!("Consciousness system: ENABLED");
            Some(Arc::new(RwLock::new(
                consciousness::ConsciousnessSystem::new(),
            )))
        } else {
            tracing::info!("Consciousness system: DISABLED (enable in config.toml)");
            None
        };

        Ok(Self {
            config,
            zsei: zsei_arc.clone(),
            pipeline_registry: Arc::new(RwLock::new(pipeline_registry)),
            task_manager: Arc::new(RwLock::new(task_manager)),
            auth: Arc::new(RwLock::new(auth)),
            integrity: Arc::new(RwLock::new(integrity)),
            network: Arc::new(RwLock::new(network)),
            session: Arc::new(RwLock::new(None)),
            consciousness,
        })
    }

    /// Start the runtime (gRPC server for UI communication)
    pub async fn start(self) -> Result<(), OzoneError> {
        tracing::info!("Starting Ozone Studio runtime");

        // Wrap self in Arc<RwLock<...>> for sharing with server handlers
        let runtime = Arc::new(RwLock::new(self));

        // Start integrity monitoring
        let integrity = runtime.read().await.integrity.clone();
        tokio::spawn(async move {
            if let Err(e) = integrity.write().await.start_monitoring().await {
                tracing::error!("Integrity monitoring failed: {}", e);
            }
        });

        // Start the real methodology meta-loop — previously TaskManager::
        // start_refinement_daemon existed as real code but had zero callers
        // anywhere, so it never ran once; that daemon's own sub-tasks were
        // also stubs (log-only "consider splitting", no actual gap
        // detection or methodology creation). This is a separate loop
        // (src/orchestrator/meta_loop.rs) because drafting a genuine new
        // methodology needs a real LLM call, which TaskManager has no way
        // to make (it only holds ZSEI access) — PromptOrchestrator's own
        // executor/store adapters do. Respects the same RefinementConfig
        // (enabled/interval_secs) the dormant daemon uses.
        {
            let executor_adapter: Arc<dyn crate::orchestrator::PipelineExecutor> =
                Arc::new(crate::orchestrator::RegistryExecutorAdapter {
                    registry: runtime.read().await.pipeline_registry.clone(),
                });
            let store_adapter: Arc<dyn crate::orchestrator::StoreAccess> =
                Arc::new(crate::orchestrator::ZseiStoreAdapter {
                    zsei: runtime.read().await.zsei.clone(),
                });
            let refinement_config = runtime
                .read()
                .await
                .task_manager
                .read()
                .await
                .refinement_config()
                .clone();
            // Both background loops are genuinely detached meta work (not
            // live request-answering — see the explicit distinction made
            // elsewhere this session for why stage_3_blueprint_assignment
            // does NOT use meta_fallback), so they use config.models.
            // meta_fallback (local+free by default) rather than the
            // user-facing fallback chain, and need the real model list to
            // resolve identifiers against.
            let available_models = runtime.read().await.config.models.available_models.clone();
            let meta_fallback = runtime.read().await.config.models.meta_fallback.clone();
            tokio::spawn(crate::orchestrator::meta_loop::run_methodology_meta_loop(
                executor_adapter.clone(),
                store_adapter.clone(),
                refinement_config.clone(),
                available_models.clone(),
                meta_fallback.clone(),
            ));

            // Real AMT re-expansion loop (orchestrator/amt_loop.rs) — same
            // shape as the methodology meta-loop above, same
            // executor/store contracts, same RefinementConfig. Reviews
            // real re-expansion candidates recorded live by
            // amt.rs::record_amt_reexpansion_candidate whenever a
            // project-anchored AMT still has an unverified node.
            tokio::spawn(crate::orchestrator::amt_loop::run_amt_reexpansion_loop(
                executor_adapter.clone(),
                store_adapter.clone(),
                refinement_config,
                available_models,
                meta_fallback,
            ));

            // GRAPH RIPPLE → AMT SYNC (task 43 groundwork): graph writes in
            // a project's scope convert into AMT re-expansion candidates and
            // wake the loop above instantly — the AMT stays context-aligned
            // with the living graph instead of waiting for the interval.
            crate::orchestrator::amt_loop::spawn_graph_ripple_sync(
                store_adapter,
                format!(
                    "{}/amt_reexpansion_candidates.json",
                    std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string())
                ),
            );
        }

        // Start gRPC server
        grpc::start_server(runtime).await?;

        Ok(())
    }

    /// Authenticate a user
    pub async fn authenticate(
        &self,
        public_key: &[u8],
        signature: &[u8],
    ) -> Result<types::auth::Session, OzoneError> {
        let session = self
            .auth
            .write()
            .await
            .authenticate(public_key, signature)
            .await?;
        *self.session.write().await = Some(session.clone());
        Ok(session)
    }

    /// Execute a pipeline
    pub async fn execute_pipeline(
        &self,
        pipeline_id: PipelineID,
        input: types::pipeline::PipelineInput,
    ) -> Result<types::pipeline::PipelineOutput, OzoneError> {
        // Ensure user is authenticated
        let session = self.session.read().await;
        let _session = session
            .as_ref()
            .ok_or_else(|| OzoneError::AuthError("Not authenticated".into()))?;

        // Execute pipeline
        let registry = self.pipeline_registry.read().await;
        registry.execute(pipeline_id, input, None).await
    }

    /// Query ZSEI
    pub async fn query_zsei(
        &self,
        query: types::zsei::ZSEIQuery,
    ) -> Result<types::zsei::ZSEIQueryResult, OzoneError> {
        self.zsei.read().await.query(query).await
    }

    /// Check if consciousness is enabled
    pub fn is_consciousness_enabled(&self) -> bool {
        self.consciousness.is_some()
    }

    /// Run the full 14-stage AMT orchestration flow.
    /// This is the ONLY entry point for prompt-driven task execution.
    pub async fn orchestrate(
        &self,
        input: crate::types::pipeline::PipelineInput,
        user_id: u64,
        device_id: u64,
    ) -> OzoneResult<OrchestrationOutput> {
        let prompt = input
            .data
            .get("prompt")
            .and_then(|v| {
                if let crate::types::Value::String(s) = v {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();

        if prompt.trim().is_empty() {
            return Err(OzoneError::ValidationError("Prompt cannot be empty".into()));
        }

        let project_id = input.data.get("project_id").and_then(|v| {
            if let crate::types::Value::Int(i) = v {
                Some(*i as u64)
            } else {
                None
            }
        });

        let workspace_id = input.data.get("workspace_id").and_then(|v| {
            if let crate::types::Value::Int(i) = v {
                Some(*i as u64)
            } else {
                None
            }
        });

        let token_budget = input
            .data
            .get("token_budget")
            .and_then(|v| {
                if let crate::types::Value::Int(i) = v {
                    Some(*i as u32)
                } else {
                    None
                }
            })
            .unwrap_or(100_000);

        let consciousness_enabled = input
            .data
            .get("consciousness_enabled")
            .and_then(|v| {
                if let crate::types::Value::Bool(b) = v {
                    Some(*b)
                } else {
                    None
                }
            })
            .unwrap_or(false);

        // The orchestrator owns task creation (Stage: Task Creation) —
        // no pre-enqueue here. Adapters wire the orchestrator's abstract
        // contracts to the concrete pipeline registry and ZSEI store.
        let attached_files: Vec<crate::orchestrator::AttachedFileSpec> = input
            .data
            .get("attached_files")
            .and_then(|v| serde_json::to_value(v).ok())
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        let model_config = input
            .data
            .get("model_config")
            .and_then(|v| serde_json::to_value(v).ok())
            .and_then(|v| serde_json::from_value(v).ok());

        let request = crate::orchestrator::OrchestrationRequest {
            prompt: prompt.clone(),
            project_id,
            workspace_id,
            user_id,
            device_id,
            consciousness_enabled,
            token_budget: Some(token_budget),
            model_config,
            attached_files,
            processing_path: Default::default(),
            executor_model: Default::default(),
            voice_input: None,
            available_models: self.config.models.available_models.clone(),
            fallback_order: self.config.models.fallback.order.clone(),
            fallback_free_only: self.config.models.fallback.free_only,
            meta_fallback_order: self.config.models.meta_fallback.order.clone(),
            meta_fallback_free_only: self.config.models.meta_fallback.free_only,
        };

        let executor_adapter = Arc::new(crate::orchestrator::RegistryExecutorAdapter {
            registry: self.pipeline_registry.clone(),
        });
        let zsei_adapter = Arc::new(crate::orchestrator::ZseiStoreAdapter {
            zsei: self.zsei.clone(),
        });

        let orchestrator = crate::orchestrator::PromptOrchestrator::new(
            executor_adapter,
            zsei_adapter,
            self.task_manager.clone(),
            Arc::new(RwLock::new(None)),
            self.config.models.context_length as u32,
            self.config.jurisdiction.clone(),
        );

        let response = orchestrator.orchestrate(request).await;

        Ok(OrchestrationOutput {
            success: response.success,
            response_text: response.response,
            task_id: response.task_id,
            blueprint_id: response.blueprint_id,
            stages_completed: response
                .stages_completed
                .iter()
                .map(|s| serde_json::to_value(s).unwrap_or_default())
                .collect(),
            needs_clarification: response.needs_clarification,
            clarification_points: response.clarification_points,
            model_used: response.model_used,
            total_tokens_used: response.total_tokens_used,
            amt_summary: response
                .amt_summary
                .as_ref()
                .and_then(|s| serde_json::to_value(s).ok()),
            error: response.error,
            thinking_log: response
                .thinking_log
                .iter()
                .map(|t| serde_json::to_value(t).unwrap_or_default())
                .collect(),
        })
    }
}

/// Initialize logging with default settings
pub fn init_logging() {
    init_logging_with_level("info");
}

/// Initialize logging with specified level
pub fn init_logging_with_level(level: &str) {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    let env_filter = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| format!("ozone_studio={},hyper=warn,tonic=warn", level));

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| env_filter.into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_thread_ids(false)
                .with_file(false),
        )
        .init();
}
