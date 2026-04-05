//! ActiveSonarSensing — Pipeline #125
//!
//! Active acoustic ranging: time-of-flight, echo strength, phase, Doppler,
//! beamforming, side-scan/synthetic aperture sonar, underwater propagation.
//!
//! DISTINCT FROM:
//!   - Audio (103): air acoustic reconstruction; not active ranging or underwater physics.
//!   - Sound Reconstruction (110): species vocalizations; not active ranging.
//!   - Both lack: underwater propagation physics (multipath, absorption, thermoclines),
//!     side-scan mosaicking, sub-bottom penetration, active echo analysis.
//!
//! CROSS-LINKS:
//!   109 (3D)     → acoustic geometry feeds 3D reconstruction
//!   110 (Sound)  → marine vocalizations within sonar data
//!   111 (Bio)    → species identification from acoustic signatures
//!   117 (Geo)    → bathymetric maps on geospatial canvas
//!   121 (Kine)   → AUV/ROV navigation from sonar returns
//!   114 (Therm)  → thermal layers affect acoustic propagation
//!   124 (Radar)  → multi-modal target fusion (above/below water handoff)
//!
//! STORAGE: ZSEI containers under /Modalities/Sonar/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

// ─────────────────────────────────────────────────────────────────────────────
// INPUT TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum SonarModalityAction {
    /// Analyze sonar data: produce SonarAnalysisResult
    Analyze {
        data: SonarDataSource,
        extract_targets: bool,
        extract_bathymetry: bool,
        extract_biology: bool,
        extract_sub_bottom: bool,
        extract_signatures: bool,
    },
    /// Create a graph from an analysis result
    CreateGraph {
        analysis: SonarAnalysisResult,
        project_id: u64,
    },
    /// Update an existing graph with new pings
    UpdateGraph {
        graph_id: u64,
        new_pings: Vec<SonarPingResult>,
        project_id: u64,
    },
    /// Process a single sonar ping → echo profile
    ProcessPing {
        raw_ping: Vec<f32>,
        transmit_freq_hz: f32,
        transmit_level_db: f32,
        pulse_length_ms: f32,
        sample_rate_hz: f32,
        water_sound_speed_ms: f32,
    },
    /// Form a synthetic aperture sonar (SAS) image
    FormSASImage {
        pings: Vec<SonarPingResult>,
        platform_trajectory: Vec<PlatformNavigation>,
        range_resolution_m: f32,
        along_track_resolution_m: f32,
    },
    /// Beamform across a hydrophone array
    BeamformArray {
        channel_data: Vec<Vec<f32>>,
        hydrophone_positions: Vec<(f32, f32, f32)>,
        steering_bearings_deg: Vec<f32>,
        freq_hz: f32,
        sound_speed_ms: f32,
    },
    /// Build a bathymetric depth map from multibeam returns
    BuildBathymetry {
        multibeam_pings: Vec<MultibeamPing>,
        nav_data: Vec<PlatformNavigation>,
        coordinate_reference: String,
    },
    /// Classify acoustic signature (species, material, target type)
    ClassifySignature {
        echo_data: Vec<f32>,
        freq_hz: f32,
        context: SignatureContext,
    },
    /// Model acoustic propagation environment
    ModelPropagation {
        sound_speed_profile: Vec<(f32, f32)>,     // (depth_m, speed_ms)
        bottom_type: BottomType,
        surface_state: SeaState,
        frequency_hz: f32,
        source_depth_m: f32,
        max_range_m: f32,
    },
    /// Query graph
    QueryGraph {
        graph_id: u64,
        query: SonarGraphQuery,
    },
    /// Retrieve full graph for Context Viewer
    GetGraph {
        graph_id: u64,
    },
    /// Trigger ZSEI semantic hooks
    TriggerSemanticHook {
        graph_id: u64,
        hook: SonarSemanticHook,
    },
    /// Export sonar products
    ExportProduct {
        graph_id: u64,
        format: SonarExportFormat,
    },
    /// Stream to UI session
    StreamToUI {
        graph_id: u64,
        session_id: String,
        display_mode: SonarDisplayMode,
    },
    /// Headless processing (AGI-first, no rendering)
    HeadlessProcess {
        graph_id: u64,
        operations: Vec<SonarOperation>,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// DATA SOURCE TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SonarDataSource {
    /// Single-beam echosounder file
    SingleBeamFile {
        file_path: String,
        format: SonarFileFormat,
        frequency_hz: f32,
    },
    /// Multibeam echosounder file
    MultibeamFile {
        file_path: String,
        format: SonarFileFormat,
        beam_count: u32,
        swath_angle_deg: f32,
    },
    /// Side-scan sonar file
    SideScanFile {
        file_path: String,
        format: SonarFileFormat,
        frequency_hz: f32,
        max_range_m: f32,
        channel: SideScanChannel,
    },
    /// Synthetic aperture sonar
    SASFile {
        file_path: String,
        format: SonarFileFormat,
        along_track_resolution_m: f32,
        range_resolution_m: f32,
    },
    /// Sub-bottom profiler
    SubBottomFile {
        file_path: String,
        format: SonarFileFormat,
        frequency_hz: f32,
        penetration_depth_m: f32,
    },
    /// Acoustic Doppler Current Profiler (ADCP)
    ADCPFile {
        file_path: String,
        bin_size_m: f32,
        frequency_hz: f32,
    },
    /// Passive monitoring array with active ping records
    HydrophoneArray {
        files: Vec<String>,
        element_positions: Vec<(f32, f32, f32)>,
        sample_rate_hz: f32,
    },
    /// Live stream from sonar hardware
    LiveStream {
        endpoint: String,
        sonar_type: SonarType,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SonarFileFormat { XTF, JSF, S7K, RAW, GSF, HDF5, NetCDF, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SideScanChannel { Port, Starboard, Both }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SonarType {
    SingleBeam, Multibeam, SideScan, SAS, SubBottom, ADCP, Echosounder, FishFinder, Custom(String)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignatureContext {
    MarineBiology,          // species/size identification
    SedimentClassification, // bottom type
    TargetDetection,        // man-made object
    BubbleColumn,           // gas seep / ship propeller
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum BottomType {
    #[default] Unknown, Sand, Mud, Rock, Gravel, Coral, Vegetation, HardBottom, SoftBottom
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SeaState {
    #[default] Calm,       // SS 0-1
    Slight,                // SS 2
    Moderate,              // SS 3-4
    Rough,                 // SS 5-6
    VeryRough,             // SS 7+
}

// ─────────────────────────────────────────────────────────────────────────────
// ANALYSIS RESULT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SonarAnalysisResult {
    pub analysis_id: u64,

    // SONAR PARAMETERS
    pub primary_frequency_hz: f32,
    pub sound_speed_profile: Vec<(f32, f32)>,       // (depth_m, speed_ms)
    pub mean_sound_speed_ms: f32,
    pub bottom_type: BottomType,
    pub sea_state: SeaState,
    pub sonar_type: Option<SonarType>,

    // DETECTIONS
    pub echo_returns: Vec<SonarEcho>,
    pub targets: Vec<SonarTarget>,
    pub clutter_echoes: Vec<ClutterEcho>,

    // BATHYMETRY
    pub bathymetry: Option<BathymetryData>,

    // ACOUSTIC SIGNATURES
    pub acoustic_signatures: Vec<AcousticSignature>,

    // SUB-BOTTOM
    pub sub_bottom_layers: Vec<SubBottomLayer>,
    pub subsurface_features: Vec<SubsurfaceFeature>,

    // BIOLOGY
    pub biological_detections: Vec<BiologicalDetection>,
    pub fish_school_detections: Vec<FishSchool>,

    // CURRENT/FLOW
    pub current_vectors: Vec<AcousticCurrentVector>,

    // 3D RECONSTRUCTION
    pub acoustic_point_cloud: Vec<AcousticPoint3D>,

    // PROPAGATION MODEL
    pub propagation_model: Option<PropagationModel>,

    // METADATA
    pub source_description: String,
    pub platform_data: Option<SonarPlatformData>,
    pub processing_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SonarEcho {
    pub echo_id: u64,
    pub range_m: f32,
    pub bearing_deg: f32,
    pub depth_m: f32,
    pub echo_strength_db: f32,
    pub phase_rad: f32,
    pub two_way_travel_time_ms: f32,
    pub pulse_index: u32,
    pub timestamp: f64,
    pub frequency_hz: f32,
    pub target_strength_db: Option<f32>,    // TS = EL - SL + 2*TL
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SonarTarget {
    pub target_id: u64,
    pub range_m: f32,
    pub bearing_deg: f32,
    pub depth_m: f32,
    pub target_strength_db: f32,
    pub radial_velocity_ms: f32,           // Doppler
    pub classification: SonarTargetClass,
    pub dimensions_estimate: Option<(f32, f32, f32)>,  // (L, W, H) estimate in meters
    pub geo_location: Option<(f64, f64, f64)>,
    pub acoustic_signature_id: Option<u64>,
    pub track_id: Option<u64>,
    pub detection_timestamp: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SonarTargetClass {
    #[default] Unknown,
    Vessel, Submarine, UUV, Mine, Wreck, Rock, Debris,
    Fish, MarineMammal, FishSchool, GasSeep, BubbleColumn,
    SedimentFeature, PipelineOrCable, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClutterEcho {
    pub clutter_id: u64,
    pub range_m: f32, pub bearing_deg: f32, pub depth_m: f32,
    pub echo_strength_db: f32,
    pub clutter_type: SonarClutterType,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SonarClutterType {
    #[default] VolumeReverberation, BoundaryReverberation,
    BiologicalNoise, ThermoclineScatter, BubbleNoise, PlatformNoise
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BathymetryData {
    pub data_id: u64,
    pub grid_resolution_m: f32,
    pub coverage_area_km2: f32,
    pub min_depth_m: f32,
    pub max_depth_m: f32,
    pub mean_depth_m: f32,
    /// Row-major grid: depth values in meters
    pub depth_grid: Vec<Vec<f32>>,
    /// Geographic extent [min_lat, min_lon, max_lat, max_lon]
    pub geo_extent: Option<[f64; 4]>,
    pub coordinate_reference: String,
    pub features: Vec<BathymetricFeature>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BathymetricFeature {
    pub feature_id: u64,
    pub feature_type: BathyFeatureType,
    pub location: (f64, f64),
    pub depth_m: f32,
    pub extent_m: (f32, f32),
    pub height_m: f32,          // relief above/below surrounding seafloor
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum BathyFeatureType {
    #[default] Unknown, Seamount, Canyon, Ridge, Trench, Depression, Channel, Escarpment, Mound
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AcousticSignature {
    pub signature_id: u64,
    pub target_id: Option<u64>,
    pub frequency_spectrum: Vec<(f32, f32)>,    // (freq_hz, level_db) at discrete bands
    pub target_strength_vs_aspect: Vec<(f32, f32)>,  // (aspect_deg, ts_db)
    pub dominant_frequency_hz: f32,
    pub bandwidth_hz: f32,
    pub identified_species: Option<String>,
    pub identified_material: Option<String>,
    pub classification_notes: Vec<String>,
    pub reference_library_match: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SubBottomLayer {
    pub layer_id: u64,
    pub top_depth_m: f32,
    pub bottom_depth_m: f32,
    pub thickness_m: f32,
    pub reflection_strength_db: f32,
    pub inferred_material: Option<String>,      // e.g., "sand", "clay", "bedrock"
    pub two_way_travel_time_ms: f32,
    pub lateral_extent_m: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SubsurfaceFeature {
    pub feature_id: u64,
    pub feature_type: SubsurfaceFeatureType,
    pub depth_m: f32,
    pub location: (f64, f64),
    pub extent_m: (f32, f32),
    pub confidence_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SubsurfaceFeatureType {
    #[default] Unknown, GasSeep, FluidExpulsion, BSR,   // bottom simulating reflector
    BuriedChannel, PaleoValley, SaltDiapir, FaultZone, BuriedDebris
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BiologicalDetection {
    pub detection_id: u64,
    pub target_id: Option<u64>,
    pub species_estimate: Option<String>,
    pub vocalization_type: Option<String>,      // if it's a call
    pub call_frequency_hz: Option<f32>,
    pub call_duration_ms: Option<f32>,
    pub bearing_deg: f32,
    pub range_m: Option<f32>,
    pub depth_m: Option<f32>,
    pub timestamp: f64,
    pub size_estimate_m: Option<f32>,
    pub behavior_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FishSchool {
    pub school_id: u64,
    pub center_range_m: f32,
    pub center_bearing_deg: f32,
    pub center_depth_m: f32,
    pub extent_m: (f32, f32, f32),
    pub mean_sv_db: f32,                    // volume scattering strength
    pub estimated_fish_count: Option<u32>,
    pub species_estimate: Option<String>,
    pub school_depth_m: f32,
    pub migration_direction_deg: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AcousticCurrentVector {
    pub vector_id: u64,
    pub depth_m: f32,
    pub range_m: f32,
    pub bearing_deg: f32,
    pub speed_ms: f32,
    pub direction_deg: f32,
    pub radial_velocity_ms: f32,            // raw Doppler measurement
    pub timestamp: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AcousticPoint3D {
    pub point_id: u64,
    pub x: f32, pub y: f32, pub z: f32,
    pub backscatter_db: f32,
    pub geo: Option<(f64, f64, f64)>,
    pub source_beam_index: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PropagationModel {
    pub model_id: u64,
    pub frequency_hz: f32,
    pub sound_speed_profile: Vec<(f32, f32)>,
    pub bottom_type: BottomType,
    pub transmission_loss_db: Vec<(f32, f32)>,   // (range_m, loss_db)
    pub critical_angle_deg: f32,
    pub convergence_zones: Vec<ConvergenceZone>,
    pub shadow_zones: Vec<ShadowZone>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConvergenceZone {
    pub zone_id: u32,
    pub center_range_km: f32,
    pub width_km: f32,
    pub gain_db: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShadowZone {
    pub zone_id: u32,
    pub start_range_m: f32,
    pub end_range_m: f32,
    pub start_depth_m: f32,
    pub end_depth_m: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SonarPlatformData {
    pub platform_type: String,      // "surface vessel", "AUV", "ROV", "moored"
    pub depth_m: f32,
    pub speed_ms: f32,
    pub heading_deg: f32,
    pub position: (f64, f64, f64),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SonarPingResult {
    pub ping_id: u64,
    pub range_samples: Vec<f32>,
    pub bearing_deg: f32,
    pub range_resolution_m: f32,
    pub timestamp: f64,
    pub peak_echo_strength_db: f32,
    pub peak_range_m: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MultibeamPing {
    pub ping_id: u64,
    pub beams: Vec<MultibeamBeam>,
    pub timestamp: f64,
    pub nav: PlatformNavigation,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MultibeamBeam {
    pub beam_index: u32,
    pub angle_deg: f32,
    pub range_m: f32,
    pub backscatter_db: f32,
    pub depth_m: f32,
    pub across_track_m: f32,
    pub valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlatformNavigation {
    pub timestamp: f64,
    pub lat: f64, pub lon: f64, pub depth_m: f32,
    pub heading_deg: f32, pub pitch_deg: f32, pub roll_deg: f32,
    pub speed_ms: f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH NODE TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SonarNodeType {
    #[default] SonarScene,             // root node
    SonarEcho,                         // single echo return
    BeamformedTarget,                  // beamformed detection
    SonarTarget,                       // classified target
    SonarTrack,                        // multi-ping target trajectory
    AcousticSignature,                 // frequency signature of a target/species
    SideScanStripe,                    // single sonar stripe (port or starboard)
    SASImagePatch,                     // synthetic aperture sonar image patch
    BathymetricNode,                   // depth measurement or feature
    SubsurfaceLayer,                   // sub-bottom reflector
    SubsurfaceFeatureNode,             // sub-bottom interpreted feature
    BiologicalNode,                    // biological detection (species, vocalization)
    FishSchoolNode,                    // fish aggregation
    CurrentVectorNode,                 // acoustic current measurement
    PropagationModelNode,              // acoustic propagation environment
    ConvergenceZoneNode,               // acoustic convergence zone
    ShadowZoneNode,                    // acoustic shadow zone
    AcousticPoint3D,                   // 3D reconstruction point
    PlatformNode,                      // sonar platform position
    GeographicLocation,                // geo-referenced position
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SonarGraphNode {
    pub node_id: u64,
    pub node_type: SonarNodeType,
    pub content: String,

    // SONAR-SPECIFIC
    pub range_m: Option<f32>,
    pub bearing_deg: Option<f32>,
    pub depth_m: Option<f32>,
    pub echo_strength_db: Option<f32>,
    pub radial_velocity_ms: Option<f32>,
    pub frequency_hz: Option<f32>,
    pub timestamp: Option<f64>,
    pub geo_location: Option<(f64, f64, f64)>,
    pub classification: Option<String>,
    pub species: Option<String>,

    // UNIVERSAL NODE FIELDS (Section B.2)
    pub provisional: bool,
    pub provisional_status: ProvisionalStatus,
    pub materialized_path: Option<String>,
    pub created_by_step: Option<u32>,
    pub updated_by_step: Option<u32>,
    pub version: u32,
    pub version_notes: Vec<VersionNote>,
    pub keywords: Vec<String>,
    pub embedding_hint: Option<String>,
    pub hotness_score: f32,
    pub source_chunk_index: Option<u32>,
    pub source_start_char: Option<usize>,
    pub source_end_char: Option<usize>,
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH EDGE TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SonarEdgeType {
    // ── STRUCTURAL ──
    #[default] Contains,
    Precedes,
    PartOf,

    // ── SONAR-SPECIFIC ──
    DetectedAtRange,                    // echo detected at range/bearing
    ReflectsWithAcousticSignature,      // backscatter matches acoustic signature
    ReconstructsTo,                     // sonar data reconstructs 3D geometry
    PenetratesTo,                       // sub-bottom penetration
    SpeciesSignature,                   // vocalization/echo matches species
    MovingWithVelocity,                 // Doppler velocity measurement
    BeamformedFrom,                     // target derived from beamforming
    FormsImageOf,                       // SAS image shows scene
    BathymetryOf,                       // bathymetric measurement of seafloor
    ShadowedBy,                         // target casts acoustic shadow
    PropagatesThrough,                  // signal propagates through medium
    ScatteredBy,                        // reverberation scattered by
    RefractedBy,                        // signal refracted by layer
    AssociatedBiology,                  // echo associated with biological target
    LayeredAbove,                       // sub-bottom layer above another
    GeoreferencedAt,                    // measurement at geo position

    // ── CROSS-MODAL ──
    /// Links acoustic point cloud → 3D geometry (109)
    ContributesTo3DGeometry,
    /// Links biological detection → species biology node (111)
    LinksToMarineBiology,
    /// Links vocalization → sound reconstruction node (110)
    VocalizationLinksToSound,
    /// Links bathymetry → geospatial map (117)
    PlottedOnGeoMap,
    /// Links AUV track → kinematics model (121)
    FeedsAUVKinematics,
    /// Links propagation model → thermal layer data (114)
    AffectedByThermalLayer,
    /// Links multi-modal target → radar track (124)
    CrossFusedWithRadar,

    // ── UNIVERSAL SEMANTIC (Section B.1) ──
    Performs, Affects, Implies, Contradicts, Elaborates, Summarizes, Supports,
    TemporalPrecedes, TemporalFollows, CausedBy, Enables, Prevents,
    FunctionalRole, InstanceOf,
    DerivedFrom, VersionOf, RefinesTo, ForkedFrom,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SonarGraphEdge {
    pub edge_id: u64,
    pub from_node: u64,
    pub to_node: u64,
    pub edge_type: SonarEdgeType,
    pub weight: f32,
    pub provenance: EdgeProvenance,
    pub created_by_step: Option<u32>,
    pub version: u32,
    pub version_notes: Vec<VersionNote>,
    pub properties: HashMap<String, serde_json::Value>,
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH STRUCTURE
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SonarGraph {
    pub graph_id: u64,
    pub project_id: u64,
    pub source_description: String,
    pub nodes: Vec<SonarGraphNode>,
    pub edges: Vec<SonarGraphEdge>,
    pub root_node_id: u64,
    pub state: GraphStateType,
    pub state_history: Vec<GraphStateTransition>,
    pub created_at: String,
    pub updated_at: String,
    pub version: u32,
    pub version_notes: Vec<VersionNote>,
}

// ─────────────────────────────────────────────────────────────────────────────
// QUERY TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SonarGraphQuery {
    NodeDetail { node_id: u64 },
    TargetsByClassification { classification: String },
    BathymetricFeaturesInRegion { lat_min: f64, lat_max: f64, lon_min: f64, lon_max: f64 },
    BiologicalDetections,
    AcousticSignatures,
    SubBottomLayers,
    CrossModalLinks { node_id: u64 },
    AGIActivity,
    AllNodes,
    AllEdges,
}

// ─────────────────────────────────────────────────────────────────────────────
// SEMANTIC HOOKS
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SonarSemanticHook {
    OnGraphCreated,
    OnInferRelationships,
    OnEdgeCompletion,
    OnCrossModalityLink { target_modality: String, target_graph_id: u64 },
}

// ─────────────────────────────────────────────────────────────────────────────
// EXPORT / DISPLAY TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SonarExportFormat {
    GeoTIFF,                // bathymetry/side-scan as georeferenced image
    GeoJSON,                // targets/features as GeoJSON
    XTF,                    // extended triton format (sonar native)
    GSF,                    // generic sensor format
    NetCDF,                 // scientific arrays
    CSV,                    // target detections
    LASPointCloud,          // acoustic 3D point cloud
    KML,                    // tracks on Google Earth
    ENICAR,                 // species identification format
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SonarDisplayMode {
    WaterfallDisplay,       // time vs bearing scrolling
    SideScanMosaic,         // stitched side-scan image
    BathymetricMap,         // depth-colored map
    EchoStrengthHeatmap,    // bearing vs range heat map
    FrequencySpectrum,      // acoustic spectrum
    TargetTable,            // tabular target list
    PropagationDisplay,     // TL vs range/depth
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SonarOperation {
    RefineTargetClassification { target_id: u64 },
    BuildMosaic { coverage_area: [f64; 4] },
    ExportBathymetry,
    CrossMatchWithBiology { bio_graph_id: u64 },
    UpdatePropagationModel,
}

// ─────────────────────────────────────────────────────────────────────────────
// OUTPUT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SonarModalityOutput {
    pub success: bool,
    pub graph_id: Option<u64>,
    pub graph: Option<SonarGraph>,
    pub analysis: Option<SonarAnalysisResult>,
    pub ping_result: Option<SonarPingResult>,
    pub sas_image: Option<SASImageResult>,
    pub bathymetry: Option<BathymetryData>,
    pub beamformed_data: Option<Vec<Vec<f32>>>,
    pub signature_result: Option<AcousticSignature>,
    pub propagation_model: Option<PropagationModel>,
    pub query_result: Option<serde_json::Value>,
    pub export_path: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SASImageResult {
    pub image_id: u64,
    pub width_pixels: u32,
    pub height_pixels: u32,
    pub along_track_resolution_m: f32,
    pub range_resolution_m: f32,
    pub geo_extent: Option<[f64; 4]>,
    pub image_file_path: Option<String>,
    pub detected_objects: Vec<SASDetection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SASDetection {
    pub detection_id: u64,
    pub pixel_x: u32, pub pixel_y: u32,
    pub extent_pixels: (u32, u32),
    pub object_type: String,
    pub shadow_length_m: f32,
    pub height_estimate_m: f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// SHARED TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ProvisionalStatus {
    #[default] Planned, Generating, Generated, Validated, Finalized, Failed, RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VersionNote {
    pub version: u32, pub note: String,
    pub step_index: Option<u32>, pub timestamp: String, pub change_type: ChangeType,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ChangeType {
    #[default] Created, Updated, CrossLinked, EnrichedBySemantic, EnrichedByHook, RolledBack, Finalized,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum EdgeProvenance {
    #[default] Unknown,
    DerivedFromPrompt, DerivedFromChunk(u32), DerivedFromChunkGraph(u64),
    DerivedFromModalityGraph(u64), DerivedFromFile(String),
    DerivedFromAMT, DerivedFromBlueprint(u32), DerivedFromMethodology(u64),
    DerivedFromCrossModal, DerivedFromHook, VersionOf(u32), ForkedFrom(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum GraphStateType {
    #[default] Created, SemanticEnriched, CrossLinked, Stable, Updated, ReValidating, Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStateTransition {
    pub from: GraphStateType, pub to: GraphStateType,
    pub timestamp: String, pub triggered_by_step: Option<u32>,
}

// ─────────────────────────────────────────────────────────────────────────────
// PIPELINE EXECUTOR
// ─────────────────────────────────────────────────────────────────────────────

struct PipelineExecutor {
    zsei_path: String,
    prompt_pipeline_path: String,
}

impl PipelineExecutor {
    fn new() -> Self {
        Self {
            zsei_path: env::var("OZONE_ZSEI_PATH").unwrap_or_else(|_| "./zsei_data".into()),
            prompt_pipeline_path: env::var("OZONE_PROMPT_PIPELINE").unwrap_or_else(|_| "./pipeline_9".into()),
        }
    }

    async fn llm_zero_shot(&self, prompt: &str, max_tokens: usize) -> Result<String, String> {
        let input = serde_json::json!({
            "action": "Prompt",
            "prompt": prompt,
            "max_tokens": max_tokens,
            "temperature": 0.05,
            "system_context": "Sonar signal analysis. Return only valid JSON."
        });
        let output = std::process::Command::new(&self.prompt_pipeline_path)
            .arg("--input").arg(input.to_string())
            .output().map_err(|e| format!("LLM call failed: {}", e))?;
        let result: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("LLM parse failed: {}", e))?;
        Ok(result["response"].as_str().unwrap_or("{}").to_string())
    }

    fn save_graph(&self, graph: &SonarGraph) -> Result<(), String> {
        let path = format!("{}/local/sonar_graph_{}.json", self.zsei_path, graph.graph_id);
        if let Some(parent) = std::path::Path::new(&path).parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, serde_json::to_string_pretty(graph).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())
    }

    fn load_graph(&self, graph_id: u64) -> Result<SonarGraph, String> {
        let path = format!("{}/local/sonar_graph_{}.json", self.zsei_path, graph_id);
        let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&data).map_err(|e| e.to_string())
    }

    fn generate_id(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as u64
    }

    fn now_iso8601(&self) -> String { format!("{}", self.generate_id()) }

    fn extract_json_array(raw: &str) -> String {
        if let (Some(s), Some(e)) = (raw.find('['), raw.rfind(']')) { raw[s..=e].to_string() }
        else { "[]".to_string() }
    }

    fn extract_json_object(raw: &str) -> String {
        if let (Some(s), Some(e)) = (raw.find('{'), raw.rfind('}')) { raw[s..=e].to_string() }
        else { "{}".to_string() }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// LLM-BASED ANALYSIS
// ─────────────────────────────────────────────────────────────────────────────

impl PipelineExecutor {
    async fn infer_target_classifications(&self, targets: &[SonarTarget]) -> Vec<(u64, String)> {
        if targets.is_empty() { return vec![]; }
        let list: Vec<serde_json::Value> = targets.iter().map(|t| serde_json::json!({
            "target_id": t.target_id, "range_m": t.range_m, "depth_m": t.depth_m,
            "target_strength_db": t.target_strength_db,
            "radial_velocity_ms": t.radial_velocity_ms,
            "dimensions_estimate": t.dimensions_estimate,
        })).collect();

        let prompt = format!(r#"
Classify each sonar target. Options: Vessel, Submarine, UUV, Mine, Wreck, Rock, Debris,
Fish, MarineMammal, FishSchool, GasSeep, BubbleColumn, SedimentFeature, PipelineOrCable, Unknown.

Targets:
{}

Return ONLY valid JSON array:
[{{"target_id": N, "classification": "...", "reasoning": "brief"}}]"#,
            serde_json::to_string_pretty(&list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 600).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str)
                    .unwrap_or_default().into_iter()
                    .filter_map(|v| Some((v["target_id"].as_u64()?, v["classification"].as_str()?.to_string())))
                    .collect()
            }
            Err(_) => vec![],
        }
    }

    async fn identify_species_from_signatures(&self, sigs: &[AcousticSignature]) -> Vec<(u64, String)> {
        if sigs.is_empty() { return vec![]; }
        let list: Vec<serde_json::Value> = sigs.iter().map(|s| serde_json::json!({
            "signature_id": s.signature_id,
            "dominant_frequency_hz": s.dominant_frequency_hz,
            "bandwidth_hz": s.bandwidth_hz,
            "spectrum_count": s.frequency_spectrum.len(),
        })).collect();

        let prompt = format!(r#"
Identify likely marine species or material from acoustic signatures.
Consider: cetaceans, fish, crustaceans, sediment types, man-made materials.

Signatures:
{}

Return ONLY valid JSON array:
[{{"signature_id": N, "identified_species": "species_or_material", "reasoning": "brief"}}]"#,
            serde_json::to_string_pretty(&list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 500).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str)
                    .unwrap_or_default().into_iter()
                    .filter_map(|v| {
                        let id = v["signature_id"].as_u64()?;
                        let sp = v["identified_species"].as_str()?.to_string();
                        Some((id, sp))
                    }).collect()
            }
            Err(_) => vec![],
        }
    }

    async fn infer_semantic_relationships(&self, nodes: &[SonarGraphNode]) -> Vec<(u64, u64, SonarEdgeType, String)> {
        if nodes.len() < 2 { return vec![]; }
        let node_list: Vec<serde_json::Value> = nodes.iter().take(30).map(|n| serde_json::json!({
            "node_id": n.node_id, "type": format!("{:?}", n.node_type),
            "content": n.content.chars().take(80).collect::<String>(),
            "range_m": n.range_m, "depth_m": n.depth_m,
        })).collect();

        let prompt = format!(r#"
Identify semantic relationships between these sonar graph nodes not captured structurally.

Nodes:
{}

Relationship types: DetectedAtRange, ReflectsWithAcousticSignature, SpeciesSignature,
MovingWithVelocity, BathymetryOf, LayeredAbove, ShadowedBy, AssociatedBiology,
Affects, CausedBy, Enables, TemporalPrecedes, DerivedFrom

Return ONLY valid JSON array:
[{{"from_node_id": N, "to_node_id": M, "edge_type": "TypeName", "reason": "brief"}}]"#,
            serde_json::to_string_pretty(&node_list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 800).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str)
                    .unwrap_or_default().into_iter()
                    .filter_map(|v| {
                        let from = v["from_node_id"].as_u64()?;
                        let to = v["to_node_id"].as_u64()?;
                        let etype = map_sonar_edge_str(v["edge_type"].as_str().unwrap_or("Affects"));
                        let reason = v["reason"].as_str().unwrap_or("").to_string();
                        Some((from, to, etype, reason))
                    }).collect()
            }
            Err(_) => vec![],
        }
    }

    async fn classify_sub_bottom_layers(&self, layers: &[SubBottomLayer]) -> Vec<(u64, String)> {
        if layers.is_empty() { return vec![]; }
        let list: Vec<serde_json::Value> = layers.iter().map(|l| serde_json::json!({
            "layer_id": l.layer_id,
            "thickness_m": l.thickness_m,
            "reflection_strength_db": l.reflection_strength_db,
            "two_way_travel_time_ms": l.two_way_travel_time_ms,
        })).collect();

        let prompt = format!(r#"
Classify sub-bottom sediment layers from acoustic reflection data.
Consider: sand, mud, clay, silt, gravel, rock, gas-bearing sediments, ice.

Layers:
{}

Return ONLY valid JSON array:
[{{"layer_id": N, "inferred_material": "material_name", "reasoning": "brief"}}]"#,
            serde_json::to_string_pretty(&list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 400).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str)
                    .unwrap_or_default().into_iter()
                    .filter_map(|v| Some((v["layer_id"].as_u64()?, v["inferred_material"].as_str()?.to_string())))
                    .collect()
            }
            Err(_) => vec![],
        }
    }
}

fn map_sonar_edge_str(s: &str) -> SonarEdgeType {
    match s {
        "DetectedAtRange"               => SonarEdgeType::DetectedAtRange,
        "ReflectsWithAcousticSignature" => SonarEdgeType::ReflectsWithAcousticSignature,
        "ReconstructsTo"                => SonarEdgeType::ReconstructsTo,
        "PenetratesTo"                  => SonarEdgeType::PenetratesTo,
        "SpeciesSignature"              => SonarEdgeType::SpeciesSignature,
        "MovingWithVelocity"            => SonarEdgeType::MovingWithVelocity,
        "BeamformedFrom"                => SonarEdgeType::BeamformedFrom,
        "FormsImageOf"                  => SonarEdgeType::FormsImageOf,
        "BathymetryOf"                  => SonarEdgeType::BathymetryOf,
        "ShadowedBy"                    => SonarEdgeType::ShadowedBy,
        "PropagatesThrough"             => SonarEdgeType::PropagatesThrough,
        "ScatteredBy"                   => SonarEdgeType::ScatteredBy,
        "RefractedBy"                   => SonarEdgeType::RefractedBy,
        "AssociatedBiology"             => SonarEdgeType::AssociatedBiology,
        "LayeredAbove"                  => SonarEdgeType::LayeredAbove,
        "ContributesTo3DGeometry"       => SonarEdgeType::ContributesTo3DGeometry,
        "LinksToMarineBiology"          => SonarEdgeType::LinksToMarineBiology,
        "VocalizationLinksToSound"      => SonarEdgeType::VocalizationLinksToSound,
        "PlottedOnGeoMap"               => SonarEdgeType::PlottedOnGeoMap,
        "FeedsAUVKinematics"            => SonarEdgeType::FeedsAUVKinematics,
        "AffectedByThermalLayer"        => SonarEdgeType::AffectedByThermalLayer,
        "CrossFusedWithRadar"           => SonarEdgeType::CrossFusedWithRadar,
        "Affects"           => SonarEdgeType::Affects,
        "CausedBy"          => SonarEdgeType::CausedBy,
        "Enables"           => SonarEdgeType::Enables,
        "Prevents"          => SonarEdgeType::Prevents,
        "TemporalPrecedes"  => SonarEdgeType::TemporalPrecedes,
        "TemporalFollows"   => SonarEdgeType::TemporalFollows,
        "DerivedFrom"       => SonarEdgeType::DerivedFrom,
        "VersionOf"         => SonarEdgeType::VersionOf,
        "PartOf"            => SonarEdgeType::PartOf,
        _                   => SonarEdgeType::Affects,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH CREATION
// ─────────────────────────────────────────────────────────────────────────────

async fn create_graph(
    executor: &PipelineExecutor,
    analysis: SonarAnalysisResult,
    project_id: u64,
) -> SonarModalityOutput {
    let graph_id = executor.generate_id();
    let now = executor.now_iso8601();
    let mut nodes: Vec<SonarGraphNode> = Vec::new();
    let mut edges: Vec<SonarGraphEdge> = Vec::new();
    let mut node_id: u64 = 1;
    let mut edge_id: u64 = 1;

    // ── ROOT NODE ──
    let root_id = node_id;
    nodes.push(SonarGraphNode {
        node_id: root_id,
        node_type: SonarNodeType::SonarScene,
        content: format!("Sonar scene: {}Hz {} targets {} bio {} bathy layers",
            analysis.primary_frequency_hz,
            analysis.targets.len(),
            analysis.biological_detections.len(),
            analysis.sub_bottom_layers.len()),
        frequency_hz: Some(analysis.primary_frequency_hz),
        materialized_path: Some(format!("/Modalities/Sonar/Project_{}/Graph_{}", project_id, graph_id)),
        provisional: false,
        provisional_status: ProvisionalStatus::Finalized,
        version: 1,
        version_notes: vec![VersionNote { version: 1, note: "Initial creation".into(), step_index: None, timestamp: now.clone(), change_type: ChangeType::Created }],
        keywords: vec!["sonar".into(), "scene".into()],
        hotness_score: 1.0,
        ..Default::default()
    });
    node_id += 1;

    // ── PLATFORM NODE ──
    if let Some(ref plat) = analysis.platform_data {
        let pid = node_id;
        nodes.push(SonarGraphNode {
            node_id: pid,
            node_type: SonarNodeType::PlatformNode,
            content: format!("Platform: {} depth={:.1}m spd={:.1}m/s hdg={:.1}°",
                plat.platform_type, plat.depth_m, plat.speed_ms, plat.heading_deg),
            depth_m: Some(plat.depth_m),
            materialized_path: Some(format!("/Modalities/Sonar/Project_{}/Graph_{}/Platform", project_id, graph_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: vec!["platform".into(), plat.platform_type.clone()],
            hotness_score: 0.8, ..Default::default()
        });
        edges.push(SonarGraphEdge { edge_id, from_node: root_id, to_node: pid, edge_type: SonarEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── PROPAGATION MODEL NODE ──
    if let Some(ref prop) = analysis.propagation_model {
        let prop_id = node_id;
        nodes.push(SonarGraphNode {
            node_id: prop_id,
            node_type: SonarNodeType::PropagationModelNode,
            content: format!("Propagation model: {}Hz {} SVP points {} CZ {} shadow zones",
                prop.frequency_hz, prop.sound_speed_profile.len(), prop.convergence_zones.len(), prop.shadow_zones.len()),
            frequency_hz: Some(prop.frequency_hz),
            materialized_path: Some(format!("/Modalities/Sonar/Project_{}/Graph_{}/PropModel", project_id, graph_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["propagation".into(), "sound-speed-profile".into()],
            hotness_score: 0.7, ..Default::default()
        });
        edges.push(SonarGraphEdge { edge_id, from_node: root_id, to_node: prop_id, edge_type: SonarEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Convergence zones
        for cz in &prop.convergence_zones {
            let cz_id = node_id + 1;
            nodes.push(SonarGraphNode {
                node_id: cz_id,
                node_type: SonarNodeType::ConvergenceZoneNode,
                content: format!("CZ {}: range={:.0}km width={:.1}km gain={:.1}dB", cz.zone_id, cz.center_range_km, cz.width_km, cz.gain_db),
                range_m: Some(cz.center_range_km * 1000.0),
                materialized_path: Some(format!("/Modalities/Sonar/Project_{}/Graph_{}/PropModel/CZ/{}", project_id, graph_id, cz.zone_id)),
                provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                keywords: vec!["convergence-zone".into()], hotness_score: 0.65, ..Default::default()
            });
            edges.push(SonarGraphEdge { edge_id, from_node: prop_id, to_node: cz_id, edge_type: SonarEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1; node_id += 1;
        }
        node_id += 1;
    }

    // ── TARGET NODES ──
    let target_node_map: HashMap<u64, u64> = analysis.targets.iter().map(|t| {
        let tid = node_id;
        nodes.push(SonarGraphNode {
            node_id: tid,
            node_type: SonarNodeType::SonarTarget,
            content: format!("Sonar target {:?}: range={:.0}m depth={:.1}m ts={:.1}dB vel={:.1}m/s",
                t.classification, t.range_m, t.depth_m, t.target_strength_db, t.radial_velocity_ms),
            range_m: Some(t.range_m), bearing_deg: Some(t.bearing_deg),
            depth_m: Some(t.depth_m), echo_strength_db: Some(t.target_strength_db),
            radial_velocity_ms: Some(t.radial_velocity_ms),
            timestamp: Some(t.detection_timestamp), geo_location: t.geo_location,
            classification: Some(format!("{:?}", t.classification)),
            materialized_path: Some(format!("/Modalities/Sonar/Project_{}/Graph_{}/Target/{}", project_id, graph_id, t.target_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["target".into(), format!("{:?}", t.classification).to_lowercase()],
            hotness_score: 0.8, ..Default::default()
        });
        node_id += 1;
        (t.target_id, tid)
    }).collect();

    for (_, tid) in &target_node_map {
        edges.push(SonarGraphEdge { edge_id, from_node: root_id, to_node: *tid, edge_type: SonarEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
    }

    // ── ACOUSTIC SIGNATURE NODES ──
    for sig in &analysis.acoustic_signatures {
        let sid = node_id;
        nodes.push(SonarGraphNode {
            node_id: sid,
            node_type: SonarNodeType::AcousticSignature,
            content: format!("Acoustic sig: {}Hz bw={:.0}Hz species={:?} material={:?}",
                sig.dominant_frequency_hz, sig.bandwidth_hz, sig.identified_species, sig.identified_material),
            frequency_hz: Some(sig.dominant_frequency_hz),
            species: sig.identified_species.clone(),
            materialized_path: Some(format!("/Modalities/Sonar/Project_{}/Graph_{}/Signature/{}", project_id, graph_id, sig.signature_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["signature".into(), "acoustic".into()],
            hotness_score: 0.75, ..Default::default()
        });
        // link signature → target
        if let Some(target_id) = sig.target_id {
            if let Some(&target_nid) = target_node_map.get(&target_id) {
                edges.push(SonarGraphEdge {
                    edge_id, from_node: target_nid, to_node: sid,
                    edge_type: SonarEdgeType::ReflectsWithAcousticSignature, weight: 1.0,
                    provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default()
                });
                edge_id += 1;
            }
        }
        edges.push(SonarGraphEdge { edge_id, from_node: root_id, to_node: sid, edge_type: SonarEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── BATHYMETRY NODE ──
    if let Some(ref bathy) = analysis.bathymetry {
        let bathy_root_id = node_id;
        nodes.push(SonarGraphNode {
            node_id: bathy_root_id,
            node_type: SonarNodeType::BathymetricNode,
            content: format!("Bathymetry: {:.1}km² res={:.1}m depth={:.0}–{:.0}m",
                bathy.coverage_area_km2, bathy.grid_resolution_m, bathy.min_depth_m, bathy.max_depth_m),
            depth_m: Some(bathy.mean_depth_m),
            materialized_path: Some(format!("/Modalities/Sonar/Project_{}/Graph_{}/Bathymetry", project_id, graph_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["bathymetry".into(), "seafloor".into()],
            hotness_score: 0.85, ..Default::default()
        });
        edges.push(SonarGraphEdge { edge_id, from_node: root_id, to_node: bathy_root_id, edge_type: SonarEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Bathymetric features
        for feat in &bathy.features {
            let fid = node_id + 1;
            nodes.push(SonarGraphNode {
                node_id: fid,
                node_type: SonarNodeType::BathymetricNode,
                content: format!("Bathy feature {:?}: depth={:.0}m relief={:.0}m",
                    feat.feature_type, feat.depth_m, feat.height_m),
                depth_m: Some(feat.depth_m),
                geo_location: Some((feat.location.0, feat.location.1, -(feat.depth_m as f64))),
                materialized_path: Some(format!("/Modalities/Sonar/Project_{}/Graph_{}/Bathymetry/Feature/{}", project_id, graph_id, feat.feature_id)),
                provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                keywords: vec!["bathymetric-feature".into(), format!("{:?}", feat.feature_type).to_lowercase()],
                hotness_score: 0.7, ..Default::default()
            });
            edges.push(SonarGraphEdge { edge_id, from_node: bathy_root_id, to_node: fid, edge_type: SonarEdgeType::BathymetryOf, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1; node_id += 1;
        }
        node_id += 1;
    }

    // ── SUB-BOTTOM LAYER NODES ──
    let mut prev_layer_nid: Option<u64> = None;
    for layer in &analysis.sub_bottom_layers {
        let lid = node_id;
        nodes.push(SonarGraphNode {
            node_id: lid,
            node_type: SonarNodeType::SubsurfaceLayer,
            content: format!("Sub-bottom layer: depth={:.1}–{:.1}m thick={:.1}m ref={:.1}dB mat={:?}",
                layer.top_depth_m, layer.bottom_depth_m, layer.thickness_m,
                layer.reflection_strength_db, layer.inferred_material),
            depth_m: Some(layer.top_depth_m),
            echo_strength_db: Some(layer.reflection_strength_db),
            materialized_path: Some(format!("/Modalities/Sonar/Project_{}/Graph_{}/SubBottom/{}", project_id, graph_id, layer.layer_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["sub-bottom".into(), "layer".into()],
            hotness_score: 0.65, ..Default::default()
        });
        edges.push(SonarGraphEdge { edge_id, from_node: root_id, to_node: lid, edge_type: SonarEdgeType::Contains, weight: 0.7, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // Layer ordering
        if let Some(prev_id) = prev_layer_nid {
            edges.push(SonarGraphEdge { edge_id, from_node: prev_id, to_node: lid, edge_type: SonarEdgeType::LayeredAbove, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        prev_layer_nid = Some(lid); node_id += 1;
    }

    // ── BIOLOGICAL DETECTION NODES ──
    for bio in &analysis.biological_detections {
        let bid = node_id;
        nodes.push(SonarGraphNode {
            node_id: bid,
            node_type: SonarNodeType::BiologicalNode,
            content: format!("Bio detection: species={:?} call_freq={:?}Hz bearing={:.1}°",
                bio.species_estimate, bio.call_frequency_hz, bio.bearing_deg),
            bearing_deg: Some(bio.bearing_deg), range_m: bio.range_m,
            depth_m: bio.depth_m, frequency_hz: bio.call_frequency_hz,
            timestamp: Some(bio.timestamp), species: bio.species_estimate.clone(),
            materialized_path: Some(format!("/Modalities/Sonar/Project_{}/Graph_{}/Bio/{}", project_id, graph_id, bio.detection_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: {
                let mut kw = vec!["biology".into(), "marine".into()];
                if let Some(ref sp) = bio.species_estimate { kw.push(sp.to_lowercase()); }
                kw
            },
            hotness_score: 0.75, ..Default::default()
        });
        edges.push(SonarGraphEdge { edge_id, from_node: root_id, to_node: bid, edge_type: SonarEdgeType::AssociatedBiology, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Cross-modal: biology → Sound Reconstruction (110)
        if bio.vocalization_type.is_some() {
            edges.push(SonarGraphEdge {
                edge_id, from_node: bid, to_node: bid,
                edge_type: SonarEdgeType::VocalizationLinksToSound, weight: 0.9,
                provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                properties: {
                    let mut p = HashMap::new();
                    p.insert("target_modality".into(), serde_json::json!("sound_reconstruction"));
                    p.insert("vocalization_type".into(), serde_json::json!(bio.vocalization_type));
                    p
                },
                ..Default::default()
            });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── FISH SCHOOL NODES ──
    for school in &analysis.fish_school_detections {
        let fid = node_id;
        nodes.push(SonarGraphNode {
            node_id: fid,
            node_type: SonarNodeType::FishSchoolNode,
            content: format!("Fish school: range={:.0}m depth={:.1}m sv={:.1}dB species={:?}",
                school.center_range_m, school.school_depth_m, school.mean_sv_db, school.species_estimate),
            range_m: Some(school.center_range_m), bearing_deg: Some(school.center_bearing_deg),
            depth_m: Some(school.school_depth_m), species: school.species_estimate.clone(),
            materialized_path: Some(format!("/Modalities/Sonar/Project_{}/Graph_{}/School/{}", project_id, graph_id, school.school_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["fish-school".into(), "biology".into()],
            hotness_score: 0.7, ..Default::default()
        });
        edges.push(SonarGraphEdge { edge_id, from_node: root_id, to_node: fid, edge_type: SonarEdgeType::AssociatedBiology, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── CURRENT VECTOR NODES ──
    for current in &analysis.current_vectors {
        let cvid = node_id;
        nodes.push(SonarGraphNode {
            node_id: cvid,
            node_type: SonarNodeType::CurrentVectorNode,
            content: format!("Current: depth={:.0}m spd={:.2}m/s dir={:.0}°",
                current.depth_m, current.speed_ms, current.direction_deg),
            depth_m: Some(current.depth_m), range_m: Some(current.range_m),
            radial_velocity_ms: Some(current.radial_velocity_ms), timestamp: Some(current.timestamp),
            materialized_path: Some(format!("/Modalities/Sonar/Project_{}/Graph_{}/Current/{}", project_id, graph_id, current.vector_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["current".into(), "adcp".into()], hotness_score: 0.6, ..Default::default()
        });
        edges.push(SonarGraphEdge { edge_id, from_node: root_id, to_node: cvid, edge_type: SonarEdgeType::Contains, weight: 0.7, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── 3D POINT CLOUD NODE ──
    if !analysis.acoustic_point_cloud.is_empty() {
        let pcl_id = node_id;
        nodes.push(SonarGraphNode {
            node_id: pcl_id,
            node_type: SonarNodeType::AcousticPoint3D,
            content: format!("Acoustic 3D point cloud: {} points", analysis.acoustic_point_cloud.len()),
            materialized_path: Some(format!("/Modalities/Sonar/Project_{}/Graph_{}/PointCloud", project_id, graph_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["point-cloud".into(), "3d-reconstruction".into()],
            hotness_score: 0.75, ..Default::default()
        });
        edges.push(SonarGraphEdge { edge_id, from_node: root_id, to_node: pcl_id, edge_type: SonarEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        // Cross-modal: 3D geometry (109)
        edges.push(SonarGraphEdge {
            edge_id: edge_id + 1, from_node: pcl_id, to_node: pcl_id,
            edge_type: SonarEdgeType::ContributesTo3DGeometry, weight: 0.9,
            provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
            properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("3d")); p },
            ..Default::default()
        });
        edge_id += 2; node_id += 1;
    }

    // ── HOOK 1: OnGraphCreated → save initial graph ──
    let _ = executor.save_graph(&SonarGraph {
        graph_id, project_id, source_description: analysis.source_description.clone(),
        nodes: nodes.clone(), edges: edges.clone(), root_node_id: root_id,
        state: GraphStateType::Created,
        state_history: vec![GraphStateTransition { from: GraphStateType::Created, to: GraphStateType::Created, timestamp: now.clone(), triggered_by_step: None }],
        created_at: now.clone(), updated_at: now.clone(), version: 1,
        version_notes: vec![VersionNote { version: 1, note: format!("Created: {} nodes {} edges", nodes.len(), edges.len()), step_index: None, timestamp: now.clone(), change_type: ChangeType::Created }],
    });

    // ── HOOK 2: OnInferRelationships ──
    let inferred = executor.infer_semantic_relationships(&nodes).await;
    let valid_ids: std::collections::HashSet<u64> = nodes.iter().map(|n| n.node_id).collect();
    for (from, to, etype, reason) in inferred {
        if valid_ids.contains(&from) && valid_ids.contains(&to) {
            edges.push(SonarGraphEdge {
                edge_id, from_node: from, to_node: to, edge_type: etype, weight: 0.8,
                provenance: EdgeProvenance::DerivedFromHook, version: 1,
                properties: { let mut p = HashMap::new(); p.insert("reason".into(), serde_json::json!(reason)); p },
                ..Default::default()
            });
            edge_id += 1;
        }
    }

    // ── HOOK 3: OnEdgeCompletion → hotness update ──
    let mut deg: HashMap<u64, u32> = HashMap::new();
    for e in &edges { *deg.entry(e.from_node).or_insert(0) += 1; *deg.entry(e.to_node).or_insert(0) += 1; }
    let max_deg = deg.values().copied().max().unwrap_or(1) as f32;
    for n in &mut nodes {
        if let Some(&d) = deg.get(&n.node_id) {
            n.hotness_score = (n.hotness_score + (d as f32 / max_deg) * 0.2).min(1.0);
        }
    }

    let final_graph = SonarGraph {
        graph_id, project_id, source_description: analysis.source_description,
        nodes, edges, root_node_id: root_id, state: GraphStateType::SemanticEnriched,
        state_history: vec![GraphStateTransition { from: GraphStateType::Created, to: GraphStateType::SemanticEnriched, timestamp: now.clone(), triggered_by_step: None }],
        created_at: now.clone(), updated_at: now.clone(), version: 1,
        version_notes: vec![VersionNote { version: 1, note: "Semantic enrichment complete".into(), step_index: None, timestamp: now, change_type: ChangeType::EnrichedBySemantic }],
    };
    let _ = executor.save_graph(&final_graph);
    SonarModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(final_graph), ..Default::default() }
}

// ─────────────────────────────────────────────────────────────────────────────
// MAIN EXECUTION
// ─────────────────────────────────────────────────────────────────────────────

pub async fn execute(input: SonarModalityAction) -> Result<SonarModalityOutput, String> {
    let executor = PipelineExecutor::new();

    match input {
        SonarModalityAction::Analyze { data, extract_targets, extract_bathymetry, extract_biology, extract_sub_bottom, extract_signatures } => {
            let analysis_id = executor.generate_id();
            let source_description = match &data {
                SonarDataSource::SingleBeamFile { file_path, frequency_hz, .. } =>
                    format!("Single-beam: {} {}kHz", file_path, frequency_hz / 1000.0),
                SonarDataSource::MultibeamFile { file_path, beam_count, swath_angle_deg, .. } =>
                    format!("Multibeam: {} {} beams {:.0}° swath", file_path, beam_count, swath_angle_deg),
                SonarDataSource::SideScanFile { file_path, frequency_hz, max_range_m, .. } =>
                    format!("Side-scan: {} {}kHz range={:.0}m", file_path, frequency_hz / 1000.0, max_range_m),
                SonarDataSource::SASFile { file_path, along_track_resolution_m, range_resolution_m, .. } =>
                    format!("SAS: {} res={}×{}m", file_path, along_track_resolution_m, range_resolution_m),
                SonarDataSource::SubBottomFile { file_path, frequency_hz, penetration_depth_m, .. } =>
                    format!("Sub-bottom: {} {}kHz pen={:.0}m", file_path, frequency_hz / 1000.0, penetration_depth_m),
                SonarDataSource::ADCPFile { file_path, bin_size_m, frequency_hz } =>
                    format!("ADCP: {} {}kHz bin={:.1}m", file_path, frequency_hz / 1000.0, bin_size_m),
                SonarDataSource::HydrophoneArray { files, .. } =>
                    format!("Hydrophone array: {} channels", files.len()),
                SonarDataSource::LiveStream { endpoint, sonar_type } =>
                    format!("Live {:?}: {}", sonar_type, endpoint),
            };

            Ok(SonarModalityOutput {
                success: true,
                analysis: Some(SonarAnalysisResult {
                    analysis_id, source_description, ..Default::default()
                }),
                ..Default::default()
            })
        }

        SonarModalityAction::CreateGraph { analysis, project_id } => {
            Ok(create_graph(&executor, analysis, project_id).await)
        }

        SonarModalityAction::UpdateGraph { graph_id, new_pings, project_id } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            let mut next_nid = graph.nodes.iter().map(|n| n.node_id).max().unwrap_or(0) + 1;
            let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
            let initial_count = graph.nodes.len();
            for ping in &new_pings {
                graph.nodes.push(SonarGraphNode {
                    node_id: next_nid, node_type: SonarNodeType::SonarEcho,
                    content: format!("Echo: bearing={:.1}° peak_range={:.0}m str={:.1}dB", ping.bearing_deg, ping.peak_range_m, ping.peak_echo_strength_db),
                    range_m: Some(ping.peak_range_m), bearing_deg: Some(ping.bearing_deg),
                    echo_strength_db: Some(ping.peak_echo_strength_db), timestamp: Some(ping.timestamp),
                    provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                    keywords: vec!["echo".into(), "ping".into()], hotness_score: 0.7, ..Default::default()
                });
                graph.edges.push(SonarGraphEdge { edge_id: next_eid, from_node: graph.root_node_id, to_node: next_nid, edge_type: SonarEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                next_eid += 1; next_nid += 1;
            }
            graph.version += 1; graph.updated_at = now.clone();
            graph.state = GraphStateType::Updated;
            graph.version_notes.push(VersionNote { version: graph.version, note: format!("Updated: {} new pings → {} nodes", new_pings.len(), graph.nodes.len() - initial_count), step_index: None, timestamp: now, change_type: ChangeType::Updated });
            executor.save_graph(&graph)?;
            Ok(SonarModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        SonarModalityAction::ProcessPing { raw_ping, transmit_freq_hz, transmit_level_db, pulse_length_ms, sample_rate_hz, water_sound_speed_ms } => {
            let range_resolution_m = water_sound_speed_ms * (1000.0 / sample_rate_hz) / 2.0;
            let n = raw_ping.len();
            let compressed: Vec<f32> = raw_ping.iter().enumerate().map(|(i, &v)| {
                let taper = (1.0 - (i as f32 / n as f32 - 0.5).powi(2) * 4.0).max(0.0);
                (v * v).sqrt() * taper
            }).collect();
            let (peak_idx, peak_val) = compressed.iter().enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, &v)| (i, v)).unwrap_or((0, 0.0));
            let echo_strength = if peak_val > 0.0 { 20.0 * peak_val.log10() } else { -999.0 };

            Ok(SonarModalityOutput {
                success: true,
                ping_result: Some(SonarPingResult {
                    ping_id: executor.generate_id(), range_samples: compressed,
                    bearing_deg: 0.0, range_resolution_m,
                    timestamp: 0.0, peak_echo_strength_db: echo_strength,
                    peak_range_m: peak_idx as f32 * range_resolution_m,
                }),
                ..Default::default()
            })
        }

        SonarModalityAction::FormSASImage { pings, platform_trajectory, range_resolution_m, along_track_resolution_m } => {
            let track_length_m = if platform_trajectory.len() > 1 {
                let first = &platform_trajectory[0];
                let last = platform_trajectory.last().unwrap();
                let dx = (last.lat - first.lat) * 111000.0;
                let dy = (last.lon - first.lon) * 111000.0;
                (dx * dx + dy * dy).sqrt() as f32
            } else { 0.0 };
            let along_track_pixels = (track_length_m / along_track_resolution_m) as u32;
            let max_range_m = pings.iter().map(|p| p.range_resolution_m * p.range_samples.len() as f32)
                .fold(0.0f32, f32::max);
            let range_pixels = (max_range_m / range_resolution_m) as u32;

            Ok(SonarModalityOutput {
                success: true,
                sas_image: Some(SASImageResult {
                    image_id: executor.generate_id(),
                    width_pixels: range_pixels.max(1), height_pixels: along_track_pixels.max(1),
                    along_track_resolution_m, range_resolution_m,
                    geo_extent: None, image_file_path: None, detected_objects: vec![],
                }),
                ..Default::default()
            })
        }

        SonarModalityAction::BeamformArray { channel_data, hydrophone_positions, steering_bearings_deg, freq_hz, sound_speed_ms } => {
            let n_elem = channel_data.len();
            let n_samples = channel_data.first().map(|c| c.len()).unwrap_or(0);
            let wavelength_m = sound_speed_ms / freq_hz;
            let mut beamformed = vec![vec![0.0f32; n_samples]; steering_bearings_deg.len()];
            for (bi, &bearing_deg) in steering_bearings_deg.iter().enumerate() {
                let bearing_rad = bearing_deg * std::f32::consts::PI / 180.0;
                let delays: Vec<f32> = hydrophone_positions.iter().map(|(x, y, _)| {
                    let projected = x * bearing_rad.sin() + y * bearing_rad.cos();
                    projected / sound_speed_ms
                }).collect();
                let max_delay_samples = delays.iter().map(|&d| (d * freq_hz).abs() as usize).max().unwrap_or(0);
                for si in 0..n_samples.saturating_sub(max_delay_samples) {
                    let sum: f32 = channel_data.iter().zip(delays.iter())
                        .map(|(ch, &d)| {
                            let offset = (d * freq_hz) as isize;
                            let idx = (si as isize + offset).clamp(0, (n_samples - 1) as isize) as usize;
                            ch[idx]
                        }).sum::<f32>() / n_elem as f32;
                    beamformed[bi][si] = sum;
                }
            }
            Ok(SonarModalityOutput { success: true, beamformed_data: Some(beamformed), ..Default::default() })
        }

        SonarModalityAction::BuildBathymetry { multibeam_pings, nav_data, coordinate_reference } => {
            let mut all_depths: Vec<f32> = Vec::new();
            for ping in &multibeam_pings {
                for beam in ping.beams.iter().filter(|b| b.valid) { all_depths.push(beam.depth_m); }
            }
            let min_d = all_depths.iter().cloned().fold(f32::INFINITY, f32::min);
            let max_d = all_depths.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let mean_d = if all_depths.is_empty() { 0.0 } else { all_depths.iter().sum::<f32>() / all_depths.len() as f32 };

            Ok(SonarModalityOutput {
                success: true,
                bathymetry: Some(BathymetryData {
                    data_id: executor.generate_id(),
                    grid_resolution_m: 1.0, coverage_area_km2: 0.01,
                    min_depth_m: min_d, max_depth_m: max_d, mean_depth_m: mean_d,
                    depth_grid: Vec::new(), geo_extent: None,
                    coordinate_reference, features: Vec::new(),
                }),
                ..Default::default()
            })
        }

        SonarModalityAction::ClassifySignature { echo_data, freq_hz, context } => {
            let n = echo_data.len();
            let mean = echo_data.iter().sum::<f32>() / n.max(1) as f32;
            let rms = (echo_data.iter().map(|v| v*v).sum::<f32>() / n.max(1) as f32).sqrt();
            let strength_db = if rms > 0.0 { 20.0 * rms.log10() } else { -60.0 };
            let sig_id = executor.generate_id();

            let (species, material) = match context {
                SignatureContext::MarineBiology => (Some("unknown_species".to_string()), None),
                SignatureContext::SedimentClassification => (None, Some("sediment".to_string())),
                SignatureContext::TargetDetection => (None, Some("unknown_material".to_string())),
                _ => (None, None),
            };

            Ok(SonarModalityOutput {
                success: true,
                signature_result: Some(AcousticSignature {
                    signature_id: sig_id, target_id: None,
                    frequency_spectrum: vec![(freq_hz, strength_db)],
                    target_strength_vs_aspect: Vec::new(),
                    dominant_frequency_hz: freq_hz, bandwidth_hz: freq_hz * 0.1,
                    identified_species: species, identified_material: material,
                    classification_notes: Vec::new(), reference_library_match: None,
                }),
                ..Default::default()
            })
        }

        SonarModalityAction::ModelPropagation { sound_speed_profile, bottom_type, surface_state, frequency_hz, source_depth_m, max_range_m } => {
            // Simplified ray-trace: compute TL at discrete ranges
            let mean_c = if sound_speed_profile.is_empty() { 1500.0f32 }
                else { sound_speed_profile.iter().map(|(_, c)| c).sum::<f32>() / sound_speed_profile.len() as f32 };
            let absorption_db_km = 0.003 * frequency_hz / 1000.0; // simplified
            let tl_vs_range: Vec<(f32, f32)> = (1..=50).map(|i| {
                let r = (max_range_m / 50.0) * i as f32;
                let spreading_loss = 20.0 * (r / 1.0f32).log10().max(0.0);
                let absorption_loss = absorption_db_km * r / 1000.0;
                (r, spreading_loss + absorption_loss)
            }).collect();

            Ok(SonarModalityOutput {
                success: true,
                propagation_model: Some(PropagationModel {
                    model_id: executor.generate_id(), frequency_hz,
                    sound_speed_profile, bottom_type, transmission_loss_db: tl_vs_range,
                    critical_angle_deg: (mean_c / 1500.0).asin() * 180.0 / std::f32::consts::PI,
                    convergence_zones: Vec::new(), shadow_zones: Vec::new(),
                }),
                ..Default::default()
            })
        }

        SonarModalityAction::QueryGraph { graph_id, query } => {
            let graph = executor.load_graph(graph_id)?;
            let result = match query {
                SonarGraphQuery::NodeDetail { node_id } => {
                    let node = graph.nodes.iter().find(|n| n.node_id == node_id);
                    let incoming: Vec<_> = graph.edges.iter().filter(|e| e.to_node == node_id).collect();
                    let outgoing: Vec<_> = graph.edges.iter().filter(|e| e.from_node == node_id).collect();
                    serde_json::json!({ "node": node, "incoming_edges": incoming, "outgoing_edges": outgoing })
                }
                SonarGraphQuery::TargetsByClassification { classification } => {
                    let t: Vec<_> = graph.nodes.iter().filter(|n| n.classification.as_deref() == Some(&classification)).collect();
                    serde_json::json!({ "targets": t })
                }
                SonarGraphQuery::BathymetricFeaturesInRegion { lat_min, lat_max, lon_min, lon_max } => {
                    let f: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, SonarNodeType::BathymetricNode)).filter(|n| n.geo_location.map(|(lat, lon, _)| lat >= lat_min && lat <= lat_max && lon >= lon_min && lon <= lon_max).unwrap_or(false)).collect();
                    serde_json::json!({ "features": f })
                }
                SonarGraphQuery::BiologicalDetections => {
                    let b: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, SonarNodeType::BiologicalNode | SonarNodeType::FishSchoolNode)).collect();
                    serde_json::json!({ "detections": b })
                }
                SonarGraphQuery::AcousticSignatures => {
                    let s: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, SonarNodeType::AcousticSignature)).collect();
                    serde_json::json!({ "signatures": s })
                }
                SonarGraphQuery::SubBottomLayers => {
                    let l: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, SonarNodeType::SubsurfaceLayer)).collect();
                    serde_json::json!({ "layers": l })
                }
                SonarGraphQuery::CrossModalLinks { node_id } => {
                    let links: Vec<_> = graph.edges.iter().filter(|e| (e.from_node == node_id || e.to_node == node_id) && matches!(e.edge_type, SonarEdgeType::ContributesTo3DGeometry | SonarEdgeType::LinksToMarineBiology | SonarEdgeType::VocalizationLinksToSound | SonarEdgeType::PlottedOnGeoMap | SonarEdgeType::FeedsAUVKinematics | SonarEdgeType::AffectedByThermalLayer | SonarEdgeType::CrossFusedWithRadar)).collect();
                    serde_json::json!({ "links": links })
                }
                SonarGraphQuery::AGIActivity => serde_json::json!({ "is_active": false }),
                SonarGraphQuery::AllNodes => serde_json::json!({ "nodes": graph.nodes }),
                SonarGraphQuery::AllEdges => serde_json::json!({ "edges": graph.edges }),
            };
            Ok(SonarModalityOutput { success: true, query_result: Some(result), ..Default::default() })
        }

        SonarModalityAction::GetGraph { graph_id } => {
            let graph = executor.load_graph(graph_id)?;
            Ok(SonarModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        SonarModalityAction::TriggerSemanticHook { graph_id, hook } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            match hook {
                SonarSemanticHook::OnGraphCreated => { graph.state = GraphStateType::SemanticEnriched; }
                SonarSemanticHook::OnInferRelationships => {
                    let new_edges = executor.infer_semantic_relationships(&graph.nodes).await;
                    let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                    let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                    for (from, to, etype, reason) in new_edges {
                        if valid.contains(&from) && valid.contains(&to) {
                            graph.edges.push(SonarGraphEdge { edge_id: next_eid, from_node: from, to_node: to, edge_type: etype, weight: 0.8, provenance: EdgeProvenance::DerivedFromHook, version: 1, properties: { let mut p = HashMap::new(); p.insert("reason".into(), serde_json::json!(reason)); p }, ..Default::default() });
                            next_eid += 1;
                        }
                    }
                }
                SonarSemanticHook::OnEdgeCompletion => {
                    let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                    graph.edges.retain(|e| valid.contains(&e.from_node) && valid.contains(&e.to_node));
                }
                SonarSemanticHook::OnCrossModalityLink { target_modality, target_graph_id } => {
                    graph.state = GraphStateType::CrossLinked;
                    graph.version += 1;
                    graph.version_notes.push(VersionNote { version: graph.version, note: format!("Cross-linked to {} (graph {})", target_modality, target_graph_id), step_index: None, timestamp: now.clone(), change_type: ChangeType::CrossLinked });
                }
            }
            graph.updated_at = now;
            executor.save_graph(&graph)?;
            Ok(SonarModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        SonarModalityAction::ExportProduct { graph_id, format } => {
            let export_path = format!("/tmp/sonar_export_{}_{:?}.dat", graph_id, format);
            Ok(SonarModalityOutput { success: true, export_path: Some(export_path), ..Default::default() })
        }

        SonarModalityAction::StreamToUI { graph_id, .. } => {
            Ok(SonarModalityOutput { success: true, graph_id: Some(graph_id), ..Default::default() })
        }

        SonarModalityAction::HeadlessProcess { graph_id, operations } => {
            let mut graph = executor.load_graph(graph_id)?;
            for op in operations {
                match op {
                    SonarOperation::ExportBathymetry => { /* export bathymetry */ }
                    SonarOperation::BuildMosaic { .. } => { /* build side-scan mosaic */ }
                    SonarOperation::RefineTargetClassification { target_id } => { /* refine */ }
                    SonarOperation::CrossMatchWithBiology { bio_graph_id } => {
                        let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        for node in graph.nodes.iter().filter(|n| matches!(n.node_type, SonarNodeType::BiologicalNode)) {
                            graph.edges.push(SonarGraphEdge { edge_id: next_eid, from_node: node.node_id, to_node: node.node_id, edge_type: SonarEdgeType::LinksToMarineBiology, weight: 0.9, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("bio_graph_id".into(), serde_json::json!(bio_graph_id)); p }, ..Default::default() });
                            next_eid += 1;
                        }
                    }
                    SonarOperation::UpdatePropagationModel => { /* re-run propagation */ }
                }
            }
            executor.save_graph(&graph)?;
            Ok(SonarModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--input" && i + 1 < args.len() { input_json = args[i + 1].clone(); i += 2; }
        else { i += 1; }
    }
    if input_json.is_empty() {
        eprintln!("Usage: sonar_sensing --input '<json>'");
        std::process::exit(1);
    }
    let input: SonarModalityAction = match serde_json::from_str(&input_json) {
        Ok(v) => v,
        Err(e) => { println!("{}", serde_json::json!({"success": false, "error": format!("Parse error: {}", e)})); std::process::exit(1); }
    };
    let rt = tokio::runtime::Runtime::new().expect("Tokio runtime");
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap_or_else(|_| r#"{"success":false,"error":"serialize"}"#.into())),
        Err(e) => { println!("{}", serde_json::json!({"success": false, "error": e})); std::process::exit(1); }
    }
}
