//! WorkspaceTabPipeline - Pipeline #6
//! 
//! Workspace and project management UI.
//! Workspaces contain projects which contain file/URL/package references.
//! 
//! STORAGE: Uses ZSEI containers for persistence
//! - User (container_id) -> Workspaces (children of type "Workspace")
//! - Workspace (container_id) -> Projects (children of type "Project")
//! - Project (container_id) -> FileRefs, URLRefs, PackageRefs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;

#[path = "../../shared/ozone_serve.rs"]
mod ozone_serve;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum WorkspaceInput {
    /// List user's workspaces
    ListWorkspaces,
    /// Create workspace
    CreateWorkspace { name: String, description: Option<String> },
    /// Get workspace details
    GetWorkspace { workspace_id: u64 },
    /// Update workspace
    UpdateWorkspace { workspace_id: u64, name: Option<String>, description: Option<String> },
    /// Delete workspace
    DeleteWorkspace { workspace_id: u64 },
    /// List projects in workspace
    ListProjects { workspace_id: u64 },
    /// Create project
    CreateProject { workspace_id: u64, name: String, description: Option<String> },
    /// Get project details
    GetProject { project_id: u64 },
    /// Update project
    UpdateProject { project_id: u64, name: Option<String>, description: Option<String> },
    /// Delete project
    DeleteProject { project_id: u64 },
    /// Get project files
    GetProjectFiles { project_id: u64 },
    /// Get project context (aggregated)
    GetProjectContext { project_id: u64, token_budget: Option<u32> },
}

/// Workspace data - field names match UI expectations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: u64,           // UI uses 'id' not 'workspace_id'
    pub name: String,
    pub description: Option<String>,
    pub project_count: u32,
    pub created_at: u64,
    pub updated_at: u64,
    /// The real ZSEI container this workspace is backed by (2026-10-18,
    /// operator-directed unification — see the "Real ZSEI graph wiring"
    /// block below CreateProject/CreateWorkspace's helpers for why this
    /// exists). `#[serde(default)]` so every pre-existing flat record
    /// (created before this field existed) deserializes as `None` instead
    /// of failing to load — ListWorkspaces backfills it lazily on next
    /// read, no separate migration step needed.
    #[serde(default)]
    pub zsei_container_id: Option<u64>,
}

/// Project data - field names match UI expectations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: u64,           // UI uses 'id' not 'project_id'
    pub workspace_id: u64,
    pub name: String,
    pub description: Option<String>,
    pub files_count: u32,  // UI expects 'files_count' not 'file_count'
    pub url_count: u32,
    pub package_count: u32,
    pub created_at: u64,
    pub updated_at: u64,
    /// The real ZSEI container this project is backed by — a structural
    /// CHILD of its owning workspace's own zsei_container_id (see the "Real
    /// ZSEI graph wiring" block below for why that parent/child edge is
    /// what makes ui/src/projectSelection.ts's traversal find it).
    /// `#[serde(default)]` for the same backward-compat reason as
    /// Workspace.zsei_container_id — ListProjects backfills it lazily.
    #[serde(default)]
    pub zsei_container_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRef {
    pub id: u64,
    pub path: String,
    pub name: String,
    pub modality: String,
    pub size_bytes: u64,
    pub last_modified: u64,
    /// The real ZSEI FileReference container this link is wired to (written
    /// by file_link/main.rs's FileRefInfo.zsei_container_id into the same
    /// files_<project_id>.json this struct reads) — previously silently
    /// dropped here since this struct didn't declare the field, so the UI
    /// never had a real id to navigate with. `#[serde(default)]` for
    /// records written before file_link added this field.
    #[serde(default)]
    pub zsei_container_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceOutput {
    pub success: bool,
    pub workspace: Option<Workspace>,
    pub workspaces: Option<Vec<Workspace>>,
    pub project: Option<Project>,
    pub projects: Option<Vec<Project>>,
    pub files: Option<Vec<FileRef>>,
    pub context: Option<String>,
    pub error: Option<String>,
}

// ============================================================================
// Persistent Storage Layer (JSON-based for simplicity, integrates with ZSEI)
// ============================================================================

/// Storage path for workspace data
fn storage_path() -> PathBuf {
    // OZONE_DATA_PATH's bare "./data" fallback is CWD-relative — this
    // pipeline runs as a subprocess spawned by the host (no explicit
    // current_dir set at the spawn site, src/pipeline/executor.rs), so it
    // silently inherits whatever CWD the host itself happens to have. That
    // has worked by coincidence every session so far (the host is always
    // launched from target/release/), but it's the same fragile-relative-
    // path class already found and fixed 3+ times elsewhere this session
    // (amt_loop.rs, jurisdiction.rs, amt.rs — all resolved by anchoring to
    // the same OZONE_ZSEI_DATA_DIR-style stable base rather than a bare
    // relative default). Anchoring this pipeline's own flat-file storage
    // alongside the real zsei_data root, not a separate ad-hoc "./data"
    // tree, for the same stability guarantee.
    let base = std::env::var("OZONE_DATA_PATH").unwrap_or_else(|_| {
        let zsei_base = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
        format!("{}/workspace_tab_data", zsei_base)
    });
    PathBuf::from(base).join("workspaces")
}

/// Load all workspaces from storage
fn load_workspaces() -> Vec<Workspace> {
    let path = storage_path().join("workspaces.json");
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(workspaces) = serde_json::from_str(&content) {
                return workspaces;
            }
        }
    }
    vec![]
}

