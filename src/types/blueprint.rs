//! Blueprint types - Section 14 of the specification

use serde::{Deserialize, Serialize};
use super::{ContainerID, PipelineID, PublicKey, SemVer, Blake3Hash};
use super::container::Modality;
use super::pipeline::ConsensusStatus;

/// Blueprint definition (§14.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blueprint {
    pub blueprint_id: ContainerID,
    pub name: String,
    pub description: String,
    
    // Task signature (for matching)
    pub task_signature: TaskSignature,
    
    // Structure
    pub steps: Vec<BlueprintStep>,
    pub dependencies: Vec<BlueprintDependency>,
    
    // Methodology linkage
    pub methodologies_used: Vec<ContainerID>,
    
    // Context
    pub modalities: Vec<Modality>,
    pub categories: Vec<ContainerID>,
    pub keywords: Vec<String>,
    pub topics: Vec<String>,
    
    // Validation
    pub validated: bool,
    pub validation_runs: u32,
    pub success_rate: f32,
    
    // Metadata
    pub created_at: u64,
    pub created_by: PublicKey,
    pub version: SemVer,
    
    // Distribution
    pub distributed: bool,
    pub consensus_status: ConsensusStatus,
    pub usage_count: u64,
}

/// Task signature for matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSignature {
    pub input_types: Vec<String>,
    pub output_type: String,
    pub constraints: Vec<String>,
    pub hash: Blake3Hash,
}

/// Blueprint step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintStep {
    pub step_id: u64,
    pub order: u32,
    pub action: String,
    pub description: String,
    pub inputs: Vec<StepInput>,
    pub outputs: Vec<StepOutput>,
    pub pipeline_suggestion: Option<PipelineID>,
    pub optional: bool,
    pub conditional: Option<StepCondition>,
}

/// Step input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepInput {
    pub name: String,
    pub source: StepInputSource,
    pub required: bool,
}

/// Step input source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepInputSource {
    TaskInput(String),
    PreviousStep(u64, String),
    Context(String),
    External(String),
}

/// Step output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepOutput {
    pub name: String,
    pub description: String,
    pub stored: bool,
}

/// Step condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepCondition {
    pub condition: String,
    pub skip_if_false: bool,
}

/// Blueprint dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintDependency {
    pub step_id: u64,
    pub depends_on: Vec<u64>,
    pub dependency_type: BlueprintDependencyType,
}

/// Blueprint dependency type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlueprintDependencyType {
    Sequential,
    DataFlow,
    Conditional,
    Optional,
}

/// Blueprint modification (§14.4)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlueprintModification {
    AddStep(BlueprintStep),
    RemoveStep(u64),
    MoveStep { step_id: u64, new_order: u32 },
    EditStep { step_id: u64, updates: StepUpdate },
    InsertBefore { reference_step: u64, new_step: BlueprintStep },
    InsertAfter { reference_step: u64, new_step: BlueprintStep },
    ReplaceStep { step_id: u64, new_step: BlueprintStep },
}

/// Step update for partial modifications
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StepUpdate {
    pub action: Option<String>,
    pub description: Option<String>,
    pub inputs: Option<Vec<StepInput>>,
    pub outputs: Option<Vec<StepOutput>>,
    pub pipeline_suggestion: Option<Option<PipelineID>>,
    pub optional: Option<bool>,
    pub conditional: Option<Option<StepCondition>>,
}
