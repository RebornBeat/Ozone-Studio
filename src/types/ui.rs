//! UI types - Section 5 of the specification

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::{PipelineID, Value};
use super::consensus::SyncStatus;

/// UI State (§5.3)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIState {
    pub theme_area: ThemeAreaState,
    pub meta_portion: MetaPortionState,
    pub connection_bar: ConnectionBarState,
    pub blocking_status: BlockingStatus,
}

/// Theme area state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeAreaState {
    pub active_theme: PipelineID,
    pub theme_state: Value,
    pub can_interrupt: bool,
}

/// Meta portion state - ALWAYS accessible (§5.3)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaPortionState {
    pub prompt_enabled: bool,      // Always true
    pub voice_enabled: bool,
    pub task_viewer_enabled: bool, // Always true
    pub home_button_enabled: bool, // Always true
    
    // Consciousness additions (if enabled)
    #[cfg(feature = "consciousness")]
    pub emotional_display: Option<EmotionalDisplayState>,
    #[cfg(feature = "consciousness")]
    pub relationship_display: Option<RelationshipDisplayState>,
}

impl Default for MetaPortionState {
    fn default() -> Self {
        Self {
            prompt_enabled: true,
            voice_enabled: true,
            task_viewer_enabled: true,
            home_button_enabled: true,
            #[cfg(feature = "consciousness")]
            emotional_display: None,
            #[cfg(feature = "consciousness")]
            relationship_display: None,
        }
    }
}

/// Connection bar state - ALWAYS visible (§5.3)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionBarState {
    pub network_status: NetworkStatus,
    pub peer_count: u32,
    pub upload_speed: f32,
    pub download_speed: f32,
    pub sync_status: SyncStatus,
    pub contribution_data: ContributionData,
    pub zsei_depth: ZSEIDepthData,
}

/// Network status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkStatus {
    Connected,
    Connecting,
    Disconnected,
    Limited,
}

/// Contribution data (§27.2)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContributionData {
    pub methodologies_contributed: u32,
    pub methodologies_adopted: u32,
    pub blueprints_contributed: u32,
    pub blueprints_adopted: u32,
    pub pipelines_contributed: u32,
    pub pipelines_adopted: u32,
    #[cfg(feature = "consciousness")]
    pub experiences_shared: u32,
    pub contribution_score: f32,
    pub contribution_rank: Option<u32>,
}

/// ZSEI depth data (§27.2)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZSEIDepthData {
    pub modality_count: u32,
    pub category_count: u32,
    pub subcategory_count: u32,
    pub methodology_count: u32,
    pub blueprint_count: u32,
    pub pipeline_count: u32,
    pub methodologies_added_today: u32,
    pub blueprints_added_today: u32,
    pub growth_trend: GrowthTrend,
}

/// Growth trend
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GrowthTrend {
    Growing,
    #[default]
    Stable,
    Declining,
}

/// Blocking status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlockingStatus {
    NotBlocked,
    ThemeBlocked {
        blocking_pipeline: PipelineID,
        can_cancel: bool,
    },
    // Meta portion and Connection Bar are NEVER in BlockingStatus
}

/// Theme pipeline (§5.4)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemePipeline {
    pub theme_id: u64,
    pub theme_name: String,
    pub render_pipeline: RenderPipeline,
    pub interaction_handlers: Vec<InteractionHandler>,
    pub sub_pipelines: Vec<PipelineID>,
    pub ui_code: CodeArtifact,
    pub ui_framework: UIFramework,
    pub can_block_theme_area: bool,
    pub provides_home_return: bool, // Must be true
}

/// Render pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderPipeline {
    pub entry_point: String,
    pub dependencies: Vec<Dependency>,
    pub state_schema: Schema,
}

/// Code artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeArtifact {
    pub code: String,
    pub language: String,
    pub hash: super::Blake3Hash,
}

/// Dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version: String,
}

/// Schema (simplified)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Schema {
    pub fields: Vec<SchemaField>,
}

/// Schema field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaField {
    pub name: String,
    pub field_type: String,
    pub required: bool,
}

/// Interaction handler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionHandler {
    pub event_type: EventType,
    pub target_pipeline: PipelineID,
    pub input_mapping: HashMap<String, String>,
}

/// Event types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    Click,
    Input,
    Submit,
    KeyPress,
    Voice,
    Scroll,
    Focus,
    Blur,
    Custom(String),
}

/// UI frameworks
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UIFramework {
    Electron,
    Native,
    Web,
    Custom(String),
}

