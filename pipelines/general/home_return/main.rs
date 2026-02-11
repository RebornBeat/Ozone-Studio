//! HomeReturnPipeline - Pipeline #35
//! Return to home dashboard from any theme.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum HomeReturnInput {
    Return,
    ReturnWithState { preserve_state: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeReturnOutput {
    pub success: bool,
    pub previous_theme: Option<String>,
    pub error: Option<String>,
}

pub async fn execute(input: HomeReturnInput) -> Result<HomeReturnOutput, String> {
    Ok(HomeReturnOutput { success: true, previous_theme: Some("code_editor".into()), error: None })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() { if args[i] == "--input" && i + 1 < args.len() { input_json = args[i + 1].clone(); } }
    let input: HomeReturnInput = serde_json::from_str(&input_json).unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { println!("{}", serde_json::json!({"success": false, "error": e})); std::process::exit(1); }
    }
}
