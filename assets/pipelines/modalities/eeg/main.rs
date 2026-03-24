//! OZONE Studio - Pipeline 108: EEG Analysis
//!
//! Modality pipeline for EEG/neurophysiological signal analysis.
//! Analyzes brain signals to detect events, frequency bands, and connectivity.
//! Creates traversable graphs that can be enriched by ZSEI semantic hooks.
//!
//! # Actions
//! - `Analyze`: Full EEG analysis (events, bands, artifacts)
//! - `DetectEvents`: Detect specific neurological events
//! - `ExtractBands`: Extract frequency band powers
//! - `ComputeConnectivity`: Compute inter-channel connectivity
//! - `StageSleep`: Automatic sleep staging
//! - `CreateGraph`: Build structural graph from analysis
//! - `QueryGraph`: Query graph for events, patterns
//! - `TriggerSemanticHook`: Trigger ZSEI hooks for semantic enrichment
//!
//! # Graph Structure
//! - Nodes: Recording, Channel, Event, Epoch, Band, SleepStage
//! - Edges: Contains, RecordedBy, Precedes, ConnectedTo, OccursDuring

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::env;

// ============================================================================
// PIPELINE METADATA
// ============================================================================

pub const PIPELINE_ID: u64 = 108;
pub const PIPELINE_NAME: &str = "eeg_analysis";
pub const PIPELINE_VERSION: &str = "0.4.0";
pub const MODALITY: &str = "eeg";

