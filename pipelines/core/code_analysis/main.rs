//! CodeAnalysisPipeline - Pipeline #18
//! 
//! STRUCTURAL code analysis - extracts syntax, dependencies, complexity.
//! This is a GENERAL-PURPOSE pipeline - NO direct LLM calls.
//! 
//! IMPORTANT: LLM-based semantic analysis is defined in BLUEPRINTS.
//! Pipelines are general-purpose building blocks.
//! Blueprints define the steps and can call prompt pipeline for LLM.
//! 
//! Per spec §8: Code Analysis & Generation System
//! 
//! WHAT THIS PIPELINE DOES (structural only):
//! - Parse code structure (functions, classes, imports)
//! - Extract dependencies and relationships
//! - Calculate complexity metrics
//! - Detect patterns via regex/heuristics
//! - Store analysis in ZSEI
//! - Build/update code graph
//! - BUILD-ON-COMPLETE: Compile check after project marked complete
//! 
//! TRIGGERS (real-time updates):
//! - file_link (new file linked to project)
//! - File modification detected
//! - Manual analysis request
//! - OnFileSave (when project status = Complete)
//! 
//! GRAPH UPDATES: This pipeline updates the code graph in real-time
//! when files are linked or modified. Essential for dependency tracking.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::Path;
use regex::Regex;
use chrono;

// ========== Input Types ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum CodeAnalysisInput {
    /// Analyze code structure (NO LLM - structural only)
    Analyze { 
        code: String, 
        language: String, 
        file_path: Option<String>,
    },
    /// Analyze file by ZSEI reference
    AnalyzeFile { file_ref_id: u64 },
    /// Extract dependencies only
    ExtractDependencies { code: String, language: String },
    /// Get code structure (AST-like)
    GetStructure { code: String, language: String },
    /// Find similar code in ZSEI
    FindSimilar { code: String, limit: Option<u32> },
    /// Analyze complexity metrics
    AnalyzeComplexity { code: String, language: String },
    /// Store analysis in ZSEI
    StoreAnalysis { analysis: CodeAnalysis, project_id: Option<u64> },
    /// Update graph after file change
    UpdateGraph { file_ref_id: u64, change_type: String },
    /// Get dependency graph for project
    GetDependencyGraph { project_id: u64 },
    
    // ========== BUILD-ON-COMPLETE ACTIONS ==========
    /// Build project and capture errors (only when project status = "complete")
    BuildAndCheck { 
        project_id: u64,
        build_command: Option<String>,  // Override default build command
    },
    /// Get build errors for a project
    GetBuildErrors { project_id: u64 },
    /// Mark project as complete (enables build-on-save)
    SetProjectStatus { 
        project_id: u64, 
        status: ProjectStatus,
    },
    /// Triggered on file save when project is "complete"
    OnFileSave {
        project_id: u64,
        file_path: String,
        trigger_build: bool,
    },
}

/// Project status for build-on-complete feature
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProjectStatus {
    /// Still in development - no build triggers
    InDevelopment,
    /// All edge cases defined - build on save enabled
    Complete,
    /// Testing phase - build and run tests on save
    Testing,
    /// Production - build with optimizations
    Production,
}

/// Build result with error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildResult {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub errors: Vec<BuildError>,
    pub warnings: Vec<BuildWarning>,
    pub duration_ms: u64,
    pub command_used: String,
}

/// Parsed build error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildError {
    pub file: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub error_code: Option<String>,
    pub message: String,
    pub suggestion: Option<String>,
}

/// Parsed build warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildWarning {
    pub file: Option<String>,
    pub line: Option<u32>,
    pub message: String,
}

