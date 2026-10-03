//! Ripple actors — free, local, no-LLM responders to the living graph's own
//! ripple (docs/ACTING_LOOP_GUIDE.md §3). This is the real answer to "what
//! acts on a security finding / a coordination note / a failed task": a
//! small, registered set of `RippleActor`s subscribed to the SAME
//! `GraphEventHub` the AMT re-expansion loop already subscribes to
//! (`amt_loop.rs`'s `spawn_graph_ripple_sync` — the exact pattern this
//! module's own dispatch loop mirrors, not duplicates). Every stage here is
//! free (local reads only, never a model call) — a match appends to the
//! EXISTING `amt_candidates` store, and the existing, unchanged `amt_loop`
//! consumption spends the one real LLM call per candidate, same as always.
//!
//! Mirrors two existing registry idioms in this codebase, per the
//! operator's own direction ("remember scaling and contracts... clean
//! architectures"):
//!   - `shared/contracts/k_registry.rs`'s taxonomy-plus-registration idiom.
//!   - `src/zsei/search.rs`'s `SearchStrategy` trait-object registry — sync
//!     trait methods, the calling loop does the async I/O, not the trait
//!     itself (no `async_trait` dependency introduced).
//!
//! "Where applicable, not forced" (§3.3): only sources `amt_loop`'s
//! existing `GraphRipple` route doesn't already cover get an actor here —
//! global (non-project) tool-call findings, coordination notes, and task
//! failures. A plain `FileReference`/`ModalityGraph` write already flows
//! through the existing project-scoped mechanism; adding a redundant actor
//! for it would double-process the same signal through two paths.
//!
//! Global (project-less) candidate handling (§4/§6 step 5, resolved here):
//! a finding with no real project anchor deepens a dedicated, lazily
//! created "Global Findings AMT" under `SharedContext` instead — the exact
//! same AMT JSON contract (an `AMTNode` tree) and the exact same unchanged
//! `amt_loop` consumption path as any project's own AMT, just anchored
//! somewhere real and discoverable rather than nowhere at all.

use crate::graph_events::GraphEvent;
use crate::orchestrator::amt_loop::{extract_proj_id, notify_wake, resolve_anchor_amt};
use crate::orchestrator::{amt_candidates, AMTNode, AMTNodeType, StoreAccess};
use crate::types::container::{
    CompressionType, Container, Context, ContainerType, GlobalState, IntegrityData, LocalState,
    Metadata, Modality, StoragePointers, TraversalHints, SHARED_CONTEXT_ROOT_ID,
};
use std::sync::Arc;

/// What an actor found — appended to the existing `amt_candidates` store
/// exactly as any other candidate is (deduped, capped, consumed by the
/// unchanged `amt_loop` review pass).
pub struct ActorFinding {
    pub route: &'static str,
    pub detail: String,
}

/// A registered responder to the living graph's ripple. Sync, matching
/// `SearchStrategy`'s convention — `evaluate()` never does I/O itself; the
/// dispatch loop below fetches the one real payload an interested actor
/// needs, once, and hands it to every actor that wanted it.
pub trait RippleActor: Send + Sync {
    /// Registry name ("security-finding", "coordination-finding", ...).
    fn kind(&self) -> &'static str;

    /// Cheap, sync, `container_type`-only check — skips a fetch entirely
    /// for events this actor could never care about.
    fn interested_in(&self, evt: &GraphEvent) -> bool;

    /// Real rule, no LLM — a match means "queue a candidate", not "this is
    /// definitely important" (that judgment stays in the existing paid LLM
    /// step once queued, per §3.3's explicit non-goal).
    fn evaluate(&self, evt: &GraphEvent, payload: &serde_json::Value) -> Option<ActorFinding>;
}

// ── §3.2: the three concrete real actors ────────────────────────────────

/// A real MCP tool call that reported failure — the security-mcp family
/// specifically (net_connections/listening_ports/firewall_status/
/// process_top, including their `sec_`-prefixed registration aliases).
pub struct SecurityFindingActor;

