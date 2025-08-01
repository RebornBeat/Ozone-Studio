// FORGE Primitive Operations Module
// 
// This module defines the minimal, built-in capabilities that FORGE's static core
// possesses before any methodologies are loaded. These are the "basic reflexes" 
// that enable FORGE to function as an ecosystem participant and methodology executor.
//
// Key Principle: These operations are intentionally LIMITED to prevent duplication
// with methodology-based capabilities. Sophisticated analysis, generation, and 
// modification capabilities come through methodology loading and ecosystem coordination.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, Instant};
use std::sync::Arc;
use std::io::{self, BufRead, BufReader};
use std::fs::File;

use tokio::sync::{RwLock, Mutex};
use serde::{Serialize, Deserialize};
use regex::Regex;
use anyhow::Result;
use thiserror::Error;

use crate::{
    ProgrammingLanguage, 
    ForgeError, 
    BasicParseResult,
    SimpleCodeStructure,
    BasicFileInfo,
    CoordinationState,
    PrimitiveCapability,
    LanguageInfo,
    DetectionMethod,
    BasicComplexity,
};

use shared_protocols::{
    AIAppCoordinationRequest,
    AIAppCoordinationResponse,
    ExecutionStatus,
};

use methodology_runtime::{
    Instruction,
    ExecutionContext,
    ExecutionResult,
};

/// Errors that can occur during primitive operations
#[derive(Debug, Error)]
pub enum PrimitiveOperationError {
    #[error("Parsing failed: {0}")]
    ParsingFailed(String),
    
    #[error("Language detection failed: {0}")]
    LanguageDetectionFailed(String),
    
    #[error("File operation failed: {0}")]
    FileOperationFailed(String),
    
    #[error("Coordination failed: {0}")]
    CoordinationFailed(String),
    
    #[error("Primitive capability not supported: {0}")]
    CapabilityNotSupported(String),
    
    #[error("Operation timeout: {0}")]
    OperationTimeout(String),
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// Configuration for primitive operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveOperationsConfig {
    /// Maximum file size for primitive parsing (1MB default)
    pub max_file_size: u64,
    
    /// Timeout for primitive operations (5 seconds default)
    pub operation_timeout: Duration,
    
    /// Maximum lines to analyze for structure detection (1000 default)
    pub max_lines_for_structure: u32,
    
    /// Whether to cache parsing results
    pub enable_caching: bool,
    
    /// Maximum cache size in bytes
    pub max_cache_size: u64,
}

impl Default for PrimitiveOperationsConfig {
    fn default() -> Self {
        Self {
            max_file_size: 1024 * 1024, // 1MB
            operation_timeout: Duration::from_secs(5),
            max_lines_for_structure: 1000,
            enable_caching: true,
            max_cache_size: 10 * 1024 * 1024, // 10MB
        }
    }
}

// =============================================================================
// BASIC CODE PARSER - Primitive code parsing capabilities
// =============================================================================

/// Basic code parser that can recognize fundamental code elements
/// This is intentionally limited - sophisticated analysis requires methodologies
pub struct BasicCodeParser {
    config: PrimitiveOperationsConfig,
    parse_cache: Arc<RwLock<HashMap<String, CachedParseResult>>>,
}

/// Cached result of basic parsing operation
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedParseResult {
    result: BasicParseResult,
    timestamp: SystemTime,
    file_hash: String,
}

/// Request for basic code parsing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicParseRequest {
    pub file_path: String,
    pub content: Option<String>, // If None, will read from file via NEXUS
    pub language_hint: Option<ProgrammingLanguage>,
    pub parse_depth: ParseDepth,
}

/// How deeply to parse - primitive operations stay shallow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParseDepth {
    /// Only detect file type and language
    FileTypeOnly,
    /// Basic structure detection (functions, classes, imports)
    BasicStructure,
    /// Include line counting and basic metrics
    WithMetrics,
}

