//! MethodologyFetchPipeline - Pipeline #11
//! Retrieve methodologies from ZSEI based on task context, keywords, or categories.
//! Per spec §13: Methodologies are reusable approaches bound to categories.
//! 
//! NOTE: Uses DIRECT ZSEI storage access for methodology retrieval.
//! Methodologies are stored in /zsei/local/methodologies/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::path::Path;
use std::env;

// Direct methodology storage - mirrors ZSEI container structure
lazy_static::lazy_static! {
    static ref METHODOLOGY_STORE: Mutex<MethodologyStore> = Mutex::new(MethodologyStore::new());
}

struct MethodologyStore {
    methodologies: HashMap<u64, Methodology>,
    by_category: HashMap<u64, Vec<u64>>,
    by_keyword: HashMap<String, Vec<u64>>,
    storage_path: String,
}

impl MethodologyStore {
    fn new() -> Self {
        let storage_path = env::var("OZONE_ZSEI_PATH")
            .unwrap_or_else(|_| "./zsei_data".to_string());
        
        let mut store = Self {
            methodologies: HashMap::new(),
            by_category: HashMap::new(),
            by_keyword: HashMap::new(),
            storage_path,
        };
        
        store.load_from_disk();
        store
    }
    
    fn load_from_disk(&mut self) {
        let path = Path::new(&self.storage_path).join("local/methodologies");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path.join("index.json")) {
                if let Ok(data) = serde_json::from_str::<MethodologyStoreData>(&content) {
                    self.methodologies = data.methodologies;
                    self.rebuild_indices();
                }
            }
        }
    }
    
    fn rebuild_indices(&mut self) {
        self.by_category.clear();
        self.by_keyword.clear();
        
        for (id, m) in &self.methodologies {
            self.by_category.entry(m.category_id).or_default().push(*id);
            for kw in &m.keywords {
                self.by_keyword.entry(kw.to_lowercase()).or_default().push(*id);
            }
        }
    }
    
    fn get(&self, id: u64) -> Option<&Methodology> {
        self.methodologies.get(&id)
    }
    
    fn search_keywords(&self, keywords: &[String], limit: usize) -> Vec<&Methodology> {
        let mut matches: HashMap<u64, usize> = HashMap::new();
        
        for kw in keywords {
            if let Some(ids) = self.by_keyword.get(&kw.to_lowercase()) {
                for id in ids {
                    *matches.entry(*id).or_default() += 1;
                }
            }
        }
        
        let mut ranked: Vec<_> = matches.into_iter().collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1));
        
        ranked.into_iter()
            .take(limit)
            .filter_map(|(id, _)| self.methodologies.get(&id))
            .collect()
    }
    
    fn get_by_category(&self, category_id: u64, limit: usize) -> Vec<&Methodology> {
        self.by_category.get(&category_id)
            .map(|ids| ids.iter()
                .take(limit)
                .filter_map(|id| self.methodologies.get(id))
                .collect())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MethodologyStoreData {
    methodologies: HashMap<u64, Methodology>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum MethodologyFetchInput {
    /// Get by ID
    GetById { methodology_id: u64 },
    /// Search by keywords
    SearchByKeywords { keywords: Vec<String>, limit: Option<u32> },
    /// Search by topics
    SearchByTopics { topics: Vec<String>, limit: Option<u32> },
    /// Get by category
    GetByCategory { category_id: u64, limit: Option<u32> },
    /// Get for task (auto-match based on task signature)
    GetForTask { task_id: u64, input_types: Vec<String>, output_type: String },
    /// Get all in modality
    GetByModality { modality: String, limit: Option<u32> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Methodology {
    pub methodology_id: u64,
    pub name: String,
    pub description: String,
    pub category_id: u64,
    pub category_name: String,
    pub principles: Vec<Principle>,
    pub heuristics: Vec<Heuristic>,
    pub decision_rules: Vec<DecisionRule>,
    pub keywords: Vec<String>,
    pub topics: Vec<String>,
    pub created_at: u64,
    pub use_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Principle {
    pub name: String,
    pub description: String,
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heuristic {
    pub condition: String,
    pub action: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRule {
    pub name: String,
    pub condition: String,
    pub outcome: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodologyFetchOutput {
    pub success: bool,
    pub methodology: Option<Methodology>,
    pub methodologies: Option<Vec<Methodology>>,
    pub error: Option<String>,
}

pub async fn execute(input: MethodologyFetchInput) -> Result<MethodologyFetchOutput, String> {
    // Direct ZSEI methodology storage access
    let store = METHODOLOGY_STORE.lock().unwrap();
    
    match input {
        MethodologyFetchInput::GetById { methodology_id } => {
            let methodology = store.get(methodology_id).cloned();
            Ok(MethodologyFetchOutput { 
                success: methodology.is_some(), 
                methodology, 
                methodologies: None, 
                error: if store.get(methodology_id).is_none() { 
                    Some(format!("Methodology {} not found", methodology_id)) 
                } else { None }
            })
        }
        MethodologyFetchInput::SearchByKeywords { keywords, limit } => {
            let limit = limit.unwrap_or(20) as usize;
            let results: Vec<Methodology> = store.search_keywords(&keywords, limit)
                .into_iter().cloned().collect();
            Ok(MethodologyFetchOutput { 
                success: true, 
                methodology: None, 
                methodologies: Some(results), 
                error: None 
            })
        }
        MethodologyFetchInput::SearchByTopics { topics, limit } => {
            // Topics search uses same mechanism as keywords
            let limit = limit.unwrap_or(20) as usize;
            let results: Vec<Methodology> = store.search_keywords(&topics, limit)
                .into_iter().cloned().collect();
            Ok(MethodologyFetchOutput { 
                success: true, 
                methodology: None, 
                methodologies: Some(results), 
                error: None 
            })
        }
        MethodologyFetchInput::GetByCategory { category_id, limit } => {
            let limit = limit.unwrap_or(20) as usize;
            let results: Vec<Methodology> = store.get_by_category(category_id, limit)
                .into_iter().cloned().collect();
            Ok(MethodologyFetchOutput { 
                success: true, 
                methodology: None, 
                methodologies: Some(results), 
                error: None 
            })
        }
        MethodologyFetchInput::GetForTask { task_id, input_types, output_type } => {
            // Search methodologies relevant to task based on input/output types as keywords
            let mut keywords = input_types.clone();
            keywords.push(output_type);
            let results: Vec<Methodology> = store.search_keywords(&keywords, 10)
                .into_iter().cloned().collect();
            Ok(MethodologyFetchOutput { 
                success: true, 
                methodology: None, 
                methodologies: Some(results), 
                error: None 
            })
        }
        MethodologyFetchInput::GetByModality { modality, limit } => {
            // Search by modality as keyword
            let limit = limit.unwrap_or(20) as usize;
            let results: Vec<Methodology> = store.search_keywords(&[modality], limit)
                .into_iter().cloned().collect();
            Ok(MethodologyFetchOutput { 
                success: true, 
                methodology: None, 
                methodologies: Some(results), 
                error: None 
            })
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() { if args[i] == "--input" && i + 1 < args.len() { input_json = args[i + 1].clone(); } }
    let input: MethodologyFetchInput = serde_json::from_str(&input_json).unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { println!("{}", serde_json::json!({"success": false, "error": e})); std::process::exit(1); }
    }
}
