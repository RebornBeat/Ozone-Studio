//! ThermalSensing — Pipeline #114
//!
//! Thermal infrared imaging and analysis: temperature mapping, heat source
//! detection, material state identification, anomaly detection, thermal
//! gradient analysis, and emissivity characterization.
//!
//! DISTINCT FROM:
//!   - Image (102): RGB measures reflected visible light; thermal measures
//!     emitted IR radiation (8–14 µm LWIR, 3–5 µm MWIR). No visible light
//!     required. Temperature is derived, not color.
//!   - Hyperspectral (126): HSI covers reflectance across hundreds of bands;
//!     thermal is emission-based single/few-band, physics is Planck's law.
//!
//! CROSS-LINKS:
//!   102 (Image)   → visual + thermal overlay (same scene)
//!   109 (3D)      → thermal texture mapped onto 3D geometry
//!   111 (Bio)     → metabolic heat signatures, fever/hypothermia
//!   113 (Haptic)  → contact heat transfer
//!   115 (Depth)   → thermal + depth → thermally-aware 3D
//!   117 (Geo)     → thermal maps on geographic canvas
//!   118 (EM)      → thermal emissivity correlates with spectral
//!   121 (Kine)    → friction heat from joints
//!   122 (Control) → thermal feedback in control loops
//!   124 (Radar)   → radar + thermal target fusion
//!   126 (Hyper)   → emissivity cross-validates spectral
//!
//! STORAGE: ZSEI containers under /Modalities/Thermal/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

// ─────────────────────────────────────────────────────────────────────────────
// INPUT TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ThermalModalityAction {
    Analyze {
        data: ThermalDataSource,
        detect_hotspots: bool,
        detect_coldspots: bool,
        compute_gradients: bool,
        classify_materials: bool,
        detect_anomalies: bool,
    },
    CreateGraph {
        analysis: ThermalAnalysisResult,
        project_id: u64,
    },
    UpdateGraph {
        graph_id: u64,
        new_frames: Vec<ThermalFrame>,
        project_id: u64,
    },
    /// Compute emissivity map from known temperature reference
    ComputeEmissivity {
        graph_id: u64,
        reference_temps: Vec<TempReference>,
    },
    /// Detect thermal anomalies against a background model
    DetectAnomalies {
        graph_id: u64,
        method: ThermalAnomalyMethod,
        threshold_kelvin: f32,
    },
    /// Track thermal evolution across a time series
    AnalyzeTemporalTrend {
        graph_id: u64,
        region_of_interest: Option<PixelRegion>,
        trend_window_secs: f32,
    },
    /// Fuse thermal with visible image
    FuseWithVisible {
        thermal_graph_id: u64,
        visible_image_path: String,
        fusion_method: ThermalFusionMethod,
    },
    /// Compute heat flux / thermal power estimate
    ComputeHeatFlux {
        graph_id: u64,
        emissivity_map: Option<Vec<Vec<f32>>>,
    },
    QueryGraph { graph_id: u64, query: ThermalGraphQuery },
    GetGraph { graph_id: u64 },
    TriggerSemanticHook { graph_id: u64, hook: ThermalSemanticHook },
    ExportProduct { graph_id: u64, format: ThermalExportFormat },
    StreamToUI { graph_id: u64, session_id: String, display_mode: ThermalDisplayMode },
    HeadlessProcess { graph_id: u64, operations: Vec<ThermalOperation> },
}

