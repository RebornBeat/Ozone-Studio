//! URLLinkPipeline - Pipeline #31
//! Link URLs to projects. Content is extracted and indexed, not copied.
//!
//! STORAGE: Persists URL references to JSON files in the data directory
//! Called FROM WorkspaceTab UI - does NOT need its own tab

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum URLLinkInput {
    /// Link a single URL to a project
    Link { project_id: u64, url: String, analyze: bool },
    /// Link multiple URLs to a project
    LinkMultiple { project_id: u64, urls: Vec<String>, analyze: bool },
    /// Unlink a URL from a project
    Unlink { project_id: u64, url_ref_id: u64 },
    /// Refresh URL status (check reachability, update content)
    Refresh { url_ref_id: u64 },
    /// Get URL reference status
    GetStatus { url_ref_id: u64 },
    /// List all URL references for a project
    ListURLs { project_id: u64 },
}

/// URL reference info - stored and returned
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct URLRefInfo {
    pub id: u64,          // Use 'id' for consistency
    pub project_id: u64,
    pub url: String,
    pub title: Option<String>,
    pub domain: String,
    pub reachable: bool,
    pub last_fetched: u64,
    pub content_extracted: bool,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct URLLinkOutput {
    pub success: bool,
    pub url_ref: Option<URLRefInfo>,
    pub url_refs: Option<Vec<URLRefInfo>>,
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

fn load_url_refs(project_id: u64) -> Vec<URLRefInfo> {
    let path = storage_path().join(format!("urls_{}.json", project_id));
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(refs) = serde_json::from_str(&content) {
                return refs;
            }
        }
    }
    vec![]
}

