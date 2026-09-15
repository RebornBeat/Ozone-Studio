//! Shared-context mirroring — coordination state as real ZSEI containers.
//!
//! The shared-context tool (tools/ozone-shared-context) keeps canonical
//! coordination state in `.ozone-context/` and mirrors events into the
//! host's monitor feed. This module completes task 42: every mirrored note,
//! decision, handoff, and file claim ALSO becomes a first-class ZSEI
//! container under the /SharedContext root — searchable by keyword (agent,
//! kind, `file:<path>`), stored as real graph nodes the orchestrator and
//! AMT can traverse, exactly like jurisdictions and methodologies.
//!
//! Idempotence: notes are unique events (mirrored once — the caller mirrors
//! on write); file claims carry a `claim:<path>` dedupe keyword so
//! re-claims of the same file update-or-reuse one container instead of
//! spamming the graph.
//!
//! Body persistence: the full event JSON is written to
//! `<data_dir>/shared_context/<dedupe-or-id>.json` and referenced via the
//! container's object_store_path — the same content-pointer pattern the
//! jurisdiction rule sets use.

use crate::types::container::{
    Container, Context, GlobalState, IntegrityData, LocalState, Metadata, Modality,
    StoragePointers, TraversalHints, ContainerType, SHARED_CONTEXT_ROOT_ID,
};
use crate::types::zsei::ZSEIQuery;
use crate::zsei::ZSEI;

/// One event to mirror into the coordination graph.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct MirrorRequest {
    /// note | decision | handoff | finding | claim
    pub kind: String,
    pub agent: String,
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub detail: Option<serde_json::Value>,
    /// SCOPE — the coordination graph mirrors Ozone-Studio's own scoping
    /// spine (Session.active_workspace/active_project → OrchestrateRequest
    /// .workspace_id/.project_id → context_aggregation's project-scoped +
    /// separate-layer doctrine):
    ///   "global"    host-wide — presence, host-ops, cross-workspace findings
    ///   "workspace" bound to one workspace (DEFAULT — the repo/session the
    ///               event was made in; claims are always at least this)
    ///   "project"   bound to one project inside a workspace
    /// Cross-workspace visibility is EXPLICIT only: promote to global, or
    /// another workspace calls global items in. Never silent bleed.
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<u64>,
    #[serde(default)]
    pub project_id: Option<u64>,
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn slug(s: &str, max: usize) -> String {
    let clean: String = s
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let collapsed: Vec<&str> = clean.split('-').filter(|p| !p.is_empty()).collect();
    let joined = collapsed.join("-").to_lowercase();
    let mut out = joined.as_str().to_string();
    out.truncate(max);
    out.trim_matches('-').to_string()
}

/// Resolve + validate scope. Rules:
///   • claims are always at least workspace-scoped (file paths belong to a
///     repo/workspace) — a "global" claim is coerced to workspace;
///   • "project" requires a workspace_id (a project lives inside one);
///   • unknown scope values fall back to workspace (never silently global).
fn resolve_scope(req: &MirrorRequest) -> (String, Option<u64>, Option<u64>) {
    let raw = req.scope.as_deref().unwrap_or("workspace").to_lowercase();
    let mut scope = match raw.as_str() {
        "global" => "global",
        "project" => "project",
        _ => "workspace",
    };
    let mut ws = req.workspace_id;
    let mut proj = req.project_id;

    if req.kind == "claim" && scope == "global" {
        scope = "workspace"; // file paths are workspace-bound
    }
    if scope == "project" {
        if ws.is_none() {
            // A project without its workspace is stored at workspace scope.
            scope = "workspace";
        }
        if proj.is_none() {
            scope = "workspace";
        }
    }
    if scope == "workspace" && ws.is_none() {
        // Workspace-scoped event with no declared workspace: keep the
        // scope marker but with the host-level workspace (None) — the
        // keyword becomes ws:unscoped rather than silently global.
        ws = Some(0);
    }
    (scope.to_string(), ws, proj)
}

/// Scope keywords — the graph-search encoding AMT/context queries filter on.
fn scope_keywords(scope: &str, ws: Option<u64>, proj: Option<u64>) -> Vec<String> {
    match scope {
        "global" => vec!["scope:global".to_string()],
        "project" => {
            let mut k = vec![format!("ws:{}", ws.unwrap_or(0))];
            if let Some(p) = proj {
                k.push(format!("proj:{}", p));
            }
            k
        }
        _ => vec![format!("ws:{}", ws.unwrap_or(0))],
    }
}