/// Save all workspaces to storage
fn save_workspaces(workspaces: &[Workspace]) -> Result<(), String> {
    let dir = storage_path();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create dir: {}", e))?;
    let path = dir.join("workspaces.json");
    let content = serde_json::to_string_pretty(workspaces)
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Failed to write: {}", e))?;
    Ok(())
}

/// Load projects for a workspace
fn load_projects(workspace_id: u64) -> Vec<Project> {
    let path = storage_path().join(format!("projects_{}.json", workspace_id));
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(projects) = serde_json::from_str(&content) {
                return projects;
            }
        }
    }
    vec![]
}

/// Save projects for a workspace
fn save_projects(workspace_id: u64, projects: &[Project]) -> Result<(), String> {
    let dir = storage_path();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create dir: {}", e))?;
    let path = dir.join(format!("projects_{}.json", workspace_id));
    let content = serde_json::to_string_pretty(projects)
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Failed to write: {}", e))?;
    Ok(())
}

/// Load file references for a project
fn load_file_refs(project_id: u64) -> Vec<FileRef> {
    let path = storage_path().join(format!("files_{}.json", project_id));
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(files) = serde_json::from_str(&content) {
                return files;
            }
        }
    }
    vec![]
}

/// Generate unique ID
fn generate_id() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64 % 10_000_000_000 // Keep manageable size
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
}

// ============================================================================
// Real ZSEI graph wiring (2026-10-18, operator-directed unification)
//
// Workspace-tab's own Workspace/Project records were, until now, flat JSON
// records with their own locally-generated ids (generate_id() above) — never
// real ZSEI containers. Every real-graph-native viewer (WorkspaceBrowser/
// FileReferenceViewer/FabricView/AmtTreeView/GraphView, via
// ui/src/projectSelection.ts's loadProjectOptions) finds projects by:
// ZSEIQuery::GetUserWorkspaces{user_id} (a flat scan over ALL containers for
// container_type=="Workspace" && owner_id==user_id — confirmed in
// src/zsei/query.rs's find_user_workspaces; it does NOT care about parent/
// child structure at all) -> for each result, GetContainer -> for each of
// THAT container's global_state.child_ids, another GetContainer, keeping
// only container_type=="Project" (confirmed directly in
// ui/src/projectSelection.ts). So: a Workspace container just needs to exist
// with the right type+owner to be found at all; a Project container must
// ADDITIONALLY be a real structural CHILD of its Workspace container
// (parent_id set + the parent's child_ids updated) — which is exactly what
// ZSEIQuery::CreateContainer's handler already does for us for free
// (src/zsei/query.rs's create_container) when called with parent_id = the
// owning workspace's real container id.
//
// owner_id is always 0 here: no real multi-user auth exists yet, and 0 is
// the only id any real container in this backend is ever tagged with
// (grep-confirmed across src/ this session — the same convention
// ui/src/projectSelection.ts documents inline) — not a placeholder, the
// actual current single-user reality.
//
// Host call mechanism mirrors url_link/file_link/package_link's established
// real pattern exactly (HTTP POST {OZONE_HOST}/zsei/query, session_token:
// "" — OZONE_ZSEI_REQUIRE_SESSION, off by default, is the only thing that
// would ever reject an empty token, and only for write variants like
// CreateContainer; confirmed in src/grpc/mod.rs's query_zsei). NOTE: prior
// to this change this file had NO zsei_query helper and GetProjectContext
// did local-only file aggregation (its own comment says "simplified - real
// impl would call ContextAggregation pipeline") — added here from scratch,
// modeled directly on url_link/main.rs's link_reference_to_graph (the real,
// already-working reference for this exact HTTP CreateContainer call), not
// reused from elsewhere in this file.
// ============================================================================