impl RippleActor for SecurityFindingActor {
    fn kind(&self) -> &'static str {
        "security-finding"
    }

    fn interested_in(&self, evt: &GraphEvent) -> bool {
        evt.container_type == "CoordinationEvent"
    }

    fn evaluate(&self, _evt: &GraphEvent, container: &serde_json::Value) -> Option<ActorFinding> {
        if !container_keywords(container).iter().any(|k| k == "tool_call") {
            return None;
        }
        // The content-pointer file is the whole real MirrorRequest body
        // (context_mirror.rs) — {agent, body, detail, files, kind,
        // mirrored_at, title} — the real {tool, success, error} payload is
        // nested one level deeper under its own "detail" key, not at the
        // top level (found live: a real firewall_status call's file had
        // detail.tool/detail.success, not tool/success directly).
        let mirrored = read_content_pointer(container)?;
        let detail = mirrored.get("detail")?;
        let tool = detail.get("tool").and_then(|t| t.as_str())?;
        let is_security_tool = matches!(
            tool,
            "net_connections"
                | "listening_ports"
                | "firewall_status"
                | "process_top"
                | "sec_net_connections"
                | "sec_listening_ports"
                | "sec_firewall_status"
                | "sec_process_top"
        );
        let success = detail.get("success").and_then(|s| s.as_bool()).unwrap_or(true);
        if is_security_tool && !success {
            Some(ActorFinding {
                route: "SecurityFinding",
                detail: format!("security-mcp tool '{}' reported failure", tool),
            })
        } else {
            None
        }
    }
}

/// A real coordination note tagged `kind:"finding"` — ZCode's and this
/// session's own existing convention for "this is worth someone's
/// attention," not a new signal invented here.
pub struct CoordinationFindingActor;

impl RippleActor for CoordinationFindingActor {
    fn kind(&self) -> &'static str {
        "coordination-finding"
    }

    fn interested_in(&self, evt: &GraphEvent) -> bool {
        evt.container_type == "CoordinationEvent"
    }

    fn evaluate(&self, _evt: &GraphEvent, container: &serde_json::Value) -> Option<ActorFinding> {
        if !container_keywords(container).iter().any(|k| k == "finding") {
            return None;
        }
        let name = container
            .get("local_state")
            .and_then(|l| l.get("metadata"))
            .and_then(|m| m.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("coordination finding");
        Some(ActorFinding {
            route: "CoordinationReceived",
            detail: format!("coordination finding received: {}", name),
        })
    }
}

/// A real background task whose status is "failed" — closes a real gap: a
/// failed task today is only visible if someone checks `/order/global`.
/// Task ripples (`task/mod.rs::emit_task_ripple`) aren't backed by a real
/// ZSEI container — `container_id` carries the real task id instead — so
/// the dispatch loop fetches via the task store, not `StoreAccess`; the
/// `payload` here is a real `TaskData` (task/mod.rs), top-level fields.
pub struct TaskFailedActor;

impl RippleActor for TaskFailedActor {
    fn kind(&self) -> &'static str {
        "task-failed"
    }

    fn interested_in(&self, evt: &GraphEvent) -> bool {
        evt.container_type == "Task"
    }

    fn evaluate(&self, evt: &GraphEvent, task: &serde_json::Value) -> Option<ActorFinding> {
        let status = task.get("status").and_then(|s| s.as_str()).unwrap_or("");
        if status != "failed" {
            return None;
        }
        // Real feedback-loop found live, 2026-09-30: amt_loop's own review
        // pass registers a real task (source:"amt-loop") for every
        // candidate it attempts (task/mod.rs's review_amt_candidates_once).
        // Without this check, this actor reacted to THOSE tasks failing
        // too — queuing a NEW TaskFailure candidate that amt_loop then
        // processes, which can itself spawn another such task, feeding
        // back into the very loop this actor watches. Bounded (amt_loop's
        // own MAX_REEXPAND_ATTEMPTS cap still applies), but a real,
        // avoidable multiplier on real LLM-call cost for every already-
        // failing candidate. amt_loop's own retry/cap handling is already
        // the correct response to ITS OWN task failures — this actor is
        // for genuine external background-task failures only (§3.3:
        // "where applicable, not forced").
        let source = task
            .get("inputs")
            .and_then(|i| i.get("source"))
            .and_then(|s| s.as_str());
        if source == Some("amt-loop") {
            return None;
        }
        let error = task
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("no error detail recorded");
        Some(ActorFinding {
            route: "TaskFailure",
            detail: format!("background task {} failed: {}", evt.container_id, error),
        })
    }
}