impl BasicCodeParser {
    pub fn new(config: PrimitiveOperationsConfig) -> Self {
        Self {
            config,
            parse_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Parse a code file using primitive parsing capabilities only
    pub async fn parse_file(&self, request: BasicParseRequest) -> Result<BasicParseResult, PrimitiveOperationError> {
        // Check cache first if enabled
        if self.config.enable_caching {
            if let Some(cached) = self.check_cache(&request.file_path).await {
                return Ok(cached.result);
            }
        }
        
        // Read file content (this would coordinate with NEXUS in real implementation)
        let content = match request.content {
            Some(content) => content,
            None => self.read_file_content(&request.file_path).await?,
        };
        
        // Check file size limit for primitive operations
        if content.len() as u64 > self.config.max_file_size {
            return Err(PrimitiveOperationError::InvalidInput(
                format!("File too large for primitive parsing: {} bytes", content.len())
            ));
        }
        
        // Perform basic parsing based on requested depth
        let result = match request.parse_depth {
            ParseDepth::FileTypeOnly => self.parse_file_type_only(&content, &request.file_path),
            ParseDepth::BasicStructure => self.parse_basic_structure(&content, &request.file_path, request.language_hint),
            ParseDepth::WithMetrics => self.parse_with_metrics(&content, &request.file_path, request.language_hint),
        }?;
        
        // Cache result if enabled
        if self.config.enable_caching {
            self.cache_result(&request.file_path, &result, &content).await;
        }
        
        Ok(result)
    }
    
    /// Parse only file type and language - most basic primitive operation
    fn parse_file_type_only(&self, content: &str, file_path: &str) -> Result<BasicParseResult, PrimitiveOperationError> {
        let language = self.detect_language(file_path, Some(content))?;
        
        Ok(BasicParseResult {
            file_path: file_path.to_string(),
            language: Some(language),
            structure: None,
            metrics: None,
            errors: Vec::new(),
            warnings: Vec::new(),
            processing_time: Duration::from_millis(1), // Minimal processing time
        })
    }
    
    /// Parse basic code structure - limited to obvious structural elements
    fn parse_basic_structure(&self, content: &str, file_path: &str, language_hint: Option<ProgrammingLanguage>) -> Result<BasicParseResult, PrimitiveOperationError> {
        let language = language_hint.unwrap_or_else(|| 
            self.detect_language(file_path, Some(content)).unwrap_or(ProgrammingLanguage::Other("unknown".to_string()))
        );
        
        let start_time = Instant::now();
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        
        // Basic structure detection using simple pattern matching
        // This is intentionally limited - sophisticated AST parsing requires methodologies
        let structure = match self.extract_basic_structure(content, &language) {
            Ok(structure) => Some(structure),
            Err(e) => {
                errors.push(format!("Structure extraction failed: {}", e));
                None
            }
        };
        
        Ok(BasicParseResult {
            file_path: file_path.to_string(),
            language: Some(language),
            structure,
            metrics: None,
            errors,
            warnings,
            processing_time: start_time.elapsed(),
        })
    }
    
    /// Parse with basic metrics - adds line counting and simple complexity estimation
    fn parse_with_metrics(&self, content: &str, file_path: &str, language_hint: Option<ProgrammingLanguage>) -> Result<BasicParseResult, PrimitiveOperationError> {
        let mut result = self.parse_basic_structure(content, file_path, language_hint)?;
        
        // Add basic metrics
        let lines: Vec<&str> = content.lines().collect();
        let line_count = lines.len() as u32;
        let character_count = content.len() as u64;
        
        // Estimate complexity based on very basic heuristics
        let complexity = self.estimate_basic_complexity(&lines);
        
        if let Some(ref mut structure) = result.structure {
            structure.line_count = line_count;
            structure.character_count = character_count;
            structure.estimated_complexity = complexity;
        }
        
        Ok(result)
    }
    
    /// Extract basic code structure using simple pattern matching
    /// This is intentionally primitive - no AST parsing or sophisticated analysis
    fn extract_basic_structure(&self, content: &str, language: &ProgrammingLanguage) -> Result<SimpleCodeStructure, PrimitiveOperationError> {
        let lines: Vec<&str> = content.lines().take(self.config.max_lines_for_structure as usize).collect();
        
        let mut functions = Vec::new();
        let mut classes = Vec::new(); 
        let mut imports = Vec::new();
        
        // Use very basic regex patterns based on language
        // This is primitive detection only - sophisticated parsing requires methodologies
        match language {
            ProgrammingLanguage::Rust => {
                self.extract_rust_basic_structure(&lines, &mut functions, &mut classes, &mut imports);
            },
            ProgrammingLanguage::Python => {
                self.extract_python_basic_structure(&lines, &mut functions, &mut classes, &mut imports);
            },
            ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript => {
                self.extract_js_basic_structure(&lines, &mut functions, &mut classes, &mut imports);
            },
            ProgrammingLanguage::Java => {
                self.extract_java_basic_structure(&lines, &mut functions, &mut classes, &mut imports);
            },
            _ => {
                // For other languages, use generic pattern matching
                self.extract_generic_basic_structure(&lines, &mut functions, &mut classes, &mut imports);
            }
        }
        
        Ok(SimpleCodeStructure {
            file_type: format!("{:?}", language),
            language: language.clone(),
            functions,
            classes,
            imports,
            line_count: lines.len() as u32,
            character_count: content.len() as u64,
            estimated_complexity: self.estimate_basic_complexity(&lines),
        })
    }
    
    /// Extract basic Rust structure using simple patterns
    fn extract_rust_basic_structure(&self, lines: &[&str], functions: &mut Vec<String>, classes: &mut Vec<String>, imports: &mut Vec<String>) {
        let fn_regex = Regex::new(r"^\s*fn\s+(\w+)").unwrap();
        let struct_regex = Regex::new(r"^\s*struct\s+(\w+)").unwrap();
        let enum_regex = Regex::new(r"^\s*enum\s+(\w+)").unwrap();
        let use_regex = Regex::new(r"^\s*use\s+([^;]+);").unwrap();
        
        for line in lines {
            if let Some(caps) = fn_regex.captures(line) {
                functions.push(caps[1].to_string());
            }
            if let Some(caps) = struct_regex.captures(line) {
                classes.push(format!("struct {}", &caps[1]));
            }
            if let Some(caps) = enum_regex.captures(line) {
                classes.push(format!("enum {}", &caps[1]));
            }
            if let Some(caps) = use_regex.captures(line) {
                imports.push(caps[1].trim().to_string());
            }
        }
    }
    
    /// Extract basic Python structure using simple patterns
    fn extract_python_basic_structure(&self, lines: &[&str], functions: &mut Vec<String>, classes: &mut Vec<String>, imports: &mut Vec<String>) {
        let def_regex = Regex::new(r"^\s*def\s+(\w+)").unwrap();
        let class_regex = Regex::new(r"^\s*class\s+(\w+)").unwrap();
        let import_regex = Regex::new(r"^\s*(?:import|from)\s+([^\s]+)").unwrap();
        
        for line in lines {
            if let Some(caps) = def_regex.captures(line) {
                functions.push(caps[1].to_string());
            }
            if let Some(caps) = class_regex.captures(line) {
                classes.push(caps[1].to_string());
            }
            if let Some(caps) = import_regex.captures(line) {
                imports.push(caps[1].to_string());
            }
        }
    }
    
    /// Extract basic JavaScript/TypeScript structure using simple patterns
    fn extract_js_basic_structure(&self, lines: &[&str], functions: &mut Vec<String>, classes: &mut Vec<String>, imports: &mut Vec<String>) {
        let function_regex = Regex::new(r"^\s*(?:function\s+(\w+)|(?:const|let|var)\s+(\w+)\s*=\s*(?:function|\(.*\)\s*=>))").unwrap();
        let class_regex = Regex::new(r"^\s*class\s+(\w+)").unwrap();
        let import_regex = Regex::new(r"^\s*import\s+.*from\s+['\"]([^'\"]+)['\"]").unwrap();
        
        for line in lines {
            if let Some(caps) = function_regex.captures(line) {
                let name = caps.get(1).or_else(|| caps.get(2)).map(|m| m.as_str()).unwrap_or("anonymous");
                functions.push(name.to_string());
            }
            if let Some(caps) = class_regex.captures(line) {
                classes.push(caps[1].to_string());
            }
            if let Some(caps) = import_regex.captures(line) {
                imports.push(caps[1].to_string());
            }
        }
    }
    
    /// Extract basic Java structure using simple patterns
    fn extract_java_basic_structure(&self, lines: &[&str], functions: &mut Vec<String>, classes: &mut Vec<String>, imports: &mut Vec<String>) {
        let method_regex = Regex::new(r"^\s*(?:public|private|protected)?\s*(?:static)?\s*\w+\s+(\w+)\s*\(").unwrap();
        let class_regex = Regex::new(r"^\s*(?:public\s+)?class\s+(\w+)").unwrap();
        let import_regex = Regex::new(r"^\s*import\s+([^;]+);").unwrap();
        
        for line in lines {
            if let Some(caps) = method_regex.captures(line) {
                functions.push(caps[1].to_string());
            }
            if let Some(caps) = class_regex.captures(line) {
                classes.push(caps[1].to_string());
            }
            if let Some(caps) = import_regex.captures(line) {
                imports.push(caps[1].to_string());
            }
        }
    }
    
    /// Generic structure extraction for unknown languages
    fn extract_generic_basic_structure(&self, lines: &[&str], functions: &mut Vec<String>, classes: &mut Vec<String>, imports: &mut Vec<String>) {
        // Very basic pattern matching for common keywords
        for line in lines {
            let line = line.trim();
            if line.contains("function") || line.contains("def ") || line.contains("fn ") {
                // Try to extract a function name
                if let Some(name) = self.extract_identifier_after_keyword(line, &["function", "def", "fn"]) {
                    functions.push(name);
                }
            }
            if line.contains("class ") || line.contains("struct ") {
                if let Some(name) = self.extract_identifier_after_keyword(line, &["class", "struct"]) {
                    classes.push(name);
                }
            }
            if line.contains("import ") || line.contains("use ") || line.contains("#include") {
                imports.push(line.to_string());
            }
        }
    }
    
    /// Helper to extract identifier after keyword
    fn extract_identifier_after_keyword(&self, line: &str, keywords: &[&str]) -> Option<String> {
        for keyword in keywords {
            if let Some(pos) = line.find(keyword) {
                let after_keyword = &line[pos + keyword.len()..];
                if let Some(identifier) = after_keyword.split_whitespace().next() {
                    if identifier.chars().all(|c| c.is_alphanumeric() || c == '_') {
                        return Some(identifier.to_string());
                    }
                }
            }
        }
        None
    }
    
    /// Estimate basic complexity using simple heuristics
    fn estimate_basic_complexity(&self, lines: &[&str]) -> BasicComplexity {
        let line_count = lines.len();
        let mut complexity_indicators = 0;
        
        // Count basic complexity indicators
        for line in lines {
            let line = line.trim();
            // Control structures
            if line.contains("if ") || line.contains("for ") || line.contains("while ") || 
               line.contains("switch ") || line.contains("match ") {
                complexity_indicators += 1;
            }
            // Nested structures
            if line.starts_with("    ") || line.starts_with("\t\t") {
                complexity_indicators += 1;
            }
        }
        
        // Very basic heuristic classification
        match (line_count, complexity_indicators) {
            (0..=50, 0..=5) => BasicComplexity::VeryLow,
            (0..=100, 0..=10) => BasicComplexity::Low, 
            (0..=500, 0..=50) => BasicComplexity::Medium,
            (0..=1000, 0..=100) => BasicComplexity::High,
            _ => BasicComplexity::VeryHigh,
        }
    }
    
    /// Detect programming language - primitive operation
    fn detect_language(&self, file_path: &str, content: Option<&str>) -> Result<ProgrammingLanguage, PrimitiveOperationError> {
        // First try file extension
        if let Some(extension) = Path::new(file_path).extension().and_then(|e| e.to_str()) {
            let lang_from_ext = ProgrammingLanguage::from_extension(extension);
            if !matches!(lang_from_ext, ProgrammingLanguage::Other(_)) {
                return Ok(lang_from_ext);
            }
        }
        
        // If content is available, try basic content-based detection
        if let Some(content) = content {
            if let Some(lang) = self.detect_language_from_content(content) {
                return Ok(lang);
            }
        }
        
        Ok(ProgrammingLanguage::Other("unknown".to_string()))
    }
    
    /// Basic content-based language detection using simple patterns
    fn detect_language_from_content(&self, content: &str) -> Option<ProgrammingLanguage> {
        let lines: Vec<&str> = content.lines().take(20).collect(); // Only check first 20 lines
        
        for line in &lines {
            let line = line.trim();
            // Rust indicators
            if line.contains("fn main()") || line.contains("use std::") || line.contains("let mut") {
                return Some(ProgrammingLanguage::Rust);
            }
            // Python indicators
            if line.starts_with("def ") || line.starts_with("import ") || line.contains("if __name__ == '__main__'") {
                return Some(ProgrammingLanguage::Python);
            }
            // JavaScript indicators
            if line.contains("function ") || line.contains("const ") || line.contains("console.log") {
                return Some(ProgrammingLanguage::JavaScript);
            }
            // Java indicators
            if line.contains("public class") || line.contains("public static void main") {
                return Some(ProgrammingLanguage::Java);
            }
        }
        
        None
    }
    
    /// Read file content - in real implementation this would coordinate with NEXUS
    async fn read_file_content(&self, file_path: &str) -> Result<String, PrimitiveOperationError> {
        // This is a placeholder - in real implementation this would coordinate with NEXUS
        // for all file operations to maintain architectural separation
        std::fs::read_to_string(file_path)
            .map_err(|e| PrimitiveOperationError::FileOperationFailed(e.to_string()))
    }
    
    /// Check cache for previous parse result
    async fn check_cache(&self, file_path: &str) -> Option<CachedParseResult> {
        let cache = self.parse_cache.read().await;
        cache.get(file_path).cloned()
    }
    
    /// Cache parse result
    async fn cache_result(&self, file_path: &str, result: &BasicParseResult, content: &str) {
        let mut cache = self.parse_cache.write().await;
        
        // Simple hash of content for cache validation
        let file_hash = format!("{:x}", md5::compute(content));
        
        let cached_result = CachedParseResult {
            result: result.clone(),
            timestamp: SystemTime::now(),
            file_hash,
        };
        
        cache.insert(file_path.to_string(), cached_result);
        
        // Basic cache size management
        if cache.len() > 1000 {
            // Remove oldest entries
            let mut entries: Vec<_> = cache.iter().collect();
            entries.sort_by_key(|(_, v)| v.timestamp);
            if let Some((key, _)) = entries.first() {
                let key = key.clone();
                cache.remove(&key);
            }
        }
    }
}

// =============================================================================
// SYNTAX VALIDATOR - Basic syntax validation capabilities
// =============================================================================

/// Basic syntax validator for primitive syntax checking
pub struct SyntaxValidator {
    config: PrimitiveOperationsConfig,
}

/// Result of syntax validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxValidationResult {
    pub is_valid: bool,
    pub errors: Vec<SyntaxError>,
    pub warnings: Vec<SyntaxWarning>,
    pub language: ProgrammingLanguage,
    pub validation_method: ValidationMethod,
}

/// Syntax error found during validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxError {
    pub line: u32,
    pub column: u32,
    pub message: String,
    pub error_type: SyntaxErrorType,
}

