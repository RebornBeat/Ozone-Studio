//! GraphVisualizationPipeline - Pipeline #22
//! Visualize ZSEI knowledge graph relationships.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum GraphVisInput {
    /// Get graph data for visualization
    GetGraph { center_id: u64, depth: u16, max_nodes: u32 },
    /// Get graph for project
    GetProjectGraph { project_id: u64 },
    /// Get relationships between containers
    GetRelationships { container_ids: Vec<u64> },
    /// Search and visualize
    SearchGraph { query: String, max_nodes: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub center_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode { pub id: u64, pub label: String, pub node_type: String, pub size: f32 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge { pub source: u64, pub target: u64, pub edge_type: String, pub weight: f32 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphVisOutput {
    pub success: bool,
    pub graph: Option<GraphData>,
    pub error: Option<String>,
}

pub async fn execute(input: GraphVisInput) -> Result<GraphVisOutput, String> {
    let center = match &input {
        GraphVisInput::GetGraph { center_id, .. } => *center_id,
        _ => 0,
    };
    let graph = GraphData {
        nodes: vec![GraphNode { id: center, label: "Center".into(), node_type: "Container".into(), size: 1.0 }],
        edges: vec![],
        center_id: center,
    };
    Ok(GraphVisOutput { success: true, graph: Some(graph), error: None })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() { if args[i] == "--input" && i + 1 < args.len() { input_json = args[i + 1].clone(); } }
    let input: GraphVisInput = serde_json::from_str(&input_json).unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { println!("{}", serde_json::json!({"success": false, "error": e})); std::process::exit(1); }
    }
}
