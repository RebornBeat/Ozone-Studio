//! IntegrityCheckPipeline - Pipeline #26
//! Verify ZSEI integrity, detect corruption, enable rollback.
//! Per spec §25: No information loss, always recoverable.
//!
//! Uses DIRECT ZSEI storage access for integrity verification.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::path::Path;
use std::env;

// Direct integrity store - persists check results
lazy_static::lazy_static! {
    static ref INTEGRITY_STORE: Mutex<IntegrityStore> = Mutex::new(IntegrityStore::new());
}

struct IntegrityStore {
    checks: HashMap<u64, IntegrityResult>,
    versions: HashMap<u64, Vec<VersionInfo>>,
    snapshots: HashMap<u64, u64>,
    storage_path: String,
}

impl IntegrityStore {
    fn new() -> Self {
        let storage_path = env::var("OZONE_ZSEI_PATH")
            .unwrap_or_else(|_| "./zsei_data".to_string());
        
        let mut store = Self {
            checks: HashMap::new(),
            versions: HashMap::new(),
            snapshots: HashMap::new(),
            storage_path,
        };
        
        store.load_from_disk();
        store
    }
    
    fn load_from_disk(&mut self) {
        let path = Path::new(&self.storage_path).join("integrity");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path.join("checks.json")) {
                if let Ok(data) = serde_json::from_str(&content) {
                    self.checks = data;
                }
            }
            if let Ok(content) = std::fs::read_to_string(path.join("versions.json")) {
                if let Ok(data) = serde_json::from_str(&content) {
                    self.versions = data;
                }
            }
        }
    }
    
    fn save_to_disk(&self) {
        let path = Path::new(&self.storage_path).join("integrity");
        let _ = std::fs::create_dir_all(&path);
        
        if let Ok(content) = serde_json::to_string_pretty(&self.checks) {
            let _ = std::fs::write(path.join("checks.json"), content);
        }
        if let Ok(content) = serde_json::to_string_pretty(&self.versions) {
            let _ = std::fs::write(path.join("versions.json"), content);
        }
    }
    
    fn check_container(&mut self, container_id: u64) -> IntegrityResult {
        // Check if container file exists
        let container_path = Path::new(&self.storage_path)
            .join("local")
            .join(format!("{}.json", container_id));
        
        let mut issues = Vec::new();
        let mut valid = true;
        let mut current_version = 1u32;
        
        if container_path.exists() {
            // Verify JSON is valid
            match std::fs::read_to_string(&container_path) {
                Ok(content) => {
                    if serde_json::from_str::<serde_json::Value>(&content).is_err() {
                        issues.push("Invalid JSON format".to_string());
                        valid = false;
                    }
                    
                    // Extract version if present
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(v) = json.get("version").and_then(|v| v.as_u64()) {
                            current_version = v as u32;
                        }
                    }
                }
                Err(e) => {
                    issues.push(format!("Cannot read container: {}", e));
                    valid = false;
                }
            }
            
            // Verify hash if stored
            let hash_path = container_path.with_extension("hash");
            if hash_path.exists() {
                if let (Ok(stored_hash), Ok(content)) = (
                    std::fs::read_to_string(&hash_path),
                    std::fs::read(&container_path)
                ) {
                    let computed_hash = format!("{:x}", md5::compute(&content));
                    if stored_hash.trim() != computed_hash {
                        issues.push("Hash mismatch - possible corruption".to_string());
                        valid = false;
                    }
                }
            }
        } else {
            issues.push("Container file not found".to_string());
            valid = false;
        }
        
        let result = IntegrityResult {
            container_id,
            valid,
            issues,
            current_version,
        };
        
        self.checks.insert(container_id, result.clone());
        self.save_to_disk();
        
        result
    }
    
    fn get_version_history(&self, container_id: u64) -> Vec<VersionInfo> {
        self.versions.get(&container_id).cloned().unwrap_or_else(|| {
            // Check versions directory
            let versions_path = Path::new(&self.storage_path)
                .join("versions")
                .join(container_id.to_string());
            
            let mut history = Vec::new();
            if versions_path.exists() {
                if let Ok(entries) = std::fs::read_dir(&versions_path) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            if let Ok(version) = name.trim_end_matches(".json").parse::<u32>() {
                                if let Ok(metadata) = entry.metadata() {
                                    let timestamp = metadata.modified()
                                        .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs())
                                        .unwrap_or(0);
                                    
                                    history.push(VersionInfo {
                                        version,
                                        timestamp,
                                        hash: "computed".to_string(),
                                        changes: format!("Version {}", version),
                                    });
                                }
                            }
                        }
                    }
                }
            }
            
            if history.is_empty() {
                vec![VersionInfo {
                    version: 1,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    hash: "current".to_string(),
                    changes: "Current version".to_string(),
                }]
            } else {
                history.sort_by(|a, b| b.version.cmp(&a.version));
                history
            }
        })
    }
    
    fn create_snapshot(&mut self, container_id: u64) -> u64 {
        let snapshot_id = container_id * 1000 + 
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64 % 1000;
        
        // Copy current container to versions
        let container_path = Path::new(&self.storage_path)
            .join("local")
            .join(format!("{}.json", container_id));
        
        let versions_path = Path::new(&self.storage_path)
            .join("versions")
            .join(container_id.to_string());
        
        let _ = std::fs::create_dir_all(&versions_path);
        
        if container_path.exists() {
            let version = self.versions.get(&container_id)
                .map(|v| v.len() as u32 + 1)
                .unwrap_or(1);
            
            let snapshot_path = versions_path.join(format!("{}.json", version));
            let _ = std::fs::copy(&container_path, &snapshot_path);
            
            // Record version info
            let version_info = VersionInfo {
                version,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                hash: format!("snapshot_{}", snapshot_id),
                changes: "Manual snapshot".to_string(),
            };
            
            self.versions.entry(container_id).or_default().push(version_info);
            self.snapshots.insert(container_id, snapshot_id);
            self.save_to_disk();
        }
        
        snapshot_id
    }
    
    fn rollback(&mut self, container_id: u64, target_version: u32) -> Result<(), String> {
        let versions_path = Path::new(&self.storage_path)
            .join("versions")
            .join(container_id.to_string())
            .join(format!("{}.json", target_version));
        
        if !versions_path.exists() {
            return Err(format!("Version {} not found for container {}", target_version, container_id));
        }
        
        let container_path = Path::new(&self.storage_path)
            .join("local")
            .join(format!("{}.json", container_id));
        
        // Create backup of current before rollback
        self.create_snapshot(container_id);
        
        // Restore from version
        std::fs::copy(&versions_path, &container_path)
            .map_err(|e| format!("Rollback failed: {}", e))?;
        
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum IntegrityInput {
    /// Check single container
    CheckContainer { container_id: u64 },
    /// Check container and all children
    CheckSubtree { root_id: u64 },
    /// Full system check
    CheckAll,
    /// Get version history
    GetVersionHistory { container_id: u64 },
    /// Rollback to previous version
    Rollback { container_id: u64, target_version: u32 },
    /// Create snapshot for recovery
    CreateSnapshot { container_id: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityResult {
    pub container_id: u64,
    pub valid: bool,
    pub issues: Vec<String>,
    pub current_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: u32,
    pub timestamp: u64,
    pub hash: String,
    pub changes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityOutput {
    pub success: bool,
    pub results: Option<Vec<IntegrityResult>>,
    pub versions: Option<Vec<VersionInfo>>,
    pub snapshot_id: Option<u64>,
    pub error: Option<String>,
}

pub async fn execute(input: IntegrityInput) -> Result<IntegrityOutput, String> {
    match input {
        IntegrityInput::CheckContainer { container_id } => {
            let mut store = INTEGRITY_STORE.lock().unwrap();
            let result = store.check_container(container_id);
            Ok(IntegrityOutput {
                success: result.valid,
                results: Some(vec![result]),
                versions: None,
                snapshot_id: None,
                error: None,
            })
        }
        
        IntegrityInput::CheckSubtree { root_id } => {
            let mut store = INTEGRITY_STORE.lock().unwrap();
            let mut results = vec![store.check_container(root_id)];
            
            // Check children (would traverse ZSEI structure)
            // For now, just check the root
            
            let all_valid = results.iter().all(|r| r.valid);
            Ok(IntegrityOutput {
                success: all_valid,
                results: Some(results),
                versions: None,
                snapshot_id: None,
                error: None,
            })
        }
        
        IntegrityInput::CheckAll => {
            let storage_path = env::var("OZONE_ZSEI_PATH")
                .unwrap_or_else(|_| "./zsei_data".to_string());
            
            let local_path = Path::new(&storage_path).join("local");
            let mut results = Vec::new();
            
            if local_path.exists() {
                let mut store = INTEGRITY_STORE.lock().unwrap();
                
                if let Ok(entries) = std::fs::read_dir(&local_path) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            if let Ok(container_id) = name.trim_end_matches(".json").parse::<u64>() {
                                results.push(store.check_container(container_id));
                            }
                        }
                    }
                }
            }
            
            let all_valid = results.iter().all(|r| r.valid);
            Ok(IntegrityOutput {
                success: all_valid,
                results: Some(results),
                versions: None,
                snapshot_id: None,
                error: if !all_valid { Some("Some containers have integrity issues".into()) } else { None },
            })
        }
        
        IntegrityInput::GetVersionHistory { container_id } => {
            let store = INTEGRITY_STORE.lock().unwrap();
            let versions = store.get_version_history(container_id);
            Ok(IntegrityOutput {
                success: true,
                results: None,
                versions: Some(versions),
                snapshot_id: None,
                error: None,
            })
        }
        
        IntegrityInput::Rollback { container_id, target_version } => {
            let mut store = INTEGRITY_STORE.lock().unwrap();
            match store.rollback(container_id, target_version) {
                Ok(()) => Ok(IntegrityOutput {
                    success: true,
                    results: None,
                    versions: None,
                    snapshot_id: None,
                    error: None,
                }),
                Err(e) => Ok(IntegrityOutput {
                    success: false,
                    results: None,
                    versions: None,
                    snapshot_id: None,
                    error: Some(e),
                })
            }
        }
        
        IntegrityInput::CreateSnapshot { container_id } => {
            let mut store = INTEGRITY_STORE.lock().unwrap();
            let snapshot_id = store.create_snapshot(container_id);
            Ok(IntegrityOutput {
                success: true,
                results: None,
                versions: None,
                snapshot_id: Some(snapshot_id),
                error: None,
            })
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() { if args[i] == "--input" && i + 1 < args.len() { input_json = args[i + 1].clone(); } }
    let input: IntegrityInput = serde_json::from_str(&input_json).unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { println!("{}", serde_json::json!({"success": false, "error": e})); std::process::exit(1); }
    }
}
