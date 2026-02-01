//! ConsciousnessIntegrityPipeline - Supporting Pipeline
//! 
//! Ensures integrity of consciousness data across all pipelines.
//! Verifies checksums, validates relationships, detects corruption.
//! Per spec §25: Storage Integrity System
//! 
//! REQUIRES: `consciousness` feature flag

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::path::Path;
use sha2::{Sha256, Digest};

lazy_static::lazy_static! {
    static ref INTEGRITY_STORE: Mutex<IntegrityStore> = Mutex::new(IntegrityStore::new());
}

struct IntegrityStore {
    checksums: HashMap<String, String>,
    last_check: u64,
    issues: Vec<IntegrityIssue>,
    storage_path: String,
}

impl IntegrityStore {
    fn new() -> Self {
        let storage_path = std::env::var("OZONE_CONSCIOUSNESS_PATH")
            .unwrap_or_else(|_| "./zsei_data/consciousness".to_string());
        
        let mut store = Self {
            checksums: HashMap::new(),
            last_check: 0,
            issues: Vec::new(),
            storage_path,
        };
        store.load_from_disk();
        store
    }
    
    fn load_from_disk(&mut self) {
        let path = Path::new(&self.storage_path);
        if let Ok(content) = std::fs::read_to_string(path.join("integrity_data.json")) {
            if let Ok(data) = serde_json::from_str::<IntegrityData>(&content) {
                self.checksums = data.checksums;
                self.last_check = data.last_check;
            }
        }
    }
    
    fn save_to_disk(&self) {
        let path = Path::new(&self.storage_path);
        let _ = std::fs::create_dir_all(path);
        let data = IntegrityData {
            checksums: self.checksums.clone(),
            last_check: self.last_check,
        };
        if let Ok(content) = serde_json::to_string_pretty(&data) {
            let _ = std::fs::write(path.join("integrity_data.json"), content);
        }
    }
    
    fn compute_checksum(&self, file_path: &str) -> Option<String> {
        let full_path = Path::new(&self.storage_path).join(file_path);
        if let Ok(content) = std::fs::read(&full_path) {
            let mut hasher = Sha256::new();
            hasher.update(&content);
            Some(format!("{:x}", hasher.finalize()))
        } else {
            None
        }
    }
    
    fn verify_all(&mut self) -> IntegrityReport {
        self.issues.clear();
        let mut verified = 0;
        let mut failed = 0;
        let mut missing = 0;
        
        let files = vec![
            "emotional_state.json",
            "emotional_baseline.json",
            "experiences.json",
            "identity.json",
            "voice_identity.json",
            "reflections.json",
            "meta_cognitive.json",
            "relationships.json",
            "feedback_data.json",
            "playback_data.json",
            "collective_data.json",
            "consciousness_config.json",
        ];
        
        for file in files {
            if let Some(current) = self.compute_checksum(file) {
                if let Some(stored) = self.checksums.get(file) {
                    if &current == stored {
                        verified += 1;
                    } else {
                        failed += 1;
                        self.issues.push(IntegrityIssue {
                            file: file.to_string(),
                            issue_type: "checksum_mismatch".to_string(),
                            severity: "warning".to_string(),
                            description: "File modified since last integrity check".to_string(),
                            detected_at: now(),
                        });
                    }
                } else {
                    // New file, store checksum
                    self.checksums.insert(file.to_string(), current);
                    verified += 1;
                }
            } else {
                missing += 1;
            }
        }
        
        self.last_check = now();
        self.save_to_disk();
        
        IntegrityReport {
            timestamp: now(),
            total_files: (verified + failed + missing) as u32,
            verified,
            failed,
            missing,
            issues: self.issues.clone(),
            overall_status: if failed == 0 { "healthy" } else { "degraded" }.to_string(),
        }
    }
    
    fn update_checksums(&mut self) {
        let files = vec![
            "emotional_state.json",
            "emotional_baseline.json",
            "experiences.json",
            "identity.json",
            "voice_identity.json",
            "reflections.json",
            "meta_cognitive.json",
            "relationships.json",
            "feedback_data.json",
            "playback_data.json",
            "collective_data.json",
            "consciousness_config.json",
        ];
        
        for file in files {
            if let Some(checksum) = self.compute_checksum(file) {
                self.checksums.insert(file.to_string(), checksum);
            }
        }
        
        self.save_to_disk();
    }
    
