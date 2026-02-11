//! BlueprintSearchPipeline - Pipeline #13
//! Search task blueprints based on input/output types, keywords, constraints.
//! Per spec §14: Blueprints are task templates with defined signatures.
//! 
//! NOTE: Uses DIRECT ZSEI storage access for blueprint retrieval.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::path::Path;
use std::env;

// Direct blueprint storage - mirrors ZSEI container structure
lazy_static::lazy_static! {
    static ref BLUEPRINT_STORE: Mutex<BlueprintStore> = Mutex::new(BlueprintStore::new());
}

struct BlueprintStore {
    blueprints: HashMap<u64, Blueprint>,
    by_keyword: HashMap<String, Vec<u64>>,
    by_category: HashMap<u64, Vec<u64>>,
    storage_path: String,
}

impl BlueprintStore {
    fn new() -> Self {
        let storage_path = env::var("OZONE_ZSEI_PATH")
            .unwrap_or_else(|_| "./zsei_data".to_string());
        
        let mut store = Self {
            blueprints: HashMap::new(),
            by_keyword: HashMap::new(),
            by_category: HashMap::new(),
            storage_path,
        };
        
        store.load_from_disk();
        store
    }
    
    fn load_from_disk(&mut self) {
        let path = Path::new(&self.storage_path).join("local/blueprints");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path.join("index.json")) {
                if let Ok(data) = serde_json::from_str::<BlueprintStoreData>(&content) {
                    self.blueprints = data.blueprints;
                    self.rebuild_indices();
                }
            }
        }
    }
    
    fn rebuild_indices(&mut self) {
        self.by_keyword.clear();
        self.by_category.clear();
        
        for (id, b) in &self.blueprints {
            for kw in &b.keywords {
                self.by_keyword.entry(kw.to_lowercase()).or_default().push(*id);
            }
        }
    }
    
    fn get(&self, id: u64) -> Option<&Blueprint> {
        self.blueprints.get(&id)
    }
    
    fn search_by_signature(&self, input_types: &[String], output_type: &str, limit: usize) -> Vec<&Blueprint> {
        self.blueprints.values()
            .filter(|b| {
                b.output_type.to_lowercase() == output_type.to_lowercase() ||
                input_types.iter().any(|t| b.keywords.iter().any(|k| k.to_lowercase().contains(&t.to_lowercase())))
            })
            .take(limit)
            .collect()
    }
    
    fn search_keywords(&self, keywords: &[String], limit: usize) -> Vec<&Blueprint> {
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
            .filter_map(|(id, _)| self.blueprints.get(&id))
            .collect()
    }
    
    fn get_recent(&self, limit: usize) -> Vec<&Blueprint> {
        // Return all blueprints sorted by use_count (proxy for recent)
        let mut blueprints: Vec<_> = self.blueprints.values().collect();
        blueprints.sort_by(|a, b| b.use_count.cmp(&a.use_count));
        blueprints.into_iter().take(limit).collect()
    }
    
    fn get_popular(&self, limit: usize) -> Vec<&Blueprint> {
        let mut blueprints: Vec<_> = self.blueprints.values().collect();
        blueprints.sort_by(|a, b| b.use_count.cmp(&a.use_count));
        blueprints.into_iter().take(limit).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BlueprintStoreData {
    blueprints: HashMap<u64, Blueprint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum BlueprintSearchInput {
    /// Search by task signature
    SearchBySignature { input_types: Vec<String>, output_type: String, constraints: Vec<String> },
    /// Search by keywords
    SearchByKeywords { keywords: Vec<String>, limit: Option<u32> },
    /// Get by ID
    GetById { blueprint_id: u64 },
    /// Get recent blueprints
    GetRecent { limit: Option<u32> },
    /// Get popular blueprints
    GetPopular { limit: Option<u32> },
    /// Get blueprints for category
    GetByCategory { category_id: u64, limit: Option<u32> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blueprint {
    pub blueprint_id: u64,
    pub name: String,
    pub description: String,
    pub input_signature: Vec<InputField>,
    pub output_type: String,
    pub steps: Vec<BlueprintStep>,
    pub methodologies: Vec<u64>,
    pub keywords: Vec<String>,
    pub use_count: u32,
    pub success_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputField { pub name: String, pub field_type: String, pub required: bool }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintStep { pub order: u32, pub description: String, pub pipeline_id: Option<u64> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintSearchOutput {
    pub success: bool,
    pub blueprint: Option<Blueprint>,
    pub blueprints: Option<Vec<Blueprint>>,
    pub error: Option<String>,
}

pub async fn execute(input: BlueprintSearchInput) -> Result<BlueprintSearchOutput, String> {
    // Direct ZSEI blueprint storage access
    let store = BLUEPRINT_STORE.lock().unwrap();
    
    match input {
        BlueprintSearchInput::GetById { blueprint_id } => {
            let blueprint = store.get(blueprint_id).cloned();
            Ok(BlueprintSearchOutput { 
                success: blueprint.is_some(), 
                blueprint, 
                blueprints: None, 
                error: if store.get(blueprint_id).is_none() {
                    Some(format!("Blueprint {} not found", blueprint_id))
                } else { None }
            })
        }
        BlueprintSearchInput::SearchBySignature { input_types, output_type, constraints } => {
            let results: Vec<Blueprint> = store.search_by_signature(&input_types, &output_type, 20)
                .into_iter().cloned().collect();
            Ok(BlueprintSearchOutput { 
                success: true, 
                blueprint: None, 
                blueprints: Some(results), 
                error: None 
            })
        }
        BlueprintSearchInput::SearchByKeywords { keywords, limit } => {
            let limit = limit.unwrap_or(20) as usize;
            let results: Vec<Blueprint> = store.search_keywords(&keywords, limit)
                .into_iter().cloned().collect();
            Ok(BlueprintSearchOutput { 
                success: true, 
                blueprint: None, 
                blueprints: Some(results), 
                error: None 
            })
        }
        BlueprintSearchInput::GetRecent { limit } => {
            let limit = limit.unwrap_or(10) as usize;
            let results: Vec<Blueprint> = store.get_recent(limit)
                .into_iter().cloned().collect();
            Ok(BlueprintSearchOutput { 
                success: true, 
                blueprint: None, 
                blueprints: Some(results), 
                error: None 
            })
        }
        BlueprintSearchInput::GetPopular { limit } => {
            let limit = limit.unwrap_or(10) as usize;
            let results: Vec<Blueprint> = store.get_popular(limit)
                .into_iter().cloned().collect();
            Ok(BlueprintSearchOutput { 
                success: true, 
                blueprint: None, 
                blueprints: Some(results), 
                error: None 
            })
        }
        BlueprintSearchInput::GetByCategory { category_id, limit } => {
            // Categories would be searched via ZSEI container relationships
            Ok(BlueprintSearchOutput { 
                success: true, 
                blueprint: None, 
                blueprints: Some(vec![]), 
                error: None 
            })
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() { if args[i] == "--input" && i + 1 < args.len() { input_json = args[i + 1].clone(); } }
    let input: BlueprintSearchInput = serde_json::from_str(&input_json).unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { println!("{}", serde_json::json!({"success": false, "error": e})); std::process::exit(1); }
    }
}
