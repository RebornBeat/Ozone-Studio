//! Index types for pipelines, methodologies, and blueprints

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineIndex {
    pub version: u32,
    pub last_updated: u64,
    pub pipeline_count: usize,
    pub pipelines: Vec<PipelineIndexEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineIndexEntry {
    pub pipeline_id: u64,
    pub name: String,
    pub version: String,
    pub category: String,
    pub modality: Option<String>,
    pub file: String,
    pub use_count: u64,
    pub success_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodologyIndex {
    pub version: u32,
    pub last_updated: u64,
    pub methodology_count: usize,
    pub methodologies: Vec<MethodologyIndexEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodologyIndexEntry {
    pub methodology_id: u64,
    pub name: String,
    pub categories: Vec<String>,
    pub file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintIndex {
    pub version: u32,
    pub last_updated: u64,
    pub blueprint_count: usize,
    pub blueprints: Vec<BlueprintIndexEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintIndexEntry {
    pub blueprint_id: u64,
    pub name: String,
    pub signature_hash: String,
    pub input_types: Vec<String>,
    pub output_types: Vec<String>,
    pub keywords: Vec<String>,
    pub use_count: u64,
    pub success_rate: f32,
    pub file: String,
}
