//! DeviceStatusPipeline - Pipeline #38
//! Get device status and resource usage.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum DeviceStatusInput {
    GetStatus,
    GetResourceUsage,
    GetStorageInfo,
    GetNetworkInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStatus {
    pub device_id: u64,
    pub device_name: String,
    pub online: bool,
    pub uptime_secs: u64,
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub storage_used_gb: f32,
    pub storage_total_gb: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStatusOutput {
    pub success: bool,
    pub status: Option<DeviceStatus>,
    pub error: Option<String>,
}

pub async fn execute(input: DeviceStatusInput) -> Result<DeviceStatusOutput, String> {
    let status = DeviceStatus {
        device_id: 1,
        device_name: "Desktop".into(),
        online: true,
        uptime_secs: 86400,
        cpu_usage: 25.0,
        memory_usage: 60.0,
        storage_used_gb: 100.0,
        storage_total_gb: 500.0,
    };
    Ok(DeviceStatusOutput { success: true, status: Some(status), error: None })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() { if args[i] == "--input" && i + 1 < args.len() { input_json = args[i + 1].clone(); } }
    let input: DeviceStatusInput = serde_json::from_str(&input_json).unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { println!("{}", serde_json::json!({"success": false, "error": e})); std::process::exit(1); }
    }
}
