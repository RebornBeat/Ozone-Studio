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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRef {
    pub id: u64,
    pub path: String,
    pub name: String,
    pub modality: String,
    pub size_bytes: u64,
    pub last_modified: u64,
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
    let base = std::env::var("OZONE_DATA_PATH")
        .unwrap_or_else(|_| "./data".to_string());
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
// Pipeline Execution
// ============================================================================

pub async fn execute(input: WorkspaceInput) -> Result<WorkspaceOutput, String> {
    match input {
        WorkspaceInput::ListWorkspaces => {
            let workspaces = load_workspaces();
            Ok(WorkspaceOutput {
                success: true,
                workspace: None,
                workspaces: Some(workspaces),
                project: None, projects: None, files: None, context: None, error: None,
            })
        }
        
        WorkspaceInput::CreateWorkspace { name, description } => {
            let mut workspaces = load_workspaces();
            let ws = Workspace {
                id: generate_id(),
                name,
                description,
                project_count: 0,
                created_at: now(),
                updated_at: now(),
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
            let projects = load_projects(workspace_id);
            Ok(WorkspaceOutput {
                success: true,
                workspace: None, workspaces: None, project: None,
                projects: Some(projects),
                files: None, context: None, error: None,
            })
        }
        
        WorkspaceInput::CreateProject { workspace_id, name, description } => {
            // Update workspace project count
            let mut workspaces = load_workspaces();
            if let Some(ws) = workspaces.iter_mut().find(|w| w.id == workspace_id) {
                ws.project_count += 1;
                ws.updated_at = now();
                save_workspaces(&workspaces)?;
            }
            
            // Create project
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
    let input: WorkspaceInput = serde_json::from_str(&input_json).unwrap_or_else(|e| {
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