// ========== Output Types ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAnalysis {
    pub language: String,
    pub file_path: Option<String>,
    pub line_count: u32,
    pub char_count: u32,
    pub functions: Vec<FunctionInfo>,
    pub classes: Vec<ClassInfo>,
    pub imports: Vec<ImportInfo>,
    pub exports: Vec<String>,
    pub dependencies: Vec<Dependency>,
    pub complexity: ComplexityMetrics,
    pub keywords: Vec<String>,
    pub topics: Vec<String>,
    pub patterns_detected: Vec<String>,
    // These are populated by BLUEPRINTS calling prompt pipeline, not this pipeline
    pub semantic_summary: Option<String>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub params: Vec<String>,
    pub return_type: Option<String>,
    pub line_start: u32,
    pub line_end: u32,
    pub is_async: bool,
    pub is_public: bool,
    pub calls: Vec<String>,  // Functions this function calls
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInfo {
    pub name: String,
    pub methods: Vec<String>,
    pub properties: Vec<String>,
    pub parent_class: Option<String>,
    pub implements: Vec<String>,
    pub line_start: u32,
    pub line_end: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportInfo {
    pub module: String,
    pub items: Vec<String>,
    pub is_relative: bool,
    pub is_external: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version: Option<String>,
    pub dep_type: String,  // runtime, dev, peer, optional
    pub is_external: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub lines_of_code: u32,
    pub lines_of_comments: u32,
    pub comment_ratio: f32,
    pub nesting_depth_max: u32,
    pub function_count: u32,
    pub class_count: u32,
    pub dependency_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarCode {
    pub container_id: u64,
    pub similarity_score: f32,
    pub file_path: Option<String>,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyNode {
    pub container_id: u64,
    pub file_path: String,
    pub depends_on: Vec<u64>,
    pub depended_by: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAnalysisOutput {
    pub success: bool,
    pub analysis: Option<CodeAnalysis>,
    pub similar: Option<Vec<SimilarCode>>,
    pub graph: Option<Vec<DependencyNode>>,
    pub container_id: Option<u64>,
    pub error: Option<String>,
}

// ========== ZSEI Integration ==========

fn call_zsei_query(input: &serde_json::Value) -> Result<serde_json::Value, String> {
    let zsei_data_path = env::var("OZONE_ZSEI_PATH").unwrap_or_else(|_| "./zsei_data".to_string());
    let action = input.get("action").and_then(|a| a.as_str()).unwrap_or("");
    
    match action {
        "GetContainer" => {
            let container_id = input.get("container_id").and_then(|c| c.as_u64()).unwrap_or(0);
            let container_path = format!("{}/local/{}.json", zsei_data_path, container_id);
            
            if let Ok(content) = std::fs::read_to_string(&container_path) {
                if let Ok(container) = serde_json::from_str::<serde_json::Value>(&content) {
                    return Ok(serde_json::json!({"success": true, "container": container}));
                }
            }
            Ok(serde_json::json!({"success": false, "error": "Container not found"}))
        }
        "Search" => {
            let query = input.get("query").and_then(|q| q.as_str()).unwrap_or("");
            let index_path = format!("{}/indices/code_index.json", zsei_data_path);
            
            if let Ok(content) = std::fs::read_to_string(&index_path) {
                if let Ok(index) = serde_json::from_str::<serde_json::Value>(&content) {
                    let containers = index.get("containers").and_then(|c| c.as_array());
                    if let Some(containers) = containers {
                        let matches: Vec<_> = containers.iter()
                            .filter(|c| {
                                let keywords = c.get("keywords").and_then(|k| k.as_array());
                                keywords.map(|kw| kw.iter().any(|k| {
                                    k.as_str().map(|s| s.to_lowercase().contains(&query.to_lowercase())).unwrap_or(false)
                                })).unwrap_or(false)
                            })
                            .cloned()
                            .collect();
                        return Ok(serde_json::json!({"success": true, "containers": matches}));
                    }
                }
            }
            Ok(serde_json::json!({"success": true, "containers": []}))
        }
        "GetProjectFiles" => {
            let project_id = input.get("project_id").and_then(|p| p.as_u64()).unwrap_or(0);
            let project_path = format!("{}/local/projects/{}", zsei_data_path, project_id);
            
            let mut files = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&project_path) {
                for entry in entries.flatten() {
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        if let Ok(container) = serde_json::from_str::<serde_json::Value>(&content) {
                            files.push(container);
                        }
                    }
                }
            }
            Ok(serde_json::json!({"success": true, "files": files}))
        }
        _ => Ok(serde_json::json!({"success": false, "error": "Unknown action"}))
    }
}

fn call_zsei_write(input: &serde_json::Value) -> Result<serde_json::Value, String> {
    let zsei_data_path = env::var("OZONE_ZSEI_PATH").unwrap_or_else(|_| "./zsei_data".to_string());
    
    let container_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    
    let local_dir = format!("{}/local", zsei_data_path);
    std::fs::create_dir_all(&local_dir).ok();
    
    let container_path = format!("{}/{}.json", local_dir, container_id);
    let container_data = serde_json::json!({
        "container_id": container_id,
        "container_type": "CodeAnalysis",
        "content": input,
        "created_at": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    });
    
    std::fs::write(&container_path, serde_json::to_string_pretty(&container_data).unwrap())
        .map_err(|e| format!("Write failed: {}", e))?;
    
    // Update code index
    update_code_index(&zsei_data_path, container_id, input)?;
    
    Ok(serde_json::json!({"success": true, "container_id": container_id}))
}

fn update_code_index(base_path: &str, container_id: u64, analysis: &serde_json::Value) -> Result<(), String> {
    let index_dir = format!("{}/indices", base_path);
    std::fs::create_dir_all(&index_dir).ok();
    
    let index_path = format!("{}/code_index.json", index_dir);
    let mut index: serde_json::Value = std::fs::read_to_string(&index_path)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_else(|| serde_json::json!({"containers": []}));
    
    let keywords = analysis.get("keywords").cloned().unwrap_or(serde_json::json!([]));
    let file_path = analysis.get("file_path").cloned();
    let language = analysis.get("language").cloned();
    
    let entry = serde_json::json!({
        "container_id": container_id,
        "keywords": keywords,
        "file_path": file_path,
        "language": language,
    });
    
    if let Some(containers) = index.get_mut("containers").and_then(|c| c.as_array_mut()) {
        containers.push(entry);
    }
    
    std::fs::write(&index_path, serde_json::to_string_pretty(&index).unwrap())
        .map_err(|e| format!("Index update failed: {}", e))?;
    
    Ok(())
}

// ========== Analysis Functions (Structural Only - NO LLM) ==========

fn detect_language(code: &str, file_path: Option<&str>) -> String {
    // First try by extension
    if let Some(path) = file_path {
        let ext = Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("");
        match ext {
            "rs" => return "rust".to_string(),
            "py" => return "python".to_string(),
            "js" => return "javascript".to_string(),
            "ts" => return "typescript".to_string(),
            "tsx" => return "typescript".to_string(),
            "jsx" => return "javascript".to_string(),
            "go" => return "go".to_string(),
            "java" => return "java".to_string(),
            "c" | "h" => return "c".to_string(),
            "cpp" | "hpp" | "cc" => return "cpp".to_string(),
            "rb" => return "ruby".to_string(),
            "php" => return "php".to_string(),
            "swift" => return "swift".to_string(),
            "kt" => return "kotlin".to_string(),
            _ => {}
        }
    }
    
    // Then try by content
    if code.contains("fn ") && code.contains("let ") { return "rust".to_string(); }
    if code.contains("def ") && code.contains("import ") { return "python".to_string(); }
    if code.contains("function") && code.contains("const ") { return "javascript".to_string(); }
    if code.contains("interface ") && code.contains(": string") { return "typescript".to_string(); }
    if code.contains("package main") { return "go".to_string(); }
    if code.contains("public class") { return "java".to_string(); }
    
    "unknown".to_string()
}

fn parse_structure(code: &str, language: &str) -> (Vec<FunctionInfo>, Vec<ClassInfo>) {
    let mut functions = Vec::new();
    let mut classes = Vec::new();
    let lines: Vec<&str> = code.lines().collect();
    
    let (fn_patterns, class_patterns): (Vec<&str>, Vec<&str>) = match language.to_lowercase().as_str() {
        "rust" => (
            vec![r"(pub\s+)?(async\s+)?fn\s+(\w+)"],
            vec![r"(pub\s+)?struct\s+(\w+)", r"(pub\s+)?enum\s+(\w+)", r"impl\s+(\w+)"]
        ),
        "python" => (
            vec![r"(async\s+)?def\s+(\w+)"],
            vec![r"class\s+(\w+)"]
        ),
        "javascript" | "typescript" => (
            vec![r"(async\s+)?function\s+(\w+)", r"(\w+)\s*=\s*\(?.*\)?\s*=>", r"(\w+)\s*=\s*async\s*\("],
            vec![r"class\s+(\w+)"]
        ),
        "go" => (
            vec![r"func\s+(\w+)"],
            vec![r"type\s+(\w+)\s+struct"]
        ),
        "java" | "kotlin" => (
            vec![r"(public|private|protected)?\s*(static)?\s*\w+\s+(\w+)\s*\("],
            vec![r"(public|private)?\s*class\s+(\w+)"]
        ),
        _ => (
            vec![r"function\s+(\w+)", r"def\s+(\w+)", r"fn\s+(\w+)", r"func\s+(\w+)"],
            vec![r"class\s+(\w+)", r"struct\s+(\w+)"]
        ),
    };
    
    for (line_num, line) in lines.iter().enumerate() {
        // Function detection
        for pattern in &fn_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(caps) = re.captures(line) {
                    let name = caps.iter().skip(1).flatten().last()
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_default();
                    if !name.is_empty() && name != "if" && name != "for" && name != "while" {
                        functions.push(FunctionInfo {
                            name,
                            params: extract_params(line),
                            return_type: extract_return_type(line, language),
                            line_start: line_num as u32 + 1,
                            line_end: find_block_end(&lines, line_num) as u32,
                            is_async: line.contains("async"),
                            is_public: line.contains("pub") || line.contains("public") || !line.trim().starts_with("_"),
                            calls: vec![],  // Would require deeper parsing
                        });
                    }
                }
            }
        }
        
        // Class detection
        for pattern in &class_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(caps) = re.captures(line) {
                    let name = caps.iter().skip(1).flatten().last()
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_default();
                    if !name.is_empty() {
                        classes.push(ClassInfo {
                            name,
                            methods: vec![],
                            properties: vec![],
                            parent_class: extract_parent_class(line),
                            implements: vec![],
                            line_start: line_num as u32 + 1,
                            line_end: find_block_end(&lines, line_num) as u32,
                        });
                    }
                }
            }
        }
    }
    
    (functions, classes)
}

