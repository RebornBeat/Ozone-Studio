//! MethodologyCreatePipeline - Pipeline #12
//! Create and update methodologies in ZSEI.
//! Methodologies are reusable approaches that can be shared via consensus.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum MethodologyCreateInput {
    Create {
        name: String,
        description: String,
        category_id: u64,
        principles: Vec<PrincipleInput>,
        heuristics: Vec<HeuristicInput>,
        decision_rules: Vec<DecisionRuleInput>,
        keywords: Vec<String>,
        topics: Vec<String>,
    },
    Update {
        methodology_id: u64,
        name: Option<String>,
        description: Option<String>,
        add_principles: Option<Vec<PrincipleInput>>,
        remove_principles: Option<Vec<String>>,
        add_heuristics: Option<Vec<HeuristicInput>>,
        add_keywords: Option<Vec<String>>,
        remove_keywords: Option<Vec<String>>,
    },
    Delete { methodology_id: u64 },
    Propose { methodology_id: u64 }, // Propose for network consensus
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipleInput { pub name: String, pub description: String, pub priority: u8 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeuristicInput { pub condition: String, pub action: String, pub confidence: f32 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRuleInput { pub name: String, pub condition: String, pub outcome: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodologyCreateOutput {
    pub success: bool,
    pub methodology_id: Option<u64>,
    pub version: Option<u32>,
    pub proposal_id: Option<u64>,
    pub error: Option<String>,
}

pub async fn execute(input: MethodologyCreateInput) -> Result<MethodologyCreateOutput, String> {
    match input {
        MethodologyCreateInput::Create { name, description, category_id, .. } => {
            let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64;
            Ok(MethodologyCreateOutput { success: true, methodology_id: Some(id), version: Some(1), proposal_id: None, error: None })
        }
        MethodologyCreateInput::Update { methodology_id, .. } => {
            Ok(MethodologyCreateOutput { success: true, methodology_id: Some(methodology_id), version: Some(2), proposal_id: None, error: None })
        }
        MethodologyCreateInput::Delete { methodology_id } => {
            Ok(MethodologyCreateOutput { success: true, methodology_id: Some(methodology_id), version: None, proposal_id: None, error: None })
        }
        MethodologyCreateInput::Propose { methodology_id } => {
            let proposal_id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64;
            Ok(MethodologyCreateOutput { success: true, methodology_id: Some(methodology_id), version: None, proposal_id: Some(proposal_id), error: None })
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() { if args[i] == "--input" && i + 1 < args.len() { input_json = args[i + 1].clone(); } }
    let input: MethodologyCreateInput = serde_json::from_str(&input_json).unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { println!("{}", serde_json::json!({"success": false, "error": e})); std::process::exit(1); }
    }
}
