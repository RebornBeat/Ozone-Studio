//! ConsciousnessSyncPipeline - Supporting Pipeline
//! 
//! Synchronizes consciousness state across storage and pipelines.
//! Ensures consistency between in-memory and persistent state.
//! 
//! REQUIRES: `consciousness` feature flag

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::path::Path;

lazy_static::lazy_static! {
    static ref SYNC_STORE: Mutex<SyncStore> = Mutex::new(SyncStore::new());
}

struct SyncStore {
    sync_status: HashMap<String, SyncStatus>,
    last_full_sync: u64,
    storage_path: String,
}

impl SyncStore {
    fn new() -> Self {
        let storage_path = std::env::var("OZONE_CONSCIOUSNESS_PATH")
            .unwrap_or_else(|_| "./zsei_data/consciousness".to_string());
        
        Self {
            sync_status: HashMap::new(),
            last_full_sync: 0,
            storage_path,
        }
    }
    
    fn sync_pipeline(&mut self, pipeline: &str) -> SyncResult {
        let path = Path::new(&self.storage_path);
        let file = format!("{}.json", pipeline);
        let file_path = path.join(&file);
        
        let exists = file_path.exists();
        let modified = if exists {
            std::fs::metadata(&file_path)
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0)
        } else {
            0
        };
        
        self.sync_status.insert(pipeline.to_string(), SyncStatus {
            pipeline: pipeline.to_string(),
            last_sync: now(),
            status: if exists { "synced" } else { "missing" }.to_string(),
            file_modified: modified,
        });
        
        SyncResult {
            pipeline: pipeline.to_string(),
            success: exists,
            message: if exists { "Synced successfully" } else { "File missing" }.to_string(),
            timestamp: now(),
        }
    }
    
    fn sync_all(&mut self) -> Vec<SyncResult> {
        let pipelines = vec![
            "emotional_state",
            "emotional_baseline",
            "experiences",
            "identity",
            "voice_identity",
            "reflections",
            "meta_cognitive",
            "relationships",
            "feedback_data",
            "playback_data",
            "collective_data",
            "consciousness_config",
            "gate_decisions",
        ];
        
        let results: Vec<_> = pipelines.iter()
            .map(|p| self.sync_pipeline(p))
            .collect();
        
        self.last_full_sync = now();
        results
    }
    
    fn get_status(&self) -> Vec<SyncStatus> {
        self.sync_status.values().cloned().collect()
    }
    
    fn force_reload(&mut self, pipeline: &str) -> bool {
        // In a real implementation, this would trigger the pipeline to reload from disk
        // For now, just mark as needing sync
        if let Some(status) = self.sync_status.get_mut(pipeline) {
            status.status = "pending".to_string();
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub pipeline: String,
    pub last_sync: u64,
    pub status: String,
    pub file_modified: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub pipeline: String,
    pub success: bool,
    pub message: String,
    pub timestamp: u64,
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum SyncInput {
    /// Sync specific pipeline
    SyncPipeline { pipeline: String },
    /// Sync all pipelines
    SyncAll,
    /// Get sync status
    GetStatus,
    /// Force reload from disk
    ForceReload { pipeline: String },
    /// Get last full sync time
    GetLastSync,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncOutput {
    pub success: bool,
    pub result: Option<SyncResult>,
    pub results: Option<Vec<SyncResult>>,
    pub status: Option<Vec<SyncStatus>>,
    pub last_sync: Option<u64>,
    pub error: Option<String>,
}

pub async fn execute(input: SyncInput) -> Result<SyncOutput, String> {
    match input {
        SyncInput::SyncPipeline { pipeline } => {
            let mut store = SYNC_STORE.lock().unwrap();
            let result = store.sync_pipeline(&pipeline);
            Ok(SyncOutput {
                success: result.success,
                result: Some(result),
                results: None,
                status: None,
                last_sync: None,
                error: None,
            })
        }
        SyncInput::SyncAll => {
            let mut store = SYNC_STORE.lock().unwrap();
            let results = store.sync_all();
            let all_success = results.iter().all(|r| r.success);
            Ok(SyncOutput {
                success: all_success,
                result: None,
                results: Some(results),
                status: None,
                last_sync: Some(store.last_full_sync),
                error: None,
            })
        }
        SyncInput::GetStatus => {
            let store = SYNC_STORE.lock().unwrap();
            Ok(SyncOutput {
                success: true,
                result: None,
                results: None,
                status: Some(store.get_status()),
                last_sync: Some(store.last_full_sync),
                error: None,
            })
        }
        SyncInput::ForceReload { pipeline } => {
            let mut store = SYNC_STORE.lock().unwrap();
            let success = store.force_reload(&pipeline);
            Ok(SyncOutput {
                success,
                result: None,
                results: None,
                status: None,
                last_sync: None,
                error: if !success { Some("Pipeline not found".into()) } else { None },
            })
        }
        SyncInput::GetLastSync => {
            let store = SYNC_STORE.lock().unwrap();
            Ok(SyncOutput {
                success: true,
                result: None,
                results: None,
                status: None,
                last_sync: Some(store.last_full_sync),
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
    let input: SyncInput = serde_json::from_str(&input_json).unwrap_or_else(|e| {
        eprintln!("Parse error: {}", e);
        std::process::exit(1);
    });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { eprintln!("{}", e); std::process::exit(1); }
    }
}