fn extract_params(line: &str) -> Vec<String> {
    if let Some(start) = line.find('(') {
        if let Some(end) = line.find(')') {
            let params_str = &line[start+1..end];
            return params_str.split(',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect();
        }
    }
    vec![]
}

fn extract_return_type(line: &str, language: &str) -> Option<String> {
    match language {
        "rust" => {
            if let Some(arrow) = line.find("->") {
                let rest = &line[arrow+2..];
                let end = rest.find('{').or(rest.find("where")).unwrap_or(rest.len());
                return Some(rest[..end].trim().to_string());
            }
        }
        "typescript" => {
            if let Some(colon) = line.rfind("):") {
                let rest = &line[colon+2..];
                let end = rest.find('{').or(rest.find("=>")).unwrap_or(rest.len());
                return Some(rest[..end].trim().to_string());
            }
        }
        _ => {}
    }
    None
}

fn extract_parent_class(line: &str) -> Option<String> {
    if let Some(ext) = line.find("extends ") {
        let rest = &line[ext+8..];
        let end = rest.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(rest.len());
        return Some(rest[..end].to_string());
    }
    if line.contains("(") {
        // Python class Foo(Bar)
        if let Some(start) = line.find('(') {
            if let Some(end) = line.find(')') {
                let parent = &line[start+1..end];
                if !parent.is_empty() && !parent.contains(",") {
                    return Some(parent.to_string());
                }
            }
        }
    }
    None
}

fn find_block_end(lines: &[&str], start: usize) -> usize {
    let mut depth = 0;
    let mut found_start = false;
    
    for (i, line) in lines.iter().enumerate().skip(start) {
        for c in line.chars() {
            match c {
                '{' | '(' if !found_start => { found_start = true; depth = 1; }
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 && found_start { return i + 1; }
                }
                _ => {}
            }
        }
        // Python uses indentation
        if i > start && !lines[i].starts_with(' ') && !lines[i].starts_with('\t') && !lines[i].is_empty() {
            return i;
        }
    }
    lines.len()
}

