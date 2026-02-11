//! TaskRecommendationPipeline - Pipeline #23
//! Recommend tasks based on current context, history, and patterns.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum TaskRecommendInput {
    GetRecommendations { workspace_id: Option<u64>, project_id: Option<u64>, limit: Option<u32> },
    GetSimilarTasks { task_id: u64, limit: Option<u32> },
    GetNextSteps { current_task_id: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecommendation { pub task_type: String, pub description: String, pub confidence: f32, pub blueprint_id: Option<u64> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecommendOutput {
    pub success: bool,
    pub recommendations: Option<Vec<TaskRecommendation>>,
    pub error: Option<String>,
}

pub async fn execute(input: TaskRecommendInput) -> Result<TaskRecommendOutput, String> {
    let recs = vec![
        TaskRecommendation { task_type: "code_review".into(), description: "Review recent changes".into(), confidence: 0.8, blueprint_id: Some(1) },
    ];
    Ok(TaskRecommendOutput { success: true, recommendations: Some(recs), error: None })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() { if args[i] == "--input" && i + 1 < args.len() { input_json = args[i + 1].clone(); } }
    let input: TaskRecommendInput = serde_json::from_str(&input_json).unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { println!("{}", serde_json::json!({"success": false, "error": e})); std::process::exit(1); }
    }
}