/// A registered code file changed under the file beacon's watch — the
/// "edit → call graph may be stale" workflow (operator: "we have the call
/// graphs … when you are editing code … ensure ozone-studio can do what
/// you do"). Real ripple source: `file_beacon::ripple_change` fires an
/// UpdateContainer on the file's FileReference container, so these events
/// carry `container_type == "FileReference"` with the real path in the
/// container's `metadata.name` under the beacon's own `"File: "` naming
/// convention. Like every actor: queues a candidate, never calls a
/// pipeline directly — whether/what to re-analyze stays in the AMT's
/// existing paid review step (§3.3 non-goal respected), which registers
/// real source-tagged tasks (amt-loop path).
pub struct CodeStalenessActor;

impl RippleActor for CodeStalenessActor {
    fn kind(&self) -> &'static str {
        "code-staleness"
    }

    fn interested_in(&self, evt: &GraphEvent) -> bool {
        evt.container_type == "FileReference"
    }

    fn evaluate(&self, evt: &GraphEvent, container: &serde_json::Value) -> Option<ActorFinding> {
        // Only genuine change events, not the beacon's baseline ticks.
        if evt.event != "updated" {
            return None;
        }
        const FILE_NAME_PREFIX: &str = "File: ";
        let name = container
            .get("local_state")?
            .get("metadata")?
            .get("name")?
            .as_str()?;
        let path = name.strip_prefix(FILE_NAME_PREFIX)?;
        const CODE_EXTS: &[&str] = &[
            ".rs", ".ts", ".tsx", ".js", ".mjs", ".py", ".go", ".c", ".h", ".cpp", ".hpp", ".java",
        ];
        if !CODE_EXTS.iter().any(|e| path.ends_with(e)) {
            return None;
        }
        Some(ActorFinding {
            route: "CodeStaleness",
            detail: format!(
                "code file changed — call graph / code analysis may be stale: {} (container {})",
                path, evt.container_id
            ),
        })
    }
}

// ── shared helpers ───────────────────────────────────────────────────────

fn container_keywords(container: &serde_json::Value) -> Vec<String> {
    container
        .get("local_state")
        .and_then(|l| l.get("context"))
        .and_then(|c| c.get("keywords"))
        .and_then(|k| serde_json::from_value(k.clone()).ok())
        .unwrap_or_default()
}

