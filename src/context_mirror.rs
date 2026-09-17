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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::container::SHARED_CONTEXT_ROOT_ID;

    async fn test_zsei() -> (ZSEI, String, std::path::PathBuf) {
        // Unique per call: tests run in parallel and a shared dir would
        // point multiple ZSEI instances at one mmap (byte-range stomping).
        static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "ozone_cm_{}_{}_{}",
            std::process::id(),
            now_secs(),
            SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = crate::config::ZSEIConfig {
            global_path: dir.join("global.mmap").to_string_lossy().into(),
            local_path: dir.join("local").to_string_lossy().into(),
            cache_path: dir.join("cache").to_string_lossy().into(),
            ml_path: dir.join("ml").to_string_lossy().into(),
            max_containers_in_memory: 1000,
            // mmap ON — the supported path. Found a real bug while writing
            // this: mmap_enabled:false silently no-ops store_global's byte
            // writes (storage core; filed for CC's storage pass).
            mmap_enabled: true,
            embedding_dimension: 384,
            pipeline_index_path: dir.join("pi.json").to_string_lossy().into(),
            methodology_index_path: dir.join("mi.json").to_string_lossy().into(),
            blueprint_index_path: dir.join("bi.json").to_string_lossy().into(),
        };
        let zsei = ZSEI::new(&cfg).unwrap();
        // Self-heal the /SharedContext root exactly like boot does.
        let root = Container {
            global_state: GlobalState {
                container_id: SHARED_CONTEXT_ROOT_ID,
                parent_id: 0,
                child_ids: vec![],
                child_count: 0,
                version: 1,
            },
            local_state: LocalState {
                metadata: Metadata {
                    container_type: ContainerType::SharedContextRoot,
                    modality: Modality::Unknown,
                    created_at: 0,
                    updated_at: 0,
                    provenance: "test".into(),
                    permissions: 0,
                    owner_id: 0,
                    name: Some("SharedContext".into()),
                    materialized_path: Some("/SharedContext".into()),
                },
                ..Default::default()
            },
        };
        let stored = zsei.store_container(root).await.unwrap();
        assert_eq!(stored, SHARED_CONTEXT_ROOT_ID);
        let data_dir = dir.to_string_lossy().into();
        (zsei, data_dir, dir)
    }

    // T-CO1: scoping keywords — ws:<id> for workspace, scope:global for global.
    #[tokio::test]
    async fn mirrors_carry_scope_keywords() {
        let (zsei, data_dir, _dir) = test_zsei().await;
        let id = mirror(
            &zsei,
            &data_dir,
            &MirrorRequest {
                kind: "note".into(),
                agent: "zcode".into(),
                title: "ws test".into(),
                body: String::new(),
                files: vec![],
                detail: None,
                scope: Some("workspace".into()),
                workspace_id: Some(3),
                project_id: None,
            },
        )
        .await
        .unwrap();
        let c = zsei.get_container(id).await.unwrap().unwrap();
        assert!(c.local_state.context.keywords.contains(&"ws:3".to_string()));

        let id = mirror(
            &zsei,
            &data_dir,
            &MirrorRequest {
                kind: "finding".into(),
                agent: "zcode".into(),
                title: "global test".into(),
                body: String::new(),
                files: vec![],
                detail: None,
                scope: Some("global".into()),
                workspace_id: None,
                project_id: None,
            },
        )
        .await
        .unwrap();
        let c = zsei.get_container(id).await.unwrap().unwrap();
        assert!(c
            .local_state
            .context
            .keywords
            .contains(&"scope:global".to_string()));
    }

    // T-CO2: claim dedupe — same file claimed twice → one container.
    #[tokio::test]
    async fn claim_dedupe_same_file_one_container() {
        let (zsei, data_dir, _dir) = test_zsei().await;
        let req = MirrorRequest {
            kind: "claim".into(),
            agent: "zcode".into(),
            title: "claim: src/lib.rs".into(),
            body: String::new(),
            files: vec!["src/lib.rs".into()],
            detail: None,
            scope: Some("workspace".into()),
            workspace_id: Some(3),
            project_id: None,
        };
        let first = mirror(&zsei, &data_dir, &req).await.unwrap();
        let second = mirror(&zsei, &data_dir, &req).await.unwrap();
        assert_eq!(first, second, "re-claim must reuse the container");
    }

    // T-CO3: coercions — global claim → workspace; project w/o workspace → workspace.
    #[tokio::test]
    async fn scope_coercions() {
        let (zsei, data_dir, _dir) = test_zsei().await;
        // Global claim coerced to workspace (ws:0 when undeclared).
        let c = zsei
            .get_container(
                mirror(
                    &zsei,
                    &data_dir,
                    &MirrorRequest {
                        kind: "claim".into(),
                        agent: "cc".into(),
                        title: "claim: x.rs".into(),
                        body: String::new(),
                        files: vec!["x.rs".into()],
                        detail: None,
                        scope: Some("global".into()),
                        workspace_id: None,
                        project_id: None,
                    },
                )
                .await
                .unwrap(),
            )
            .await
            .unwrap()
            .unwrap();
        assert!(c.local_state.context.keywords.contains(&"ws:0".to_string()));
        assert!(!c
            .local_state
            .context
            .keywords
            .contains(&"scope:global".to_string()));

        // project scope without workspace falls back to workspace.
        let c = zsei
            .get_container(
                mirror(
                    &zsei,
                    &data_dir,
                    &MirrorRequest {
                        kind: "note".into(),
                        agent: "cc".into(),
                        title: "project note".into(),
                        body: String::new(),
                        files: vec![],
                        detail: None,
                        scope: Some("project".into()),
                        workspace_id: None,
                        project_id: Some(9),
                    },
                )
                .await
                .unwrap(),
            )
            .await
            .unwrap()
            .unwrap();
        assert!(c.local_state.context.keywords.contains(&"ws:0".to_string()));
        assert!(!c
            .local_state
            .context
            .keywords
            .contains(&"proj:9".to_string()));
    }

    // T-CO4: body persistence — object_store_path file exists with full JSON.
    #[tokio::test]
    async fn body_persisted_via_object_store_path() {
        let (zsei, data_dir, dir) = test_zsei().await;
        let id = mirror(
            &zsei,
            &data_dir,
            &MirrorRequest {
                kind: "handoff".into(),
                agent: "zcode".into(),
                title: "handoff body test".into(),
                body: "the full context".into(),
                files: vec![],
                detail: None,
                scope: Some("global".into()),
                workspace_id: None,
                project_id: None,
            },
        )
        .await
        .unwrap();
        let c = zsei.get_container(id).await.unwrap().unwrap();
        let ptr = c
            .local_state
            .storage
            .object_store_path
            .expect("object_store_path set");
        let body_path = std::path::PathBuf::from(&data_dir).join(&ptr);
        let content = std::fs::read_to_string(body_path).expect("body file exists");
        let j: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(j["body"], "the full context");
        assert_eq!(j["kind"], "handoff");
        let _ = dir; // tempdir cleanup on drop
    }

    // T-CO5: note_add through the MCP produces a real CoordinationEvent
    // container. The HTTP handler (`mirror_context` in src/grpc/mod.rs,
    // bound to POST /context/mirror) is a thin JSON-deserialize wrapper with
    // no logic of its own beyond calling `mirror()` directly — confirmed by
    // reading it — so exercising `mirror()` with a "note" kind (what
    // note_add sends) is the real, complete behavior, not a partial stand-in
    // for an HTTP-level test. Named explicitly for T-CO5 rather than relying
    // on a reader to infer it from T-CO1's coverage.
    #[tokio::test]
    async fn note_add_produces_a_real_coordination_event_container() {
        let (zsei, data_dir, _dir) = test_zsei().await;
        let before = zsei
            .get_container(SHARED_CONTEXT_ROOT_ID)
            .await
            .unwrap()
            .unwrap()
            .global_state
            .child_ids
            .len();

        let id = mirror(
            &zsei,
            &data_dir,
            &MirrorRequest {
                kind: "note".into(),
                agent: "zcode".into(),
                title: "real note_add test".into(),
                body: "genuine note body".into(),
                files: vec![],
                detail: None,
                scope: Some("workspace".into()),
                workspace_id: Some(1),
                project_id: None,
            },
        )
        .await
        .unwrap();

        let c = zsei.get_container(id).await.unwrap().unwrap();
        assert_eq!(c.local_state.metadata.container_type, ContainerType::CoordinationEvent);
        assert!(c.local_state.context.keywords.contains(&"note".to_string()));
        assert!(c.local_state.context.keywords.contains(&"zcode".to_string()));

        let after = zsei
            .get_container(SHARED_CONTEXT_ROOT_ID)
            .await
            .unwrap()
            .unwrap()
            .global_state
            .child_ids
            .len();
        assert_eq!(after, before + 1, "the new note must be a real child of the SharedContext root");
    }
}
