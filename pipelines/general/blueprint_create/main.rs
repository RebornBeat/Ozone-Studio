//! BlueprintCreatePipeline - Pipeline #14
//! Create and update task blueprints. Per spec §14.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum BlueprintCreateInput {
    Create {
        name: String,
        description: String,
        input_signature: Vec<InputField>,
        output_type: String,
        steps: Vec<StepInput>,
        methodologies: Vec<u64>,
        keywords: Vec<String>,
    },
    Update {
        blueprint_id: u64,
        name: Option<String>,
        description: Option<String>,
        add_steps: Option<Vec<StepInput>>,
        remove_steps: Option<Vec<u32>>,
        add_keywords: Option<Vec<String>>,
    },
    Delete { blueprint_id: u64 },
    Clone { blueprint_id: u64, new_name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputField { pub name: String, pub field_type: String, pub required: bool }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepInput { pub order: u32, pub description: String, pub pipeline_id: Option<u64> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintCreateOutput {
    pub success: bool,
    pub blueprint_id: Option<u64>,
    pub version: Option<u32>,
    pub error: Option<String>,
}

fn gen_id() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64 }

pub async fn execute(input: BlueprintCreateInput) -> Result<BlueprintCreateOutput, String> {
    match input {
        BlueprintCreateInput::Create { name, .. } => {
            Ok(BlueprintCreateOutput { success: true, blueprint_id: Some(gen_id()), version: Some(1), error: None })
        }
        BlueprintCreateInput::Update { blueprint_id, .. } => {
            Ok(BlueprintCreateOutput { success: true, blueprint_id: Some(blueprint_id), version: Some(2), error: None })
        }
        BlueprintCreateInput::Delete { blueprint_id } => {
            Ok(BlueprintCreateOutput { success: true, blueprint_id: Some(blueprint_id), version: None, error: None })
        }
        BlueprintCreateInput::Clone { blueprint_id, new_name } => {
            Ok(BlueprintCreateOutput { success: true, blueprint_id: Some(gen_id()), version: Some(1), error: None })
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() { if args[i] == "--input" && i + 1 < args.len() { input_json = args[i + 1].clone(); } }
    let input: BlueprintCreateInput = serde_json::from_str(&input_json).unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { println!("{}", serde_json::json!({"success": false, "error": e})); std::process::exit(1); }
    }
}