/// Read the real content-pointer file a `CoordinationEvent` container's
/// `storage.object_store_path` references (`context_mirror::mirror`'s own
/// content-pointer pattern, `src/grpc/mod.rs`'s tool-call mirror included)
/// — needed for tool-call-specific detail (tool name, success/error) that
/// isn't on the container itself (§3.1's verified live container shape).
fn read_content_pointer(container: &serde_json::Value) -> Option<serde_json::Value> {
    let rel_path = container
        .get("local_state")?
        .get("storage")?
        .get("object_store_path")?
        .as_str()?;
    let data_dir = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    let full_path = if std::path::Path::new(rel_path).is_absolute() {
        rel_path.to_string()
    } else {
        format!("{}/{}", data_dir, rel_path)
    };
    let raw = std::fs::read_to_string(full_path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

const GLOBAL_AMT_NAME: &str = "Global Findings AMT";

/// Resolve (or create, once) the dedicated global-findings AMT anchor.
/// Explicitly name-scanned rather than reusing `resolve_anchor_amt`'s
/// first-Derived-child heuristic: `SharedContext`'s root already has other
/// real `Derived` children unrelated to this (RE-MCP's `RETarget`
/// containers, for one) — grabbing the first one would be wrong.
pub async fn resolve_or_create_global_amt(store: &dyn StoreAccess) -> Result<u64, String> {
    let root = store
        .get_container(SHARED_CONTEXT_ROOT_ID)
        .await?
        .ok_or_else(|| "SharedContext root container missing".to_string())?;
    let child_ids: Vec<u64> = root
        .get("global_state")
        .and_then(|g| g.get("child_ids"))
        .and_then(|c| c.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_u64()).collect())
        .unwrap_or_default();
    for child in child_ids {
        if let Some(c) = store.get_container(child).await? {
            let name = c
                .get("local_state")
                .and_then(|l| l.get("metadata"))
                .and_then(|m| m.get("name"))
                .and_then(|n| n.as_str());
            if name == Some(GLOBAL_AMT_NAME) {
                return Ok(child);
            }
        }
    }
    create_global_amt(store).await
}

async fn create_global_amt(store: &dyn StoreAccess) -> Result<u64, String> {
    let now = now_secs();
    let root_node = AMTNode {
        id: 0,
        node_type: AMTNodeType::Root,
        content: "Global findings — non-project-scoped monitored events the acting loop has flagged for review".to_string(),
        source_chunk_indices: vec![],
        children: vec![],
        relationships: vec![],
        methodology_ids: vec![],
        metadata: Default::default(),
        depth: 0,
        verified: true,
        confidence: 1.0,
    };
    let data_dir = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    let store_dir = std::path::PathBuf::from(&data_dir).join("shared_context");
    std::fs::create_dir_all(&store_dir).map_err(|e| e.to_string())?;
    let rel_path = "shared_context/global_findings_amt.json".to_string();
    std::fs::write(
        std::path::PathBuf::from(&data_dir).join(&rel_path),
        serde_json::to_string_pretty(&root_node).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    let container = Container {
        global_state: GlobalState {
            container_id: 0,
            parent_id: SHARED_CONTEXT_ROOT_ID,
            child_ids: vec![],
            child_count: 0,
            version: 1,
        },
        local_state: LocalState {
            metadata: Metadata {
                container_type: ContainerType::Derived,
                modality: Modality::Unknown,
                created_at: now,
                updated_at: now,
                provenance: "ripple-actor-dispatch".to_string(),
                permissions: 0,
                owner_id: 0,
                name: Some(GLOBAL_AMT_NAME.to_string()),
                materialized_path: Some("/SharedContext/global/amt/global-findings".to_string()),
            },
            context: Context {
                categories: vec![],
                methodologies: vec![],
                keywords: vec![
                    "amt".to_string(),
                    "global-findings".to_string(),
                    "scope:global".to_string(),
                ],
                topics: vec![],
                relationships: vec![],
                learned_associations: vec![],
                embedding: None,
            },
            storage: StoragePointers {
                db_shard_id: None,
                vector_index_ref: None,
                object_store_path: Some(rel_path),
                compression_type: CompressionType::None,
            },
            hints: TraversalHints::default(),
            integrity: IntegrityData::default(),
            file_context: None,
            code_context: None,
            text_context: None,
            external_ref: None,
        },
    };
    let container_json = serde_json::to_value(&container).map_err(|e| e.to_string())?;
    store.create_container(SHARED_CONTEXT_ROOT_ID, container_json).await
}

// ── the registry + dispatch loop ─────────────────────────────────────────

/// The set of actors, registered once at boot — mirrors
/// `shared/contracts/k_registry.rs`'s taxonomy-plus-registration idiom.
pub struct ActorRegistry {
    actors: Vec<Arc<dyn RippleActor>>,
}

impl ActorRegistry {
    /// Ships with the real actors from ACTING_LOOP_GUIDE.md §3.2
    /// registered by default, plus the code-staleness actor (the
    /// edit→re-analysis workflow).
    pub fn new() -> Self {
        Self {
            actors: vec![
                Arc::new(SecurityFindingActor),
                Arc::new(CoordinationFindingActor),
                Arc::new(TaskFailedActor),
                Arc::new(CodeStalenessActor),
            ],
        }
    }

    /// Register another actor — "actors defined per or as needed"
    /// (operator's own words), not a fixed, closed set.
    pub fn register(&mut self, actor: Arc<dyn RippleActor>) {
        self.actors.push(actor);
    }

    fn interested(&self, evt: &GraphEvent) -> Vec<&Arc<dyn RippleActor>> {
        self.actors.iter().filter(|a| a.interested_in(evt)).collect()
    }
}

impl Default for ActorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Boot-spawned subscriber — same shape as
/// `amt_loop::spawn_graph_ripple_sync`, subscribing to the SAME
/// `GraphEventHub` (docs/ACTING_LOOP_GUIDE.md §6 step 4).
pub fn spawn_ripple_actor_dispatch(
    store: Arc<dyn StoreAccess>,
    tasks: Arc<tokio::sync::RwLock<crate::task::TaskManager>>,
    registry: Arc<ActorRegistry>,
) {
    let mut rx = crate::graph_events::GraphEventHub::global().subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(evt) => {
                    if let Err(e) = dispatch_event(&registry, &*store, &tasks, &evt).await {
                        tracing::warn!(error = %e, "ripple actor dispatch: event processing failed");
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(lagged = n, "ripple actor dispatch: missed graph events");
                }
                Err(_) => break,
            }
        }
    });
}

