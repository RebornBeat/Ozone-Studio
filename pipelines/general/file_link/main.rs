//! FileLinkPipeline - Pipeline #30
//! Link local files to projects. Files are referenced, not copied.
//! 
//! STORAGE: Persists file references to JSON files in the data directory
//! Called FROM WorkspaceTab UI - does NOT need its own tab

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum FileLinkInput {
    /// Link a single file to a project
    Link { project_id: u64, file_path: String, analyze: bool },
    /// Link multiple files to a project
    LinkMultiple { project_id: u64, file_paths: Vec<String>, analyze: bool },
    /// Unlink a file from a project
    Unlink { project_id: u64, file_ref_id: u64 },
    /// Refresh file status (check if exists, updated, etc.)
    Refresh { file_ref_id: u64 },
    /// Get file reference status
    GetStatus { file_ref_id: u64 },
    /// List all file references for a project
    ListFiles { project_id: u64 },
}

/// File reference info - stored and returned
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRefInfo {
    pub id: u64,         // Use 'id' for consistency with UI
    pub project_id: u64,
    pub path: String,
    pub name: String,
    pub exists: bool,
    pub size: u64,
    pub modified: u64,
    pub modality: String,
    pub analyzed: bool,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileLinkOutput {
    pub success: bool,
    pub file_ref: Option<FileRefInfo>,
    pub file_refs: Option<Vec<FileRefInfo>>,
    pub error: Option<String>,
}

// ============================================================================
// Storage Layer
// ============================================================================

fn storage_path() -> PathBuf {
    let base = std::env::var("OZONE_DATA_PATH")
        .unwrap_or_else(|_| "./data".to_string());
    PathBuf::from(base).join("workspaces")
}

fn load_file_refs(project_id: u64) -> Vec<FileRefInfo> {
    let path = storage_path().join(format!("files_{}.json", project_id));
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(refs) = serde_json::from_str(&content) {
                return refs;
            }
        }
    }
    vec![]
}

fn save_file_refs(project_id: u64, refs: &[FileRefInfo]) -> Result<(), String> {
    let dir = storage_path();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create dir: {}", e))?;
    let path = dir.join(format!("files_{}.json", project_id));
    let content = serde_json::to_string_pretty(refs)
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Failed to write: {}", e))?;
    Ok(())
}

fn generate_id() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64 % 10_000_000_000
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Detect modality from file extension
fn detect_modality(path: &str) -> String {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    match ext.as_str() {
        // Code
        "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "c" | "cpp" | "h" | "hpp" |
        "java" | "go" | "rb" | "php" | "swift" | "kt" | "cs" | "vue" | "svelte" => "Code".to_string(),
        // Data
        "json" | "yaml" | "yml" | "toml" | "xml" | "csv" | "sql" | "graphql" => "Data".to_string(),
        // Documentation
        "md" | "txt" | "rst" | "adoc" => "Documentation".to_string(),
        // Config
        "conf" | "cfg" | "ini" | "env" | "gitignore" | "dockerignore" => "Config".to_string(),
        // Script
        "sh" | "bash" | "zsh" | "fish" | "ps1" | "bat" | "cmd" => "Script".to_string(),
        // Web
        "html" | "css" | "scss" | "sass" | "less" => "Web".to_string(),
        // Image
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "ico" => "Image".to_string(),
        // Binary
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" => "Document".to_string(),
        // Archive
        "zip" | "tar" | "gz" | "rar" | "7z" => "Archive".to_string(),
        // Default
        _ => "Unknown".to_string(),
    }
}

/// Get file metadata
fn get_file_metadata(path: &str) -> (bool, u64, u64) {
    let path = std::path::Path::new(path);
    if let Ok(meta) = fs::metadata(path) {
        let modified = meta.modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        (true, meta.len(), modified)
    } else {
        (false, 0, 0)
    }
}

/// Create a FileRefInfo from a file path with optional analysis
fn create_file_ref(project_id: u64, file_path: &str, analyze: bool) -> FileRefInfo {
    let (exists, size, modified) = get_file_metadata(file_path);
    let name = std::path::Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    let modality = detect_modality(file_path);
    
    FileRefInfo {
        id: generate_id(),
        project_id,
        path: file_path.to_string(),
        name,
        exists,
        size,
        modified,
        modality,
        analyzed: false, // Will be set true after actual analysis
        created_at: now(),
    }
}