/// Types of syntax errors we can detect primitively
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyntaxErrorType {
    MismatchedBrackets,
    MismatchedParentheses,
    MismatchedQuotes,
    UnexpectedEndOfFile,
    InvalidCharacter,
    Other(String),
}

/// Syntax warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxWarning {
    pub line: u32,
    pub message: String,
    pub warning_type: SyntaxWarningType,
}

/// Types of syntax warnings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyntaxWarningType {
    LongLine,
    TrailingWhitespace,
    InconsistentIndentation,
    Other(String),
}

/// Method used for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationMethod {
    BasicPatternMatching,
    BracketMatching,
    SimpleHeuristics,
}

impl SyntaxValidator {
    pub fn new(config: PrimitiveOperationsConfig) -> Self {
        Self { config }
    }
    
    /// Validate syntax using primitive validation methods
    pub fn validate_syntax(&self, content: &str, language: &ProgrammingLanguage) -> Result<SyntaxValidationResult, PrimitiveOperationError> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        
        // Basic bracket matching - this is a primitive operation we can do
        self.check_bracket_matching(content, &mut errors);
        
        // Basic quote matching
        self.check_quote_matching(content, &mut errors);
        
        // Basic warnings
        self.check_basic_warnings(content, &mut warnings);
        
        let is_valid = errors.is_empty();
        
