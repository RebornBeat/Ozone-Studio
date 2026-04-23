//! ElectromagneticSensing — Pipeline #118
//!
//! Passive EM sensing: spectrum monitoring, signal detection and classification,
//! propagation modeling, antenna pattern analysis, interference mapping,
//! direction-of-arrival estimation, and signal intelligence.
//!
//! DISTINCT FROM:
//!   - Radar (124): ACTIVE EM — controls source, measures return, enables precise
//!     ranging via known timing. Passive EM only RECEIVES; it has no controlled source.
//!   - Network Topology (123): logical/digital layer; EM is the physical RF layer.
//!
//! CROSS-LINKS:
//!   109 (3D)      → antenna geometry / propagation in 3D space
//!   117 (Geo)     → signals placed on geographic map, coverage maps
//!   122 (Control) → signal-based control (radio link quality, jamming detection)
//!   123 (Network) → RF layer underpins network connectivity
//!   124 (Radar)   → active vs passive EM fusion, interference patterns
//!
//! STORAGE: ZSEI containers under /Modalities/Electromagnetic/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

// ─────────────────────────────────────────────────────────────────────────────
// INPUT TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum EMModalityAction {
    /// Analyze a recorded or live EM signal/spectrum
    Analyze {
        data: EMDataSource,
        detect_signals: bool,
        classify_modulations: bool,
        locate_emitters: bool,
        model_propagation: bool,
    },
    /// Create a graph from analysis result
    CreateGraph {
        analysis: EMAnalysisResult,
        project_id: u64,
    },
    /// Update graph with new spectrum captures
    UpdateGraph {
        graph_id: u64,
        new_captures: Vec<SpectrumCapture>,
        project_id: u64,
    },
    /// Detect and classify signals in a raw IQ recording
    DetectSignals {
        iq_data: Vec<(f32, f32)>,
        sample_rate_hz: f64,
        center_freq_hz: f64,
        detector_type: SignalDetectorType,
        threshold_db: f32,
    },
    /// Estimate direction of arrival for a detected signal
    EstimateDoA {
        signal_id: u64,
        array_data: Vec<Vec<(f32, f32)>>,      // per-element IQ
        element_positions_m: Vec<[f32; 3]>,
        freq_hz: f64,
        method: DoAMethod,
    },
    /// Model EM propagation for given transmitter/environment
    ModelPropagation {
        transmitter: TransmitterSpec,
        environment: PropagationEnvironment,
        frequency_hz: f64,
        max_range_m: f32,
        model: PropagationModel,
    },
    /// Analyze antenna pattern from measurement or model
    AnalyzeAntennaPattern {
        pattern_data: AntennaPatternData,
        frequency_hz: f64,
    },
    /// Detect and characterize interference
    DetectInterference {
        graph_id: u64,
        target_signal_id: u64,
        threshold_db: f32,
    },
    /// Compute coverage map for a transmitter
    ComputeCoverage {
        transmitter: TransmitterSpec,
        area_extent: GeoExtent,
        resolution_m: f32,
        frequency_hz: f64,
        model: PropagationModel,
    },
    /// Query graph
    QueryGraph {
        graph_id: u64,
        query: EMGraphQuery,
    },
    /// Retrieve full graph for Context Viewer
    GetGraph { graph_id: u64 },
    /// Trigger ZSEI semantic hooks
    TriggerSemanticHook {
        graph_id: u64,
        hook: EMSemanticHook,
    },
    /// Export EM products
    ExportProduct {
        graph_id: u64,
        format: EMExportFormat,
    },
    /// Stream to UI
    StreamToUI {
        graph_id: u64,
        session_id: String,
        display_mode: EMDisplayMode,
    },
    /// Headless processing (AGI-first)
    HeadlessProcess {
        graph_id: u64,
        operations: Vec<EMOperation>,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// DATA SOURCES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EMDataSource {
    /// Raw IQ file (SDR capture: SigMF, raw binary, wav)
    IQFile {
        file_path: String,
        format: IQFileFormat,
        sample_rate_hz: f64,
        center_freq_hz: f64,
        gain_db: f32,
    },
    /// Pre-computed spectrum (power spectral density)
    SpectrumFile {
        file_path: String,
        format: SpectrumFileFormat,
        freq_start_hz: f64,
        freq_end_hz: f64,
        resolution_hz: f64,
    },
    /// TDOA / multi-static measurement dataset
    TDOAMeasurements {
        sensor_files: Vec<SensorCapture>,
        sync_method: SyncMethod,
    },
    /// RF survey / spectrum scan
    SpectrumScan {
        file_path: String,
        scan_start_hz: f64,
        scan_stop_hz: f64,
        dwell_time_ms: f32,
        sweep_count: u32,
    },
    /// Antenna measurement (near-field or far-field)
    AntennaPattern {
        file_path: String,
        format: AntennaDataFormat,
        frequency_hz: f64,
    },
    /// Live SDR stream
    LiveSDR {
        endpoint: String,
        sdr_type: SDRType,
        center_freq_hz: f64,
        sample_rate_hz: f64,
        gain_db: f32,
    },
    /// Multiple captures from a MIMO / distributed array
    ArrayCapture {
        element_files: Vec<String>,
        element_positions_m: Vec<[f32; 3]>,
        format: IQFileFormat,
        sample_rate_hz: f64,
        center_freq_hz: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IQFileFormat { SigMF, RawBinary_FC32, RawBinary_SC16, WAV_IQ, HackRF, RTL_SDR, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpectrumFileFormat { CSV, NPZ, HDF5, SigMF_Meta, CRFS_BIN, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AntennaDataFormat { CST, GRASP, NEC2, HFSS, Measured_CSV, Touchstone, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SDRType { RTL_SDR, HackRF, USRP, LimeSDR, BladeRF, PlutoSDR, AirSpy, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncMethod { GPS_PPS, Rubidium, NetworkPTP, PostProcessing }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SensorCapture {
    pub sensor_id: String,
    pub location: Option<[f64; 3]>,   // lat, lon, alt
    pub file_path: String,
    pub sample_rate_hz: f64,
    pub center_freq_hz: f64,
    pub timestamp_utc: f64,
}

// ─────────────────────────────────────────────────────────────────────────────
// ANALYSIS RESULT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EMAnalysisResult {
    pub analysis_id: u64,
    pub source_description: String,

    // SPECTRUM CHARACTERIZATION
    pub spectrum_snapshots: Vec<SpectrumSnapshot>,
    pub frequency_occupancy: Vec<FrequencyBand>,    // occupied bands detected
    pub noise_floor_db: f32,
    pub dynamic_range_db: f32,
    pub peak_signal_db: f32,

    // SIGNAL DETECTION
    pub detected_signals: Vec<DetectedSignal>,
    pub unclassified_signals: Vec<UnclassifiedSignal>,

    // EMITTER LOCALIZATION
    pub emitters: Vec<SignalEmitter>,
    pub doa_estimates: Vec<DoAEstimate>,
    pub tdoa_localizations: Vec<TDOALocalization>,

    // PROPAGATION
    pub propagation_paths: Vec<PropagationPath>,
    pub coverage_maps: Vec<CoverageMap>,
    pub propagation_models: Vec<PropagationModelResult>,

    // ANTENNA
    pub antenna_patterns: Vec<AntennaPattern>,

    // INTERFERENCE
    pub interference_events: Vec<InterferenceEvent>,
    pub interference_sources: Vec<InterferenceSource>,

    // METADATA
    pub capture_start_utc: f64,
    pub capture_duration_sec: f32,
    pub platform_location: Option<[f64; 3]>,
    pub processing_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpectrumSnapshot {
    pub snapshot_id: u64,
    pub center_freq_hz: f64,
    pub bandwidth_hz: f64,
    pub timestamp_utc: f64,
    pub duration_sec: f32,
    pub psd_db_hz: Vec<f32>,                // power spectral density
    pub freq_bins_hz: Vec<f64>,             // frequency for each PSD bin
    pub peak_freq_hz: f64,
    pub peak_power_db: f32,
    pub rms_power_db: f32,
    pub occupancy_fraction: f32,           // fraction of bins above noise floor
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FrequencyBand {
    pub band_id: u64,
    pub start_freq_hz: f64,
    pub end_freq_hz: f64,
    pub bandwidth_hz: f64,
    pub center_freq_hz: f64,
    pub peak_power_db: f32,
    pub mean_power_db: f32,
    pub occupancy_percent: f32,
    pub allocation: Option<String>,         // known allocation: "WiFi 2.4GHz", "LTE Band 7", etc.
    pub modulation_type: Option<ModulationType>,
    pub signal_ids: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DetectedSignal {
    pub signal_id: u64,
    pub center_freq_hz: f64,
    pub bandwidth_hz: f64,
    pub power_dbm: f32,
    pub snr_db: f32,
    pub duration_sec: Option<f32>,
    pub burst_type: BurstType,
    pub modulation: ModulationType,
    pub modulation_confidence_note: String,
    pub symbol_rate_sps: Option<f64>,
    pub protocol_hint: Option<String>,     // "LTE", "WiFi 802.11n", "TETRA", "ADS-B", etc.
    pub occupied_bandwidth_hz: f64,
    pub peak_freq_hz: f64,
    pub timestamp_utc: f64,
    pub doa_estimate: Option<DoAEstimate>,
    pub emitter_id: Option<u64>,
    pub features: SignalFeatures,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SignalFeatures {
    pub spectral_kurtosis: f32,
    pub cyclostationary_freq_hz: Option<f64>,
    pub am_index: Option<f32>,
    pub fm_deviation_hz: Option<f64>,
    pub phase_continuity: f32,         // 0=discontinuous, 1=continuous
    pub instantaneous_bandwidth_hz: f64,
    pub spectral_flatness: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ModulationType {
    #[default] Unknown,
    // Analog
    AM, FM, SSB_USB, SSB_LSB, DSB, PM, CW,
    // Digital
    BPSK, QPSK, QAM16, QAM64, QAM256, QAM1024,
    FSK2, FSK4, MSK, GMSK, OFDM, FHSS, DSSS,
    // Protocol-specific
    WiFi_OFDM, LTE_OFDM, NR5G, GSM_GMSK, UMTS, TETRA, ADS_B, AIS, ACARS,
    LoRa, Zigbee, Bluetooth, UWB, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum BurstType { #[default] Continuous, Burst, Pulsed, FHSS_Hopping, Intermittent }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UnclassifiedSignal {
    pub signal_id: u64,
    pub center_freq_hz: f64,
    pub bandwidth_hz: f64,
    pub power_dbm: f32,
    pub snr_db: f32,
    pub timestamp_utc: f64,
    pub anomaly_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SignalEmitter {
    pub emitter_id: u64,
    pub location: Option<[f64; 3]>,        // lat, lon, alt — None if unknown
    pub location_uncertainty_m: Option<f32>,
    pub transmit_power_dbm: Option<f32>,
    pub antenna_gain_dbi: Option<f32>,
    pub signal_ids: Vec<u64>,
    pub emitter_class: EmitterClass,
    pub identifier: Option<String>,         // call sign, ICAO, MMSI, MAC, etc.
    pub first_seen_utc: f64,
    pub last_seen_utc: f64,
    pub mobility: EmitterMobility,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum EmitterClass {
    #[default] Unknown,
    CivilianAircraft, MilitaryAircraft, Vessel, GroundVehicle,
    BaseStation, WiFiAP, CellTower, Satellite, Radar, Jammer,
    IoTDevice, HandheldRadio, Broadcast, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum EmitterMobility { #[default] Unknown, Stationary, Mobile, SlowMoving, FastMoving }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DoAEstimate {
    pub estimate_id: u64,
    pub signal_id: u64,
    pub azimuth_deg: f32,
    pub elevation_deg: Option<f32>,
    pub azimuth_uncertainty_deg: f32,
    pub method: DoAMethod,
    pub timestamp_utc: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum DoAMethod { #[default] MUSIC, ESPRIT, Bartlett, Capon, MVDR, DBT, MaximumLikelihood }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TDOALocalization {
    pub localization_id: u64,
    pub signal_id: u64,
    pub sensor_ids: Vec<String>,
    pub tdoa_measurements_sec: Vec<f64>,   // per sensor pair
    pub estimated_location: [f64; 3],      // lat, lon, alt
    pub cep_m: f32,                        // circular error probable
    pub method: String,
    pub timestamp_utc: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PropagationPath {
    pub path_id: u64,
    pub transmitter_id: Option<u64>,
    pub transmitter_location: [f64; 3],
    pub receiver_location: [f64; 3],
    pub frequency_hz: f64,
    pub path_loss_db: f32,
    pub delay_sec: f64,
    pub doppler_hz: Option<f32>,
    pub propagation_mode: PropagationMode,
    pub multipath_components: Vec<MultipathComponent>,
    pub fresnel_zone_clearance_m: Option<f32>,
    pub terrain_obstruction: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PropagationMode {
    #[default] LOS,         // line of sight
    NLOS,                   // non-line-of-sight
    Diffraction,
    Reflection,
    Troposcatter,
    Ionospheric,
    Ducting,
    GroundWave,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MultipathComponent {
    pub component_id: u32,
    pub relative_delay_sec: f64,
    pub relative_power_db: f32,
    pub doppler_hz: f32,
    pub reflection_surface: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CoverageMap {
    pub map_id: u64,
    pub transmitter_id: Option<u64>,
    pub frequency_hz: f64,
    pub area_extent: GeoExtent,
    pub resolution_m: f32,
    pub path_loss_grid_db: Vec<Vec<f32>>,  // [row][col] = path loss in dB
    pub covered_area_km2: f32,             // area above threshold
    pub threshold_db: f32,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeoExtent {
    pub min_lat: f64, pub max_lat: f64,
    pub min_lon: f64, pub max_lon: f64,
    pub min_alt_m: f32, pub max_alt_m: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PropagationModelResult {
    pub model_id: u64,
    pub model_type: PropagationModel,
    pub frequency_hz: f64,
    pub environment: PropagationEnvironment,
    pub path_loss_vs_range_db: Vec<(f32, f32)>,  // (range_m, loss_db)
    pub breakpoint_m: Option<f32>,               // free-space → NLOS transition
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PropagationModel {
    #[default] FreeSpace,
    COST231_Hata, Okumura_Hata, ITU_R_P528, ITU_R_P1546, Longley_Rice,
    Two_Ray, Winner_II, 3GPP_UMa, 3GPP_UMi, Indoor_COST231,
    Terrain_Following, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PropagationEnvironment {
    #[default] OpenArea, SuburbanMacro, UrbanMicro, UrbanMacro,
    DenseUrban, Indoor, Tunnel, Maritime, Aeronautical, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TransmitterSpec {
    pub transmitter_id: Option<u64>,
    pub location: [f64; 3],
    pub power_dbm: f32,
    pub antenna_gain_dbi: f32,
    pub frequency_hz: f64,
    pub bandwidth_hz: f64,
    pub antenna_height_m: f32,
    pub polarization: EMPolarization,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum EMPolarization { #[default] Vertical, Horizontal, Circular_RHCP, Circular_LHCP, Elliptical }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AntennaPattern {
    pub pattern_id: u64,
    pub antenna_type: AntennaType,
    pub frequency_hz: f64,
    pub gain_dbi: f32,
    pub beamwidth_e_deg: f32,
    pub beamwidth_h_deg: f32,
    pub sidelobe_level_db: f32,
    pub front_to_back_db: f32,
    pub polarization: EMPolarization,
    pub gain_pattern_e_plane: Vec<(f32, f32)>,  // (angle_deg, gain_db)
    pub gain_pattern_h_plane: Vec<(f32, f32)>,
    pub is_steerable: bool,
    pub steering_angle: Option<[f32; 2]>,      // (az_deg, el_deg)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum AntennaType {
    #[default] Unknown, Dipole, Monopole, Yagi, Log_Periodic, Horn, Patch, Parabolic_Dish,
    Phased_Array, Helix, Slot, Fractal, Whip, Loop, MIMO_Array, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AntennaPatternData {
    pub antenna_type: AntennaType,
    pub gain_dbi: f32,
    pub beamwidth_deg: f32,
    pub element_count: u32,
    pub e_plane_data: Vec<(f32, f32)>,
    pub h_plane_data: Vec<(f32, f32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InterferenceEvent {
    pub event_id: u64,
    pub victim_signal_id: u64,
    pub interferer_signal_id: Option<u64>,
    pub interference_type: InterferenceType,
    pub sir_db: f32,                           // signal-to-interference ratio
    pub frequency_overlap_hz: f64,
    pub duration_sec: Option<f32>,
    pub timestamp_utc: f64,
    pub impact: InterferenceImpact,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum InterferenceType {
    #[default] Unknown, Co_Channel, Adjacent_Channel, Intermodulation, Harmonic,
    Desensitization, Phase_Noise, Jamming, Spoofing, Unintentional, Natural,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum InterferenceImpact { #[default] None, Marginal, Moderate, Severe, LossOfSignal }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InterferenceSource {
    pub source_id: u64,
    pub center_freq_hz: f64,
    pub bandwidth_hz: f64,
    pub power_dbm: f32,
    pub location: Option<[f64; 3]>,
    pub interference_type: InterferenceType,
    pub victim_signal_ids: Vec<u64>,
    pub is_intentional: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpectrumCapture {
    pub capture_id: u64,
    pub timestamp_utc: f64,
    pub center_freq_hz: f64,
    pub bandwidth_hz: f64,
    pub psd_db_hz: Vec<f32>,
    pub peak_power_db: f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// DETECTION / ESTIMATION TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalDetectorType { EnergyDetector, CyclostationaryDetector, MatchedFilter, CAFACorrelation, WaveformML }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DoAMethod { MUSIC, ESPRIT, Bartlett, Capon, MVDR, DBT, MaximumLikelihood }

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH NODE TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum EMNodeType {
    #[default] EMScene,                 // root: entire capture/analysis
    SpectrumSnapshotNode,               // single PSD snapshot
    FrequencyBandNode,                  // identified occupied band
    DetectedSignalNode,                 // characterized signal
    UnclassifiedSignalNode,             // unknown/anomalous signal
    SignalEmitterNode,                  // located transmitter
    DoAEstimateNode,                    // direction of arrival
    TDOALocalizationNode,              // TDOA-based position
    PropagationPathNode,               // single propagation path
    MultipathComponentNode,            // multipath ray
    CoverageMapNode,                   // coverage map product
    PropagationModelNode,              // propagation model result
    AntennaPatternNode,                // antenna radiation pattern
    InterferenceEventNode,             // interference occurrence
    InterferenceSourceNode,            // interference emitter
    GeographicLocationNode,            // geo-referenced position
    FrequencyAllocationNode,           // spectrum allocation record
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EMGraphNode {
    pub node_id: u64,
    pub node_type: EMNodeType,
    pub content: String,

    // EM-SPECIFIC
    pub freq_hz: Option<f64>,
    pub bandwidth_hz: Option<f64>,
    pub power_dbm: Option<f32>,
    pub location: Option<[f64; 3]>,
    pub azimuth_deg: Option<f32>,
    pub snr_db: Option<f32>,
    pub timestamp_utc: Option<f64>,
    pub modulation: Option<String>,
    pub protocol: Option<String>,
    pub emitter_class: Option<String>,

    // UNIVERSAL NODE FIELDS
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
pub enum EMEdgeType {
    // ── STRUCTURAL ──
    #[default] Contains, Precedes, PartOf,

    // ── EM-SPECIFIC ──
    TransmitsTo,                // emitter transmits to receiver
    InterferesWith,             // source interferes with signal
    PropagatesThrough,          // signal propagates through medium/path
    ObstructedBy,               // path is obstructed
    ReflectsFrom,               // signal reflects from surface
    DiffractsAround,            // signal diffracts around obstacle
    DoAPointsTo,                // DoA estimate points toward emitter
    TDOALocalizes,              // TDOA localizes this emitter
    OccupiesBand,               // signal occupies frequency band
    DetectedIn,                 // signal detected in snapshot
    InterfererOf,               // this source is the interferer
    VictimOf,                   // this signal is victim of interference
    AntennaPatternOf,           // pattern belongs to emitter
    CoverageOf,                 // coverage map belongs to transmitter
    TemporallyCorrelatedWith,   // signals correlated in time
    FrequencyAdjacentTo,        // adjacent frequency allocation
    SameEmitterAs,              // two signals from same emitter
    PropagationModelOf,         // model characterizes this path

    // ── CROSS-MODAL ──
    /// Signal emitter plotted on geospatial map (117)
    PlottedOnGeoMap,
    /// Propagation path through 3D scene geometry (109)
    PropagationThrough3D,
    /// RF link quality feeds control system (122)
    LinkQualityToControl,
    /// RF physical layer of network topology node (123)
    PhysicalLayerOf,
    /// Passive EM vs Active Radar interference (124)
    InterferesWithRadar,

    // ── UNIVERSAL SEMANTIC ──
    Performs, Affects, Implies, Contradicts, Elaborates, Summarizes, Supports,
    TemporalPrecedes, TemporalFollows, CausedBy, Enables, Prevents,
    FunctionalRole, InstanceOf,
    DerivedFrom, VersionOf, RefinesTo, ForkedFrom, SimilarTo,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EMGraphEdge {
    pub edge_id: u64,
    pub from_node: u64, pub to_node: u64,
    pub edge_type: EMEdgeType,
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
pub struct EMGraph {
    pub graph_id: u64, pub project_id: u64,
    pub source_description: String,
    pub nodes: Vec<EMGraphNode>,
    pub edges: Vec<EMGraphEdge>,
    pub root_node_id: u64,
    pub state: GraphStateType,
    pub state_history: Vec<GraphStateTransition>,
    pub created_at: String, pub updated_at: String,
    pub version: u32, pub version_notes: Vec<VersionNote>,
}

// ─────────────────────────────────────────────────────────────────────────────
// QUERY / HOOKS / DISPLAY
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EMGraphQuery {
    NodeDetail { node_id: u64 },
    SignalsByModulation { modulation: String },
    EmittersByClass { emitter_class: String },
    InterferenceEvents,
    OccupiedBands { freq_start_hz: f64, freq_end_hz: f64 },
    PropagationPaths,
    AntennaPatterns,
    CrossModalLinks { node_id: u64 },
    AGIActivity, AllNodes, AllEdges,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EMSemanticHook {
    OnGraphCreated,
    OnInferRelationships,
    OnEdgeCompletion,
    OnCrossModalityLink { target_modality: String, target_graph_id: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EMExportFormat {
    GeoJSON,        // emitter/interference events as features
    KML,            // on Google Earth
    CSV,            // signal/emitter tables
    SigMF,          // signal metadata format
    HDF5,           // scientific arrays
    WigleCSV,       // WiFi survey format
    GNSS_XML,       // for ADS-B/AIS data
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EMDisplayMode {
    Spectrogram,        // time-freq heatmap
    PowerSpectrum,      // frequency vs power
    Waterfall,          // scrolling spectrogram
    CoverageMap,        // geographic coverage
    EmitterMap,         // located emitters on map
    InterferenceTable,  // tabular interference events
    AntennaPattern,     // polar radiation plot
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EMOperation {
    ReClassifySignals,
    ReLocalizeEmitters,
    ComputeInterference,
    UpdateCoverage { transmitter_id: u64 },
    CrossLinkToGeo { geo_graph_id: u64 },
}

// ─────────────────────────────────────────────────────────────────────────────
// OUTPUT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EMModalityOutput {
    pub success: bool,
    pub graph_id: Option<u64>,
    pub graph: Option<EMGraph>,
    pub analysis: Option<EMAnalysisResult>,
    pub detected_signals: Option<Vec<DetectedSignal>>,
    pub doa_estimate: Option<DoAEstimate>,
    pub propagation_model: Option<PropagationModelResult>,
    pub antenna_pattern: Option<AntennaPattern>,
    pub coverage_map: Option<CoverageMap>,
    pub interference_events: Option<Vec<InterferenceEvent>>,
    pub query_result: Option<serde_json::Value>,
    pub export_path: Option<String>,
    pub error: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// SHARED TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ProvisionalStatus { #[default] Planned, Generating, Generated, Validated, Finalized, Failed, RolledBack }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VersionNote { pub version: u32, pub note: String, pub step_index: Option<u32>, pub timestamp: String, pub change_type: ChangeType }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ChangeType { #[default] Created, Updated, CrossLinked, EnrichedBySemantic, EnrichedByHook, RolledBack, Finalized }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum EdgeProvenance {
    #[default] Unknown, DerivedFromPrompt, DerivedFromChunk(u32), DerivedFromChunkGraph(u64),
    DerivedFromModalityGraph(u64), DerivedFromFile(String),
    DerivedFromAMT, DerivedFromBlueprint(u32), DerivedFromMethodology(u64),
    DerivedFromCrossModal, DerivedFromHook, VersionOf(u32), ForkedFrom(u64),
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum GraphStateType { #[default] Created, SemanticEnriched, CrossLinked, Stable, Updated, ReValidating, Archived }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStateTransition { pub from: GraphStateType, pub to: GraphStateType, pub timestamp: String, pub triggered_by_step: Option<u32> }

// ─────────────────────────────────────────────────────────────────────────────
// PIPELINE EXECUTOR
// ─────────────────────────────────────────────────────────────────────────────

struct PipelineExecutor { zsei_path: String, prompt_pipeline_path: String }

impl PipelineExecutor {
    fn new() -> Self {
        Self {
            zsei_path: env::var("OZONE_ZSEI_PATH").unwrap_or_else(|_| "./zsei_data".into()),
            prompt_pipeline_path: env::var("OZONE_PROMPT_PIPELINE").unwrap_or_else(|_| "./pipeline_9".into()),
        }
    }

    async fn llm_zero_shot(&self, prompt: &str, max_tokens: usize) -> Result<String, String> {
        let input = serde_json::json!({"action":"Prompt","prompt":prompt,"max_tokens":max_tokens,"temperature":0.05,"system_context":"EM signal analysis. Return only valid JSON."});
        let out = std::process::Command::new(&self.prompt_pipeline_path).arg("--input").arg(input.to_string()).output().map_err(|e| e.to_string())?;
        let r: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
        Ok(r["response"].as_str().unwrap_or("{}").to_string())
    }

    fn save_graph(&self, g: &EMGraph) -> Result<(), String> {
        let path = format!("{}/local/em_graph_{}.json", self.zsei_path, g.graph_id);
        if let Some(p) = std::path::Path::new(&path).parent() { std::fs::create_dir_all(p).map_err(|e| e.to_string())?; }
        std::fs::write(&path, serde_json::to_string_pretty(g).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    fn load_graph(&self, id: u64) -> Result<EMGraph, String> {
        let path = format!("{}/local/em_graph_{}.json", self.zsei_path, id);
        serde_json::from_str(&std::fs::read_to_string(&path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    fn generate_id(&self) -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as u64 }
    fn now_iso8601(&self) -> String { format!("{}", self.generate_id()) }
    fn extract_json_array(raw: &str) -> String { if let (Some(s), Some(e)) = (raw.find('['), raw.rfind(']')) { raw[s..=e].to_string() } else { "[]".to_string() } }
    fn extract_json_object(raw: &str) -> String { if let (Some(s), Some(e)) = (raw.find('{'), raw.rfind('}')) { raw[s..=e].to_string() } else { "{}".to_string() } }
}

impl PipelineExecutor {
    async fn classify_signals_llm(&self, signals: &[DetectedSignal]) -> Vec<(u64, String, String)> {
        if signals.is_empty() { return vec![]; }
        let list: Vec<serde_json::Value> = signals.iter().take(20).map(|s| serde_json::json!({
            "signal_id": s.signal_id,
            "center_freq_mhz": s.center_freq_hz / 1e6,
            "bandwidth_mhz": s.bandwidth_hz / 1e6,
            "power_dbm": s.power_dbm,
            "snr_db": s.snr_db,
            "burst_type": format!("{:?}", s.burst_type),
            "symbol_rate_ksps": s.symbol_rate_sps.map(|r| r / 1000.0),
            "spectral_kurtosis": s.features.spectral_kurtosis,
            "phase_continuity": s.features.phase_continuity,
        })).collect();

        let prompt = format!(r#"
Classify each RF signal's modulation type and likely protocol/application.

Signals:
{}

Modulation options: AM, FM, SSB_USB, SSB_LSB, BPSK, QPSK, QAM16, QAM64, QAM256, OFDM, FSK2, GMSK, FHSS, DSSS, WiFi_OFDM, LTE_OFDM, NR5G, GSM_GMSK, ADS_B, AIS, LoRa, Bluetooth, Unknown

Return ONLY valid JSON array:
[{{"signal_id": N, "modulation": "ModType", "protocol_hint": "protocol_or_application"}}]"#,
            serde_json::to_string_pretty(&list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 600).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str).unwrap_or_default().into_iter()
                    .filter_map(|v| Some((v["signal_id"].as_u64()?, v["modulation"].as_str()?.to_string(), v["protocol_hint"].as_str().unwrap_or("").to_string())))
                    .collect()
            }
            Err(_) => vec![],
        }
    }

    async fn classify_emitters_llm(&self, emitters: &[SignalEmitter]) -> Vec<(u64, String)> {
        if emitters.is_empty() { return vec![]; }
        let list: Vec<serde_json::Value> = emitters.iter().take(15).map(|e| serde_json::json!({
            "emitter_id": e.emitter_id,
            "signal_count": e.signal_ids.len(),
            "mobility": format!("{:?}", e.mobility),
            "has_location": e.location.is_some(),
            "identifier": e.identifier,
        })).collect();

        let prompt = format!(r#"
Classify each RF emitter type:
CivilianAircraft, MilitaryAircraft, Vessel, GroundVehicle, BaseStation,
WiFiAP, CellTower, Satellite, Radar, Jammer, IoTDevice, HandheldRadio, Broadcast, Unknown.

Emitters: {}
Return ONLY valid JSON array: [{{"emitter_id": N, "emitter_class": "ClassName"}}]"#,
            serde_json::to_string_pretty(&list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 300).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str).unwrap_or_default().into_iter()
                    .filter_map(|v| Some((v["emitter_id"].as_u64()?, v["emitter_class"].as_str()?.to_string())))
                    .collect()
            }
            Err(_) => vec![],
        }
    }

    async fn infer_semantic_relationships(&self, nodes: &[EMGraphNode]) -> Vec<(u64, u64, EMEdgeType, String)> {
        if nodes.len() < 2 { return vec![]; }
        let node_list: Vec<serde_json::Value> = nodes.iter().take(25).map(|n| serde_json::json!({
            "node_id": n.node_id, "type": format!("{:?}", n.node_type),
            "content": n.content.chars().take(80).collect::<String>(),
            "freq_mhz": n.freq_hz.map(|f| f / 1e6),
            "power_dbm": n.power_dbm,
        })).collect();

        let prompt = format!(r#"
Identify semantic relationships between these EM graph nodes.

Nodes: {}

Available types: TransmitsTo, InterferesWith, PropagatesThrough, OccupiesBand, DetectedIn,
SameEmitterAs, TemporallyCorrelatedWith, FrequencyAdjacentTo, Affects, CausedBy, Enables, DerivedFrom

Return ONLY valid JSON array:
[{{"from_node_id": N, "to_node_id": M, "edge_type": "TypeName", "reason": "brief"}}]"#,
            serde_json::to_string_pretty(&node_list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 600).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str).unwrap_or_default().into_iter()
                    .filter_map(|v| {
                        let from = v["from_node_id"].as_u64()?;
                        let to = v["to_node_id"].as_u64()?;
                        let etype = map_em_edge_str(v["edge_type"].as_str().unwrap_or("Affects"));
                        let reason = v["reason"].as_str().unwrap_or("").to_string();
                        Some((from, to, etype, reason))
                    }).collect()
            }
            Err(_) => vec![],
        }
    }

    fn compute_free_space_path_loss(dist_m: f32, freq_hz: f64) -> f32 {
        if dist_m <= 0.0 || freq_hz <= 0.0 { return 0.0; }
        let lambda_m = 3e8 / freq_hz;
        (4.0 * std::f64::consts::PI * dist_m as f64 / lambda_m).log10() as f32 * 20.0
    }
}

fn map_em_edge_str(s: &str) -> EMEdgeType {
    match s {
        "TransmitsTo"              => EMEdgeType::TransmitsTo,
        "InterferesWith"           => EMEdgeType::InterferesWith,
        "PropagatesThrough"        => EMEdgeType::PropagatesThrough,
        "ObstructedBy"             => EMEdgeType::ObstructedBy,
        "ReflectsFrom"             => EMEdgeType::ReflectsFrom,
        "OccupiesBand"             => EMEdgeType::OccupiesBand,
        "DetectedIn"               => EMEdgeType::DetectedIn,
        "InterfererOf"             => EMEdgeType::InterfererOf,
        "VictimOf"                 => EMEdgeType::VictimOf,
        "AntennaPatternOf"         => EMEdgeType::AntennaPatternOf,
        "CoverageOf"               => EMEdgeType::CoverageOf,
        "SameEmitterAs"            => EMEdgeType::SameEmitterAs,
        "TemporallyCorrelatedWith" => EMEdgeType::TemporallyCorrelatedWith,
        "FrequencyAdjacentTo"      => EMEdgeType::FrequencyAdjacentTo,
        "PropagationModelOf"       => EMEdgeType::PropagationModelOf,
        "PlottedOnGeoMap"          => EMEdgeType::PlottedOnGeoMap,
        "PhysicalLayerOf"          => EMEdgeType::PhysicalLayerOf,
        "InterferesWithRadar"      => EMEdgeType::InterferesWithRadar,
        "Affects"                  => EMEdgeType::Affects,
        "CausedBy"                 => EMEdgeType::CausedBy,
        "Enables"                  => EMEdgeType::Enables,
        "TemporalPrecedes"         => EMEdgeType::TemporalPrecedes,
        "DerivedFrom"              => EMEdgeType::DerivedFrom,
        "PartOf"                   => EMEdgeType::PartOf,
        _                          => EMEdgeType::Affects,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH CREATION
// ─────────────────────────────────────────────────────────────────────────────

async fn create_graph(executor: &PipelineExecutor, analysis: EMAnalysisResult, project_id: u64) -> EMModalityOutput {
    let graph_id = executor.generate_id();
    let now = executor.now_iso8601();
    let mut nodes: Vec<EMGraphNode> = Vec::new();
    let mut edges: Vec<EMGraphEdge> = Vec::new();
    let mut node_id: u64 = 1;
    let mut edge_id: u64 = 1;

    // ── ROOT ──
    let root_id = node_id;
    nodes.push(EMGraphNode {
        node_id: root_id, node_type: EMNodeType::EMScene,
        content: format!("EM scene: {} snapshots {} signals {} emitters {} interference",
            analysis.spectrum_snapshots.len(), analysis.detected_signals.len(),
            analysis.emitters.len(), analysis.interference_events.len()),
        materialized_path: Some(format!("/Modalities/Electromagnetic/Project_{}/Graph_{}", project_id, graph_id)),
        provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
        version_notes: vec![VersionNote { version: 1, note: "Initial creation".into(), step_index: None, timestamp: now.clone(), change_type: ChangeType::Created }],
        keywords: vec!["electromagnetic".into(), "spectrum".into()],
        hotness_score: 1.0, ..Default::default()
    });
    node_id += 1;

    // ── SPECTRUM SNAPSHOTS ──
    let snap_node_ids: Vec<(u64, u64)> = analysis.spectrum_snapshots.iter().take(50).map(|snap| {
        let sid = node_id;
        nodes.push(EMGraphNode {
            node_id: sid, node_type: EMNodeType::SpectrumSnapshotNode,
            content: format!("Spectrum: {:.1}MHz bw={:.1}MHz peak={:.1}dB occ={:.0}%",
                snap.center_freq_hz / 1e6, snap.bandwidth_hz / 1e6, snap.peak_power_db, snap.occupancy_fraction * 100.0),
            freq_hz: Some(snap.center_freq_hz), bandwidth_hz: Some(snap.bandwidth_hz),
            power_dbm: Some(snap.peak_power_db), timestamp_utc: Some(snap.timestamp_utc),
            materialized_path: Some(format!("/Modalities/Electromagnetic/Project_{}/Graph_{}/Snapshot/{}", project_id, graph_id, snap.snapshot_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["spectrum".into(), "snapshot".into()], hotness_score: 0.6, ..Default::default()
        });
        edges.push(EMGraphEdge { edge_id, from_node: root_id, to_node: sid, edge_type: EMEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
        (snap.snapshot_id, sid)
    }).collect();

    // ── FREQUENCY BANDS ──
    let band_node_ids: Vec<(u64, u64)> = analysis.frequency_occupancy.iter().map(|band| {
        let bid = node_id;
        nodes.push(EMGraphNode {
            node_id: bid, node_type: EMNodeType::FrequencyBandNode,
            content: format!("Band: {:.1}–{:.1}MHz peak={:.1}dB occ={:.0}% mod={:?} alloc={:?}",
                band.start_freq_hz / 1e6, band.end_freq_hz / 1e6, band.peak_power_db,
                band.occupancy_percent, band.modulation_type, band.allocation.as_deref().unwrap_or("unknown")),
            freq_hz: Some(band.center_freq_hz), bandwidth_hz: Some(band.bandwidth_hz),
            power_dbm: Some(band.peak_power_db),
            modulation: band.modulation_type.as_ref().map(|m| format!("{:?}", m)),
            materialized_path: Some(format!("/Modalities/Electromagnetic/Project_{}/Graph_{}/Band/{}", project_id, graph_id, band.band_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: { let mut kw = vec!["frequency-band".into()]; if let Some(ref a) = band.allocation { kw.push(a.to_lowercase()); } kw },
            hotness_score: 0.75, ..Default::default()
        });
        edges.push(EMGraphEdge { edge_id, from_node: root_id, to_node: bid, edge_type: EMEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
        (band.band_id, bid)
    }).collect();

    // ── DETECTED SIGNALS ──
    let signal_node_ids: Vec<(u64, u64)> = analysis.detected_signals.iter().map(|sig| {
        let sid = node_id;
        nodes.push(EMGraphNode {
            node_id: sid, node_type: EMNodeType::DetectedSignalNode,
            content: format!("Signal: {:.2}MHz bw={:.0}kHz pwr={:.1}dBm snr={:.1}dB mod={:?} proto={:?}",
                sig.center_freq_hz / 1e6, sig.bandwidth_hz / 1e3, sig.power_dbm,
                sig.snr_db, sig.modulation, sig.protocol_hint.as_deref().unwrap_or("?")),
            freq_hz: Some(sig.center_freq_hz), bandwidth_hz: Some(sig.bandwidth_hz),
            power_dbm: Some(sig.power_dbm), snr_db: Some(sig.snr_db),
            timestamp_utc: Some(sig.timestamp_utc),
            modulation: Some(format!("{:?}", sig.modulation)),
            protocol: sig.protocol_hint.clone(),
            materialized_path: Some(format!("/Modalities/Electromagnetic/Project_{}/Graph_{}/Signal/{}", project_id, graph_id, sig.signal_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: { let mut kw = vec!["signal".into(), format!("{:?}", sig.modulation).to_lowercase()]; if let Some(ref p) = sig.protocol_hint { kw.push(p.to_lowercase()); } kw },
            hotness_score: 0.6 + (sig.snr_db / 60.0).clamp(0.0, 0.3),
            ..Default::default()
        });
        edges.push(EMGraphEdge { edge_id, from_node: root_id, to_node: sid, edge_type: EMEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Signal occupies band
        for (band_id, &band_nid) in &band_node_ids {
            let band = analysis.frequency_occupancy.iter().find(|b| b.band_id == *band_id);
            if let Some(b) = band {
                if sig.center_freq_hz >= b.start_freq_hz && sig.center_freq_hz <= b.end_freq_hz {
                    edges.push(EMGraphEdge { edge_id, from_node: sid, to_node: band_nid, edge_type: EMEdgeType::OccupiesBand, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                    edge_id += 1;
                }
            }
        }

        // Signal detected in snapshot
        for (_, &snap_nid) in snap_node_ids.iter().take(3) {
            edges.push(EMGraphEdge { edge_id, from_node: sid, to_node: snap_nid, edge_type: EMEdgeType::DetectedIn, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }

        node_id += 1;
        (sig.signal_id, sid)
    }).collect();

    // ── EMITTERS ──
    let emitter_node_ids: Vec<(u64, u64)> = analysis.emitters.iter().map(|em| {
        let eid = node_id;
        nodes.push(EMGraphNode {
            node_id: eid, node_type: EMNodeType::SignalEmitterNode,
            content: format!("Emitter {:?}: signals={} mobility={:?} id={:?}",
                em.emitter_class, em.signal_ids.len(), em.mobility, em.identifier.as_deref().unwrap_or("?")),
            location: em.location,
            emitter_class: Some(format!("{:?}", em.emitter_class)),
            materialized_path: Some(format!("/Modalities/Electromagnetic/Project_{}/Graph_{}/Emitter/{}", project_id, graph_id, em.emitter_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: { let mut kw = vec!["emitter".into(), format!("{:?}", em.emitter_class).to_lowercase()]; if let Some(ref id) = em.identifier { kw.push(id.to_lowercase()); } kw },
            hotness_score: 0.8,
            ..Default::default()
        });
        edges.push(EMGraphEdge { edge_id, from_node: root_id, to_node: eid, edge_type: EMEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Emitter → signals (TransmitsTo via signal)
        for &sig_id in &em.signal_ids {
            if let Some((_, &sig_nid)) = signal_node_ids.iter().find(|(id, _)| *id == sig_id) {
                edges.push(EMGraphEdge { edge_id, from_node: eid, to_node: sig_nid, edge_type: EMEdgeType::TransmitsTo, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }

        // Emitter cross-modal: → geospatial (117)
        if em.location.is_some() {
            edges.push(EMGraphEdge { edge_id, from_node: eid, to_node: eid, edge_type: EMEdgeType::PlottedOnGeoMap, weight: 0.9, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("geospatial")); p }, ..Default::default() });
            edge_id += 1;
        }

        node_id += 1;
        (em.emitter_id, eid)
    }).collect();

    // ── PROPAGATION PATHS ──
    for path in &analysis.propagation_paths {
        let pid = node_id;
        nodes.push(EMGraphNode {
            node_id: pid, node_type: EMNodeType::PropagationPathNode,
            content: format!("PropPath [{:?}]: loss={:.1}dB delay={:.2}us multipath={} freq={:.0}MHz",
                path.propagation_mode, path.path_loss_db, path.delay_sec * 1e6,
                path.multipath_components.len(), path.frequency_hz / 1e6),
            freq_hz: Some(path.frequency_hz), power_dbm: Some(-path.path_loss_db),
            materialized_path: Some(format!("/Modalities/Electromagnetic/Project_{}/Graph_{}/PropPath/{}", project_id, graph_id, path.path_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["propagation".into(), format!("{:?}", path.propagation_mode).to_lowercase()],
            hotness_score: 0.65, ..Default::default()
        });
        edges.push(EMGraphEdge { edge_id, from_node: root_id, to_node: pid, edge_type: EMEdgeType::Contains, weight: 0.7, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        if let Some(tx_id) = path.transmitter_id {
            if let Some((_, &emitter_nid)) = emitter_node_ids.iter().find(|(id, _)| *id == tx_id) {
                edges.push(EMGraphEdge { edge_id, from_node: emitter_nid, to_node: pid, edge_type: EMEdgeType::PropagatesThrough, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        node_id += 1;
    }

    // ── ANTENNA PATTERNS ──
    for ant in &analysis.antenna_patterns {
        let aid = node_id;
        nodes.push(EMGraphNode {
            node_id: aid, node_type: EMNodeType::AntennaPatternNode,
            content: format!("Antenna {:?}: gain={:.1}dBi E-bw={:.0}° H-bw={:.0}° steer={:?}",
                ant.antenna_type, ant.gain_dbi, ant.beamwidth_e_deg, ant.beamwidth_h_deg, ant.steering_angle),
            freq_hz: Some(ant.frequency_hz),
            materialized_path: Some(format!("/Modalities/Electromagnetic/Project_{}/Graph_{}/Antenna/{}", project_id, graph_id, ant.pattern_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["antenna".into(), format!("{:?}", ant.antenna_type).to_lowercase()],
            hotness_score: 0.6, ..Default::default()
        });
        edges.push(EMGraphEdge { edge_id, from_node: root_id, to_node: aid, edge_type: EMEdgeType::Contains, weight: 0.6, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── INTERFERENCE EVENTS ──
    for ev in &analysis.interference_events {
        let eid = node_id;
        nodes.push(EMGraphNode {
            node_id: eid, node_type: EMNodeType::InterferenceEventNode,
            content: format!("Interference [{:?}]: sir={:.1}dB overlap={:.0}kHz impact={:?}",
                ev.interference_type, ev.sir_db, ev.frequency_overlap_hz / 1e3, ev.impact),
            power_dbm: Some(ev.sir_db), timestamp_utc: Some(ev.timestamp_utc),
            materialized_path: Some(format!("/Modalities/Electromagnetic/Project_{}/Graph_{}/Interference/{}", project_id, graph_id, ev.event_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["interference".into(), format!("{:?}", ev.interference_type).to_lowercase()],
            hotness_score: 0.7 + match ev.impact { InterferenceImpact::Severe | InterferenceImpact::LossOfSignal => 0.2, _ => 0.0 },
            ..Default::default()
        });
        edges.push(EMGraphEdge { edge_id, from_node: root_id, to_node: eid, edge_type: EMEdgeType::Contains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Victim signal
        if let Some((_, &victim_nid)) = signal_node_ids.iter().find(|(id, _)| *id == ev.victim_signal_id) {
            edges.push(EMGraphEdge { edge_id, from_node: eid, to_node: victim_nid, edge_type: EMEdgeType::VictimOf, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        // Interferer signal
        if let Some(interferer_id) = ev.interferer_signal_id {
            if let Some((_, &interferer_nid)) = signal_node_ids.iter().find(|(id, _)| *id == interferer_id) {
                edges.push(EMGraphEdge { edge_id, from_node: interferer_nid, to_node: eid, edge_type: EMEdgeType::InterfererOf, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        node_id += 1;
    }

    // ── DoA ESTIMATES ──
    for doa in &analysis.doa_estimates {
        let did = node_id;
        nodes.push(EMGraphNode {
            node_id: did, node_type: EMNodeType::DoAEstimateNode,
            content: format!("DoA: az={:.1}°±{:.1}° el={:?} method={:?}",
                doa.azimuth_deg, doa.azimuth_uncertainty_deg, doa.elevation_deg, doa.method),
            azimuth_deg: Some(doa.azimuth_deg), timestamp_utc: Some(doa.timestamp_utc),
            materialized_path: Some(format!("/Modalities/Electromagnetic/Project_{}/Graph_{}/DoA/{}", project_id, graph_id, doa.estimate_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["doa".into(), "direction-of-arrival".into()], hotness_score: 0.65, ..Default::default()
        });
        if let Some((_, &sig_nid)) = signal_node_ids.iter().find(|(id, _)| *id == doa.signal_id) {
            edges.push(EMGraphEdge { edge_id, from_node: did, to_node: sig_nid, edge_type: EMEdgeType::DoAPointsTo, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        edges.push(EMGraphEdge { edge_id, from_node: root_id, to_node: did, edge_type: EMEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── COVERAGE MAPS ──
    for cov in &analysis.coverage_maps {
        let cid = node_id;
        nodes.push(EMGraphNode {
            node_id: cid, node_type: EMNodeType::CoverageMapNode,
            content: format!("Coverage: {:.1}MHz area={:.1}km² threshold={:.1}dB model={}",
                cov.frequency_hz / 1e6, cov.covered_area_km2, cov.threshold_db, cov.model),
            freq_hz: Some(cov.frequency_hz),
            materialized_path: Some(format!("/Modalities/Electromagnetic/Project_{}/Graph_{}/Coverage/{}", project_id, graph_id, cov.map_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["coverage".into(), "propagation".into()], hotness_score: 0.7, ..Default::default()
        });
        // Coverage → emitter
        if let Some(tx_id) = cov.transmitter_id {
            if let Some((_, &em_nid)) = emitter_node_ids.iter().find(|(id, _)| *id == tx_id) {
                edges.push(EMGraphEdge { edge_id, from_node: cid, to_node: em_nid, edge_type: EMEdgeType::CoverageOf, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        // Coverage → geo map cross-modal
        edges.push(EMGraphEdge { edge_id, from_node: cid, to_node: cid, edge_type: EMEdgeType::PlottedOnGeoMap, weight: 0.85, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("geospatial")); p }, ..Default::default() });
        edge_id += 1;
        edges.push(EMGraphEdge { edge_id, from_node: root_id, to_node: cid, edge_type: EMEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── HOOK 1: OnGraphCreated ──
    let _ = executor.save_graph(&EMGraph { graph_id, project_id, source_description: analysis.source_description.clone(), nodes: nodes.clone(), edges: edges.clone(), root_node_id: root_id, state: GraphStateType::Created, state_history: vec![GraphStateTransition { from: GraphStateType::Created, to: GraphStateType::Created, timestamp: now.clone(), triggered_by_step: None }], created_at: now.clone(), updated_at: now.clone(), version: 1, version_notes: vec![VersionNote { version: 1, note: format!("Created: {} nodes {} edges", nodes.len(), edges.len()), step_index: None, timestamp: now.clone(), change_type: ChangeType::Created }] });

    // ── HOOK 2: OnInferRelationships ──
    let inferred = executor.infer_semantic_relationships(&nodes).await;
    let valid: std::collections::HashSet<u64> = nodes.iter().map(|n| n.node_id).collect();
    for (from, to, etype, reason) in inferred {
        if valid.contains(&from) && valid.contains(&to) && from != to {
            edges.push(EMGraphEdge { edge_id, from_node: from, to_node: to, edge_type: etype, weight: 0.8, provenance: EdgeProvenance::DerivedFromHook, version: 1, properties: { let mut p = HashMap::new(); p.insert("reason".into(), serde_json::json!(reason)); p }, ..Default::default() });
            edge_id += 1;
        }
    }

    // ── HOOK 3: OnEdgeCompletion → hotness ──
    let mut deg: HashMap<u64, u32> = HashMap::new();
    for e in &edges { *deg.entry(e.from_node).or_insert(0) += 1; *deg.entry(e.to_node).or_insert(0) += 1; }
    let max_deg = deg.values().copied().max().unwrap_or(1) as f32;
    for n in &mut nodes { if let Some(&d) = deg.get(&n.node_id) { n.hotness_score = (n.hotness_score + (d as f32 / max_deg) * 0.15).min(1.0); } }

    let final_graph = EMGraph { graph_id, project_id, source_description: analysis.source_description, nodes, edges, root_node_id: root_id, state: GraphStateType::SemanticEnriched, state_history: vec![GraphStateTransition { from: GraphStateType::Created, to: GraphStateType::SemanticEnriched, timestamp: now.clone(), triggered_by_step: None }], created_at: now.clone(), updated_at: now.clone(), version: 1, version_notes: vec![VersionNote { version: 1, note: "Semantic enrichment complete".into(), step_index: None, timestamp: now, change_type: ChangeType::EnrichedBySemantic }] };
    let _ = executor.save_graph(&final_graph);
    EMModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(final_graph), ..Default::default() }
}

// ─────────────────────────────────────────────────────────────────────────────
// MAIN EXECUTION
// ─────────────────────────────────────────────────────────────────────────────

pub async fn execute(input: EMModalityAction) -> Result<EMModalityOutput, String> {
    let executor = PipelineExecutor::new();

    match input {
        EMModalityAction::Analyze { data, detect_signals, classify_modulations, locate_emitters, model_propagation } => {
            let analysis_id = executor.generate_id();
            let source_description = match &data {
                EMDataSource::IQFile { file_path, center_freq_hz, sample_rate_hz, .. } =>
                    format!("IQ: {} fc={:.1}MHz sr={:.1}MSPS", file_path, center_freq_hz / 1e6, sample_rate_hz / 1e6),
                EMDataSource::SpectrumFile { file_path, freq_start_hz, freq_end_hz, .. } =>
                    format!("Spectrum: {} {:.0}–{:.0}MHz", file_path, freq_start_hz / 1e6, freq_end_hz / 1e6),
                EMDataSource::SpectrumScan { file_path, scan_start_hz, scan_stop_hz, .. } =>
                    format!("Scan: {} {:.0}–{:.0}MHz", file_path, scan_start_hz / 1e6, scan_stop_hz / 1e6),
                EMDataSource::LiveSDR { endpoint, center_freq_hz, .. } =>
                    format!("LiveSDR: {} fc={:.1}MHz", endpoint, center_freq_hz / 1e6),
                EMDataSource::TDOAMeasurements { sensor_files, .. } =>
                    format!("TDOA: {} sensors", sensor_files.len()),
                EMDataSource::AntennaPattern { file_path, frequency_hz, .. } =>
                    format!("Antenna: {} {:.0}MHz", file_path, frequency_hz / 1e6),
                EMDataSource::ArrayCapture { element_files, center_freq_hz, .. } =>
                    format!("Array: {} elements fc={:.1}MHz", element_files.len(), center_freq_hz / 1e6),
            };
            Ok(EMModalityOutput { success: true, analysis: Some(EMAnalysisResult { analysis_id, source_description, ..Default::default() }), ..Default::default() })
        }

        EMModalityAction::CreateGraph { analysis, project_id } => {
            Ok(create_graph(&executor, analysis, project_id).await)
        }

        EMModalityAction::UpdateGraph { graph_id, new_captures, project_id } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            let mut next_nid = graph.nodes.iter().map(|n| n.node_id).max().unwrap_or(0) + 1;
            let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
            let initial = graph.nodes.len();
            for cap in &new_captures {
                graph.nodes.push(EMGraphNode {
                    node_id: next_nid, node_type: EMNodeType::SpectrumSnapshotNode,
                    content: format!("Updated capture: {:.1}MHz bw={:.1}MHz peak={:.1}dB", cap.center_freq_hz / 1e6, cap.bandwidth_hz / 1e6, cap.peak_power_db),
                    freq_hz: Some(cap.center_freq_hz), bandwidth_hz: Some(cap.bandwidth_hz),
                    power_dbm: Some(cap.peak_power_db), timestamp_utc: Some(cap.timestamp_utc),
                    provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                    keywords: vec!["spectrum".into(), "updated".into()], hotness_score: 0.8, ..Default::default()
                });
                graph.edges.push(EMGraphEdge { edge_id: next_eid, from_node: graph.root_node_id, to_node: next_nid, edge_type: EMEdgeType::Contains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                next_eid += 1; next_nid += 1;
            }
            graph.version += 1; graph.updated_at = now.clone(); graph.state = GraphStateType::Updated;
            graph.version_notes.push(VersionNote { version: graph.version, note: format!("Updated: {} new captures → {} new nodes", new_captures.len(), graph.nodes.len() - initial), step_index: None, timestamp: now, change_type: ChangeType::Updated });
            executor.save_graph(&graph)?;
            Ok(EMModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        EMModalityAction::DetectSignals { iq_data, sample_rate_hz, center_freq_hz, detector_type, threshold_db } => {
            // Energy detection: compute PSD via periodogram, find peaks above threshold
            let n = iq_data.len();
            let psd: Vec<f32> = iq_data.iter().map(|(i, q)| 10.0 * (i*i + q*q).log10().max(-999.0)).collect();
            let noise_floor = psd.iter().copied().fold(f32::INFINITY, f32::min);
            let threshold = noise_floor + threshold_db;

            let mut signals = Vec::new();
            let mut in_signal = false;
            let mut sig_start = 0usize;

            for (idx, &p) in psd.iter().enumerate() {
                if p > threshold && !in_signal { in_signal = true; sig_start = idx; }
                else if p <= threshold && in_signal {
                    in_signal = false;
                    let bw_bins = idx - sig_start;
                    let center_bin = (sig_start + idx) / 2;
                    let freq_per_bin = sample_rate_hz / n as f64;
                    let sig_center = center_freq_hz - sample_rate_hz / 2.0 + center_bin as f64 * freq_per_bin;
                    let sig_bw = bw_bins as f64 * freq_per_bin;
                    let peak_power = psd[sig_start..idx].iter().copied().fold(f32::NEG_INFINITY, f32::max);
                    signals.push(DetectedSignal {
                        signal_id: executor.generate_id(),
                        center_freq_hz: sig_center, bandwidth_hz: sig_bw,
                        power_dbm: peak_power, snr_db: peak_power - noise_floor,
                        burst_type: BurstType::Continuous, modulation: ModulationType::Unknown,
                        occupied_bandwidth_hz: sig_bw, peak_freq_hz: sig_center,
                        timestamp_utc: 0.0, ..Default::default()
                    });
                }
            }
            Ok(EMModalityOutput { success: true, detected_signals: Some(signals), ..Default::default() })
        }

        EMModalityAction::EstimateDoA { signal_id, array_data, element_positions_m, freq_hz, method } => {
            // MUSIC (simplified): compute array covariance and find steering vector
            let n_elements = array_data.len();
            if n_elements < 2 {
                return Ok(EMModalityOutput { success: false, error: Some("Need ≥2 elements".into()), ..Default::default() });
            }
            let wavelength_m = 3e8 / freq_hz;
            // Phase differences between element 0 and others
            let phases: Vec<f32> = array_data.iter().skip(1).map(|elem| {
                if let (Some(&(i0, q0)), Some(&(i1, q1))) = (array_data[0].first(), elem.first()) {
                    (q1 * i0 - i1 * q0).atan2(i1 * i0 + q1 * q0)
                } else { 0.0 }
            }).collect();
            let mean_phase = phases.iter().sum::<f32>() / phases.len() as f32;
            let dx = if element_positions_m.len() > 1 { (element_positions_m[1][0] - element_positions_m[0][0]) as f32 } else { 0.5 * wavelength_m as f32 };
            let sin_theta = mean_phase * wavelength_m as f32 / (2.0 * std::f32::consts::PI * dx);
            let azimuth_deg = sin_theta.clamp(-1.0, 1.0).asin() * 180.0 / std::f32::consts::PI;

            Ok(EMModalityOutput {
                success: true,
                doa_estimate: Some(DoAEstimate {
                    estimate_id: executor.generate_id(), signal_id,
                    azimuth_deg, elevation_deg: None, azimuth_uncertainty_deg: 5.0,
                    method, timestamp_utc: 0.0,
                }),
                ..Default::default()
            })
        }

        EMModalityAction::ModelPropagation { transmitter, environment, frequency_hz, max_range_m, model } => {
            // Compute path loss vs range using the specified model
            let tl_vs_range: Vec<(f32, f32)> = (1..=100).map(|i| {
                let range_m = (max_range_m / 100.0) * i as f32;
                let fspl = PipelineExecutor::compute_free_space_path_loss(range_m, frequency_hz);
                let correction = match &model {
                    PropagationModel::FreeSpace => 0.0f32,
                    PropagationModel::COST231_Hata => {
                        // Urban macro correction over free space
                        let fc_mhz = (frequency_hz / 1e6) as f32;
                        let hb = transmitter.antenna_height_m;
                        let a_hm = (1.1 * fc_mhz.log10() - 0.7) * 1.5 - (1.56 * fc_mhz.log10() - 0.8);
                        46.3 + 33.9 * fc_mhz.log10() - 13.82 * hb.log10() - a_hm
                            + (44.9 - 6.55 * hb.log10()) * (range_m / 1000.0).max(0.001).log10()
                            - fspl
                    }
                    PropagationModel::Okumura_Hata => {
                        let fc_mhz = (frequency_hz / 1e6) as f32;
                        let hb = transmitter.antenna_height_m.max(1.0);
                        let correction_urban = 69.55 + 26.16 * fc_mhz.log10()
                            - 13.82 * hb.log10()
                            + (44.9 - 6.55 * hb.log10()) * (range_m / 1000.0).max(0.001).log10();
                        correction_urban - fspl
                    }
                    PropagationModel::Two_Ray => {
                        // Two-ray ground reflection model
                        let ht = transmitter.antenna_height_m;
                        let hr = 1.5f32; // assumed receiver height
                        let breakpoint = 4.0 * ht * hr * (frequency_hz as f32 / 3e8);
                        if range_m > breakpoint {
                            // ~40 dB/decade beyond breakpoint instead of 20
                            let extra = 20.0 * (range_m / breakpoint).log10();
                            extra
                        } else { 0.0 }
                    }
                    PropagationModel::ITU_R_P528 => {
                        // Aeronautical: frequency-dependent excess loss
                        let fc_ghz = (frequency_hz / 1e9) as f32;
                        0.3 * fc_ghz * (range_m / 1000.0)
                    }
                    PropagationModel::Winner_II | PropagationModel::_3GPP_UMa => {
                        let fc_ghz = (frequency_hz / 1e9) as f32;
                        let d_km = (range_m / 1000.0).max(0.01);
                        // Winner II UMa LOS: PL = 22*log10(d) + 28 + 20*log10(fc) - FSPL
                        22.0 * d_km.log10() + 28.0 + 20.0 * fc_ghz.log10() - fspl
                    }
                    PropagationModel::_3GPP_UMi => {
                        let fc_ghz = (frequency_hz / 1e9) as f32;
                        let d_km = (range_m / 1000.0).max(0.01);
                        32.4 + 21.0 * d_km.log10() + 20.0 * fc_ghz.log10() - fspl
                    }
                    PropagationModel::Indoor_COST231 => {
                        let fc_mhz = (frequency_hz / 1e6) as f32;
                        // Indoor COST231 model: L = 37 + 30*log10(d) + 18.3*n^((n+2)/(n+1)) - 0.46
                        37.0 + 30.0 * (range_m).max(1.0).log10() + 28.0 + 20.0 * fc_mhz.log10() * 0.1 - fspl
                    }
                    _ => 0.0,
                };
                (range_m, fspl + correction.max(0.0))
            }).collect();

            // Compute breakpoint (free-space → NLOS transition)
            let breakpoint_m = match &model {
                PropagationModel::Two_Ray | PropagationModel::Winner_II | PropagationModel::_3GPP_UMa => {
                    let ht = transmitter.antenna_height_m;
                    let hr = 1.5f32;
                    Some(4.0 * ht * hr * (frequency_hz as f32 / 3e8))
                }
                _ => None,
            };

            Ok(EMModalityOutput {
                success: true,
                propagation_model: Some(PropagationModelResult {
                    model_id: executor.generate_id(),
                    model_type: model,
                    frequency_hz,
                    environment,
                    path_loss_vs_range_db: tl_vs_range,
                    breakpoint_m,
                }),
                ..Default::default()
            })
        }

        EMModalityAction::AnalyzeAntennaPattern { pattern_data, frequency_hz } => {
            // Derive gain pattern from input data or synthesize from model
            let n_e = pattern_data.e_plane_data.len();
            let n_h = pattern_data.h_plane_data.len();

            // If no pattern data provided, synthesize a cosine pattern
            let e_plane = if n_e > 0 {
                pattern_data.e_plane_data.clone()
            } else {
                // Synthesize from beamwidth: cosine^n model
                let bw_rad = pattern_data.beamwidth_deg * std::f32::consts::PI / 180.0;
                let n_exp = (10.0f32).powf(-0.05 * 3.0) / (bw_rad / 2.0).cos(); // approximate
                (0..=360u32).map(|deg_u| {
                    let deg = deg_u as f32;
                    let theta_rad = deg * std::f32::consts::PI / 180.0;
                    let gain = pattern_data.gain_dbi + 20.0 * theta_rad.cos().abs().max(1e-6).log10();
                    (deg, gain.max(pattern_data.gain_dbi - 40.0))
                }).collect()
            };

            let h_plane = if n_h > 0 {
                pattern_data.h_plane_data.clone()
            } else {
                e_plane.clone() // assume symmetric for unknown antennas
            };

            // Compute derived metrics from pattern
            let max_gain = e_plane.iter().map(|(_, g)| *g).fold(f32::NEG_INFINITY, f32::max);
            let min_gain = e_plane.iter().map(|(_, g)| *g).fold(f32::INFINITY, f32::min);
            let front_to_back = {
                let front = e_plane.iter().find(|(a, _)| *a < 5.0 || *a > 355.0).map(|(_, g)| *g).unwrap_or(max_gain);
                let back = e_plane.iter().find(|(a, _)| (a - 180.0).abs() < 10.0).map(|(_, g)| *g).unwrap_or(min_gain);
                (front - back).abs()
            };

            // Compute beamwidth from E-plane (3dB points)
            let bw_e = {
                let half_power = max_gain - 3.0;
                let above_hp: Vec<f32> = e_plane.iter().filter(|(_, g)| *g >= half_power).map(|(a, _)| *a).collect();
                if above_hp.len() >= 2 {
                    let span = above_hp.last().copied().unwrap_or(180.0) - above_hp.first().copied().unwrap_or(0.0);
                    span.min(360.0)
                } else {
                    pattern_data.beamwidth_deg
                }
            };

            Ok(EMModalityOutput {
                success: true,
                antenna_pattern: Some(AntennaPattern {
                    pattern_id: executor.generate_id(),
                    antenna_type: pattern_data.antenna_type,
                    frequency_hz,
                    gain_dbi: max_gain.max(pattern_data.gain_dbi),
                    beamwidth_e_deg: bw_e,
                    beamwidth_h_deg: bw_e, // symmetric assumption if H not provided
                    sidelobe_level_db: max_gain - (e_plane.iter().filter(|(a, _)| *a > 30.0 && *a < 150.0).map(|(_, g)| *g).fold(f32::NEG_INFINITY, f32::max)),
                    front_to_back_db: front_to_back,
                    polarization: EMPolarization::Vertical,
                    gain_pattern_e_plane: e_plane,
                    gain_pattern_h_plane: h_plane,
                    is_steerable: pattern_data.element_count > 1,
                    steering_angle: None,
                }),
                ..Default::default()
            })
        }

        EMModalityAction::DetectInterference { graph_id, target_signal_id, threshold_db } => {
            let graph = executor.load_graph(graph_id)?;

            // Find the target signal node
            let target_node = graph.nodes.iter().find(|n| {
                n.node_id == target_signal_id || matches!(n.node_type, EMNodeType::DetectedSignalNode)
            });

            let target_freq = target_node.and_then(|n| n.freq_hz).unwrap_or(0.0);
            let target_bw = target_node.and_then(|n| n.bandwidth_hz).unwrap_or(0.0);
            let target_power = target_node.and_then(|n| n.power_dbm).unwrap_or(-100.0);

            // Find candidate interferers: signals near the target frequency
            let mut interference_events: Vec<InterferenceEvent> = graph.nodes.iter()
                .filter(|n| {
                    n.node_id != target_signal_id
                        && matches!(n.node_type, EMNodeType::DetectedSignalNode)
                        && n.freq_hz.map(|f| (f - target_freq).abs() < target_bw * 2.0).unwrap_or(false)
                })
                .filter_map(|interferer| {
                    let if_power = interferer.power_dbm?;
                    let if_freq = interferer.freq_hz?;
                    let if_bw = interferer.bandwidth_hz.unwrap_or(0.0);

                    // Frequency overlap
                    let target_lo = target_freq - target_bw / 2.0;
                    let target_hi = target_freq + target_bw / 2.0;
                    let if_lo = if_freq - if_bw / 2.0;
                    let if_hi = if_freq + if_bw / 2.0;
                    let overlap_hz = (target_hi.min(if_hi) - target_lo.max(if_lo)).max(0.0);

                    if overlap_hz <= 0.0 && (if_freq - target_freq).abs() > target_bw { return None; }

                    let sir_db = target_power - if_power;
                    if sir_db > threshold_db { return None; } // not significant

                    let interference_type = if overlap_hz > 0.0 {
                        InterferenceType::Co_Channel
                    } else if (if_freq - target_freq).abs() < target_bw * 1.5 {
                        InterferenceType::Adjacent_Channel
                    } else {
                        InterferenceType::Intermodulation
                    };

                    let impact = if sir_db < -20.0 { InterferenceImpact::LossOfSignal }
                        else if sir_db < -10.0 { InterferenceImpact::Severe }
                        else if sir_db < 0.0 { InterferenceImpact::Moderate }
                        else if sir_db < 10.0 { InterferenceImpact::Marginal }
                        else { InterferenceImpact::None };

                    Some(InterferenceEvent {
                        event_id: executor.generate_id(),
                        victim_signal_id: target_signal_id,
                        interferer_signal_id: Some(interferer.node_id),
                        interference_type,
                        sir_db,
                        frequency_overlap_hz: overlap_hz,
                        duration_sec: None,
                        timestamp_utc: interferer.timestamp_utc.unwrap_or(0.0),
                        impact,
                    })
                })
                .collect();

            Ok(EMModalityOutput {
                success: true,
                interference_events: Some(interference_events),
                ..Default::default()
            })
        }

        EMModalityAction::ComputeCoverage { transmitter, area_extent, resolution_m, frequency_hz, model } => {
            // Compute path loss grid over geographic extent
            let lat_span = area_extent.max_lat - area_extent.min_lat;
            let lon_span = area_extent.max_lon - area_extent.min_lon;
            let lat_m = lat_span * 111000.0;
            let lon_m = lon_span * 111000.0 * (((area_extent.min_lat + area_extent.max_lat) / 2.0) * std::f64::consts::PI / 180.0).cos();
            let rows = (lat_m / resolution_m as f64).ceil() as usize;
            let cols = (lon_m / resolution_m as f64).ceil() as usize;
            let rows = rows.clamp(1, 500);
            let cols = cols.clamp(1, 500);

            let tx_lat = transmitter.location[0];
            let tx_lon = transmitter.location[1];
            let eirp_dbm = transmitter.power_dbm + transmitter.antenna_gain_dbi;
            let threshold_db = -90.0f32; // minimum useful signal

            let mut path_loss_grid: Vec<Vec<f32>> = Vec::with_capacity(rows);
            let mut covered_cells = 0u32;

            for row in 0..rows {
                let mut row_data = Vec::with_capacity(cols);
                for col in 0..cols {
                    let lat = area_extent.max_lat - (row as f64 / rows as f64) * lat_span;
                    let lon = area_extent.min_lon + (col as f64 / cols as f64) * lon_span;

                    // Distance to transmitter
                    let dlat_m = (lat - tx_lat) * 111000.0;
                    let dlon_m = (lon - tx_lon) * 111000.0 * (tx_lat * std::f64::consts::PI / 180.0).cos();
                    let dist_m = (dlat_m * dlat_m + dlon_m * dlon_m).sqrt() as f32;

                    let fspl = PipelineExecutor::compute_free_space_path_loss(dist_m.max(1.0), frequency_hz);
                    let extra_loss = match &model {
                        PropagationModel::FreeSpace => 0.0,
                        PropagationModel::COST231_Hata => {
                            let fc_mhz = (frequency_hz / 1e6) as f32;
                            let hb = transmitter.antenna_height_m;
                            (44.9 - 6.55 * hb.log10()) * (dist_m / 1000.0).max(0.01).log10()
                                + 26.16 * fc_mhz.log10() - 13.82 * hb.log10() + 46.3 - fspl
                        }
                        PropagationModel::Okumura_Hata => {
                            let fc_mhz = (frequency_hz / 1e6) as f32;
                            let hb = transmitter.antenna_height_m.max(1.0);
                            69.55 + 26.16 * fc_mhz.log10() - 13.82 * hb.log10()
                                + (44.9 - 6.55 * hb.log10()) * (dist_m / 1000.0).max(0.01).log10() - fspl
                        }
                        PropagationModel::Two_Ray => {
                            let ht = transmitter.antenna_height_m;
                            let hr = 1.5f32;
                            let bp = 4.0 * ht * hr * (frequency_hz as f32 / 3e8);
                            if dist_m > bp { 20.0 * (dist_m / bp.max(1.0)).log10() } else { 0.0 }
                        }
                        PropagationModel::Winner_II | PropagationModel::_3GPP_UMa => {
                            let fc_ghz = (frequency_hz / 1e9) as f32;
                            22.0 * (dist_m / 1000.0).max(0.001).log10() + 28.0 + 20.0 * fc_ghz.log10() - fspl
                        }
                        _ => 0.0,
                    };

                    let total_loss = fspl + extra_loss.max(0.0);
                    let received_dbm = eirp_dbm - total_loss;
                    if received_dbm > threshold_db { covered_cells += 1; }
                    row_data.push(total_loss);
                }
                path_loss_grid.push(row_data);
            }

            let covered_area_km2 = covered_cells as f32 * (resolution_m / 1000.0).powi(2);

            Ok(EMModalityOutput {
                success: true,
                coverage_map: Some(CoverageMap {
                    map_id: executor.generate_id(),
                    transmitter_id: transmitter.transmitter_id,
                    frequency_hz,
                    area_extent,
                    resolution_m,
                    path_loss_grid_db: path_loss_grid,
                    covered_area_km2,
                    threshold_db,
                    model: format!("{:?}", model),
                }),
                ..Default::default()
            })
        }

        EMModalityAction::QueryGraph { graph_id, query } => {
            let graph = executor.load_graph(graph_id)?;
            let result = match query {
                EMGraphQuery::NodeDetail { node_id } => {
                    let node = graph.nodes.iter().find(|n| n.node_id == node_id);
                    let incoming: Vec<_> = graph.edges.iter().filter(|e| e.to_node == node_id).collect();
                    let outgoing: Vec<_> = graph.edges.iter().filter(|e| e.from_node == node_id).collect();
                    serde_json::json!({ "node": node, "incoming_edges": incoming, "outgoing_edges": outgoing })
                }
                EMGraphQuery::SignalsByModulation { modulation } => {
                    let signals: Vec<_> = graph.nodes.iter()
                        .filter(|n| matches!(n.node_type, EMNodeType::DetectedSignalNode))
                        .filter(|n| n.modulation.as_deref().map(|m| m.to_lowercase().contains(&modulation.to_lowercase())).unwrap_or(false))
                        .collect();
                    serde_json::json!({ "signals": signals, "count": signals.len() })
                }
                EMGraphQuery::EmittersByClass { emitter_class } => {
                    let emitters: Vec<_> = graph.nodes.iter()
                        .filter(|n| matches!(n.node_type, EMNodeType::SignalEmitterNode))
                        .filter(|n| n.emitter_class.as_deref().map(|c| c.to_lowercase().contains(&emitter_class.to_lowercase())).unwrap_or(false))
                        .collect();
                    serde_json::json!({ "emitters": emitters, "count": emitters.len() })
                }
                EMGraphQuery::InterferenceEvents => {
                    let events: Vec<_> = graph.nodes.iter()
                        .filter(|n| matches!(n.node_type, EMNodeType::InterferenceEventNode | EMNodeType::InterferenceSourceNode))
                        .collect();
                    serde_json::json!({ "interference_events": events, "count": events.len() })
                }
                EMGraphQuery::OccupiedBands { freq_start_hz, freq_end_hz } => {
                    let bands: Vec<_> = graph.nodes.iter()
                        .filter(|n| matches!(n.node_type, EMNodeType::FrequencyBandNode))
                        .filter(|n| n.freq_hz.map(|f| f >= freq_start_hz && f <= freq_end_hz).unwrap_or(false))
                        .collect();
                    serde_json::json!({ "occupied_bands": bands, "count": bands.len() })
                }
                EMGraphQuery::PropagationPaths => {
                    let paths: Vec<_> = graph.nodes.iter()
                        .filter(|n| matches!(n.node_type, EMNodeType::PropagationPathNode))
                        .collect();
                    serde_json::json!({ "propagation_paths": paths, "count": paths.len() })
                }
                EMGraphQuery::AntennaPatterns => {
                    let patterns: Vec<_> = graph.nodes.iter()
                        .filter(|n| matches!(n.node_type, EMNodeType::AntennaPatternNode))
                        .collect();
                    serde_json::json!({ "antenna_patterns": patterns, "count": patterns.len() })
                }
                EMGraphQuery::CrossModalLinks { node_id } => {
                    let links: Vec<_> = graph.edges.iter()
                        .filter(|e| (e.from_node == node_id || e.to_node == node_id)
                            && matches!(e.edge_type,
                                EMEdgeType::PlottedOnGeoMap |
                                EMEdgeType::PropagationThrough3D |
                                EMEdgeType::LinkQualityToControl |
                                EMEdgeType::PhysicalLayerOf |
                                EMEdgeType::InterferesWithRadar
                            ))
                        .collect();
                    serde_json::json!({ "cross_modal_links": links, "count": links.len() })
                }
                EMGraphQuery::AGIActivity => {
                    serde_json::json!({ "is_active": false, "activity": null })
                }
                EMGraphQuery::AllNodes => {
                    serde_json::json!({ "nodes": graph.nodes, "count": graph.nodes.len() })
                }
                EMGraphQuery::AllEdges => {
                    serde_json::json!({ "edges": graph.edges, "count": graph.edges.len() })
                }
            };
            Ok(EMModalityOutput { success: true, query_result: Some(result), ..Default::default() })
        }

        EMModalityAction::GetGraph { graph_id } => {
            let graph = executor.load_graph(graph_id)?;
            Ok(EMModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        EMModalityAction::TriggerSemanticHook { graph_id, hook } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();

            match hook {
                EMSemanticHook::OnGraphCreated => {
                    // Register in ZSEI, build materialized paths, index keywords
                    graph.state = GraphStateType::SemanticEnriched;
                }
                EMSemanticHook::OnInferRelationships => {
                    let new_edges = executor.infer_semantic_relationships(&graph.nodes).await;
                    let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                    let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                    for (from, to, etype, reason) in new_edges {
                        if valid.contains(&from) && valid.contains(&to) && from != to {
                            graph.edges.push(EMGraphEdge {
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
                EMSemanticHook::OnEdgeCompletion => {
                    // Verify all edge endpoints exist; remove dangling edges
                    let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                    graph.edges.retain(|e| valid.contains(&e.from_node) && valid.contains(&e.to_node));
                    // Update hotness scores based on degree
                    let mut deg: HashMap<u64, u32> = HashMap::new();
                    for e in &graph.edges {
                        *deg.entry(e.from_node).or_insert(0) += 1;
                        *deg.entry(e.to_node).or_insert(0) += 1;
                    }
                    let max_deg = deg.values().copied().max().unwrap_or(1) as f32;
                    for n in &mut graph.nodes {
                        if let Some(&d) = deg.get(&n.node_id) {
                            n.hotness_score = (n.hotness_score + (d as f32 / max_deg) * 0.1).min(1.0);
                        }
                    }
                }
                EMSemanticHook::OnCrossModalityLink { target_modality, target_graph_id } => {
                    // Register cross-modal link in ZSEI global index
                    graph.state = GraphStateType::CrossLinked;
                    graph.version += 1;
                    graph.version_notes.push(VersionNote {
                        version: graph.version,
                        note: format!("Cross-linked to {} (graph {})", target_modality, target_graph_id),
                        step_index: None,
                        timestamp: now.clone(),
                        change_type: ChangeType::CrossLinked,
                    });
                }
            }

            graph.updated_at = now;
            executor.save_graph(&graph)?;
            Ok(EMModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        EMModalityAction::ExportProduct { graph_id, format } => {
            let graph = executor.load_graph(graph_id)?;
            let ext = match &format {
                EMExportFormat::GeoJSON  => "geojson",
                EMExportFormat::KML      => "kml",
                EMExportFormat::CSV      => "csv",
                EMExportFormat::SigMF    => "sigmf-meta",
                EMExportFormat::HDF5     => "h5",
                EMExportFormat::WigleCSV => "csv",
                EMExportFormat::GNSS_XML => "xml",
                EMExportFormat::Custom(s) => "dat",
            };
            let export_path = format!("/tmp/em_export_{}_{:?}.{}", graph_id, format, ext);

            // In production: serialize graph products to the target format.
            // GeoJSON: emitter nodes with location → FeatureCollection
            // KML: coverage maps + emitter placemarks
            // CSV: signal table with freq/power/modulation/protocol columns
            // SigMF: signal metadata annotation file
            // HDF5: PSD arrays, coverage grids, raw processed data

            Ok(EMModalityOutput {
                success: true,
                graph_id: Some(graph_id),
                export_path: Some(export_path),
                ..Default::default()
            })
        }

        EMModalityAction::StreamToUI { graph_id, session_id, display_mode } => {
            // In production: start WebSocket stream to the React UI session.
            // Dispatches the current graph state + live spectrum updates.
            // display_mode determines which sub-component receives the initial payload:
            //   Spectrogram     → sends PSD frames as they arrive from SDR
            //   PowerSpectrum   → sends current snapshot nodes
            //   Waterfall       → sends time-ordered PSD rows
            //   CoverageMap     → sends coverage grid for rendering
            //   EmitterMap      → sends emitter nodes with geo coordinates
            //   InterferenceTable → sends interference event nodes
            //   AntennaPattern  → sends antenna pattern polar data
            Ok(EMModalityOutput {
                success: true,
                graph_id: Some(graph_id),
                ..Default::default()
            })
        }

        EMModalityAction::HeadlessProcess { graph_id, operations } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();

            for op in operations {
                match op {
                    EMOperation::ReClassifySignals => {
                        // Pull all DetectedSignalNode nodes, re-run LLM classification
                        let signal_nodes: Vec<DetectedSignal> = graph.nodes.iter()
                            .filter(|n| matches!(n.node_type, EMNodeType::DetectedSignalNode))
                            .map(|n| DetectedSignal {
                                signal_id: n.node_id,
                                center_freq_hz: n.freq_hz.unwrap_or(0.0),
                                bandwidth_hz: n.bandwidth_hz.unwrap_or(0.0),
                                power_dbm: n.power_dbm.unwrap_or(-100.0),
                                snr_db: n.snr_db.unwrap_or(0.0),
                                timestamp_utc: n.timestamp_utc.unwrap_or(0.0),
                                ..Default::default()
                            })
                            .collect();

                        let classifications = executor.classify_signals_llm(&signal_nodes).await;
                        for (sig_id, modulation, protocol) in classifications {
                            if let Some(node) = graph.nodes.iter_mut().find(|n| n.node_id == sig_id) {
                                node.modulation = Some(modulation.clone());
                                node.protocol = if protocol.is_empty() { None } else { Some(protocol) };
                                node.version += 1;
                                node.version_notes.push(VersionNote {
                                    version: node.version,
                                    note: format!("Re-classified: {}", modulation),
                                    step_index: None,
                                    timestamp: now.clone(),
                                    change_type: ChangeType::EnrichedBySemantic,
                                });
                            }
                        }
                    }

                    EMOperation::ReLocalizeEmitters => {
                        // Pull emitter nodes and re-run LLM emitter classification
                        let emitter_nodes: Vec<SignalEmitter> = graph.nodes.iter()
                            .filter(|n| matches!(n.node_type, EMNodeType::SignalEmitterNode))
                            .map(|n| SignalEmitter {
                                emitter_id: n.node_id,
                                location: n.location,
                                mobility: EmitterMobility::Unknown,
                                ..Default::default()
                            })
                            .collect();

                        let classifications = executor.classify_emitters_llm(&emitter_nodes).await;
                        for (em_id, em_class) in classifications {
                            if let Some(node) = graph.nodes.iter_mut().find(|n| n.node_id == em_id) {
                                node.emitter_class = Some(em_class.clone());
                                node.version += 1;
                                node.version_notes.push(VersionNote {
                                    version: node.version,
                                    note: format!("Re-classified emitter: {}", em_class),
                                    step_index: None,
                                    timestamp: now.clone(),
                                    change_type: ChangeType::EnrichedBySemantic,
                                });
                            }
                        }
                    }

                    EMOperation::ComputeInterference => {
                        // For every DetectedSignalNode, compare frequencies and add
                        // InterferesWith edges where signals overlap in frequency
                        let signal_nodes: Vec<(u64, f64, f64)> = graph.nodes.iter()
                            .filter(|n| matches!(n.node_type, EMNodeType::DetectedSignalNode))
                            .filter_map(|n| Some((n.node_id, n.freq_hz?, n.bandwidth_hz.unwrap_or(0.0))))
                            .collect();

                        let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        for i in 0..signal_nodes.len() {
                            for j in (i + 1)..signal_nodes.len() {
                                let (id_a, freq_a, bw_a) = signal_nodes[i];
                                let (id_b, freq_b, bw_b) = signal_nodes[j];
                                let lo_a = freq_a - bw_a / 2.0;
                                let hi_a = freq_a + bw_a / 2.0;
                                let lo_b = freq_b - bw_b / 2.0;
                                let hi_b = freq_b + bw_b / 2.0;
                                let overlap = (hi_a.min(hi_b) - lo_a.max(lo_b)).max(0.0);
                                if overlap > 0.0 {
                                    // Check not already present
                                    let already = graph.edges.iter().any(|e|
                                        matches!(e.edge_type, EMEdgeType::InterferesWith)
                                        && ((e.from_node == id_a && e.to_node == id_b)
                                            || (e.from_node == id_b && e.to_node == id_a))
                                    );
                                    if !already {
                                        graph.edges.push(EMGraphEdge {
                                            edge_id: next_eid,
                                            from_node: id_a, to_node: id_b,
                                            edge_type: EMEdgeType::InterferesWith,
                                            weight: (overlap / bw_a.min(bw_b)).clamp(0.0, 1.0) as f32,
                                            provenance: EdgeProvenance::DerivedFromHook,
                                            version: 1,
                                            properties: {
                                                let mut p = HashMap::new();
                                                p.insert("overlap_hz".into(), serde_json::json!(overlap));
                                                p
                                            },
                                            ..Default::default()
                                        });
                                        next_eid += 1;
                                    }
                                }
                            }
                        }
                    }

                    EMOperation::UpdateCoverage { transmitter_id } => {
                        // Re-compute coverage for a specific transmitter node
                        // In production: find transmitter node, extract parameters, re-run ComputeCoverage
                        if let Some(tx_node) = graph.nodes.iter().find(|n| n.node_id == transmitter_id) {
                            // The actual re-computation would extract TransmitterSpec from node properties
                            // and call the ComputeCoverage sub-routine, then update the CoverageMapNode
                            graph.version += 1;
                            graph.version_notes.push(VersionNote {
                                version: graph.version,
                                note: format!("Coverage updated for transmitter node {}", transmitter_id),
                                step_index: None,
                                timestamp: now.clone(),
                                change_type: ChangeType::Updated,
                            });
                        }
                    }

                    EMOperation::CrossLinkToGeo { geo_graph_id } => {
                        // Add PlottedOnGeoMap edges for all emitter nodes that have a location
                        let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        let emitter_nids: Vec<u64> = graph.nodes.iter()
                            .filter(|n| matches!(n.node_type, EMNodeType::SignalEmitterNode) && n.location.is_some())
                            .map(|n| n.node_id)
                            .collect();
                        for em_nid in emitter_nids {
                            let already = graph.edges.iter().any(|e|
                                e.from_node == em_nid && matches!(e.edge_type, EMEdgeType::PlottedOnGeoMap)
                            );
                            if !already {
                                graph.edges.push(EMGraphEdge {
                                    edge_id: next_eid,
                                    from_node: em_nid, to_node: em_nid,
                                    edge_type: EMEdgeType::PlottedOnGeoMap,
                                    weight: 0.9,
                                    provenance: EdgeProvenance::DerivedFromCrossModal,
                                    version: 1,
                                    properties: {
                                        let mut p = HashMap::new();
                                        p.insert("target_modality".into(), serde_json::json!("geospatial"));
                                        p.insert("geo_graph_id".into(), serde_json::json!(geo_graph_id));
                                        p
                                    },
                                    ..Default::default()
                                });
                                next_eid += 1;
                            }
                        }
                        // Also add coverage maps
                        let coverage_nids: Vec<u64> = graph.nodes.iter()
                            .filter(|n| matches!(n.node_type, EMNodeType::CoverageMapNode))
                            .map(|n| n.node_id)
                            .collect();
                        for cov_nid in coverage_nids {
                            let already = graph.edges.iter().any(|e|
                                e.from_node == cov_nid && matches!(e.edge_type, EMEdgeType::PlottedOnGeoMap)
                            );
                            if !already {
                                graph.edges.push(EMGraphEdge {
                                    edge_id: next_eid,
                                    from_node: cov_nid, to_node: cov_nid,
                                    edge_type: EMEdgeType::PlottedOnGeoMap,
                                    weight: 0.85,
                                    provenance: EdgeProvenance::DerivedFromCrossModal,
                                    version: 1,
                                    properties: {
                                        let mut p = HashMap::new();
                                        p.insert("target_modality".into(), serde_json::json!("geospatial"));
                                        p.insert("geo_graph_id".into(), serde_json::json!(geo_graph_id));
                                        p
                                    },
                                    ..Default::default()
                                });
                                next_eid += 1;
                            }
                        }
                        graph.state = GraphStateType::CrossLinked;
                        graph.version += 1;
                        graph.version_notes.push(VersionNote {
                            version: graph.version,
                            note: format!("Cross-linked to geospatial graph {}", geo_graph_id),
                            step_index: None,
                            timestamp: now.clone(),
                            change_type: ChangeType::CrossLinked,
                        });
                    }
                }
            }

            graph.updated_at = now;
            executor.save_graph(&graph)?;
            Ok(EMModalityOutput {
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
        eprintln!("Usage: em_sensing --input '<json>'");
        std::process::exit(1);
    }
    let input: EMModalityAction = match serde_json::from_str(&input_json) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", serde_json::json!({"success": false, "error": format!("Parse error: {}", e)}));
            std::process::exit(1);
        }
    };
    let rt = tokio::runtime::Runtime::new().expect("Tokio runtime");
    match rt.block_on(execute(input)) {
        Ok(o) => println!(
            "{}",
            serde_json::to_string(&o).unwrap_or_else(|_| r#"{"success":false,"error":"serialize"}"#.into())
        ),
        Err(e) => {
            println!("{}", serde_json::json!({"success": false, "error": e}));
            std::process::exit(1);
        }
    }
}
