//! PackageRelationshipPipeline - Pipeline #29
//! Analyze relationships between packages.

use serde::{Deserialize, Serialize};

#[path = "../../shared/ozone_serve.rs"]
mod ozone_serve;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum PackageRelInput {
    GetDependencies { package_id: u64 },
    GetDependents { package_id: u64 },
    GetRelationshipGraph { package_ids: Vec<u64> },
    FindConflicts { package_ids: Vec<u64> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageRelOutput {
    pub success: bool,
    pub dependencies: Option<Vec<u64>>,
    pub dependents: Option<Vec<u64>>,
    pub conflicts: Option<Vec<String>>,
    pub error: Option<String>,
}

pub async fn execute(input: PackageRelInput) -> Result<PackageRelOutput, String> {
    match input {
        PackageRelInput::GetDependencies { package_id } => {
            Ok(PackageRelOutput { success: true, dependencies: Some(vec![]), dependents: None, conflicts: None, error: None })
        }
        PackageRelInput::GetDependents { package_id } => {
            Ok(PackageRelOutput { success: true, dependencies: None, dependents: Some(vec![]), conflicts: None, error: None })
        }
        PackageRelInput::FindConflicts { package_ids } => {
            Ok(PackageRelOutput { success: true, dependencies: None, dependents: None, conflicts: Some(vec![]), error: None })
        }
        _ => Ok(PackageRelOutput { success: true, dependencies: None, dependents: None, conflicts: None, error: None })
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() { if args[i] == "--input" && i + 1 < args.len() { input_json = ozone_input_arg(&args[i + 1]); } }
    let input: PackageRelInput = serde_json::from_str(&input_json).unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { println!("{}", serde_json::json!({"success": false, "error": e})); std::process::exit(1); }
    }
}

/// `--input @<path>`: the host stages oversize inputs (> 100 KB) in a file because a
/// single argv string is capped at 128 KiB (E2BIG). Plain values pass through unchanged.
#[allow(dead_code)]
fn ozone_input_arg(raw: &str) -> String {
    match raw.strip_prefix('@') {
        Some(path) if std::path::Path::new(path).is_file() => std::fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("Failed to read staged input {}: {}", path, e);
            std::process::exit(1);
        }),
        _ => raw.to_string(),
    }
}