        Ok(SyntaxValidationResult {
            is_valid,
            errors,
            warnings,
            language: language.clone(),
            validation_method: ValidationMethod::BasicPatternMatching,
        })
    }
    
    /// Check for mismatched brackets, braces, and parentheses
    fn check_bracket_matching(&self, content: &str, errors: &mut Vec<SyntaxError>) {
        let mut stack = Vec::new();
        let mut line_num = 1u32;
        let mut col_num = 1u32;
        
        for ch in content.chars() {
            match ch {
                '(' | '[' | '{' => {
                    stack.push((ch, line_num, col_num));
                },
                ')' | ']' | '}' => {
                    if let Some((open_ch, _, _)) = stack.pop() {
                        let matches = match (open_ch, ch) {
                            ('(', ')') | ('[', ']') | ('{', '}') => true,
                            _ => false,
                        };
                        if !matches {
                            errors.push(SyntaxError {
                                line: line_num,
                                column: col_num,
                                message: format!("Mismatched bracket: expected closing for '{}', found '{}'", open_ch, ch),
                                error_type: SyntaxErrorType::MismatchedBrackets,
                            });
                        }
                    } else {
                        errors.push(SyntaxError {
                            line: line_num,
                            column: col_num,
                            message: format!("Unexpected closing bracket: '{}'", ch),
                            error_type: SyntaxErrorType::MismatchedBrackets,
                        });
                    }
                },
                '\n' => {
                    line_num += 1;
                    col_num = 1;
                    continue;
                },
                _ => {},
            }
            col_num += 1;
        }
        
        // Check for unclosed brackets
        for (open_ch, line, col) in stack {
            errors.push(SyntaxError {
                line,
                column: col,
                message: format!("Unclosed bracket: '{}'", open_ch),
                error_type: SyntaxErrorType::MismatchedBrackets,
            });
        }
    }
    
    /// Check for mismatched quotes
    fn check_quote_matching(&self, content: &str, errors: &mut Vec<SyntaxError>) {
        let mut in_single_quote = false;
        let mut in_double_quote = false;
        let mut line_num = 1u32;
        let mut col_num = 1u32;
        let mut prev_char = '\0';
        
        for ch in content.chars() {
            match ch {
                '\'' if prev_char != '\\' => {
                    if !in_double_quote {
                        in_single_quote = !in_single_quote;
                    }
                },
                '"' if prev_char != '\\' => {
                    if !in_single_quote {
                        in_double_quote = !in_double_quote;
                    }
                },
                '\n' => {
                    if in_single_quote || in_double_quote {
                        errors.push(SyntaxError {
                            line: line_num,
                            column: col_num,
                            message: "Unterminated string literal".to_string(),
                            error_type: SyntaxErrorType::MismatchedQuotes,
                        });
                        in_single_quote = false;
                        in_double_quote = false;
                    }
                    line_num += 1;
                    col_num = 1;
                    prev_char = ch;
                    continue;
                },
                _ => {},
            }
            
            prev_char = ch;
            col_num += 1;
        }
        
        // Check for unterminated quotes at end of file
        if in_single_quote || in_double_quote {
            errors.push(SyntaxError {
                line: line_num,
                column: col_num,
                message: "Unterminated string literal at end of file".to_string(),
                error_type: SyntaxErrorType::UnexpectedEndOfFile,
            });
        }
    }
    
    /// Check for basic warnings
    fn check_basic_warnings(&self, content: &str, warnings: &mut Vec<SyntaxWarning>) {
        for (line_num, line) in content.lines().enumerate() {
            let line_num = line_num as u32 + 1;
            
            // Check for long lines
            if line.len() > 120 {
                warnings.push(SyntaxWarning {
                    line: line_num,
                    message: format!("Line length {} exceeds recommended 120 characters", line.len()),
                    warning_type: SyntaxWarningType::LongLine,
                });
            }
            
            // Check for trailing whitespace
            if line.ends_with(' ') || line.ends_with('\t') {
                warnings.push(SyntaxWarning {
                    line: line_num,
                    message: "Line has trailing whitespace".to_string(),
                    warning_type: SyntaxWarningType::TrailingWhitespace,
                });
            }
        }
    }
}

// =============================================================================
// FILE STRUCTURE READER - Basic file structure reading capabilities
// =============================================================================

/// File structure reader for basic file system operations
pub struct FileStructureReader {
    config: PrimitiveOperationsConfig,
}

/// Request for file structure reading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStructureRequest {
    pub root_path: String,
    pub max_depth: u32,
    pub include_hidden: bool,
    pub file_pattern: Option<String>,
}

/// Result of file structure reading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStructureResult {
    pub root_path: String,
    pub files: Vec<BasicFileInfo>,
    pub directories: Vec<String>,
    pub total_files: u32,
    pub total_size: u64,
    pub errors: Vec<String>,
}

impl FileStructureReader {
    pub fn new(config: PrimitiveOperationsConfig) -> Self {
        Self { config }
    }
    
    /// Read file structure - primitive operation
    /// In real implementation, this would coordinate with NEXUS for all file operations
    pub async fn read_structure(&self, request: FileStructureRequest) -> Result<FileStructureResult, PrimitiveOperationError> {
        // This is a placeholder implementation
        // In the real system, this would send requests to NEXUS for all file operations
        
        let mut files = Vec::new();
        let mut directories = Vec::new();
        let mut total_size = 0u64;
        let mut errors = Vec::new();
        
        // This would be replaced with NEXUS coordination calls
        match self.scan_directory(&request.root_path, 0, request.max_depth, request.include_hidden) {
            Ok((dir_files, dir_directories, size)) => {
                files.extend(dir_files);
                directories.extend(dir_directories);
                total_size += size;
            },
            Err(e) => {
                errors.push(format!("Failed to scan directory: {}", e));
            }
        }
        
        Ok(FileStructureResult {
            root_path: request.root_path,
            total_files: files.len() as u32,
            files,
            directories,
            total_size,
            errors,
        })
    }
    