fn ozone_host() -> String {
    std::env::var("OZONE_HOST").unwrap_or_else(|_| "http://127.0.0.1:50051".to_string())
}

async fn zsei_query(query: serde_json::Value) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/zsei/query", ozone_host()))
        .json(&serde_json::json!({"query": query, "session_token": ""}))
        .send()
        .await
        .map_err(|e| format!("zsei query request failed: {}", e))?;
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("zsei query response parse failed: {}", e))?;
    if body.get("success").and_then(|s| s.as_bool()) != Some(true) {
        return Err(body
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("zsei query failed")
            .to_string());
    }
    Ok(body.get("result").cloned().unwrap_or(serde_json::Value::Null))
}

/// Create a real ZSEI container (container_type: "Workspace" | "Project")
/// and return its real, host-allocated container id. `parent_id: 0` is the
/// established root-level fallback this codebase already uses everywhere a
/// container has no real structural parent (graph_events.rs tests,
/// integrity/mod.rs, context_mirror.rs all pass parent_id: 0) —
/// CreateContainer's own handler treats an unresolvable parent as a no-op
/// on the parent's child_ids, never an error, so this is safe even though
/// container 0 itself is never actually created anywhere.
async fn create_zsei_container(parent_id: u64, container_type: &str, name: &str) -> Result<u64, String> {
    let ts = now();
    let container = serde_json::json!({
        "global_state": { "container_id": 0, "child_count": 0, "version": 1, "parent_id": 0, "child_ids": [] },
        "local_state": {
            "metadata": {
                "container_type": container_type,
                "modality": "Unknown",
                "created_at": ts,
                "updated_at": ts,
                "provenance": "workspace_tab",
                "permissions": 0,
                "owner_id": 0,
                "name": name,
                "materialized_path": null
            },
            "context": {
                "categories": [],
                "methodologies": [],
                "keywords": [],
                "topics": [],
                "relationships": [],
                "learned_associations": [],
                "embedding": null
            },
            "storage": { "db_shard_id": null, "vector_index_ref": null, "object_store_path": null, "compression_type": "None" },
            "hints": { "access_frequency": 0, "hotness_score": 0.0, "last_accessed": 0, "centroid": null, "ml_prediction_weight": 0.0 },
            "integrity": { "content_hash": vec![0u8; 32], "semantic_fingerprint": [], "last_verified": ts, "integrity_score": 1.0, "version_history": [] },
            "file_context": null,
            "code_context": null,
            "text_context": null,
            "external_ref": null
        }
    });

    let result = zsei_query(serde_json::json!({
        "CreateContainer": { "parent_id": parent_id, "container": container }
    }))
    .await?;
    result
        .get("ContainerID")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "CreateContainer did not return a ContainerID".to_string())
}

// ============================================================================
// Pipeline Execution
// ============================================================================

