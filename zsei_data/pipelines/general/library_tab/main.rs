//! LibraryTabPipeline - Pipeline #7
//! 
//! Browse and search the library of:
//! - Methodologies (reusable approaches)
//! - Blueprints (task templates)
//! - Pipelines (executable units)
//! - Categories (organizational structure)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum LibraryInput {
    /// Browse categories
    GetCategories { modality: Option<String>, parent_id: Option<u64> },
    /// Search methodologies
    SearchMethodologies { keywords: Vec<String>, category_id: Option<u64>, limit: Option<u32> },
    /// Get methodology details
    GetMethodology { methodology_id: u64 },
    /// Search blueprints
    SearchBlueprints { keywords: Vec<String>, input_types: Option<Vec<String>>, limit: Option<u32> },
    /// Get blueprint details
    GetBlueprint { blueprint_id: u64 },
    /// List available pipelines
    ListPipelines { builtin_only: bool },
    /// Get pipeline details
    GetPipeline { pipeline_id: u64 },
    /// Get recently used items
    GetRecent { item_type: String, limit: Option<u32> },
    /// Get favorites
    GetFavorites { item_type: Option<String> },
    /// Add to favorites
    AddFavorite { item_type: String, item_id: u64 },
    /// Remove from favorites
    RemoveFavorite { item_type: String, item_id: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub category_id: u64,
    pub name: String,
    pub description: Option<String>,
    pub modality: String,
    pub parent_id: Option<u64>,
    pub child_count: u32,
    pub methodology_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodologySummary {
    pub methodology_id: u64,
    pub name: String,
    pub description: String,
    pub category_id: u64,
    pub keywords: Vec<String>,
    pub use_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintSummary {
    pub blueprint_id: u64,
    pub name: String,
    pub description: String,
    pub input_types: Vec<String>,
    pub output_type: String,
    pub step_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSummary {
    pub pipeline_id: u64,
    pub name: String,
    pub description: String,
    pub is_builtin: bool,
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryOutput {
    pub success: bool,
    pub categories: Option<Vec<Category>>,
    pub methodologies: Option<Vec<MethodologySummary>>,
    pub methodology: Option<MethodologySummary>,
    pub blueprints: Option<Vec<BlueprintSummary>>,
    pub blueprint: Option<BlueprintSummary>,
    pub pipelines: Option<Vec<PipelineSummary>>,
    pub pipeline: Option<PipelineSummary>,
    pub error: Option<String>,
}

pub async fn execute(input: LibraryInput) -> Result<LibraryOutput, String> {
    match input {
        LibraryInput::GetCategories { modality, parent_id } => {
            let categories = vec![
                Category { category_id: 1, name: "Programming".into(), description: Some("Software development".into()), modality: "Code".into(), parent_id: None, child_count: 5, methodology_count: 20 },
                Category { category_id: 2, name: "Writing".into(), description: Some("Text composition".into()), modality: "Text".into(), parent_id: None, child_count: 3, methodology_count: 15 },
                Category { category_id: 3, name: "Analysis".into(), description: Some("Data analysis".into()), modality: "Data".into(), parent_id: None, child_count: 4, methodology_count: 12 },
            ];
            Ok(LibraryOutput { success: true, categories: Some(categories), methodologies: None, methodology: None, blueprints: None, blueprint: None, pipelines: None, pipeline: None, error: None })
        }
        LibraryInput::SearchMethodologies { keywords, category_id, limit } => {
            let methodologies = vec![
                MethodologySummary { methodology_id: 1, name: "Test-Driven Development".into(), description: "Write tests before code".into(), category_id: 1, keywords: vec!["testing".into(), "tdd".into()], use_count: 150 },
                MethodologySummary { methodology_id: 2, name: "Code Review Best Practices".into(), description: "Effective code review".into(), category_id: 1, keywords: vec!["review".into(), "quality".into()], use_count: 120 },
            ];
            Ok(LibraryOutput { success: true, categories: None, methodologies: Some(methodologies), methodology: None, blueprints: None, blueprint: None, pipelines: None, pipeline: None, error: None })
        }
        LibraryInput::GetMethodology { methodology_id } => {
            let m = MethodologySummary { methodology_id, name: "Methodology".into(), description: "Description".into(), category_id: 1, keywords: vec![], use_count: 100 };
            Ok(LibraryOutput { success: true, categories: None, methodologies: None, methodology: Some(m), blueprints: None, blueprint: None, pipelines: None, pipeline: None, error: None })
        }
        LibraryInput::SearchBlueprints { keywords, input_types, limit } => {
            let blueprints = vec![
                BlueprintSummary { blueprint_id: 1, name: "Code Review".into(), description: "Review code for issues".into(), input_types: vec!["Code".into()], output_type: "Review".into(), step_count: 5 },
            ];
            Ok(LibraryOutput { success: true, categories: None, methodologies: None, methodology: None, blueprints: Some(blueprints), blueprint: None, pipelines: None, pipeline: None, error: None })
        }
        LibraryInput::ListPipelines { builtin_only } => {
            let pipelines = vec![
                PipelineSummary { pipeline_id: 1, name: "AuthPipeline".into(), description: "Authentication".into(), is_builtin: true, author: "Ozone".into() },
                PipelineSummary { pipeline_id: 9, name: "PromptPipeline".into(), description: "LLM interaction".into(), is_builtin: true, author: "Ozone".into() },
            ];
            Ok(LibraryOutput { success: true, categories: None, methodologies: None, methodology: None, blueprints: None, blueprint: None, pipelines: Some(pipelines), pipeline: None, error: None })
        }
        _ => Ok(LibraryOutput { success: true, categories: None, methodologies: None, methodology: None, blueprints: None, blueprint: None, pipelines: None, pipeline: None, error: None })
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() { if args[i] == "--input" && i + 1 < args.len() { input_json = args[i + 1].clone(); } }
    let input: LibraryInput = serde_json::from_str(&input_json).unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { println!("{}", serde_json::json!({"success": false, "error": e})); std::process::exit(1); }
    }
}