fn extract_imports(code: &str, language: &str) -> Vec<ImportInfo> {
    let mut imports = Vec::new();
    
    let patterns: Vec<(&str, bool)> = match language.to_lowercase().as_str() {
        "rust" => vec![
            (r"use\s+([\w:]+)", false),
            (r"use\s+crate::([\w:]+)", false),
            (r"extern\s+crate\s+(\w+)", true),
        ],
        "python" => vec![
            (r"from\s+([\w.]+)\s+import", false),
            (r"^import\s+([\w.]+)", true),
        ],
        "javascript" | "typescript" => vec![
            (r#"import\s+.*from\s+['"]([\w@/.-]+)['"]"#, true),
            (r#"require\(['"]([\w@/.-]+)['"]\)"#, true),
        ],
        "go" => vec![
            (r#"import\s+["\(]([^"]+)["\)]"#, true),
        ],
        _ => vec![
            (r"import\s+([\w.]+)", true),
        ],
    };
    
    for (pattern, is_external) in patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            for caps in re.captures_iter(code) {
                if let Some(m) = caps.get(1) {
                    let module = m.as_str().to_string();
                    let is_rel = module.starts_with('.') || module.starts_with("crate::");
                    imports.push(ImportInfo {
                        module,
                        items: vec![],
                        is_relative: is_rel,
                        is_external: is_external && !is_rel,
                    });
                }
            }
        }
    }
    
    // Deduplicate
    imports.sort_by(|a, b| a.module.cmp(&b.module));
    imports.dedup_by(|a, b| a.module == b.module);
    
    imports
}

fn calculate_complexity(code: &str) -> ComplexityMetrics {
    let lines: Vec<&str> = code.lines().collect();
    let total_lines = lines.len();
    
    let code_lines = lines.iter()
        .filter(|l| {
            let trimmed = l.trim();
            !trimmed.is_empty() && 
            !trimmed.starts_with("//") && 
            !trimmed.starts_with("#") &&
            !trimmed.starts_with("/*") &&
            !trimmed.starts_with("*")
        })
        .count();
    
    let comment_lines = lines.iter()
        .filter(|l| {
            let trimmed = l.trim();
            trimmed.starts_with("//") || 
            trimmed.starts_with("#") ||
            trimmed.starts_with("/*") ||
            trimmed.starts_with("*")
        })
        .count();
    
    // Cyclomatic complexity (decision points)
    let decision_keywords = ["if", "else", "elif", "for", "while", "match", "case", "catch", "?", "&&", "||"];
    let cyclomatic: u32 = decision_keywords.iter()
        .map(|kw| {
            if kw.len() <= 2 {
                code.matches(kw).count() as u32
            } else {
                // Word boundary check for keywords
                code.split_whitespace()
                    .filter(|w| w.trim_matches(|c: char| !c.is_alphanumeric()) == *kw)
                    .count() as u32
            }
        })
        .sum::<u32>() + 1;
    
    // Nesting depth
    let mut max_depth = 0u32;
    let mut current_depth = 0u32;
    for c in code.chars() {
        match c {
            '{' | '(' => { current_depth += 1; max_depth = max_depth.max(current_depth); }
            '}' | ')' => current_depth = current_depth.saturating_sub(1),
            _ => {}
        }
    }
    
    let fn_count = code.matches("fn ").count() + 
                   code.matches("def ").count() + 
                   code.matches("function ").count() +
                   code.matches("func ").count();
    let class_count = code.matches("class ").count() + 
                      code.matches("struct ").count() +
                      code.matches("interface ").count();
    
    let imports = extract_imports(code, "unknown");
    
    ComplexityMetrics {
        cyclomatic,
        cognitive: cyclomatic + max_depth,
        lines_of_code: code_lines as u32,
        lines_of_comments: comment_lines as u32,
        comment_ratio: if total_lines > 0 { comment_lines as f32 / total_lines as f32 } else { 0.0 },
        nesting_depth_max: max_depth,
        function_count: fn_count as u32,
        class_count: class_count as u32,
        dependency_count: imports.len() as u32,
    }
}

fn extract_keywords(code: &str, language: &str) -> (Vec<String>, Vec<String>) {
    let mut keywords = vec![language.to_lowercase()];
    let mut topics = Vec::new();
    
    // Topic detection via patterns (NO LLM)
    let topic_patterns = [
        ("async", vec!["async", "await", "Promise", "Future"]),
        ("testing", vec!["test", "Test", "spec", "expect", "assert"]),
        ("networking", vec!["http", "fetch", "request", "socket", "tcp", "udp"]),
        ("database", vec!["sql", "database", "query", "SELECT", "INSERT", "mongo"]),
        ("cryptography", vec!["encrypt", "decrypt", "hash", "crypto", "sha", "md5"]),
        ("serialization", vec!["json", "serialize", "deserialize", "parse", "stringify"]),
        ("error_handling", vec!["error", "Error", "exception", "catch", "Result"]),
        ("logging", vec!["log", "debug", "info", "warn", "error", "trace"]),
        ("ui", vec!["render", "component", "view", "DOM", "React", "Vue"]),
        ("api", vec!["api", "endpoint", "route", "REST", "GraphQL"]),
        ("auth", vec!["auth", "login", "token", "jwt", "oauth"]),
        ("file_io", vec!["file", "read", "write", "fs", "path"]),
        ("concurrency", vec!["thread", "mutex", "channel", "spawn", "parallel"]),
    ];
    
    for (topic, patterns) in topic_patterns {
        if patterns.iter().any(|p| code.contains(p)) {
            topics.push(topic.to_string());
        }
    }
    
    // Extract identifiers as keywords (simplified)
    let identifier_re = regex::Regex::new(r"\b([A-Z][a-zA-Z0-9]+)\b").unwrap();
    for caps in identifier_re.captures_iter(code) {
        if let Some(m) = caps.get(1) {
            let ident = m.as_str().to_string();
            if ident.len() > 2 && !keywords.contains(&ident.to_lowercase()) {
                keywords.push(ident.to_lowercase());
            }
        }
    }
    
    // Limit keywords
    keywords.truncate(20);
    
    (keywords, topics)
}

fn detect_patterns(code: &str, language: &str) -> Vec<String> {
    let mut patterns = Vec::new();
    
    // Structural pattern detection (NO LLM)
    if code.contains("singleton") || (code.contains("static") && code.contains("instance")) {
        patterns.push("singleton".to_string());
    }
    if code.contains("factory") || code.contains("Factory") {
        patterns.push("factory".to_string());
    }
    if code.contains("observer") || code.contains("subscribe") || code.contains("emit") {
        patterns.push("observer".to_string());
    }
    if code.contains("async") && code.contains("await") {
        patterns.push("async_await".to_string());
    }
    if code.contains("Result<") || code.contains("Result[") {
        patterns.push("result_type".to_string());
    }
    if code.contains("Option<") || code.contains("Optional") {
        patterns.push("optional_type".to_string());
    }
    if code.contains("impl") && code.contains("trait") {
        patterns.push("trait_impl".to_string());
    }
    if code.contains("interface") && code.contains("implements") {
        patterns.push("interface_impl".to_string());
    }
    
    patterns
}

// ========== Main Execution ==========

pub async fn execute(input: CodeAnalysisInput) -> Result<CodeAnalysisOutput, String> {
    match input {
        CodeAnalysisInput::Analyze { code, language, file_path } => {
            // Detect language if not provided or "unknown"
            let lang = if language == "unknown" || language.is_empty() {
                detect_language(&code, file_path.as_deref())
            } else {
                language
            };
            
            // Structural analysis (NO LLM)
            let (functions, classes) = parse_structure(&code, &lang);
            let imports = extract_imports(&code, &lang);
            let complexity = calculate_complexity(&code);
            let (keywords, topics) = extract_keywords(&code, &lang);
            let patterns = detect_patterns(&code, &lang);
            
            // Build dependencies from imports
            let dependencies: Vec<Dependency> = imports.iter()
                .filter(|i| i.is_external)
                .map(|i| Dependency {
                    name: i.module.clone(),
                    version: None,
                    dep_type: "runtime".to_string(),
                    is_external: true,
                })
                .collect();
            
            let analysis = CodeAnalysis {
                language: lang,
                file_path,
                line_count: code.lines().count() as u32,
                char_count: code.len() as u32,
                functions,
                classes,
                imports,
                exports: vec![],
                dependencies,
                complexity,
                keywords,
                topics,
                patterns_detected: patterns,
                semantic_summary: None,  // Populated by BLUEPRINT calling prompt
                suggestions: vec![],     // Populated by BLUEPRINT calling prompt
            };
            
            Ok(CodeAnalysisOutput {
                success: true,
                analysis: Some(analysis),
                similar: None,
                graph: None,
                container_id: None,
                error: None,
            })
        }
        
        CodeAnalysisInput::AnalyzeFile { file_ref_id } => {
            let query = serde_json::json!({"action": "GetContainer", "container_id": file_ref_id});
            let result = call_zsei_query(&query)?;
            
            if let Some(container) = result.get("container") {
                let file_path = container.get("file_path")
                    .or_else(|| container.get("content").and_then(|c| c.get("file_path")))
                    .and_then(|p| p.as_str());
                
                let language = container.get("language")
                    .or_else(|| container.get("content").and_then(|c| c.get("language")))
                    .and_then(|l| l.as_str())
                    .unwrap_or("unknown");
                
                if let Some(path) = file_path {
                    if let Ok(code) = std::fs::read_to_string(path) {
                        return Box::pin(execute(CodeAnalysisInput::Analyze {
                            code,
                            language: language.to_string(),
                            file_path: Some(path.to_string()),
                        })).await;
                    }
                }
            }
            
            Ok(CodeAnalysisOutput {
                success: false,
                analysis: None,
                similar: None,
                graph: None,
                container_id: None,
                error: Some("File not found in ZSEI".to_string()),
            })
        }
        
        CodeAnalysisInput::ExtractDependencies { code, language } => {
            let imports = extract_imports(&code, &language);
            let dependencies: Vec<Dependency> = imports.iter()
                .map(|i| Dependency {
                    name: i.module.clone(),
                    version: None,
                    dep_type: "runtime".to_string(),
                    is_external: i.is_external,
                })
                .collect();
            
            Ok(CodeAnalysisOutput {
                success: true,
                analysis: Some(CodeAnalysis {
                    language,
                    file_path: None,
                    line_count: 0,
                    char_count: 0,
                    functions: vec![],
                    classes: vec![],
                    imports,
                    exports: vec![],
                    dependencies,
                    complexity: ComplexityMetrics {
                        cyclomatic: 0, cognitive: 0, lines_of_code: 0, lines_of_comments: 0,
                        comment_ratio: 0.0, nesting_depth_max: 0, function_count: 0,
                        class_count: 0, dependency_count: 0,
                    },
                    keywords: vec![],
                    topics: vec![],
                    patterns_detected: vec![],
                    semantic_summary: None,
                    suggestions: vec![],
                }),
                similar: None,
                graph: None,
                container_id: None,
                error: None,
            })
        }
        
        CodeAnalysisInput::GetStructure { code, language } => {
            let (functions, classes) = parse_structure(&code, &language);
            
            Ok(CodeAnalysisOutput {
                success: true,
                analysis: Some(CodeAnalysis {
                    language,
                    file_path: None,
                    line_count: code.lines().count() as u32,
                    char_count: code.len() as u32,
                    functions,
                    classes,
                    imports: vec![],
                    exports: vec![],
                    dependencies: vec![],
                    complexity: ComplexityMetrics {
                        cyclomatic: 0, cognitive: 0, lines_of_code: 0, lines_of_comments: 0,
                        comment_ratio: 0.0, nesting_depth_max: 0, function_count: 0,
                        class_count: 0, dependency_count: 0,
                    },
                    keywords: vec![],
                    topics: vec![],
                    patterns_detected: vec![],
                    semantic_summary: None,
                    suggestions: vec![],
                }),
                similar: None,
                graph: None,
                container_id: None,
                error: None,
            })
        }
        
        CodeAnalysisInput::FindSimilar { code, limit } => {
            let (keywords, _) = extract_keywords(&code, "unknown");
            let query = serde_json::json!({
                "action": "Search",
                "query": keywords.join(" "),
            });
            
            let result = call_zsei_query(&query)?;
            let similar: Vec<SimilarCode> = result.get("containers")
                .and_then(|c| c.as_array())
                .map(|arr| arr.iter().take(limit.unwrap_or(10) as usize).filter_map(|v| {
                    Some(SimilarCode {
                        container_id: v.get("container_id")?.as_u64()?,
                        similarity_score: 0.8,  // Would compute actual similarity
                        file_path: v.get("file_path").and_then(|p| p.as_str()).map(|s| s.to_string()),
                        language: v.get("language").and_then(|l| l.as_str()).unwrap_or("unknown").to_string(),
                    })
                }).collect())
                .unwrap_or_default();
            
            Ok(CodeAnalysisOutput {
                success: true,
                analysis: None,
                similar: Some(similar),
                graph: None,
                container_id: None,
                error: None,
            })
        }
        
        CodeAnalysisInput::AnalyzeComplexity { code, language } => {
            let complexity = calculate_complexity(&code);
            
            Ok(CodeAnalysisOutput {
                success: true,
                analysis: Some(CodeAnalysis {
                    language,
                    file_path: None,
                    line_count: code.lines().count() as u32,
                    char_count: code.len() as u32,
                    functions: vec![],
                    classes: vec![],
                    imports: vec![],
                    exports: vec![],
                    dependencies: vec![],
                    complexity,
                    keywords: vec![],
                    topics: vec![],
                    patterns_detected: vec![],
                    semantic_summary: None,
                    suggestions: vec![],
                }),
                similar: None,
                graph: None,
                container_id: None,
                error: None,
            })
        }
        
        CodeAnalysisInput::StoreAnalysis { analysis, project_id } => {
            let data = serde_json::json!({
                "analysis": analysis,
                "project_id": project_id,
            });
            
            let result = call_zsei_write(&data)?;
            let container_id = result.get("container_id").and_then(|c| c.as_u64());
            
            Ok(CodeAnalysisOutput {
                success: true,
                analysis: Some(analysis),
                similar: None,
                graph: None,
                container_id,
                error: None,
            })
        }
        
        CodeAnalysisInput::UpdateGraph { file_ref_id, change_type } => {
            // Re-analyze the file and update ZSEI
            let result = Box::pin(execute(CodeAnalysisInput::AnalyzeFile { file_ref_id })).await?;
            
            if let Some(analysis) = result.analysis {
                // Store updated analysis
                let _ = Box::pin(execute(CodeAnalysisInput::StoreAnalysis {
                    analysis: analysis.clone(),
                    project_id: None,
                })).await?;
            }
            
            Ok(CodeAnalysisOutput {
                success: true,
                analysis: None,
                similar: None,
                graph: None,
                container_id: None,
                error: None,
            })
        }
        
        CodeAnalysisInput::GetDependencyGraph { project_id } => {
            let query = serde_json::json!({
                "action": "GetProjectFiles",
                "project_id": project_id,
            });
            
            let result = call_zsei_query(&query)?;
            let mut nodes = Vec::new();
            
            if let Some(files) = result.get("files").and_then(|f| f.as_array()) {
                for file in files {
                    let container_id = file.get("container_id").and_then(|c| c.as_u64()).unwrap_or(0);
                    let file_path = file.get("file_path").and_then(|p| p.as_str()).unwrap_or("").to_string();
                    
                    // Get dependencies from analysis
                    let depends_on: Vec<u64> = file.get("content")
                        .and_then(|c| c.get("analysis"))
                        .and_then(|a| a.get("imports"))
                        .and_then(|i| i.as_array())
                        .map(|arr| arr.iter()
                            .filter_map(|i| i.get("container_id").and_then(|c| c.as_u64()))
                            .collect())
                        .unwrap_or_default();
                    
                    nodes.push(DependencyNode {
                        container_id,
                        file_path,
                        depends_on,
                        depended_by: vec![],  // Would compute reverse dependencies
                    });
                }
            }
            
            Ok(CodeAnalysisOutput {
                success: true,
                analysis: None,
                similar: None,
                graph: Some(nodes),
                container_id: None,
                error: None,
            })
        }
        
        // ========== BUILD-ON-COMPLETE HANDLERS ==========
        
        CodeAnalysisInput::BuildAndCheck { project_id, build_command } => {
            // Get project info to determine build command
            let query = serde_json::json!({
                "action": "GetContainer",
                "container_id": project_id,
            });
            let project = call_zsei_query(&query)?;
            
            // Determine language/build system
            let language = project.get("container")
                .and_then(|c| c.get("content"))
                .and_then(|c| c.get("primary_language"))
                .and_then(|l| l.as_str())
                .unwrap_or("rust");
            
            let project_path = project.get("container")
                .and_then(|c| c.get("content"))
                .and_then(|c| c.get("path"))
                .and_then(|p| p.as_str())
                .unwrap_or(".");
            
            // Default build commands by language
            let cmd = build_command.unwrap_or_else(|| {
                match language {
                    "rust" => "cargo build --release 2>&1".to_string(),
                    "javascript" | "typescript" => "npm run build 2>&1".to_string(),
                    "python" => "python -m py_compile . 2>&1".to_string(),
                    "go" => "go build ./... 2>&1".to_string(),
                    "java" => "mvn compile 2>&1".to_string(),
                    _ => "make 2>&1".to_string(),
                }
            });
            
            // Execute build
            let start = std::time::Instant::now();
            let output = std::process::Command::new("sh")
                .arg("-c")
                .arg(&cmd)
                .current_dir(project_path)
                .output();
            let duration_ms = start.elapsed().as_millis() as u64;
            
            match output {
                Ok(result) => {
                    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&result.stderr).to_string();
                    let combined = format!("{}\n{}", stdout, stderr);
                    
                    // Parse errors based on language
                    let (errors, warnings) = parse_build_output(&combined, language);
                    
                    let build_result = BuildResult {
                        success: result.status.success(),
                        exit_code: result.status.code().unwrap_or(-1),
                        stdout,
                        stderr,
                        errors,
                        warnings,
                        duration_ms,
                        command_used: cmd,
                    };
                    
                    // Store build result in ZSEI for context inclusion
                    let store_query = serde_json::json!({
                        "action": "StoreContainer",
                        "container": {
                            "container_type": "BuildLog",
                            "project_id": project_id,
                            "timestamp": chrono::Utc::now().timestamp(),
                            "content": build_result,
                        }
                    });
                    let _ = call_zsei_query(&store_query);
                    
                    Ok(CodeAnalysisOutput {
                        success: true,
                        analysis: None,
                        similar: None,
                        graph: None,
                        container_id: None,
                        error: if build_result.success { None } else { 
                            Some(format!("{} errors, {} warnings", 
                                build_result.errors.len(), 
                                build_result.warnings.len()))
                        },
                    })
                }
                Err(e) => Ok(CodeAnalysisOutput {
                    success: false,
                    analysis: None,
                    similar: None,
                    graph: None,
                    container_id: None,
                    error: Some(format!("Build failed to execute: {}", e)),
                })
            }
        }
        
        CodeAnalysisInput::GetBuildErrors { project_id } => {
            // Query ZSEI for latest build logs
            let query = serde_json::json!({
                "action": "Traverse",
                "start_container": project_id,
                "filters": [{"field": "container_type", "op": "eq", "value": "BuildLog"}],
                "max_results": 1,
            });
            
            let result = call_zsei_query(&query)?;
            
            // Return latest build errors for context inclusion
            if let Some(containers) = result.get("containers").and_then(|c| c.as_array()) {
                if let Some(latest) = containers.first() {
                    return Ok(CodeAnalysisOutput {
                        success: true,
                        analysis: None,
                        similar: None,
                        graph: None,
                        container_id: latest.get("container_id").and_then(|c| c.as_u64()),
                        error: None,
                    });
                }
            }
            
            Ok(CodeAnalysisOutput {
                success: true,
                analysis: None,
                similar: None,
                graph: None,
                container_id: None,
                error: Some("No build logs found".to_string()),
            })
        }
        
        CodeAnalysisInput::SetProjectStatus { project_id, status } => {
            // Update project status in ZSEI
            let query = serde_json::json!({
                "action": "UpdateContainer",
                "container_id": project_id,
                "updates": {
                    "metadata": {
                        "project_status": status,
                        "build_on_save_enabled": status == ProjectStatus::Complete 
                            || status == ProjectStatus::Testing 
                            || status == ProjectStatus::Production,
                    }
                }
            });
            
            let _ = call_zsei_query(&query)?;
            
            Ok(CodeAnalysisOutput {
                success: true,
                analysis: None,
                similar: None,
                graph: None,
                container_id: Some(project_id),
                error: None,
            })
        }
        
        CodeAnalysisInput::OnFileSave { project_id, file_path, trigger_build } => {
            // Check if project has build-on-save enabled
            let query = serde_json::json!({
                "action": "GetContainer",
                "container_id": project_id,
            });
            let project = call_zsei_query(&query)?;
            
            let build_enabled = project.get("container")
                .and_then(|c| c.get("metadata"))
                .and_then(|m| m.get("build_on_save_enabled"))
                .and_then(|b| b.as_bool())
                .unwrap_or(false);
            
            if !build_enabled || !trigger_build {
                // Just update the file analysis, no build
                return Ok(CodeAnalysisOutput {
                    success: true,
                    analysis: None,
                    similar: None,
                    graph: None,
                    container_id: None,
                    error: None,
                });
            }
            
            // Trigger build and capture errors
            let build_result = Box::pin(execute(CodeAnalysisInput::BuildAndCheck {
                project_id,
                build_command: None,
            })).await?;
            
            Ok(build_result)
        }
    }
}