    /// Scan directory recursively - placeholder for NEXUS coordination
    fn scan_directory(&self, path: &str, current_depth: u32, max_depth: u32, include_hidden: bool) -> Result<(Vec<BasicFileInfo>, Vec<String>, u64), PrimitiveOperationError> {
        // Placeholder implementation - real version would coordinate with NEXUS
        if current_depth >= max_depth {
            return Ok((Vec::new(), Vec::new(), 0));
        }
        
        let mut files = Vec::new();
        let mut directories = Vec::new();
        let mut total_size = 0u64;
        
        // This would be replaced with NEXUS file system operations
        match std::fs::read_dir(path) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            let path = entry.path();
                            let file_name = path.file_name().unwrap().to_string_lossy();
                            
                            // Skip hidden files if requested
                            if !include_hidden && file_name.starts_with('.') {
                                continue;
                            }
                            
                            if path.is_file() {
                                match entry.metadata() {
                                    Ok(metadata) => {
                                        let size = metadata.len();
                                        total_size += size;
                                        
                                        files.push(BasicFileInfo {
                                            file_name: file_name.to_string(),
                                            file_path: path.to_string_lossy().to_string(),
                                            file_size: size,
                                            last_modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                                            file_type: path.extension().unwrap_or_default().to_string_lossy().to_string(),
                                            language: detect_language_from_path(&path),
                                            encoding: None, // Would be detected by more sophisticated analysis
                                        });
                                    },
                                    Err(_) => {
                                        // Skip files we can't read metadata for
                                    }
                                }
                            } else if path.is_dir() {
                                directories.push(path.to_string_lossy().to_string());
                                
                                // Recursively scan subdirectory
                                match self.scan_directory(&path.to_string_lossy(), current_depth + 1, max_depth, include_hidden) {
                                    Ok((sub_files, sub_dirs, sub_size)) => {
                                        files.extend(sub_files);
                                        directories.extend(sub_dirs);
                                        total_size += sub_size;
                                    },
                                    Err(_) => {
                                        // Skip directories we can't read
                                    }
                                }
                            }
                        },
                        Err(_) => {
                            // Skip entries we can't read
                        }
                    }
                }
            },
            Err(e) => {
                return Err(PrimitiveOperationError::FileOperationFailed(e.to_string()));
            }
        }
        
        Ok((files, directories, total_size))
    }
}

// =============================================================================
// LANGUAGE DETECTOR - Programming language detection capabilities
// =============================================================================

/// Language detector for primitive language detection
pub struct LanguageDetector {
    config: PrimitiveOperationsConfig,
}

/// Language detection request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageDetectionRequest {
    pub file_path: Option<String>,
    pub content: Option<String>,
    pub detection_methods: Vec<DetectionMethod>,
}

/// Language detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageDetectionResult {
    pub detected_language: ProgrammingLanguage,
    pub confidence: f32, // 0.0 to 1.0
    pub detection_method: DetectionMethod,
    pub alternative_candidates: Vec<LanguageCandidate>,
}

/// Alternative language candidate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageCandidate {
    pub language: ProgrammingLanguage,
    pub confidence: f32,
    pub detection_method: DetectionMethod,
}

impl LanguageDetector {
    pub fn new(config: PrimitiveOperationsConfig) -> Self {
        Self { config }
    }
    
    /// Detect programming language using primitive methods
    pub fn detect_language(&self, request: LanguageDetectionRequest) -> Result<LanguageDetectionResult, PrimitiveOperationError> {
        let mut candidates = Vec::new();
        
        // Try each requested detection method
        for method in &request.detection_methods {
            match method {
                DetectionMethod::FileExtension => {
                    if let Some(ref file_path) = request.file_path {
                        if let Some(candidate) = self.detect_by_extension(file_path) {
                            candidates.push(candidate);
                        }
                    }
                },
                DetectionMethod::ContentAnalysis => {
                    if let Some(ref content) = request.content {
                        if let Some(candidate) = self.detect_by_content(content) {
                            candidates.push(candidate);
                        }
                    }
                },
                DetectionMethod::Heuristic => {
                    if let (Some(ref file_path), Some(ref content)) = (&request.file_path, &request.content) {
                        if let Some(candidate) = self.detect_by_heuristics(file_path, content) {
                            candidates.push(candidate);
                        }
                    }
                },
                DetectionMethod::UserProvided => {
                    // User provided detection would come from outside this function
                }
            }
        }
        
        // Find best candidate
        let best_candidate = candidates.into_iter()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(LanguageCandidate {
                language: ProgrammingLanguage::Other("unknown".to_string()),
                confidence: 0.0,
                detection_method: DetectionMethod::Heuristic,
            });
        
        Ok(LanguageDetectionResult {
            detected_language: best_candidate.language.clone(),
            confidence: best_candidate.confidence,
            detection_method: best_candidate.detection_method.clone(),
            alternative_candidates: Vec::new(), // Could be populated with other candidates
        })
    }
    
    /// Detect language by file extension
    fn detect_by_extension(&self, file_path: &str) -> Option<LanguageCandidate> {
        if let Some(extension) = Path::new(file_path).extension().and_then(|e| e.to_str()) {
            let language = ProgrammingLanguage::from_extension(extension);
            let confidence = match language {
                ProgrammingLanguage::Other(_) => 0.1,
                _ => 0.9, // High confidence for known extensions
            };
            
            Some(LanguageCandidate {
                language,
                confidence,
                detection_method: DetectionMethod::FileExtension,
            })
        } else {
            None
        }
    }
    
