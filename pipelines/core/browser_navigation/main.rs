//! BrowserNavigationPipeline - Pipeline #25
//! Automated browser navigation for extracting web content context.
//! Per spec §24: Navigate, extract semantic content, build relationships.
//!
//! Uses headless browser (Chrome/Chromium) via subprocess for security.

use serde::{Deserialize, Serialize};
use std::process::Command;
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum BrowserNavInput {
    /// Navigate to URL and optionally extract content
    Navigate { url: String, extract_content: bool },
    /// Extract specific content using CSS selectors
    ExtractContent { url: String, selectors: Option<Vec<String>> },
    /// Take screenshot
    Screenshot { url: String },
    /// Execute JavaScript and return result
    ExecuteScript { url: String, script: String },
    /// Build semantic snapshot for ZSEI
    BuildSemanticSnapshot { url: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserNavOutput {
    pub success: bool,
    pub url: Option<String>,
    pub title: Option<String>,
    pub content: Option<String>,
    pub screenshot_base64: Option<String>,
    pub script_result: Option<serde_json::Value>,
    pub semantic_snapshot: Option<SemanticSnapshot>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSnapshot {
    pub summary: String,
    pub key_concepts: Vec<String>,
    pub keywords: Vec<String>,
    pub topics: Vec<String>,
    pub structure_outline: Option<String>,
    pub captured_at: u64,
}

/// Get the browser executable path
fn get_browser_path() -> String {
    env::var("BROWSER_PATH").unwrap_or_else(|_| {
        // Try common locations
        for path in &[
            "/usr/bin/chromium-browser",
            "/usr/bin/chromium",
            "/usr/bin/google-chrome",
            "/usr/bin/google-chrome-stable",
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        ] {
            if std::path::Path::new(path).exists() {
                return path.to_string();
            }
        }
        "chromium".to_string()
    })
}

/// Navigate to URL using headless browser
async fn navigate_headless(url: &str, extract: bool) -> Result<(String, String), String> {
    let browser = get_browser_path();
    
    // Use headless mode with minimal JS to extract title and content
    let script = if extract {
        r#"document.title + '\n---CONTENT---\n' + document.body.innerText.slice(0, 50000)"#
    } else {
        r#"document.title"#
    };
    
    let output = Command::new(&browser)
        .args([
            "--headless",
            "--disable-gpu",
            "--no-sandbox",
            "--disable-dev-shm-usage",
            "--dump-dom",
            url,
        ])
        .output();
    
    match output {
        Ok(result) => {
            if result.status.success() {
                let html = String::from_utf8_lossy(&result.stdout).to_string();
                
                // Extract title from HTML
                let title = extract_title(&html).unwrap_or_else(|| "Untitled".to_string());
                
                // Extract text content
                let content = if extract {
                    extract_text_content(&html)
                } else {
                    String::new()
                };
                
                Ok((title, content))
            } else {
                Err(String::from_utf8_lossy(&result.stderr).to_string())
            }
        }
        Err(e) => {
            // Fallback: try using curl for basic content
            let curl_output = Command::new("curl")
                .args(["-sL", "-A", "Mozilla/5.0 OzoneStudio/0.3", url])
                .output();
            
            match curl_output {
                Ok(result) if result.status.success() => {
                    let html = String::from_utf8_lossy(&result.stdout).to_string();
                    let title = extract_title(&html).unwrap_or_else(|| "Untitled".to_string());
                    let content = if extract { extract_text_content(&html) } else { String::new() };
                    Ok((title, content))
                }
                _ => Err(format!("Browser not available and curl fallback failed: {}", e))
            }
        }
    }
}

/// Extract title from HTML
fn extract_title(html: &str) -> Option<String> {
    // Simple regex-free extraction
    let start = html.find("<title>")?;
    let end = html.find("</title>")?;
    if start < end {
        Some(html[start + 7..end].trim().to_string())
    } else {
        None
    }
}

/// Extract text content from HTML (simplified)
fn extract_text_content(html: &str) -> String {
    let mut content = html.to_string();
    
    // Remove script and style tags
    while let Some(start) = content.find("<script") {
        if let Some(end) = content[start..].find("</script>") {
            content = format!("{}{}", &content[..start], &content[start + end + 9..]);
        } else {
            break;
        }
    }
    while let Some(start) = content.find("<style") {
        if let Some(end) = content[start..].find("</style>") {
            content = format!("{}{}", &content[..start], &content[start + end + 8..]);
        } else {
            break;
        }
    }
    
    // Remove HTML tags
    let mut result = String::new();
    let mut in_tag = false;
    for c in content.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
            result.push(' ');
        } else if !in_tag {
            result.push(c);
        }
    }
    
    // Clean up whitespace
    result.split_whitespace().collect::<Vec<_>>().join(" ")
        .chars().take(50000).collect()
}

/// Build semantic snapshot from content
fn build_snapshot(content: &str) -> SemanticSnapshot {
    // Extract keywords (simple word frequency)
    let words: Vec<&str> = content.split_whitespace()
        .filter(|w| w.len() > 4)
        .collect();
    
    let mut word_counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for word in &words {
        let w = word.to_lowercase().trim_matches(|c: char| !c.is_alphanumeric()).to_string();
        if w.len() > 4 {
            *word_counts.entry(w).or_insert(0) += 1;
        }
    }
    
    let mut sorted: Vec<_> = word_counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    
    let keywords: Vec<String> = sorted.iter().take(20).map(|(w, _)| w.clone()).collect();
    
    // Generate simple summary (first 500 chars)
    let summary = content.chars().take(500).collect::<String>() + "...";
    
    SemanticSnapshot {
        summary,
        key_concepts: keywords.iter().take(5).cloned().collect(),
        keywords,
        topics: vec![], // Would need NLP/LLM for proper topics
        structure_outline: None,
        captured_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    }
}

pub async fn execute(input: BrowserNavInput) -> Result<BrowserNavOutput, String> {
    match input {
        BrowserNavInput::Navigate { url, extract_content } => {
            match navigate_headless(&url, extract_content).await {
                Ok((title, content)) => {
                    Ok(BrowserNavOutput {
                        success: true,
                        url: Some(url),
                        title: Some(title),
                        content: if extract_content { Some(content) } else { None },
                        screenshot_base64: None,
                        script_result: None,
                        semantic_snapshot: None,
                        error: None,
                    })
                }
                Err(e) => Ok(BrowserNavOutput {
                    success: false,
                    url: Some(url),
                    title: None,
                    content: None,
                    screenshot_base64: None,
                    script_result: None,
                    semantic_snapshot: None,
                    error: Some(e),
                })
            }
        }
        
        BrowserNavInput::ExtractContent { url, selectors } => {
            match navigate_headless(&url, true).await {
                Ok((title, content)) => {
                    Ok(BrowserNavOutput {
                        success: true,
                        url: Some(url),
                        title: Some(title),
                        content: Some(content),
                        screenshot_base64: None,
                        script_result: None,
                        semantic_snapshot: None,
                        error: None,
                    })
                }
                Err(e) => Ok(BrowserNavOutput {
                    success: false,
                    url: Some(url),
                    title: None,
                    content: None,
                    screenshot_base64: None,
                    script_result: None,
                    semantic_snapshot: None,
                    error: Some(e),
                })
            }
        }
        
        BrowserNavInput::Screenshot { url } => {
            let browser = get_browser_path();
            let temp_file = format!("/tmp/ozone_screenshot_{}.png", std::process::id());
            
            let output = Command::new(&browser)
                .args([
                    "--headless",
                    "--disable-gpu",
                    "--no-sandbox",
                    &format!("--screenshot={}", temp_file),
                    "--window-size=1920,1080",
                    &url,
                ])
                .output();
            
            match output {
                Ok(result) if result.status.success() => {
                    let screenshot = std::fs::read(&temp_file)
                        .map(|bytes| base64::encode(&bytes))
                        .ok();
                    let _ = std::fs::remove_file(&temp_file);
                    
                    Ok(BrowserNavOutput {
                        success: true,
                        url: Some(url),
                        title: None,
                        content: None,
                        screenshot_base64: screenshot,
                        script_result: None,
                        semantic_snapshot: None,
                        error: None,
                    })
                }
                _ => Ok(BrowserNavOutput {
                    success: false,
                    url: Some(url),
                    title: None,
                    content: None,
                    screenshot_base64: None,
                    script_result: None,
                    semantic_snapshot: None,
                    error: Some("Screenshot failed - browser not available".to_string()),
                })
            }
        }
        
        BrowserNavInput::ExecuteScript { url, script } => {
            // Script execution requires full browser - return error for now
            Ok(BrowserNavOutput {
                success: false,
                url: Some(url),
                title: None,
                content: None,
                screenshot_base64: None,
                script_result: None,
                semantic_snapshot: None,
                error: Some("Script execution requires Playwright/Puppeteer - use Navigate for basic content".to_string()),
            })
        }
        
        BrowserNavInput::BuildSemanticSnapshot { url } => {
            match navigate_headless(&url, true).await {
                Ok((title, content)) => {
                    let snapshot = build_snapshot(&content);
                    
                    Ok(BrowserNavOutput {
                        success: true,
                        url: Some(url),
                        title: Some(title),
                        content: None, // Don't return raw content
                        screenshot_base64: None,
                        script_result: None,
                        semantic_snapshot: Some(snapshot),
                        error: None,
                    })
                }
                Err(e) => Ok(BrowserNavOutput {
                    success: false,
                    url: Some(url),
                    title: None,
                    content: None,
                    screenshot_base64: None,
                    script_result: None,
                    semantic_snapshot: None,
                    error: Some(e),
                })
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() { if args[i] == "--input" && i + 1 < args.len() { input_json = args[i + 1].clone(); } }
    let input: BrowserNavInput = serde_json::from_str(&input_json).unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { println!("{}", serde_json::json!({"success": false, "error": e})); std::process::exit(1); }
    }
}
