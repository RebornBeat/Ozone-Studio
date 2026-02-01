//! TaskViewerPipeline - Pipeline #36
//! View detailed task information and history.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum TaskViewerInput {
    GetDetails { task_id: u64 },
    GetInputs { task_id: u64 },
    GetOutputs { task_id: u64 },
    GetTimeline { task_id: u64 },
    Compare { task_ids: Vec<u64> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDetails {
    pub task_id: u64,
    pub pipeline_name: String,
    pub status: String,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub duration_secs: Option<u64>,
    pub input_summary: String,
    pub output_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskViewerOutput {
    pub success: bool,
    pub details: Option<TaskDetails>,
    pub inputs: Option<serde_json::Value>,
    pub outputs: Option<serde_json::Value>,
    pub timeline: Option<Vec<TimelineEvent>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent { pub timestamp: u64, pub event: String, pub details: Option<String> }

pub async fn execute(input: TaskViewerInput) -> Result<TaskViewerOutput, String> {
    match input {
        TaskViewerInput::GetDetails { task_id } => {
            let details = TaskDetails { task_id, pipeline_name: "PromptPipeline".into(), status: "completed".into(), started_at: Some(1700000000), completed_at: Some(1700000030), duration_secs: Some(30), input_summary: "User query".into(), output_summary: Some("Generated response".into()) };
            Ok(TaskViewerOutput { success: true, details: Some(details), inputs: None, outputs: None, timeline: None, error: None })
        }
        TaskViewerInput::GetTimeline { task_id } => {
            let timeline = vec![
                TimelineEvent { timestamp: 1700000000, event: "created".into(), details: None },
                TimelineEvent { timestamp: 1700000001, event: "started".into(), details: None },
                TimelineEvent { timestamp: 1700000030, event: "completed".into(), details: None },
            ];
            Ok(TaskViewerOutput { success: true, details: None, inputs: None, outputs: None, timeline: Some(timeline), error: None })
        }
        _ => Ok(TaskViewerOutput { success: true, details: None, inputs: None, outputs: None, timeline: None, error: None })
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() { if args[i] == "--input" && i + 1 < args.len() { input_json = args[i + 1].clone(); } }
    let input: TaskViewerInput = serde_json::from_str(&input_json).unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { println!("{}", serde_json::json!({"success": false, "error": e})); std::process::exit(1); }
    }
}