/// Mirror one coordination event as a CoordinationEvent container under the
/// /SharedContext root. Returns the container id (existing id when a
/// `claim:<path>` dedupe keyword matches a prior container).
pub async fn mirror(zsei: &ZSEI, data_dir: &str, req: &MirrorRequest) -> Result<u64, String> {
    let now = now_secs();

    // Dedupe key: file claims collapse to one container per path; other
    // kinds are unique events keyed by time.
    let dedupe_keyword = if req.kind == "claim" && !req.files.is_empty() {
        Some(format!("claim:{}", req.files[0]))
    } else {
        None
    };

    // Idempotence: scan the root's children for an existing dedupe keyword.
    if let Some(dk) = &dedupe_keyword {
        if let Ok(Some(root)) = zsei.get_container(SHARED_CONTEXT_ROOT_ID).await {
            for child_id in &root.global_state.child_ids {
                if let Ok(Some(child)) = zsei.get_container(*child_id).await {
                    if child.local_state.context.keywords.contains(dk) {
                        return Ok(*child_id);
                    }
                }
            }
        }
    }

    // Persist the full event body beside the store (content-pointer pattern).
    let body_json = serde_json::json!({
        "kind": req.kind,
        "agent": req.agent,
        "title": req.title,
        "body": req.body,
        "files": req.files,
        "detail": req.detail,
        "mirrored_at": now,
    });
    let store_dir = std::path::PathBuf::from(data_dir).join("shared_context");
    std::fs::create_dir_all(&store_dir)
        .map_err(|e| format!("failed to create shared_context dir: {}", e))?;
    let file_key = match &dedupe_keyword {
        Some(k) => slug(k.trim_start_matches("claim:"), 80),
        None => format!("{}-{}-{}", req.kind, req.agent, now),
    };
    let rel_path = format!("shared_context/{}.json", file_key);
    std::fs::write(
        std::path::PathBuf::from(data_dir).join(&rel_path),
        serde_json::to_string_pretty(&body_json).unwrap_or_default(),
    )
    .map_err(|e| format!("failed to write event body: {}", e))?;

    // keywords: kind, agent, file:<path> per file, scope keywords
    // (scope:global | ws:<id> | proj:<id>), + dedupe keyword hidden inside
    // the same set — scan-based idempotence reads it back.
    let (scope, ws, proj) = resolve_scope(req);
    let mut keywords: Vec<String> = vec![req.kind.to_lowercase(), req.agent.to_lowercase()];
    for f in &req.files {
        keywords.push(format!("file:{}", f));
    }
    keywords.extend(scope_keywords(&scope, ws, proj));
    if let Some(dk) = &dedupe_keyword {
        keywords.push(dk.clone());
    }
    keywords.dedup();

    let title_slug = slug(&req.title, 60);
    let scope_seg = match (scope.as_str(), ws, proj) {
        ("global", _, _) => "global".to_string(),
        ("project", w, p) => format!("proj-{}-{}", w.unwrap_or(0), p.unwrap_or(0)),
        _ => format!("ws-{}", ws.unwrap_or(0)),
    };
    let container = Container {
        global_state: GlobalState {
            container_id: 0, // allocated by CreateContainer
            parent_id: SHARED_CONTEXT_ROOT_ID,
            child_ids: vec![],
            child_count: 0,
            version: 1,
        },
        local_state: LocalState {
            metadata: Metadata {
                container_type: ContainerType::CoordinationEvent,
                modality: Modality::Unknown,
                created_at: now,
                updated_at: now,
                provenance: req.agent.clone(),
                permissions: 0,
                owner_id: 0,
                name: Some(req.title.clone()),
                materialized_path: Some(format!(
                    "/SharedContext/{}/{}/{}",
                    scope_seg, req.kind, title_slug
                )),
            },
            context: Context {
                categories: vec![],
                methodologies: vec![],
                keywords,
                topics: vec!["coordination".to_string()],
                relationships: vec![],
                learned_associations: vec![],
                embedding: None,
            },
            storage: StoragePointers {
                db_shard_id: None,
                vector_index_ref: None,
                object_store_path: Some(rel_path),
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
        .query(ZSEIQuery::CreateContainer {
            parent_id: SHARED_CONTEXT_ROOT_ID,
            container,
        })
        .await
    {
        Ok(crate::types::zsei::ZSEIQueryResult::ContainerID(id)) => Ok(id),
        Ok(_) => Err("unexpected CreateContainer result".to_string()),
        Err(e) => Err(e.to_string()),
    }
}