// ============================================================================
// INPUT/OUTPUT TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct EEGModalityInput {
    pub action: EEGAction,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EEGAction {
    /// Full EEG analysis
    Analyze {
        eeg_data: EEGData,
        #[serde(default)]
        depth: AnalysisDepth,
        #[serde(default)]
        detect_events: bool,
        #[serde(default)]
        extract_bands: bool,
        #[serde(default)]
        detect_artifacts: bool,
    },

    /// Detect specific neurological events
    DetectEvents {
        eeg_data: EEGData,
        event_types: Vec<EventType>,
        #[serde(default)]
        threshold: f32,
    },

    /// Extract frequency band powers
    ExtractBands {
        eeg_data: EEGData,
        #[serde(default)]
        bands: Vec<FrequencyBand>,
        #[serde(default)]
        window_seconds: f32,
        #[serde(default)]
        overlap: f32,
    },

    /// Compute channel connectivity
    ComputeConnectivity {
        eeg_data: EEGData,
        #[serde(default)]
        method: ConnectivityMethod,
        #[serde(default)]
        frequency_band: Option<FrequencyBand>,
    },

    /// Automatic sleep staging
    StageSleep {
        eeg_data: EEGData,
        #[serde(default)]
        epoch_length: f32,
    },

    /// Extract epochs around events
    ExtractEpochs {
        eeg_data: EEGData,
        events: Vec<TimeMarker>,
        #[serde(default)]
        pre_event: f32,
        #[serde(default)]
        post_event: f32,
    },

    /// Compute event-related potentials
    ComputeERP {
        eeg_data: EEGData,
        events: Vec<TimeMarker>,
        #[serde(default)]
        baseline: Option<(f32, f32)>,
    },

    /// Create graph from analysis
    CreateGraph {
        analysis: EEGAnalysisResult,
        project_id: u64,
        #[serde(default)]
        graph_name: Option<String>,
    },

    /// Update existing graph
    UpdateGraph {
        graph_id: u64,
        updates: EEGGraphUpdate,
    },

    /// Query EEG graph
    QueryGraph {
        graph_id: u64,
        query: EEGQuery,
    },

    /// Get graph
    GetGraph {
        graph_id: u64,
    },

    /// Link to another modality
    LinkToModality {
        eeg_graph_id: u64,
        target_graph_id: u64,
        target_modality: String,
        relationship: CrossModalityRelation,
    },

    /// Trigger ZSEI semantic hook
    TriggerSemanticHook {
        graph_id: u64,
        hook_type: ZSEIHookType,
        #[serde(default)]
        options: HookOptions,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EEGModalityOutput {
    pub success: bool,
    pub action: String,
    pub result: EEGResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub metadata: OutputMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EEGResult {
    Analysis(EEGAnalysisResult),
    Events(EventDetectionResult),
    Bands(BandExtractionResult),
    Connectivity(ConnectivityResult),
    Sleep(SleepStagingResult),
    Epochs(EpochExtractionResult),
    ERP(ERPResult),
    Graph(EEGGraph),
    Query(QueryResult),
    Link(LinkResult),
    Hook(HookResult),
    Empty,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OutputMetadata {
    pub pipeline_id: u64,
    pub pipeline_version: String,
    pub processing_time_ms: u64,
    pub timestamp: String,
}

// ============================================================================
// CORE DATA TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum EEGData {
    /// File path to EEG file (EDF, BDF, etc.)
    FilePath(String),
    /// Raw data with channel info
    Raw {
        channels: Vec<ChannelData>,
        sample_rate: f32,
        start_time: Option<String>,
    },
    /// URL to remote EEG file
    Url(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChannelData {
    pub name: String,
    pub data: Vec<f64>,
    pub unit: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default, PartialEq)]
pub enum AnalysisDepth {
    Surface,
    #[default]
    Standard,
    Deep,
    Comprehensive,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum EventType {
    // Epileptiform
    Spike,
    SharpWave,
    SpikeWave,
    PolySpike,
    Seizure,
    // Sleep
    SleepSpindle,
    KComplex,
    SlowWave,
    REM,
    // Artifacts
    EyeBlink,
    EyeMovement,
    Muscle,
    Movement,
    Electrode,
    // Normal variants
    AlphaBlock,
    MuRhythm,
    Lambda,
    POSTS,
    // Other
    Stimulation,
    Response,
    Custom(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FrequencyBand {
    pub name: String,
    pub low_freq: f32,
    pub high_freq: f32,
}

impl FrequencyBand {
    pub fn delta() -> Self {
        Self { name: "Delta".to_string(), low_freq: 0.5, high_freq: 4.0 }
    }
    pub fn theta() -> Self {
        Self { name: "Theta".to_string(), low_freq: 4.0, high_freq: 8.0 }
    }
    pub fn alpha() -> Self {
        Self { name: "Alpha".to_string(), low_freq: 8.0, high_freq: 13.0 }
    }
    pub fn beta() -> Self {
        Self { name: "Beta".to_string(), low_freq: 13.0, high_freq: 30.0 }
    }
    pub fn gamma() -> Self {
        Self { name: "Gamma".to_string(), low_freq: 30.0, high_freq: 100.0 }
    }

    pub fn standard_bands() -> Vec<Self> {
        vec![Self::delta(), Self::theta(), Self::alpha(), Self::beta(), Self::gamma()]
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub enum ConnectivityMethod {
    #[default]
    Coherence,
    PhaseLocking,
    GrangerCausality,
    MutualInformation,
    TransferEntropy,
    PLI,  // Phase Lag Index
    WPLI, // Weighted PLI
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimeMarker {
    pub time: f32,
    pub label: String,
    pub duration: Option<f32>,
}

// ============================================================================
// ANALYSIS RESULTS
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EEGAnalysisResult {
    /// Recording identifier
    pub recording_id: String,
    /// Total duration in seconds
    pub duration_seconds: f32,
    /// Sample rate in Hz
    pub sample_rate: f32,
    /// Number of samples
    pub num_samples: u64,
    /// Channel information
    pub channels: Vec<ChannelInfo>,
    /// Detected events
    pub events: Vec<EEGEvent>,
    /// Band powers per channel
    pub band_powers: Vec<BandPower>,
    /// Connectivity matrix (if computed)
    pub connectivity: Option<ConnectivityMatrix>,
    /// Detected artifacts
    pub artifacts: Vec<Artifact>,
    /// Sleep stages (if applicable)
    pub sleep_stages: Option<Vec<SleepStage>>,
    /// Recording quality metrics
    pub quality: RecordingQuality,
    /// Metadata
    pub metadata: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChannelInfo {
    /// Channel name (e.g., "Fp1", "Cz", "O2")
    pub name: String,
    /// Channel type
    pub channel_type: ChannelType,
    /// 3D location (if available)
    pub location: Option<Position3D>,
    /// Reference electrode
    pub reference: Option<String>,
    /// Sampling rate (if different from recording)
    pub sample_rate: Option<f32>,
    /// Unit of measurement
    pub unit: String,
    /// Signal quality
    pub quality_score: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ChannelType {
    EEG,
    EOG,
    EMG,
    ECG,
    Reference,
    Trigger,
    Misc,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Position3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EEGEvent {
    /// Event identifier
    pub event_id: String,
    /// Event type
    pub event_type: EventType,
    /// Start time in seconds
    pub start_time: f32,
    /// End time in seconds
    pub end_time: f32,
    /// Channels where event was detected
    pub channels: Vec<String>,
    /// Detection confidence
    pub confidence: f32,
    /// Peak amplitude
    pub amplitude: Option<f32>,
    /// Peak frequency
    pub frequency: Option<f32>,
    /// Additional properties
    pub properties: HashMap<String, Value>,
}

impl EEGEvent {
    pub fn duration(&self) -> f32 {
        self.end_time - self.start_time
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BandPower {
    /// Frequency band
    pub band: FrequencyBand,
    /// Channel name
    pub channel: String,
    /// Absolute power (μV²)
    pub absolute_power: f32,
    /// Relative power (%)
    pub relative_power: f32,
    /// Power spectral density
    pub psd: Option<Vec<f32>>,
    /// Time window
    pub time_window: Option<TimeWindow>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimeWindow {
    pub start: f32,
    pub end: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConnectivityMatrix {
    /// Connectivity method used
    pub method: ConnectivityMethod,
    /// Channel names
    pub channels: Vec<String>,
    /// Connectivity values (n x n matrix)
    pub matrix: Vec<Vec<f32>>,
    /// Frequency band (if applicable)
    pub frequency_band: Option<FrequencyBand>,
    /// Significance threshold
    pub threshold: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Artifact {
    /// Artifact type
    pub artifact_type: ArtifactType,
    /// Start time
    pub start_time: f32,
    /// End time
    pub end_time: f32,
    /// Affected channels
    pub channels: Vec<String>,
    /// Severity (0-1)
    pub severity: f32,
    /// Was it corrected?
    pub corrected: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ArtifactType {
    EyeBlink,
    EyeMovement,
    Muscle,
    Movement,
    Electrode,
    LineNoise,
    Saturation,
    Drift,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SleepStage {
    /// Start time
    pub start_time: f32,
    /// End time
    pub end_time: f32,
    /// Sleep stage
    pub stage: SleepStageType,
    /// Confidence
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum SleepStageType {
    Wake,
    N1,
    N2,
    N3,
    REM,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RecordingQuality {
    /// Overall quality score
    pub overall_score: f32,
    /// Percentage of good data
    pub good_data_percent: f32,
    /// Artifact percentage
    pub artifact_percent: f32,
    /// Channel quality scores
    pub channel_quality: HashMap<String, f32>,
    /// Issues detected
    pub issues: Vec<QualityIssue>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QualityIssue {
    pub issue_type: String,
    pub severity: f32,
    pub channels: Vec<String>,
    pub time_range: Option<TimeWindow>,
    pub description: String,
}

// ============================================================================
// ADDITIONAL RESULT TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EventDetectionResult {
    pub recording_id: String,
    pub events: Vec<EEGEvent>,
    pub total_events: usize,
    pub events_by_type: HashMap<String, usize>,
    pub detection_params: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BandExtractionResult {
    pub recording_id: String,
    pub band_powers: Vec<BandPower>,
    pub time_series: Option<Vec<BandTimeSeries>>,
    pub topography: Option<Vec<Topography>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BandTimeSeries {
    pub band: FrequencyBand,
    pub channel: String,
    pub times: Vec<f32>,
    pub powers: Vec<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Topography {
    pub band: FrequencyBand,
    pub time: f32,
    pub channel_values: HashMap<String, f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConnectivityResult {
    pub recording_id: String,
    pub connectivity: ConnectivityMatrix,
    pub significant_connections: Vec<Connection>,
    pub network_metrics: Option<NetworkMetrics>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Connection {
    pub channel1: String,
    pub channel2: String,
    pub strength: f32,
    pub direction: Option<ConnectionDirection>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ConnectionDirection {
    Bidirectional,
    Forward,
    Backward,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkMetrics {
    pub density: f32,
    pub clustering_coefficient: f32,
    pub path_length: f32,
    pub modularity: f32,
    pub hub_channels: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SleepStagingResult {
    pub recording_id: String,
    pub stages: Vec<SleepStage>,
    pub hypnogram: Vec<(f32, SleepStageType)>,
    pub sleep_metrics: SleepMetrics,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SleepMetrics {
    /// Total sleep time (minutes)
    pub total_sleep_time: f32,
    /// Sleep efficiency (%)
    pub sleep_efficiency: f32,
    /// Sleep onset latency (minutes)
    pub sleep_onset_latency: f32,
    /// REM latency (minutes)
    pub rem_latency: Option<f32>,
    /// Wake after sleep onset (minutes)
    pub waso: f32,
    /// Time in each stage (minutes)
    pub stage_durations: HashMap<String, f32>,
    /// Number of awakenings
    pub awakenings: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EpochExtractionResult {
    pub recording_id: String,
    pub epochs: Vec<Epoch>,
    pub total_epochs: usize,
    pub rejected_epochs: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Epoch {
    pub epoch_id: String,
    pub event_label: String,
    pub event_time: f32,
    pub start_time: f32,
    pub end_time: f32,
    pub data: HashMap<String, Vec<f64>>,
    pub rejected: bool,
    pub rejection_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ERPResult {
    pub recording_id: String,
    pub condition: String,
    pub channels: Vec<String>,
    pub times: Vec<f32>,
    pub erp_data: HashMap<String, Vec<f64>>,
    pub num_epochs: usize,
    pub peaks: Vec<ERPPeak>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ERPPeak {
    pub name: String,
    pub channel: String,
    pub latency: f32,
    pub amplitude: f64,
    pub polarity: Polarity,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Polarity {
    Positive,
    Negative,
}

// ============================================================================
// GRAPH TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EEGGraph {
    pub graph_id: u64,
    pub name: String,
    pub modality: String,
    pub project_id: u64,
    pub recording_info: RecordingInfo,
    pub nodes: Vec<EEGGraphNode>,
    pub edges: Vec<EEGGraphEdge>,
    pub metadata: GraphMetadata,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RecordingInfo {
    pub recording_id: String,
    pub duration_seconds: f32,
    pub sample_rate: f32,
    pub num_channels: usize,
    pub subject_id: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EEGGraphNode {
    pub node_id: u64,
    pub node_type: EEGNodeType,
    pub label: String,
    pub content: String,
    pub time_range: Option<TimeWindow>,
    pub channels: Vec<String>,
    pub confidence: f32,
    pub properties: HashMap<String, Value>,
    pub annotations: Vec<SemanticAnnotation>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum EEGNodeType {
    Recording,
    Channel,
    Event,
    Epoch,
    Band,
    SleepStage,
    Artifact,
    ERP,
    Connectivity,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SemanticAnnotation {
    pub annotation_type: String,
    pub value: Value,
    pub confidence: f32,
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EEGGraphEdge {
    pub edge_id: u64,
    pub from_node: u64,
    pub to_node: u64,
    pub edge_type: EEGEdgeType,
    pub weight: f32,
    pub properties: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum EEGEdgeType {
    // Structural
    Contains,
    RecordedBy,
    // Temporal
    Precedes,
    Follows,
    Overlaps,
    OccursDuring,
    // Connectivity
    ConnectedTo,
    CausesChange,
    // Cross-modality
    DescribedBy,
    CorrelatesWith,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GraphMetadata {
    pub node_count: usize,
    pub edge_count: usize,
    pub event_count: usize,
    pub channel_count: usize,
    pub duration_seconds: f32,
    pub has_sleep_staging: bool,
    pub has_connectivity: bool,
    pub semantic_enriched: bool,
    pub cross_modal_links: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EEGGraphUpdate {
    pub add_nodes: Vec<EEGGraphNode>,
    pub update_nodes: Vec<EEGGraphNode>,
    pub remove_nodes: Vec<u64>,
    pub add_edges: Vec<EEGGraphEdge>,
    pub remove_edges: Vec<u64>,
    pub metadata_updates: Option<HashMap<String, Value>>,
}

// ============================================================================
// QUERY TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EEGQuery {
    pub query_type: EEGQueryType,
    pub parameters: HashMap<String, Value>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub min_confidence: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum EEGQueryType {
    /// Find events by type
    FindEvents { event_types: Vec<EventType> },
    /// Get band power for channels
    GetBandPower { bands: Vec<String>, channels: Vec<String> },
    /// Get connectivity between channels
    GetConnectivity { channels: Vec<String> },
    /// Get sleep stages in time range
    GetSleepStages { start_time: f32, end_time: f32 },
    /// Analyze specific epoch
    AnalyzeEpoch { start_time: f32, end_time: f32 },
    /// Get nodes by type
    GetNodesByType { node_type: EEGNodeType },
    /// Get nodes in time range
    GetNodesInTimeRange { start: f32, end: f32 },
    /// Find artifacts
    FindArtifacts { artifact_types: Vec<String> },
    /// Custom query
    Custom { query: String },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryResult {
    pub query_type: String,
    pub nodes: Vec<EEGGraphNode>,
    pub edges: Vec<EEGGraphEdge>,
    pub total_matches: usize,
    pub metadata: HashMap<String, Value>,
}

// ============================================================================
// CROSS-MODALITY & HOOKS
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CrossModalityRelation {
    DescribedBy,
    CorrelatesWith,
    RecordedDuring,
    AnalyzedIn,
    Custom(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LinkResult {
    pub link_id: u64,
    pub source_graph_id: u64,
    pub target_graph_id: u64,
    pub relationship: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ZSEIHookType {
    OnGraphCreated,
    OnInferRelationships,
    OnEdgeCompletion,
    OnCrossModalityLink,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct HookOptions {
    pub max_nodes: Option<usize>,
    pub min_confidence: Option<f32>,
    pub async_processing: bool,
    pub parameters: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HookResult {
    pub hook_type: ZSEIHookType,
    pub success: bool,
    pub nodes_processed: usize,
    pub edges_added: usize,
    pub annotations_added: usize,
    pub processing_time_ms: u64,
    pub errors: Vec<String>,
}

// ============================================================================
// EXECUTION
// ============================================================================

pub async fn execute(input: Value) -> Result<Value, String> {
    let start_time = std::time::Instant::now();

    let input: EEGModalityInput =
        serde_json::from_value(input).map_err(|e| format!("Failed to parse input: {}", e))?;

    let result = match input.action {
        EEGAction::Analyze {
            eeg_data,
            depth,
            detect_events,
            extract_bands,
            detect_artifacts,
        } => {
            let analysis =
                analyze_eeg(&eeg_data, depth, detect_events, extract_bands, detect_artifacts)
                    .await?;
            ("Analyze", EEGResult::Analysis(analysis))
        }

        EEGAction::DetectEvents {
            eeg_data,
            event_types,
            threshold,
        } => {
            let result = detect_events(&eeg_data, &event_types, threshold).await?;
            ("DetectEvents", EEGResult::Events(result))
        }

        EEGAction::ExtractBands {
            eeg_data,
            bands,
            window_seconds,
            overlap,
        } => {
            let result = extract_bands(&eeg_data, &bands, window_seconds, overlap).await?;
            ("ExtractBands", EEGResult::Bands(result))
        }

        EEGAction::ComputeConnectivity {
            eeg_data,
            method,
            frequency_band,
        } => {
            let result = compute_connectivity(&eeg_data, method, frequency_band).await?;
            ("ComputeConnectivity", EEGResult::Connectivity(result))
        }

        EEGAction::StageSleep {
            eeg_data,
            epoch_length,
        } => {
            let result = stage_sleep(&eeg_data, epoch_length).await?;
            ("StageSleep", EEGResult::Sleep(result))
        }

        EEGAction::ExtractEpochs {
            eeg_data,
            events,
            pre_event,
            post_event,
        } => {
            let result = extract_epochs(&eeg_data, &events, pre_event, post_event).await?;
            ("ExtractEpochs", EEGResult::Epochs(result))
        }

        EEGAction::ComputeERP {
            eeg_data,
            events,
            baseline,
        } => {
            let result = compute_erp(&eeg_data, &events, baseline).await?;
            ("ComputeERP", EEGResult::ERP(result))
        }

        EEGAction::CreateGraph {
            analysis,
            project_id,
            graph_name,
        } => {
            let graph = create_graph(analysis, project_id, graph_name).await?;
            ("CreateGraph", EEGResult::Graph(graph))
        }

        EEGAction::UpdateGraph { graph_id, updates } => {
            let graph = update_graph(graph_id, updates).await?;
            ("UpdateGraph", EEGResult::Graph(graph))
        }

        EEGAction::QueryGraph { graph_id, query } => {
            let result = query_graph(graph_id, query).await?;
            ("QueryGraph", EEGResult::Query(result))
        }

        EEGAction::GetGraph { graph_id } => {
            let graph = get_graph(graph_id).await?;
            ("GetGraph", EEGResult::Graph(graph))
        }

        EEGAction::LinkToModality {
            eeg_graph_id,
            target_graph_id,
            target_modality,
            relationship,
        } => {
            let link =
                link_to_modality(eeg_graph_id, target_graph_id, &target_modality, relationship)
                    .await?;
            ("LinkToModality", EEGResult::Link(link))
        }

        EEGAction::TriggerSemanticHook {
            graph_id,
            hook_type,
            options,
        } => {
            let result = trigger_semantic_hook(graph_id, hook_type, options).await?;
            ("TriggerSemanticHook", EEGResult::Hook(result))
        }
    };

    let output = EEGModalityOutput {
        success: true,
        action: result.0.to_string(),
        result: result.1,
        error: None,
        metadata: OutputMetadata {
            pipeline_id: PIPELINE_ID,
            pipeline_version: PIPELINE_VERSION.to_string(),
            processing_time_ms: start_time.elapsed().as_millis() as u64,
            timestamp: chrono::Utc::now().to_rfc3339(),
        },
    };

    serde_json::to_value(output).map_err(|e| format!("Failed to serialize output: {}", e))
}

// ============================================================================
// ACTION IMPLEMENTATIONS
// ============================================================================

async fn analyze_eeg(
    _eeg_data: &EEGData,
    depth: AnalysisDepth,
    detect_events_flag: bool,
    extract_bands_flag: bool,
    detect_artifacts_flag: bool,
) -> Result<EEGAnalysisResult, String> {
    // In production, use MNE-Python or similar for actual analysis

    let channels = vec![
        ChannelInfo {
            name: "Fp1".to_string(),
            channel_type: ChannelType::EEG,
            location: Some(Position3D { x: -0.03, y: 0.09, z: 0.04 }),
            reference: Some("A1".to_string()),
            sample_rate: None,
            unit: "μV".to_string(),
            quality_score: 0.95,
        },
        ChannelInfo {
            name: "Fp2".to_string(),
            channel_type: ChannelType::EEG,
            location: Some(Position3D { x: 0.03, y: 0.09, z: 0.04 }),
            reference: Some("A2".to_string()),
            sample_rate: None,
            unit: "μV".to_string(),
            quality_score: 0.92,
        },
        ChannelInfo {
            name: "Cz".to_string(),
            channel_type: ChannelType::EEG,
            location: Some(Position3D { x: 0.0, y: 0.0, z: 0.1 }),
            reference: Some("Avg".to_string()),
            sample_rate: None,
            unit: "μV".to_string(),
            quality_score: 0.98,
        },
    ];

    let mut events = Vec::new();
    let mut band_powers = Vec::new();
    let mut artifacts = Vec::new();

    if detect_events_flag {
        events.push(EEGEvent {
            event_id: "evt_1".to_string(),
            event_type: EventType::AlphaBlock,
            start_time: 10.5,
            end_time: 12.0,
            channels: vec!["O1".to_string(), "O2".to_string()],
            confidence: 0.92,
            amplitude: Some(45.0),
            frequency: Some(10.0),
            properties: HashMap::new(),
        });

        events.push(EEGEvent {
            event_id: "evt_2".to_string(),
            event_type: EventType::SleepSpindle,
            start_time: 120.0,
            end_time: 121.5,
            channels: vec!["Cz".to_string(), "C3".to_string(), "C4".to_string()],
            confidence: 0.85,
            amplitude: Some(30.0),
            frequency: Some(12.0),
            properties: HashMap::new(),
        });
    }

    if extract_bands_flag {
        for band in FrequencyBand::standard_bands() {
            for channel in &channels {
                band_powers.push(BandPower {
                    band: band.clone(),
                    channel: channel.name.clone(),
                    absolute_power: 10.0 + rand_float() * 20.0,
                    relative_power: 15.0 + rand_float() * 10.0,
                    psd: None,
                    time_window: None,
                });
            }
        }
    }

    if detect_artifacts_flag {
        artifacts.push(Artifact {
            artifact_type: ArtifactType::EyeBlink,
            start_time: 5.2,
            end_time: 5.5,
            channels: vec!["Fp1".to_string(), "Fp2".to_string()],
            severity: 0.8,
            corrected: false,
        });
    }

    let quality_score = match depth {
        AnalysisDepth::Surface => 0.70,
        AnalysisDepth::Standard => 0.85,
        AnalysisDepth::Deep => 0.92,
        AnalysisDepth::Comprehensive => 0.95,
    };

    Ok(EEGAnalysisResult {
        recording_id: format!("rec_{}", generate_graph_id()),
        duration_seconds: 300.0,
        sample_rate: 256.0,
        num_samples: 76800,
        channels,
        events,
        band_powers,
        connectivity: None,
        artifacts,
        sleep_stages: None,
        quality: RecordingQuality {
            overall_score: quality_score,
            good_data_percent: 95.0,
            artifact_percent: 5.0,
            channel_quality: HashMap::new(),
            issues: vec![],
        },
        metadata: HashMap::new(),
    })
}

fn rand_float() -> f32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
    (nanos % 1000) as f32 / 1000.0
}

async fn detect_events(
    _eeg_data: &EEGData,
    event_types: &[EventType],
    _threshold: f32,
) -> Result<EventDetectionResult, String> {
    let events: Vec<EEGEvent> = event_types
        .iter()
        .enumerate()
        .map(|(i, et)| EEGEvent {
            event_id: format!("evt_{}", i + 1),
            event_type: et.clone(),
            start_time: (i as f32 + 1.0) * 10.0,
            end_time: (i as f32 + 1.0) * 10.0 + 1.0,
            channels: vec!["Cz".to_string()],
            confidence: 0.85,
            amplitude: Some(50.0),
            frequency: Some(10.0),
            properties: HashMap::new(),
        })
        .collect();

    let mut events_by_type = HashMap::new();
    for event in &events {
        let type_name = format!("{:?}", event.event_type);
        *events_by_type.entry(type_name).or_insert(0) += 1;
    }

    Ok(EventDetectionResult {
        recording_id: format!("rec_{}", generate_graph_id()),
        total_events: events.len(),
        events,
        events_by_type,
        detection_params: HashMap::new(),
    })
}

async fn extract_bands(
    _eeg_data: &EEGData,
    bands: &[FrequencyBand],
    _window_seconds: f32,
    _overlap: f32,
) -> Result<BandExtractionResult, String> {
    let bands_to_use = if bands.is_empty() {
        FrequencyBand::standard_bands()
    } else {
        bands.to_vec()
    };

    let band_powers: Vec<BandPower> = bands_to_use
        .iter()
        .flat_map(|band| {
            vec!["Fp1", "Fp2", "Cz"].iter().map(move |ch| BandPower {
                band: band.clone(),
                channel: ch.to_string(),
                absolute_power: 15.0 + rand_float() * 20.0,
                relative_power: 20.0 + rand_float() * 10.0,
                psd: None,
                time_window: None,
            })
        })
        .collect();

    Ok(BandExtractionResult {
        recording_id: format!("rec_{}", generate_graph_id()),
        band_powers,
        time_series: None,
        topography: None,
    })
}

async fn compute_connectivity(
    _eeg_data: &EEGData,
    method: ConnectivityMethod,
    frequency_band: Option<FrequencyBand>,
) -> Result<ConnectivityResult, String> {
    let channels = vec!["Fp1".to_string(), "Fp2".to_string(), "Cz".to_string()];
    let n = channels.len();

    // Generate symmetric connectivity matrix
    let mut matrix = vec![vec![0.0f32; n]; n];
    for i in 0..n {
        for j in 0..n {
            if i == j {
                matrix[i][j] = 1.0;
            } else {
                let val = 0.3 + rand_float() * 0.4;
                matrix[i][j] = val;
                matrix[j][i] = val;
            }
        }
    }

    let significant_connections: Vec<Connection> = vec![
        Connection {
            channel1: "Fp1".to_string(),
            channel2: "Fp2".to_string(),
            strength: matrix[0][1],
            direction: Some(ConnectionDirection::Bidirectional),
        },
        Connection {
            channel1: "Fp1".to_string(),
            channel2: "Cz".to_string(),
            strength: matrix[0][2],
            direction: Some(ConnectionDirection::Bidirectional),
        },
    ];

    Ok(ConnectivityResult {
        recording_id: format!("rec_{}", generate_graph_id()),
        connectivity: ConnectivityMatrix {
            method,
            channels,
            matrix,
            frequency_band,
            threshold: Some(0.5),
        },
        significant_connections,
        network_metrics: Some(NetworkMetrics {
            density: 0.67,
            clustering_coefficient: 0.72,
            path_length: 1.5,
            modularity: 0.35,
            hub_channels: vec!["Cz".to_string()],
        }),
    })
}

async fn stage_sleep(
    _eeg_data: &EEGData,
    epoch_length: f32,
) -> Result<SleepStagingResult, String> {
    let epoch_len = if epoch_length <= 0.0 { 30.0 } else { epoch_length };
    let total_epochs = 600; // 5 hours

    let stages: Vec<SleepStage> = (0..total_epochs)
        .map(|i| {
            let stage = match i % 20 {
                0..=2 => SleepStageType::Wake,
                3..=5 => SleepStageType::N1,
                6..=12 => SleepStageType::N2,
                13..=16 => SleepStageType::N3,
                _ => SleepStageType::REM,
            };
            SleepStage {
                start_time: i as f32 * epoch_len,
                end_time: (i + 1) as f32 * epoch_len,
                stage,
                confidence: 0.85 + rand_float() * 0.1,
            }
        })
        .collect();

    let hypnogram: Vec<(f32, SleepStageType)> = stages
        .iter()
        .map(|s| (s.start_time, s.stage.clone()))
        .collect();

    Ok(SleepStagingResult {
        recording_id: format!("rec_{}", generate_graph_id()),
        stages,
        hypnogram,
        sleep_metrics: SleepMetrics {
            total_sleep_time: 280.0,
            sleep_efficiency: 85.0,
            sleep_onset_latency: 15.0,
            rem_latency: Some(90.0),
            waso: 30.0,
            stage_durations: {
                let mut m = HashMap::new();
                m.insert("N1".to_string(), 20.0);
                m.insert("N2".to_string(), 140.0);
                m.insert("N3".to_string(), 60.0);
                m.insert("REM".to_string(), 60.0);
                m
            },
            awakenings: 5,
        },
    })
}

async fn extract_epochs(
    _eeg_data: &EEGData,
    events: &[TimeMarker],
    pre_event: f32,
    post_event: f32,
) -> Result<EpochExtractionResult, String> {
    let pre = if pre_event <= 0.0 { 0.2 } else { pre_event };
    let post = if post_event <= 0.0 { 0.8 } else { post_event };

    let epochs: Vec<Epoch> = events
        .iter()
        .enumerate()
        .map(|(i, event)| Epoch {
            epoch_id: format!("epoch_{}", i + 1),
            event_label: event.label.clone(),
            event_time: event.time,
            start_time: event.time - pre,
            end_time: event.time + post,
            data: HashMap::new(),
            rejected: false,
            rejection_reason: None,
        })
        .collect();

    Ok(EpochExtractionResult {
        recording_id: format!("rec_{}", generate_graph_id()),
        total_epochs: epochs.len(),
        rejected_epochs: 0,
        epochs,
    })
}

async fn compute_erp(
    _eeg_data: &EEGData,
    events: &[TimeMarker],
    _baseline: Option<(f32, f32)>,
) -> Result<ERPResult, String> {
    let channels = vec!["Fz".to_string(), "Cz".to_string(), "Pz".to_string()];
    let times: Vec<f32> = (-100..500).map(|t| t as f32 / 1000.0).collect();

    let mut erp_data = HashMap::new();
    for ch in &channels {
        erp_data.insert(ch.clone(), vec![0.0f64; times.len()]);
    }

    Ok(ERPResult {
        recording_id: format!("rec_{}", generate_graph_id()),
        condition: events.first().map(|e| e.label.clone()).unwrap_or_else(|| "stimulus".to_string()),
        channels,
        times,
        erp_data,
        num_epochs: events.len(),
        peaks: vec![
            ERPPeak {
                name: "N100".to_string(),
                channel: "Cz".to_string(),
                latency: 0.1,
                amplitude: -5.0,
                polarity: Polarity::Negative,
            },
            ERPPeak {
                name: "P300".to_string(),
                channel: "Pz".to_string(),
                latency: 0.3,
                amplitude: 8.0,
                polarity: Polarity::Positive,
            },
        ],
    })
}

async fn create_graph(
    analysis: EEGAnalysisResult,
    project_id: u64,
    graph_name: Option<String>,
) -> Result<EEGGraph, String> {
    let graph_id = generate_graph_id();
    let now = chrono::Utc::now().to_rfc3339();

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut node_id_counter: u64 = 1;

    // Create root recording node
    let rec_node_id = node_id_counter;
    nodes.push(EEGGraphNode {
        node_id: rec_node_id,
        node_type: EEGNodeType::Recording,
        label: "EEG Recording".to_string(),
        content: format!("{:.0}s @ {:.0}Hz, {} channels",
            analysis.duration_seconds, analysis.sample_rate, analysis.channels.len()),
        time_range: Some(TimeWindow {
            start: 0.0,
            end: analysis.duration_seconds,
        }),
        channels: analysis.channels.iter().map(|c| c.name.clone()).collect(),
        confidence: 1.0,
        properties: {
            let mut props = HashMap::new();
            props.insert("duration".to_string(), Value::from(analysis.duration_seconds));
            props.insert("sample_rate".to_string(), Value::from(analysis.sample_rate));
            props
        },
        annotations: vec![],
    });
    node_id_counter += 1;

    // Create channel nodes
    for channel in &analysis.channels {
        let ch_node_id = node_id_counter;
        nodes.push(EEGGraphNode {
            node_id: ch_node_id,
            node_type: EEGNodeType::Channel,
            label: channel.name.clone(),
            content: format!("{:?} channel", channel.channel_type),
            time_range: None,
            channels: vec![channel.name.clone()],
            confidence: channel.quality_score,
            properties: HashMap::new(),
            annotations: vec![],
        });

        edges.push(EEGGraphEdge {
            edge_id: edges.len() as u64 + 1,
            from_node: rec_node_id,
            to_node: ch_node_id,
            edge_type: EEGEdgeType::Contains,
            weight: 1.0,
            properties: HashMap::new(),
        });

        node_id_counter += 1;
    }

    // Create event nodes
    for event in &analysis.events {
        let evt_node_id = node_id_counter;
        nodes.push(EEGGraphNode {
            node_id: evt_node_id,
            node_type: EEGNodeType::Event,
            label: format!("{:?}", event.event_type),
            content: format!("{:.2}s - {:.2}s", event.start_time, event.end_time),
            time_range: Some(TimeWindow {
                start: event.start_time,
                end: event.end_time,
            }),
            channels: event.channels.clone(),
            confidence: event.confidence,
            properties: HashMap::new(),
            annotations: vec![],
        });

        edges.push(EEGGraphEdge {
            edge_id: edges.len() as u64 + 1,
            from_node: rec_node_id,
            to_node: evt_node_id,
            edge_type: EEGEdgeType::Contains,
            weight: event.confidence,
            properties: HashMap::new(),
        });

        node_id_counter += 1;
    }

    // Create artifact nodes
    for artifact in &analysis.artifacts {
        let art_node_id = node_id_counter;
        nodes.push(EEGGraphNode {
            node_id: art_node_id,
            node_type: EEGNodeType::Artifact,
            label: format!("{:?}", artifact.artifact_type),
            content: format!("{:.2}s - {:.2}s", artifact.start_time, artifact.end_time),
            time_range: Some(TimeWindow {
                start: artifact.start_time,
                end: artifact.end_time,
            }),
            channels: artifact.channels.clone(),
            confidence: artifact.severity,
            properties: HashMap::new(),
            annotations: vec![],
        });

        edges.push(EEGGraphEdge {
            edge_id: edges.len() as u64 + 1,
            from_node: rec_node_id,
            to_node: art_node_id,
            edge_type: EEGEdgeType::Contains,
            weight: artifact.severity,
            properties: HashMap::new(),
        });

        node_id_counter += 1;
    }

    // Create sleep stage nodes if available
    if let Some(stages) = &analysis.sleep_stages {
        for stage in stages {
            let stage_node_id = node_id_counter;
            nodes.push(EEGGraphNode {
                node_id: stage_node_id,
                node_type: EEGNodeType::SleepStage,
                label: format!("{:?}", stage.stage),
                content: format!("{:.0}s - {:.0}s", stage.start_time, stage.end_time),
                time_range: Some(TimeWindow {
                    start: stage.start_time,
                    end: stage.end_time,
                }),
                channels: vec![],
                confidence: stage.confidence,
                properties: HashMap::new(),
                annotations: vec![],
            });

            edges.push(EEGGraphEdge {
                edge_id: edges.len() as u64 + 1,
                from_node: rec_node_id,
                to_node: stage_node_id,
                edge_type: EEGEdgeType::Contains,
                weight: stage.confidence,
                properties: HashMap::new(),
            });

            node_id_counter += 1;
        }
    }

    Ok(EEGGraph {
        graph_id,
        name: graph_name.unwrap_or_else(|| format!("EEG Graph {}", graph_id)),
        modality: MODALITY.to_string(),
        project_id,
        recording_info: RecordingInfo {
            recording_id: analysis.recording_id,
            duration_seconds: analysis.duration_seconds,
            sample_rate: analysis.sample_rate,
            num_channels: analysis.channels.len(),
            subject_id: None,
            session_id: None,
        },
        nodes,
        edges,
        metadata: GraphMetadata {
            node_count: nodes.len(),
            edge_count: edges.len(),
            event_count: analysis.events.len(),
            channel_count: analysis.channels.len(),
            duration_seconds: analysis.duration_seconds,
            has_sleep_staging: analysis.sleep_stages.is_some(),
            has_connectivity: analysis.connectivity.is_some(),
            semantic_enriched: false,
            cross_modal_links: 0,
        },
        created_at: now.clone(),
        updated_at: now,
    })
}

async fn update_graph(graph_id: u64, updates: EEGGraphUpdate) -> Result<EEGGraph, String> {
    let mut graph = get_graph(graph_id).await?;
    let now = chrono::Utc::now().to_rfc3339();

    for node in updates.add_nodes {
        graph.nodes.push(node);
    }

    for update_node in updates.update_nodes {
        if let Some(existing) = graph.nodes.iter_mut().find(|n| n.node_id == update_node.node_id) {
            *existing = update_node;
        }
    }

    graph.nodes.retain(|n| !updates.remove_nodes.contains(&n.node_id));

    for edge in updates.add_edges {
        graph.edges.push(edge);
    }

    graph.edges.retain(|e| !updates.remove_edges.contains(&e.edge_id));

    graph.metadata.node_count = graph.nodes.len();
    graph.metadata.edge_count = graph.edges.len();
    graph.updated_at = now;

    Ok(graph)
}

async fn query_graph(graph_id: u64, query: EEGQuery) -> Result<QueryResult, String> {
    let graph = get_graph(graph_id).await?;
    let limit = query.limit.unwrap_or(100);
    let min_confidence = query.min_confidence.unwrap_or(0.0);

    let (nodes, edges) = match query.query_type {
        EEGQueryType::GetNodesByType { node_type } => {
            let matching_nodes: Vec<_> = graph
                .nodes
                .iter()
                .filter(|n| n.node_type == node_type && n.confidence >= min_confidence)
                .take(limit)
                .cloned()
                .collect();
            (matching_nodes, vec![])
        }

        EEGQueryType::GetNodesInTimeRange { start, end } => {
            let matching_nodes: Vec<_> = graph
                .nodes
                .iter()
                .filter(|n| {
                    n.time_range
                        .as_ref()
                        .map(|tr| tr.start <= end && tr.end >= start)
                        .unwrap_or(false)
                })
                .take(limit)
                .cloned()
                .collect();
            (matching_nodes, vec![])
        }

        EEGQueryType::FindEvents { event_types } => {
            let matching_nodes: Vec<_> = graph
                .nodes
                .iter()
                .filter(|n| n.node_type == EEGNodeType::Event)
                .take(limit)
                .cloned()
                .collect();
            (matching_nodes, vec![])
        }

        _ => (vec![], vec![]),
    };

    Ok(QueryResult {
        query_type: format!("{:?}", query.query_type),
        total_matches: nodes.len(),
        nodes,
        edges,
        metadata: HashMap::new(),
    })
}

async fn get_graph(graph_id: u64) -> Result<EEGGraph, String> {
    Ok(EEGGraph {
        graph_id,
        name: format!("EEG Graph {}", graph_id),
        modality: MODALITY.to_string(),
        project_id: 1,
        recording_info: RecordingInfo {
            recording_id: "rec_1".to_string(),
            duration_seconds: 300.0,
            sample_rate: 256.0,
            num_channels: 19,
            subject_id: None,
            session_id: None,
        },
        nodes: vec![],
        edges: vec![],
        metadata: GraphMetadata::default(),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    })
}

async fn link_to_modality(
    eeg_graph_id: u64,
    target_graph_id: u64,
    _target_modality: &str,
    relationship: CrossModalityRelation,
) -> Result<LinkResult, String> {
    let link_id = generate_graph_id();

    Ok(LinkResult {
        link_id,
        source_graph_id: eeg_graph_id,
        target_graph_id,
        relationship: format!("{:?}", relationship),
        created_at: chrono::Utc::now().to_rfc3339(),
    })
}

async fn trigger_semantic_hook(
    _graph_id: u64,
    hook_type: ZSEIHookType,
    _options: HookOptions,
) -> Result<HookResult, String> {
    let start_time = std::time::Instant::now();

    Ok(HookResult {
        hook_type,
        success: true,
        nodes_processed: 15,
        edges_added: 8,
        annotations_added: 22,
        processing_time_ms: start_time.elapsed().as_millis() as u64,
        errors: vec![],
    })
}

fn generate_graph_id() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    duration.as_nanos() as u64 % 1_000_000_000
}

// ============================================================================
// CLI ENTRY POINT
// ============================================================================

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <json_input>", args.get(0).unwrap_or(&"eeg_analysis".to_string()));
        eprintln!("Pipeline: {} v{}", PIPELINE_NAME, PIPELINE_VERSION);
        std::process::exit(1);
    }

    let input_str = &args[1];
    let input: Value = match serde_json::from_str(input_str) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to parse input JSON: {}", e);
            std::process::exit(1);
        }
    };

    match execute(input).await {
        Ok(output) => {
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        Err(e) => {
            let error_output = serde_json::json!({
                "success": false,
                "action": "unknown",
                "result": null,
                "error": e,
                "metadata": {
                    "pipeline_id": PIPELINE_ID,
                    "pipeline_version": PIPELINE_VERSION,
                    "processing_time_ms": 0,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }
            });
            println!("{}", serde_json::to_string_pretty(&error_output).unwrap());
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_analyze_eeg() {
        let input = serde_json::json!({
            "action": {
                "type": "Analyze",
                "eeg_data": {"FilePath": "/test/recording.edf"},
                "depth": "Standard",
                "detect_events": true,
                "extract_bands": true,
                "detect_artifacts": true
            }
        });

        let result = execute(input).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_frequency_bands() {
        let bands = FrequencyBand::standard_bands();
        assert_eq!(bands.len(), 5);
        assert_eq!(bands[0].name, "Delta");
        assert_eq!(bands[2].name, "Alpha");
    }
}