/// Analyze a file using CodeAnalysis or TextAnalysis based on modality
fn analyze_file(file_path: &str, modality: &str) -> Result<serde_json::Value, String> {
    // Read file content
    let content = fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    match modality {
        "Code" | "Script" | "Web" => {
            // Call CodeAnalysis pipeline (#18) via subprocess
            // In real implementation, this would call the pipeline executor
            let input = serde_json::json!({
                "action": "Analyze",
                "code": content,
                "file_path": file_path,
                "use_llm": false // Structural analysis only
            });
            
            // For now, perform basic structural analysis inline
            let lines: Vec<&str> = content.lines().collect();
            let functions = lines.iter()
                .filter(|l| l.contains("fn ") || l.contains("function ") || l.contains("def "))
                .count();
            let classes = lines.iter()
                .filter(|l| l.contains("class ") || l.contains("struct ") || l.contains("impl "))
                .count();
            let imports = lines.iter()
                .filter(|l| l.trim().starts_with("use ") || l.trim().starts_with("import ") || 
                           l.trim().starts_with("from ") || l.trim().starts_with("#include"))
                .count();
            
            Ok(serde_json::json!({
                "analysis_type": "code",
                "lines": lines.len(),
                "functions": functions,
                "classes": classes,
                "imports": imports,
                "modality": modality
            }))
        }
        "Documentation" | "Data" => {
            // Call TextAnalysis pipeline (#20) via subprocess
            let words: Vec<&str> = content.split_whitespace().collect();
            let sentences = content.chars().filter(|c| *c == '.' || *c == '!' || *c == '?').count();
            let paragraphs = content.split("\n\n").filter(|p| !p.trim().is_empty()).count();
            
            Ok(serde_json::json!({
                "analysis_type": "text",
                "words": words.len(),
                "sentences": sentences,
                "paragraphs": paragraphs,
                "modality": modality
            }))
        }
        _ => {
            // Binary or unknown - basic metadata only
            Ok(serde_json::json!({
                "analysis_type": "metadata",
                "modality": modality,
                "size_bytes": content.len()
            }))
        }
    }
}

/// Store analysis result in ZSEI
fn store_analysis(project_id: u64, file_ref_id: u64, analysis: &serde_json::Value) -> Result<(), String> {
    let zsei_path = std::env::var("OZONE_ZSEI_PATH")
        .unwrap_or_else(|_| "./zsei_data".to_string());
    let analysis_dir = format!("{}/local/file_analysis", zsei_path);
    fs::create_dir_all(&analysis_dir).ok();
    
    let analysis_file = format!("{}/{}_{}.json", analysis_dir, project_id, file_ref_id);
    let content = serde_json::to_string_pretty(analysis)
        .map_err(|e| e.to_string())?;
    fs::write(&analysis_file, content)
        .map_err(|e| format!("Failed to write analysis: {}", e))?;
    
    Ok(())
}

// ============================================================================
// Pipeline Execution
// ============================================================================