    /// Detect language by content analysis
    fn detect_by_content(&self, content: &str) -> Option<LanguageCandidate> {
        let lines: Vec<&str> = content.lines().take(50).collect(); // Check first 50 lines
        let mut scores = HashMap::new();
        
        // Score different languages based on content patterns
        for line in &lines {
            let line = line.trim();
            
            // Rust patterns
            if line.contains("fn ") || line.contains("use std::") || line.contains("let mut") || line.contains("impl ") {
                *scores.entry(ProgrammingLanguage::Rust).or_insert(0.0) += 1.0;
            }
            
            // Python patterns
            if line.starts_with("def ") || line.starts_with("import ") || line.contains("if __name__") || line.contains("self.") {
                *scores.entry(ProgrammingLanguage::Python).or_insert(0.0) += 1.0;
            }
            
            // JavaScript patterns
            if line.contains("function ") || line.contains("const ") || line.contains("console.log") || line.contains("=>") {
                *scores.entry(ProgrammingLanguage::JavaScript).or_insert(0.0) += 1.0;
            }
            
            // Java patterns
            if line.contains("public class") || line.contains("public static") || line.contains("System.out") {
                *scores.entry(ProgrammingLanguage::Java).or_insert(0.0) += 1.0;
            }
            
            // Add more patterns as needed...
        }
        
        // Find language with highest score
        if let Some((language, score)) = scores.into_iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)) {
            let confidence = (score / lines.len() as f32).min(1.0);
            if confidence > 0.1 {
                return Some(LanguageCandidate {
                    language,
                    confidence,
                    detection_method: DetectionMethod::ContentAnalysis,
                });
            }
        }
        
        None
    }
    
    /// Detect language using heuristics combining file path and content
    fn detect_by_heuristics(&self, file_path: &str, content: &str) -> Option<LanguageCandidate> {
        // Combine extension and content detection
        let extension_result = self.detect_by_extension(file_path);
        let content_result = self.detect_by_content(content);
        
        match (extension_result, content_result) {
            (Some(ext), Some(cont)) if ext.language == cont.language => {
                // Both agree - high confidence
                Some(LanguageCandidate {
                    language: ext.language,
                    confidence: (ext.confidence + cont.confidence) / 2.0,
                    detection_method: DetectionMethod::Heuristic,
                })
            },
            (Some(ext), Some(_cont)) => {
                // Disagree - lower confidence, prefer extension
                Some(LanguageCandidate {
                    language: ext.language,
                    confidence: ext.confidence * 0.7,
                    detection_method: DetectionMethod::Heuristic,
                })
            },
            (Some(ext), None) => {
                // Only extension detection worked
                Some(LanguageCandidate {
                    language: ext.language,
                    confidence: ext.confidence * 0.8,
                    detection_method: DetectionMethod::Heuristic,
                })
            },
            (None, Some(cont)) => {
                // Only content detection worked
                Some(LanguageCandidate {
                    language: cont.language,
                    confidence: cont.confidence * 0.6,
                    detection_method: DetectionMethod::Heuristic,
                })
            },
            (None, None) => None,
        }
    }
}

// =============================================================================
// SIMPLE STATUS REPORTER - Basic status reporting for coordination
// =============================================================================

/// Simple status reporter for basic coordination messages
pub struct SimpleStatusReporter {
    component_id: String,
}

/// Status message types that can be reported
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatusMessage {
    /// Component is initializing
    Initializing { progress: f32 },
    /// Component is ready for operations
    Ready,
    /// Component is processing a request
    Processing { operation_id: String, progress: f32 },
    /// Operation completed successfully
    OperationComplete { operation_id: String, duration: Duration },
    /// Operation failed
    OperationFailed { operation_id: String, error: String },
    /// Component encountered an error
    Error { error_type: String, message: String },
    /// Component is shutting down
    ShuttingDown,
}

/// Status report containing the message and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusReport {
    pub component_id: String,
    pub timestamp: SystemTime,
    pub message: StatusMessage,
    pub details: Option<String>,
}

impl SimpleStatusReporter {
    pub fn new(component_id: String) -> Self {
        Self { component_id }
    }
    
    /// Report status - primitive operation for ecosystem coordination
    pub fn report_status(&self, message: StatusMessage, details: Option<String>) -> StatusReport {
        StatusReport {
            component_id: self.component_id.clone(),
            timestamp: SystemTime::now(),
            message,
            details,
        }
    }
    
    /// Create a simple text status message - primitive operation
    pub fn create_simple_message(&self, message: &str) -> String {
        format!("[{}] {}", self.component_id, message)
    }
    
    /// Create an error message - primitive operation
    pub fn create_error_message(&self, error: &str) -> String {
        format!("[{}] ERROR: {}", self.component_id, error)
    }
    
    /// Create a progress message - primitive operation
    pub fn create_progress_message(&self, operation: &str, progress: f32) -> String {
        format!("[{}] {}: {:.1}%", self.component_id, operation, progress * 100.0)
    }
}

// =============================================================================
// BASIC CODE FORMATTER - Simple code formatting capabilities
// =============================================================================

/// Basic code formatter for primitive formatting operations
pub struct BasicCodeFormatter {
    config: PrimitiveOperationsConfig,
}

/// Code formatting request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatRequest {
    pub content: String,
    pub language: ProgrammingLanguage,
    pub format_options: FormatOptions,
}

/// Basic formatting options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatOptions {
    pub indent_size: u32,
    pub use_tabs: bool,
    pub max_line_length: u32,
    pub remove_trailing_whitespace: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            indent_size: 4,
            use_tabs: false,
            max_line_length: 120,
            remove_trailing_whitespace: true,
        }
    }
}

/// Code formatting result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatResult {
    pub formatted_content: String,
    pub changes_made: Vec<String>,
    pub warnings: Vec<String>,
}

impl BasicCodeFormatter {
    pub fn new(config: PrimitiveOperationsConfig) -> Self {
        Self { config }
    }
    
    /// Format code using basic formatting rules - primitive operation
    pub fn format_code(&self, request: FormatRequest) -> Result<FormatResult, PrimitiveOperationError> {
        let mut formatted_lines = Vec::new();
        let mut changes_made = Vec::new();
        let mut warnings = Vec::new();
        
        let lines: Vec<&str> = request.content.lines().collect();
        let mut current_indent = 0i32;
        
        for (line_num, line) in lines.iter().enumerate() {
            let line_num = line_num + 1;
            let trimmed = line.trim();
            
            // Skip empty lines
            if trimmed.is_empty() {
                formatted_lines.push(String::new());
                continue;
            }
            
            // Check if this line should decrease indent (closing braces, etc.)
            if self.should_decrease_indent(trimmed, &request.language) {
                current_indent = (current_indent - 1).max(0);
            }
            
            // Create indented line
            let indent_str = if request.format_options.use_tabs {
                "\t".repeat(current_indent as usize)
            } else {
                " ".repeat((current_indent as usize) * request.format_options.indent_size as usize)
            };
            
            let formatted_line = format!("{}{}", indent_str, trimmed);
            
            // Check for long lines
            if formatted_line.len() > request.format_options.max_line_length as usize {
                warnings.push(format!("Line {}: exceeds maximum length", line_num));
            }
            
            // Check if original line was different
            if line != &formatted_line {
                changes_made.push(format!("Line {}: reformatted indentation", line_num));
            }
            
            formatted_lines.push(formatted_line);
            
            // Check if this line should increase indent (opening braces, etc.)
            if self.should_increase_indent(trimmed, &request.language) {
                current_indent += 1;
            }
        }
        
        Ok(FormatResult {
            formatted_content: formatted_lines.join("\n"),
            changes_made,
            warnings,
        })
    }
    
    /// Check if line should decrease indentation
    fn should_decrease_indent(&self, line: &str, language: &ProgrammingLanguage) -> bool {
        match language {
            ProgrammingLanguage::Rust | ProgrammingLanguage::JavaScript | 
            ProgrammingLanguage::TypeScript | ProgrammingLanguage::Java |
            ProgrammingLanguage::CPlusPlus | ProgrammingLanguage::C => {
                line.starts_with('}')
            },
            ProgrammingLanguage::Python => {
                // Python indentation is more complex, this is very basic
                false
            },
            _ => {
                line.starts_with('}') || line.starts_with(')')
            }
        }
    }
    
