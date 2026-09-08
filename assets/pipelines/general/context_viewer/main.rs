//! ContextViewerPipeline — Pipeline #55
//!
//! Provides the Context Viewer UI with graph metadata, task-to-graph mappings,
//! AMT branch data, and cross-modal link lookups.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

pub const PIPELINE_ID: u64 = 55;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ContextViewerInput {
    GetTaskGraphs {
        task_id: u64,
    },
    GetProjectGraphs {
        project_id: u64,
    },
    GetAMTBranches {
        task_id: u64,
    },
    GetCrossModalLinks {
        graph_id: u64,
        node_id: u64,
    },
    GetGraphSummary {
        graph_id: u64,
        modality: String,
    },
    RegisterTaskGraphs {
        task_id: u64,
        graphs: HashMap<String, u64>,
        project_id: Option<u64>,
        workspace_id: Option<u64>,
    },
    RegisterTaskAMT {
        task_id: u64,
        amt_intent: String,
        branches: Vec<AMTBranchRecord>,
    },
    RegisterCrossModalLink {
        source_graph_id: u64,
        source_node_id: u64,
        source_modality: String,
        target_graph_id: u64,
        target_node_id: u64,
        target_modality: String,
        edge_type: String,
        bidirectional: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GraphMeta {
    pub graph_id: u64,
    pub modality: String,
    pub node_count: Option<u32>,
    pub edge_count: Option<u32>,
    pub cross_modal_edge_count: u32,
    pub state: String,
    pub last_modified: u64,
    pub pipeline_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AMTBranchRecord {
    pub branch: String,
    pub parent_intent: String,
    pub methodology_ids: Vec<u64>,
    pub linked_node_ids: Vec<u64>,
    pub linked_modalities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossModalLinkRecord {
    pub entry_id: u64,
    pub source_graph_id: u64,
    pub source_node_id: u64,
    pub source_modality: String,
    pub target_graph_id: u64,
    pub target_node_id: u64,
    pub target_modality: String,
    pub edge_type: String,
    pub bidirectional: bool,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct TaskGraphRecord {
    pub task_id: u64,
    pub project_id: Option<u64>,
    pub workspace_id: Option<u64>,
    pub graphs: HashMap<String, u64>,
    pub amt_intent: Option<String>,
    pub amt_branches: Vec<AMTBranchRecord>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextViewerOutput {
    pub success: bool,
    pub graphs: Option<Vec<GraphMeta>>,
    pub amt_branches: Option<Vec<AMTBranchRecord>>,
    pub cross_modal_links: Option<Vec<CrossModalLinkRecord>>,
    pub graph_summary: Option<GraphMeta>,
    pub registered: Option<bool>,
    pub error: Option<String>,
}

fn storage_dir() -> std::path::PathBuf {
    let base = env::var("OZONE_DATA_PATH").unwrap_or_else(|_| "./data".to_string());
    std::path::PathBuf::from(base).join("context_viewer")
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn load_task_graph_index() -> HashMap<u64, TaskGraphRecord> {
    let path = storage_dir().join("task_graph_index.json");
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(index) = serde_json::from_str(&content) {
                return index;
            }
        }
    }
    HashMap::new()
}

fn save_task_graph_index(index: &HashMap<u64, TaskGraphRecord>) {
    let dir = storage_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("task_graph_index.json");
    if let Ok(content) = serde_json::to_string_pretty(index) {
        let _ = std::fs::write(path, content);
    }
}

fn load_cross_modal_index() -> Vec<CrossModalLinkRecord> {
    let path = storage_dir().join("cross_modal_index.json");
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(index) = serde_json::from_str(&content) {
                return index;
            }
        }
    }
    vec![]
}

fn save_cross_modal_index(index: &[CrossModalLinkRecord]) {
    let dir = storage_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("cross_modal_index.json");
    if let Ok(content) = serde_json::to_string_pretty(index) {
        let _ = std::fs::write(path, content);
    }
}

fn load_project_graph_index() -> HashMap<u64, Vec<u64>> {
    let path = storage_dir().join("project_task_index.json");
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(index) = serde_json::from_str(&content) {
                return index;
            }
        }
    }
    HashMap::new()
}

fn save_project_graph_index(index: &HashMap<u64, Vec<u64>>) {
    let dir = storage_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("project_task_index.json");
    if let Ok(content) = serde_json::to_string_pretty(index) {
        let _ = std::fs::write(path, content);
    }
}

fn modality_to_pipeline_id(modality: &str) -> u64 {
    match modality {
        "text" => 100,
        "code" => 101,
        "image" => 102,
        "audio" => 103,
        "video" => 104,
        "math" => 105,
        "chemistry" => 106,
        "dna" => 107,
        "eeg" => 108,
        "3d" => 109,
        "sound" => 110,
        "biology" => 111,
        "proteomics" => 112,
        "haptic" => 113,
        "thermal" => 114,
        "depth" => 115,
        "imu" => 116,
        "geospatial" => 117,
        "electromagnetic" => 118,
        "bci" => 119,
        "parametric_cad" => 120,
        "kinematics" => 121,
        "control_systems" => 122,
        "network_topology" => 123,
        "radar" => 124,
        "sonar" => 125,
        "hyperspectral" => 126,
        _ => 100,
    }
}

pub async fn execute(input: ContextViewerInput) -> Result<ContextViewerOutput, String> {
    match input {
        ContextViewerInput::GetTaskGraphs { task_id } => {
            let index = load_task_graph_index();
            let xm = load_cross_modal_index();
            let graphs: Vec<GraphMeta> = index
                .get(&task_id)
                .map(|r| {
                    r.graphs
                        .iter()
                        .map(|(modality, &graph_id)| GraphMeta {
                            graph_id,
                            modality: modality.clone(),
                            node_count: None,
                            edge_count: None,
                            cross_modal_edge_count: xm
                                .iter()
                                .filter(|l| {
                                    l.source_graph_id == graph_id || l.target_graph_id == graph_id
                                })
                                .count() as u32,
                            state: "Stable".to_string(),
                            last_modified: r.updated_at,
                            pipeline_id: modality_to_pipeline_id(modality),
                        })
                        .collect()
                })
                .unwrap_or_default();

            Ok(ContextViewerOutput {
                success: true,
                graphs: Some(graphs),
                ..Default::default()
            })
        }

        ContextViewerInput::GetProjectGraphs { project_id } => {
            let project_index = load_project_graph_index();
            let task_graph_index = load_task_graph_index();
            let xm = load_cross_modal_index();

            let task_ids = project_index.get(&project_id).cloned().unwrap_or_default();
            let mut seen_graphs: HashMap<u64, GraphMeta> = HashMap::new();

            for task_id in &task_ids {
                if let Some(record) = task_graph_index.get(task_id) {
                    for (modality, &graph_id) in &record.graphs {
                        seen_graphs.entry(graph_id).or_insert(GraphMeta {
                            graph_id,
                            modality: modality.clone(),
                            node_count: None,
                            edge_count: None,
                            cross_modal_edge_count: 0,
                            state: "Stable".to_string(),
                            last_modified: record.updated_at,
                            pipeline_id: modality_to_pipeline_id(modality),
                        });
                    }
                }
            }

            for graph in seen_graphs.values_mut() {
                graph.cross_modal_edge_count = xm
                    .iter()
                    .filter(|l| {
                        l.source_graph_id == graph.graph_id || l.target_graph_id == graph.graph_id
                    })
                    .count() as u32;
            }

            Ok(ContextViewerOutput {
                success: true,
                graphs: Some(seen_graphs.into_values().collect()),
                ..Default::default()
            })
        }

        ContextViewerInput::GetAMTBranches { task_id } => {
            let index = load_task_graph_index();
            let branches = index
                .get(&task_id)
                .map(|r| r.amt_branches.clone())
                .unwrap_or_default();

            Ok(ContextViewerOutput {
                success: true,
                amt_branches: Some(branches),
                ..Default::default()
            })
        }

        ContextViewerInput::GetCrossModalLinks { graph_id, node_id } => {
            let index = load_cross_modal_index();
            let links: Vec<CrossModalLinkRecord> = index
                .into_iter()
                .filter(|l| {
                    (l.source_graph_id == graph_id && l.source_node_id == node_id)
                        || (l.target_graph_id == graph_id && l.target_node_id == node_id)
                })
                .collect();

            Ok(ContextViewerOutput {
                success: true,
                cross_modal_links: Some(links),
                ..Default::default()
            })
        }

        ContextViewerInput::GetGraphSummary { graph_id, modality } => {
            let xm = load_cross_modal_index();
            let cross_count = xm
                .iter()
                .filter(|l| l.source_graph_id == graph_id || l.target_graph_id == graph_id)
                .count() as u32;

            Ok(ContextViewerOutput {
                success: true,
                graph_summary: Some(GraphMeta {
                    graph_id,
                    modality: modality.clone(),
                    node_count: None,
                    edge_count: None,
                    cross_modal_edge_count: cross_count,
                    state: "Stable".to_string(),
                    last_modified: now(),
                    pipeline_id: modality_to_pipeline_id(&modality),
                }),
                ..Default::default()
            })
        }

        ContextViewerInput::RegisterTaskGraphs {
            task_id,
            graphs,
            project_id,
            workspace_id,
        } => {
            let mut index = load_task_graph_index();
            let record = index.entry(task_id).or_insert(TaskGraphRecord {
                task_id,
                project_id,
                workspace_id,
                graphs: HashMap::new(),
                amt_intent: None,
                amt_branches: vec![],
                created_at: now(),
                updated_at: now(),
            });
            record.graphs = graphs;
            record.project_id = project_id.or(record.project_id);
            record.workspace_id = workspace_id.or(record.workspace_id);
            record.updated_at = now();
            save_task_graph_index(&index);

            if let Some(pid) = project_id {
                let mut proj_index = load_project_graph_index();
                let task_list = proj_index.entry(pid).or_default();
                if !task_list.contains(&task_id) {
                    task_list.push(task_id);
                }
                save_project_graph_index(&proj_index);
            }

            Ok(ContextViewerOutput {
                success: true,
                registered: Some(true),
                ..Default::default()
            })
        }

        ContextViewerInput::RegisterTaskAMT {
            task_id,
            amt_intent,
            branches,
        } => {
            let mut index = load_task_graph_index();
            let record = index.entry(task_id).or_insert(TaskGraphRecord {
                task_id,
                project_id: None,
                workspace_id: None,
                graphs: HashMap::new(),
                amt_intent: None,
                amt_branches: vec![],
                created_at: now(),
                updated_at: now(),
            });
            record.amt_intent = Some(amt_intent);
            record.amt_branches = branches;
            record.updated_at = now();
            save_task_graph_index(&index);

            Ok(ContextViewerOutput {
                success: true,
                registered: Some(true),
                ..Default::default()
            })
        }

        ContextViewerInput::RegisterCrossModalLink {
            source_graph_id,
            source_node_id,
            source_modality,
            target_graph_id,
            target_node_id,
            target_modality,
            edge_type,
            bidirectional,
        } => {
            let mut index = load_cross_modal_index();
            index.push(CrossModalLinkRecord {
                entry_id: now(),
                source_graph_id,
                source_node_id,
                source_modality: source_modality.clone(),
                target_graph_id,
                target_node_id,
                target_modality: target_modality.clone(),
                edge_type: edge_type.clone(),
                bidirectional,
                created_at: now(),
            });
            if bidirectional {
                index.push(CrossModalLinkRecord {
                    entry_id: now() + 1,
                    source_graph_id: target_graph_id,
                    source_node_id: target_node_id,
                    source_modality: target_modality,
                    target_graph_id: source_graph_id,
                    target_node_id: source_node_id,
                    target_modality: source_modality,
                    edge_type,
                    bidirectional: false,
                    created_at: now(),
                });
            }
            save_cross_modal_index(&index);
            Ok(ContextViewerOutput {
                success: true,
                registered: Some(true),
                ..Default::default()
            })
        }
    }
}

/// Executor contract: `--input` carries the full PipelineInput JSON
/// ({"data":{...},"context":{...}}) — unwrap the data envelope. A bare
/// action payload is accepted too; stdin is the legacy standalone path.
fn parse_cli_input<T: serde::de::DeserializeOwned>() -> Result<T, String> {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--input" && i + 1 < args.len() {
            input_json = Some(args[i + 1].clone());
            i += 2;
        } else {
            i += 1;
        }
    }
    let raw = match input_json {
        Some(s) => s,
        None => {
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| e.to_string())?;
            buf
        }
    };
    let v: serde_json::Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let inner = v.get("data").cloned().unwrap_or(v);
    serde_json::from_value(inner).map_err(|e| e.to_string())
}