fn save_url_refs(project_id: u64, refs: &[URLRefInfo]) -> Result<(), String> {
    let dir = storage_path();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create dir: {}", e))?;
    let path = dir.join(format!("urls_{}.json", project_id));
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

/// Extract domain from URL
fn extract_domain(url: &str) -> String {
    url.trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("www.")
        .split('/')
        .next()
        .unwrap_or("unknown")
        .to_string()
}

/// Create a URLRefInfo from a URL with optional content fetching
fn create_url_ref(project_id: u64, url: &str, analyze: bool) -> URLRefInfo {
    let domain = extract_domain(url);
    
    let (title, reachable, content_extracted) = if analyze {
        // Actually fetch the URL
        match fetch_url_content(url) {
            Ok((fetched_title, _content)) => {
                (Some(fetched_title), true, true)
            }
            Err(_) => (None, false, false)
        }
    } else {
        (None, true, false) // Assume reachable, not fetched
    };
    
    URLRefInfo {
        id: generate_id(),
        project_id,
        url: url.to_string(),
        title,
        domain,
        reachable,
        last_fetched: if analyze { now() } else { 0 },
        content_extracted,
        created_at: now(),
    }
}

/// Fetch URL content using curl
fn fetch_url_content(url: &str) -> Result<(String, String), String> {
    use std::process::Command;
    
    // Use curl to fetch the page
    let output = Command::new("curl")
        .args([
            "-sL",                    // Silent, follow redirects
            "-A", "Mozilla/5.0 OzoneStudio/0.4",  // User agent
            "--connect-timeout", "10",  // 10 second timeout
            "--max-time", "30",         // 30 second max
            url
        ])
        .output()
        .map_err(|e| format!("Failed to fetch URL: {}", e))?;
    
    if !output.status.success() {
        return Err("URL fetch failed".to_string());
    }
    
    let html = String::from_utf8_lossy(&output.stdout).to_string();
    
    // Extract title from HTML
    let title = extract_title_from_html(&html).unwrap_or_else(|| "Untitled".to_string());
    
    // Clean content (remove HTML tags for basic text)
    let content = extract_text_from_html(&html);
    
    Ok((title, content))
}

/// Extract title from HTML
fn extract_title_from_html(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let start = lower.find("<title>")?;
    let end = lower.find("</title>")?;
    if start < end {
        Some(html[start + 7..end].trim().to_string())
    } else {
        None
    }
}

/// Extract text content from HTML (basic tag stripping)
fn extract_text_from_html(html: &str) -> String {
    let mut result = html.to_string();
    
    // Remove script and style tags
    while let Some(start) = result.to_lowercase().find("<script") {
        if let Some(end) = result.to_lowercase()[start..].find("</script>") {
            result = format!("{}{}", &result[..start], &result[start + end + 9..]);
        } else { break; }
    }
    while let Some(start) = result.to_lowercase().find("<style") {
        if let Some(end) = result.to_lowercase()[start..].find("</style>") {
            result = format!("{}{}", &result[..start], &result[start + end + 8..]);
        } else { break; }
    }
    
    // Remove HTML tags
    let mut text = String::new();
    let mut in_tag = false;
    for c in result.chars() {
        match c {
            '<' => in_tag = true,
            '>' => { in_tag = false; text.push(' '); }
            _ if !in_tag => text.push(c),
            _ => {}
        }
    }
    
    // Clean whitespace
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ============================================================================
// Pipeline Execution
// ============================================================================

pub async fn execute(input: URLLinkInput) -> Result<URLLinkOutput, String> {
    match input {
        URLLinkInput::Link { project_id, url, analyze } => {
            // Validate URL format
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Ok(URLLinkOutput {
                    success: false,
                    url_ref: None,
                    url_refs: None,
                    error: Some("URL must start with http:// or https://".to_string()),
                });
            }
            
            let mut refs = load_url_refs(project_id);
            
            // Check if already linked
            if refs.iter().any(|r| r.url == url) {
                return Ok(URLLinkOutput {
                    success: false,
                    url_ref: None,
                    url_refs: None,
                    error: Some("URL already linked to this project".to_string()),
                });
            }
            
            let url_ref = create_url_ref(project_id, &url, analyze);
            refs.push(url_ref.clone());
            save_url_refs(project_id, &refs)?;
            
            // analyze=true will fetch content in create_url_ref
            
            Ok(URLLinkOutput {
                success: true,
                url_ref: Some(url_ref),
                url_refs: None,
                error: None,
            })
        }
        
        URLLinkInput::LinkMultiple { project_id, urls, analyze } => {
            let mut refs = load_url_refs(project_id);
            let mut new_refs = vec![];
            
            for url in urls {
                // Validate URL format
                if !url.starts_with("http://") && !url.starts_with("https://") {
                    continue;
                }
                
                // Skip if already linked
                if refs.iter().any(|r| r.url == url) {
                    continue;
                }
                
                let url_ref = create_url_ref(project_id, &url, analyze);
                refs.push(url_ref.clone());
                new_refs.push(url_ref);
            }
            
            save_url_refs(project_id, &refs)?;
            
            Ok(URLLinkOutput {
                success: true,
                url_ref: None,
                url_refs: Some(new_refs),
                error: None,
            })
        }
        
        URLLinkInput::Unlink { project_id, url_ref_id } => {
            let mut refs = load_url_refs(project_id);
            let initial_len = refs.len();
            refs.retain(|r| r.id != url_ref_id);
            
            if refs.len() < initial_len {
                save_url_refs(project_id, &refs)?;
                Ok(URLLinkOutput {
                    success: true,
                    url_ref: None,
                    url_refs: None,
                    error: None,
                })
            } else {
                Ok(URLLinkOutput {
                    success: false,
                    url_ref: None,
                    url_refs: None,
                    error: Some("URL reference not found".to_string()),
                })
            }
        }
        
        URLLinkInput::Refresh { url_ref_id } => {
            // Find the URL ref across all projects
            let dir = storage_path();
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with("urls_") && name.ends_with(".json") {
                            if let Ok(content) = fs::read_to_string(&path) {
                                if let Ok(mut refs) = serde_json::from_str::<Vec<URLRefInfo>>(&content) {
                                    if let Some(url_ref) = refs.iter_mut().find(|r| r.id == url_ref_id) {
                                        // Actually fetch the URL to refresh
                                        match fetch_url_content(&url_ref.url) {
                                            Ok((title, _content)) => {
                                                url_ref.title = Some(title);
                                                url_ref.last_fetched = now();
                                                url_ref.reachable = true;
                                                url_ref.content_extracted = true;
                                            }
                                            Err(_) => {
                                                url_ref.last_fetched = now();
                                                url_ref.reachable = false;
                                                url_ref.content_extracted = false;
                                            }
                                        }
                                        
                                        // Save updated refs
                                        let content = serde_json::to_string_pretty(&refs)
                                            .map_err(|e| e.to_string())?;
                                        fs::write(&path, content).map_err(|e| e.to_string())?;
                                        
                                        return Ok(URLLinkOutput {
                                            success: true,
                                            url_ref: Some(url_ref.clone()),
                                            url_refs: None,
                                            error: None,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            Ok(URLLinkOutput {
                success: false,
                url_ref: None,
                url_refs: None,
                error: Some("URL reference not found".to_string()),
            })
        }
        
        URLLinkInput::GetStatus { url_ref_id } => {
            // Find the URL ref
            let dir = storage_path();
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with("urls_") && name.ends_with(".json") {
                            if let Ok(content) = fs::read_to_string(&path) {
                                if let Ok(refs) = serde_json::from_str::<Vec<URLRefInfo>>(&content) {
                                    if let Some(url_ref) = refs.iter().find(|r| r.id == url_ref_id) {
                                        return Ok(URLLinkOutput {
                                            success: true,
                                            url_ref: Some(url_ref.clone()),
                                            url_refs: None,
                                            error: None,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            Ok(URLLinkOutput {
                success: false,
                url_ref: None,
                url_refs: None,
                error: Some("URL reference not found".to_string()),
            })
        }
        
        URLLinkInput::ListURLs { project_id } => {
            let refs = load_url_refs(project_id);
            Ok(URLLinkOutput {
                success: true,
                url_ref: None,
                url_refs: Some(refs),
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
    let input: URLLinkInput = serde_json::from_str(&input_json).unwrap_or_else(|e| {
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