    /// Check if line should increase indentation
    fn should_increase_indent(&self, line: &str, language: &ProgrammingLanguage) -> bool {
        match language {
            ProgrammingLanguage::Rust | ProgrammingLanguage::JavaScript | 
            ProgrammingLanguage::TypeScript | ProgrammingLanguage::Java |
            ProgrammingLanguage::CPlusPlus | ProgrammingLanguage::C => {
                line.ends_with('{')
            },
            ProgrammingLanguage::Python => {
                line.ends_with(':')
            },
            _ => {
                line.ends_with('{') || line.ends_with('(')
            }
        }
    }
}

// =============================================================================
// COORDINATION MESSENGER - Simple ecosystem messaging
// =============================================================================

/// Coordination messenger for basic ecosystem communication
pub struct CoordinationMessenger {
    component_id: String,
    message_counter: Arc<Mutex<u64>>,
}

/// Simple coordination message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationMessage {
    pub message_id: String,
    pub sender_id: String,
    pub recipient_id: String,
    pub message_type: CoordinationMessageType,
    pub content: String,
    pub timestamp: SystemTime,
    pub priority: MessagePriority,
}

/// Types of coordination messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinationMessageType {
    StatusUpdate,
    OperationRequest,
    OperationResponse,
    ErrorNotification,
    HealthCheck,
    Simple,
}

/// Message priority levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessagePriority {
    Low,
    Normal,
    High,
    Critical,
}

impl CoordinationMessenger {
    pub fn new(component_id: String) -> Self {
        Self {
            component_id,
            message_counter: Arc::new(Mutex::new(0)),
        }
    }
    
    /// Send a simple coordination message - primitive operation
    pub async fn send_simple_message(&self, recipient_id: String, content: String) -> CoordinationMessage {
        let message_id = self.generate_message_id().await;
        
        CoordinationMessage {
            message_id,
            sender_id: self.component_id.clone(),
            recipient_id,
            message_type: CoordinationMessageType::Simple,
            content,
            timestamp: SystemTime::now(),
            priority: MessagePriority::Normal,
        }
    }
    
    /// Send a status update message - primitive operation
    pub async fn send_status_update(&self, recipient_id: String, status: String) -> CoordinationMessage {
        let message_id = self.generate_message_id().await;
        
        CoordinationMessage {
            message_id,
            sender_id: self.component_id.clone(),
            recipient_id,
            message_type: CoordinationMessageType::StatusUpdate,
            content: status,
            timestamp: SystemTime::now(),
            priority: MessagePriority::Normal,
        }
    }
    
    /// Send an error notification - primitive operation
    pub async fn send_error_notification(&self, recipient_id: String, error: String) -> CoordinationMessage {
        let message_id = self.generate_message_id().await;
        
        CoordinationMessage {
            message_id,
            sender_id: self.component_id.clone(),
            recipient_id,
            message_type: CoordinationMessageType::ErrorNotification,
            content: error,
            timestamp: SystemTime::now(),
            priority: MessagePriority::High,
        }
    }
    
    /// Generate unique message ID
    async fn generate_message_id(&self) -> String {
        let mut counter = self.message_counter.lock().await;
        *counter += 1;
        format!("{}-{}-{}", self.component_id, SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(), *counter)
    }
}

// =============================================================================
// COORDINATION HANDLER - Handle ecosystem coordination requests
// =============================================================================

/// Coordination handler for processing ecosystem coordination requests
pub struct CoordinationHandler {
    component_id: String,
    capabilities: Vec<PrimitiveCapability>,
}

/// Response to coordination requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationHandlerResponse {
    pub request_id: String,
    pub status: ExecutionStatus,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub processing_time: Duration,
}

impl CoordinationHandler {
    pub fn new(component_id: String, capabilities: Vec<PrimitiveCapability>) -> Self {
        Self {
            component_id,
            capabilities,
        }
    }
    
    /// Handle coordination request from OZONE STUDIO - primitive operation
    pub async fn handle_coordination_request(&self, request: AIAppCoordinationRequest) -> CoordinationHandlerResponse {
        let start_time = Instant::now();
        
        // Check if we have the required capability
        if !self.has_required_capability(&request) {
            return CoordinationHandlerResponse {
                request_id: request.request_id.clone(),
                status: ExecutionStatus::Failed,
                result: None,
                error: Some("Required capability not available".to_string()),
                processing_time: start_time.elapsed(),
            };
        }
        
        // Process the request based on its type
        let result = self.process_coordination_request(request).await;
        
        CoordinationHandlerResponse {
            request_id: result.0,
            status: result.1,
            result: result.2,
            error: result.3,
            processing_time: start_time.elapsed(),
        }
    }
    
    /// Check if we have the required capability for this request
    fn has_required_capability(&self, request: &AIAppCoordinationRequest) -> bool {
        // This would check the request type against our capabilities
        // For now, assume we can handle basic coordination requests
        true
    }
    
    /// Process the coordination request
    async fn process_coordination_request(&self, request: AIAppCoordinationRequest) -> (String, ExecutionStatus, Option<serde_json::Value>, Option<String>) {
        // This would process different types of coordination requests
        // For now, return a simple success response
        (
            request.request_id,
            ExecutionStatus::Completed,
            Some(serde_json::json!({"message": "Request processed"})),
            None,
        )
    }
    
    /// Get current coordination state
    pub fn get_coordination_state(&self) -> CoordinationState {
        CoordinationState {
            component_id: self.component_id.clone(),
            status: crate::ComponentStatus::Ready,
            capabilities: self.capabilities.clone(),
            active_methodologies: Vec::new(), // Would track loaded methodologies
            last_heartbeat: SystemTime::now(),
            memory_usage: 0, // Would track actual memory usage
            cpu_usage: 0.0,  // Would track actual CPU usage
        }
    }
}

// =============================================================================
// METHODOLOGY RECEIVER - Receive and load methodologies from ZSEI
// =============================================================================

/// Methodology receiver for loading methodologies from ZSEI
pub struct MethodologyReceiver {
    component_id: String,
    loaded_methodologies: Arc<RwLock<HashMap<String, methodology_runtime::Methodology>>>,
}

/// Response to methodology loading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodologyLoadResponse {
    pub methodology_id: String,
    pub load_status: MethodologyLoadStatus,
    pub error: Option<String>,
    pub load_time: Duration,
}

/// Status of methodology loading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MethodologyLoadStatus {
    Success,
    Failed,
    AlreadyLoaded,
    ValidationFailed,
}