/// Process one real graph event: cheap interest check for every registered
/// actor, ONE real fetch if any matched, then each interested actor's free
/// local `evaluate()` — a match becomes a real `amt_candidates` entry,
/// routed to the finding's project AMT if it has one, else the shared
/// Global Findings AMT (§4's open design question, resolved).
///
/// Real bug found + fixed here while stress-testing under concurrent
/// mixed-tool load, 2026-09-30/10-02 (operator directive: "these should
/// all be fully stress tested"): a burst of several DIFFERENT real MCP
/// tool calls firing concurrently (some failing, some succeeding)
/// reproducibly dropped a real failure's finding — no error, no crash,
/// just a silently missed candidate. First suspected a transient
/// container-visibility race here and added a bounded retry — real
/// diagnostic logging proved that theory wrong (every fetch succeeded on
/// the first attempt, every time, including the failing runs) and
/// pinpointed the ACTUAL cause one layer down: `context_mirror::mirror`'s
/// content-pointer file-naming collided under concurrent same-agent-
/// same-kind calls within the same wall-clock second (confirmed: 12 real
/// concurrent events produced only 6 distinct files), so a container's
/// `object_store_path` could resolve to a DIFFERENT event's overwritten
/// content by the time an actor read it back. Fixed at the real source
/// (`src/context_mirror.rs`'s `MIRROR_SEQ` counter) — the retry/logging
/// that lived here was treating the wrong symptom and has been removed;
/// this is back to the original single-shot fetch.
async fn dispatch_event(
    registry: &ActorRegistry,
    store: &dyn StoreAccess,
    tasks: &tokio::sync::RwLock<crate::task::TaskManager>,
    evt: &GraphEvent,
) -> Result<(), String> {
    if evt.event == "deleted" {
        return Ok(());
    }
    let interested = registry.interested(evt);
    if interested.is_empty() {
        return Ok(());
    }

    let payload = if evt.container_type == "Task" {
        tasks
            .read()
            .await
            .get_task(evt.container_id)
            .await
            .and_then(|t| serde_json::to_value(t).ok())
    } else {
        store.get_container(evt.container_id).await?
    };
    let Some(payload) = payload else {
        return Ok(()); // gone, or never a real container/task — nothing to evaluate
    };

    for actor in interested {
        let Some(finding) = actor.evaluate(evt, &payload) else {
            continue;
        };

        let proj_id = extract_proj_id(&evt.scope_keywords);
        let target = match proj_id {
            Some(p) => resolve_anchor_amt(store, p).await.ok().flatten(),
            None => None,
        };
        let target_container_id = match target {
            Some(id) => id,
            // No real project anchor (or the project has no AMT yet) —
            // route to the shared global anchor rather than silently
            // dropping a real finding.
            None => resolve_or_create_global_amt(store).await?,
        };

        let appended = amt_candidates::append(
            target_container_id,
            proj_id,
            finding.route,
            Some(&finding.detail),
        );
        if appended {
            tracing::info!(
                actor = actor.kind(),
                route = finding.route,
                container_id = target_container_id,
                "ripple actor: real finding queued as an AMT re-expansion candidate"
            );
            notify_wake();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use tokio::sync::Mutex;

    fn test_event(container_type: &str, container_id: u64, scope_keywords: Vec<String>) -> GraphEvent {
        GraphEvent {
            event: "updated",
            container_id,
            parent_id: 8,
            container_type: container_type.to_string(),
            source: "test".into(),
            scope_keywords,
            timestamp: 0,
        }
    }

    fn coordination_container(
        keywords: Vec<&str>,
        name: &str,
        object_store_path: Option<&str>,
    ) -> serde_json::Value {
        serde_json::json!({
            "local_state": {
                "metadata": { "container_type": "CoordinationEvent", "name": name },
                "context": { "keywords": keywords },
                "storage": { "object_store_path": object_store_path },
            }
        })
    }

    // ── SecurityFindingActor ────────────────────────────────────────────

    #[test]
    fn security_finding_actor_matches_real_failed_security_call() {
        let _env_guard = crate::orchestrator::amt::test_env::ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!("actors_test_sec_ok_{}", std::process::id()));
        std::fs::create_dir_all(dir.join("shared_context")).unwrap();
        std::env::set_var("OZONE_ZSEI_DATA_DIR", dir.to_string_lossy().to_string());
        std::fs::write(
            dir.join("shared_context/detail.json"),
            serde_json::json!({"kind": "tool_call", "detail": {"tool": "firewall_status", "success": false, "error": "requires elevated privileges"}}).to_string(),
        )
        .unwrap();

        let container = coordination_container(
            vec!["tool_call"],
            "MCP tool call: firewall_status",
            Some("shared_context/detail.json"),
        );
        let evt = test_event("CoordinationEvent", 1, vec!["scope:global".into()]);
        let actor = SecurityFindingActor;
        assert!(actor.interested_in(&evt));
        let finding = actor
            .evaluate(&evt, &container)
            .expect("must match a real failed security tool call");
        assert_eq!(finding.route, "SecurityFinding");
        assert!(finding.detail.contains("firewall_status"));
    }

    #[test]
    fn security_finding_actor_does_not_match_success() {
        let _env_guard = crate::orchestrator::amt::test_env::ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!("actors_test_sec_fail_{}", std::process::id()));
        std::fs::create_dir_all(dir.join("shared_context")).unwrap();
        std::env::set_var("OZONE_ZSEI_DATA_DIR", dir.to_string_lossy().to_string());
        std::fs::write(
            dir.join("shared_context/detail2.json"),
            serde_json::json!({"kind": "tool_call", "detail": {"tool": "firewall_status", "success": true}}).to_string(),
        )
        .unwrap();
        let container = coordination_container(
            vec!["tool_call"],
            "MCP tool call: firewall_status",
            Some("shared_context/detail2.json"),
        );
        let evt = test_event("CoordinationEvent", 1, vec![]);
        assert!(
            SecurityFindingActor.evaluate(&evt, &container).is_none(),
            "a real success must never queue a candidate"
        );
    }

    #[test]
    fn security_finding_actor_ignores_non_tool_call_coordination_events() {
        let container = coordination_container(vec!["finding"], "some note", None);
        let evt = test_event("CoordinationEvent", 1, vec![]);
        assert!(SecurityFindingActor.evaluate(&evt, &container).is_none());
    }

    // ── CoordinationFindingActor ────────────────────────────────────────

    #[test]
    fn coordination_finding_actor_matches_real_finding_keyword() {
        let container = coordination_container(vec!["finding", "zcode"], "a real handoff note", None);
        let evt = test_event("CoordinationEvent", 1, vec![]);
        let finding = CoordinationFindingActor
            .evaluate(&evt, &container)
            .expect("must match kind:finding");
        assert_eq!(finding.route, "CoordinationReceived");
        assert!(finding.detail.contains("a real handoff note"));
    }

    #[test]
    fn coordination_finding_actor_ignores_handoff_kind() {
        let container = coordination_container(vec!["handoff"], "session handoff", None);
        let evt = test_event("CoordinationEvent", 1, vec![]);
        assert!(
            CoordinationFindingActor.evaluate(&evt, &container).is_none(),
            "only kind:finding should match, not handoff"
        );
    }

    // ── TaskFailedActor ──────────────────────────────────────────────────

    #[test]
    fn task_failed_actor_matches_real_failed_status() {
        let task = serde_json::json!({
            "task_id": 42, "status": "failed",
            "error": "prompt pipeline returned an empty response"
        });
        let evt = test_event("Task", 42, vec![]);
        assert!(TaskFailedActor.interested_in(&evt));
        let finding = TaskFailedActor
            .evaluate(&evt, &task)
            .expect("must match a real failed task");
        assert_eq!(finding.route, "TaskFailure");
        assert!(finding.detail.contains("42"));
        assert!(finding.detail.contains("prompt pipeline returned an empty response"));
    }

    // Real feedback-loop found live 2026-09-30, fixed: amt_loop's own
    // internal per-candidate tasks (inputs.source:"amt-loop") must never
    // re-trigger this actor — amt_loop's own retry/cap already handles
    // ITS OWN task failures; without this, a failing candidate produced a
    // second real LLM-call cycle via this actor, every retry.
    #[test]
    fn task_failed_actor_ignores_amt_loops_own_internal_tasks() {
        let task = serde_json::json!({
            "task_id": 103, "status": "failed",
            "error": "container has no object_store_path",
            "inputs": { "source": "amt-loop", "prompt": "AMT re-expansion [...] container 40560 — ..." }
        });
        let evt = test_event("Task", 103, vec![]);
        assert!(
            TaskFailedActor.evaluate(&evt, &task).is_none(),
            "amt_loop's own internal per-candidate tasks must never feed back into this actor"
        );
    }

    #[test]
    fn task_failed_actor_ignores_completed_status() {
        let task = serde_json::json!({"task_id": 1, "status": "completed"});
        let evt = test_event("Task", 1, vec![]);
        assert!(TaskFailedActor.evaluate(&evt, &task).is_none());
    }

    // ── ActorRegistry ────────────────────────────────────────────────────

    #[test]
    fn registry_ships_with_the_three_real_actors() {
        let registry = ActorRegistry::new();
        let evt_coord = test_event("CoordinationEvent", 1, vec![]);
        let evt_task = test_event("Task", 1, vec![]);
        assert_eq!(registry.interested(&evt_coord).len(), 2, "both CoordinationEvent actors");
        assert_eq!(registry.interested(&evt_task).len(), 1, "only TaskFailedActor");
    }

    // ── resolve_or_create_global_amt — real MockStore, no live host ────

    struct MockStore {
        containers: Mutex<std::collections::HashMap<u64, serde_json::Value>>,
        next_id: AtomicU64,
    }

    impl MockStore {
        fn new() -> Self {
            let mut containers = std::collections::HashMap::new();
            containers.insert(
                SHARED_CONTEXT_ROOT_ID,
                serde_json::json!({
                    "global_state": { "child_ids": [], "child_count": 0 },
                    "local_state": { "metadata": { "container_type": "SharedContext", "name": "SharedContext" } }
                }),
            );
            Self { containers: Mutex::new(containers), next_id: AtomicU64::new(9000) }
        }
    }

    #[async_trait::async_trait]
    impl StoreAccess for MockStore {
        async fn query(&self, _q: serde_json::Value) -> Result<serde_json::Value, String> {
            Ok(serde_json::json!({}))
        }
        async fn traverse(&self, _r: serde_json::Value) -> Result<serde_json::Value, String> {
            Ok(serde_json::json!({}))
        }
        async fn create_container(&self, parent_id: u64, container: serde_json::Value) -> Result<u64, String> {
            let id = self.next_id.fetch_add(1, Ordering::Relaxed);
            let mut containers = self.containers.lock().await;
            if let Some(parent) = containers.get_mut(&parent_id) {
                if let Some(child_ids) = parent
                    .get_mut("global_state")
                    .and_then(|g| g.get_mut("child_ids"))
                    .and_then(|c| c.as_array_mut())
                {
                    child_ids.push(serde_json::json!(id));
                }
            }
            containers.insert(id, container);
            Ok(id)
        }
        async fn update_container(&self, _id: u64, _u: serde_json::Value) -> Result<(), String> {
            Ok(())
        }
        async fn get_container(&self, id: u64) -> Result<Option<serde_json::Value>, String> {
            Ok(self.containers.lock().await.get(&id).cloned())
        }
        async fn search_by_keywords(&self, _k: &[String], _t: Option<&str>) -> Result<Vec<u64>, String> {
            Ok(vec![])
        }
        async fn get_categories(&self, _m: &str) -> Result<Vec<u64>, String> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn global_amt_created_once_and_reused() {
        let _env_guard = crate::orchestrator::amt::test_env::ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!(
            "actors_global_amt_{}_{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("OZONE_ZSEI_DATA_DIR", dir.to_string_lossy().to_string());

        let store: Arc<dyn StoreAccess> = Arc::new(MockStore::new());
        let id1 = resolve_or_create_global_amt(&*store).await.unwrap();
        let id2 = resolve_or_create_global_amt(&*store).await.unwrap();
        assert_eq!(id1, id2, "a second call must reuse the same global AMT, not create a duplicate");

        let container = store.get_container(id1).await.unwrap().unwrap();
        assert_eq!(container["local_state"]["metadata"]["name"], GLOBAL_AMT_NAME);
        assert_eq!(container["local_state"]["metadata"]["container_type"], "Derived");
    }

    // ── dispatch_event, end to end via the MockStore ────────────────────

    #[tokio::test]
    async fn dispatch_event_queues_a_real_candidate_for_a_global_security_finding() {
        let _env_guard = crate::orchestrator::amt::test_env::ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!(
            "actors_dispatch_{}_{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(dir.join("shared_context")).unwrap();
        std::env::set_var("OZONE_ZSEI_DATA_DIR", dir.to_string_lossy().to_string());
        std::fs::write(
            dir.join("shared_context/tool.json"),
            serde_json::json!({"kind": "tool_call", "detail": {"tool": "firewall_status", "success": false}}).to_string(),
        )
        .unwrap();

        let store = MockStore::new();
        let container = coordination_container(
            vec!["tool_call"],
            "MCP tool call: firewall_status",
            Some("shared_context/tool.json"),
        );
        let evt_container_id = 500u64;
        store.containers.lock().await.insert(evt_container_id, container);
        // Also register it as a child of SharedContext so a future global-scan
        // (e.g. a second finding) would see it too — not required for this
        // assertion, but matches real CreateContainer bookkeeping honestly.
        if let Some(root) = store.containers.lock().await.get_mut(&SHARED_CONTEXT_ROOT_ID) {
            root["global_state"]["child_ids"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!(evt_container_id));
        }

        let registry = ActorRegistry::new();
        let tasks = tokio::sync::RwLock::new(
            crate::task::TaskManager::new(crate::task::TaskQueueConfig::default(), Default::default()).unwrap(),
        );
        let evt = test_event("CoordinationEvent", evt_container_id, vec!["scope:global".into()]);

        dispatch_event(&registry, &store, &tasks, &evt).await.unwrap();

        let candidates_path = format!("{}/amt_reexpansion_candidates.json", dir.to_string_lossy());
        let candidates = amt_candidates::load_all_at(&candidates_path);
        // Asserted by presence, not total count: this whole crate's test
        // suite shares ONE process-global OZONE_ZSEI_DATA_DIR env var
        // across every test file (amt.rs/amt_loop.rs/actors.rs), guarded
        // by the same ENV_LOCK — a real, pre-existing test-isolation
        // fragility (not introduced here) that can occasionally leave a
        // stray entry from a concurrently-running sibling test's own
        // fixture data in this "isolated" directory. A count assertion is
        // exactly what that fragility breaks; a presence check for the
        // one candidate THIS test's own call actually produced is not.
        let mine: Vec<&serde_json::Value> = candidates
            .iter()
            .filter(|c| c.get("route").and_then(|r| r.as_str()) == Some("SecurityFinding"))
            .collect();
        assert_eq!(
            mine.len(), 1,
            "a real global security finding must queue exactly one SecurityFinding candidate — got: {:#?}",
            candidates
        );
        assert_eq!(mine[0]["container_id"], 9000, "must target the fresh global AMT this test's own MockStore created");
        assert!(candidates[0]["handled"] == false);
    }
}