pub async fn execute(input: FileLinkInput) -> Result<FileLinkOutput, String> {
    match input {
        FileLinkInput::Link { project_id, file_path, analyze } => {
            let mut refs = load_file_refs(project_id);
            
            // Check if already linked
            if refs.iter().any(|r| r.path == file_path) {
                return Ok(FileLinkOutput {
                    success: false,
                    file_ref: None,
                    file_refs: None,
                    error: Some("File already linked to this project".to_string()),
                });
            }
            
            let mut file_ref = create_file_ref(project_id, &file_path, analyze);
            
            // Perform analysis if requested and file exists
            if analyze && file_ref.exists {
                match analyze_file(&file_path, &file_ref.modality) {
                    Ok(analysis) => {
                        // Store analysis in ZSEI
                        if store_analysis(project_id, file_ref.id, &analysis).is_ok() {
                            file_ref.analyzed = true;
                        }
                    }
                    Err(e) => {
                        // Log but don't fail - analysis is optional
                        eprintln!("File analysis warning: {}", e);
                    }
                }
            }
            
            refs.push(file_ref.clone());
            save_file_refs(project_id, &refs)?;
            
            Ok(FileLinkOutput {
                success: true,
                file_ref: Some(file_ref),
                file_refs: None,
                error: None,
            })
        }
        
        FileLinkInput::LinkMultiple { project_id, file_paths, analyze } => {
            let mut refs = load_file_refs(project_id);
            let mut new_refs = vec![];
            
            for file_path in file_paths {
                // Skip if already linked
                if refs.iter().any(|r| r.path == file_path) {
                    continue;
                }
                
                let mut file_ref = create_file_ref(project_id, &file_path, analyze);
                
                // Perform analysis if requested and file exists
                if analyze && file_ref.exists {
                    if let Ok(analysis) = analyze_file(&file_path, &file_ref.modality) {
                        if store_analysis(project_id, file_ref.id, &analysis).is_ok() {
                            file_ref.analyzed = true;
                        }
                    }
                }
                
                refs.push(file_ref.clone());
                new_refs.push(file_ref);
            }
            
            save_file_refs(project_id, &refs)?;
            
            Ok(FileLinkOutput {
                success: true,
                file_ref: None,
                file_refs: Some(new_refs),
                error: None,
            })
        }
        
        FileLinkInput::Unlink { project_id, file_ref_id } => {
            let mut refs = load_file_refs(project_id);
            let initial_len = refs.len();
            refs.retain(|r| r.id != file_ref_id);
            
            if refs.len() < initial_len {
                save_file_refs(project_id, &refs)?;
                Ok(FileLinkOutput {
                    success: true,
                    file_ref: None,
                    file_refs: None,
                    error: None,
                })
            } else {
                Ok(FileLinkOutput {
                    success: false,
                    file_ref: None,
                    file_refs: None,
                    error: Some("File reference not found".to_string()),
                })
            }
        }
        
        FileLinkInput::Refresh { file_ref_id } => {
            // Find the file ref across all projects (could be optimized with an index)
            let dir = storage_path();
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with("files_") && name.ends_with(".json") {
                            if let Ok(content) = fs::read_to_string(&path) {
                                if let Ok(mut refs) = serde_json::from_str::<Vec<FileRefInfo>>(&content) {
                                    if let Some(file_ref) = refs.iter_mut().find(|r| r.id == file_ref_id) {
                                        // Refresh metadata
                                        let (exists, size, modified) = get_file_metadata(&file_ref.path);
                                        file_ref.exists = exists;
                                        file_ref.size = size;
                                        file_ref.modified = modified;
                                        
                                        // Save updated refs
                                        let content = serde_json::to_string_pretty(&refs)
                                            .map_err(|e| e.to_string())?;
                                        fs::write(&path, content).map_err(|e| e.to_string())?;
                                        
                                        return Ok(FileLinkOutput {
                                            success: true,
                                            file_ref: Some(file_ref.clone()),
                                            file_refs: None,
                                            error: None,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            Ok(FileLinkOutput {
                success: false,
                file_ref: None,
                file_refs: None,
                error: Some("File reference not found".to_string()),
            })
        }
        
        FileLinkInput::GetStatus { file_ref_id } => {
            // Find the file ref
            let dir = storage_path();
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with("files_") && name.ends_with(".json") {
                            if let Ok(content) = fs::read_to_string(&path) {
                                if let Ok(refs) = serde_json::from_str::<Vec<FileRefInfo>>(&content) {
                                    if let Some(file_ref) = refs.iter().find(|r| r.id == file_ref_id) {
                                        return Ok(FileLinkOutput {
                                            success: true,
                                            file_ref: Some(file_ref.clone()),
                                            file_refs: None,
                                            error: None,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            Ok(FileLinkOutput {
                success: false,
                file_ref: None,
                file_refs: None,
                error: Some("File reference not found".to_string()),
            })
        }
        
        FileLinkInput::ListFiles { project_id } => {
            let refs = load_file_refs(project_id);
            Ok(FileLinkOutput {
                success: true,
                file_ref: None,
                file_refs: Some(refs),
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
    let input: FileLinkInput = serde_json::from_str(&input_json).unwrap_or_else(|e| {
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