pub async fn execute(input: WorkspaceInput) -> Result<WorkspaceOutput, String> {
    match input {
        WorkspaceInput::ListWorkspaces => {
            // Backfill (2026-10-18): heal any workspace whose record predates
            // zsei_container_id, or whose create-time container attempt
            // failed, by creating its real container now. Done here (not in
            // GetWorkspace — see that handler's comment) because this action
            // runs on every tab load: one place, no separate migration
            // script, self-heals silently but observably (logged below, not
            // a silent surprise to a future reader of this code).
            let mut workspaces = load_workspaces();
            let mut changed = false;
            for ws in workspaces.iter_mut() {
                if ws.zsei_container_id.is_none() {
                    match create_zsei_container(0, "Workspace", &ws.name).await {
                        Ok(id) => {
                            ws.zsei_container_id = Some(id);
                            changed = true;
                        }
                        Err(e) => eprintln!(
                            "workspace_tab: ListWorkspaces backfill: ZSEI container creation failed for workspace '{}' (id={}): {} — will retry next load",
                            ws.name, ws.id, e
                        ),
                    }
                }
            }
            if changed {
                save_workspaces(&workspaces)?;
            }
            Ok(WorkspaceOutput {
                success: true,
                workspace: None,
                workspaces: Some(workspaces),
                project: None, projects: None, files: None, context: None, error: None,
            })
        }
        
        WorkspaceInput::CreateWorkspace { name, description } => {
            let mut workspaces = load_workspaces();
            // Real ZSEI container (2026-10-18 unification) — see the "Real
            // ZSEI graph wiring" block above. Best-effort: a failure here
            // does NOT fail workspace creation (today's existing behavior
            // is preserved exactly) — it just leaves zsei_container_id
            // None, which ListWorkspaces' backfill will retry on the next
            // tab load.
            let zsei_container_id = match create_zsei_container(0, "Workspace", &name).await {
                Ok(id) => Some(id),
                Err(e) => {
                    eprintln!(
                        "workspace_tab: CreateWorkspace: real ZSEI container creation failed for '{}': {} — stored without graph backing, will self-heal on next list",
                        name, e
                    );
                    None
                }
            };
            let ws = Workspace {
                id: generate_id(),
                name,
                description,
                project_count: 0,
                created_at: now(),
                updated_at: now(),
                zsei_container_id,
            };
            workspaces.push(ws.clone());
            save_workspaces(&workspaces)?;
            
            Ok(WorkspaceOutput {
                success: true,
                workspace: Some(ws),
                workspaces: None, project: None, projects: None, files: None, context: None, error: None,
            })
        }
        
        WorkspaceInput::GetWorkspace { workspace_id } => {
            // Deliberately NOT backfilled here — zsei_container_id healing
            // lives in ListWorkspaces (called on every tab load), by design,
            // to keep the self-heal logic in one place. A workspace reached
            // only via a direct GetWorkspace (never through a List call)
            // could still show None here; not expected in practice since the
            // UI always lists before selecting.
            let workspaces = load_workspaces();
            let ws = workspaces.into_iter().find(|w| w.id == workspace_id);
            
            if let Some(workspace) = ws {
                Ok(WorkspaceOutput {
                    success: true,
                    workspace: Some(workspace),
                    workspaces: None, project: None, projects: None, files: None, context: None, error: None,
                })
            } else {
                Ok(WorkspaceOutput {
                    success: false,
                    workspace: None, workspaces: None, project: None, projects: None, files: None, context: None,
                    error: Some(format!("Workspace {} not found", workspace_id)),
                })
            }
        }
        
        WorkspaceInput::UpdateWorkspace { workspace_id, name, description } => {
            let mut workspaces = load_workspaces();
            if let Some(ws) = workspaces.iter_mut().find(|w| w.id == workspace_id) {
                if let Some(n) = name { ws.name = n; }
                if let Some(d) = description { ws.description = Some(d); }
                ws.updated_at = now();
                let updated = ws.clone();
                save_workspaces(&workspaces)?;
                
                Ok(WorkspaceOutput {
                    success: true,
                    workspace: Some(updated),
                    workspaces: None, project: None, projects: None, files: None, context: None, error: None,
                })
            } else {
                Ok(WorkspaceOutput {
                    success: false,
                    workspace: None, workspaces: None, project: None, projects: None, files: None, context: None,
                    error: Some(format!("Workspace {} not found", workspace_id)),
                })
            }
        }
        
        WorkspaceInput::DeleteWorkspace { workspace_id } => {
            let mut workspaces = load_workspaces();
            let initial_len = workspaces.len();
            workspaces.retain(|w| w.id != workspace_id);
            
            if workspaces.len() < initial_len {
                save_workspaces(&workspaces)?;
                // Also delete associated projects file
                let projects_path = storage_path().join(format!("projects_{}.json", workspace_id));
                let _ = fs::remove_file(projects_path);
                
                Ok(WorkspaceOutput {
                    success: true,
                    workspace: None, workspaces: None, project: None, projects: None, files: None, context: None, error: None,
                })
            } else {
                Ok(WorkspaceOutput {
                    success: false,
                    workspace: None, workspaces: None, project: None, projects: None, files: None, context: None,
                    error: Some(format!("Workspace {} not found", workspace_id)),
                })
            }
        }
        
        WorkspaceInput::ListProjects { workspace_id } => {
            // Backfill (2026-10-18), same rationale as ListWorkspaces. A
            // project container must parent onto its WORKSPACE's real
            // container (see "Real ZSEI graph wiring" above), so the owning
            // workspace is healed first if it still has none itself.
            let mut workspaces = load_workspaces();
            let mut workspaces_changed = false;
            let parent_container_id = if let Some(ws) = workspaces.iter_mut().find(|w| w.id == workspace_id) {
                if ws.zsei_container_id.is_none() {
                    match create_zsei_container(0, "Workspace", &ws.name).await {
                        Ok(id) => {
                            ws.zsei_container_id = Some(id);
                            workspaces_changed = true;
                        }
                        Err(e) => eprintln!(
                            "workspace_tab: ListProjects backfill: owning workspace '{}' (id={}) ZSEI container creation failed: {} — its projects can't attach for real yet, will retry next load",
                            ws.name, ws.id, e
                        ),
                    }
                }
                ws.zsei_container_id
            } else {
                None
            };
            if workspaces_changed {
                save_workspaces(&workspaces)?;
            }

            let mut projects = load_projects(workspace_id);
            let mut projects_changed = false;
            for proj in projects.iter_mut() {
                if proj.zsei_container_id.is_none() {
                    if let Some(parent_id) = parent_container_id {
                        match create_zsei_container(parent_id, "Project", &proj.name).await {
                            Ok(id) => {
                                proj.zsei_container_id = Some(id);
                                projects_changed = true;
                            }
                            Err(e) => eprintln!(
                                "workspace_tab: ListProjects backfill: ZSEI container creation failed for project '{}' (id={}): {} — will retry next load",
                                proj.name, proj.id, e
                            ),
                        }
                    } else {
                        eprintln!(
                            "workspace_tab: ListProjects backfill: cannot create container for project '{}' (id={}) — owning workspace {} has no real container yet",
                            proj.name, proj.id, workspace_id
                        );
                    }
                }
            }
            if projects_changed {
                save_projects(workspace_id, &projects)?;
            }

            Ok(WorkspaceOutput {
                success: true,
                workspace: None, workspaces: None, project: None,
                projects: Some(projects),
                files: None, context: None, error: None,
            })
        }
        
        WorkspaceInput::CreateProject { workspace_id, name, description } => {
            // Update workspace project count, and lazily heal its real ZSEI
            // container if it doesn't have one yet (an old pre-existing
            // workspace, or one whose own create-time attempt failed) —
            // the project container needs a real parent to attach to, see
            // the "Real ZSEI graph wiring" block above, rather than failing
            // outright.
            let mut workspaces = load_workspaces();
            let mut parent_container_id: Option<u64> = None;
            if let Some(ws) = workspaces.iter_mut().find(|w| w.id == workspace_id) {
                ws.project_count += 1;
                ws.updated_at = now();
                if ws.zsei_container_id.is_none() {
                    match create_zsei_container(0, "Workspace", &ws.name).await {
                        Ok(id) => ws.zsei_container_id = Some(id),
                        Err(e) => eprintln!(
                            "workspace_tab: CreateProject: lazy workspace-container creation failed for '{}' (id={}): {} — project will be stored without graph backing, will self-heal on next list",
                            ws.name, ws.id, e
                        ),
                    }
                }
                parent_container_id = ws.zsei_container_id;
                save_workspaces(&workspaces)?;
            }

            // Create project — real ZSEI container (best-effort, same
            // degrade-honestly-then-self-heal pattern as CreateWorkspace).
            let zsei_container_id = match parent_container_id {
                Some(pid) => match create_zsei_container(pid, "Project", &name).await {
                    Ok(id) => Some(id),
                    Err(e) => {
                        eprintln!(
                            "workspace_tab: CreateProject: real ZSEI container creation failed for '{}': {} — stored without graph backing, will self-heal on next list",
                            name, e
                        );
                        None
                    }
                },
                None => None,
            };

            let mut projects = load_projects(workspace_id);
            let proj = Project {
                id: generate_id(),
                workspace_id,
                name,
                description,
                files_count: 0,
                url_count: 0,
                package_count: 0,
                created_at: now(),
                updated_at: now(),
                zsei_container_id,
            };
            projects.push(proj.clone());
            save_projects(workspace_id, &projects)?;
            
            Ok(WorkspaceOutput {
                success: true,
                workspace: None, workspaces: None,
                project: Some(proj),
                projects: None, files: None, context: None, error: None,
            })
        }
        
        WorkspaceInput::GetProject { project_id } => {
            // Deliberately NOT backfilled here — see GetWorkspace's comment;
            // zsei_container_id healing lives in ListProjects, called on
            // every tab load, by design.
            // Search across all workspaces (could be optimized with index)
            for ws in load_workspaces() {
                let projects = load_projects(ws.id);
                if let Some(proj) = projects.into_iter().find(|p| p.id == project_id) {
                    return Ok(WorkspaceOutput {
                        success: true,
                        workspace: None, workspaces: None,
                        project: Some(proj),
                        projects: None, files: None, context: None, error: None,
                    });
                }
            }
            
            Ok(WorkspaceOutput {
                success: false,
                workspace: None, workspaces: None, project: None, projects: None, files: None, context: None,
                error: Some(format!("Project {} not found", project_id)),
            })
        }
        
        WorkspaceInput::UpdateProject { project_id, name, description } => {
            // Find and update project
            for ws in load_workspaces() {
                let mut projects = load_projects(ws.id);
                if let Some(proj) = projects.iter_mut().find(|p| p.id == project_id) {
                    if let Some(n) = name { proj.name = n; }
                    if let Some(d) = description { proj.description = Some(d); }
                    proj.updated_at = now();
                    let updated = proj.clone();
                    save_projects(ws.id, &projects)?;
                    
                    return Ok(WorkspaceOutput {
                        success: true,
                        workspace: None, workspaces: None,
                        project: Some(updated),
                        projects: None, files: None, context: None, error: None,
                    });
                }
            }
            
            Ok(WorkspaceOutput {
                success: false,
                workspace: None, workspaces: None, project: None, projects: None, files: None, context: None,
                error: Some(format!("Project {} not found", project_id)),
            })
        }
        
        WorkspaceInput::DeleteProject { project_id } => {
            for ws in &mut load_workspaces() {
                let mut projects = load_projects(ws.id);
                let initial_len = projects.len();
                projects.retain(|p| p.id != project_id);
                
                if projects.len() < initial_len {
                    save_projects(ws.id, &projects)?;
                    
                    // Update workspace project count
                    let mut workspaces = load_workspaces();
                    if let Some(workspace) = workspaces.iter_mut().find(|w| w.id == ws.id) {
                        if workspace.project_count > 0 {
                            workspace.project_count -= 1;
                        }
                        workspace.updated_at = now();
                        save_workspaces(&workspaces)?;
                    }
                    
                    // Delete associated files
                    let files_path = storage_path().join(format!("files_{}.json", project_id));
                    let _ = fs::remove_file(files_path);
                    
                    return Ok(WorkspaceOutput {
                        success: true,
                        workspace: None, workspaces: None, project: None, projects: None, files: None, context: None, error: None,
                    });
                }
            }
            
            Ok(WorkspaceOutput {
                success: false,
                workspace: None, workspaces: None, project: None, projects: None, files: None, context: None,
                error: Some(format!("Project {} not found", project_id)),
            })
        }
        
        WorkspaceInput::GetProjectFiles { project_id } => {
            let files = load_file_refs(project_id);
            Ok(WorkspaceOutput {
                success: true,
                workspace: None, workspaces: None, project: None, projects: None,
                files: Some(files),
                context: None, error: None,
            })
        }
        
        WorkspaceInput::GetProjectContext { project_id, token_budget } => {
            let budget = token_budget.unwrap_or(4096);
            let files = load_file_refs(project_id);
            
            // Aggregate context from files (simplified - real impl would call ContextAggregation pipeline)
            let mut context_parts = vec![];
            for file in &files {
                context_parts.push(format!("File: {} ({})", file.name, file.modality));
            }
            
            let context = if context_parts.is_empty() {
                format!("Project context (no files linked, budget: {} tokens)", budget)
            } else {
                format!("Project context ({} files, budget: {} tokens):\n{}", 
                    files.len(), budget, context_parts.join("\n"))
            };
            
            Ok(WorkspaceOutput {
                success: true,
                workspace: None, workspaces: None, project: None, projects: None, files: None,
                context: Some(context),
                error: None,
            })
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() {
        if args[i] == "--input" && i + 1 < args.len() {
            input_json = args[i + 1].clone();
        }
    }
    // Envelope contract: the host passes the full PipelineInput
    // {data, context} — unwrap `data`; a bare WorkspaceInput JSON also works.
    let value: serde_json::Value = serde_json::from_str(&input_json).unwrap_or_else(|e| {
        eprintln!("Parse error: {}", e);
        std::process::exit(1);
    });
    let data_json = match value.get("data") {
        Some(d) => serde_json::to_string(d).unwrap_or(input_json.clone()),
        None => input_json.clone(),
    };
    let input: WorkspaceInput = serde_json::from_str(&data_json).unwrap_or_else(|e| {
        eprintln!("Parse error: {}", e);
        std::process::exit(1);
    });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => {
            println!("{}", serde_json::json!({"success": false, "error": e}));
            std::process::exit(1);
        }
    }
}
