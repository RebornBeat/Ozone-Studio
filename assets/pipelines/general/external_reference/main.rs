//! ExternalReferencePipeline - Pipeline #28
//! Manage external references (files, URLs, packages).
//! Per spec §23: Context not copies - store semantic meaning, link by reference.
//!
//! Uses DIRECT storage access for reference management.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::path::Path;
use std::env;

// Direct reference store
lazy_static::lazy_static! {
    static ref REFERENCE_STORE: Mutex<ReferenceStore> = Mutex::new(ReferenceStore::new());
}

struct ReferenceStore {
    refs: HashMap<u64, ExternalRef>,
    by_project: HashMap<u64, Vec<u64>>,
    next_id: u64,
    storage_path: String,
}

impl ReferenceStore {
    fn new() -> Self {
        let storage_path = env::var("OZONE_ZSEI_PATH")
            .unwrap_or_else(|_| "./zsei_data".to_string());
        
        let mut store = Self {
            refs: HashMap::new(),
            by_project: HashMap::new(),
            next_id: 1,
            storage_path,
        };
        
        store.load_from_disk();
        store
    }
    
    fn load_from_disk(&mut self) {
        let path = Path::new(&self.storage_path).join("references");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path.join("index.json")) {
                if let Ok(data) = serde_json::from_str::<ReferenceStoreData>(&content) {
                    self.refs = data.refs;
                    self.by_project = data.by_project;
                    self.next_id = data.next_id;
                }
            }
        }
    }
    
    fn save_to_disk(&self) {
        let path = Path::new(&self.storage_path).join("references");
        let _ = std::fs::create_dir_all(&path);
        
        let data = ReferenceStoreData {
            refs: self.refs.clone(),
            by_project: self.by_project.clone(),
            next_id: self.next_id,
        };
        
        if let Ok(content) = serde_json::to_string_pretty(&data) {
            let _ = std::fs::write(path.join("index.json"), content);
        }
    }
    
    fn get_all(&self, project_id: u64) -> Vec<&ExternalRef> {
        self.by_project.get(&project_id)
            .map(|ids| ids.iter().filter_map(|id| self.refs.get(id)).collect())
            .unwrap_or_default()
    }
    
    fn get_by_type(&self, project_id: u64, ref_type: &str) -> Vec<&ExternalRef> {
        self.get_all(project_id)
            .into_iter()
            .filter(|r| r.ref_type == ref_type)
            .collect()
    }
    
    fn refresh(&mut self, ref_id: u64) -> Option<&ExternalRef> {
        if let Some(r) = self.refs.get_mut(&ref_id) {
            // Check if file exists
            if r.ref_type == "file" {
                r.healthy = Path::new(&r.path_or_url).exists();
            }
            // For URLs, would need to make HTTP request
            r.last_checked = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            
            self.save_to_disk();
        }
        self.refs.get(&ref_id)
    }
    
    fn check_health(&self, project_id: u64) -> HealthStatus {
        let refs = self.get_all(project_id);
        let total = refs.len() as u32;
        let healthy = refs.iter().filter(|r| r.healthy).count() as u32;
        let unhealthy = total - healthy;
        
        let issues: Vec<String> = refs.iter()
            .filter(|r| !r.healthy)
            .map(|r| format!("{}: {} ({})", r.ref_type, r.name, r.path_or_url))
            .collect();
        
        HealthStatus { total, healthy, unhealthy, issues }
    }
    
    fn add_reference(&mut self, project_id: u64, ref_type: String, path_or_url: String, name: String) -> u64 {
        let ref_id = self.next_id;
        self.next_id += 1;
        
        let healthy = if ref_type == "file" {
            Path::new(&path_or_url).exists()
        } else {
            true // Assume URLs are healthy until checked
        };
        
        let reference = ExternalRef {
            ref_id,
            ref_type,
            path_or_url,
            name,
            healthy,
            last_checked: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        self.refs.insert(ref_id, reference);
        self.by_project.entry(project_id).or_default().push(ref_id);
        self.save_to_disk();
        
        ref_id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReferenceStoreData {
    refs: HashMap<u64, ExternalRef>,
    by_project: HashMap<u64, Vec<u64>>,
    next_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ExternalRefInput {
    /// Get all references for a project
    GetAll { project_id: u64 },
    /// Get file references only
    GetFiles { project_id: u64 },
    /// Get URL references only
    GetURLs { project_id: u64 },
    /// Get package references only
    GetPackages { project_id: u64 },
    /// Refresh a specific reference
    Refresh { ref_id: u64 },
    /// Refresh all references for a project
    RefreshAll { project_id: u64 },
    /// Check health of all references
    CheckHealth { project_id: u64 },
    /// Add a new reference
    Add { project_id: u64, ref_type: String, path_or_url: String, name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalRef {
    pub ref_id: u64,
    pub ref_type: String,
    pub path_or_url: String,
    pub name: String,
    pub healthy: bool,
    pub last_checked: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalRefOutput {
    pub success: bool,
    pub refs: Option<Vec<ExternalRef>>,
    pub health_status: Option<HealthStatus>,
    pub ref_id: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub total: u32,
    pub healthy: u32,
    pub unhealthy: u32,
    pub issues: Vec<String>,
}

pub async fn execute(input: ExternalRefInput) -> Result<ExternalRefOutput, String> {
    match input {
        ExternalRefInput::GetAll { project_id } => {
            let store = REFERENCE_STORE.lock().unwrap();
            let refs: Vec<ExternalRef> = store.get_all(project_id).into_iter().cloned().collect();
            Ok(ExternalRefOutput {
                success: true,
                refs: Some(refs),
                health_status: None,
                ref_id: None,
                error: None,
            })
        }
        
        ExternalRefInput::GetFiles { project_id } => {
            let store = REFERENCE_STORE.lock().unwrap();
            let refs: Vec<ExternalRef> = store.get_by_type(project_id, "file").into_iter().cloned().collect();
            Ok(ExternalRefOutput {
                success: true,
                refs: Some(refs),
                health_status: None,
                ref_id: None,
                error: None,
            })
        }
        
        ExternalRefInput::GetURLs { project_id } => {
            let store = REFERENCE_STORE.lock().unwrap();
            let refs: Vec<ExternalRef> = store.get_by_type(project_id, "url").into_iter().cloned().collect();
            Ok(ExternalRefOutput {
                success: true,
                refs: Some(refs),
                health_status: None,
                ref_id: None,
                error: None,
            })
        }
        
        ExternalRefInput::GetPackages { project_id } => {
            let store = REFERENCE_STORE.lock().unwrap();
            let refs: Vec<ExternalRef> = store.get_by_type(project_id, "package").into_iter().cloned().collect();
            Ok(ExternalRefOutput {
                success: true,
                refs: Some(refs),
                health_status: None,
                ref_id: None,
                error: None,
            })
        }
        
        ExternalRefInput::Refresh { ref_id } => {
            let mut store = REFERENCE_STORE.lock().unwrap();
            let refreshed = store.refresh(ref_id).cloned();
            Ok(ExternalRefOutput {
                success: refreshed.is_some(),
                refs: refreshed.map(|r| vec![r]),
                health_status: None,
                ref_id: None,
                error: if refreshed.is_none() { Some(format!("Reference {} not found", ref_id)) } else { None },
            })
        }
        
        ExternalRefInput::RefreshAll { project_id } => {
            let mut store = REFERENCE_STORE.lock().unwrap();
            let ref_ids: Vec<u64> = store.by_project.get(&project_id)
                .cloned()
                .unwrap_or_default();
            
            for ref_id in ref_ids {
                store.refresh(ref_id);
            }
            
            let refs: Vec<ExternalRef> = store.get_all(project_id).into_iter().cloned().collect();
            Ok(ExternalRefOutput {
                success: true,
                refs: Some(refs),
                health_status: None,
                ref_id: None,
                error: None,
            })
        }
        
        ExternalRefInput::CheckHealth { project_id } => {
            let store = REFERENCE_STORE.lock().unwrap();
            let status = store.check_health(project_id);
            Ok(ExternalRefOutput {
                success: true,
                refs: None,
                health_status: Some(status),
                ref_id: None,
                error: None,
            })
        }
        
        ExternalRefInput::Add { project_id, ref_type, path_or_url, name } => {
            let mut store = REFERENCE_STORE.lock().unwrap();
            let ref_id = store.add_reference(project_id, ref_type, path_or_url, name);
            Ok(ExternalRefOutput {
                success: true,
                refs: None,
                health_status: None,
                ref_id: Some(ref_id),
                error: None,
            })
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() { if args[i] == "--input" && i + 1 < args.len() { input_json = args[i + 1].clone(); } }
    let input: ExternalRefInput = serde_json::from_str(&input_json).unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { println!("{}", serde_json::json!({"success": false, "error": e})); std::process::exit(1); }
    }
}
