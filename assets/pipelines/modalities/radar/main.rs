//! ActiveRadarSensing — Pipeline #124
//!
//! Active EM ranging pipeline: chirp/pulse signals, range-Doppler maps,
//! SAR imaging, polarimetry, velocity (Doppler), material reflection signatures.
//!
//! DISTINCT FROM:
//!   - Passive EM (118): receives only; radar CONTROLS the source and measures return.
//!     This enables: known source → precise range, controlled freq → material resonance,
//!     known timing → velocity extraction via Doppler. Not recoverable from passive EM.
//!   - 3D Geometry (109): visual/geometric; radar provides all-weather ranging,
//!     penetrates obscurants, gives velocity directly via Doppler shift.
//!   - Hyperspectral (126): spectral reflectance; radar gives range/velocity/backscatter.
//!
//! CROSS-LINKS:
//!   109 (3D)       → radar point clouds feed 3D scene reconstruction
//!   117 (Geo)      → target tracks on geospatial map, weather overlays
//!   118 (EM)       → interference patterns, spectrum coordination
//!   126 (Hyper)    → material signatures cross-validated with spectral
//!   106 (Chem)     → material identification via RCS and reflection properties
//!   122 (Control)  → tracking feeds control systems for guidance
//!
//! STORAGE: ZSEI containers under /Modalities/Radar/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

