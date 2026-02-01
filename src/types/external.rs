//! External reference types - Sections 23-24 of the specification

use serde::{Deserialize, Serialize};

/// Browser navigation configuration (§24.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserNavigationConfig {
    pub browser_engine: BrowserEngine,
    pub navigation_config: NavigationConfig,
    pub extraction_config: ExtractionConfig,
}

/// Browser engines
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrowserEngine {
    Playwright,
    Puppeteer,
    Custom(String),
}

/// Navigation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationConfig {
    pub page_load_timeout_ms: u64,
    pub navigation_timeout_ms: u64,
    pub min_delay_between_requests_ms: u64,
    pub max_concurrent_pages: u32,
    pub allowed_domains: Vec<String>,
    pub blocked_domains: Vec<String>,
    pub user_agent: String,
}

impl Default for NavigationConfig {
    fn default() -> Self {
        Self {
            page_load_timeout_ms: 30000,
            navigation_timeout_ms: 30000,
            min_delay_between_requests_ms: 1000,
            max_concurrent_pages: 3,
            allowed_domains: vec![
                "docs.rs".into(),
                "crates.io".into(),
                "npmjs.com".into(),
                "pypi.org".into(),
                "github.com".into(),
            ],
            blocked_domains: Vec::new(),
            user_agent: "OzoneStudio/0.3".into(),
        }
    }
}

/// Extraction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionConfig {
    pub extract_text_content: bool,
    pub extract_code_blocks: bool,
    pub extract_api_signatures: bool,
    pub extract_links: bool,
    pub max_content_length: usize,
    pub max_links_per_page: usize,
    pub generate_semantic_summary: bool,
    pub generate_embedding: bool,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            extract_text_content: true,
            extract_code_blocks: true,
            extract_api_signatures: true,
            extract_links: true,
            max_content_length: 100000,
            max_links_per_page: 50,
            generate_semantic_summary: true,
            generate_embedding: true,
        }
    }
}

/// Package usage analysis (§26.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageUsageAnalysis {
    pub package_ref_id: u64,
    pub code_file_id: u64,
    pub imported_items: Vec<ImportedItem>,
    pub usage_patterns: Vec<UsagePattern>,
    pub depends_on_features: Vec<String>,
    pub version_constraints: Vec<VersionConstraint>,
}

/// Imported item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedItem {
    pub name: String,
    pub item_type: super::container::APIType,
    pub alias: Option<String>,
    pub usage_count: u32,
}

/// Usage pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsagePattern {
    pub pattern_type: UsagePatternType,
    pub locations: Vec<CodeLocation>,
    pub description: String,
}

/// Usage pattern types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UsagePatternType {
    DirectCall,
    TypeUsage,
    TraitImplementation,
    Inheritance,
    Composition,
    Configuration,
}

/// Code location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeLocation {
    pub file_id: u64,
    pub line_start: u32,
    pub line_end: u32,
    pub function_context: Option<String>,
}

/// Version constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionConstraint {
    pub constraint: String,
    pub reason: Option<String>,
    pub breaking_if_changed: bool,
}