#[path = "../../shared/ozone_serve.rs"]
mod ozone_serve;

/// Block on the async execute (serve-mode handler runs on std threads).
fn rt_block_on_execute(
    input: ContextViewerInput,
) -> Result<ContextViewerOutput, String> {
    let rt = tokio::runtime::Runtime::new().expect("Tokio runtime");
    rt.block_on(execute(input))
}

fn main() {
    // SERVE MODE — connect-model (see text pipeline reference impl).
    if let Some(opts) = ozone_serve::serve_mode() {
        let handler = std::sync::Arc::new(|action_payload: serde_json::Value| {
            let input: ContextViewerInput = serde_json::from_value(action_payload)
                .unwrap_or(ContextViewerInput::GetTaskGraphs { task_id: 0 });
            match rt_block_on_execute(input) {
                Ok(output) => serde_json::to_value(&output)
                    .unwrap_or(serde_json::json!({"success": false})),
                Err(e) => serde_json::json!({"success": false, "error": e}),
            }
        });
        ozone_serve::serve(opts, PIPELINE_ID, "context_viewer".to_string(), handler);
    }

    let input: ContextViewerInput = match parse_cli_input() {
        Ok(v) => v,
        Err(e) => {
            println!(
                "{}",
                serde_json::json!({"success": false, "error": format!("Parse error: {}", e)})
            );
            std::process::exit(1);
        }
    };
    let rt = tokio::runtime::Runtime::new().expect("Tokio runtime");
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap_or_default()),
        Err(e) => {
            println!("{}", serde_json::json!({"success": false, "error": e}));
            std::process::exit(1);
        }
    }
}