/// Parse build output to extract structured errors and warnings
fn parse_build_output(output: &str, language: &str) -> (Vec<BuildError>, Vec<BuildWarning>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    
    match language {
        "rust" => {
            // Rust error format: error[E0XXX]: message
            //   --> file.rs:line:col
            let error_re = regex::Regex::new(r"error\[([A-Z]\d+)\]: (.+)").unwrap();
            let location_re = regex::Regex::new(r"--> (.+):(\d+):(\d+)").unwrap();
            let warning_re = regex::Regex::new(r"warning: (.+)").unwrap();
            
            let mut current_error: Option<BuildError> = None;
            
            for line in output.lines() {
                if let Some(caps) = error_re.captures(line) {
                    // Save previous error if exists
                    if let Some(err) = current_error.take() {
                        errors.push(err);
                    }
                    current_error = Some(BuildError {
                        file: None,
                        line: None,
                        column: None,
                        error_code: Some(caps[1].to_string()),
                        message: caps[2].to_string(),
                        suggestion: None,
                    });
                } else if let Some(caps) = location_re.captures(line) {
                    if let Some(ref mut err) = current_error {
                        err.file = Some(caps[1].to_string());
                        err.line = caps[2].parse().ok();
                        err.column = caps[3].parse().ok();
                    }
                } else if let Some(caps) = warning_re.captures(line) {
                    warnings.push(BuildWarning {
                        file: None,
                        line: None,
                        message: caps[1].to_string(),
                    });
                }
            }
            
            if let Some(err) = current_error {
                errors.push(err);
            }
        }
        "javascript" | "typescript" => {
            // TypeScript/ESLint format: file(line,col): error TS####: message
            let error_re = regex::Regex::new(r"(.+)\((\d+),(\d+)\): error (TS\d+): (.+)").unwrap();
            
            for line in output.lines() {
                if let Some(caps) = error_re.captures(line) {
                    errors.push(BuildError {
                        file: Some(caps[1].to_string()),
                        line: caps[2].parse().ok(),
                        column: caps[3].parse().ok(),
                        error_code: Some(caps[4].to_string()),
                        message: caps[5].to_string(),
                        suggestion: None,
                    });
                }
            }
        }
        _ => {
            // Generic: look for "error" lines
            for line in output.lines() {
                let lower = line.to_lowercase();
                if lower.contains("error") {
                    errors.push(BuildError {
                        file: None,
                        line: None,
                        column: None,
                        error_code: None,
                        message: line.to_string(),
                        suggestion: None,
                    });
                } else if lower.contains("warning") {
                    warnings.push(BuildWarning {
                        file: None,
                        line: None,
                        message: line.to_string(),
                    });
                }
            }
        }
    }
    
    (errors, warnings)
}

// ========== CLI Entry Point ==========

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    
    for i in 1..args.len() {
        if args[i] == "--input" && i + 1 < args.len() {
            input_json = args[i + 1].clone();
        }
    }
    
    let input: CodeAnalysisInput = serde_json::from_str(&input_json).unwrap_or_else(|e| {
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
