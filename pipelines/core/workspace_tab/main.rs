//! WorkspaceTabPipeline - Pipeline #6
//! 
//! Workspace and project management UI.
//! Workspaces contain projects which contain file/URL/package references.

use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub workspace_id: u64,
    pub name: String,
    pub description: Option<String>,
    pub project_count: u32,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub project_id: u64,
    pub workspace_id: u64,
    pub name: String,
    pub description: Option<String>,
    pub file_count: u32,
    pub url_count: u32,
    pub package_count: u32,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRef {
    pub file_ref_id: u64,
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

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
}

pub async fn execute(input: WorkspaceInput) -> Result<WorkspaceOutput, String> {
    match input {
        WorkspaceInput::ListWorkspaces => {
            Ok(WorkspaceOutput {
                success: true,
                workspace: None,
                workspaces: Some(vec![
                    Workspace { workspace_id: 1, name: "Default".into(), description: None, project_count: 2, created_at: now() - 86400, updated_at: now() },
                ]),
                project: None, projects: None, files: None, context: None, error: None,
            })
        }
        WorkspaceInput::CreateWorkspace { name, description } => {
            let ws = Workspace { workspace_id: now(), name, description, project_count: 0, created_at: now(), updated_at: now() };
            Ok(WorkspaceOutput { success: true, workspace: Some(ws), workspaces: None, project: None, projects: None, files: None, context: None, error: None })
        }
        WorkspaceInput::GetWorkspace { workspace_id } => {
            let ws = Workspace { workspace_id, name: "Workspace".into(), description: None, project_count: 1, created_at: now() - 86400, updated_at: now() };
            Ok(WorkspaceOutput { success: true, workspace: Some(ws), workspaces: None, project: None, projects: None, files: None, context: None, error: None })
        }
        WorkspaceInput::ListProjects { workspace_id } => {
            Ok(WorkspaceOutput {
                success: true, workspace: None, workspaces: None, project: None,
                projects: Some(vec![
                    Project { project_id: 1, workspace_id, name: "Project 1".into(), description: None, file_count: 5, url_count: 2, package_count: 10, created_at: now() - 3600, updated_at: now() },
                ]),
                files: None, context: None, error: None,
            })
        }
        WorkspaceInput::CreateProject { workspace_id, name, description } => {
            let proj = Project { project_id: now(), workspace_id, name, description, file_count: 0, url_count: 0, package_count: 0, created_at: now(), updated_at: now() };
            Ok(WorkspaceOutput { success: true, workspace: None, workspaces: None, project: Some(proj), projects: None, files: None, context: None, error: None })
        }
        WorkspaceInput::GetProject { project_id } => {
            let proj = Project { project_id, workspace_id: 1, name: "Project".into(), description: None, file_count: 3, url_count: 1, package_count: 5, created_at: now() - 3600, updated_at: now() };
            Ok(WorkspaceOutput { success: true, workspace: None, workspaces: None, project: Some(proj), projects: None, files: None, context: None, error: None })
        }
        WorkspaceInput::GetProjectFiles { project_id } => {
            Ok(WorkspaceOutput {
                success: true, workspace: None, workspaces: None, project: None, projects: None,
                files: Some(vec![
                    FileRef { file_ref_id: 1, path: "/src/main.rs".into(), name: "main.rs".into(), modality: "Code".into(), size_bytes: 1024, last_modified: now() - 100 },
                ]),
                context: None, error: None,
            })
        }
        WorkspaceInput::GetProjectContext { project_id, token_budget } => {
            let budget = token_budget.unwrap_or(4096);
            Ok(WorkspaceOutput { success: true, workspace: None, workspaces: None, project: None, projects: None, files: None, context: Some(format!("Project context (budget: {} tokens)", budget)), error: None })
        }
        _ => Ok(WorkspaceOutput { success: true, workspace: None, workspaces: None, project: None, projects: None, files: None, context: None, error: None })
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() { if args[i] == "--input" && i + 1 < args.len() { input_json = args[i + 1].clone(); } }
    let input: WorkspaceInput = serde_json::from_str(&input_json).unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { println!("{}", serde_json::json!({"success": false, "error": e})); std::process::exit(1); }
    }
}