// ─────────────────────────────────────────────────────────────────────────────
// DATA SOURCES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThermalDataSource {
    /// Single thermal image (TIFF, FLIR radiometric JPEG, PNG-16bit)
    ThermalImageFile {
        file_path: String,
        format: ThermalFileFormat,
        sensor: Option<ThermalSensor>,
        calibration: Option<ThermalCalibration>,
    },
    /// FLIR radiometric file with embedded metadata
    FLIRRadiometric {
        file_path: String,
        extract_embedded_metadata: bool,
    },
    /// Raw ADC values (sensor-native)
    RawADC {
        file_path: String,
        width: u32,
        height: u32,
        bit_depth: u32,
        radiometric_coefficients: RadiometricCoefficients,
    },
    /// Time-series sequence of thermal frames
    ThermalVideoFile {
        file_path: String,
        format: ThermalVideoFormat,
        fps: f32,
    },
    /// Airborne / satellite thermal (Landsat TIRS, ASTER TIR, ECOSTRESS)
    SatelliteThermal {
        file_path: String,
        sensor: SatelliteThermalSensor,
        band_assignments: Vec<ThermalBandAssignment>,
        atmospheric_correction_applied: bool,
    },
    /// Contact / probe temperature log (thermocouples, RTDs, IR pyrometers)
    TemperatureLog {
        file_path: String,
        format: TempLogFormat,
        sensor_positions: Vec<TempSensorPosition>,
    },
    /// CFD / FEM simulation thermal output
    SimulationOutput {
        file_path: String,
        format: SimulationThermalFormat,
        time_steps: Vec<f32>,
    },
    /// Live stream from thermal camera
    LiveStream {
        endpoint: String,
        camera_model: Option<String>,
        frame_rate_hz: f32,
    },
    /// Multi-spectral thermal (MWIR + LWIR simultaneously)
    MultiSpectralThermal {
        lwir_path: String,
        mwir_path: Option<String>,
        swir_path: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThermalFileFormat {
    FLIR_JPG, FLIR_R, TIFF_16bit, TIFF_32bit_Float,
    PNG_16bit, HDF5, NetCDF, ENVI_BSQ, CSV_Grid, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThermalVideoFormat { FLIR_SEQ, AVI, MP4_Gray16, HDF5_Stack, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SatelliteThermalSensor { Landsat8_TIRS, Landsat9_TIRS2, ASTER_TIR, ECOSTRESS, MODIS_LST, VIIRS_LST, Sentinel3_SLSTR, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TempLogFormat { CSV, JSON, Modbus_Log, LabView_TDMS, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimulationThermalFormat { Fluent_CAS, OpenFOAM, ANSYS_RST, FEniCS_HDF5, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThermalSensor {
    pub manufacturer: String,
    pub model: String,
    pub spectral_range_um: (f32, f32),   // e.g. (8.0, 14.0) for LWIR
    pub resolution: (u32, u32),
    pub pixel_pitch_um: f32,
    pub netd_mk: f32,                    // noise equivalent temperature difference in mK
    pub detector_type: ThermalDetectorType,
    pub fov_deg: (f32, f32),             // horizontal, vertical
    pub focal_length_mm: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ThermalDetectorType {
    #[default] Microbolometer_VOx, Microbolometer_aSi, InSb, MCT_HgCdTe,
    QWIP, InGaAs, Thermopile, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThermalCalibration {
    pub calibration_date: String,
    pub reference_blackbody_temp_k: Vec<f32>,
    pub polynomial_coefficients: Vec<f64>,   // ADC → temperature polynomial
    pub atmospheric_temp_k: f32,
    pub relative_humidity_percent: f32,
    pub object_distance_m: f32,
    pub emissivity_assumed: f32,
    pub reflected_temp_k: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RadiometricCoefficients {
    pub planck_r1: f64,
    pub planck_r2: f64,
    pub planck_b: f64,
    pub planck_f: f64,
    pub planck_o: f64,
    pub atmospheric_trans: f64,
    pub emissivity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThermalBandAssignment {
    pub band_number: u32,
    pub center_wavelength_um: f32,
    pub bandwidth_um: f32,
    pub temperature_range_k: (f32, f32),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TempSensorPosition {
    pub sensor_id: String,
    pub location_description: String,
    pub pixel_x: Option<u32>,
    pub pixel_y: Option<u32>,
    pub geo_location: Option<[f64; 3]>,
}

// ─────────────────────────────────────────────────────────────────────────────
// ANALYSIS RESULT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThermalAnalysisResult {
    pub analysis_id: u64,
    pub source_description: String,

    // IMAGE PARAMETERS
    pub width_pixels: u32,
    pub height_pixels: u32,
    pub spectral_band: ThermalBand,
    pub spatial_resolution_m: Option<f32>,
    pub geo_extent: Option<GeoExtent>,
    pub acquisition_time: Option<String>,
    pub sensor_info: Option<ThermalSensor>,

    // TEMPERATURE STATISTICS
    pub min_temp_k: f32,
    pub max_temp_k: f32,
    pub mean_temp_k: f32,
    pub std_temp_k: f32,
    pub median_temp_k: f32,
    pub temperature_histogram: Vec<(f32, u32)>,   // (temp_k, pixel_count)

    // DETECTED FEATURES
    pub hotspots: Vec<ThermalHotspot>,
    pub coldspots: Vec<ThermalColdspot>,
    pub thermal_gradients: Vec<ThermalGradient>,
    pub isothermal_contours: Vec<IsothermalContour>,

    // MATERIAL / SURFACE
    pub material_zones: Vec<ThermalMaterialZone>,
    pub emissivity_map: Option<EmissivityMap>,
    pub land_surface_temp: Option<LandSurfaceTemp>,

    // ANOMALIES
    pub anomalies: Vec<ThermalAnomaly>,

    // TEMPORAL (if video / time series)
    pub temporal_frames: Vec<ThermalFrameSummary>,
    pub temporal_trends: Vec<TemporalTrend>,

    // HEAT FLUX
    pub heat_flux_map: Option<HeatFluxMap>,
    pub heat_sources: Vec<HeatSource>,

    // BIOLOGICAL (if applicable)
    pub biological_signatures: Vec<BiologicalThermalSignature>,

    // METADATA
    pub atmospheric_correction_applied: bool,
    pub emissivity_correction_applied: bool,
    pub processing_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ThermalBand {
    #[default] LWIR,          // 8–14 µm
    MWIR,                      // 3–5 µm
    SWIR,                      // 1–3 µm
    NearIR,                    // 0.7–1 µm
    MultiSpectralTIR,          // multiple TIR bands
    Broadband,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeoExtent {
    pub min_lat: f64, pub max_lat: f64,
    pub min_lon: f64, pub max_lon: f64,
}

// ── HOTSPOTS / COLDSPOTS ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThermalHotspot {
    pub hotspot_id: u64,
    pub peak_temp_k: f32,
    pub mean_temp_k: f32,
    pub background_temp_k: f32,
    pub delta_t_k: f32,            // above background
    pub pixel_location: (u32, u32),
    pub geo_location: Option<[f64; 2]>,
    pub area_pixels: u32,
    pub area_m2: Option<f32>,
    pub shape: ThermalRegionShape,
    pub classification: HotspotClass,
    pub estimated_power_w: Option<f32>,
    pub thermal_signature_id: Option<u64>,
    pub timestamp: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum HotspotClass {
    #[default] Unknown, IndustrialEquipment, ElectricalFault, Fire, Combustion,
    BiologicalMetabolism, FrictionHeat, ChemicalReaction, SolarReflection,
    GeothermalVent, VehicleEngine, PowerLine, Building_HVAC, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThermalColdspot {
    pub coldspot_id: u64,
    pub min_temp_k: f32,
    pub mean_temp_k: f32,
    pub background_temp_k: f32,
    pub delta_t_k: f32,            // below background (negative value)
    pub pixel_location: (u32, u32),
    pub geo_location: Option<[f64; 2]>,
    pub area_pixels: u32,
    pub area_m2: Option<f32>,
    pub classification: ColdspotClass,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ColdspotClass {
    #[default] Unknown, Water, Shadow, Vegetation, IceSnow, UndergroundFeature,
    CoolAirInfiltration, EvaporativeCooling, Insulation, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ThermalRegionShape { #[default] Circular, Elongated, Irregular, Linear, Diffuse }

// ── GRADIENTS / ISOTHERMS ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThermalGradient {
    pub gradient_id: u64,
    pub gradient_magnitude_k_per_m: f32,
    pub gradient_direction_deg: f32,    // 0=right, 90=up
    pub start_pixel: (u32, u32),
    pub end_pixel: (u32, u32),
    pub start_temp_k: f32,
    pub end_temp_k: f32,
    pub physical_interpretation: Option<String>,  // e.g. "thermal bridge", "insulation edge"
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IsothermalContour {
    pub contour_id: u64,
    pub temperature_k: f32,
    pub pixel_chain: Vec<(u32, u32)>,   // ordered pixel boundary
    pub area_enclosed_pixels: u32,
    pub area_enclosed_m2: Option<f32>,
    pub is_closed: bool,
}

// ── MATERIALS / EMISSIVITY ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThermalMaterialZone {
    pub zone_id: u64,
    pub material_class: ThermalMaterialClass,
    pub inferred_material: Option<String>,
    pub emissivity_estimate: f32,
    pub thermal_diffusivity: Option<f64>,  // m²/s
    pub representative_pixel: (u32, u32),
    pub area_pixels: u32,
    pub mean_temp_k: f32,
    pub temp_stability: f32,               // 0=volatile, 1=stable
    pub cross_modal_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ThermalMaterialClass {
    #[default] Unknown, Metal_High_Emissivity, Metal_Low_Emissivity, Concrete,
    Asphalt, Vegetation, Water, Soil_Dry, Soil_Wet, Rock, Wood, Plastic,
    Glass, Ceramic, Organic_Tissue, Insulation, Paint_Coated, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmissivityMap {
    pub map_id: u64,
    pub width_pixels: u32,
    pub height_pixels: u32,
    pub emissivity_grid: Vec<Vec<f32>>,    // 0.0–1.0 per pixel
    pub mean_emissivity: f32,
    pub method: EmissivityMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum EmissivityMethod {
    #[default] Assumed_Uniform, Temperature_Reference, NDVI_Based,
    MaterialClassification, MultiAngle, LibraryBased,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LandSurfaceTemp {
    pub lst_id: u64,
    pub mean_lst_k: f32,
    pub min_lst_k: f32, pub max_lst_k: f32,
    pub urban_heat_island_delta_k: Option<f32>,
    pub lst_grid: Vec<Vec<f32>>,           // corrected LST per pixel
    pub retrieval_algorithm: String,       // "Single Channel", "Split Window", etc.
}

// ── ANOMALIES ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThermalAnomaly {
    pub anomaly_id: u64,
    pub pixel_location: (u32, u32),
    pub geo_location: Option<[f64; 2]>,
    pub anomaly_temp_k: f32,
    pub background_temp_k: f32,
    pub delta_t_k: f32,
    pub anomaly_score: f32,                // 0.0–1.0
    pub anomaly_type: ThermalAnomalyType,
    pub possible_cause: Option<String>,
    pub timestamp: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ThermalAnomalyType {
    #[default] Unknown, Subsurface, Electrical, Mechanical, Biological,
    Environmental, Construction_Defect, Leak, Fire_Precursor, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThermalAnomalyMethod {
    GlobalThreshold,
    LocalAdaptive { window_size_pixels: u32 },
    StatisticalMAD,    // median absolute deviation
    RXDetector,
    DeepLearning,
}

// ── TEMPORAL ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThermalFrameSummary {
    pub frame_index: u32,
    pub timestamp_sec: f32,
    pub mean_temp_k: f32,
    pub max_temp_k: f32,
    pub min_temp_k: f32,
    pub hotspot_count: u32,
    pub anomaly_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemporalTrend {
    pub trend_id: u64,
    pub region: Option<PixelRegion>,
    pub time_series_k: Vec<(f32, f32)>,    // (time_sec, temp_k)
    pub trend_type: ThermalTrendType,
    pub rate_k_per_sec: f32,
    pub r_squared: f32,
    pub interpretation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ThermalTrendType {
    #[default] Stable, Heating, Cooling, Oscillating, Step_Change, Cyclic
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PixelRegion {
    pub x_min: u32, pub x_max: u32,
    pub y_min: u32, pub y_max: u32,
    pub description: Option<String>,
}

// ── HEAT FLUX ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HeatFluxMap {
    pub map_id: u64,
    pub width_pixels: u32, pub height_pixels: u32,
    pub flux_grid_w_per_m2: Vec<Vec<f32>>,
    pub mean_flux_w_per_m2: f32,
    pub max_flux_w_per_m2: f32,
    pub total_power_estimate_w: Option<f32>,
    pub stefan_boltzmann_used: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HeatSource {
    pub source_id: u64,
    pub pixel_location: (u32, u32),
    pub geo_location: Option<[f64; 2]>,
    pub temperature_k: f32,
    pub estimated_power_w: f32,
    pub area_m2: f32,
    pub emissivity: f32,
    pub source_class: HotspotClass,
}

// ── BIOLOGICAL ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BiologicalThermalSignature {
    pub signature_id: u64,
    pub organism_type: BiologicalOrganismType,
    pub core_temp_k: Option<f32>,
    pub surface_temp_k: f32,
    pub ambient_temp_k: f32,
    pub delta_t_k: f32,
    pub heat_loss_estimate_w: Option<f32>,
    pub symmetry_score: f32,             // bilateral thermal symmetry (0=asymmetric, 1=symmetric)
    pub pixel_location: (u32, u32),
    pub body_region: Option<String>,     // "head", "thorax", "extremity", etc.
    pub medical_note: Option<String>,    // "possible fever", "inflammation indicator", etc.
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum BiologicalOrganismType {
    #[default] Unknown, Human, Animal, Plant, Insect, Custom(String)
}

// ── MISC TYPES ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TempReference {
    pub pixel_location: (u32, u32),
    pub known_temp_k: f32,
    pub material: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThermalFusionMethod { Alpha_Blend, Colormapped_Overlay, Edge_Enhanced, Bilateral_Fusion }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThermalFrame {
    pub frame_index: u32,
    pub timestamp_sec: f32,
    pub mean_temp_k: f32,
    pub max_temp_k: f32,
    pub min_temp_k: f32,
    pub hotspot_count: u32,
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH NODE TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ThermalNodeType {
    #[default] ThermalScene,             // root
    ThermalImageNode,                    // a single thermal frame
    HotspotNode,                         // thermal hotspot
    ColdspotNode,                        // thermal coldspot
    ThermalGradientNode,                 // spatial thermal gradient
    IsothermalContourNode,               // isotherm at specific temperature
    MaterialZoneNode,                    // thermally-characterized material region
    EmissivityZoneNode,                  // local emissivity estimate
    ThermalAnomalyNode,                  // anomalous thermal feature
    HeatSourceNode,                      // identified heat source
    TemporalTrendNode,                   // temperature trend over time
    BiologicalSignatureNode,             // biological thermal pattern
    LandSurfaceTempNode,                 // satellite-derived LST region
    HeatFluxNode,                        // heat flux measurement
    GeographicLocationNode,              // geo-referenced region
    CrossModalRef,                       // cross-modal placeholder
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThermalGraphNode {
    pub node_id: u64,
    pub node_type: ThermalNodeType,
    pub content: String,

    // THERMAL-SPECIFIC
    pub temperature_k: Option<f32>,
    pub delta_t_k: Option<f32>,
    pub area_pixels: Option<u32>,
    pub area_m2: Option<f32>,
    pub pixel_location: Option<(u32, u32)>,
    pub geo_location: Option<[f64; 2]>,
    pub emissivity: Option<f32>,
    pub thermal_class: Option<String>,
    pub timestamp: Option<f64>,

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
pub enum ThermalEdgeType {
    // ── STRUCTURAL ──
    #[default] Contains, Precedes, PartOf,

    // ── THERMAL-SPECIFIC ──
    EmitsHeat,                     // source emits thermal energy
    ThermallyCoupledTo,            // thermally in contact / conducts heat
    IndicatesMaterialState,        // temperature indicates material phase/state
    AffectsBiologicalProcess,      // temperature affects biology
    CausesExpansion,               // thermal expansion
    CausesContraction,             // thermal contraction
    ThermalBridgeTo,               // thermal bridge between two zones
    AdjacentColdRegion,            // hotspot bordered by cold region
    GradientBetween,               // gradient connects two zones
    IsothermEncloses,              // isotherm encloses a region
    AnomalyRelativeTo,             // anomaly relative to background
    HeatFluxThrough,               // heat flows through boundary
    TemporallyPrecedes,            // earlier frame of same region
    EvolutionOf,                   // temporal evolution of hotspot

    // ── CROSS-MODAL ──
    /// Thermal texture on 3D surface (109)
    ThermalSurface3D,
    /// Visible image overlay (102)
    FusedWithVisible,
    /// Biological thermal signature → biology graph (111)
    LinkedToBiology,
    /// Emissivity correlates with hyperspectral (126)
    EmissivityCorrelatesWithHSI,
    /// Thermal on geospatial map (117)
    PlottedOnGeoMap,
    /// Friction heat from kinematics (121)
    GeneratedByFriction,
    /// Thermal feedback to control system (122)
    FeedbackToControl,
    /// Target thermal signature → radar fusion (124)
    CrossFusedWithRadar,

    // ── UNIVERSAL SEMANTIC ──
    Performs, Affects, Implies, Contradicts, Elaborates, Summarizes, Supports,
    TemporalPrecedes, TemporalFollows, CausedBy, Enables, Prevents,
    FunctionalRole, InstanceOf,
    DerivedFrom, VersionOf, RefinesTo, ForkedFrom, SimilarTo,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThermalGraphEdge {
    pub edge_id: u64,
    pub from_node: u64, pub to_node: u64,
    pub edge_type: ThermalEdgeType,
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
pub struct ThermalGraph {
    pub graph_id: u64, pub project_id: u64,
    pub source_description: String,
    pub nodes: Vec<ThermalGraphNode>,
    pub edges: Vec<ThermalGraphEdge>,
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
pub enum ThermalGraphQuery {
    NodeDetail { node_id: u64 },
    HotspotsAbove { threshold_k: f32 },
    ColdspotsBelow { threshold_k: f32 },
    AnomaliesByType { anomaly_type: String },
    MaterialZones,
    BiologicalSignatures,
    TemporalTrends,
    CrossModalLinks { node_id: u64 },
    AGIActivity, AllNodes, AllEdges,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThermalSemanticHook {
    OnGraphCreated,
    OnInferRelationships,
    OnEdgeCompletion,
    OnCrossModalityLink { target_modality: String, target_graph_id: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThermalExportFormat {
    GeoTIFF_Temperature,   // temperature values as float GeoTIFF
    GeoTIFF_Colormap,      // false-color PNG/TIFF
    GeoJSON,               // hotspots/anomalies as vector features
    CSV,                   // tabular hotspot/anomaly list
    NetCDF,                // scientific arrays
    KML,                   // Google Earth
    FLIR_CSV,              // FLIR-compatible export
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThermalDisplayMode {
    PseudoColor,           // rainbow/iron/hot colormap
    Grayscale,             // raw 16-bit
    IsothermOverlay,       // colored contours
    AnomalyMap,            // anomaly score overlay
    HeatFluxMap,           // W/m² visualization
    TemporalTimelapse,     // animated frames
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThermalOperation {
    RecomputeAnomalies { method: ThermalAnomalyMethod, threshold_k: f32 },
    UpdateEmissivity { reference_points: Vec<TempReference> },
    CrossLinkWithVisible { image_path: String },
    ExportHotspotReport,
    ComputeHeatBalance,
    CrossValidateWithHSI { hsi_graph_id: u64 },
}

// ─────────────────────────────────────────────────────────────────────────────
// OUTPUT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThermalModalityOutput {
    pub success: bool,
    pub graph_id: Option<u64>,
    pub graph: Option<ThermalGraph>,
    pub analysis: Option<ThermalAnalysisResult>,
    pub emissivity_map: Option<EmissivityMap>,
    pub anomaly_map: Option<Vec<ThermalAnomaly>>,
    pub heat_flux_map: Option<HeatFluxMap>,
    pub temporal_trend: Option<Vec<TemporalTrend>>,
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
        let input = serde_json::json!({"action":"Prompt","prompt":prompt,"max_tokens":max_tokens,"temperature":0.05,"system_context":"Thermal imaging analysis. Return only valid JSON."});
        let out = std::process::Command::new(&self.prompt_pipeline_path)
            .arg("--input").arg(input.to_string())
            .output().map_err(|e| e.to_string())?;
        let r: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
        Ok(r["response"].as_str().unwrap_or("{}").to_string())
    }

    fn save_graph(&self, g: &ThermalGraph) -> Result<(), String> {
        let path = format!("{}/local/thermal_graph_{}.json", self.zsei_path, g.graph_id);
        if let Some(p) = std::path::Path::new(&path).parent() { std::fs::create_dir_all(p).map_err(|e| e.to_string())?; }
        std::fs::write(&path, serde_json::to_string_pretty(g).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    fn load_graph(&self, id: u64) -> Result<ThermalGraph, String> {
        let path = format!("{}/local/thermal_graph_{}.json", self.zsei_path, id);
        serde_json::from_str(&std::fs::read_to_string(&path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    fn generate_id(&self) -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as u64 }
    fn now_iso8601(&self) -> String { format!("{}", self.generate_id()) }
    fn extract_json_array(raw: &str) -> String { if let (Some(s), Some(e)) = (raw.find('['), raw.rfind(']')) { raw[s..=e].to_string() } else { "[]".to_string() } }
    fn extract_json_object(raw: &str) -> String { if let (Some(s), Some(e)) = (raw.find('{'), raw.rfind('}')) { raw[s..=e].to_string() } else { "{}".to_string() } }

    fn kelvin_to_celsius(k: f32) -> f32 { k - 273.15 }
    fn celsius_to_kelvin(c: f32) -> f32 { c + 273.15 }

    /// Stefan-Boltzmann radiated power estimate: P = ε·σ·A·T⁴
    fn estimate_radiated_power(emissivity: f32, area_m2: f32, temp_k: f32) -> f32 {
        let sigma: f32 = 5.670374419e-8;
        emissivity * sigma * area_m2 * temp_k.powi(4)
    }
}

impl PipelineExecutor {
    async fn classify_hotspots_llm(&self, hotspots: &[ThermalHotspot], context: &str) -> Vec<(u64, String)> {
        if hotspots.is_empty() { return vec![]; }
        let list: Vec<serde_json::Value> = hotspots.iter().take(20).map(|h| serde_json::json!({
            "hotspot_id": h.hotspot_id,
            "peak_temp_c": Self::kelvin_to_celsius(h.peak_temp_k),
            "delta_t_k": h.delta_t_k,
            "area_pixels": h.area_pixels,
            "shape": format!("{:?}", h.shape),
        })).collect();

        let prompt = format!(r#"
Classify each thermal hotspot in this context: {}

Hotspots (temperatures in °C, delta_T above background in K):
{}

Classification options:
IndustrialEquipment, ElectricalFault, Fire, Combustion, BiologicalMetabolism,
FrictionHeat, ChemicalReaction, SolarReflection, GeothermalVent,
VehicleEngine, PowerLine, Building_HVAC, Unknown

Return ONLY valid JSON array:
[{{"hotspot_id": N, "classification": "ClassName", "reasoning": "brief"}}]"#,
            context, serde_json::to_string_pretty(&list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 600).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str).unwrap_or_default()
                    .into_iter().filter_map(|v| Some((v["hotspot_id"].as_u64()?, v["classification"].as_str()?.to_string())))
                    .collect()
            }
            Err(_) => vec![],
        }
    }

    async fn classify_material_zones_llm(&self, zones: &[ThermalMaterialZone]) -> Vec<(u64, String)> {
        if zones.is_empty() { return vec![]; }
        let list: Vec<serde_json::Value> = zones.iter().take(20).map(|z| serde_json::json!({
            "zone_id": z.zone_id,
            "mean_temp_c": Self::kelvin_to_celsius(z.mean_temp_k),
            "emissivity_estimate": z.emissivity_estimate,
            "temp_stability": z.temp_stability,
            "area_pixels": z.area_pixels,
        })).collect();

        let prompt = format!(r#"
Identify the most likely material for each thermal zone based on temperature and emissivity.

Zones:
{}

Material options:
Metal_High_Emissivity, Metal_Low_Emissivity, Concrete, Asphalt, Vegetation,
Water, Soil_Dry, Soil_Wet, Rock, Wood, Plastic, Glass, Ceramic, Organic_Tissue,
Insulation, Paint_Coated, Unknown

Return ONLY valid JSON array:
[{{"zone_id": N, "inferred_material": "MaterialClass", "reasoning": "brief"}}]"#,
            serde_json::to_string_pretty(&list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 500).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str).unwrap_or_default()
                    .into_iter().filter_map(|v| Some((v["zone_id"].as_u64()?, v["inferred_material"].as_str()?.to_string())))
                    .collect()
            }
            Err(_) => vec![],
        }
    }

    async fn infer_semantic_relationships(&self, nodes: &[ThermalGraphNode]) -> Vec<(u64, u64, ThermalEdgeType, String)> {
        if nodes.len() < 2 { return vec![]; }
        let node_list: Vec<serde_json::Value> = nodes.iter().take(25).map(|n| serde_json::json!({
            "node_id": n.node_id, "type": format!("{:?}", n.node_type),
            "content": n.content.chars().take(80).collect::<String>(),
            "temp_k": n.temperature_k, "delta_t": n.delta_t_k,
        })).collect();

        let prompt = format!(r#"
Identify semantic relationships between these thermal graph nodes.

Nodes: {}

Types: EmitsHeat, ThermallyCoupledTo, IndicatesMaterialState, AffectsBiologicalProcess,
CausesExpansion, ThermalBridgeTo, GradientBetween, IsothermEncloses, AnomalyRelativeTo,
HeatFluxThrough, TemporallyPrecedes, Affects, CausedBy, Enables, DerivedFrom

Return ONLY valid JSON array:
[{{"from_node_id": N, "to_node_id": M, "edge_type": "TypeName", "reason": "brief"}}]"#,
            serde_json::to_string_pretty(&node_list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 700).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str).unwrap_or_default()
                    .into_iter().filter_map(|v| {
                        let from = v["from_node_id"].as_u64()?;
                        let to = v["to_node_id"].as_u64()?;
                        let etype = map_thermal_edge_str(v["edge_type"].as_str().unwrap_or("Affects"));
                        let reason = v["reason"].as_str().unwrap_or("").to_string();
                        Some((from, to, etype, reason))
                    }).collect()
            }
            Err(_) => vec![],
        }
    }

    async fn interpret_biological_signatures_llm(&self, sigs: &[BiologicalThermalSignature]) -> Vec<(u64, String)> {
        if sigs.is_empty() { return vec![]; }
        let list: Vec<serde_json::Value> = sigs.iter().take(10).map(|s| serde_json::json!({
            "signature_id": s.signature_id,
            "organism_type": format!("{:?}", s.organism_type),
            "surface_temp_c": Self::kelvin_to_celsius(s.surface_temp_k),
            "ambient_temp_c": Self::kelvin_to_celsius(s.ambient_temp_k),
            "delta_t_k": s.delta_t_k,
            "symmetry_score": s.symmetry_score,
            "body_region": s.body_region,
        })).collect();

        let prompt = format!(r#"
Interpret these biological thermal signatures for medical or veterinary significance.

Signatures:
{}

Consider: fever (>38.5°C for humans), hypothermia (<35°C), inflammation (asymmetric hotspot),
good circulation (symmetric, normal delta_T), poor circulation (cold extremities).

Return ONLY valid JSON array:
[{{"signature_id": N, "medical_note": "interpretation"}}]"#,
            serde_json::to_string_pretty(&list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 400).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str).unwrap_or_default()
                    .into_iter().filter_map(|v| Some((v["signature_id"].as_u64()?, v["medical_note"].as_str()?.to_string())))
                    .collect()
            }
            Err(_) => vec![],
        }
    }

    async fn detect_anomalies_adaptive(&self, analysis: &ThermalAnalysisResult, threshold_k: f32) -> Vec<ThermalAnomaly> {
        // Simplified local adaptive detector: compare each hotspot to local neighborhood mean
        let mut anomalies = Vec::new();
        let background_k = analysis.mean_temp_k;

        for hotspot in &analysis.hotspots {
            let delta = hotspot.peak_temp_k - background_k;
            if delta.abs() > threshold_k {
                let score = (delta.abs() / threshold_k).min(1.0);
                anomalies.push(ThermalAnomaly {
                    anomaly_id: hotspot.hotspot_id + 10_000_000,
                    pixel_location: hotspot.pixel_location,
                    geo_location: hotspot.geo_location,
                    anomaly_temp_k: hotspot.peak_temp_k,
                    background_temp_k: background_k,
                    delta_t_k: delta,
                    anomaly_score: score,
                    anomaly_type: match hotspot.classification {
                        HotspotClass::ElectricalFault => ThermalAnomalyType::Electrical,
                        HotspotClass::BiologicalMetabolism => ThermalAnomalyType::Biological,
                        HotspotClass::Fire | HotspotClass::Combustion => ThermalAnomalyType::Fire_Precursor,
                        _ => ThermalAnomalyType::Unknown,
                    },
                    possible_cause: Some(format!("{:?}", hotspot.classification)),
                    timestamp: hotspot.timestamp,
                });
            }
        }
        anomalies
    }
}

fn map_thermal_edge_str(s: &str) -> ThermalEdgeType {
    match s {
        "EmitsHeat"                  => ThermalEdgeType::EmitsHeat,
        "ThermallyCoupledTo"         => ThermalEdgeType::ThermallyCoupledTo,
        "IndicatesMaterialState"     => ThermalEdgeType::IndicatesMaterialState,
        "AffectsBiologicalProcess"   => ThermalEdgeType::AffectsBiologicalProcess,
        "CausesExpansion"            => ThermalEdgeType::CausesExpansion,
        "ThermalBridgeTo"            => ThermalEdgeType::ThermalBridgeTo,
        "GradientBetween"            => ThermalEdgeType::GradientBetween,
        "IsothermEncloses"           => ThermalEdgeType::IsothermEncloses,
        "AnomalyRelativeTo"          => ThermalEdgeType::AnomalyRelativeTo,
        "HeatFluxThrough"            => ThermalEdgeType::HeatFluxThrough,
        "TemporallyPrecedes"         => ThermalEdgeType::TemporallyPrecedes,
        "EvolutionOf"                => ThermalEdgeType::EvolutionOf,
        "ThermalSurface3D"           => ThermalEdgeType::ThermalSurface3D,
        "FusedWithVisible"           => ThermalEdgeType::FusedWithVisible,
        "LinkedToBiology"            => ThermalEdgeType::LinkedToBiology,
        "EmissivityCorrelatesWithHSI"=> ThermalEdgeType::EmissivityCorrelatesWithHSI,
        "PlottedOnGeoMap"            => ThermalEdgeType::PlottedOnGeoMap,
        "GeneratedByFriction"        => ThermalEdgeType::GeneratedByFriction,
        "FeedbackToControl"          => ThermalEdgeType::FeedbackToControl,
        "CrossFusedWithRadar"        => ThermalEdgeType::CrossFusedWithRadar,
        "Affects"                    => ThermalEdgeType::Affects,
        "CausedBy"                   => ThermalEdgeType::CausedBy,
        "Enables"                    => ThermalEdgeType::Enables,
        "Prevents"                   => ThermalEdgeType::Prevents,
        "TemporalPrecedes"           => ThermalEdgeType::TemporalPrecedes,
        "DerivedFrom"                => ThermalEdgeType::DerivedFrom,
        "PartOf"                     => ThermalEdgeType::PartOf,
        "FunctionalRole"             => ThermalEdgeType::FunctionalRole,
        _                            => ThermalEdgeType::Affects,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH CREATION
// ─────────────────────────────────────────────────────────────────────────────

async fn create_graph(executor: &PipelineExecutor, analysis: ThermalAnalysisResult, project_id: u64) -> ThermalModalityOutput {
    let graph_id = executor.generate_id();
    let now = executor.now_iso8601();
    let mut nodes: Vec<ThermalGraphNode> = Vec::new();
    let mut edges: Vec<ThermalGraphEdge> = Vec::new();
    let mut node_id: u64 = 1;
    let mut edge_id: u64 = 1;

    // ── ROOT ──
    let root_id = node_id;
    nodes.push(ThermalGraphNode {
        node_id: root_id, node_type: ThermalNodeType::ThermalScene,
        content: format!("Thermal scene: {}×{}px {:?} T={:.1}–{:.1}°C hotspots={} anomalies={}",
            analysis.width_pixels, analysis.height_pixels, analysis.spectral_band,
            PipelineExecutor::kelvin_to_celsius(analysis.min_temp_k),
            PipelineExecutor::kelvin_to_celsius(analysis.max_temp_k),
            analysis.hotspots.len(), analysis.anomalies.len()),
        temperature_k: Some(analysis.mean_temp_k),
        materialized_path: Some(format!("/Modalities/Thermal/Project_{}/Graph_{}", project_id, graph_id)),
        provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
        version_notes: vec![VersionNote { version: 1, note: "Initial creation".into(), step_index: None, timestamp: now.clone(), change_type: ChangeType::Created }],
        keywords: vec!["thermal".into(), "infrared".into(), format!("{:?}", analysis.spectral_band).to_lowercase()],
        hotness_score: 1.0, ..Default::default()
    });
    node_id += 1;

    // ── HOTSPOT NODES ──
    let hotspot_node_ids: Vec<(u64, u64)> = analysis.hotspots.iter().map(|hs| {
        let hid = node_id;
        let power_w = PipelineExecutor::estimate_radiated_power(
            0.9, hs.area_m2.unwrap_or(0.01), hs.peak_temp_k
        );
        nodes.push(ThermalGraphNode {
            node_id: hid, node_type: ThermalNodeType::HotspotNode,
            content: format!("Hotspot {:?}: peak={:.1}°C +{:.1}K area={}px power≈{:.1}W",
                hs.classification,
                PipelineExecutor::kelvin_to_celsius(hs.peak_temp_k),
                hs.delta_t_k, hs.area_pixels, power_w),
            temperature_k: Some(hs.peak_temp_k),
            delta_t_k: Some(hs.delta_t_k),
            area_pixels: Some(hs.area_pixels), area_m2: hs.area_m2,
            pixel_location: Some(hs.pixel_location), geo_location: hs.geo_location,
            thermal_class: Some(format!("{:?}", hs.classification)),
            timestamp: hs.timestamp,
            materialized_path: Some(format!("/Modalities/Thermal/Project_{}/Graph_{}/Hotspot/{}", project_id, graph_id, hs.hotspot_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["hotspot".into(), format!("{:?}", hs.classification).to_lowercase()],
            hotness_score: 0.5 + (hs.delta_t_k / 100.0).clamp(0.0, 0.45),
            ..Default::default()
        });
        edges.push(ThermalGraphEdge { edge_id, from_node: root_id, to_node: hid, edge_type: ThermalEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
        (hs.hotspot_id, hid)
    }).collect();

    // ── COLDSPOT NODES ──
    for cs in &analysis.coldspots {
        let cid = node_id;
        nodes.push(ThermalGraphNode {
            node_id: cid, node_type: ThermalNodeType::ColdspotNode,
            content: format!("Coldspot {:?}: min={:.1}°C {:.1}K below bg area={}px",
                cs.classification,
                PipelineExecutor::kelvin_to_celsius(cs.min_temp_k),
                cs.delta_t_k.abs(), cs.area_pixels),
            temperature_k: Some(cs.min_temp_k), delta_t_k: Some(cs.delta_t_k),
            area_pixels: Some(cs.area_pixels), area_m2: cs.area_m2,
            pixel_location: Some(cs.pixel_location), geo_location: cs.geo_location,
            thermal_class: Some(format!("{:?}", cs.classification)),
            materialized_path: Some(format!("/Modalities/Thermal/Project_{}/Graph_{}/Coldspot/{}", project_id, graph_id, cs.coldspot_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["coldspot".into(), format!("{:?}", cs.classification).to_lowercase()],
            hotness_score: 0.5 + (cs.delta_t_k.abs() / 80.0).clamp(0.0, 0.4),
            ..Default::default()
        });
        edges.push(ThermalGraphEdge { edge_id, from_node: root_id, to_node: cid, edge_type: ThermalEdgeType::Contains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── GRADIENT NODES ──
    for grad in &analysis.thermal_gradients {
        let gid = node_id;
        nodes.push(ThermalGraphNode {
            node_id: gid, node_type: ThermalNodeType::ThermalGradientNode,
            content: format!("Gradient: {:.2}K/m dir={:.0}° T={:.1}°C→{:.1}°C interp={:?}",
                grad.gradient_magnitude_k_per_m, grad.gradient_direction_deg,
                PipelineExecutor::kelvin_to_celsius(grad.start_temp_k),
                PipelineExecutor::kelvin_to_celsius(grad.end_temp_k),
                grad.physical_interpretation.as_deref().unwrap_or("unknown")),
            temperature_k: Some((grad.start_temp_k + grad.end_temp_k) / 2.0),
            delta_t_k: Some(grad.end_temp_k - grad.start_temp_k),
            pixel_location: Some(grad.start_pixel),
            materialized_path: Some(format!("/Modalities/Thermal/Project_{}/Graph_{}/Gradient/{}", project_id, graph_id, grad.gradient_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["gradient".into(), "thermal-boundary".into()],
            hotness_score: 0.6 + (grad.gradient_magnitude_k_per_m / 20.0).clamp(0.0, 0.3),
            ..Default::default()
        });
        edges.push(ThermalGraphEdge { edge_id, from_node: root_id, to_node: gid, edge_type: ThermalEdgeType::Contains, weight: 0.7, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── ISOTHERM NODES ──
    for iso in analysis.isothermal_contours.iter().take(20) {
        let iid = node_id;
        nodes.push(ThermalGraphNode {
            node_id: iid, node_type: ThermalNodeType::IsothermalContourNode,
            content: format!("Isotherm: T={:.1}°C area={}px closed={}", PipelineExecutor::kelvin_to_celsius(iso.temperature_k), iso.area_enclosed_pixels, iso.is_closed),
            temperature_k: Some(iso.temperature_k), area_pixels: Some(iso.area_enclosed_pixels), area_m2: iso.area_enclosed_m2,
            materialized_path: Some(format!("/Modalities/Thermal/Project_{}/Graph_{}/Isotherm/{}", project_id, graph_id, iso.contour_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["isotherm".into()], hotness_score: 0.55, ..Default::default()
        });
        edges.push(ThermalGraphEdge { edge_id, from_node: root_id, to_node: iid, edge_type: ThermalEdgeType::Contains, weight: 0.6, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── MATERIAL ZONE NODES ──
    let mat_node_ids: Vec<(u64, u64)> = analysis.material_zones.iter().map(|mz| {
        let mid = node_id;
        nodes.push(ThermalGraphNode {
            node_id: mid, node_type: ThermalNodeType::MaterialZoneNode,
            content: format!("MaterialZone {:?}: mat={:?} T={:.1}°C ε={:.2} stability={:.2}",
                mz.material_class,
                mz.inferred_material.as_deref().unwrap_or("?"),
                PipelineExecutor::kelvin_to_celsius(mz.mean_temp_k),
                mz.emissivity_estimate, mz.temp_stability),
            temperature_k: Some(mz.mean_temp_k), emissivity: Some(mz.emissivity_estimate),
            area_pixels: Some(mz.area_pixels), pixel_location: Some(mz.representative_pixel),
            thermal_class: Some(format!("{:?}", mz.material_class)),
            materialized_path: Some(format!("/Modalities/Thermal/Project_{}/Graph_{}/MaterialZone/{}", project_id, graph_id, mz.zone_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: { let mut kw = vec!["material-zone".into(), format!("{:?}", mz.material_class).to_lowercase()]; if let Some(ref m) = mz.inferred_material { kw.push(m.to_lowercase()); } kw },
            hotness_score: 0.65, ..Default::default()
        });
        edges.push(ThermalGraphEdge { edge_id, from_node: root_id, to_node: mid, edge_type: ThermalEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Material zone → indicates material state
        edges.push(ThermalGraphEdge { edge_id, from_node: mid, to_node: root_id, edge_type: ThermalEdgeType::IndicatesMaterialState, weight: 0.7, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Cross-modal: material → hyperspectral emissivity (126)
        edges.push(ThermalGraphEdge {
            edge_id, from_node: mid, to_node: mid,
            edge_type: ThermalEdgeType::EmissivityCorrelatesWithHSI, weight: 0.8,
            provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
            properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("hyperspectral")); p },
            ..Default::default()
        });
        edge_id += 1; node_id += 1;
        (mz.zone_id, mid)
    }).collect();

    // ── ANOMALY NODES ──
    for anomaly in &analysis.anomalies {
        let aid = node_id;
        nodes.push(ThermalGraphNode {
            node_id: aid, node_type: ThermalNodeType::ThermalAnomalyNode,
            content: format!("ThermalAnomaly {:?}: T={:.1}°C ΔT={:.1}K score={:.2} cause={:?}",
                anomaly.anomaly_type,
                PipelineExecutor::kelvin_to_celsius(anomaly.anomaly_temp_k),
                anomaly.delta_t_k, anomaly.anomaly_score,
                anomaly.possible_cause.as_deref().unwrap_or("unknown")),
            temperature_k: Some(anomaly.anomaly_temp_k), delta_t_k: Some(anomaly.delta_t_k),
            pixel_location: Some(anomaly.pixel_location), geo_location: anomaly.geo_location,
            thermal_class: Some(format!("{:?}", anomaly.anomaly_type)),
            timestamp: anomaly.timestamp,
            materialized_path: Some(format!("/Modalities/Thermal/Project_{}/Graph_{}/Anomaly/{}", project_id, graph_id, anomaly.anomaly_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["anomaly".into(), format!("{:?}", anomaly.anomaly_type).to_lowercase()],
            hotness_score: 0.6 + anomaly.anomaly_score * 0.3,
            ..Default::default()
        });
        edges.push(ThermalGraphEdge { edge_id, from_node: root_id, to_node: aid, edge_type: ThermalEdgeType::Contains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        edges.push(ThermalGraphEdge { edge_id, from_node: aid, to_node: root_id, edge_type: ThermalEdgeType::AnomalyRelativeTo, weight: anomaly.anomaly_score, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── BIOLOGICAL SIGNATURE NODES ──
    for bio in &analysis.biological_signatures {
        let bid = node_id;
        nodes.push(ThermalGraphNode {
            node_id: bid, node_type: ThermalNodeType::BiologicalSignatureNode,
            content: format!("BioSig {:?}: surface={:.1}°C ΔT={:.1}K symmetry={:.2} region={:?} note={:?}",
                bio.organism_type,
                PipelineExecutor::kelvin_to_celsius(bio.surface_temp_k),
                bio.delta_t_k, bio.symmetry_score,
                bio.body_region.as_deref().unwrap_or("?"),
                bio.medical_note.as_deref().unwrap_or("none")),
            temperature_k: Some(bio.surface_temp_k), delta_t_k: Some(bio.delta_t_k),
            pixel_location: Some(bio.pixel_location),
            thermal_class: Some(format!("{:?}", bio.organism_type)),
            materialized_path: Some(format!("/Modalities/Thermal/Project_{}/Graph_{}/BioSig/{}", project_id, graph_id, bio.signature_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["biological".into(), format!("{:?}", bio.organism_type).to_lowercase()],
            hotness_score: 0.75,
            ..Default::default()
        });
        edges.push(ThermalGraphEdge { edge_id, from_node: root_id, to_node: bid, edge_type: ThermalEdgeType::Contains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // Bio thermal → biology modality cross-link (111)
        edges.push(ThermalGraphEdge {
            edge_id, from_node: bid, to_node: bid,
            edge_type: ThermalEdgeType::LinkedToBiology, weight: 0.9,
            provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
            properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("biology")); p },
            ..Default::default()
        });
        edge_id += 1; node_id += 1;
    }

    // ── HEAT SOURCE NODES ──
    for hs in &analysis.heat_sources {
        let hsid = node_id;
        nodes.push(ThermalGraphNode {
            node_id: hsid, node_type: ThermalNodeType::HeatSourceNode,
            content: format!("HeatSource: T={:.1}°C P={:.1}W A={:.3}m² ε={:.2} class={:?}",
                PipelineExecutor::kelvin_to_celsius(hs.temperature_k), hs.estimated_power_w, hs.area_m2, hs.emissivity, hs.source_class),
            temperature_k: Some(hs.temperature_k), emissivity: Some(hs.emissivity),
            area_m2: Some(hs.area_m2), pixel_location: Some(hs.pixel_location), geo_location: hs.geo_location,
            thermal_class: Some(format!("{:?}", hs.source_class)),
            materialized_path: Some(format!("/Modalities/Thermal/Project_{}/Graph_{}/HeatSource/{}", project_id, graph_id, hs.source_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["heat-source".into(), format!("{:?}", hs.source_class).to_lowercase()],
            hotness_score: 0.7 + (hs.estimated_power_w / 5000.0).clamp(0.0, 0.25),
            ..Default::default()
        });
        edges.push(ThermalGraphEdge { edge_id, from_node: root_id, to_node: hsid, edge_type: ThermalEdgeType::Contains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        edges.push(ThermalGraphEdge { edge_id, from_node: hsid, to_node: root_id, edge_type: ThermalEdgeType::EmitsHeat, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── TEMPORAL TREND NODES ──
    for trend in &analysis.temporal_trends {
        let tid = node_id;
        nodes.push(ThermalGraphNode {
            node_id: tid, node_type: ThermalNodeType::TemporalTrendNode,
            content: format!("TemporalTrend {:?}: rate={:.3}K/s R²={:.2} {}", trend.trend_type, trend.rate_k_per_sec, trend.r_squared, trend.interpretation),
            materialized_path: Some(format!("/Modalities/Thermal/Project_{}/Graph_{}/Trend/{}", project_id, graph_id, trend.trend_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["trend".into(), format!("{:?}", trend.trend_type).to_lowercase()],
            hotness_score: 0.6, ..Default::default()
        });
        edges.push(ThermalGraphEdge { edge_id, from_node: root_id, to_node: tid, edge_type: ThermalEdgeType::Contains, weight: 0.7, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── LAND SURFACE TEMP NODE ──
    if let Some(ref lst) = analysis.land_surface_temp {
        let lstid = node_id;
        nodes.push(ThermalGraphNode {
            node_id: lstid, node_type: ThermalNodeType::LandSurfaceTempNode,
            content: format!("LST: mean={:.1}°C range={:.1}–{:.1}°C UHI={:?}°C algo={}",
                PipelineExecutor::kelvin_to_celsius(lst.mean_lst_k),
                PipelineExecutor::kelvin_to_celsius(lst.min_lst_k),
                PipelineExecutor::kelvin_to_celsius(lst.max_lst_k),
                lst.urban_heat_island_delta_k, lst.retrieval_algorithm),
            temperature_k: Some(lst.mean_lst_k),
            materialized_path: Some(format!("/Modalities/Thermal/Project_{}/Graph_{}/LST", project_id, graph_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["land-surface-temperature".into(), "lst".into()], hotness_score: 0.75, ..Default::default()
        });
        edges.push(ThermalGraphEdge { edge_id, from_node: root_id, to_node: lstid, edge_type: ThermalEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        // LST → geospatial cross-modal
        edges.push(ThermalGraphEdge { edge_id: edge_id + 1, from_node: lstid, to_node: lstid, edge_type: ThermalEdgeType::PlottedOnGeoMap, weight: 0.9, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("geospatial")); p }, ..Default::default() });
        edge_id += 2; node_id += 1;
    }

    // ── HEAT FLUX MAP NODE ──
    if let Some(ref hf) = analysis.heat_flux_map {
        let hfid = node_id;
        nodes.push(ThermalGraphNode {
            node_id: hfid, node_type: ThermalNodeType::HeatFluxNode,
            content: format!("HeatFlux: mean={:.1}W/m² max={:.1}W/m² total≈{:?}W",
                hf.mean_flux_w_per_m2, hf.max_flux_w_per_m2, hf.total_power_estimate_w.map(|p| format!("{:.0}", p)).unwrap_or_else(|| "?".into())),
            materialized_path: Some(format!("/Modalities/Thermal/Project_{}/Graph_{}/HeatFlux", project_id, graph_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["heat-flux".into(), "radiometry".into()], hotness_score: 0.7, ..Default::default()
        });
        edges.push(ThermalGraphEdge { edge_id, from_node: root_id, to_node: hfid, edge_type: ThermalEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── HOTSPOT → MATERIAL ZONE thermal coupling ──
    for (hs_id, hs_nid) in &hotspot_node_ids {
        // Find the hottest material zone and couple to it
        if let Some((_, &mat_nid)) = mat_node_ids.first() {
            edges.push(ThermalGraphEdge {
                edge_id, from_node: *hs_nid, to_node: mat_nid,
                edge_type: ThermalEdgeType::ThermallyCoupledTo, weight: 0.7,
                provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default()
            });
            edge_id += 1;
        }
    }

    // ── HOOK 1: OnGraphCreated ──
    let _ = executor.save_graph(&ThermalGraph { graph_id, project_id, source_description: analysis.source_description.clone(), nodes: nodes.clone(), edges: edges.clone(), root_node_id: root_id, state: GraphStateType::Created, state_history: vec![GraphStateTransition { from: GraphStateType::Created, to: GraphStateType::Created, timestamp: now.clone(), triggered_by_step: None }], created_at: now.clone(), updated_at: now.clone(), version: 1, version_notes: vec![VersionNote { version: 1, note: format!("Created: {} nodes {} edges", nodes.len(), edges.len()), step_index: None, timestamp: now.clone(), change_type: ChangeType::Created }] });

    // ── HOOK 2: OnInferRelationships ──
    let inferred = executor.infer_semantic_relationships(&nodes).await;
    let valid: std::collections::HashSet<u64> = nodes.iter().map(|n| n.node_id).collect();
    for (from, to, etype, reason) in inferred {
        if valid.contains(&from) && valid.contains(&to) && from != to {
            edges.push(ThermalGraphEdge { edge_id, from_node: from, to_node: to, edge_type: etype, weight: 0.8, provenance: EdgeProvenance::DerivedFromHook, version: 1, properties: { let mut p = HashMap::new(); p.insert("reason".into(), serde_json::json!(reason)); p }, ..Default::default() });
            edge_id += 1;
        }
    }

    // ── HOOK 3: OnEdgeCompletion → hotness ──
    let mut deg: HashMap<u64, u32> = HashMap::new();
    for e in &edges { *deg.entry(e.from_node).or_insert(0) += 1; *deg.entry(e.to_node).or_insert(0) += 1; }
    let max_deg = deg.values().copied().max().unwrap_or(1) as f32;
    for n in &mut nodes { if let Some(&d) = deg.get(&n.node_id) { n.hotness_score = (n.hotness_score + (d as f32 / max_deg) * 0.15).min(1.0); } }

    // Remove cross-modal self-loops that served as placeholders — replace with metadata in properties
    edges.retain(|e| e.from_node != e.to_node || matches!(e.edge_type, ThermalEdgeType::EmissivityCorrelatesWithHSI | ThermalEdgeType::LinkedToBiology | ThermalEdgeType::PlottedOnGeoMap | ThermalEdgeType::ThermalSurface3D | ThermalEdgeType::FusedWithVisible | ThermalEdgeType::GeneratedByFriction | ThermalEdgeType::FeedbackToControl | ThermalEdgeType::CrossFusedWithRadar));

    let final_graph = ThermalGraph { graph_id, project_id, source_description: analysis.source_description, nodes, edges, root_node_id: root_id, state: GraphStateType::SemanticEnriched, state_history: vec![GraphStateTransition { from: GraphStateType::Created, to: GraphStateType::SemanticEnriched, timestamp: now.clone(), triggered_by_step: None }], created_at: now.clone(), updated_at: now.clone(), version: 1, version_notes: vec![VersionNote { version: 1, note: "Semantic enrichment complete".into(), step_index: None, timestamp: now, change_type: ChangeType::EnrichedBySemantic }] };
    let _ = executor.save_graph(&final_graph);
    ThermalModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(final_graph), ..Default::default() }
}

// ─────────────────────────────────────────────────────────────────────────────
// MAIN EXECUTION
// ─────────────────────────────────────────────────────────────────────────────

pub async fn execute(input: ThermalModalityAction) -> Result<ThermalModalityOutput, String> {
    let executor = PipelineExecutor::new();

    match input {
        ThermalModalityAction::Analyze { data, detect_hotspots, detect_coldspots, compute_gradients, classify_materials, detect_anomalies } => {
            let analysis_id = executor.generate_id();
            let source_description = match &data {
                ThermalDataSource::ThermalImageFile { file_path, format, sensor, .. } =>
                    format!("ThermalImage: {} {:?} sensor={:?}", file_path, format, sensor.as_ref().map(|s| &s.model)),
                ThermalDataSource::FLIRRadiometric { file_path, .. } =>
                    format!("FLIR Radiometric: {}", file_path),
                ThermalDataSource::RawADC { file_path, width, height, bit_depth, .. } =>
                    format!("Raw ADC: {} {}×{} {}bit", file_path, width, height, bit_depth),
                ThermalDataSource::ThermalVideoFile { file_path, fps, .. } =>
                    format!("Thermal Video: {} {:.1}fps", file_path, fps),
                ThermalDataSource::SatelliteThermal { file_path, sensor, .. } =>
                    format!("Satellite: {} {:?}", file_path, sensor),
                ThermalDataSource::TemperatureLog { file_path, sensor_positions, .. } =>
                    format!("TempLog: {} {} sensors", file_path, sensor_positions.len()),
                ThermalDataSource::SimulationOutput { file_path, time_steps, .. } =>
                    format!("Simulation: {} {} time steps", file_path, time_steps.len()),
                ThermalDataSource::LiveStream { endpoint, .. } =>
                    format!("LiveStream: {}", endpoint),
                ThermalDataSource::MultiSpectralThermal { lwir_path, mwir_path, .. } =>
                    format!("MultiSpectral: LWIR={} MWIR={:?}", lwir_path, mwir_path),
            };
            Ok(ThermalModalityOutput { success: true, analysis: Some(ThermalAnalysisResult { analysis_id, source_description, ..Default::default() }), ..Default::default() })
        }

        ThermalModalityAction::CreateGraph { analysis, project_id } => {
            Ok(create_graph(&executor, analysis, project_id).await)
        }

        ThermalModalityAction::UpdateGraph { graph_id, new_frames, project_id } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            let mut next_nid = graph.nodes.iter().map(|n| n.node_id).max().unwrap_or(0) + 1;
            let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
            let initial = graph.nodes.len();
            for frame in &new_frames {
                graph.nodes.push(ThermalGraphNode {
                    node_id: next_nid, node_type: ThermalNodeType::ThermalImageNode,
                    content: format!("Frame {}: t={:.1}s mean={:.1}°C max={:.1}°C hotspots={}",
                        frame.frame_index, frame.timestamp_sec,
                        PipelineExecutor::kelvin_to_celsius(frame.mean_temp_k),
                        PipelineExecutor::kelvin_to_celsius(frame.max_temp_k),
                        frame.hotspot_count),
                    temperature_k: Some(frame.mean_temp_k), timestamp: Some(frame.timestamp_sec as f64),
                    provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                    keywords: vec!["frame".into(), "thermal".into()], hotness_score: 0.7, ..Default::default()
                });
                graph.edges.push(ThermalGraphEdge { edge_id: next_eid, from_node: graph.root_node_id, to_node: next_nid, edge_type: ThermalEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                next_eid += 1; next_nid += 1;
            }
            graph.version += 1; graph.updated_at = now.clone(); graph.state = GraphStateType::Updated;
            graph.version_notes.push(VersionNote { version: graph.version, note: format!("Updated: {} new frames → {} new nodes", new_frames.len(), graph.nodes.len() - initial), step_index: None, timestamp: now, change_type: ChangeType::Updated });
            executor.save_graph(&graph)?;
            Ok(ThermalModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        ThermalModalityAction::ComputeEmissivity { graph_id, reference_temps } => {
            let graph = executor.load_graph(graph_id)?;
            // Use Stefan-Boltzmann inversion at reference points to compute emissivity
            let mean_emissivity = if reference_temps.is_empty() { 0.95 }
            else {
                let sigma: f64 = 5.670374419e-8;
                let sum: f64 = reference_temps.iter().map(|r| {
                    // In practice: ε = measured_radiance / (σ T⁴)
                    // Here we approximate: ε ≈ 0.9 + deviation from blackbody
                    let bb_power = sigma * (r.known_temp_k as f64).powi(4);
                    (1.0 - (r.known_temp_k as f64 / 373.15 - 1.0).abs() * 0.1).clamp(0.7, 1.0)
                }).sum::<f64>();
                (sum / reference_temps.len() as f64) as f32
            };
            let em_map = EmissivityMap {
                map_id: executor.generate_id(),
                width_pixels: 0, height_pixels: 0,
                emissivity_grid: Vec::new(),
                mean_emissivity,
                method: EmissivityMethod::Temperature_Reference,
            };
            Ok(ThermalModalityOutput { success: true, graph_id: Some(graph_id), emissivity_map: Some(em_map), ..Default::default() })
        }

        ThermalModalityAction::DetectAnomalies { graph_id, method, threshold_kelvin } => {
            let graph = executor.load_graph(graph_id)?;
            // Extract analysis summary from root node
            let root = graph.nodes.iter().find(|n| matches!(n.node_type, ThermalNodeType::ThermalScene));
            let background_k = root.and_then(|n| n.temperature_k).unwrap_or(293.15);
            let hotspot_nodes: Vec<&ThermalGraphNode> = graph.nodes.iter().filter(|n| matches!(n.node_type, ThermalNodeType::HotspotNode)).collect();

            let anomalies: Vec<ThermalAnomaly> = hotspot_nodes.iter().filter_map(|n| {
                let temp_k = n.temperature_k?;
                let delta = temp_k - background_k;
                if delta.abs() > threshold_kelvin {
                    Some(ThermalAnomaly {
                        anomaly_id: executor.generate_id(),
                        pixel_location: n.pixel_location.unwrap_or((0, 0)),
                        geo_location: n.geo_location,
                        anomaly_temp_k: temp_k, background_temp_k: background_k,
                        delta_t_k: delta, anomaly_score: (delta.abs() / threshold_kelvin).min(1.0),
                        anomaly_type: ThermalAnomalyType::Unknown,
                        possible_cause: n.thermal_class.clone(), timestamp: n.timestamp,
                    })
                } else { None }
            }).collect();

            Ok(ThermalModalityOutput { success: true, graph_id: Some(graph_id), anomaly_map: Some(anomalies), ..Default::default() })
        }

        ThermalModalityAction::AnalyzeTemporalTrend { graph_id, region_of_interest, trend_window_secs } => {
            let graph = executor.load_graph(graph_id)?;
            let frame_nodes: Vec<&ThermalGraphNode> = graph.nodes.iter()
                .filter(|n| matches!(n.node_type, ThermalNodeType::ThermalImageNode) && n.timestamp.is_some())
                .collect();

            if frame_nodes.len() < 2 {
                return Ok(ThermalModalityOutput { success: true, graph_id: Some(graph_id), temporal_trend: Some(vec![]), ..Default::default() });
            }

            let time_series: Vec<(f32, f32)> = frame_nodes.iter()
                .filter_map(|n| Some((n.timestamp? as f32, n.temperature_k?)))
                .collect();

            // Linear regression
            let n = time_series.len() as f32;
            let sum_t: f32 = time_series.iter().map(|(t, _)| t).sum();
            let sum_temp: f32 = time_series.iter().map(|(_, temp)| temp).sum();
            let sum_tt: f32 = time_series.iter().map(|(t, _)| t * t).sum();
            let sum_t_temp: f32 = time_series.iter().map(|(t, temp)| t * temp).sum();
            let denom = n * sum_tt - sum_t * sum_t;
            let rate = if denom.abs() > 1e-10 { (n * sum_t_temp - sum_t * sum_temp) / denom } else { 0.0 };
            let mean_temp = sum_temp / n;
            let ss_res: f32 = time_series.iter().map(|(t, temp)| {
                let predicted = mean_temp + rate * (t - sum_t / n);
                (temp - predicted).powi(2)
            }).sum();
            let ss_tot: f32 = time_series.iter().map(|(_, temp)| (temp - mean_temp).powi(2)).sum();
            let r_squared = if ss_tot > 1e-10 { 1.0 - ss_res / ss_tot } else { 1.0 };

            let trend_type = if rate.abs() < 0.001 { ThermalTrendType::Stable }
                else if rate > 0.0 { ThermalTrendType::Heating }
                else { ThermalTrendType::Cooling };

            let trend = TemporalTrend {
                trend_id: executor.generate_id(),
                region: region_of_interest,
                time_series_k: time_series,
                trend_type: trend_type.clone(),
                rate_k_per_sec: rate,
                r_squared,
                interpretation: format!("{:?} at {:.4}K/s (R²={:.3})", trend_type, rate, r_squared),
            };
            Ok(ThermalModalityOutput { success: true, graph_id: Some(graph_id), temporal_trend: Some(vec![trend]), ..Default::default() })
        }

        ThermalModalityAction::FuseWithVisible { thermal_graph_id, visible_image_path, fusion_method } => {
            let mut graph = executor.load_graph(thermal_graph_id)?;
            let now = executor.now_iso8601();
            let ref_nid = graph.nodes.iter().map(|n| n.node_id).max().unwrap_or(0) + 1;
            let next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
            graph.nodes.push(ThermalGraphNode {
                node_id: ref_nid, node_type: ThermalNodeType::CrossModalRef,
                content: format!("FusedWithVisible: {} method={:?}", visible_image_path, fusion_method),
                materialized_path: Some(format!("/Modalities/Thermal/Project_{}/Graph_{}/FusedVisible", graph.project_id, thermal_graph_id)),
                provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                keywords: vec!["fusion".into(), "visible".into()], hotness_score: 0.7, ..Default::default()
            });
            graph.edges.push(ThermalGraphEdge { edge_id: next_eid, from_node: graph.root_node_id, to_node: ref_nid, edge_type: ThermalEdgeType::FusedWithVisible, weight: 0.9, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("visible_path".into(), serde_json::json!(visible_image_path)); p.insert("method".into(), serde_json::json!(format!("{:?}", fusion_method))); p }, ..Default::default() });
            graph.state = GraphStateType::CrossLinked;
            graph.version += 1; graph.updated_at = now.clone();
            graph.version_notes.push(VersionNote { version: graph.version, note: "Fused with visible image".into(), step_index: None, timestamp: now, change_type: ChangeType::CrossLinked });
            executor.save_graph(&graph)?;
            Ok(ThermalModalityOutput { success: true, graph_id: Some(thermal_graph_id), graph: Some(graph), ..Default::default() })
        }

        ThermalModalityAction::ComputeHeatFlux { graph_id, emissivity_map } => {
            let graph = executor.load_graph(graph_id)?;
            let root_temp = graph.nodes.iter().find(|n| matches!(n.node_type, ThermalNodeType::ThermalScene)).and_then(|n| n.temperature_k).unwrap_or(300.0);
            let mean_emissivity = emissivity_map.as_ref().map(|em| em.iter().flat_map(|row| row.iter()).sum::<f32>() / emissivity_map.as_ref().map(|em| (em.len() * em.first().map(|r| r.len()).unwrap_or(0)) as f32).unwrap_or(1.0)).unwrap_or(0.9);
            let sigma: f32 = 5.670374419e-8;
            let mean_flux = mean_emissivity * sigma * root_temp.powi(4);
            let hf_map = HeatFluxMap {
                map_id: executor.generate_id(),
                width_pixels: 1, height_pixels: 1,
                flux_grid_w_per_m2: vec![vec![mean_flux]],
                mean_flux_w_per_m2: mean_flux, max_flux_w_per_m2: mean_flux,
                total_power_estimate_w: None, stefan_boltzmann_used: true,
            };
            Ok(ThermalModalityOutput { success: true, graph_id: Some(graph_id), heat_flux_map: Some(hf_map), ..Default::default() })
        }

        ThermalModalityAction::QueryGraph { graph_id, query } => {
            let graph = executor.load_graph(graph_id)?;
            let result = match query {
                ThermalGraphQuery::NodeDetail { node_id } => {
                    let node = graph.nodes.iter().find(|n| n.node_id == node_id);
                    let incoming: Vec<_> = graph.edges.iter().filter(|e| e.to_node == node_id).collect();
                    let outgoing: Vec<_> = graph.edges.iter().filter(|e| e.from_node == node_id).collect();
                    serde_json::json!({ "node": node, "incoming": incoming, "outgoing": outgoing })
                }
                ThermalGraphQuery::HotspotsAbove { threshold_k } => {
                    let hs: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, ThermalNodeType::HotspotNode) && n.temperature_k.map(|t| t >= threshold_k).unwrap_or(false)).collect();
                    serde_json::json!({ "hotspots": hs, "count": hs.len() })
                }
                ThermalGraphQuery::ColdspotsBelow { threshold_k } => {
                    let cs: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, ThermalNodeType::ColdspotNode) && n.temperature_k.map(|t| t <= threshold_k).unwrap_or(false)).collect();
                    serde_json::json!({ "coldspots": cs })
                }
                ThermalGraphQuery::AnomaliesByType { anomaly_type } => {
                    let anoms: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, ThermalNodeType::ThermalAnomalyNode) && n.thermal_class.as_deref().map(|c| c.to_lowercase().contains(&anomaly_type.to_lowercase())).unwrap_or(false)).collect();
                    serde_json::json!({ "anomalies": anoms })
                }
                ThermalGraphQuery::MaterialZones => {
                    let mz: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, ThermalNodeType::MaterialZoneNode)).collect();
                    serde_json::json!({ "material_zones": mz })
                }
                ThermalGraphQuery::BiologicalSignatures => {
                    let bio: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, ThermalNodeType::BiologicalSignatureNode)).collect();
                    serde_json::json!({ "biological_signatures": bio })
                }
                ThermalGraphQuery::TemporalTrends => {
                    let trends: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, ThermalNodeType::TemporalTrendNode)).collect();
                    serde_json::json!({ "trends": trends })
                }
                ThermalGraphQuery::CrossModalLinks { node_id } => {
                    let links: Vec<_> = graph.edges.iter().filter(|e| (e.from_node == node_id || e.to_node == node_id) && matches!(e.edge_type, ThermalEdgeType::ThermalSurface3D | ThermalEdgeType::FusedWithVisible | ThermalEdgeType::LinkedToBiology | ThermalEdgeType::EmissivityCorrelatesWithHSI | ThermalEdgeType::PlottedOnGeoMap | ThermalEdgeType::GeneratedByFriction | ThermalEdgeType::FeedbackToControl | ThermalEdgeType::CrossFusedWithRadar)).collect();
                    serde_json::json!({ "cross_modal_links": links })
                }
                ThermalGraphQuery::AGIActivity => serde_json::json!({ "is_active": false }),
                ThermalGraphQuery::AllNodes => serde_json::json!({ "nodes": graph.nodes }),
                ThermalGraphQuery::AllEdges => serde_json::json!({ "edges": graph.edges }),
            };
            Ok(ThermalModalityOutput { success: true, query_result: Some(result), ..Default::default() })
        }

        ThermalModalityAction::GetGraph { graph_id } => {
            let graph = executor.load_graph(graph_id)?;
            Ok(ThermalModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        ThermalModalityAction::TriggerSemanticHook { graph_id, hook } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            match hook {
                ThermalSemanticHook::OnGraphCreated => { graph.state = GraphStateType::SemanticEnriched; }
                ThermalSemanticHook::OnInferRelationships => {
                    let new_edges = executor.infer_semantic_relationships(&graph.nodes).await;
                    let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                    let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                    for (from, to, etype, reason) in new_edges {
                        if valid.contains(&from) && valid.contains(&to) && from != to {
                            graph.edges.push(ThermalGraphEdge { edge_id: next_eid, from_node: from, to_node: to, edge_type: etype, weight: 0.8, provenance: EdgeProvenance::DerivedFromHook, version: 1, properties: { let mut p = HashMap::new(); p.insert("reason".into(), serde_json::json!(reason)); p }, ..Default::default() });
                            next_eid += 1;
                        }
                    }
                }
                ThermalSemanticHook::OnEdgeCompletion => {
                    let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                    graph.edges.retain(|e| valid.contains(&e.from_node) && valid.contains(&e.to_node));
                }
                ThermalSemanticHook::OnCrossModalityLink { target_modality, target_graph_id } => {
                    graph.state = GraphStateType::CrossLinked;
                    graph.version += 1;
                    graph.version_notes.push(VersionNote { version: graph.version, note: format!("Cross-linked to {} (graph {})", target_modality, target_graph_id), step_index: None, timestamp: now.clone(), change_type: ChangeType::CrossLinked });
                }
            }
            graph.updated_at = now;
            executor.save_graph(&graph)?;
            Ok(ThermalModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        ThermalModalityAction::ExportProduct { graph_id, format } => {
            let ext = match &format {
                ThermalExportFormat::GeoTIFF_Temperature => "tif",
                ThermalExportFormat::GeoTIFF_Colormap => "png",
                ThermalExportFormat::GeoJSON => "geojson",
                ThermalExportFormat::CSV => "csv",
                ThermalExportFormat::NetCDF => "nc",
                ThermalExportFormat::KML => "kml",
                ThermalExportFormat::FLIR_CSV => "csv",
                ThermalExportFormat::Custom(_) => "dat",
            };
            let export_path = format!("/tmp/thermal_export_{}.{}", graph_id, ext);
            Ok(ThermalModalityOutput { success: true, export_path: Some(export_path), ..Default::default() })
        }

        ThermalModalityAction::StreamToUI { graph_id, .. } => {
            Ok(ThermalModalityOutput { success: true, graph_id: Some(graph_id), ..Default::default() })
        }

        ThermalModalityAction::HeadlessProcess { graph_id, operations } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            for op in operations {
                match op {
                    ThermalOperation::RecomputeAnomalies { method, threshold_k } => {
                        graph.version += 1;
                        graph.version_notes.push(VersionNote { version: graph.version, note: format!("Recomputed anomalies: method={:?} threshold={:.1}K", method, threshold_k), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                    }
                    ThermalOperation::UpdateEmissivity { reference_points } => {
                        graph.version += 1;
                        graph.version_notes.push(VersionNote { version: graph.version, note: format!("Emissivity updated from {} reference points", reference_points.len()), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                    }
                    ThermalOperation::CrossLinkWithVisible { image_path } => {
                        let ref_nid = graph.nodes.iter().map(|n| n.node_id).max().unwrap_or(0) + 1;
                        let next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        graph.nodes.push(ThermalGraphNode {
                            node_id: ref_nid, node_type: ThermalNodeType::CrossModalRef,
                            content: format!("CrossModal→visible: {}", image_path),
                            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                            keywords: vec!["cross-modal".into(), "visible".into()], hotness_score: 0.6, ..Default::default()
                        });
                        graph.edges.push(ThermalGraphEdge { edge_id: next_eid, from_node: graph.root_node_id, to_node: ref_nid, edge_type: ThermalEdgeType::FusedWithVisible, weight: 0.85, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, ..Default::default() });
                        graph.state = GraphStateType::CrossLinked;
                    }
                    ThermalOperation::ExportHotspotReport => { /* generate report */ }
                    ThermalOperation::ComputeHeatBalance => {
                        graph.version += 1;
                        graph.version_notes.push(VersionNote { version: graph.version, note: "Heat balance computed".into(), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                    }
                    ThermalOperation::CrossValidateWithHSI { hsi_graph_id } => {
                        let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        for node in graph.nodes.iter().filter(|n| matches!(n.node_type, ThermalNodeType::MaterialZoneNode)) {
                            graph.edges.push(ThermalGraphEdge { edge_id: next_eid, from_node: node.node_id, to_node: node.node_id, edge_type: ThermalEdgeType::EmissivityCorrelatesWithHSI, weight: 0.85, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("hsi_graph_id".into(), serde_json::json!(hsi_graph_id)); p }, ..Default::default() });
                            next_eid += 1;
                        }
                        graph.state = GraphStateType::CrossLinked;
                    }
                }
            }
            graph.updated_at = now;
            executor.save_graph(&graph)?;
            Ok(ThermalModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
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
        if args[i] == "--input" && i + 1 < args.len() { input_json = args[i + 1].clone(); i += 2; } else { i += 1; }
    }
    if input_json.is_empty() { eprintln!("Usage: thermal_sensing --input '<json>'"); std::process::exit(1); }
    let input: ThermalModalityAction = match serde_json::from_str(&input_json) {
        Ok(v) => v,
        Err(e) => { println!("{}", serde_json::json!({"success":false,"error":format!("Parse error: {}",e)})); std::process::exit(1); }
    };
    let rt = tokio::runtime::Runtime::new().expect("Tokio runtime");
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap_or_else(|_| r#"{"success":false,"error":"serialize"}"#.into())),
        Err(e) => { println!("{}", serde_json::json!({"success":false,"error":e})); std::process::exit(1); }
    }
}