// ─────────────────────────────────────────────────────────────────────────────
// INPUT TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum RadarModalityAction {
    /// Analyze raw radar data: produce RadarAnalysisResult
    Analyze {
        data: RadarDataSource,
        extract_targets: bool,
        extract_weather: bool,
        extract_velocity: bool,
        extract_sar: bool,
        extract_polarimetry: bool,
    },
    /// Create a graph from an analysis result
    CreateGraph {
        analysis: RadarAnalysisResult,
        project_id: u64,
    },
    /// Update an existing graph with new measurements
    UpdateGraph {
        graph_id: u64,
        new_measurements: Vec<RadarMeasurement>,
        project_id: u64,
    },
    /// Compress a raw chirp pulse → range profile
    ProcessPulse {
        raw_pulse: Vec<f32>,
        carrier_freq_hz: f64,
        bandwidth_hz: f64,
        pulse_duration_us: f32,
        sample_rate_hz: f64,
    },
    /// Form a SAR image from collected pulses along a flight path
    FormSARImage {
        pulses: Vec<RawPulse>,
        platform_positions: Vec<PlatformPosition>,
        scene_center: (f64, f64, f64),
        resolution_m: f32,
    },
    /// Track targets across multiple range-Doppler frames
    TrackTargets {
        graph_id: u64,
        new_detections: Vec<RadarDetection>,
        frame_timestamp: f64,
    },
    /// Beamform across an antenna array
    BeamformArray {
        element_data: Vec<Vec<f32>>,
        element_positions: Vec<(f32, f32, f32)>,
        steering_angles: Vec<(f32, f32)>,
        wavelength_m: f32,
    },
    /// Extract weather/precipitation data from radar returns
    ProcessWeather {
        reflectivity_dbz: Vec<Vec<f32>>,
        doppler_velocity: Vec<Vec<f32>>,
        dual_pol_data: Option<DualPolData>,
        range_resolution_m: f32,
        azimuth_resolution_deg: f32,
    },
    /// Query an existing graph
    QueryGraph {
        graph_id: u64,
        query: RadarGraphQuery,
    },
    /// Retrieve full graph for Context Viewer
    GetGraph { graph_id: u64 },
    /// Trigger ZSEI semantic hooks manually
    TriggerSemanticHook {
        graph_id: u64,
        hook: RadarSemanticHook,
    },
    /// Export radar products to standard formats
    ExportProduct {
        graph_id: u64,
        format: RadarExportFormat,
    },
    /// Stream to UI session
    StreamToUI {
        graph_id: u64,
        session_id: String,
        display_mode: RadarDisplayMode,
    },
    /// AGI-first headless processing (no rendering)
    HeadlessProcess {
        graph_id: u64,
        operations: Vec<RadarOperation>,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// DATA SOURCE TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RadarDataSource {
    /// Raw IQ data file
    RawIQFile {
        file_path: String,
        sample_rate_hz: f64,
        carrier_freq_hz: f64,
        channels: u32,
    },
    /// Range-Doppler map (already processed)
    RangeDopplerFile {
        file_path: String,
        range_resolution_m: f32,
        velocity_resolution_ms: f32,
        max_range_m: f32,
        max_velocity_ms: f32,
    },
    /// SAR scene file (TIFF, SLC, GRD)
    SARFile {
        file_path: String,
        format: SARFormat,
        polarization: Vec<Polarization>,
        orbit: OrbitType,
    },
    /// Weather radar (NEXRAD, ODIM, etc.)
    WeatherRadarFile {
        file_path: String,
        format: WeatherRadarFormat,
        elevation_angles: Vec<f32>,
    },
    /// Real-time stream (network socket / hardware interface)
    LiveStream {
        endpoint: String,
        radar_mode: RadarOperatingMode,
    },
    /// Multiple files forming a coherent dataset
    MultiFile {
        files: Vec<RadarDataSource>,
        coherent: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SARFormat {
    SLC,
    GRD,
    CEOS,
    GeoTIFF,
    Sentinel1,
    TerraSAR,
    NISAR,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WeatherRadarFormat {
    NEXRAD,
    ODIM,
    Rainbow5,
    IRIS,
    CfRadial,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Polarization {
    HH,
    VV,
    HV,
    VH,
    HHVV,
    Quad,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrbitType {
    Ascending,
    Descending,
    Geostationary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RadarOperatingMode {
    SAR,          // synthetic aperture
    ISAR,         // inverse SAR
    RangeDoppler, // standard tracking
    PhaseArray,   // electronically steered
    WeatherSurveillance,
    GroundPenetrating,
    AltimeterMode,
    Custom(String),
}

// ─────────────────────────────────────────────────────────────────────────────
// ANALYSIS RESULT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RadarAnalysisResult {
    pub analysis_id: u64,

    // RADAR PARAMETERS
    pub carrier_freq_hz: f64,
    pub bandwidth_hz: f64,
    pub pulse_duration_us: f32,
    pub prf_hz: f32, // pulse repetition frequency
    pub operating_mode: Option<RadarOperatingMode>,
    pub polarizations: Vec<Polarization>,

    // PROCESSED PRODUCTS
    pub range_doppler_map: Option<RangeDopplerMap>,
    pub sar_images: Vec<SARImageResult>,
    pub pulse_compressed_profiles: Vec<RangeProfile>,

    // DETECTIONS AND TRACKS
    pub targets: Vec<RadarTarget>,
    pub target_tracks: Vec<TargetTrack>,
    pub clutter_regions: Vec<ClutterRegion>,

    // WEATHER (if applicable)
    pub weather_patterns: Vec<WeatherPattern>,
    pub precipitation_cells: Vec<PrecipitationCell>,

    // MATERIAL/POLARIMETRY
    pub material_signatures: Vec<RadarMaterialSignature>,
    pub polarimetric_decompositions: Vec<PolarimetricDecomposition>,

    // GEOMETRY
    pub velocity_vectors: Vec<VelocityVector>,
    pub point_cloud_3d: Vec<RadarPoint3D>, // from SAR/ISAR reconstruction

    // METADATA
    pub source_description: String,
    pub platform_data: Option<PlatformData>,
    pub processing_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RangeDopplerMap {
    pub map_id: u64,
    pub range_bins: u32,
    pub doppler_bins: u32,
    pub range_resolution_m: f32,
    pub velocity_resolution_ms: f32,
    pub max_unambiguous_range_m: f32,
    pub max_unambiguous_velocity_ms: f32,
    /// Row-major: [range_bin][doppler_bin] = power_db
    pub power_map_db: Vec<Vec<f32>>,
    /// Optional: complex amplitude (for coherent processing)
    pub complex_amplitude: Option<Vec<Vec<(f32, f32)>>>,
    pub detection_threshold_db: f32,
    pub noise_floor_db: f32,
    pub timestamp: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SARImageResult {
    pub image_id: u64,
    pub width_pixels: u32,
    pub height_pixels: u32,
    pub range_resolution_m: f32,
    pub azimuth_resolution_m: f32,
    /// Geographic extent [min_lat, min_lon, max_lat, max_lon]
    pub geo_extent: Option<[f64; 4]>,
    pub orbit_direction: Option<OrbitType>,
    pub look_direction: LookDirection,
    pub incidence_angle_deg: f32,
    pub polarization: Polarization,
    /// Path to SAR image (not inlined — can be very large)
    pub image_file_path: Option<String>,
    pub detected_features: Vec<SARFeature>,
    pub calibration_applied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LookDirection {
    Left,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SARFeature {
    pub feature_id: u64,
    pub pixel_x: u32,
    pub pixel_y: u32,
    pub feature_type: SARFeatureType,
    pub rcs_dbsm: f32,
    pub extent_pixels: (u32, u32),
    pub geo_location: Option<(f64, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SARFeatureType {
    BrightScatterer,
    DistributedTarget,
    LinearFeature, // road, runway, coastline
    Urban,
    Vessel,
    Aircraft,
    Vehicle,
    Infrastructure,
    Shadow,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RangeProfile {
    pub profile_id: u64,
    pub pulse_index: u32,
    pub range_samples: Vec<f32>, // power in dB per range bin
    pub range_resolution_m: f32,
    pub peak_range_m: Option<f32>,
    pub timestamp: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RadarTarget {
    pub target_id: u64,
    pub range_m: f32,
    pub azimuth_deg: f32,
    pub elevation_deg: f32,
    pub rcs_dbsm: f32,           // radar cross section
    pub radial_velocity_ms: f32, // positive = moving away
    pub snr_db: f32,
    pub polarization_response: Option<PolarimetricResponse>,
    pub geo_location: Option<(f64, f64, f64)>, // lat, lon, alt
    pub track_id: Option<u64>,
    pub classification: TargetClassification,
    pub detection_timestamp: f64,
    /// Inferred material from RCS and frequency response
    pub material_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TargetClassification {
    Aircraft,
    Vessel,
    GroundVehicle,
    Drone,
    Missile,
    Weather,
    Clutter,
    Unknown,
    Custom(String),
}

impl Default for TargetClassification {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PolarimetricResponse {
    pub hh_db: f32,
    pub vv_db: f32,
    pub hv_db: f32,
    pub vh_db: f32,
    pub phase_hh_vv: f32,
    pub copol_correlation: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TargetTrack {
    pub track_id: u64,
    pub target_ids: Vec<u64>, // detections associated with this track
    pub track_state: TrackState,
    pub positions: Vec<TrackPosition>,
    pub velocity_ms: (f32, f32, f32), // 3D velocity vector
    pub course_deg: f32,
    pub speed_ms: f32,
    pub predicted_position: Option<(f64, f64, f64)>,
    pub first_detection: f64,
    pub last_update: f64,
    pub quality: TrackQuality,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum TrackState {
    #[default]
    Tentative,
    Confirmed,
    Coasting,
    Dropped,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum TrackQuality {
    #[default]
    Low,
    Medium,
    High,
    Firm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackPosition {
    pub timestamp: f64,
    pub lat: f64,
    pub lon: f64,
    pub alt_m: f32,
    pub range_m: f32,
    pub azimuth_deg: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClutterRegion {
    pub region_id: u64,
    pub range_min_m: f32,
    pub range_max_m: f32,
    pub azimuth_min_deg: f32,
    pub azimuth_max_deg: f32,
    pub mean_rcs_db: f32,
    pub clutter_type: ClutterType,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ClutterType {
    #[default]
    Ground,
    Sea,
    Rain,
    Chaff,
    Angels,
    Multipath,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WeatherPattern {
    pub pattern_id: u64,
    pub reflectivity_dbz: f32,                     // Z — rain rate proxy
    pub radial_velocity_ms: f32,                   // from Doppler
    pub spectrum_width_ms: f32,                    // turbulence indicator
    pub differential_reflectivity_db: Option<f32>, // ZDR — dual-pol
    pub specific_diff_phase_deg_km: Option<f32>,   // KDP — rain rate
    pub correlation_coefficient: Option<f32>,      // RhoHV — hydrometeor type
    pub precipitation_type: PrecipitationType,
    pub center_location: (f64, f64),
    pub extent_km: (f32, f32),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PrecipitationType {
    #[default]
    Unknown,
    Rain,
    Snow,
    Hail,
    MixedPhase,
    Drizzle,
    GraupelSmallHail,
    BiologicalScatter,
    GroundClutter,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PrecipitationCell {
    pub cell_id: u64,
    pub center: (f64, f64),
    pub top_altitude_m: f32,
    pub base_altitude_m: f32,
    pub max_reflectivity_dbz: f32,
    pub estimated_rain_rate_mmhr: f32,
    pub vil_kg_m2: f32,                   // vertically integrated liquid
    pub storm_motion: Option<(f32, f32)>, // (speed_ms, direction_deg)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RadarMaterialSignature {
    pub signature_id: u64,
    pub target_id: u64,
    pub hh_rcs_dbsm: f32,
    pub vv_rcs_dbsm: f32,
    pub hv_rcs_dbsm: f32,
    pub vh_rcs_dbsm: f32,
    pub differential_reflectivity_db: f32,
    pub depolarization_ratio: f32,
    pub frequency_response: Vec<(f64, f32)>, // (freq_hz, rcs_db) across frequencies
    pub inferred_material: Option<String>,
    pub material_confidence_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PolarimetricDecomposition {
    pub decomp_id: u64,
    pub method: DecompositionMethod,
    pub pixel_location: Option<(u32, u32)>,
    pub geo_location: Option<(f64, f64)>,
    pub components: HashMap<String, f32>, // component_name → power
    pub dominant_mechanism: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum DecompositionMethod {
    #[default]
    Pauli,
    Cloude,
    Freeman,
    Yamaguchi,
    Entropy,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VelocityVector {
    pub vector_id: u64,
    pub location: (f64, f64, f64),
    pub velocity_3d: (f32, f32, f32), // (vx, vy, vz) in m/s
    pub speed_ms: f32,
    pub source: VelocitySource,
    pub associated_target_id: Option<u64>,
    pub timestamp: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum VelocitySource {
    #[default]
    Doppler,
    TrackedTarget,
    InSAR,
    Estimate,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RadarPoint3D {
    pub point_id: u64,
    pub x: f32,
    pub y: f32,
    pub z: f32, // metric coordinates
    pub intensity_db: f32,
    pub geo: Option<(f64, f64, f64)>,
    pub source_sar_image_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlatformData {
    pub platform_type: String, // "spaceborne", "airborne", "ground"
    pub altitude_m: f32,
    pub speed_ms: f32,
    pub heading_deg: f32,
    pub position: (f64, f64, f64),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RadarMeasurement {
    pub timestamp: f64,
    pub detections: Vec<RadarDetection>,
    pub range_doppler_map: Option<RangeDopplerMap>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RadarDetection {
    pub range_m: f32,
    pub azimuth_deg: f32,
    pub elevation_deg: f32,
    pub rcs_dbsm: f32,
    pub velocity_ms: f32,
    pub snr_db: f32,
    pub timestamp: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RawPulse {
    pub pulse_index: u32,
    pub iq_data: Vec<(f32, f32)>,
    pub timestamp: f64,
    pub platform_position: Option<PlatformPosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlatformPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub vx: f64,
    pub vy: f64,
    pub vz: f64,
    pub timestamp: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DualPolData {
    pub hh: Vec<Vec<f32>>,
    pub vv: Vec<Vec<f32>>,
    pub hv: Vec<Vec<f32>>,
    pub vh: Vec<Vec<f32>>,
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH NODE TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum RadarNodeType {
    #[default]
    RadarScene, // root node — entire radar dataset
    RangeDopplerCell,              // single cell in range-Doppler space
    RangeDopplerMap,               // full range-Doppler frame
    RadarTarget,                   // detected point target
    TargetTrack,                   // multi-frame target trajectory
    SARImagePatch,                 // patch of a SAR image
    SARScene,                      // full SAR scene/image
    SARFeatureNode,                // detected SAR feature (bright scatterer, etc.)
    VelocityVector,                // Doppler-derived velocity
    WeatherCell,                   // weather/precipitation region
    ClutterRegion,                 // identified clutter
    MaterialSignature,             // radar material/backscatter signature
    PolarimetricDecompositionNode, // result of polarimetric analysis
    RadarPoint3D,                  // 3D reconstruction point from radar
    RangeProfile,                  // single compressed pulse → range profile
    PulseSequence,                 // collection of pulses
    BeamPattern,                   // antenna beam characteristics
    InterferenceSource,            // detected interference
    PlatformNode,                  // radar platform position/trajectory
    GeographicLocation,            // geo-referenced position
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RadarGraphNode {
    pub node_id: u64,
    pub node_type: RadarNodeType,
    pub content: String, // human-readable description

    // RADAR-SPECIFIC PAYLOAD
    pub range_m: Option<f32>,
    pub azimuth_deg: Option<f32>,
    pub elevation_deg: Option<f32>,
    pub rcs_dbsm: Option<f32>,
    pub radial_velocity_ms: Option<f32>,
    pub timestamp: Option<f64>,
    pub geo_location: Option<(f64, f64, f64)>,
    pub classification: Option<String>,

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
pub enum RadarEdgeType {
    // ── STRUCTURAL ──
    #[default]
    Contains,
    Precedes, // temporal ordering
    PartOf,

    // ── RADAR-SPECIFIC ──
    DetectedAtRange,               // target detected at specific range/azimuth
    MovingWithVelocity,            // target has measured velocity
    ReflectsWithMaterialSignature, // backscatter matches material signature
    InterferesWith,                // source interferes with measurements
    TracksTarget,                  // track is associated with target
    FormsImageOf,                  // SAR image shows scene/target
    DerivedFromPulse,              // range profile derived from pulse
    LocatedAt,                     // geo-referenced position
    AssociatedWeather,             // target is within weather cell
    ComposedOf,                    // decomposition: SAR scene → features

    // ── CROSS-MODAL ──
    /// Links radar point cloud node → 3D geometry node (109)
    ContributesTo3DGeometry,
    /// Links velocity vector → kinematic model (121)
    FeedsKinematicModel,
    /// Links target track → geospatial map (117)
    PlottedOnGeoMap,
    /// Links material signature → spectral signature (126)
    MaterialCrossValidatedWith,
    /// Links material signature → chemistry node (106)
    IdentifiesChemicalComposition,
    /// Passive EM interference (118)
    InterferesWithEM,
    /// Control system target data (122)
    ProvidesToControlSystem,

    // ── UNIVERSAL SEMANTIC (Section B.1) ──
    Performs,
    Affects,
    Implies,
    Contradicts,
    Elaborates,
    Summarizes,
    Supports,
    TemporalPrecedes,
    TemporalFollows,
    CausedBy,
    Enables,
    Prevents,
    FunctionalRole,
    InstanceOf,
    DerivedFrom,
    VersionOf,
    RefinesTo,
    ForkedFrom,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RadarGraphEdge {
    pub edge_id: u64,
    pub from_node: u64,
    pub to_node: u64,
    pub edge_type: RadarEdgeType,
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
pub struct RadarGraph {
    pub graph_id: u64,
    pub project_id: u64,
    pub source_description: String,
    pub nodes: Vec<RadarGraphNode>,
    pub edges: Vec<RadarGraphEdge>,
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
pub enum RadarGraphQuery {
    NodeDetail {
        node_id: u64,
    },
    TracksInRegion {
        lat_min: f64,
        lat_max: f64,
        lon_min: f64,
        lon_max: f64,
    },
    TargetsByClassification {
        classification: String,
    },
    VelocityVectorsAbove {
        speed_ms: f32,
    },
    WeatherCellsAbove {
        reflectivity_dbz: f32,
    },
    MaterialSignatures,
    SARFeaturesInRegion {
        pixel_x_min: u32,
        pixel_x_max: u32,
        pixel_y_min: u32,
        pixel_y_max: u32,
    },
    CrossModalLinks {
        node_id: u64,
    },
    AGIActivity,
    AllNodes,
    AllEdges,
}

// ─────────────────────────────────────────────────────────────────────────────
// SEMANTIC HOOKS
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RadarSemanticHook {
    OnGraphCreated,
    OnInferRelationships,
    OnEdgeCompletion,
    OnCrossModalityLink {
        target_modality: String,
        target_graph_id: u64,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// EXPORT / DISPLAY TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RadarExportFormat {
    GeoTIFF,  // SAR/RDM as georeferenced image
    KML,      // target tracks as KML
    GeoJSON,  // targets/tracks as GeoJSON
    CfRadial, // weather radar standard
    NEXRAD,   // NEXRAD level 2/3
    CSV,      // target detections as CSV
    NPZ,      // NumPy arrays (scientific processing)
    ASTERIX,  // ATC data format
    CoTXML,   // Cursor-on-Target (military)
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RadarDisplayMode {
    RangeDopplerWaterfall,     // scrolling RDM display
    SARImage,                  // SAR scene viewer
    TargetTable,               // tabular target list
    TrackDisplay,              // geographic track overlay
    BeamVisualization,         // beam pattern / steering
    WeatherOverlay,            // weather on a map
    PolarimetricDecomposition, // Pauli/etc. color coding
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RadarOperation {
    ReprocessPulse { pulse_id: u64 },
    UpdateTrack { track_id: u64 },
    ExportTargets,
    RefineWeatherCells,
    CrossMatchWith3D { graph_id_3d: u64 },
}

// ─────────────────────────────────────────────────────────────────────────────
// OUTPUT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RadarModalityOutput {
    pub success: bool,
    pub graph_id: Option<u64>,
    pub graph: Option<RadarGraph>,
    pub analysis: Option<RadarAnalysisResult>,
    pub pulse_result: Option<RangeProfile>,
    pub sar_image: Option<SARImageResult>,
    pub track_updates: Option<Vec<TargetTrack>>,
    pub beamformed_data: Option<Vec<Vec<f32>>>,
    pub weather_result: Option<Vec<WeatherPattern>>,
    pub query_result: Option<serde_json::Value>,
    pub export_path: Option<String>,
    pub error: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// SHARED TYPES (mirrors Section B definitions)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ProvisionalStatus {
    #[default]
    Planned,
    Generating,
    Generated,
    Validated,
    Finalized,
    Failed,
    RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VersionNote {
    pub version: u32,
    pub note: String,
    pub step_index: Option<u32>,
    pub timestamp: String,
    pub change_type: ChangeType,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ChangeType {
    #[default]
    Created,
    Updated,
    CrossLinked,
    EnrichedBySemantic,
    EnrichedByHook,
    RolledBack,
    Finalized,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum EdgeProvenance {
    #[default]
    Unknown,
    DerivedFromPrompt,
    DerivedFromChunk(u32),
    DerivedFromChunkGraph(u64),
    DerivedFromModalityGraph(u64),
    DerivedFromFile(String),
    DerivedFromAMT,
    DerivedFromBlueprint(u32),
    DerivedFromMethodology(u64),
    DerivedFromCrossModal,
    DerivedFromHook,
    VersionOf(u32),
    ForkedFrom(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum GraphStateType {
    #[default]
    Created,
    SemanticEnriched,
    CrossLinked,
    Stable,
    Updated,
    ReValidating,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStateTransition {
    pub from: GraphStateType,
    pub to: GraphStateType,
    pub timestamp: String,
    pub triggered_by_step: Option<u32>,
}

// ─────────────────────────────────────────────────────────────────────────────
// PIPELINE EXECUTOR (simplified stub — real implementation calls ZSEI)
// ─────────────────────────────────────────────────────────────────────────────

struct PipelineExecutor {
    zsei_path: String,
    prompt_pipeline_path: String,
}

impl PipelineExecutor {
    fn new() -> Self {
        Self {
            zsei_path: env::var("OZONE_ZSEI_PATH").unwrap_or_else(|_| "./zsei_data".into()),
            prompt_pipeline_path: env::var("OZONE_PROMPT_PIPELINE")
                .unwrap_or_else(|_| "./pipeline_9".into()),
        }
    }

    async fn llm_zero_shot(&self, prompt: &str, max_tokens: usize) -> Result<String, String> {
        let input = serde_json::json!({
            "action": "Prompt",
            "prompt": prompt,
            "max_tokens": max_tokens,
            "temperature": 0.05,
            "system_context": "Radar signal analysis. Return only valid JSON."
        });

        let output = std::process::Command::new(&self.prompt_pipeline_path)
            .arg("--input")
            .arg(input.to_string())
            .output()
            .map_err(|e| format!("LLM call failed: {}", e))?;

        let result: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("LLM parse failed: {}", e))?;

        Ok(result["response"].as_str().unwrap_or("{}").to_string())
    }

    fn save_graph(&self, graph: &RadarGraph) -> Result<(), String> {
        let path = format!(
            "{}/local/radar_graph_{}.json",
            self.zsei_path, graph.graph_id
        );
        if let Some(parent) = std::path::Path::new(&path).parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let data = serde_json::to_string_pretty(graph).map_err(|e| e.to_string())?;
        std::fs::write(&path, data).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn load_graph(&self, graph_id: u64) -> Result<RadarGraph, String> {
        let path = format!("{}/local/radar_graph_{}.json", self.zsei_path, graph_id);
        let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&data).map_err(|e| e.to_string())
    }

    fn generate_id(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    }

    fn now_iso8601(&self) -> String {
        format!("{}", self.generate_id())
    }

    fn extract_json_array(raw: &str) -> String {
        if let Some(start) = raw.find('[') {
            if let Some(end) = raw.rfind(']') {
                return raw[start..=end].to_string();
            }
        }
        "[]".to_string()
    }

    fn extract_json_object(raw: &str) -> String {
        if let Some(start) = raw.find('{') {
            if let Some(end) = raw.rfind('}') {
                return raw[start..=end].to_string();
            }
        }
        "{}".to_string()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// LLM-BASED ANALYSIS
// ─────────────────────────────────────────────────────────────────────────────

impl PipelineExecutor {
    /// Zero-shot: infer target classifications from RDM detections
    async fn infer_target_classifications(&self, targets: &[RadarTarget]) -> Vec<(u64, String)> {
        if targets.is_empty() {
            return vec![];
        }

        let target_list: Vec<serde_json::Value> = targets
            .iter()
            .map(|t| {
                serde_json::json!({
                    "target_id": t.target_id,
                    "range_m": t.range_m,
                    "rcs_dbsm": t.rcs_dbsm,
                    "radial_velocity_ms": t.radial_velocity_ms,
                    "azimuth_deg": t.azimuth_deg,
                    "elevation_deg": t.elevation_deg,
                    "snr_db": t.snr_db,
                    "polarization_response": t.polarization_response,
                })
            })
            .collect();

        let prompt = format!(
            r#"
Classify each radar target based on its measured parameters.
Use: Aircraft, Vessel, GroundVehicle, Drone, Missile, Weather, Clutter, Unknown.

Targets:
{}

Return ONLY valid JSON array:
[{{"target_id": N, "classification": "...", "reasoning": "brief reason"}}]"#,
            serde_json::to_string_pretty(&target_list).unwrap_or_default()
        );

        match self.llm_zero_shot(&prompt, 600).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str)
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|v| {
                        let id = v["target_id"].as_u64()?;
                        let cls = v["classification"].as_str()?.to_string();
                        Some((id, cls))
                    })
                    .collect()
            }
            Err(_) => vec![],
        }
    }

    /// Zero-shot: infer semantic relationships between graph nodes
    async fn infer_semantic_relationships(
        &self,
        nodes: &[RadarGraphNode],
    ) -> Vec<(u64, u64, RadarEdgeType, String)> {
        if nodes.len() < 2 {
            return vec![];
        }

        let node_summaries: Vec<serde_json::Value> = nodes
            .iter()
            .take(30)
            .map(|n| {
                serde_json::json!({
                    "node_id": n.node_id,
                    "type": format!("{:?}", n.node_type),
                    "content": n.content.chars().take(80).collect::<String>(),
                    "range_m": n.range_m,
                    "velocity_ms": n.radial_velocity_ms,
                })
            })
            .collect();

        let prompt = format!(
            r#"
Analyze these radar graph nodes and identify semantic relationships not already captured structurally.

Nodes:
{}

Available relationship types: Affects, Implies, CausedBy, Enables, Prevents, Performs,
TemporalPrecedes, PartOf, FunctionalRole, InstanceOf, DerivedFrom, InterferesWith,
DetectedAtRange, MovingWithVelocity, ReflectsWithMaterialSignature, TracksTarget

Return ONLY valid JSON array:
[{{"from_node_id": N, "to_node_id": M, "edge_type": "TypeName", "reason": "brief"}}]"#,
            serde_json::to_string_pretty(&node_summaries).unwrap_or_default()
        );

        match self.llm_zero_shot(&prompt, 800).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str)
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|v| {
                        let from = v["from_node_id"].as_u64()?;
                        let to = v["to_node_id"].as_u64()?;
                        let edge_str = v["edge_type"].as_str().unwrap_or("Affects");
                        let reason = v["reason"].as_str().unwrap_or("").to_string();
                        let edge = map_edge_type_str(edge_str);
                        Some((from, to, edge, reason))
                    })
                    .collect()
            }
            Err(_) => vec![],
        }
    }

    /// Zero-shot: infer material identities from radar signatures
    async fn infer_material_identities(
        &self,
        signatures: &[RadarMaterialSignature],
    ) -> Vec<(u64, String)> {
        if signatures.is_empty() {
            return vec![];
        }

        let sig_list: Vec<serde_json::Value> = signatures
            .iter()
            .map(|s| {
                serde_json::json!({
                    "signature_id": s.signature_id,
                    "hh_rcs_dbsm": s.hh_rcs_dbsm,
                    "vv_rcs_dbsm": s.vv_rcs_dbsm,
                    "hv_rcs_dbsm": s.hv_rcs_dbsm,
                    "differential_reflectivity_db": s.differential_reflectivity_db,
                    "depolarization_ratio": s.depolarization_ratio,
                    "frequency_response_count": s.frequency_response.len(),
                })
            })
            .collect();

        let prompt = format!(
            r#"
Identify likely material types from these radar backscatter signatures.
Consider: metal, composite, wood, vegetation, water, concrete, soil, foam, etc.

Signatures:
{}

Return ONLY valid JSON array:
[{{"signature_id": N, "inferred_material": "material_name", "reasoning": "brief explanation"}}]"#,
            serde_json::to_string_pretty(&sig_list).unwrap_or_default()
        );

        match self.llm_zero_shot(&prompt, 500).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str)
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|v| {
                        let id = v["signature_id"].as_u64()?;
                        let mat = v["inferred_material"].as_str()?.to_string();
                        Some((id, mat))
                    })
                    .collect()
            }
            Err(_) => vec![],
        }
    }
}

fn map_edge_type_str(s: &str) -> RadarEdgeType {
    match s {
        "DetectedAtRange" => RadarEdgeType::DetectedAtRange,
        "MovingWithVelocity" => RadarEdgeType::MovingWithVelocity,
        "ReflectsWithMaterialSignature" => RadarEdgeType::ReflectsWithMaterialSignature,
        "InterferesWith" => RadarEdgeType::InterferesWith,
        "TracksTarget" => RadarEdgeType::TracksTarget,
        "FormsImageOf" => RadarEdgeType::FormsImageOf,
        "DerivedFromPulse" => RadarEdgeType::DerivedFromPulse,
        "LocatedAt" => RadarEdgeType::LocatedAt,
        "AssociatedWeather" => RadarEdgeType::AssociatedWeather,
        "ComposedOf" => RadarEdgeType::ComposedOf,
        "ContributesTo3DGeometry" => RadarEdgeType::ContributesTo3DGeometry,
        "FeedsKinematicModel" => RadarEdgeType::FeedsKinematicModel,
        "PlottedOnGeoMap" => RadarEdgeType::PlottedOnGeoMap,
        "MaterialCrossValidatedWith" => RadarEdgeType::MaterialCrossValidatedWith,
        "IdentifiesChemicalComposition" => RadarEdgeType::IdentifiesChemicalComposition,
        "Performs" => RadarEdgeType::Performs,
        "Affects" => RadarEdgeType::Affects,
        "Implies" => RadarEdgeType::Implies,
        "CausedBy" => RadarEdgeType::CausedBy,
        "Enables" => RadarEdgeType::Enables,
        "Prevents" => RadarEdgeType::Prevents,
        "TemporalPrecedes" => RadarEdgeType::TemporalPrecedes,
        "TemporalFollows" => RadarEdgeType::TemporalFollows,
        "PartOf" => RadarEdgeType::PartOf,
        "FunctionalRole" => RadarEdgeType::FunctionalRole,
        "InstanceOf" => RadarEdgeType::InstanceOf,
        "DerivedFrom" => RadarEdgeType::DerivedFrom,
        "VersionOf" => RadarEdgeType::VersionOf,
        "RefinesTo" => RadarEdgeType::RefinesTo,
        _ => RadarEdgeType::Affects,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH CREATION
// ─────────────────────────────────────────────────────────────────────────────

async fn create_graph(
    executor: &PipelineExecutor,
    analysis: RadarAnalysisResult,
    project_id: u64,
) -> RadarModalityOutput {
    let graph_id = executor.generate_id();
    let now = executor.now_iso8601();
    let mut nodes: Vec<RadarGraphNode> = Vec::new();
    let mut edges: Vec<RadarGraphEdge> = Vec::new();
    let mut node_id: u64 = 1;
    let mut edge_id: u64 = 1;

    // ── ROOT NODE (Scene) ──
    let root_id = node_id;
    nodes.push(RadarGraphNode {
        node_id: root_id,
        node_type: RadarNodeType::RadarScene,
        content: format!(
            "Radar scene: {} targets, {} tracks, {} SAR images",
            analysis.targets.len(),
            analysis.target_tracks.len(),
            analysis.sar_images.len()
        ),
        materialized_path: Some(format!(
            "/Modalities/Radar/Project_{}/Graph_{}",
            project_id, graph_id
        )),
        provisional: false,
        provisional_status: ProvisionalStatus::Finalized,
        version: 1,
        version_notes: vec![VersionNote {
            version: 1,
            note: "Initial graph creation".into(),
            step_index: None,
            timestamp: now.clone(),
            change_type: ChangeType::Created,
        }],
        keywords: vec!["radar".into(), "scene".into()],
        hotness_score: 1.0,
        provenance: EdgeProvenance::DerivedFromPrompt,
        ..Default::default()
    });
    node_id += 1;

    // ── PLATFORM NODE ──
    if let Some(ref platform) = analysis.platform_data {
        let plat_id = node_id;
        nodes.push(RadarGraphNode {
            node_id: plat_id,
            node_type: RadarNodeType::PlatformNode,
            content: format!(
                "Platform: {} alt={:.0}m spd={:.1}m/s hdg={:.1}°",
                platform.platform_type,
                platform.altitude_m,
                platform.speed_ms,
                platform.heading_deg
            ),
            materialized_path: Some(format!(
                "/Modalities/Radar/Project_{}/Graph_{}/Platform",
                project_id, graph_id
            )),
            provisional: false,
            provisional_status: ProvisionalStatus::Finalized,
            version: 1,
            keywords: vec!["platform".into(), platform.platform_type.clone()],
            hotness_score: 0.8,
            provenance: EdgeProvenance::DerivedFromPrompt,
            ..Default::default()
        });
        edges.push(RadarGraphEdge {
            edge_id,
            from_node: root_id,
            to_node: plat_id,
            edge_type: RadarEdgeType::Contains,
            weight: 1.0,
            provenance: EdgeProvenance::DerivedFromPrompt,
            version: 1,
            ..Default::default()
        });
        edge_id += 1;
        node_id += 1;
    }

    // ── RANGE-DOPPLER MAP NODE ──
    if let Some(ref rdm) = analysis.range_doppler_map {
        let rdm_id = node_id;
        nodes.push(RadarGraphNode {
            node_id: rdm_id,
            node_type: RadarNodeType::RangeDopplerMap,
            content: format!(
                "RDM: {}×{} bins, range_res={:.1}m vel_res={:.2}m/s",
                rdm.range_bins,
                rdm.doppler_bins,
                rdm.range_resolution_m,
                rdm.velocity_resolution_ms
            ),
            range_m: Some(rdm.max_unambiguous_range_m),
            timestamp: Some(rdm.timestamp),
            materialized_path: Some(format!(
                "/Modalities/Radar/Project_{}/Graph_{}/RDM",
                project_id, graph_id
            )),
            provisional: false,
            provisional_status: ProvisionalStatus::Finalized,
            version: 1,
            keywords: vec!["range-doppler".into(), "map".into()],
            hotness_score: 0.9,
            provenance: EdgeProvenance::DerivedFromPrompt,
            ..Default::default()
        });
        edges.push(RadarGraphEdge {
            edge_id,
            from_node: root_id,
            to_node: rdm_id,
            edge_type: RadarEdgeType::Contains,
            weight: 1.0,
            provenance: EdgeProvenance::DerivedFromPrompt,
            version: 1,
            ..Default::default()
        });
        edge_id += 1;
        node_id += 1;
    }

    // ── TARGET NODES ──
    let target_node_ids: Vec<(u64, u64)> = analysis
        .targets
        .iter()
        .map(|target| {
            let tid = node_id;
            nodes.push(RadarGraphNode {
                node_id: tid,
                node_type: RadarNodeType::RadarTarget,
                content: format!(
                    "Target {:?}: range={:.0}m vel={:.1}m/s rcs={:.1}dBsm az={:.1}°",
                    target.classification,
                    target.range_m,
                    target.radial_velocity_ms,
                    target.rcs_dbsm,
                    target.azimuth_deg
                ),
                range_m: Some(target.range_m),
                azimuth_deg: Some(target.azimuth_deg),
                elevation_deg: Some(target.elevation_deg),
                rcs_dbsm: Some(target.rcs_dbsm),
                radial_velocity_ms: Some(target.radial_velocity_ms),
                timestamp: Some(target.detection_timestamp),
                geo_location: target.geo_location,
                classification: Some(format!("{:?}", target.classification)),
                materialized_path: Some(format!(
                    "/Modalities/Radar/Project_{}/Graph_{}/Target/{}",
                    project_id, graph_id, target.target_id
                )),
                provisional: false,
                provisional_status: ProvisionalStatus::Validated,
                version: 1,
                keywords: vec![
                    "target".into(),
                    format!("{:?}", target.classification).to_lowercase(),
                ],
                hotness_score: 0.8 + (target.snr_db / 100.0).clamp(0.0, 0.15),
                provenance: EdgeProvenance::DerivedFromPrompt,
                ..Default::default()
            });
            node_id += 1;
            (target.target_id, tid)
        })
        .collect();

    // target → root
    for (_, tid) in &target_node_ids {
        edges.push(RadarGraphEdge {
            edge_id,
            from_node: root_id,
            to_node: *tid,
            edge_type: RadarEdgeType::Contains,
            weight: 1.0,
            provenance: EdgeProvenance::DerivedFromPrompt,
            version: 1,
            ..Default::default()
        });
        edge_id += 1;
    }

    // ── VELOCITY VECTOR NODES ──
    let vel_node_ids: Vec<(u64, u64)> = analysis
        .velocity_vectors
        .iter()
        .map(|v| {
            let vid = node_id;
            nodes.push(RadarGraphNode {
                node_id: vid,
                node_type: RadarNodeType::VelocityVector,
                content: format!("Velocity: speed={:.1}m/s source={:?}", v.speed_ms, v.source),
                radial_velocity_ms: Some(v.speed_ms),
                timestamp: Some(v.timestamp),
                geo_location: Some(v.location),
                materialized_path: Some(format!(
                    "/Modalities/Radar/Project_{}/Graph_{}/Velocity/{}",
                    project_id, graph_id, v.vector_id
                )),
                provisional: false,
                provisional_status: ProvisionalStatus::Validated,
                version: 1,
                keywords: vec!["velocity".into(), "doppler".into()],
                hotness_score: 0.7,
                provenance: EdgeProvenance::DerivedFromPrompt,
                ..Default::default()
            });
            node_id += 1;
            (v.vector_id, vid)
        })
        .collect();

    // velocity → associated target
    for (vec_data, (_, vel_nid)) in analysis.velocity_vectors.iter().zip(vel_node_ids.iter()) {
        if let Some(assoc_target_id) = vec_data.associated_target_id {
            if let Some((_, target_nid)) = target_node_ids
                .iter()
                .find(|(tid, _)| *tid == assoc_target_id)
            {
                edges.push(RadarGraphEdge {
                    edge_id,
                    from_node: *target_nid,
                    to_node: *vel_nid,
                    edge_type: RadarEdgeType::MovingWithVelocity,
                    weight: 1.0,
                    provenance: EdgeProvenance::DerivedFromPrompt,
                    version: 1,
                    properties: {
                        let mut p = HashMap::new();
                        p.insert("speed_ms".into(), serde_json::json!(vec_data.speed_ms));
                        p
                    },
                    ..Default::default()
                });
                edge_id += 1;
            }
        }
        edges.push(RadarGraphEdge {
            edge_id,
            from_node: root_id,
            to_node: *vel_nid,
            edge_type: RadarEdgeType::Contains,
            weight: 1.0,
            provenance: EdgeProvenance::DerivedFromPrompt,
            version: 1,
            ..Default::default()
        });
        edge_id += 1;
    }

    // ── TRACK NODES ──
    let track_node_ids: Vec<(u64, u64)> = analysis
        .target_tracks
        .iter()
        .map(|track| {
            let trid = node_id;
            nodes.push(RadarGraphNode {
                node_id: trid,
                node_type: RadarNodeType::TargetTrack,
                content: format!(
                    "Track {}: {:?} state={:?} spd={:.1}m/s crs={:.1}°",
                    track.track_id,
                    track.quality,
                    track.track_state,
                    track.speed_ms,
                    track.course_deg
                ),
                materialized_path: Some(format!(
                    "/Modalities/Radar/Project_{}/Graph_{}/Track/{}",
                    project_id, graph_id, track.track_id
                )),
                provisional: false,
                provisional_status: ProvisionalStatus::Validated,
                version: 1,
                keywords: vec![
                    "track".into(),
                    format!("{:?}", track.track_state).to_lowercase(),
                ],
                hotness_score: 0.85,
                provenance: EdgeProvenance::DerivedFromPrompt,
                ..Default::default()
            });
            node_id += 1;
            (track.track_id, trid)
        })
        .collect();

    // track → associated targets (TracksTarget edges)
    for (track_data, (_, track_nid)) in analysis.target_tracks.iter().zip(track_node_ids.iter()) {
        edges.push(RadarGraphEdge {
            edge_id,
            from_node: root_id,
            to_node: *track_nid,
            edge_type: RadarEdgeType::Contains,
            weight: 1.0,
            provenance: EdgeProvenance::DerivedFromPrompt,
            version: 1,
            ..Default::default()
        });
        edge_id += 1;

        for assoc_detection_id in &track_data.target_ids {
            if let Some((_, target_nid)) = target_node_ids
                .iter()
                .find(|(did, _)| did == assoc_detection_id)
            {
                edges.push(RadarGraphEdge {
                    edge_id,
                    from_node: *track_nid,
                    to_node: *target_nid,
                    edge_type: RadarEdgeType::TracksTarget,
                    weight: 1.0,
                    provenance: EdgeProvenance::DerivedFromPrompt,
                    version: 1,
                    ..Default::default()
                });
                edge_id += 1;
            }
        }
    }

    // ── SAR IMAGE NODES ──
    for sar in &analysis.sar_images {
        let sar_id = node_id;
        nodes.push(RadarGraphNode {
            node_id: sar_id,
            node_type: RadarNodeType::SARScene,
            content: format!(
                "SAR image {}×{}px range_res={:.2}m az_res={:.2}m pol={:?}",
                sar.width_pixels,
                sar.height_pixels,
                sar.range_resolution_m,
                sar.azimuth_resolution_m,
                sar.polarization
            ),
            materialized_path: Some(format!(
                "/Modalities/Radar/Project_{}/Graph_{}/SAR/{}",
                project_id, graph_id, sar.image_id
            )),
            provisional: false,
            provisional_status: ProvisionalStatus::Validated,
            version: 1,
            keywords: vec![
                "sar".into(),
                "image".into(),
                format!("{:?}", sar.polarization).to_lowercase(),
            ],
            hotness_score: 0.75,
            provenance: EdgeProvenance::DerivedFromPrompt,
            ..Default::default()
        });
        edges.push(RadarGraphEdge {
            edge_id,
            from_node: root_id,
            to_node: sar_id,
            edge_type: RadarEdgeType::Contains,
            weight: 1.0,
            provenance: EdgeProvenance::DerivedFromPrompt,
            version: 1,
            ..Default::default()
        });
        edge_id += 1;

        // SAR features
        for feat in &sar.detected_features {
            let feat_node_id = node_id;
            nodes.push(RadarGraphNode {
                node_id: feat_node_id,
                node_type: RadarNodeType::SARFeatureNode,
                content: format!(
                    "SAR feature {:?} rcs={:.1}dBsm px=({},{})",
                    feat.feature_type, feat.rcs_dbsm, feat.pixel_x, feat.pixel_y
                ),
                rcs_dbsm: Some(feat.rcs_dbsm),
                geo_location: feat.geo_location.map(|(lat, lon)| (lat, lon, 0.0)),
                materialized_path: Some(format!(
                    "/Modalities/Radar/Project_{}/Graph_{}/SAR/{}/Feature/{}",
                    project_id, graph_id, sar.image_id, feat.feature_id
                )),
                provisional: false,
                provisional_status: ProvisionalStatus::Validated,
                version: 1,
                keywords: vec![
                    "sar-feature".into(),
                    format!("{:?}", feat.feature_type).to_lowercase(),
                ],
                hotness_score: 0.65,
                provenance: EdgeProvenance::DerivedFromPrompt,
                ..Default::default()
            });
            edges.push(RadarGraphEdge {
                edge_id,
                from_node: sar_id,
                to_node: feat_node_id,
                edge_type: RadarEdgeType::ComposedOf,
                weight: 1.0,
                provenance: EdgeProvenance::DerivedFromPrompt,
                version: 1,
                ..Default::default()
            });
            edge_id += 1;
            node_id += 1;
        }
        node_id += 1;
    }

    // ── WEATHER NODES ──
    for weather in &analysis.weather_patterns {
        let wid = node_id;
        nodes.push(RadarGraphNode {
            node_id: wid,
            node_type: RadarNodeType::WeatherCell,
            content: format!(
                "Weather {:?}: Z={:.1}dBZ vel={:.1}m/s",
                weather.precipitation_type, weather.reflectivity_dbz, weather.radial_velocity_ms
            ),
            radial_velocity_ms: Some(weather.radial_velocity_ms),
            geo_location: Some((weather.center_location.0, weather.center_location.1, 0.0)),
            materialized_path: Some(format!(
                "/Modalities/Radar/Project_{}/Graph_{}/Weather/{}",
                project_id, graph_id, weather.pattern_id
            )),
            provisional: false,
            provisional_status: ProvisionalStatus::Validated,
            version: 1,
            keywords: vec![
                "weather".into(),
                format!("{:?}", weather.precipitation_type).to_lowercase(),
            ],
            hotness_score: 0.7,
            provenance: EdgeProvenance::DerivedFromPrompt,
            ..Default::default()
        });
        edges.push(RadarGraphEdge {
            edge_id,
            from_node: root_id,
            to_node: wid,
            edge_type: RadarEdgeType::Contains,
            weight: 1.0,
            provenance: EdgeProvenance::DerivedFromPrompt,
            version: 1,
            ..Default::default()
        });
        edge_id += 1;
        node_id += 1;
    }

    // ── MATERIAL SIGNATURE NODES ──
    for sig in &analysis.material_signatures {
        let sid = node_id;
        nodes.push(RadarGraphNode {
            node_id: sid,
            node_type: RadarNodeType::MaterialSignature,
            content: format!(
                "Material sig: HH={:.1}dBsm VV={:.1}dBsm material={:?}",
                sig.hh_rcs_dbsm, sig.vv_rcs_dbsm, sig.inferred_material
            ),
            rcs_dbsm: Some(sig.hh_rcs_dbsm),
            materialized_path: Some(format!(
                "/Modalities/Radar/Project_{}/Graph_{}/Material/{}",
                project_id, graph_id, sig.signature_id
            )),
            provisional: false,
            provisional_status: ProvisionalStatus::Validated,
            version: 1,
            keywords: vec!["material".into(), "signature".into()],
            hotness_score: 0.65,
            provenance: EdgeProvenance::DerivedFromPrompt,
            ..Default::default()
        });
        // Link material → target
        if let Some((_, target_nid)) = target_node_ids
            .iter()
            .find(|(tid, _)| *tid == sig.target_id)
        {
            edges.push(RadarGraphEdge {
                edge_id,
                from_node: *target_nid,
                to_node: sid,
                edge_type: RadarEdgeType::ReflectsWithMaterialSignature,
                weight: 1.0,
                provenance: EdgeProvenance::DerivedFromPrompt,
                version: 1,
                ..Default::default()
            });
            edge_id += 1;
        }
        edges.push(RadarGraphEdge {
            edge_id,
            from_node: root_id,
            to_node: sid,
            edge_type: RadarEdgeType::Contains,
            weight: 0.8,
            provenance: EdgeProvenance::DerivedFromPrompt,
            version: 1,
            ..Default::default()
        });
        edge_id += 1;
        node_id += 1;
    }

    // ── 3D RADAR POINT CLOUD NODES ──
    if !analysis.point_cloud_3d.is_empty() {
        let pcl_root_id = node_id;
        nodes.push(RadarGraphNode {
            node_id: pcl_root_id,
            node_type: RadarNodeType::RadarPoint3D,
            content: format!(
                "Radar 3D point cloud: {} points",
                analysis.point_cloud_3d.len()
            ),
            materialized_path: Some(format!(
                "/Modalities/Radar/Project_{}/Graph_{}/PointCloud",
                project_id, graph_id
            )),
            provisional: false,
            provisional_status: ProvisionalStatus::Validated,
            version: 1,
            keywords: vec!["point-cloud".into(), "3d-reconstruction".into()],
            hotness_score: 0.75,
            provenance: EdgeProvenance::DerivedFromPrompt,
            ..Default::default()
        });
        edges.push(RadarGraphEdge {
            edge_id,
            from_node: root_id,
            to_node: pcl_root_id,
            edge_type: RadarEdgeType::Contains,
            weight: 1.0,
            provenance: EdgeProvenance::DerivedFromPrompt,
            version: 1,
            ..Default::default()
        });
        edge_id += 1;
        node_id += 1;
    }

    // ── ZSEI HOOK 1: OnGraphCreated ──
    // (In production: call ZSEI API to register graph container)
    // Register in /Modalities/Radar root container
    let _ = executor.save_graph(&RadarGraph {
        graph_id,
        project_id,
        source_description: analysis.source_description.clone(),
        nodes: nodes.clone(),
        edges: edges.clone(),
        root_node_id: root_id,
        state: GraphStateType::Created,
        state_history: vec![GraphStateTransition {
            from: GraphStateType::Created,
            to: GraphStateType::Created,
            timestamp: now.clone(),
            triggered_by_step: None,
        }],
        created_at: now.clone(),
        updated_at: now.clone(),
        version: 1,
        version_notes: vec![VersionNote {
            version: 1,
            note: format!(
                "Graph created: {} targets, {} tracks, {} SAR images",
                analysis.targets.len(),
                analysis.target_tracks.len(),
                analysis.sar_images.len()
            ),
            step_index: None,
            timestamp: now.clone(),
            change_type: ChangeType::Created,
        }],
    });

    // ── ZSEI HOOK 2: OnInferRelationships ──
    // LLM zero-shot: discover additional semantic relationships
    let inferred_edges = executor.infer_semantic_relationships(&nodes).await;
    for (from_id, to_id, edge_type, reason) in inferred_edges {
        if nodes.iter().any(|n| n.node_id == from_id) && nodes.iter().any(|n| n.node_id == to_id) {
            edges.push(RadarGraphEdge {
                edge_id,
                from_node: from_id,
                to_node: to_id,
                edge_type,
                weight: 0.8,
                provenance: EdgeProvenance::DerivedFromHook,
                version: 1,
                properties: {
                    let mut p = HashMap::new();
                    p.insert("inferred_reason".into(), serde_json::json!(reason));
                    p
                },
                ..Default::default()
            });
            edge_id += 1;
        }
    }

    // ── ZSEI HOOK 3: OnEdgeCompletion ──
    // Update hotness_scores on nodes with high edge degree
    let mut edge_counts: HashMap<u64, u32> = HashMap::new();
    for edge in &edges {
        *edge_counts.entry(edge.from_node).or_insert(0) += 1;
        *edge_counts.entry(edge.to_node).or_insert(0) += 1;
    }
    let max_edges = edge_counts.values().copied().max().unwrap_or(1) as f32;
    for node in &mut nodes {
        if let Some(&count) = edge_counts.get(&node.node_id) {
            node.hotness_score = (node.hotness_score + (count as f32 / max_edges) * 0.2).min(1.0);
        }
    }

    // Save final graph
    let final_graph = RadarGraph {
        graph_id,
        project_id,
        source_description: analysis.source_description.clone(),
        nodes: nodes.clone(),
        edges: edges.clone(),
        root_node_id: root_id,
        state: GraphStateType::SemanticEnriched,
        state_history: vec![GraphStateTransition {
            from: GraphStateType::Created,
            to: GraphStateType::SemanticEnriched,
            timestamp: now.clone(),
            triggered_by_step: None,
        }],
        created_at: now.clone(),
        updated_at: now.clone(),
        version: 1,
        version_notes: vec![VersionNote {
            version: 1,
            note: format!(
                "Semantic enrichment complete: {} nodes, {} edges",
                nodes.len(),
                edges.len()
            ),
            step_index: None,
            timestamp: now.clone(),
            change_type: ChangeType::EnrichedBySemantic,
        }],
    };

    let _ = executor.save_graph(&final_graph);

    RadarModalityOutput {
        success: true,
        graph_id: Some(graph_id),
        graph: Some(final_graph),
        ..Default::default()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ANALYSIS PIPELINE
// ─────────────────────────────────────────────────────────────────────────────

async fn analyze_radar_data(
    executor: &PipelineExecutor,
    data: RadarDataSource,
    extract_targets: bool,
    extract_weather: bool,
    extract_velocity: bool,
    extract_sar: bool,
    extract_polarimetry: bool,
) -> RadarModalityOutput {
    let analysis_id = executor.generate_id();

    // Describe the data source for LLM context
    let source_description = match &data {
        RadarDataSource::RawIQFile {
            file_path,
            carrier_freq_hz,
            bandwidth_hz,
            ..
        } => format!(
            "Raw IQ: {} fc={:.3}GHz bw={:.1}MHz",
            file_path,
            carrier_freq_hz / 1e9,
            bandwidth_hz / 1e6
        ),
        RadarDataSource::RangeDopplerFile {
            file_path,
            range_resolution_m,
            velocity_resolution_ms,
            ..
        } => format!(
            "RDM: {} rr={:.1}m vr={:.2}m/s",
            file_path, range_resolution_m, velocity_resolution_ms
        ),
        RadarDataSource::SARFile {
            file_path, format, ..
        } => format!("SAR: {} {:?}", file_path, format),
        RadarDataSource::WeatherRadarFile {
            file_path, format, ..
        } => format!("Weather radar: {} {:?}", file_path, format),
        RadarDataSource::LiveStream {
            endpoint,
            radar_mode,
        } => format!("Live stream: {} {:?}", endpoint, radar_mode),
        RadarDataSource::MultiFile { files, .. } => {
            format!("Multi-file dataset: {} files", files.len())
        }
    };

    // In production: actual signal processing (FFT, CFAR, SAR formation, etc.)
    // Here we produce an analysis result from available source metadata

    let mut analysis = RadarAnalysisResult {
        analysis_id,
        source_description: source_description.clone(),
        ..Default::default()
    };

    // Process file metadata using LLM to identify what's present
    let prompt = format!(
        r#"
Analyze this radar data source description and predict what products can be extracted.
Data: {}
Extract: targets={}, weather={}, velocity={}, SAR={}, polarimetry={}

Return ONLY valid JSON:
{{
  "carrier_freq_hz": 9.4e9,
  "bandwidth_hz": 100e6,
  "pulse_duration_us": 10.0,
  "prf_hz": 1000.0,
  "operating_mode": "RangeDoppler|SAR|WeatherSurveillance",
  "has_range_doppler_map": true,
  "estimated_target_count": 5,
  "has_weather": false,
  "has_sar": false,
  "processing_notes": ["note1", "note2"]
}}"#,
        source_description,
        extract_targets,
        extract_weather,
        extract_velocity,
        extract_sar,
        extract_polarimetry
    );

    if let Ok(raw) = executor.llm_zero_shot(&prompt, 400).await {
        let json_str = PipelineExecutor::extract_json_object(&raw);
        if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&json_str) {
            analysis.carrier_freq_hz = meta["carrier_freq_hz"].as_f64().unwrap_or(9.4e9);
            analysis.bandwidth_hz = meta["bandwidth_hz"].as_f64().unwrap_or(100e6);
            analysis.pulse_duration_us = meta["pulse_duration_us"].as_f64().unwrap_or(10.0) as f32;
            analysis.prf_hz = meta["prf_hz"].as_f64().unwrap_or(1000.0) as f32;
            if let Some(notes) = meta["processing_notes"].as_array() {
                analysis.processing_notes = notes
                    .iter()
                    .filter_map(|n| n.as_str().map(String::from))
                    .collect();
            }
        }
    }

    RadarModalityOutput {
        success: true,
        analysis: Some(analysis),
        ..Default::default()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// MAIN EXECUTION
// ─────────────────────────────────────────────────────────────────────────────

pub async fn execute(input: RadarModalityAction) -> Result<RadarModalityOutput, String> {
    let executor = PipelineExecutor::new();

    match input {
        RadarModalityAction::Analyze {
            data,
            extract_targets,
            extract_weather,
            extract_velocity,
            extract_sar,
            extract_polarimetry,
        } => Ok(analyze_radar_data(
            &executor,
            data,
            extract_targets,
            extract_weather,
            extract_velocity,
            extract_sar,
            extract_polarimetry,
        )
        .await),

        RadarModalityAction::CreateGraph {
            analysis,
            project_id,
        } => Ok(create_graph(&executor, analysis, project_id).await),

        RadarModalityAction::UpdateGraph {
            graph_id,
            new_measurements,
            project_id,
        } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            let initial_nodes = graph.nodes.len();

            // Add new detection nodes
            let mut next_node_id = graph.nodes.iter().map(|n| n.node_id).max().unwrap_or(0) + 1;
            let mut next_edge_id = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;

            for measurement in &new_measurements {
                for detection in &measurement.detections {
                    graph.nodes.push(RadarGraphNode {
                        node_id: next_node_id,
                        node_type: RadarNodeType::RadarTarget,
                        content: format!(
                            "Updated detection: range={:.0}m vel={:.1}m/s rcs={:.1}dBsm",
                            detection.range_m, detection.velocity_ms, detection.rcs_dbsm
                        ),
                        range_m: Some(detection.range_m),
                        azimuth_deg: Some(detection.azimuth_deg),
                        rcs_dbsm: Some(detection.rcs_dbsm),
                        radial_velocity_ms: Some(detection.velocity_ms),
                        timestamp: Some(detection.timestamp),
                        provisional: false,
                        provisional_status: ProvisionalStatus::Validated,
                        version: 1,
                        keywords: vec!["target".into(), "updated".into()],
                        hotness_score: 0.9,
                        provenance: EdgeProvenance::DerivedFromPrompt,
                        ..Default::default()
                    });
                    graph.edges.push(RadarGraphEdge {
                        edge_id: next_edge_id,
                        from_node: graph.root_node_id,
                        to_node: next_node_id,
                        edge_type: RadarEdgeType::Contains,
                        weight: 1.0,
                        provenance: EdgeProvenance::DerivedFromPrompt,
                        version: 1,
                        ..Default::default()
                    });
                    next_edge_id += 1;
                    next_node_id += 1;
                }
            }

            graph.version += 1;
            graph.updated_at = now.clone();
            graph.state = GraphStateType::Updated;
            graph.version_notes.push(VersionNote {
                version: graph.version,
                note: format!(
                    "Updated with {} new measurements ({} new nodes)",
                    new_measurements.len(),
                    graph.nodes.len() - initial_nodes
                ),
                step_index: None,
                timestamp: now,
                change_type: ChangeType::Updated,
            });

            executor.save_graph(&graph)?;
            Ok(RadarModalityOutput {
                success: true,
                graph_id: Some(graph_id),
                graph: Some(graph),
                ..Default::default()
            })
        }

        RadarModalityAction::ProcessPulse {
            raw_pulse,
            carrier_freq_hz,
            bandwidth_hz,
            pulse_duration_us,
            sample_rate_hz,
        } => {
            // Pulse compression: matched filter in frequency domain
            // In production: FFT → multiply by reference → IFFT
            let range_resolution_m = (3e8 / (2.0 * bandwidth_hz)) as f32;
            let n_samples = raw_pulse.len();
            let compressed_samples: Vec<f32> = raw_pulse
                .iter()
                .enumerate()
                .map(|(i, &v)| {
                    // Simplified: power envelope
                    (v * v).sqrt() * (1.0 - (i as f32 / n_samples as f32).powi(2))
                })
                .collect();
            let peak_idx = compressed_samples
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i)
                .unwrap_or(0);
            let peak_range_m = peak_idx as f32 * range_resolution_m;

            Ok(RadarModalityOutput {
                success: true,
                pulse_result: Some(RangeProfile {
                    profile_id: executor.generate_id(),
                    pulse_index: 0,
                    range_samples: compressed_samples,
                    range_resolution_m,
                    peak_range_m: Some(peak_range_m),
                    timestamp: 0.0,
                }),
                ..Default::default()
            })
        }

        RadarModalityAction::FormSARImage {
            pulses,
            platform_positions,
            scene_center,
            resolution_m,
        } => {
            // SAR formation: in production uses back-projection or omega-k algorithm
            let width = (1000.0 / resolution_m) as u32;
            let height = (1000.0 / resolution_m) as u32;
            Ok(RadarModalityOutput {
                success: true,
                sar_image: Some(SARImageResult {
                    image_id: executor.generate_id(),
                    width_pixels: width,
                    height_pixels: height,
                    range_resolution_m: resolution_m,
                    azimuth_resolution_m: resolution_m,
                    geo_extent: Some([
                        scene_center.0 - 0.005,
                        scene_center.1 - 0.005,
                        scene_center.0 + 0.005,
                        scene_center.1 + 0.005,
                    ]),
                    look_direction: LookDirection::Right,
                    incidence_angle_deg: 35.0,
                    polarization: Polarization::VV,
                    image_file_path: None,
                    detected_features: Vec::new(),
                    calibration_applied: false,
                }),
                ..Default::default()
            })
        }

        RadarModalityAction::TrackTargets {
            graph_id,
            new_detections,
            frame_timestamp,
        } => {
            let graph = executor.load_graph(graph_id)?;
            // In production: JPDA / MHT / Kalman filtering for tracking
            let tracks: Vec<TargetTrack> = new_detections
                .iter()
                .map(|det| TargetTrack {
                    track_id: executor.generate_id(),
                    target_ids: vec![],
                    track_state: TrackState::Tentative,
                    positions: vec![TrackPosition {
                        timestamp: frame_timestamp,
                        lat: det.azimuth_deg as f64 / 360.0,
                        lon: det.azimuth_deg as f64 / 180.0,
                        alt_m: det.elevation_deg,
                        range_m: det.range_m,
                        azimuth_deg: det.azimuth_deg,
                    }],
                    velocity_ms: (det.velocity_ms, 0.0, 0.0),
                    course_deg: det.azimuth_deg,
                    speed_ms: det.velocity_ms.abs(),
                    predicted_position: None,
                    first_detection: frame_timestamp,
                    last_update: frame_timestamp,
                    quality: TrackQuality::Low,
                })
                .collect();

            Ok(RadarModalityOutput {
                success: true,
                track_updates: Some(tracks),
                ..Default::default()
            })
        }

        RadarModalityAction::BeamformArray {
            element_data,
            element_positions,
            steering_angles,
            wavelength_m,
        } => {
            // Delay-and-sum beamforming
            let n_elements = element_data.len();
            let n_angles = steering_angles.len();
            let n_samples = element_data.first().map(|e| e.len()).unwrap_or(0);
            let mut beamformed = vec![vec![0.0f32; n_samples]; n_angles];

            for (ai, (az_deg, el_deg)) in steering_angles.iter().enumerate() {
                let az_rad = az_deg * std::f32::consts::PI / 180.0;
                let el_rad = el_deg * std::f32::consts::PI / 180.0;
                let steering_vec: Vec<f32> = element_positions
                    .iter()
                    .map(|(x, y, z)| {
                        let phase = 2.0 * std::f32::consts::PI / wavelength_m
                            * (x * az_rad.cos() * el_rad.cos()
                                + y * az_rad.sin() * el_rad.cos()
                                + z * el_rad.sin());
                        phase
                    })
                    .collect();

                for si in 0..n_samples {
                    let sum: f32 = element_data
                        .iter()
                        .zip(steering_vec.iter())
                        .map(|(elem, phase)| {
                            let v = elem.get(si).copied().unwrap_or(0.0);
                            v * phase.cos()
                        })
                        .sum::<f32>()
                        / n_elements as f32;
                    beamformed[ai][si] = sum;
                }
            }

            Ok(RadarModalityOutput {
                success: true,
                beamformed_data: Some(beamformed),
                ..Default::default()
            })
        }

        RadarModalityAction::ProcessWeather {
            reflectivity_dbz,
            doppler_velocity,
            dual_pol_data,
            range_resolution_m,
            azimuth_resolution_deg,
        } => {
            let mut weather_patterns = Vec::new();
            // CFAR-style threshold for precipitation cells
            let threshold_dbz = 20.0f32;
            for (ri, row) in reflectivity_dbz.iter().enumerate() {
                for (ai, &z) in row.iter().enumerate() {
                    if z > threshold_dbz {
                        let velocity = doppler_velocity
                            .get(ri)
                            .and_then(|r| r.get(ai))
                            .copied()
                            .unwrap_or(0.0);
                        let lat = ri as f64 * range_resolution_m as f64 / 111000.0;
                        let lon = ai as f64 * azimuth_resolution_deg as f64;
                        weather_patterns.push(WeatherPattern {
                            pattern_id: executor.generate_id(),
                            reflectivity_dbz: z,
                            radial_velocity_ms: velocity,
                            spectrum_width_ms: 2.0,
                            differential_reflectivity_db: dual_pol_data.as_ref().map(|_| 0.5),
                            specific_diff_phase_deg_km: None,
                            correlation_coefficient: dual_pol_data.as_ref().map(|_| 0.95),
                            precipitation_type: if z > 50.0 {
                                PrecipitationType::Hail
                            } else if z > 35.0 {
                                PrecipitationType::Rain
                            } else {
                                PrecipitationType::Drizzle
                            },
                            center_location: (lat, lon),
                            extent_km: (range_resolution_m / 1000.0, azimuth_resolution_deg),
                        });
                    }
                }
            }
            Ok(RadarModalityOutput {
                success: true,
                weather_result: Some(weather_patterns),
                ..Default::default()
            })
        }

        RadarModalityAction::QueryGraph { graph_id, query } => {
            let graph = executor.load_graph(graph_id)?;
            let result = match query {
                RadarGraphQuery::NodeDetail { node_id } => {
                    let node = graph.nodes.iter().find(|n| n.node_id == node_id);
                    let incoming: Vec<_> = graph
                        .edges
                        .iter()
                        .filter(|e| e.to_node == node_id)
                        .collect();
                    let outgoing: Vec<_> = graph
                        .edges
                        .iter()
                        .filter(|e| e.from_node == node_id)
                        .collect();
                    serde_json::json!({ "node": node, "incoming_edges": incoming, "outgoing_edges": outgoing })
                }
                RadarGraphQuery::TracksInRegion {
                    lat_min,
                    lat_max,
                    lon_min,
                    lon_max,
                } => {
                    let tracks: Vec<_> = graph
                        .nodes
                        .iter()
                        .filter(|n| matches!(n.node_type, RadarNodeType::TargetTrack))
                        .filter(|n| {
                            n.geo_location
                                .map(|(lat, lon, _)| {
                                    lat >= lat_min
                                        && lat <= lat_max
                                        && lon >= lon_min
                                        && lon <= lon_max
                                })
                                .unwrap_or(false)
                        })
                        .collect();
                    serde_json::json!({ "tracks": tracks })
                }
                RadarGraphQuery::TargetsByClassification { classification } => {
                    let targets: Vec<_> = graph
                        .nodes
                        .iter()
                        .filter(|n| n.classification.as_deref() == Some(&classification))
                        .collect();
                    serde_json::json!({ "targets": targets })
                }
                RadarGraphQuery::VelocityVectorsAbove { speed_ms } => {
                    let vels: Vec<_> = graph
                        .nodes
                        .iter()
                        .filter(|n| matches!(n.node_type, RadarNodeType::VelocityVector))
                        .filter(|n| {
                            n.radial_velocity_ms
                                .map(|v| v.abs() > speed_ms)
                                .unwrap_or(false)
                        })
                        .collect();
                    serde_json::json!({ "velocity_vectors": vels })
                }
                RadarGraphQuery::WeatherCellsAbove { reflectivity_dbz } => {
                    let cells: Vec<_> = graph
                        .nodes
                        .iter()
                        .filter(|n| matches!(n.node_type, RadarNodeType::WeatherCell))
                        .collect();
                    serde_json::json!({ "weather_cells": cells })
                }
                RadarGraphQuery::MaterialSignatures => {
                    let sigs: Vec<_> = graph
                        .nodes
                        .iter()
                        .filter(|n| matches!(n.node_type, RadarNodeType::MaterialSignature))
                        .collect();
                    serde_json::json!({ "material_signatures": sigs })
                }
                RadarGraphQuery::AllNodes => serde_json::json!({ "nodes": graph.nodes }),
                RadarGraphQuery::AllEdges => serde_json::json!({ "edges": graph.edges }),
                RadarGraphQuery::CrossModalLinks { node_id } => {
                    let cross: Vec<_> = graph
                        .edges
                        .iter()
                        .filter(|e| {
                            (e.from_node == node_id || e.to_node == node_id)
                                && matches!(
                                    e.edge_type,
                                    RadarEdgeType::ContributesTo3DGeometry
                                        | RadarEdgeType::FeedsKinematicModel
                                        | RadarEdgeType::PlottedOnGeoMap
                                        | RadarEdgeType::MaterialCrossValidatedWith
                                        | RadarEdgeType::IdentifiesChemicalComposition
                                        | RadarEdgeType::InterferesWithEM
                                        | RadarEdgeType::ProvidesToControlSystem
                                )
                        })
                        .collect();
                    serde_json::json!({ "links": cross })
                }
                RadarGraphQuery::SARFeaturesInRegion {
                    pixel_x_min,
                    pixel_x_max,
                    pixel_y_min,
                    pixel_y_max,
                } => {
                    let features: Vec<_> = graph
                        .nodes
                        .iter()
                        .filter(|n| matches!(n.node_type, RadarNodeType::SARFeatureNode))
                        .collect();
                    serde_json::json!({ "sar_features": features })
                }
                RadarGraphQuery::AGIActivity => {
                    serde_json::json!({ "is_active": false, "activity": null })
                }
            };
            Ok(RadarModalityOutput {
                success: true,
                query_result: Some(result),
                ..Default::default()
            })
        }

        RadarModalityAction::GetGraph { graph_id } => {
            let graph = executor.load_graph(graph_id)?;
            Ok(RadarModalityOutput {
                success: true,
                graph_id: Some(graph_id),
                graph: Some(graph),
                ..Default::default()
            })
        }

        RadarModalityAction::TriggerSemanticHook { graph_id, hook } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            match hook {
                RadarSemanticHook::OnGraphCreated => {
                    // Register in ZSEI, build materialized paths, index keywords
                    graph.state = GraphStateType::SemanticEnriched;
                }
                RadarSemanticHook::OnInferRelationships => {
                    let new_edges = executor.infer_semantic_relationships(&graph.nodes).await;
                    let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                    for (from, to, etype, reason) in new_edges {
                        if graph.nodes.iter().any(|n| n.node_id == from)
                            && graph.nodes.iter().any(|n| n.node_id == to)
                        {
                            graph.edges.push(RadarGraphEdge {
                                edge_id: next_eid,
                                from_node: from,
                                to_node: to,
                                edge_type: etype,
                                weight: 0.8,
                                provenance: EdgeProvenance::DerivedFromHook,
                                version: 1,
                                properties: {
                                    let mut p = HashMap::new();
                                    p.insert("reason".into(), serde_json::json!(reason));
                                    p
                                },
                                ..Default::default()
                            });
                            next_eid += 1;
                        }
                    }
                }
                RadarSemanticHook::OnEdgeCompletion => {
                    // Verify endpoints, update hotness
                    let valid_ids: std::collections::HashSet<u64> =
                        graph.nodes.iter().map(|n| n.node_id).collect();
                    graph.edges.retain(|e| {
                        valid_ids.contains(&e.from_node) && valid_ids.contains(&e.to_node)
                    });
                }
                RadarSemanticHook::OnCrossModalityLink {
                    target_modality,
                    target_graph_id,
                } => {
                    // Register cross-modal edge in ZSEI global index
                    graph.state = GraphStateType::CrossLinked;
                    graph.version_notes.push(VersionNote {
                        version: graph.version + 1,
                        note: format!(
                            "Cross-linked to {} (graph {})",
                            target_modality, target_graph_id
                        ),
                        step_index: None,
                        timestamp: now.clone(),
                        change_type: ChangeType::CrossLinked,
                    });
                    graph.version += 1;
                }
            }
            graph.updated_at = now;
            executor.save_graph(&graph)?;
            Ok(RadarModalityOutput {
                success: true,
                graph_id: Some(graph_id),
                graph: Some(graph),
                ..Default::default()
            })
        }

        RadarModalityAction::ExportProduct { graph_id, format } => {
            let graph = executor.load_graph(graph_id)?;
            let export_path = format!("/tmp/radar_export_{}_{:?}.dat", graph_id, format);
            // In production: actual export logic per format
            Ok(RadarModalityOutput {
                success: true,
                export_path: Some(export_path),
                ..Default::default()
            })
        }

        RadarModalityAction::StreamToUI {
            graph_id,
            session_id,
            display_mode,
        } => {
            // In production: start WebSocket stream to UI session
            Ok(RadarModalityOutput {
                success: true,
                graph_id: Some(graph_id),
                ..Default::default()
            })
        }

        RadarModalityAction::HeadlessProcess {
            graph_id,
            operations,
        } => {
            let mut graph = executor.load_graph(graph_id)?;
            // Execute operations without rendering
            for op in operations {
                match op {
                    RadarOperation::ExportTargets => { /* export to CSV/GeoJSON */ }
                    RadarOperation::UpdateTrack { track_id } => { /* update track state */ }
                    RadarOperation::ReprocessPulse { pulse_id } => { /* reprocess */ }
                    RadarOperation::RefineWeatherCells => { /* refine classifications */ }
                    RadarOperation::CrossMatchWith3D { graph_id_3d } => {
                        // Add cross-modal edges to 3D graph nodes
                        let mut next_eid =
                            graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        for node in graph
                            .nodes
                            .iter()
                            .filter(|n| matches!(n.node_type, RadarNodeType::RadarPoint3D))
                        {
                            graph.edges.push(RadarGraphEdge {
                                edge_id: next_eid,
                                from_node: node.node_id,
                                to_node: node.node_id,
                                edge_type: RadarEdgeType::ContributesTo3DGeometry,
                                weight: 1.0,
                                provenance: EdgeProvenance::DerivedFromCrossModal,
                                version: 1,
                                properties: {
                                    let mut p = HashMap::new();
                                    p.insert(
                                        "target_graph_3d".into(),
                                        serde_json::json!(graph_id_3d),
                                    );
                                    p
                                },
                                ..Default::default()
                            });
                            next_eid += 1;
                        }
                    }
                }
            }
            executor.save_graph(&graph)?;
            Ok(RadarModalityOutput {
                success: true,
                graph_id: Some(graph_id),
                graph: Some(graph),
                ..Default::default()
            })
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// MAIN
// ─────────────────────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--input" && i + 1 < args.len() {
            input_json = args[i + 1].clone();
            i += 2;
        } else {
            i += 1;
        }
    }
    if input_json.is_empty() {
        eprintln!("Usage: radar_sensing --input '<json>'");
        std::process::exit(1);
    }
    let input: RadarModalityAction = match serde_json::from_str(&input_json) {
        Ok(v) => v,
        Err(e) => {
            println!(
                "{}",
                serde_json::json!({"success": false, "error": format!("Parse error: {}", e)})
            );
            std::process::exit(1);
        }
    };
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    match rt.block_on(execute(input)) {
        Ok(output) => {
            println!(
                "{}",
                serde_json::to_string(&output)
                    .unwrap_or_else(|_| r#"{"success":false,"error":"serialize error"}"#.into())
            );
        }
        Err(e) => {
            println!("{}", serde_json::json!({"success": false, "error": e}));
            std::process::exit(1);
        }
    }
}
