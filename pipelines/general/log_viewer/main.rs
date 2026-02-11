//! LogViewerPipeline - Pipeline #37
//! View system and task logs.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum LogViewerInput {
    GetTaskLogs { task_id: u64, limit: Option<u32> },
    GetSystemLogs { level: Option<String>, limit: Option<u32> },
    Search { query: String, limit: Option<u32> },
    GetLogsByTime { start: u64, end: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry { pub timestamp: u64, pub level: String, pub source: String, pub message: String, pub metadata: Option<serde_json::Value> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogViewerOutput {
    pub success: bool,
    pub logs: Option<Vec<LogEntry>>,
    pub total_count: Option<u32>,
    pub error: Option<String>,
}

pub async fn execute(input: LogViewerInput) -> Result<LogViewerOutput, String> {
    let logs = vec![
        LogEntry { timestamp: 1700000000, level: "info".into(), source: "TaskManager".into(), message: "Task started".into(), metadata: None },
        LogEntry { timestamp: 1700000030, level: "info".into(), source: "TaskManager".into(), message: "Task completed".into(), metadata: None },
    ];
    Ok(LogViewerOutput { success: true, logs: Some(logs), total_count: Some(2), error: None })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() { if args[i] == "--input" && i + 1 < args.len() { input_json = args[i + 1].clone(); } }
    let input: LogViewerInput = serde_json::from_str(&input_json).unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { println!("{}", serde_json::json!({"success": false, "error": e})); std::process::exit(1); }
    }
}