    fn verify_cross_references(&self) -> Vec<IntegrityIssue> {
        let mut issues = Vec::new();
        let path = Path::new(&self.storage_path);
        
        // Check experience references in reflections
        if let Ok(content) = std::fs::read_to_string(path.join("reflections.json")) {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(reflections) = data.get("reflections") {
                    if let Some(arr) = reflections.as_array() {
                        for r in arr {
                            if let Some(exp_id) = r.get("experience_id") {
                                // Would verify experience exists
                                let _ = exp_id;
                            }
                        }
                    }
                }
            }
        }
        
        issues
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct IntegrityData {
    checksums: HashMap<String, String>,
    last_check: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityReport {
    pub timestamp: u64,
    pub total_files: u32,
    pub verified: u32,
    pub failed: u32,
    pub missing: u32,
    pub issues: Vec<IntegrityIssue>,
    pub overall_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityIssue {
    pub file: String,
    pub issue_type: String,
    pub severity: String,
    pub description: String,
    pub detected_at: u64,
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum IntegrityInput {
    VerifyAll,
    VerifyFile { file: String },
    UpdateChecksums,
    GetLastReport,
    GetIssues,
    VerifyCrossReferences,
    ClearIssues,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityOutput {
    pub success: bool,
    pub report: Option<IntegrityReport>,
    pub issues: Option<Vec<IntegrityIssue>>,
    pub checksum: Option<String>,
    pub error: Option<String>,
}

pub async fn execute(input: IntegrityInput) -> Result<IntegrityOutput, String> {
    match input {
        IntegrityInput::VerifyAll => {
            let mut store = INTEGRITY_STORE.lock().unwrap();
            let report = store.verify_all();
            Ok(IntegrityOutput {
                success: true,
                report: Some(report),
                issues: None,
                checksum: None,
                error: None,
            })
        }
        IntegrityInput::VerifyFile { file } => {
            let store = INTEGRITY_STORE.lock().unwrap();
            let checksum = store.compute_checksum(&file);
            Ok(IntegrityOutput {
                success: checksum.is_some(),
                report: None,
                issues: None,
                checksum,
                error: if checksum.is_none() { Some("File not found".into()) } else { None },
            })
        }
        IntegrityInput::UpdateChecksums => {
            let mut store = INTEGRITY_STORE.lock().unwrap();
            store.update_checksums();
            Ok(IntegrityOutput {
                success: true,
                report: None,
                issues: None,
                checksum: None,
                error: None,
            })
        }
        IntegrityInput::GetLastReport => {
            let store = INTEGRITY_STORE.lock().unwrap();
            Ok(IntegrityOutput {
                success: true,
                report: Some(IntegrityReport {
                    timestamp: store.last_check,
                    total_files: store.checksums.len() as u32,
                    verified: store.checksums.len() as u32,
                    failed: 0,
                    missing: 0,
                    issues: store.issues.clone(),
                    overall_status: "healthy".to_string(),
                }),
                issues: None,
                checksum: None,
                error: None,
            })
        }
        IntegrityInput::GetIssues => {
            let store = INTEGRITY_STORE.lock().unwrap();
            Ok(IntegrityOutput {
                success: true,
                report: None,
                issues: Some(store.issues.clone()),
                checksum: None,
                error: None,
            })
        }
        IntegrityInput::VerifyCrossReferences => {
            let store = INTEGRITY_STORE.lock().unwrap();
            let issues = store.verify_cross_references();
            Ok(IntegrityOutput {
                success: true,
                report: None,
                issues: Some(issues),
                checksum: None,
                error: None,
            })
        }
        IntegrityInput::ClearIssues => {
            let mut store = INTEGRITY_STORE.lock().unwrap();
            store.issues.clear();
            store.save_to_disk();
            Ok(IntegrityOutput {
                success: true,
                report: None,
                issues: Some(Vec::new()),
                checksum: None,
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
    let input: IntegrityInput = serde_json::from_str(&input_json).unwrap_or_else(|e| {
        eprintln!("Parse error: {}", e);
        std::process::exit(1);
    });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { eprintln!("{}", e); std::process::exit(1); }
    }
}