impl MethodologyReceiver {
    pub fn new(component_id: String) -> Self {
        Self {
            component_id,
            loaded_methodologies: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Load methodology from ZSEI - primitive operation
    pub async fn load_methodology(&self, methodology: methodology_runtime::Methodology) -> MethodologyLoadResponse {
        let start_time = Instant::now();
        let methodology_id = methodology.metadata.id.clone();
        
        // Check if already loaded
        {
            let loaded = self.loaded_methodologies.read().await;
            if loaded.contains_key(&methodology_id) {
                return MethodologyLoadResponse {
                    methodology_id,
                    load_status: MethodologyLoadStatus::AlreadyLoaded,
                    error: None,
                    load_time: start_time.elapsed(),
                };
            }
        }
        
        // Validate methodology
        if let Err(e) = self.validate_methodology(&methodology) {
            return MethodologyLoadResponse {
                methodology_id,
                load_status: MethodologyLoadStatus::ValidationFailed,
                error: Some(e.to_string()),
                load_time: start_time.elapsed(),
            };
        }
        
        // Load methodology
        {
            let mut loaded = self.loaded_methodologies.write().await;
            loaded.insert(methodology_id.clone(), methodology);
        }
        
        MethodologyLoadResponse {
            methodology_id,
            load_status: MethodologyLoadStatus::Success,
            error: None,
            load_time: start_time.elapsed(),
        }
    }
    
    /// Get loaded methodology
    pub async fn get_methodology(&self, methodology_id: &str) -> Option<methodology_runtime::Methodology> {
        let loaded = self.loaded_methodologies.read().await;
        loaded.get(methodology_id).cloned()
    }
    
    /// List loaded methodologies
    pub async fn list_loaded_methodologies(&self) -> Vec<String> {
        let loaded = self.loaded_methodologies.read().await;
        loaded.keys().cloned().collect()
    }
    
    /// Validate methodology before loading
    fn validate_methodology(&self, methodology: &methodology_runtime::Methodology) -> Result<(), PrimitiveOperationError> {
        // Basic validation - ensure required fields are present
        if methodology.metadata.id.is_empty() {
            return Err(PrimitiveOperationError::InvalidInput("Methodology ID cannot be empty".to_string()));
        }
        
        if methodology.metadata.name.is_empty() {
            return Err(PrimitiveOperationError::InvalidInput("Methodology name cannot be empty".to_string()));
        }
        
        // Additional validation would go here
        Ok(())
    }
}

// =============================================================================
// INSTRUCTION PROCESSOR - Process methodology instructions
// =============================================================================

/// Instruction processor for executing methodology instructions
pub struct InstructionProcessor {
    component_id: String,
    execution_context: Arc<RwLock<ExecutionContext>>,
}

/// Response to instruction execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionExecutionResponse {
    pub instruction_id: String,
    pub execution_status: ExecutionStatus,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub execution_time: Duration,
}

impl InstructionProcessor {
    pub fn new(component_id: String) -> Self {
        Self {
            component_id,
            execution_context: Arc::new(RwLock::new(ExecutionContext::default())),
        }
    }
    
    /// Execute methodology instruction - primitive operation
    pub async fn execute_instruction(&self, instruction: Instruction) -> InstructionExecutionResponse {
        let start_time = Instant::now();
        let instruction_id = format!("{}_{}", self.component_id, SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs());
        
        // Execute the instruction based on its type
        let result = self.process_instruction(instruction).await;
        
        InstructionExecutionResponse {
            instruction_id,
            execution_status: if result.is_ok() { ExecutionStatus::Completed } else { ExecutionStatus::Failed },
            result: result.as_ref().ok().cloned(),
            error: result.as_ref().err().map(|e| e.to_string()),
            execution_time: start_time.elapsed(),
        }
    }
    
    /// Process individual instruction
    async fn process_instruction(&self, instruction: Instruction) -> Result<serde_json::Value, PrimitiveOperationError> {
        // This would process different types of instructions
        // For now, return a simple success response
        match instruction {
            // Different instruction types would be handled here
            _ => Ok(serde_json::json!({"status": "processed"})),
        }
    }
    
    /// Update execution context
    pub async fn update_execution_context(&self, context: ExecutionContext) {
        let mut current_context = self.execution_context.write().await;
        *current_context = context;
    }
    
    /// Get current execution context
    pub async fn get_execution_context(&self) -> ExecutionContext {
        let context = self.execution_context.read().await;
        context.clone()
    }
}

// =============================================================================
// HELPER FUNCTIONS FOR PRIMITIVE OPERATIONS
// =============================================================================

/// Helper function to detect language from file path - used by other primitives
pub fn detect_language_from_path(file_path: &Path) -> Option<ProgrammingLanguage> {
    file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(ProgrammingLanguage::from_extension)
        .filter(|lang| !matches!(lang, ProgrammingLanguage::Other(_)))
}

/// Helper function to create basic file info
pub fn create_basic_file_info(file_path: &str, metadata: &std::fs::Metadata) -> BasicFileInfo {
    let path = Path::new(file_path);
    BasicFileInfo {
        file_name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
        file_path: file_path.to_string(),
        file_size: metadata.len(),
        last_modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        file_type: path.extension().unwrap_or_default().to_string_lossy().to_string(),
        language: detect_language_from_path(path),
        encoding: None,
    }
}

// =============================================================================
// EXPLICIT TYPE EXPORTS - No wildcards allowed
// =============================================================================

// Basic Code Parser exports
pub use BasicCodeParser;
pub use BasicParseRequest;
pub use ParseDepth;

// Syntax Validator exports  
pub use SyntaxValidator;
pub use SyntaxValidationResult;
pub use SyntaxError;
pub use SyntaxErrorType;
pub use SyntaxWarning;
pub use SyntaxWarningType;
pub use ValidationMethod;

// File Structure Reader exports
pub use FileStructureReader;
pub use FileStructureRequest;
pub use FileStructureResult;

// Language Detector exports
pub use LanguageDetector;
pub use LanguageDetectionRequest;
pub use LanguageDetectionResult;
pub use LanguageCandidate;

// Status Reporter exports
pub use SimpleStatusReporter;
pub use StatusMessage;
pub use StatusReport;

// Code Formatter exports
pub use BasicCodeFormatter;
pub use FormatRequest;
pub use FormatOptions;
pub use FormatResult;

// Coordination Messenger exports
pub use CoordinationMessenger;
pub use CoordinationMessage;
pub use CoordinationMessageType;
pub use MessagePriority;

// Coordination Handler exports
pub use CoordinationHandler;
pub use CoordinationHandlerResponse;

// Methodology Receiver exports
pub use MethodologyReceiver;
pub use MethodologyLoadResponse;
pub use MethodologyLoadStatus;

// Instruction Processor exports
pub use InstructionProcessor;
pub use InstructionExecutionResponse;

// Error and Config exports
pub use PrimitiveOperationError;
pub use PrimitiveOperationsConfig;
