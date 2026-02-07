//! BrowserNavigationPipeline - Pipeline #25
//! Automated browser navigation for extracting web content context.
//! Per spec §24: Navigate, extract semantic content, build relationships.
//!
//! Uses Puppeteer in HEADFUL mode to:
//! - Avoid bot detection (headless browsers are often blocked)
//! - Allow user to view browser activity via VNC
//! - Support virtual displays (Xvfb) for server environments
//! - Provide full mouse/keyboard control for complex interactions
//!
//! ARCHITECTURE:
//! - Xvfb creates virtual display (e.g., :99) offset from main screen
//! - x11vnc exposes the display for remote viewing
//! - Puppeteer runs Chrome in headful mode on virtual display
//! - User can connect via VNC to watch/control browser activity
//!
//! IMPORTANT: We use headful mode because:
//! 1. Many sites detect and block headless browsers
//! 2. Users may want to see what the browser is doing
//! 3. Some JavaScript features work differently in headless

use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use std::env;
use std::sync::Mutex;
use std::collections::HashMap;

lazy_static::lazy_static! {
    static ref BROWSER_SESSIONS: Mutex<HashMap<String, BrowserSession>> = Mutex::new(HashMap::new());
    static ref VIRTUAL_DISPLAYS: Mutex<HashMap<String, DisplaySession>> = Mutex::new(HashMap::new());
}

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum BrowserNavInput {
    Navigate { 
        url: String, 
        extract_content: bool,
        headful: Option<bool>,
        virtual_display: Option<String>,
        wait_for: Option<String>,
        timeout_ms: Option<u32>,
    },
    ExtractContent { 
        url: String, 
        selectors: Vec<String>,
        wait_for_selector: Option<String>,
    },
    Screenshot { 
        url: String,
        full_page: Option<bool>,
        selector: Option<String>,
    },
    ExecuteScript { url: String, script: String },
    BuildSemanticSnapshot { url: String, use_llm_topics: Option<bool> },
    MouseAction {
        session_id: String,
        action_type: MouseActionType,
        x: Option<i32>,
        y: Option<i32>,
        selector: Option<String>,
    },
    KeyboardAction {
        session_id: String,
        action_type: KeyboardActionType,
        text: Option<String>,
        key: Option<String>,
        modifiers: Option<Vec<String>>,
    },
    ScrollAction {
        session_id: String,
        direction: ScrollDirection,
        amount: Option<i32>,
        selector: Option<String>,
    },
    StartSession {
        display: Option<String>,
        enable_vnc: Option<bool>,
        vnc_port: Option<u16>,
        vnc_password: Option<String>,
        initial_url: Option<String>,
    },
    GetSessionInfo { session_id: Option<String> },
    SessionNavigate { session_id: String, url: String },
    StopSession { session_id: String },
    ListSessions,
    StartVirtualDisplay { display: String, width: Option<u32>, height: Option<u32>, depth: Option<u8> },
    StopVirtualDisplay { display: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseActionType { Click, DoubleClick, RightClick, Move, Drag, Hover }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyboardActionType { Type, Press, KeyDown, KeyUp }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScrollDirection { Up, Down, Left, Right, ToElement }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserNavOutput {
    pub success: bool,
    pub url: Option<String>,
    pub title: Option<String>,
    pub content: Option<String>,
    pub screenshot_base64: Option<String>,
    pub script_result: Option<serde_json::Value>,
    pub semantic_snapshot: Option<SemanticSnapshot>,
    pub session_info: Option<SessionInfo>,
    pub sessions: Option<Vec<SessionInfo>>,
    pub extracted_content: Option<HashMap<String, String>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSnapshot {
    pub summary: String,
    pub key_concepts: Vec<String>,
    pub keywords: Vec<String>,
    pub topics: Vec<String>,
    pub structure_outline: Option<String>,
    pub links: Vec<LinkInfo>,
    pub images: Vec<ImageInfo>,
    pub captured_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkInfo { pub text: String, pub href: String, pub is_external: bool }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo { pub alt: String, pub src: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub active: bool,
    pub display: String,
    pub vnc_enabled: bool,
    pub vnc_port: Option<u16>,
    pub vnc_url: Option<String>,
    pub current_url: Option<String>,
    pub current_title: Option<String>,
    pub created_at: u64,
}

struct BrowserSession {
    display: String,
    vnc_port: Option<u16>,
    current_url: Option<String>,
    created_at: u64,
}

struct DisplaySession {
    xvfb_pid: u32,
    vnc_pid: Option<u32>,
    vnc_port: Option<u16>,
    width: u32,
    height: u32,
}

// ============================================================================
// Virtual Display Management
// ============================================================================

fn start_virtual_display(display: &str, width: u32, height: u32, depth: u8) -> Result<u32, String> {
    let check = Command::new("xdpyinfo").env("DISPLAY", display)
        .stdout(Stdio::null()).stderr(Stdio::null()).status();
    
    if check.is_ok() && check.unwrap().success() {
        let displays = VIRTUAL_DISPLAYS.lock().unwrap();
        if let Some(session) = displays.get(display) {
            return Ok(session.xvfb_pid);
        }
        return Ok(0);
    }
    
    let screen = format!("{}x{}x{}", width, height, depth);
    let child = Command::new("Xvfb")
        .args([display, "-screen", "0", &screen, "-ac", "-noreset"])
        .stdout(Stdio::null()).stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to start Xvfb: {}. Install: apt install xvfb", e))?;
    
    let pid = child.id();
    std::thread::sleep(std::time::Duration::from_millis(500));
    
    let mut displays = VIRTUAL_DISPLAYS.lock().unwrap();
    displays.insert(display.to_string(), DisplaySession {
        xvfb_pid: pid, vnc_pid: None, vnc_port: None, width, height,
    });
    
    Ok(pid)
}

fn start_vnc(display: &str, port: u16, password: Option<&str>) -> Result<u32, String> {
    let mut args = vec!["-display", display, "-rfbport", &port.to_string(), "-forever", "-shared", "-noxdamage"];
    
    let child = Command::new("x11vnc").args(&args)
        .stdout(Stdio::null()).stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to start x11vnc: {}. Install: apt install x11vnc", e))?;
    
    let pid = child.id();
    
    let mut displays = VIRTUAL_DISPLAYS.lock().unwrap();
    if let Some(session) = displays.get_mut(display) {
        session.vnc_pid = Some(pid);
        session.vnc_port = Some(port);
    }
    
    Ok(pid)
}

fn stop_virtual_display(display: &str) -> Result<(), String> {
    let mut displays = VIRTUAL_DISPLAYS.lock().unwrap();
    if let Some(session) = displays.remove(display) {
        if let Some(vnc_pid) = session.vnc_pid {
            let _ = Command::new("kill").arg(vnc_pid.to_string()).status();
        }
        let _ = Command::new("kill").arg(session.xvfb_pid.to_string()).status();
    }
    Ok(())
}

// ============================================================================
// Puppeteer Scripts
// ============================================================================

fn generate_navigation_script(url: &str, extract: bool, headful: bool, display: Option<&str>, wait_for: Option<&str>, timeout: u32) -> String {
    let display_env = display.map(|d| format!("process.env.DISPLAY = '{}';", d)).unwrap_or_default();
    let headless = if headful { "false" } else { "'new'" };
    let wait_script = wait_for.map(|s| format!("await page.waitForSelector('{}', {{ timeout: {} }});", s, timeout)).unwrap_or_default();
    
    format!(r#"
const puppeteer = require('puppeteer');
{display_env}
(async () => {{
    const browser = await puppeteer.launch({{
        headless: {headless},
        args: ['--no-sandbox', '--disable-setuid-sandbox', '--disable-blink-features=AutomationControlled', '--window-size=1920,1080', '--start-maximized', '--disable-infobars'],
        defaultViewport: null,
        ignoreDefaultArgs: ['--enable-automation'],
    }});
    const page = await browser.newPage();
    await page.setUserAgent('Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/121.0.0.0 Safari/537.36');
    await page.evaluateOnNewDocument(() => {{
        Object.defineProperty(navigator, 'webdriver', {{ get: () => undefined }});
        window.chrome = {{ runtime: {{}} }};
    }});
    try {{
        await page.goto('{url}', {{ waitUntil: 'networkidle2', timeout: {timeout} }});
        {wait_script}
        const title = await page.title();
        let content = null, links = [], images = [];
        if ({extract}) {{
            const extracted = await page.evaluate(() => {{
                document.querySelectorAll('script, style, noscript').forEach(el => el.remove());
                return {{
                    content: document.body.innerText.slice(0, 100000),
                    links: Array.from(document.querySelectorAll('a[href]')).slice(0, 100).map(a => ({{ text: a.innerText.trim().slice(0, 100), href: a.href, isExternal: !a.href.startsWith(window.location.origin) }})),
                    images: Array.from(document.querySelectorAll('img[src]')).slice(0, 50).map(img => ({{ alt: img.alt || '', src: img.src }})),
                }};
            }});
            content = extracted.content; links = extracted.links; images = extracted.images;
        }}
        console.log(JSON.stringify({{ success: true, url: page.url(), title, content, links, images }}));
    }} catch (error) {{
        console.log(JSON.stringify({{ success: false, error: error.message }}));
    }} finally {{
        await browser.close();
    }}
}})();
"#, display_env = display_env, headless = headless, url = url, timeout = timeout, wait_script = wait_script, extract = extract)
}

fn run_puppeteer_script(script: &str) -> Result<serde_json::Value, String> {
    let node = env::var("NODE_PATH").unwrap_or_else(|_| "node".to_string());
    
    let output = Command::new(&node).args(["-e", script])
        .stdout(Stdio::piped()).stderr(Stdio::piped())
        .output().map_err(|e| format!("Failed to run Node.js: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json_line = stdout.lines().filter(|l| l.trim().starts_with('{')).last()
        .ok_or_else(|| format!("No JSON output. stderr: {}", String::from_utf8_lossy(&output.stderr)))?;
    
    serde_json::from_str(json_line).map_err(|e| format!("Parse error: {}", e))
}

fn build_semantic_snapshot(content: &str, links: Vec<LinkInfo>, images: Vec<ImageInfo>) -> SemanticSnapshot {
    let mut word_counts: HashMap<String, u32> = HashMap::new();
    for word in content.split_whitespace().filter(|w| w.len() > 4) {
        let w = word.to_lowercase().trim_matches(|c: char| !c.is_alphanumeric()).to_string();
        if w.len() > 4 { *word_counts.entry(w).or_insert(0) += 1; }
    }
    let mut sorted: Vec<_> = word_counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    let keywords: Vec<String> = sorted.iter().take(30).map(|(w, _)| w.clone()).collect();
    let key_concepts: Vec<String> = keywords.iter().take(10).cloned().collect();
    let summary = content.split('\n').map(|s| s.trim()).filter(|s| s.len() > 100).next()
        .unwrap_or(&content[..500.min(content.len())]).to_string();
    
    SemanticSnapshot {
        summary, key_concepts: key_concepts.clone(), keywords, topics: key_concepts.iter().take(5).cloned().collect(),
        structure_outline: None, links, images, captured_at: now(),
    }
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
}

// ============================================================================
// Execute
// ============================================================================

pub async fn execute(input: BrowserNavInput) -> Result<BrowserNavOutput, String> {
    match input {
        BrowserNavInput::Navigate { url, extract_content, headful, virtual_display, wait_for, timeout_ms } => {
            let use_headful = headful.unwrap_or(true);
            let timeout = timeout_ms.unwrap_or(30000);
            
            let display = if use_headful {
                if let Some(ref disp) = virtual_display {
                    start_virtual_display(disp, 1920, 1080, 24)?;
                    Some(disp.as_str())
                } else if env::var("DISPLAY").is_err() {
                    start_virtual_display(":99", 1920, 1080, 24)?;
                    Some(":99")
                } else { None }
            } else { None };
            
            let script = generate_navigation_script(&url, extract_content, use_headful, display, wait_for.as_deref(), timeout);
            
            match run_puppeteer_script(&script) {
                Ok(result) => {
                    let success = result.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
                    Ok(BrowserNavOutput {
                        success,
                        url: result.get("url").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        title: result.get("title").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        content: result.get("content").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        screenshot_base64: None, script_result: None, semantic_snapshot: None,
                        session_info: None, sessions: None, extracted_content: None,
                        error: result.get("error").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    })
                }
                Err(e) => Ok(BrowserNavOutput {
                    success: false, url: Some(url), title: None, content: None,
                    screenshot_base64: None, script_result: None, semantic_snapshot: None,
                    session_info: None, sessions: None, extracted_content: None, error: Some(e),
                })
            }
        }
        
        BrowserNavInput::Screenshot { url, full_page, selector } => {
            let fp = full_page.unwrap_or(false);
            let script = format!(r#"
const puppeteer = require('puppeteer');
(async () => {{
    const browser = await puppeteer.launch({{ headless: 'new', args: ['--no-sandbox'] }});
    const page = await browser.newPage();
    await page.setViewport({{ width: 1920, height: 1080 }});
    try {{
        await page.goto('{}', {{ waitUntil: 'networkidle2', timeout: 30000 }});
        const screenshot = await page.screenshot({{ encoding: 'base64', fullPage: {} }});
        console.log(JSON.stringify({{ success: true, url: page.url(), title: await page.title(), screenshot_base64: screenshot }}));
    }} catch (error) {{
        console.log(JSON.stringify({{ success: false, error: error.message }}));
    }} finally {{
        await browser.close();
    }}
}})();
"#, url, fp);
            
            match run_puppeteer_script(&script) {
                Ok(result) => Ok(BrowserNavOutput {
                    success: result.get("success").and_then(|v| v.as_bool()).unwrap_or(false),
                    url: result.get("url").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    title: result.get("title").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    content: None,
                    screenshot_base64: result.get("screenshot_base64").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    script_result: None, semantic_snapshot: None, session_info: None, sessions: None,
                    extracted_content: None,
                    error: result.get("error").and_then(|v| v.as_str()).map(|s| s.to_string()),
                }),
                Err(e) => Ok(BrowserNavOutput {
                    success: false, url: Some(url), title: None, content: None,
                    screenshot_base64: None, script_result: None, semantic_snapshot: None,
                    session_info: None, sessions: None, extracted_content: None, error: Some(e),
                })
            }
        }
        
        BrowserNavInput::BuildSemanticSnapshot { url, use_llm_topics } => {
            let script = generate_navigation_script(&url, true, false, None, None, 30000);
            match run_puppeteer_script(&script) {
                Ok(result) => {
                    let content = result.get("content").and_then(|v| v.as_str()).unwrap_or("");
                    let links: Vec<LinkInfo> = result.get("links").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
                    let images: Vec<ImageInfo> = result.get("images").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
                    let snapshot = build_semantic_snapshot(content, links, images);
                    
                    Ok(BrowserNavOutput {
                        success: true,
                        url: result.get("url").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        title: result.get("title").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        content: Some(content.to_string()),
                        screenshot_base64: None, script_result: None,
                        semantic_snapshot: Some(snapshot),
                        session_info: None, sessions: None, extracted_content: None, error: None,
                    })
                }
                Err(e) => Ok(BrowserNavOutput {
                    success: false, url: Some(url), title: None, content: None,
                    screenshot_base64: None, script_result: None, semantic_snapshot: None,
                    session_info: None, sessions: None, extracted_content: None, error: Some(e),
                })
            }
        }
        
        BrowserNavInput::StartSession { display, enable_vnc, vnc_port, vnc_password, initial_url } => {
            let disp = display.unwrap_or_else(|| ":99".to_string());
            let display_num: u32 = disp.trim_start_matches(':').parse().unwrap_or(99);
            
            start_virtual_display(&disp, 1920, 1080, 24)?;
            
            let vnc_info = if enable_vnc.unwrap_or(true) {
                let port = vnc_port.unwrap_or(5900 + display_num as u16);
                start_vnc(&disp, port, vnc_password.as_deref())?;
                Some(port)
            } else { None };
            
            let session_id = format!("session_{}", now());
            
            let mut sessions = BROWSER_SESSIONS.lock().unwrap();
            sessions.insert(session_id.clone(), BrowserSession {
                display: disp.clone(), vnc_port: vnc_info, current_url: initial_url.clone(), created_at: now(),
            });
            
            Ok(BrowserNavOutput {
                success: true, url: None, title: None, content: None,
                screenshot_base64: None, script_result: None, semantic_snapshot: None,
                session_info: Some(SessionInfo {
                    session_id, active: true, display: disp.clone(),
                    vnc_enabled: vnc_info.is_some(), vnc_port: vnc_info,
                    vnc_url: vnc_info.map(|p| format!("vnc://localhost:{}", p)),
                    current_url: initial_url, current_title: None, created_at: now(),
                }),
                sessions: None, extracted_content: None, error: None,
            })
        }
        
        BrowserNavInput::GetSessionInfo { session_id } => {
            let sessions = BROWSER_SESSIONS.lock().unwrap();
            if let Some(sid) = session_id {
                if let Some(s) = sessions.get(&sid) {
                    Ok(BrowserNavOutput {
                        success: true, url: None, title: None, content: None,
                        screenshot_base64: None, script_result: None, semantic_snapshot: None,
                        session_info: Some(SessionInfo {
                            session_id: sid, active: true, display: s.display.clone(),
                            vnc_enabled: s.vnc_port.is_some(), vnc_port: s.vnc_port,
                            vnc_url: s.vnc_port.map(|p| format!("vnc://localhost:{}", p)),
                            current_url: s.current_url.clone(), current_title: None, created_at: s.created_at,
                        }),
                        sessions: None, extracted_content: None, error: None,
                    })
                } else {
                    Ok(BrowserNavOutput {
                        success: false, url: None, title: None, content: None,
                        screenshot_base64: None, script_result: None, semantic_snapshot: None,
                        session_info: None, sessions: None, extracted_content: None,
                        error: Some("Session not found".to_string()),
                    })
                }
            } else {
                let all: Vec<SessionInfo> = sessions.iter().map(|(id, s)| SessionInfo {
                    session_id: id.clone(), active: true, display: s.display.clone(),
                    vnc_enabled: s.vnc_port.is_some(), vnc_port: s.vnc_port,
                    vnc_url: s.vnc_port.map(|p| format!("vnc://localhost:{}", p)),
                    current_url: s.current_url.clone(), current_title: None, created_at: s.created_at,
                }).collect();
                
                Ok(BrowserNavOutput {
                    success: true, url: None, title: None, content: None,
                    screenshot_base64: None, script_result: None, semantic_snapshot: None,
                    session_info: None, sessions: Some(all), extracted_content: None, error: None,
                })
            }
        }
        
        BrowserNavInput::StopSession { session_id } => {
            let mut sessions = BROWSER_SESSIONS.lock().unwrap();
            if let Some(session) = sessions.remove(&session_id) {
                let _ = stop_virtual_display(&session.display);
                Ok(BrowserNavOutput {
                    success: true, url: None, title: None, content: None,
                    screenshot_base64: None, script_result: None, semantic_snapshot: None,
                    session_info: None, sessions: None, extracted_content: None, error: None,
                })
            } else {
                Ok(BrowserNavOutput {
                    success: false, url: None, title: None, content: None,
                    screenshot_base64: None, script_result: None, semantic_snapshot: None,
                    session_info: None, sessions: None, extracted_content: None,
                    error: Some("Session not found".to_string()),
                })
            }
        }
        
        BrowserNavInput::ListSessions => {
            let sessions = BROWSER_SESSIONS.lock().unwrap();
            let all: Vec<SessionInfo> = sessions.iter().map(|(id, s)| SessionInfo {
                session_id: id.clone(), active: true, display: s.display.clone(),
                vnc_enabled: s.vnc_port.is_some(), vnc_port: s.vnc_port,
                vnc_url: s.vnc_port.map(|p| format!("vnc://localhost:{}", p)),
                current_url: s.current_url.clone(), current_title: None, created_at: s.created_at,
            }).collect();
            
            Ok(BrowserNavOutput {
                success: true, url: None, title: None, content: None,
                screenshot_base64: None, script_result: None, semantic_snapshot: None,
                session_info: None, sessions: Some(all), extracted_content: None, error: None,
            })
        }
        
        BrowserNavInput::StartVirtualDisplay { display, width, height, depth } => {
            let pid = start_virtual_display(&display, width.unwrap_or(1920), height.unwrap_or(1080), depth.unwrap_or(24))?;
            Ok(BrowserNavOutput {
                success: true, url: None, title: None,
                content: Some(format!("Virtual display {} started (PID: {})", display, pid)),
                screenshot_base64: None, script_result: None, semantic_snapshot: None,
                session_info: None, sessions: None, extracted_content: None, error: None,
            })
        }
        
        BrowserNavInput::StopVirtualDisplay { display } => {
            stop_virtual_display(&display)?;
            Ok(BrowserNavOutput {
                success: true, url: None, title: None,
                content: Some(format!("Virtual display {} stopped", display)),
                screenshot_base64: None, script_result: None, semantic_snapshot: None,
                session_info: None, sessions: None, extracted_content: None, error: None,
            })
        }
        
        BrowserNavInput::ExtractContent { url, selectors, wait_for_selector } => {
            let selectors_json = serde_json::to_string(&selectors).unwrap_or("[]".to_string());
            let wait = wait_for_selector.map(|s| format!("await page.waitForSelector('{}', {{ timeout: 10000 }});", s)).unwrap_or_default();
            
            let script = format!(r#"
const puppeteer = require('puppeteer');
(async () => {{
    const browser = await puppeteer.launch({{ headless: 'new', args: ['--no-sandbox'] }});
    const page = await browser.newPage();
    try {{
        await page.goto('{}', {{ waitUntil: 'networkidle2', timeout: 30000 }});
        {}
        const selectors = {};
        const results = {{}};
        for (const sel of selectors) {{
            try {{
                const els = await page.$$(sel);
                results[sel] = await Promise.all(els.map(async el => await el.evaluate(e => e.innerText || e.textContent || '')));
            }} catch (e) {{ results[sel] = []; }}
        }}
        console.log(JSON.stringify({{ success: true, url: page.url(), title: await page.title(), extracted_content: results }}));
    }} catch (error) {{
        console.log(JSON.stringify({{ success: false, error: error.message }}));
    }} finally {{
        await browser.close();
    }}
}})();
"#, url, wait, selectors_json);
            
            match run_puppeteer_script(&script) {
                Ok(result) => {
                    let extracted: HashMap<String, String> = result.get("extracted_content")
                        .and_then(|v| v.as_object())
                        .map(|obj| obj.iter().map(|(k, v)| {
                            (k.clone(), v.as_array().map(|arr| arr.iter().filter_map(|s| s.as_str()).collect::<Vec<_>>().join("\n")).unwrap_or_default())
                        }).collect())
                        .unwrap_or_default();
                    
                    Ok(BrowserNavOutput {
                        success: result.get("success").and_then(|v| v.as_bool()).unwrap_or(false),
                        url: result.get("url").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        title: result.get("title").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        content: None, screenshot_base64: None, script_result: None, semantic_snapshot: None,
                        session_info: None, sessions: None, extracted_content: Some(extracted),
                        error: result.get("error").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    })
                }
                Err(e) => Ok(BrowserNavOutput {
                    success: false, url: Some(url), title: None, content: None,
                    screenshot_base64: None, script_result: None, semantic_snapshot: None,
                    session_info: None, sessions: None, extracted_content: None, error: Some(e),
                })
            }
        }
        
        BrowserNavInput::ExecuteScript { url, script } => {
            let pscript = format!(r#"
const puppeteer = require('puppeteer');
(async () => {{
    const browser = await puppeteer.launch({{ headless: 'new', args: ['--no-sandbox'] }});
    const page = await browser.newPage();
    try {{
        await page.goto('{}', {{ waitUntil: 'networkidle2', timeout: 30000 }});
        const result = await page.evaluate(() => {{ {} }});
        console.log(JSON.stringify({{ success: true, url: page.url(), title: await page.title(), script_result: result }}));
    }} catch (error) {{
        console.log(JSON.stringify({{ success: false, error: error.message }}));
    }} finally {{
        await browser.close();
    }}
}})();
"#, url, script);
            
            match run_puppeteer_script(&pscript) {
                Ok(result) => Ok(BrowserNavOutput {
                    success: result.get("success").and_then(|v| v.as_bool()).unwrap_or(false),
                    url: result.get("url").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    title: result.get("title").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    content: None, screenshot_base64: None,
                    script_result: result.get("script_result").cloned(),
                    semantic_snapshot: None, session_info: None, sessions: None, extracted_content: None,
                    error: result.get("error").and_then(|v| v.as_str()).map(|s| s.to_string()),
                }),
                Err(e) => Ok(BrowserNavOutput {
                    success: false, url: Some(url), title: None, content: None,
                    screenshot_base64: None, script_result: None, semantic_snapshot: None,
                    session_info: None, sessions: None, extracted_content: None, error: Some(e),
                })
            }
        }
        
        BrowserNavInput::SessionNavigate { session_id, url } => {
            let sessions = BROWSER_SESSIONS.lock().unwrap();
            if let Some(session) = sessions.get(&session_id) {
                let display = session.display.clone();
                drop(sessions);
                
                let script = generate_navigation_script(&url, true, true, Some(&display), None, 30000);
                match run_puppeteer_script(&script) {
                    Ok(result) => {
                        let mut sessions = BROWSER_SESSIONS.lock().unwrap();
                        if let Some(s) = sessions.get_mut(&session_id) {
                            s.current_url = Some(url.clone());
                        }
                        Ok(BrowserNavOutput {
                            success: result.get("success").and_then(|v| v.as_bool()).unwrap_or(false),
                            url: result.get("url").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            title: result.get("title").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            content: result.get("content").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            screenshot_base64: None, script_result: None, semantic_snapshot: None,
                            session_info: None, sessions: None, extracted_content: None,
                            error: result.get("error").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        })
                    }
                    Err(e) => Ok(BrowserNavOutput {
                        success: false, url: Some(url), title: None, content: None,
                        screenshot_base64: None, script_result: None, semantic_snapshot: None,
                        session_info: None, sessions: None, extracted_content: None, error: Some(e),
                    })
                }
            } else {
                Ok(BrowserNavOutput {
                    success: false, url: Some(url), title: None, content: None,
                    screenshot_base64: None, script_result: None, semantic_snapshot: None,
                    session_info: None, sessions: None, extracted_content: None,
                    error: Some("Session not found".to_string()),
                })
            }
        }
        
        BrowserNavInput::MouseAction { session_id, action_type, x, y, selector } => {
            // Get session display
            let sessions = BROWSER_SESSIONS.lock().unwrap();
            let display = sessions.get(&session_id).map(|s| s.display.clone());
            drop(sessions);
            
            let display = display.ok_or("Session not found")?;
            
            // Generate Puppeteer script for mouse action
            let action_code = match action_type {
                MouseActionType::Click => {
                    if let Some(sel) = selector {
                        format!("await page.click('{}');", sel)
                    } else if let (Some(x), Some(y)) = (x, y) {
                        format!("await page.mouse.click({}, {});", x, y)
                    } else {
                        return Ok(BrowserNavOutput {
                            success: false, url: None, title: None, content: None,
                            screenshot_base64: None, script_result: None, semantic_snapshot: None,
                            session_info: None, sessions: None, extracted_content: None,
                            error: Some("Click requires selector or x,y coordinates".to_string()),
                        });
                    }
                }
                MouseActionType::DoubleClick => {
                    if let Some(sel) = selector {
                        format!("await page.click('{}', {{ clickCount: 2 }});", sel)
                    } else if let (Some(x), Some(y)) = (x, y) {
                        format!("await page.mouse.click({}, {}, {{ clickCount: 2 }});", x, y)
                    } else {
                        return Ok(BrowserNavOutput {
                            success: false, url: None, title: None, content: None,
                            screenshot_base64: None, script_result: None, semantic_snapshot: None,
                            session_info: None, sessions: None, extracted_content: None,
                            error: Some("DoubleClick requires selector or x,y coordinates".to_string()),
                        });
                    }
                }
                MouseActionType::RightClick => {
                    if let Some(sel) = selector {
                        format!("await page.click('{}', {{ button: 'right' }});", sel)
                    } else if let (Some(x), Some(y)) = (x, y) {
                        format!("await page.mouse.click({}, {}, {{ button: 'right' }});", x, y)
                    } else {
                        return Ok(BrowserNavOutput {
                            success: false, url: None, title: None, content: None,
                            screenshot_base64: None, script_result: None, semantic_snapshot: None,
                            session_info: None, sessions: None, extracted_content: None,
                            error: Some("RightClick requires selector or x,y coordinates".to_string()),
                        });
                    }
                }
                MouseActionType::Move => {
                    if let (Some(x), Some(y)) = (x, y) {
                        format!("await page.mouse.move({}, {});", x, y)
                    } else {
                        return Ok(BrowserNavOutput {
                            success: false, url: None, title: None, content: None,
                            screenshot_base64: None, script_result: None, semantic_snapshot: None,
                            session_info: None, sessions: None, extracted_content: None,
                            error: Some("Move requires x,y coordinates".to_string()),
                        });
                    }
                }
                MouseActionType::Hover => {
                    if let Some(sel) = selector {
                        format!("await page.hover('{}');", sel)
                    } else {
                        return Ok(BrowserNavOutput {
                            success: false, url: None, title: None, content: None,
                            screenshot_base64: None, script_result: None, semantic_snapshot: None,
                            session_info: None, sessions: None, extracted_content: None,
                            error: Some("Hover requires selector".to_string()),
                        });
                    }
                }
                MouseActionType::Drag => {
                    // Drag requires start and end positions - not fully implemented
                    "console.log('Drag not fully implemented');".to_string()
                }
            };
            
            let script = format!(r#"
const puppeteer = require('puppeteer');
process.env.DISPLAY = '{}';
(async () => {{
    const browser = await puppeteer.launch({{
        headless: false,
        args: ['--no-sandbox', '--disable-setuid-sandbox'],
    }});
    const page = (await browser.pages())[0] || await browser.newPage();
    try {{
        {}
        console.log(JSON.stringify({{ success: true }}));
    }} catch (error) {{
        console.log(JSON.stringify({{ success: false, error: error.message }}));
    }} finally {{
        await browser.close();
    }}
}})();
"#, display, action_code);
            
            match run_puppeteer_script(&script) {
                Ok(result) => Ok(BrowserNavOutput {
                    success: result.get("success").and_then(|v| v.as_bool()).unwrap_or(false),
                    url: None, title: None, content: None,
                    screenshot_base64: None, script_result: None, semantic_snapshot: None,
                    session_info: None, sessions: None, extracted_content: None,
                    error: result.get("error").and_then(|v| v.as_str()).map(|s| s.to_string()),
                }),
                Err(e) => Ok(BrowserNavOutput {
                    success: false, url: None, title: None, content: None,
                    screenshot_base64: None, script_result: None, semantic_snapshot: None,
                    session_info: None, sessions: None, extracted_content: None,
                    error: Some(e),
                })
            }
        }
        
        BrowserNavInput::KeyboardAction { session_id, action_type, text, key, modifiers } => {
            let sessions = BROWSER_SESSIONS.lock().unwrap();
            let display = sessions.get(&session_id).map(|s| s.display.clone());
            drop(sessions);
            
            let display = display.ok_or("Session not found")?;
            
            let action_code = match action_type {
                KeyboardActionType::Type => {
                    if let Some(t) = text {
                        format!("await page.keyboard.type('{}');", t.replace("'", "\\'"))
                    } else {
                        return Ok(BrowserNavOutput {
                            success: false, url: None, title: None, content: None,
                            screenshot_base64: None, script_result: None, semantic_snapshot: None,
                            session_info: None, sessions: None, extracted_content: None,
                            error: Some("Type requires text".to_string()),
                        });
                    }
                }
                KeyboardActionType::Press => {
                    if let Some(k) = key {
                        let mods = modifiers.unwrap_or_default();
                        let mod_down: Vec<String> = mods.iter().map(|m| format!("await page.keyboard.down('{}');", m)).collect();
                        let mod_up: Vec<String> = mods.iter().rev().map(|m| format!("await page.keyboard.up('{}');", m)).collect();
                        format!("{} await page.keyboard.press('{}'); {}", mod_down.join(" "), k, mod_up.join(" "))
                    } else {
                        return Ok(BrowserNavOutput {
                            success: false, url: None, title: None, content: None,
                            screenshot_base64: None, script_result: None, semantic_snapshot: None,
                            session_info: None, sessions: None, extracted_content: None,
                            error: Some("Press requires key".to_string()),
                        });
                    }
                }
                KeyboardActionType::KeyDown => {
                    if let Some(k) = key {
                        format!("await page.keyboard.down('{}');", k)
                    } else {
                        return Ok(BrowserNavOutput {
                            success: false, url: None, title: None, content: None,
                            screenshot_base64: None, script_result: None, semantic_snapshot: None,
                            session_info: None, sessions: None, extracted_content: None,
                            error: Some("KeyDown requires key".to_string()),
                        });
                    }
                }
                KeyboardActionType::KeyUp => {
                    if let Some(k) = key {
                        format!("await page.keyboard.up('{}');", k)
                    } else {
                        return Ok(BrowserNavOutput {
                            success: false, url: None, title: None, content: None,
                            screenshot_base64: None, script_result: None, semantic_snapshot: None,
                            session_info: None, sessions: None, extracted_content: None,
                            error: Some("KeyUp requires key".to_string()),
                        });
                    }
                }
            };
            
            let script = format!(r#"
const puppeteer = require('puppeteer');
process.env.DISPLAY = '{}';
(async () => {{
    const browser = await puppeteer.launch({{ headless: false, args: ['--no-sandbox'] }});
    const page = (await browser.pages())[0] || await browser.newPage();
    try {{
        {}
        console.log(JSON.stringify({{ success: true }}));
    }} catch (error) {{
        console.log(JSON.stringify({{ success: false, error: error.message }}));
    }} finally {{
        await browser.close();
    }}
}})();
"#, display, action_code);
            
            match run_puppeteer_script(&script) {
                Ok(result) => Ok(BrowserNavOutput {
                    success: result.get("success").and_then(|v| v.as_bool()).unwrap_or(false),
                    url: None, title: None, content: None,
                    screenshot_base64: None, script_result: None, semantic_snapshot: None,
                    session_info: None, sessions: None, extracted_content: None,
                    error: result.get("error").and_then(|v| v.as_str()).map(|s| s.to_string()),
                }),
                Err(e) => Ok(BrowserNavOutput {
                    success: false, url: None, title: None, content: None,
                    screenshot_base64: None, script_result: None, semantic_snapshot: None,
                    session_info: None, sessions: None, extracted_content: None,
                    error: Some(e),
                })
            }
        }
        
        BrowserNavInput::ScrollAction { session_id, direction, amount, selector } => {
            let sessions = BROWSER_SESSIONS.lock().unwrap();
            let display = sessions.get(&session_id).map(|s| s.display.clone());
            drop(sessions);
            
            let display = display.ok_or("Session not found")?;
            let scroll_amount = amount.unwrap_or(300);
            
            let action_code = match direction {
                ScrollDirection::Down => {
                    if let Some(sel) = selector {
                        format!("await page.$eval('{}', el => el.scrollBy(0, {}));", sel, scroll_amount)
                    } else {
                        format!("await page.evaluate(() => window.scrollBy(0, {}));", scroll_amount)
                    }
                }
                ScrollDirection::Up => {
                    if let Some(sel) = selector {
                        format!("await page.$eval('{}', el => el.scrollBy(0, -{}));", sel, scroll_amount)
                    } else {
                        format!("await page.evaluate(() => window.scrollBy(0, -{}));", scroll_amount)
                    }
                }
                ScrollDirection::Right => {
                    if let Some(sel) = selector {
                        format!("await page.$eval('{}', el => el.scrollBy({}, 0));", sel, scroll_amount)
                    } else {
                        format!("await page.evaluate(() => window.scrollBy({}, 0));", scroll_amount)
                    }
                }
                ScrollDirection::Left => {
                    if let Some(sel) = selector {
                        format!("await page.$eval('{}', el => el.scrollBy(-{}, 0));", sel, scroll_amount)
                    } else {
                        format!("await page.evaluate(() => window.scrollBy(-{}, 0));", scroll_amount)
                    }
                }
                ScrollDirection::ToElement => {
                    if let Some(sel) = selector {
                        format!("await page.$eval('{}', el => el.scrollIntoView({{ behavior: 'smooth', block: 'center' }}));", sel)
                    } else {
                        return Ok(BrowserNavOutput {
                            success: false, url: None, title: None, content: None,
                            screenshot_base64: None, script_result: None, semantic_snapshot: None,
                            session_info: None, sessions: None, extracted_content: None,
                            error: Some("ToElement scroll requires selector".to_string()),
                        });
                    }
                }
            };
            
            let script = format!(r#"
const puppeteer = require('puppeteer');
process.env.DISPLAY = '{}';
(async () => {{
    const browser = await puppeteer.launch({{ headless: false, args: ['--no-sandbox'] }});
    const page = (await browser.pages())[0] || await browser.newPage();
    try {{
        {}
        console.log(JSON.stringify({{ success: true }}));
    }} catch (error) {{
        console.log(JSON.stringify({{ success: false, error: error.message }}));
    }} finally {{
        await browser.close();
    }}
}})();
"#, display, action_code);
            
            match run_puppeteer_script(&script) {
                Ok(result) => Ok(BrowserNavOutput {
                    success: result.get("success").and_then(|v| v.as_bool()).unwrap_or(false),
                    url: None, title: None, content: None,
                    screenshot_base64: None, script_result: None, semantic_snapshot: None,
                    session_info: None, sessions: None, extracted_content: None,
                    error: result.get("error").and_then(|v| v.as_str()).map(|s| s.to_string()),
                }),
                Err(e) => Ok(BrowserNavOutput {
                    success: false, url: None, title: None, content: None,
                    screenshot_base64: None, script_result: None, semantic_snapshot: None,
                    session_info: None, sessions: None, extracted_content: None,
                    error: Some(e),
                })
            }
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
    
    let input: BrowserNavInput = serde_json::from_str(&input_json).unwrap_or_else(|e| {
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