/// UI modification request (§5.7)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIModificationRequest {
    pub requesting_pipeline: PipelineID,
    pub parent_ui: PipelineID,
    pub modification_type: UIModificationType,
    pub constraints: UIConstraints,
}

/// UI modification types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UIModificationType {
    AddComponent(ComponentSpec),
    RemoveComponent(ComponentID),
    UpdateState(StateUpdate),
    InsertBefore(ComponentID, ComponentSpec),
    InsertAfter(ComponentID, ComponentSpec),
    Replace(ComponentID, ComponentSpec),
    ShowModal(ModalSpec),
    HideModal(ModalID),
}

pub type ComponentID = String;
pub type ModalID = String;

/// Component specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentSpec {
    pub component_type: String,
    pub props: HashMap<String, Value>,
    pub children: Vec<ComponentSpec>,
}

/// State update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateUpdate {
    pub path: String,
    pub value: Value,
}

/// Modal specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalSpec {
    pub modal_id: ModalID,
    pub title: String,
    pub content: ComponentSpec,
    pub closable: bool,
}

/// UI constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIConstraints {
    pub max_width_percent: Option<f32>,
    pub max_height_percent: Option<f32>,
    pub allowed_areas: Vec<UIArea>,
    pub isolation_required: bool,
    pub blocking_allowed: bool, // Cannot block Meta Portion or Connection Bar
}

/// UI areas
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UIArea {
    ThemeMain,
    ThemeSidebar,
    ThemeModal,
    // Meta Portion and Connection Bar areas are NOT modifiable by pipelines
}

/// Task recommendation (§21.3)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecommendation {
    pub recommendation_id: u64,
    pub user_id: u64,
    pub suggested_action: String,
    pub suggested_pipeline: Option<PipelineID>,
    pub observation_source: u64,
    pub reasoning: String,
    pub confidence: f32,
    pub status: RecommendationStatus,
    pub presented_at: Option<u64>,
    pub response: Option<RecommendationResponse>,
}

/// Recommendation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecommendationStatus {
    Pending,
    Presented,
    Accepted,
    Declined,
    Ignored,
    Expired,
}

/// Recommendation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationResponse {
    Accept,
    Decline { reason: Option<String> },
    Later,
    Never,
}

/// Task observation (§21.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskObservation {
    pub observation_type: ObservationType,
    pub timestamp: u64,
    pub context: ObservationContext,
    pub data: Value,
}

/// Observation types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservationType {
    DataIngestion,
    PatternDetected,
    FrequentTask,
    RelatedContent,
    ExternalTrigger,
    PackageUpdate,
    URLChange,
}

/// Observation context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationContext {
    pub user_id: u64,
    pub workspace_id: Option<u64>,
    pub project_id: Option<u64>,
    pub active_task: Option<u64>,
}

/// Emotional display state (consciousness)
#[cfg(feature = "consciousness")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalDisplayState {
    pub current_emotion: String,
    pub intensity: f32,
    pub valence_indicator: ValenceIndicator,
    pub secondary_emotions: Vec<(String, f32)>,
    pub emotional_trend: EmotionalTrend,
    pub detail_level: DisplayDetailLevel,
}

#[cfg(feature = "consciousness")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValenceIndicator {
    Positive,
    Neutral,
    Negative,
    Mixed,
}

#[cfg(feature = "consciousness")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmotionalTrend {
    Rising,
    Falling,
    Stable,
    Volatile,
}

#[cfg(feature = "consciousness")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplayDetailLevel {
    Minimal,
    Standard,
    Detailed,
}

/// Relationship display state (consciousness)
#[cfg(feature = "consciousness")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipDisplayState {
    pub user_id: u64,
    pub trust_level: f32,
    pub relationship_stage: String,
    pub recent_interaction_quality: f32,
}

// ============================================================================
// PEER INFO TYPES (§27.2)
// ============================================================================

/// Detailed peer information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PeerInfo {
    pub connected_peers: u32,
    pub peer_list: Vec<PeerSummary>,
    pub upload_speed_kbps: f32,
    pub download_speed_kbps: f32,
}

/// Summary of a connected peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerSummary {
    pub peer_id: String,
    pub location: Option<String>,
    pub connection_quality: f32,
}

/// Full connection display (§27.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionDisplay {
    pub network_status: NetworkStatus,
    pub peer_info: PeerInfo,
    pub sync_status: super::consensus::SyncStatusData,
    pub contribution_data: ContributionData,
    pub zsei_depth: ZSEIDepthData,
}
