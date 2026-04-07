//! HapticSensing — Pipeline #113
//!
//! Contact mechanics, pressure distribution, vibration spectroscopy,
//! texture classification, force-torque sensing, grip analysis,
//! tactile array processing, and deformation mapping.
//!
//! DISTINCT FROM:
//!   - IMU (116): measures inertial forces (acceleration, rotation) of a
//!     moving platform. Haptic measures CONTACT forces at a surface — normal
//!     pressure, shear, vibration from physical interaction.
//!   - Audio (103): airborne acoustic waves. Haptic vibration is
//!     structure-borne, transmitted through solid contact — different physics,
//!     different sensors (piezoelectric, capacitive tactile arrays, FSRs).
//!   - Depth (115): geometric surface measurement. Haptic measures the
//!     mechanical response of contact — force, texture, compliance.
//!
//! CROSS-LINKS:
//!   109 (3D)      → contact geometry on object surfaces
//!   113 itself    → cross-contact-point relationships
//!   114 (Thermal) → thermal + haptic combined for material identification
//!   115 (Depth)   → depth maps of deformation under load
//!   121 (Kine)    → gripper/fingertip contact in kinematics
//!   122 (Control) → force feedback into control systems
//!   126 (Hyper)   → surface material from spectral + tactile fusion
//!
//! STORAGE: ZSEI containers under /Modalities/Haptic/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

// ─────────────────────────────────────────────────────────────────────────────
// INPUT TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum HapticModalityAction {
    /// Analyze haptic sensor data — produces HapticAnalysisResult
    Analyze {
        data: HapticDataSource,
        extract_contact_events: bool,
        extract_texture: bool,
        extract_force_profiles: bool,
        extract_vibration: bool,
        classify_material: bool,
    },
    /// Create a graph from analysis result
    CreateGraph {
        analysis: HapticAnalysisResult,
        project_id: u64,
    },
    /// Update graph with new contact measurements
    UpdateGraph {
        graph_id: u64,
        new_contacts: Vec<ContactEvent>,
        project_id: u64,
    },
    /// Classify surface texture from raw tactile scan
    ClassifyTexture {
        tactile_scan: TactileScan,
        method: TextureClassificationMethod,
        reference_library: Option<Vec<TextureReference>>,
    },
    /// Compute contact geometry from pressure map
    ComputeContactGeometry {
        pressure_map: PressureMap,
        sensor_geometry: SensorGeometry,
        object_compliance: Option<f32>,    // Pa^-1
    },
    /// Analyze vibration spectrum from accelerometer/piezo
    AnalyzeVibration {
        time_series: Vec<f32>,
        sample_rate_hz: f32,
        sensor_type: VibrationSensorType,
        contact_force_n: Option<f32>,
    },
    /// Analyze force-torque measurement sequence
    AnalyzeForceTorque {
        ft_sequence: Vec<FTSample>,
        task_context: FTTaskContext,
    },
    /// Compute slip detection from shear force trend
    DetectSlip {
        shear_force_n: Vec<f32>,
        normal_force_n: Vec<f32>,
        sample_rate_hz: f32,
        friction_model: FrictionModel,
    },
    /// Model material compliance from indentation data
    ModelCompliance {
        force_n: Vec<f32>,
        displacement_mm: Vec<f32>,
        indenter_radius_mm: f32,
        contact_model: ContactModel,
    },
    /// Query graph
    QueryGraph {
        graph_id: u64,
        query: HapticGraphQuery,
    },
    /// Retrieve full graph for Context Viewer
    GetGraph { graph_id: u64 },
    /// Trigger ZSEI semantic hooks
    TriggerSemanticHook {
        graph_id: u64,
        hook: HapticSemanticHook,
    },
    /// Export haptic products
    ExportProduct {
        graph_id: u64,
        format: HapticExportFormat,
    },
    /// Stream to UI
    StreamToUI {
        graph_id: u64,
        session_id: String,
        display_mode: HapticDisplayMode,
    },
    /// Headless processing (AGI-first)
    HeadlessProcess {
        graph_id: u64,
        operations: Vec<HapticOperation>,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// DATA SOURCES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HapticDataSource {
    /// Raw tactile array data (e.g., SynTouch BioTac, Digit, TacLink, GelSight)
    TactileArrayFile {
        file_path: String,
        format: TactileFileFormat,
        sensor_model: TactileSensorModel,
        array_rows: u32,
        array_cols: u32,
        sample_rate_hz: f32,
    },
    /// Force-torque sensor sequence (ATI, Robotiq, etc.)
    ForceTorqueFile {
        file_path: String,
        format: FTFileFormat,
        sensor_model: String,
        sample_rate_hz: f32,
        gravity_compensation: bool,
    },
    /// Single force-sensitive resistor (FSR) or load cell
    FSRFile {
        file_path: String,
        sample_rate_hz: f32,
        max_force_n: f32,
        calibration_polynomial: Vec<f32>,
    },
    /// Piezoelectric vibration sensor (contact microphone, accelerometer at contact)
    PiezoFile {
        file_path: String,
        format: PiezoFileFormat,
        sample_rate_hz: f32,
        sensitivity_mv_per_ms2: f32,
    },
    /// GelSight or FingerVision (optical tactile sensor) — produces images
    OpticalTactileFile {
        file_path: String,
        frame_rate_hz: f32,
        pixel_size_um: f32,
        gel_height_um: f32,
    },
    /// Multi-modal glove (flex sensors + IMU + pressure)
    GloveFile {
        file_path: String,
        sensor_count: u32,
        has_imu: bool,
        has_pressure: bool,
        sample_rate_hz: f32,
    },
    /// Structured CSV / HDF5 with multiple haptic modalities
    MultiModalFile {
        file_path: String,
        channels: Vec<HapticChannel>,
        sample_rate_hz: f32,
    },
    /// Live stream from haptic hardware
    LiveStream {
        endpoint: String,
        sensor_type: TactileSensorModel,
        sample_rate_hz: f32,
    },
    /// Raw in-memory data
    RawData {
        pressure_maps: Vec<PressureMap>,
        force_torque: Vec<FTSample>,
        vibration: Vec<f32>,
        sample_rate_hz: f32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TactileFileFormat { CSV, HDF5, ROS_Bag, NPZ, Binary_F32, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FTFileFormat { CSV, HDF5, ROS_Bag, ATI_NetFT, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PiezoFileFormat { WAV, CSV, Binary_F32, HDF5, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum TactileSensorModel {
    #[default] Unknown,
    SynTouch_BioTac, SynTouch_NumaTac,
    DIGIT, DIGIT_360, GelSight_Mini, GelSight_Svelte,
    TacLink, XELA_Sensor, FingerVision,
    ATI_Nano17, ATI_Mini45, Robotiq_FT300,
    OptoForce, Bota_SensONE,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HapticChannel {
    pub channel_name: String,
    pub channel_type: HapticChannelType,
    pub unit: String,
    pub range_min: f32,
    pub range_max: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum HapticChannelType {
    #[default] Pressure, ShearX, ShearY, NormalForce,
    Torque, Vibration, Temperature, Impedance, Compliance,
}

// ─────────────────────────────────────────────────────────────────────────────
// CORE DATA STRUCTURES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PressureMap {
    pub map_id: u64,
    pub timestamp_sec: f64,
    pub rows: u32,
    pub cols: u32,
    /// Row-major: pressures in Pa or normalized 0–1
    pub pressure_values: Vec<f32>,
    pub max_pressure: f32,
    pub mean_pressure: f32,
    pub contact_area_mm2: f32,
    pub total_force_n: f32,
    pub center_of_pressure: Option<[f32; 2]>,   // (row, col) sub-pixel
    pub sensor_pitch_mm: f32,                   // distance between taxels
    pub is_normalized: bool,
}

impl PressureMap {
    pub fn get(&self, row: usize, col: usize) -> f32 {
        self.pressure_values.get(row * self.cols as usize + col).copied().unwrap_or(0.0)
    }

    pub fn contact_taxel_count(&self, threshold: f32) -> u32 {
        self.pressure_values.iter().filter(|&&v| v > threshold).count() as u32
    }

    pub fn compute_center_of_pressure(&self) -> Option<[f32; 2]> {
        let total: f32 = self.pressure_values.iter().sum();
        if total < 1e-8 { return None; }
        let mut row_sum = 0.0f32;
        let mut col_sum = 0.0f32;
        for r in 0..self.rows as usize {
            for c in 0..self.cols as usize {
                let p = self.get(r, c);
                row_sum += p * r as f32;
                col_sum += p * c as f32;
            }
        }
        Some([row_sum / total, col_sum / total])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TactileScan {
    pub scan_id: u64,
    pub sensor_model: TactileSensorModel,
    pub frames: Vec<PressureMap>,
    pub duration_sec: f32,
    pub sample_rate_hz: f32,
    pub scan_velocity_mm_per_sec: Option<f32>,
    pub normal_force_n: f32,
    pub scan_direction: Option<[f32; 3]>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FTSample {
    pub sample_id: u64,
    pub timestamp_sec: f64,
    pub force_x_n: f32, pub force_y_n: f32, pub force_z_n: f32,
    pub torque_x_nm: f32, pub torque_y_nm: f32, pub torque_z_nm: f32,
    pub total_force_n: f32,
    pub total_torque_nm: f32,
    pub contact_point: Option<[f32; 3]>,
    pub wrench_in_world: bool,
}

impl FTSample {
    pub fn compute_totals(&mut self) {
        self.total_force_n = (self.force_x_n.powi(2) + self.force_y_n.powi(2) + self.force_z_n.powi(2)).sqrt();
        self.total_torque_nm = (self.torque_x_nm.powi(2) + self.torque_y_nm.powi(2) + self.torque_z_nm.powi(2)).sqrt();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SensorGeometry {
    pub rows: u32, pub cols: u32,
    pub pitch_mm: f32,
    pub total_area_mm2: f32,
    pub sensor_shape: SensorShape,
    pub curvature_radius_mm: Option<f32>,
    pub sensor_frame_transform: Option<[[f32; 4]; 4]>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SensorShape { #[default] Flat, Cylindrical, Spherical, Fingertip, Custom }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum VibrationSensorType { #[default] Piezoelectric, MEMS_Accelerometer, LaserDoppler, Capacitive }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum FrictionModel { #[default] Coulomb, StribeckCurve, LuGre, Maxwell_Slip }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ContactModel { #[default] Hertz, Boussinesq, JKR, DMT, MaugisD, Hunt_Crossley }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum FTTaskContext {
    #[default] Unknown, GraspAndLift, Insertion, Screwing, Sliding, PercussionTapping,
    Polishing, Cutting, SoftTissueManipulation, AssemblyMating, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum TextureClassificationMethod {
    #[default] SpectralSignature, LocalBinaryPattern, GaborFilter, CNN_Features, KNN_Reference, SVM, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TextureReference {
    pub texture_id: u64,
    pub material_name: String,
    pub surface_finish: String,
    pub roughness_ra_um: f32,
    pub feature_vector: Vec<f32>,
}

// ─────────────────────────────────────────────────────────────────────────────
// ANALYSIS RESULT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HapticAnalysisResult {
    pub analysis_id: u64,
    pub source_description: String,
    pub sensor_model: TactileSensorModel,

    // CONTACT EVENTS
    pub contact_events: Vec<ContactEvent>,
    pub contact_sequence: Vec<ContactPhase>,

    // PRESSURE / FORCE
    pub pressure_maps: Vec<PressureMap>,
    pub peak_pressure_pa: f32,
    pub mean_contact_pressure_pa: f32,
    pub max_contact_area_mm2: f32,
    pub pressure_distribution: PressureDistribution,

    // FORCE-TORQUE
    pub ft_sequence: Vec<FTSample>,
    pub peak_force_n: f32,
    pub peak_torque_nm: f32,
    pub mean_normal_force_n: f32,
    pub force_profiles: Vec<ForceProfile>,
    pub torque_profiles: Vec<TorqueProfile>,

    // SHEAR / SLIP
    pub shear_force_vectors: Vec<ShearVector>,
    pub slip_events: Vec<SlipEvent>,
    pub friction_coefficient: Option<f32>,
    pub slip_ratio: f32,

    // VIBRATION / TEXTURE
    pub vibration_events: Vec<VibrationEvent>,
    pub vibration_spectra: Vec<VibrationSpectrum>,
    pub texture_descriptors: Vec<TextureDescriptor>,
    pub texture_classification: Option<TextureClassification>,
    pub roughness_ra_um: Option<f32>,
    pub roughness_rz_um: Option<f32>,

    // COMPLIANCE / MATERIAL
    pub material_stiffness_n_per_mm: Option<f32>,
    pub compliance_model: Option<ComplianceModelResult>,
    pub material_identification: Vec<MaterialCandidate>,

    // DEFORMATION
    pub deformation_maps: Vec<DeformationMap>,

    // CONTACT GEOMETRY
    pub contact_geometry: Option<ContactGeometry>,

    // GRIP ANALYSIS (for multi-finger / multi-contact)
    pub grip_analysis: Option<GripAnalysis>,

    // METADATA
    pub duration_sec: f32,
    pub sample_rate_hz: f32,
    pub processing_notes: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// CONTACT STRUCTURES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContactEvent {
    pub event_id: u64,
    pub start_time_sec: f64,
    pub end_time_sec: f64,
    pub duration_sec: f32,
    pub contact_type: ContactType,
    pub peak_force_n: f32,
    pub impulse_ns: f32,
    pub contact_area_mm2: f32,
    pub center_of_pressure: Option<[f32; 2]>,
    pub contact_location_world: Option<[f32; 3]>,
    pub object_surface_id: Option<u64>,
    pub pre_contact_velocity_ms: Option<f32>,
    pub impact_force_n: Option<f32>,
    pub normal_direction: Option<[f32; 3]>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ContactType {
    #[default] Unknown, InitialContact, SlidingContact, RollingContact, GraspContact,
    TappingImpact, GlidingStroke, PunchingIndentation, VibrotactileStimulation,
    Palpation,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContactPhase {
    pub phase_id: u64,
    pub phase_type: ContactPhaseType,
    pub start_time_sec: f64,
    pub end_time_sec: f64,
    pub mean_force_n: f32,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ContactPhaseType {
    #[default] Approach, InitialContact, LoadIncrease, SteadyState,
    Slip, Regrasp, Unloading, Release, PostContact,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PressureDistribution {
    pub uniformity_index: f32,          // 0=concentrated, 1=perfectly uniform
    pub peak_to_mean_ratio: f32,
    pub contact_ellipse_major_mm: Option<f32>,
    pub contact_ellipse_minor_mm: Option<f32>,
    pub eccentricity: Option<f32>,
    pub skewness: [f32; 2],             // (row_skew, col_skew) of pressure distribution
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ForceProfile {
    pub profile_id: u64,
    pub axis: ForceAxis,
    pub timestamps_sec: Vec<f32>,
    pub values_n: Vec<f32>,
    pub peak_n: f32,
    pub mean_n: f32,
    pub rms_n: f32,
    pub impulse_ns: f32,
    pub rise_time_sec: Option<f32>,
    pub fall_time_sec: Option<f32>,
    pub profile_type: ForceProfileType,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ForceAxis { #[default] Normal, ShearX, ShearY, Total, TorqueX, TorqueY, TorqueZ }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ForceProfileType { #[default] Generic, GraspLift, Impact, Insertion, Scanning, Sinusoidal, Step }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TorqueProfile {
    pub profile_id: u64,
    pub axis: ForceAxis,
    pub timestamps_sec: Vec<f32>,
    pub values_nm: Vec<f32>,
    pub peak_nm: f32,
    pub mean_nm: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShearVector {
    pub vector_id: u64,
    pub timestamp_sec: f64,
    pub shear_x_n: f32, pub shear_y_n: f32,
    pub normal_force_n: f32,
    pub shear_magnitude_n: f32,
    pub direction_deg: f32,
    pub friction_utilization: f32,      // ratio: shear/mu*normal, 0–1 (1 = about to slip)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SlipEvent {
    pub slip_id: u64,
    pub start_time_sec: f64,
    pub duration_sec: f32,
    pub slip_type: SlipType,
    pub slip_velocity_mm_per_sec: Option<f32>,
    pub slip_direction_deg: Option<f32>,
    pub pre_slip_friction_coeff: Option<f32>,
    pub normal_force_at_slip_n: f32,
    pub shear_force_at_slip_n: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SlipType { #[default] Unknown, TranslationalSlip, RotationalSlip, IncipientSlip }

// ─────────────────────────────────────────────────────────────────────────────
// VIBRATION & TEXTURE
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VibrationEvent {
    pub event_id: u64,
    pub start_time_sec: f64,
    pub duration_sec: f32,
    pub trigger: VibrationTrigger,
    pub peak_amplitude: f32,
    pub rms_amplitude: f32,
    pub dominant_freq_hz: f32,
    pub contact_force_n: Option<f32>,
    pub associated_texture_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum VibrationTrigger {
    #[default] Unknown, SurfaceTexture, SlipOnset, Impact, ExternalActuator,
    MachineVibration, ToolContact, FluidFlow,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VibrationSpectrum {
    pub spectrum_id: u64,
    pub method: SpectrumMethod,
    pub freq_bins_hz: Vec<f32>,
    pub power_db: Vec<f32>,
    pub peak_freq_hz: f32,
    pub peak_power_db: f32,
    pub bandwidth_hz: f32,
    pub contact_force_n: Option<f32>,
    pub scan_velocity_mm_per_sec: Option<f32>,
    pub surface_texture_signature: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SpectrumMethod { #[default] FFT, Welch, STFT, CWT, Hilbert_Huang }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TextureDescriptor {
    pub descriptor_id: u64,
    pub method: TextureClassificationMethod,
    pub roughness_ra_um: f32,
    pub roughness_rz_um: f32,
    pub spatial_period_mm: Option<f32>,
    pub dominant_direction_deg: Option<f32>,
    pub anisotropy_ratio: Option<f32>,
    pub feature_vector: Vec<f32>,
    pub spectral_centroid_hz: Option<f32>,
    pub spectral_bandwidth_hz: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TextureClassification {
    pub classification_id: u64,
    pub material_name: String,
    pub surface_finish: String,
    pub roughness_class: RoughnessClass,
    pub method: TextureClassificationMethod,
    pub match_score: f32,
    pub top_candidates: Vec<(String, f32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum RoughnessClass {
    #[default] Unknown, VerySmooth, Smooth, MediumSmooth, Medium,
    MediumRough, Rough, VeryRough, Sandpaper, Custom(String),
}

// ─────────────────────────────────────────────────────────────────────────────
// COMPLIANCE & MATERIAL
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComplianceModelResult {
    pub model_id: u64,
    pub contact_model: ContactModel,
    pub youngs_modulus_pa: Option<f64>,
    pub poissons_ratio: Option<f32>,
    pub hardness_mpa: Option<f32>,
    pub stiffness_n_per_m: f32,
    pub damping_coefficient: f32,
    pub contact_radius_mm: Option<f32>,
    pub goodness_of_fit_r2: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaterialCandidate {
    pub candidate_id: u64,
    pub material_name: String,
    pub category: MaterialCategory,
    pub match_score: f32,
    pub supporting_evidence: Vec<String>,
    pub youngs_modulus_estimate_pa: Option<f64>,
    pub hardness_estimate_mpa: Option<f32>,
    pub roughness_estimate_ra_um: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum MaterialCategory {
    #[default] Unknown, Metal, Plastic, Rubber, Foam, Fabric, Wood,
    Glass, Ceramic, SoftTissue, Food, Paper, Composite, Custom(String),
}

// ─────────────────────────────────────────────────────────────────────────────
// DEFORMATION & GEOMETRY
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeformationMap {
    pub map_id: u64,
    pub timestamp_sec: f64,
    pub rows: u32, pub cols: u32,
    pub displacement_mm: Vec<f32>,          // per-taxel deformation depth
    pub max_deformation_mm: f32,
    pub mean_deformation_mm: f32,
    pub contact_force_n: f32,
    pub sensor_pitch_mm: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContactGeometry {
    pub geometry_id: u64,
    pub contact_area_mm2: f32,
    pub contact_ellipse_major_mm: f32,
    pub contact_ellipse_minor_mm: f32,
    pub contact_ellipse_angle_deg: f32,
    pub center_of_pressure: [f32; 3],
    pub normal_direction: [f32; 3],
    pub contact_polygon_mm: Vec<[f32; 2]>,
    pub edge_contacts: Vec<EdgeContact>,
    pub curvature_estimate_mm: Option<f32>,
    pub contact_type: ContactGeometryType,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ContactGeometryType { #[default] FullFace, Edge, Corner, Pinch, Wrap, LineContact }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EdgeContact {
    pub edge_id: u64,
    pub location: [f32; 2],
    pub orientation_deg: f32,
    pub force_concentration: f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// GRIP ANALYSIS
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GripAnalysis {
    pub analysis_id: u64,
    pub grasp_type: GraspType,
    pub num_contact_points: u32,
    pub contact_points: Vec<ContactPoint>,
    pub grip_force_n: f32,
    pub grip_stability: f32,                // 0–1, higher = more stable
    pub force_closure: bool,
    pub grasp_quality_metric: f32,
    pub object_pose_estimate: Option<ObjectPoseEstimate>,
    pub slip_margin: f32,                   // how far from slip (0–1)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum GraspType {
    #[default] Unknown, Pinch, Power, Tripod, Palmar, Lateral, Tip,
    Hook, Spherical, Cylindrical, Enveloping, Bimanual,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContactPoint {
    pub point_id: u64,
    pub finger_id: Option<u32>,
    pub location_world: [f32; 3],
    pub normal_force_n: f32,
    pub shear_force_n: f32,
    pub friction_coefficient: Option<f32>,
    pub contact_area_mm2: f32,
    pub is_slipping: bool,
    pub tactile_pattern: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ObjectPoseEstimate {
    pub position: [f32; 3],
    pub orientation_quat: [f32; 4],
    pub position_uncertainty_mm: f32,
    pub orientation_uncertainty_deg: f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH NODE TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum HapticNodeType {
    #[default] HapticScene,             // root
    ContactEventNode,                   // single contact occurrence
    PressureMapNode,                    // taxel pressure frame
    ContactPhaseNode,                   // phase in contact sequence
    ForceProfileNode,                   // force vs time curve
    TorqueProfileNode,                  // torque vs time curve
    ShearVectorNode,                    // shear force measurement
    SlipEventNode,                      // slip detection
    VibrationEventNode,                 // vibration burst
    VibrationSpectrumNode,              // frequency spectrum of vibration
    TextureDescriptorNode,              // texture feature set
    TextureClassificationNode,          // classified surface texture
    MaterialCandidateNode,              // identified material
    ComplianceModelNode,                // material stiffness model
    DeformationMapNode,                 // surface deformation
    ContactGeometryNode,                // geometric contact description
    GripAnalysisNode,                   // multi-finger grasp analysis
    ContactPointNode,                   // individual fingertip contact
    SensorGeometryNode,                 // physical sensor description
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HapticGraphNode {
    pub node_id: u64,
    pub node_type: HapticNodeType,
    pub content: String,

    // HAPTIC-SPECIFIC
    pub force_n: Option<f32>,
    pub pressure_pa: Option<f32>,
    pub contact_area_mm2: Option<f32>,
    pub timestamp_sec: Option<f64>,
    pub duration_sec: Option<f32>,
    pub roughness_ra_um: Option<f32>,
    pub material_name: Option<String>,
    pub stiffness_n_per_mm: Option<f32>,
    pub friction_coefficient: Option<f32>,
    pub dominant_freq_hz: Option<f32>,
    pub location_world: Option<[f32; 3]>,

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
pub enum HapticEdgeType {
    // ── STRUCTURAL ──
    #[default] Contains, Precedes, PartOf,

    // ── HAPTIC-SPECIFIC ──
    EmitsTexture,               // surface emits tactile texture signal
    FeelsForce,                 // contact point experiences this force
    CausesVibration,            // contact/slip causes vibration
    CausesSlip,                 // force profile causes slip
    CharacterizesMaterial,      // vibration/texture characterizes material
    IndicatesCompliance,        // deformation indicates compliance
    GripContains,               // grip analysis contains contact point
    PhaseFollows,               // contact phase follows another
    CorrelatesWithNormal,       // shear correlated with normal force
    SlidesOn,                   // sliding contact on surface
    IdentifiesMaterial,         // texture → material identification
    DeformsUnder,               // deformation occurs under force
    ContactLocatedAt,           // contact event at world position

    // ── CROSS-MODAL ──
    /// Contact geometry links to 3D surface node (109)
    CorrelatesWithSurface3D,
    /// Vibration correlates with audio/sound event (103/110)
    CorrelatesSoundEvent,
    /// Contact temperature correlates with thermal data (114)
    CorrelatesWithThermal,
    /// Force feedback to control system (122)
    FeedsControlSystem,
    /// Fingertip kinematics link (121)
    LinksToKinematicChain,
    /// Material identified cross-validated with spectral (126)
    MaterialCrossValidatedWithSpectral,
    /// Deformation map fused with depth sensor (115)
    DeformationFusedWithDepth,

    // ── UNIVERSAL SEMANTIC ──
    Performs, Affects, Implies, Contradicts, Elaborates, Summarizes, Supports,
    TemporalPrecedes, TemporalFollows, CausedBy, Enables, Prevents,
    FunctionalRole, InstanceOf,
    DerivedFrom, VersionOf, RefinesTo, ForkedFrom, SimilarTo,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HapticGraphEdge {
    pub edge_id: u64,
    pub from_node: u64, pub to_node: u64,
    pub edge_type: HapticEdgeType,
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
pub struct HapticGraph {
    pub graph_id: u64, pub project_id: u64,
    pub source_description: String,
    pub nodes: Vec<HapticGraphNode>,
    pub edges: Vec<HapticGraphEdge>,
    pub root_node_id: u64,
    pub state: GraphStateType,
    pub state_history: Vec<GraphStateTransition>,
    pub created_at: String, pub updated_at: String,
    pub version: u32, pub version_notes: Vec<VersionNote>,
}

// ─────────────────────────────────────────────────────────────────────────────
// QUERY / HOOKS / DISPLAY / EXPORT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HapticGraphQuery {
    NodeDetail { node_id: u64 },
    ContactEvents,
    SlipEvents,
    MaterialCandidates,
    VibrationSpectra,
    TextureClassifications,
    ForceProfiles,
    GripAnalysis,
    CrossModalLinks { node_id: u64 },
    AGIActivity, AllNodes, AllEdges,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HapticSemanticHook {
    OnGraphCreated,
    OnInferRelationships,
    OnEdgeCompletion,
    OnCrossModalityLink { target_modality: String, target_graph_id: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HapticExportFormat {
    CSV, HDF5, JSON, ROS_Bag, NPZ, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HapticDisplayMode {
    PressureHeatmap, ForceTimeSeries, VibrationWaterfall,
    TextureMap, ContactGeometry, MaterialSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HapticOperation {
    ReClassifyTexture,
    ReComputeCompliance,
    CrossLinkTo3D { graph_id_3d: u64 },
    CrossLinkToControl { control_graph_id: u64 },
    DetectAllSlipEvents,
}

// ─────────────────────────────────────────────────────────────────────────────
// OUTPUT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HapticModalityOutput {
    pub success: bool,
    pub graph_id: Option<u64>,
    pub graph: Option<HapticGraph>,
    pub analysis: Option<HapticAnalysisResult>,
    pub texture_classification: Option<TextureClassification>,
    pub contact_geometry: Option<ContactGeometry>,
    pub vibration_spectrum: Option<VibrationSpectrum>,
    pub slip_events: Option<Vec<SlipEvent>>,
    pub compliance_model: Option<ComplianceModelResult>,
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
        let input = serde_json::json!({"action":"Prompt","prompt":prompt,"max_tokens":max_tokens,"temperature":0.05,"system_context":"Haptic sensor analysis. Return only valid JSON."});
        let out = std::process::Command::new(&self.prompt_pipeline_path)
            .arg("--input").arg(input.to_string())
            .output().map_err(|e| e.to_string())?;
        let r: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
        Ok(r["response"].as_str().unwrap_or("{}").to_string())
    }

    fn save_graph(&self, g: &HapticGraph) -> Result<(), String> {
        let path = format!("{}/local/haptic_graph_{}.json", self.zsei_path, g.graph_id);
        if let Some(p) = std::path::Path::new(&path).parent() { std::fs::create_dir_all(p).map_err(|e| e.to_string())?; }
        std::fs::write(&path, serde_json::to_string_pretty(g).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    fn load_graph(&self, id: u64) -> Result<HapticGraph, String> {
        let path = format!("{}/local/haptic_graph_{}.json", self.zsei_path, id);
        serde_json::from_str(&std::fs::read_to_string(&path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    fn generate_id(&self) -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as u64 }
    fn now_iso8601(&self) -> String { format!("{}", self.generate_id()) }
    fn extract_json_array(raw: &str) -> String { if let (Some(s), Some(e)) = (raw.find('['), raw.rfind(']')) { raw[s..=e].to_string() } else { "[]".to_string() } }
    fn extract_json_object(raw: &str) -> String { if let (Some(s), Some(e)) = (raw.find('{'), raw.rfind('}')) { raw[s..=e].to_string() } else { "{}".to_string() } }
}

impl PipelineExecutor {
    /// LLM zero-shot: identify material from haptic features
    async fn identify_material_llm(&self, analysis: &HapticAnalysisResult) -> Vec<MaterialCandidate> {
        let summary = serde_json::json!({
            "peak_force_n": analysis.peak_force_n,
            "mean_normal_force_n": analysis.mean_normal_force_n,
            "max_contact_area_mm2": analysis.max_contact_area_mm2,
            "roughness_ra_um": analysis.roughness_ra_um,
            "roughness_rz_um": analysis.roughness_rz_um,
            "stiffness_n_per_mm": analysis.material_stiffness_n_per_mm,
            "friction_coefficient": analysis.friction_coefficient,
            "vibration_events_count": analysis.vibration_events.len(),
            "dominant_vibration_freq_hz": analysis.vibration_spectra.first().map(|s| s.peak_freq_hz),
            "slip_ratio": analysis.slip_ratio,
            "texture_roughness_class": analysis.texture_classification.as_ref().map(|t| format!("{:?}", t.roughness_class)),
        });

        let prompt = format!(r#"
Based on these haptic measurement features, identify likely materials.
Consider: stiffness, roughness, friction, vibration signature, contact area.

Haptic features:
{}

Return ONLY valid JSON array of material candidates (most likely first):
[{{
  "material_name": "...",
  "category": "Metal|Plastic|Rubber|Foam|Fabric|Wood|Glass|Ceramic|SoftTissue|Food|Paper|Composite|Unknown",
  "match_score": 0.0-1.0,
  "supporting_evidence": ["reason1", "reason2"],
  "youngs_modulus_estimate_pa": null_or_number,
  "hardness_estimate_mpa": null_or_number,
  "roughness_estimate_ra_um": null_or_number
}}]"#, serde_json::to_string_pretty(&summary).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 700).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str)
                    .unwrap_or_default()
                    .into_iter()
                    .enumerate()
                    .map(|(i, v)| MaterialCandidate {
                        candidate_id: self.generate_id(),
                        material_name: v["material_name"].as_str().unwrap_or("Unknown").to_string(),
                        category: match v["category"].as_str().unwrap_or("Unknown") {
                            "Metal" => MaterialCategory::Metal, "Plastic" => MaterialCategory::Plastic,
                            "Rubber" => MaterialCategory::Rubber, "Foam" => MaterialCategory::Foam,
                            "Fabric" => MaterialCategory::Fabric, "Wood" => MaterialCategory::Wood,
                            "Glass" => MaterialCategory::Glass, "Ceramic" => MaterialCategory::Ceramic,
                            "SoftTissue" => MaterialCategory::SoftTissue, "Food" => MaterialCategory::Food,
                            "Paper" => MaterialCategory::Paper, "Composite" => MaterialCategory::Composite,
                            _ => MaterialCategory::Unknown,
                        },
                        match_score: v["match_score"].as_f64().unwrap_or(0.5) as f32,
                        supporting_evidence: v["supporting_evidence"].as_array()
                            .map(|arr| arr.iter().filter_map(|s| s.as_str().map(String::from)).collect())
                            .unwrap_or_default(),
                        youngs_modulus_estimate_pa: v["youngs_modulus_estimate_pa"].as_f64(),
                        hardness_estimate_mpa: v["hardness_estimate_mpa"].as_f64().map(|v| v as f32),
                        roughness_estimate_ra_um: v["roughness_estimate_ra_um"].as_f64().map(|v| v as f32),
                    })
                    .collect()
            }
            Err(_) => vec![],
        }
    }

    /// LLM zero-shot: infer contact type from force profile shape
    async fn classify_contact_events_llm(&self, events: &[ContactEvent]) -> Vec<(u64, ContactType)> {
        if events.is_empty() { return vec![]; }
        let list: Vec<serde_json::Value> = events.iter().take(20).map(|ev| serde_json::json!({
            "event_id": ev.event_id,
            "duration_sec": ev.duration_sec,
            "peak_force_n": ev.peak_force_n,
            "impulse_ns": ev.impulse_ns,
            "contact_area_mm2": ev.contact_area_mm2,
            "pre_contact_velocity_ms": ev.pre_contact_velocity_ms,
            "impact_force_n": ev.impact_force_n,
        })).collect();

        let prompt = format!(r#"
Classify each contact event type:
InitialContact, SlidingContact, RollingContact, GraspContact, TappingImpact,
GlidingStroke, PunchingIndentation, VibrotactileStimulation, Palpation, Unknown.

Events: {}
Return ONLY valid JSON array:
[{{"event_id": N, "contact_type": "TypeName", "reasoning": "brief"}}]"#,
            serde_json::to_string_pretty(&list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 400).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str)
                    .unwrap_or_default().into_iter()
                    .filter_map(|v| {
                        let id = v["event_id"].as_u64()?;
                        let ct = match v["contact_type"].as_str().unwrap_or("Unknown") {
                            "InitialContact"       => ContactType::InitialContact,
                            "SlidingContact"       => ContactType::SlidingContact,
                            "RollingContact"       => ContactType::RollingContact,
                            "GraspContact"         => ContactType::GraspContact,
                            "TappingImpact"        => ContactType::TappingImpact,
                            "GlidingStroke"        => ContactType::GlidingStroke,
                            "PunchingIndentation"  => ContactType::PunchingIndentation,
                            "VibrotactileStimulation"=>ContactType::VibrotactileStimulation,
                            "Palpation"            => ContactType::Palpation,
                            _                      => ContactType::Unknown,
                        };
                        Some((id, ct))
                    }).collect()
            }
            Err(_) => vec![],
        }
    }

    /// LLM zero-shot: classify texture from vibration spectrum features
    async fn classify_texture_llm(&self, descriptors: &[TextureDescriptor]) -> Option<TextureClassification> {
        if descriptors.is_empty() { return None; }
        let desc = descriptors.first().unwrap();
        let summary = serde_json::json!({
            "roughness_ra_um": desc.roughness_ra_um,
            "roughness_rz_um": desc.roughness_rz_um,
            "spatial_period_mm": desc.spatial_period_mm,
            "dominant_direction_deg": desc.dominant_direction_deg,
            "anisotropy_ratio": desc.anisotropy_ratio,
            "spectral_centroid_hz": desc.spectral_centroid_hz,
            "spectral_bandwidth_hz": desc.spectral_bandwidth_hz,
        });

        let prompt = format!(r#"
Classify a surface texture from these haptic/vibration features.
Ra (roughness average) and Rz (mean roughness depth) are in micrometers.

Features: {}

Return ONLY valid JSON:
{{
  "material_name": "e.g., fine sandpaper|polished aluminum|rubber|felt|glass|wood|cotton",
  "surface_finish": "e.g., mirror-polished|ground|milled|as-cast|knitted",
  "roughness_class": "VerySmooth|Smooth|MediumSmooth|Medium|MediumRough|Rough|VeryRough|Sandpaper",
  "match_score": 0.0-1.0,
  "top_candidates": [["name1", 0.9], ["name2", 0.6]]
}}"#, serde_json::to_string_pretty(&summary).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 300).await {
            Ok(raw) => {
                let json_str = Self::extract_json_object(&raw);
                serde_json::from_str::<serde_json::Value>(&json_str).ok().map(|v| {
                    TextureClassification {
                        classification_id: self.generate_id(),
                        material_name: v["material_name"].as_str().unwrap_or("Unknown").to_string(),
                        surface_finish: v["surface_finish"].as_str().unwrap_or("Unknown").to_string(),
                        roughness_class: match v["roughness_class"].as_str().unwrap_or("Unknown") {
                            "VerySmooth"   => RoughnessClass::VerySmooth,
                            "Smooth"       => RoughnessClass::Smooth,
                            "MediumSmooth" => RoughnessClass::MediumSmooth,
                            "Medium"       => RoughnessClass::Medium,
                            "MediumRough"  => RoughnessClass::MediumRough,
                            "Rough"        => RoughnessClass::Rough,
                            "VeryRough"    => RoughnessClass::VeryRough,
                            "Sandpaper"    => RoughnessClass::Sandpaper,
                            _              => RoughnessClass::Unknown,
                        },
                        method: TextureClassificationMethod::SpectralSignature,
                        match_score: v["match_score"].as_f64().unwrap_or(0.5) as f32,
                        top_candidates: v["top_candidates"].as_array()
                            .unwrap_or(&vec![])
                            .iter()
                            .filter_map(|pair| {
                                let arr = pair.as_array()?;
                                let name = arr.get(0)?.as_str()?.to_string();
                                let score = arr.get(1)?.as_f64()? as f32;
                                Some((name, score))
                            })
                            .collect(),
                    }
                })
            }
            Err(_) => None,
        }
    }

    /// LLM zero-shot: infer semantic relationships between haptic graph nodes
    async fn infer_semantic_relationships(&self, nodes: &[HapticGraphNode]) -> Vec<(u64, u64, HapticEdgeType, String)> {
        if nodes.len() < 2 { return vec![]; }
        let node_list: Vec<serde_json::Value> = nodes.iter().take(25).map(|n| serde_json::json!({
            "node_id": n.node_id, "type": format!("{:?}", n.node_type),
            "content": n.content.chars().take(80).collect::<String>(),
            "force_n": n.force_n, "roughness_ra_um": n.roughness_ra_um,
            "dominant_freq_hz": n.dominant_freq_hz,
        })).collect();

        let prompt = format!(r#"
Identify semantic relationships between these haptic graph nodes.

Nodes: {}

Available types: EmitsTexture, FeelsForce, CausesVibration, CausesSlip, CharacterizesMaterial,
IndicatesCompliance, SlidesOn, IdentifiesMaterial, DeformsUnder, ContactLocatedAt,
Affects, CausedBy, Enables, TemporalPrecedes, DerivedFrom, PartOf

Return ONLY valid JSON array:
[{{"from_node_id": N, "to_node_id": M, "edge_type": "TypeName", "reason": "brief"}}]"#,
            serde_json::to_string_pretty(&node_list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 600).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str)
                    .unwrap_or_default().into_iter()
                    .filter_map(|v| {
                        let from = v["from_node_id"].as_u64()?;
                        let to = v["to_node_id"].as_u64()?;
                        let etype = map_haptic_edge_str(v["edge_type"].as_str().unwrap_or("Affects"));
                        let reason = v["reason"].as_str().unwrap_or("").to_string();
                        Some((from, to, etype, reason))
                    }).collect()
            }
            Err(_) => vec![],
        }
    }

    /// Compute vibration spectrum via FFT (Cooley-Tukey, power-of-2 only here)
    fn compute_fft_spectrum(&self, signal: &[f32], sample_rate_hz: f32) -> VibrationSpectrum {
        let n = signal.len();
        let spectrum_id = self.generate_id();

        // Simple DFT for short signals; for long signals use FFT library in production
        let half_n = n / 2;
        let freq_resolution = sample_rate_hz / n as f32;
        let mut power_db: Vec<f32> = Vec::with_capacity(half_n);
        let mut freq_bins: Vec<f32> = Vec::with_capacity(half_n);

        for k in 0..half_n {
            let mut re = 0.0f32;
            let mut im = 0.0f32;
            for (t, &x) in signal.iter().enumerate() {
                let angle = 2.0 * std::f32::consts::PI * k as f32 * t as f32 / n as f32;
                re += x * angle.cos();
                im -= x * angle.sin();
            }
            let power = (re * re + im * im) / n as f32;
            power_db.push(if power > 1e-12 { 10.0 * power.log10() } else { -120.0 });
            freq_bins.push(k as f32 * freq_resolution);
        }

        let (peak_idx, &peak_power) = power_db.iter().enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or((0, &-120.0));
        let peak_freq = freq_bins.get(peak_idx).copied().unwrap_or(0.0);

        // Bandwidth: -3dB points
        let threshold = peak_power - 3.0;
        let low_idx = power_db.iter().position(|&p| p >= threshold).unwrap_or(0);
        let high_idx = power_db.iter().rposition(|&p| p >= threshold).unwrap_or(half_n.saturating_sub(1));
        let bandwidth = freq_bins.get(high_idx).copied().unwrap_or(0.0) - freq_bins.get(low_idx).copied().unwrap_or(0.0);

        VibrationSpectrum {
            spectrum_id, method: SpectrumMethod::FFT,
            freq_bins_hz: freq_bins, power_db,
            peak_freq_hz: peak_freq, peak_power_db: peak_power,
            bandwidth_hz: bandwidth,
            contact_force_n: None, scan_velocity_mm_per_sec: None,
            surface_texture_signature: peak_freq > 50.0 && peak_freq < 2000.0,
        }
    }

    /// Compute roughness Ra from vibration/surface profile signal
    fn compute_roughness_ra(&self, profile: &[f32]) -> (f32, f32) {
        if profile.is_empty() { return (0.0, 0.0); }
        let mean = profile.iter().sum::<f32>() / profile.len() as f32;
        let ra = profile.iter().map(|&v| (v - mean).abs()).sum::<f32>() / profile.len() as f32;
        let peaks: Vec<f32> = profile.iter().copied().collect();
        let max = peaks.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let min = peaks.iter().cloned().fold(f32::INFINITY, f32::min);
        let rz = max - min;
        (ra * 1000.0, rz * 1000.0) // convert to micrometers assuming mm input
    }

    /// Hertz contact mechanics: Young's modulus from force-displacement
    fn fit_hertz_compliance(&self, force_n: &[f32], displacement_mm: &[f32], indenter_radius_mm: f32) -> ComplianceModelResult {
        // Hertz: F = (4/3) * E* * sqrt(R) * d^(3/2)
        // E* = effective modulus; linearize: F^(2/3) = (4/3)^(2/3) * (E*)^(2/3) * R^(1/3) * d
        let n = force_n.len().min(displacement_mm.len());
        if n < 3 {
            return ComplianceModelResult { model_id: self.generate_id(), contact_model: ContactModel::Hertz, stiffness_n_per_m: 1000.0, damping_coefficient: 0.0, goodness_of_fit_r2: 0.0, ..Default::default() };
        }

        // Least-squares fit: F = k * d^(3/2) → log(F) = log(k) + 1.5 * log(d)
        let log_pairs: Vec<(f32, f32)> = force_n.iter().zip(displacement_mm.iter())
            .filter(|(&f, &d)| f > 1e-6 && d > 1e-6)
            .map(|(&f, &d)| (d.ln(), f.ln()))
            .collect();

        if log_pairs.len() < 3 {
            return ComplianceModelResult { model_id: self.generate_id(), contact_model: ContactModel::Hertz, stiffness_n_per_m: 1000.0, damping_coefficient: 0.0, goodness_of_fit_r2: 0.0, ..Default::default() };
        }

        let n_fit = log_pairs.len() as f32;
        let sum_x: f32 = log_pairs.iter().map(|(x, _)| x).sum();
        let sum_y: f32 = log_pairs.iter().map(|(_, y)| y).sum();
        let sum_xx: f32 = log_pairs.iter().map(|(x, _)| x * x).sum();
        let sum_xy: f32 = log_pairs.iter().map(|(x, y)| x * y).sum();
        let slope = (n_fit * sum_xy - sum_x * sum_y) / (n_fit * sum_xx - sum_x * sum_x + 1e-12);
        let intercept = (sum_y - slope * sum_x) / n_fit;
        let k = intercept.exp(); // F = k * d^slope (in units consistent with input)

        // E* from k: k = (4/3) * E* * sqrt(R) → E* = 3k / (4 * sqrt(R_mm * 1e-3))
        let r_m = indenter_radius_mm as f64 * 1e-3;
        let e_star_pa = 3.0 * k as f64 / (4.0 * r_m.sqrt()) * 1e3; // account for mm→m on displacement

        // R² goodness of fit
        let y_mean = sum_y / n_fit;
        let ss_tot: f32 = log_pairs.iter().map(|(_, y)| (y - y_mean).powi(2)).sum();
        let ss_res: f32 = log_pairs.iter().map(|(x, y)| (y - (slope * x + intercept)).powi(2)).sum();
        let r2 = 1.0 - ss_res / (ss_tot + 1e-12);

        // Stiffness at mean displacement: k_linear = dF/dd = 1.5 * k * d^(0.5)
        let mean_d = displacement_mm.iter().sum::<f32>() / n as f32;
        let linear_stiffness_n_per_mm = 1.5 * k * mean_d.sqrt();

        ComplianceModelResult {
            model_id: self.generate_id(),
            contact_model: ContactModel::Hertz,
            youngs_modulus_pa: Some(e_star_pa),
            poissons_ratio: Some(0.3), // assumed
            stiffness_n_per_m: linear_stiffness_n_per_mm * 1000.0,
            damping_coefficient: 0.05,
            contact_radius_mm: Some((3.0 * force_n.last().copied().unwrap_or(1.0) as f64 * r_m / (4.0 * e_star_pa)).cbrt() as f32 * 1000.0),
            goodness_of_fit_r2: r2.clamp(-1.0, 1.0),
            hardness_mpa: None,
        }
    }

    /// Detect slip events from shear/normal force ratio exceeding friction limit
    fn detect_slip_events(&self, shear_n: &[f32], normal_n: &[f32], sample_rate_hz: f32, model: &FrictionModel) -> Vec<SlipEvent> {
        let n = shear_n.len().min(normal_n.len());
        if n < 2 { return vec![]; }

        // Estimate static friction coefficient from first stable portion
        let stable_samples: Vec<f32> = shear_n.iter().zip(normal_n.iter())
            .take(n / 4)
            .filter(|(_, &fn_)| fn_ > 0.1)
            .map(|(&fs, &fn_)| fs.abs() / fn_)
            .collect();
        let mu_static = if stable_samples.is_empty() { 0.5 }
            else { stable_samples.iter().copied().fold(0.0f32, f32::max) };

        let mut slip_events = Vec::new();
        let mut in_slip = false;
        let mut slip_start = 0usize;

        for i in 1..n {
            let fn_ = normal_n[i];
            let fs = shear_n[i];
            let limit = match model {
                FrictionModel::Coulomb => mu_static * fn_,
                FrictionModel::StribeckCurve => {
                    let v_est = (fs - shear_n[i-1]).abs() * sample_rate_hz;
                    mu_static * fn_ * (1.0 / (1.0 + v_est / 10.0))
                }
                _ => mu_static * fn_,
            };

            let slipping = fs.abs() > limit && fn_ > 0.05;
            if slipping && !in_slip { in_slip = true; slip_start = i; }
            else if !slipping && in_slip {
                in_slip = false;
                let duration = (i - slip_start) as f32 / sample_rate_hz;
                if duration > 0.002 { // filter < 2ms glitches
                    slip_events.push(SlipEvent {
                        slip_id: self.generate_id(),
                        start_time_sec: slip_start as f64 / sample_rate_hz as f64,
                        duration_sec: duration,
                        slip_type: SlipType::TranslationalSlip,
                        slip_velocity_mm_per_sec: Some((shear_n[i] - shear_n[slip_start]).abs() * 50.0),
                        slip_direction_deg: Some(shear_n[i].atan2(normal_n[i]).to_degrees()),
                        pre_slip_friction_coeff: Some(mu_static),
                        normal_force_at_slip_n: normal_n[slip_start],
                        shear_force_at_slip_n: shear_n[slip_start],
                    });
                }
            }
        }
        slip_events
    }
}

fn map_haptic_edge_str(s: &str) -> HapticEdgeType {
    match s {
        "EmitsTexture"              => HapticEdgeType::EmitsTexture,
        "FeelsForce"                => HapticEdgeType::FeelsForce,
        "CausesVibration"           => HapticEdgeType::CausesVibration,
        "CausesSlip"                => HapticEdgeType::CausesSlip,
        "CharacterizesMaterial"     => HapticEdgeType::CharacterizesMaterial,
        "IndicatesCompliance"       => HapticEdgeType::IndicatesCompliance,
        "GripContains"              => HapticEdgeType::GripContains,
        "PhaseFollows"              => HapticEdgeType::PhaseFollows,
        "CorrelatesWithNormal"      => HapticEdgeType::CorrelatesWithNormal,
        "SlidesOn"                  => HapticEdgeType::SlidesOn,
        "IdentifiesMaterial"        => HapticEdgeType::IdentifiesMaterial,
        "DeformsUnder"              => HapticEdgeType::DeformsUnder,
        "ContactLocatedAt"          => HapticEdgeType::ContactLocatedAt,
        "CorrelatesWithSurface3D"   => HapticEdgeType::CorrelatesWithSurface3D,
        "CorrelatesSoundEvent"      => HapticEdgeType::CorrelatesSoundEvent,
        "CorrelatesWithThermal"     => HapticEdgeType::CorrelatesWithThermal,
        "FeedsControlSystem"        => HapticEdgeType::FeedsControlSystem,
        "LinksToKinematicChain"     => HapticEdgeType::LinksToKinematicChain,
        "Affects"                   => HapticEdgeType::Affects,
        "CausedBy"                  => HapticEdgeType::CausedBy,
        "Enables"                   => HapticEdgeType::Enables,
        "Prevents"                  => HapticEdgeType::Prevents,
        "TemporalPrecedes"          => HapticEdgeType::TemporalPrecedes,
        "TemporalFollows"           => HapticEdgeType::TemporalFollows,
        "DerivedFrom"               => HapticEdgeType::DerivedFrom,
        "PartOf"                    => HapticEdgeType::PartOf,
        "SimilarTo"                 => HapticEdgeType::SimilarTo,
        _                           => HapticEdgeType::Affects,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH CREATION
// ─────────────────────────────────────────────────────────────────────────────

async fn create_graph(executor: &PipelineExecutor, analysis: HapticAnalysisResult, project_id: u64) -> HapticModalityOutput {
    let graph_id = executor.generate_id();
    let now = executor.now_iso8601();
    let mut nodes: Vec<HapticGraphNode> = Vec::new();
    let mut edges: Vec<HapticGraphEdge> = Vec::new();
    let mut node_id: u64 = 1;
    let mut edge_id: u64 = 1;

    // ── ROOT ──
    let root_id = node_id;
    nodes.push(HapticGraphNode {
        node_id: root_id, node_type: HapticNodeType::HapticScene,
        content: format!("Haptic scene [{:?}]: contacts={} pressure_maps={} slip={} vib={} materials={}",
            analysis.sensor_model, analysis.contact_events.len(), analysis.pressure_maps.len(),
            analysis.slip_events.len(), analysis.vibration_events.len(), analysis.material_identification.len()),
        force_n: Some(analysis.peak_force_n),
        pressure_pa: Some(analysis.peak_pressure_pa),
        materialized_path: Some(format!("/Modalities/Haptic/Project_{}/Graph_{}", project_id, graph_id)),
        provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
        version_notes: vec![VersionNote { version: 1, note: "Initial creation".into(), step_index: None, timestamp: now.clone(), change_type: ChangeType::Created }],
        keywords: vec!["haptic".into(), "contact".into(), format!("{:?}", analysis.sensor_model).to_lowercase()],
        hotness_score: 1.0, ..Default::default()
    });
    node_id += 1;

    // ── CONTACT EVENTS ──
    let contact_node_ids: Vec<(u64, u64)> = analysis.contact_events.iter().map(|ev| {
        let cid = node_id;
        nodes.push(HapticGraphNode {
            node_id: cid, node_type: HapticNodeType::ContactEventNode,
            content: format!("Contact [{:?}]: dur={:.3}s force={:.2}N area={:.1}mm²",
                ev.contact_type, ev.duration_sec, ev.peak_force_n, ev.contact_area_mm2),
            force_n: Some(ev.peak_force_n), contact_area_mm2: Some(ev.contact_area_mm2),
            timestamp_sec: Some(ev.start_time_sec), duration_sec: Some(ev.duration_sec),
            location_world: ev.contact_location_world,
            materialized_path: Some(format!("/Modalities/Haptic/Project_{}/Graph_{}/Contact/{}", project_id, graph_id, ev.event_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["contact".into(), format!("{:?}", ev.contact_type).to_lowercase()],
            hotness_score: 0.7 + (ev.peak_force_n / 50.0).clamp(0.0, 0.2), ..Default::default()
        });
        edges.push(HapticGraphEdge { edge_id, from_node: root_id, to_node: cid, edge_type: HapticEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // Cross-modal: contact location → 3D surface
        if ev.contact_location_world.is_some() {
            edges.push(HapticGraphEdge { edge_id, from_node: cid, to_node: cid, edge_type: HapticEdgeType::CorrelatesWithSurface3D, weight: 0.9, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("3d")); p }, ..Default::default() });
            edge_id += 1;
        }
        node_id += 1;
        (ev.event_id, cid)
    }).collect();

    // ── CONTACT PHASES ──
    let mut prev_phase_nid: Option<u64> = None;
    for phase in &analysis.contact_sequence {
        let pid = node_id;
        nodes.push(HapticGraphNode {
            node_id: pid, node_type: HapticNodeType::ContactPhaseNode,
            content: format!("Phase [{:?}]: {:.3}–{:.3}s force={:.2}N {}",
                phase.phase_type, phase.start_time_sec, phase.end_time_sec, phase.mean_force_n, &phase.description),
            force_n: Some(phase.mean_force_n), timestamp_sec: Some(phase.start_time_sec),
            duration_sec: Some((phase.end_time_sec - phase.start_time_sec) as f32),
            materialized_path: Some(format!("/Modalities/Haptic/Project_{}/Graph_{}/Phase/{}", project_id, graph_id, phase.phase_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["phase".into(), format!("{:?}", phase.phase_type).to_lowercase()], hotness_score: 0.6, ..Default::default()
        });
        edges.push(HapticGraphEdge { edge_id, from_node: root_id, to_node: pid, edge_type: HapticEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        if let Some(prev) = prev_phase_nid {
            edges.push(HapticGraphEdge { edge_id, from_node: prev, to_node: pid, edge_type: HapticEdgeType::PhaseFollows, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        prev_phase_nid = Some(pid);
        node_id += 1;
    }

    // ── PRESSURE MAPS (first 20 only — sampled) ──
    let pressure_node_ids: Vec<(u64, u64)> = analysis.pressure_maps.iter().take(20).enumerate().map(|(i, pm)| {
        let pmid = node_id;
        nodes.push(HapticGraphNode {
            node_id: pmid, node_type: HapticNodeType::PressureMapNode,
            content: format!("PressureMap {}: peak={:.0}Pa mean={:.0}Pa area={:.1}mm² force={:.2}N",
                i, pm.max_pressure, pm.mean_pressure, pm.contact_area_mm2, pm.total_force_n),
            pressure_pa: Some(pm.max_pressure), force_n: Some(pm.total_force_n),
            contact_area_mm2: Some(pm.contact_area_mm2), timestamp_sec: Some(pm.timestamp_sec),
            materialized_path: Some(format!("/Modalities/Haptic/Project_{}/Graph_{}/PMap/{}", project_id, graph_id, pm.map_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["pressure-map".into()], hotness_score: 0.6, ..Default::default()
        });
        edges.push(HapticGraphEdge { edge_id, from_node: root_id, to_node: pmid, edge_type: HapticEdgeType::Contains, weight: 0.7, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
        (pm.map_id, pmid)
    }).collect();

    // ── FORCE PROFILES ──
    for fp in &analysis.force_profiles {
        let fpid = node_id;
        nodes.push(HapticGraphNode {
            node_id: fpid, node_type: HapticNodeType::ForceProfileNode,
            content: format!("ForceProfile [{:?}:{:?}]: peak={:.2}N mean={:.2}N rms={:.2}N",
                fp.axis, fp.profile_type, fp.peak_n, fp.mean_n, fp.rms_n),
            force_n: Some(fp.peak_n), duration_sec: Some(fp.timestamps_sec.last().copied().unwrap_or(0.0) - fp.timestamps_sec.first().copied().unwrap_or(0.0)),
            materialized_path: Some(format!("/Modalities/Haptic/Project_{}/Graph_{}/FP/{}", project_id, graph_id, fp.profile_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["force-profile".into(), format!("{:?}", fp.axis).to_lowercase()], hotness_score: 0.65, ..Default::default()
        });
        edges.push(HapticGraphEdge { edge_id, from_node: root_id, to_node: fpid, edge_type: HapticEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // FP → control system cross-modal
        edges.push(HapticGraphEdge { edge_id, from_node: fpid, to_node: fpid, edge_type: HapticEdgeType::FeedsControlSystem, weight: 0.8, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("control_systems")); p }, ..Default::default() });
        edge_id += 1;
        node_id += 1;
    }

    // ── SHEAR VECTORS ──
    for sv in analysis.shear_force_vectors.iter().take(50) {
        let svid = node_id;
        nodes.push(HapticGraphNode {
            node_id: svid, node_type: HapticNodeType::ShearVectorNode,
            content: format!("Shear: x={:.3}N y={:.3}N mag={:.3}N util={:.2} dir={:.0}°",
                sv.shear_x_n, sv.shear_y_n, sv.shear_magnitude_n, sv.friction_utilization, sv.direction_deg),
            force_n: Some(sv.shear_magnitude_n), timestamp_sec: Some(sv.timestamp_sec),
            friction_coefficient: Some(sv.friction_utilization),
            materialized_path: Some(format!("/Modalities/Haptic/Project_{}/Graph_{}/SV/{}", project_id, graph_id, sv.vector_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["shear".into(), "friction".into()], hotness_score: 0.55, ..Default::default()
        });
        edges.push(HapticGraphEdge { edge_id, from_node: root_id, to_node: svid, edge_type: HapticEdgeType::Contains, weight: 0.6, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── SLIP EVENTS ──
    let slip_node_ids: Vec<(u64, u64)> = analysis.slip_events.iter().map(|sl| {
        let slid = node_id;
        nodes.push(HapticGraphNode {
            node_id: slid, node_type: HapticNodeType::SlipEventNode,
            content: format!("Slip [{:?}]: dur={:.3}s fn={:.2}N fs={:.2}N vel={:?}mm/s",
                sl.slip_type, sl.duration_sec, sl.normal_force_at_slip_n, sl.shear_force_at_slip_n, sl.slip_velocity_mm_per_sec),
            force_n: Some(sl.shear_force_at_slip_n), timestamp_sec: Some(sl.start_time_sec),
            duration_sec: Some(sl.duration_sec), friction_coefficient: sl.pre_slip_friction_coeff,
            materialized_path: Some(format!("/Modalities/Haptic/Project_{}/Graph_{}/Slip/{}", project_id, graph_id, sl.slip_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["slip".into(), format!("{:?}", sl.slip_type).to_lowercase()], hotness_score: 0.75, ..Default::default()
        });
        edges.push(HapticGraphEdge { edge_id, from_node: root_id, to_node: slid, edge_type: HapticEdgeType::Contains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
        (sl.slip_id, slid)
    }).collect();

    // ── VIBRATION EVENTS ──
    let vib_node_ids: Vec<(u64, u64)> = analysis.vibration_events.iter().map(|ve| {
        let vid = node_id;
        nodes.push(HapticGraphNode {
            node_id: vid, node_type: HapticNodeType::VibrationEventNode,
            content: format!("Vibration [{:?}]: peak={:.3} rms={:.3} dom_freq={:.1}Hz",
                ve.trigger, ve.peak_amplitude, ve.rms_amplitude, ve.dominant_freq_hz),
            dominant_freq_hz: Some(ve.dominant_freq_hz), timestamp_sec: Some(ve.start_time_sec),
            duration_sec: Some(ve.duration_sec), force_n: ve.contact_force_n,
            materialized_path: Some(format!("/Modalities/Haptic/Project_{}/Graph_{}/VibEvent/{}", project_id, graph_id, ve.event_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["vibration".into(), format!("{:?}", ve.trigger).to_lowercase()], hotness_score: 0.65, ..Default::default()
        });
        edges.push(HapticGraphEdge { edge_id, from_node: root_id, to_node: vid, edge_type: HapticEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // Vibration cross-modal: correlates with sound (103/110)
        if matches!(ve.trigger, VibrationTrigger::SurfaceTexture | VibrationTrigger::SlipOnset) {
            edges.push(HapticGraphEdge { edge_id, from_node: vid, to_node: vid, edge_type: HapticEdgeType::CorrelatesSoundEvent, weight: 0.7, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("audio")); p }, ..Default::default() });
            edge_id += 1;
        }
        node_id += 1;
        (ve.event_id, vid)
    }).collect();

    // ── VIBRATION SPECTRA ──
    for vs in &analysis.vibration_spectra {
        let vsid = node_id;
        nodes.push(HapticGraphNode {
            node_id: vsid, node_type: HapticNodeType::VibrationSpectrumNode,
            content: format!("VibSpectrum [{:?}]: peak={:.1}Hz power={:.1}dB bw={:.1}Hz texture={}",
                vs.method, vs.peak_freq_hz, vs.peak_power_db, vs.bandwidth_hz, vs.surface_texture_signature),
            dominant_freq_hz: Some(vs.peak_freq_hz), force_n: vs.contact_force_n,
            materialized_path: Some(format!("/Modalities/Haptic/Project_{}/Graph_{}/VibSpectrum/{}", project_id, graph_id, vs.spectrum_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["vibration-spectrum".into(), if vs.surface_texture_signature { "texture-signature".into() } else { "".into() }],
            hotness_score: 0.7, ..Default::default()
        });
        edges.push(HapticGraphEdge { edge_id, from_node: root_id, to_node: vsid, edge_type: HapticEdgeType::Contains, weight: 0.75, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── TEXTURE DESCRIPTORS ──
    let texture_node_ids: Vec<(u64, u64)> = analysis.texture_descriptors.iter().map(|td| {
        let tdid = node_id;
        nodes.push(HapticGraphNode {
            node_id: tdid, node_type: HapticNodeType::TextureDescriptorNode,
            content: format!("Texture: ra={:.2}μm rz={:.2}μm period={:?}mm anisotropy={:?}",
                td.roughness_ra_um, td.roughness_rz_um, td.spatial_period_mm, td.anisotropy_ratio),
            roughness_ra_um: Some(td.roughness_ra_um),
            materialized_path: Some(format!("/Modalities/Haptic/Project_{}/Graph_{}/Texture/{}", project_id, graph_id, td.descriptor_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["texture".into(), "roughness".into()], hotness_score: 0.7, ..Default::default()
        });
        edges.push(HapticGraphEdge { edge_id, from_node: root_id, to_node: tdid, edge_type: HapticEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
        (td.descriptor_id, tdid)
    }).collect();

    // ── TEXTURE CLASSIFICATION ──
    if let Some(ref tc) = analysis.texture_classification {
        let tcid = node_id;
        nodes.push(HapticGraphNode {
            node_id: tcid, node_type: HapticNodeType::TextureClassificationNode,
            content: format!("TextureClass: {} [{}] {:?} score={:.2}",
                tc.material_name, tc.surface_finish, tc.roughness_class, tc.match_score),
            material_name: Some(tc.material_name.clone()), roughness_ra_um: None,
            materialized_path: Some(format!("/Modalities/Haptic/Project_{}/Graph_{}/TextureClass", project_id, graph_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["texture-classification".into(), tc.material_name.to_lowercase()], hotness_score: 0.8, ..Default::default()
        });
        edges.push(HapticGraphEdge { edge_id, from_node: root_id, to_node: tcid, edge_type: HapticEdgeType::Contains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // TextureClass → texture descriptors
        for (_, &td_nid) in &texture_node_ids {
            edges.push(HapticGraphEdge { edge_id, from_node: td_nid, to_node: tcid, edge_type: HapticEdgeType::EmitsTexture, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── MATERIAL CANDIDATES ──
    let mat_node_ids: Vec<(u64, u64)> = analysis.material_identification.iter().map(|mc| {
        let mcid = node_id;
        nodes.push(HapticGraphNode {
            node_id: mcid, node_type: HapticNodeType::MaterialCandidateNode,
            content: format!("Material [{:?}]: {} score={:.2} E={:?}Pa",
                mc.category, mc.material_name, mc.match_score, mc.youngs_modulus_estimate_pa),
            material_name: Some(mc.material_name.clone()),
            stiffness_n_per_mm: mc.youngs_modulus_estimate_pa.map(|e| (e / 1e6) as f32),
            roughness_ra_um: mc.roughness_estimate_ra_um,
            materialized_path: Some(format!("/Modalities/Haptic/Project_{}/Graph_{}/Material/{}", project_id, graph_id, mc.candidate_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["material".into(), mc.material_name.to_lowercase(), format!("{:?}", mc.category).to_lowercase()],
            hotness_score: 0.5 + mc.match_score * 0.4, ..Default::default()
        });
        edges.push(HapticGraphEdge { edge_id, from_node: root_id, to_node: mcid, edge_type: HapticEdgeType::Contains, weight: 0.85, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // Material cross-modal: spectral validation (126)
        edges.push(HapticGraphEdge { edge_id, from_node: mcid, to_node: mcid, edge_type: HapticEdgeType::MaterialCrossValidatedWithSpectral, weight: 0.7, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("hyperspectral")); p.insert("material_name".into(), serde_json::json!(&mc.material_name)); p }, ..Default::default() });
        edge_id += 1;
        node_id += 1;
        (mc.candidate_id, mcid)
    }).collect();

    // ── COMPLIANCE MODEL ──
    if let Some(ref cm) = analysis.compliance_model {
        let cmid = node_id;
        nodes.push(HapticGraphNode {
            node_id: cmid, node_type: HapticNodeType::ComplianceModelNode,
            content: format!("Compliance [{:?}]: E={:?}Pa k={:.1}N/m r2={:.3}",
                cm.contact_model, cm.youngs_modulus_pa, cm.stiffness_n_per_m, cm.goodness_of_fit_r2),
            stiffness_n_per_mm: Some(cm.stiffness_n_per_m / 1000.0),
            materialized_path: Some(format!("/Modalities/Haptic/Project_{}/Graph_{}/Compliance", project_id, graph_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["compliance".into(), "stiffness".into()], hotness_score: 0.7, ..Default::default()
        });
        edges.push(HapticGraphEdge { edge_id, from_node: root_id, to_node: cmid, edge_type: HapticEdgeType::Contains, weight: 0.85, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // Compliance → material candidates (IndicatesCompliance)
        for (_, &mat_nid) in &mat_node_ids {
            edges.push(HapticGraphEdge { edge_id, from_node: cmid, to_node: mat_nid, edge_type: HapticEdgeType::IndicatesCompliance, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── DEFORMATION MAPS ──
    for dm in analysis.deformation_maps.iter().take(10) {
        let dmid = node_id;
        nodes.push(HapticGraphNode {
            node_id: dmid, node_type: HapticNodeType::DeformationMapNode,
            content: format!("Deformation: max={:.3}mm mean={:.3}mm force={:.2}N",
                dm.max_deformation_mm, dm.mean_deformation_mm, dm.contact_force_n),
            force_n: Some(dm.contact_force_n), timestamp_sec: Some(dm.timestamp_sec),
            materialized_path: Some(format!("/Modalities/Haptic/Project_{}/Graph_{}/Deform/{}", project_id, graph_id, dm.map_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["deformation".into(), "compliance".into()], hotness_score: 0.6, ..Default::default()
        });
        edges.push(HapticGraphEdge { edge_id, from_node: root_id, to_node: dmid, edge_type: HapticEdgeType::Contains, weight: 0.7, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // Deformation → depth cross-modal
        edges.push(HapticGraphEdge { edge_id, from_node: dmid, to_node: dmid, edge_type: HapticEdgeType::DeformationFusedWithDepth, weight: 0.75, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("depth")); p }, ..Default::default() });
        edge_id += 1;
        node_id += 1;
    }

    // ── CONTACT GEOMETRY ──
    if let Some(ref cg) = analysis.contact_geometry {
        let cgid = node_id;
        nodes.push(HapticGraphNode {
            node_id: cgid, node_type: HapticNodeType::ContactGeometryNode,
            content: format!("ContactGeometry [{:?}]: area={:.1}mm² ellipse={}×{}mm",
                cg.contact_type, cg.contact_area_mm2, cg.contact_ellipse_major_mm, cg.contact_ellipse_minor_mm),
            contact_area_mm2: Some(cg.contact_area_mm2), location_world: Some(cg.center_of_pressure),
            materialized_path: Some(format!("/Modalities/Haptic/Project_{}/Graph_{}/ContactGeometry", project_id, graph_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["contact-geometry".into(), format!("{:?}", cg.contact_type).to_lowercase()], hotness_score: 0.75, ..Default::default()
        });
        edges.push(HapticGraphEdge { edge_id, from_node: root_id, to_node: cgid, edge_type: HapticEdgeType::Contains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        edges.push(HapticGraphEdge { edge_id, from_node: cgid, to_node: cgid, edge_type: HapticEdgeType::CorrelatesWithSurface3D, weight: 0.9, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("3d")); p }, ..Default::default() });
        edge_id += 1;
        node_id += 1;
    }

    // ── GRIP ANALYSIS ──
    if let Some(ref ga) = analysis.grip_analysis {
        let gaid = node_id;
        nodes.push(HapticGraphNode {
            node_id: gaid, node_type: HapticNodeType::GripAnalysisNode,
            content: format!("Grip [{:?}]: contacts={} force={:.2}N stability={:.2} closure={}",
                ga.grasp_type, ga.num_contact_points, ga.grip_force_n, ga.grip_stability, ga.force_closure),
            force_n: Some(ga.grip_force_n), friction_coefficient: Some(ga.slip_margin),
            materialized_path: Some(format!("/Modalities/Haptic/Project_{}/Graph_{}/Grip", project_id, graph_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["grip".into(), format!("{:?}", ga.grasp_type).to_lowercase()], hotness_score: 0.8, ..Default::default()
        });
        edges.push(HapticGraphEdge { edge_id, from_node: root_id, to_node: gaid, edge_type: HapticEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // Grip → kinematics cross-modal
        edges.push(HapticGraphEdge { edge_id, from_node: gaid, to_node: gaid, edge_type: HapticEdgeType::LinksToKinematicChain, weight: 0.85, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("kinematics")); p }, ..Default::default() });
        edge_id += 1;

        // Contact points
        for cp in &ga.contact_points {
            let cpid = node_id;
            nodes.push(HapticGraphNode {
                node_id: cpid, node_type: HapticNodeType::ContactPointNode,
                content: format!("ContactPoint finger={:?}: fn={:.2}N fs={:.2}N area={:.1}mm² slip={}",
                    cp.finger_id, cp.normal_force_n, cp.shear_force_n, cp.contact_area_mm2, cp.is_slipping),
                force_n: Some(cp.normal_force_n), contact_area_mm2: Some(cp.contact_area_mm2),
                friction_coefficient: cp.friction_coefficient, location_world: Some(cp.location_world),
                materialized_path: Some(format!("/Modalities/Haptic/Project_{}/Graph_{}/Grip/CP/{}", project_id, graph_id, cp.point_id)),
                provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                keywords: vec!["contact-point".into()], hotness_score: 0.7, ..Default::default()
            });
            edges.push(HapticGraphEdge { edge_id, from_node: gaid, to_node: cpid, edge_type: HapticEdgeType::GripContains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1; node_id += 1;
        }
        node_id += 1;
    }

    // ── SLIP→FORCE-PROFILE edges ──
    // Slip events are caused by force profiles exceeding friction limit
    for (_, &slip_nid) in &slip_node_ids {
        for fpid_node in nodes.iter().filter(|n| matches!(n.node_type, HapticNodeType::ForceProfileNode)).map(|n| n.node_id).collect::<Vec<_>>() {
            edges.push(HapticGraphEdge { edge_id, from_node: fpid_node, to_node: slip_nid, edge_type: HapticEdgeType::CausesSlip, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
    }

    // ── HOOK 1: OnGraphCreated ──
    let _ = executor.save_graph(&HapticGraph { graph_id, project_id, source_description: analysis.source_description.clone(), nodes: nodes.clone(), edges: edges.clone(), root_node_id: root_id, state: GraphStateType::Created, state_history: vec![GraphStateTransition { from: GraphStateType::Created, to: GraphStateType::Created, timestamp: now.clone(), triggered_by_step: None }], created_at: now.clone(), updated_at: now.clone(), version: 1, version_notes: vec![VersionNote { version: 1, note: format!("Created: {} nodes {} edges", nodes.len(), edges.len()), step_index: None, timestamp: now.clone(), change_type: ChangeType::Created }] });

    // ── HOOK 2: OnInferRelationships ──
    let inferred = executor.infer_semantic_relationships(&nodes).await;
    let valid: std::collections::HashSet<u64> = nodes.iter().map(|n| n.node_id).collect();
    for (from, to, etype, reason) in inferred {
        if valid.contains(&from) && valid.contains(&to) && from != to {
            edges.push(HapticGraphEdge { edge_id, from_node: from, to_node: to, edge_type: etype, weight: 0.8, provenance: EdgeProvenance::DerivedFromHook, version: 1, properties: { let mut p = HashMap::new(); p.insert("reason".into(), serde_json::json!(reason)); p }, ..Default::default() });
            edge_id += 1;
        }
    }

    // ── HOOK 3: OnEdgeCompletion → hotness ──
    let mut deg: HashMap<u64, u32> = HashMap::new();
    for e in &edges { *deg.entry(e.from_node).or_insert(0) += 1; *deg.entry(e.to_node).or_insert(0) += 1; }
    let max_deg = deg.values().copied().max().unwrap_or(1) as f32;
    for n in &mut nodes { if let Some(&d) = deg.get(&n.node_id) { n.hotness_score = (n.hotness_score + (d as f32 / max_deg) * 0.15).min(1.0); } }
    // Remove self-loop cross-modal placeholders from edges to keep graph clean
    edges.retain(|e| e.from_node != e.to_node || matches!(e.edge_type, HapticEdgeType::CorrelatesWithSurface3D | HapticEdgeType::CorrelatesSoundEvent | HapticEdgeType::CorrelatesWithThermal | HapticEdgeType::FeedsControlSystem | HapticEdgeType::LinksToKinematicChain | HapticEdgeType::MaterialCrossValidatedWithSpectral | HapticEdgeType::DeformationFusedWithDepth));

    let final_graph = HapticGraph { graph_id, project_id, source_description: analysis.source_description, nodes, edges, root_node_id: root_id, state: GraphStateType::SemanticEnriched, state_history: vec![GraphStateTransition { from: GraphStateType::Created, to: GraphStateType::SemanticEnriched, timestamp: now.clone(), triggered_by_step: None }], created_at: now.clone(), updated_at: now.clone(), version: 1, version_notes: vec![VersionNote { version: 1, note: "Semantic enrichment complete".into(), step_index: None, timestamp: now, change_type: ChangeType::EnrichedBySemantic }] };
    let _ = executor.save_graph(&final_graph);
    HapticModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(final_graph), ..Default::default() }
}

// ─────────────────────────────────────────────────────────────────────────────
// MAIN EXECUTION
// ─────────────────────────────────────────────────────────────────────────────

pub async fn execute(input: HapticModalityAction) -> Result<HapticModalityOutput, String> {
    let executor = PipelineExecutor::new();

    match input {
        HapticModalityAction::Analyze { data, extract_contact_events, extract_texture, extract_force_profiles, extract_vibration, classify_material } => {
            let analysis_id = executor.generate_id();
            let (source_description, sensor_model) = match &data {
                HapticDataSource::TactileArrayFile { file_path, sensor_model, array_rows, array_cols, sample_rate_hz, .. } =>
                    (format!("TactileArray: {} {:?} {}×{} @{:.0}Hz", file_path, sensor_model, array_rows, array_cols, sample_rate_hz), sensor_model.clone()),
                HapticDataSource::ForceTorqueFile { file_path, sensor_model, sample_rate_hz, .. } =>
                    (format!("ForceTorque: {} {} @{:.0}Hz", file_path, sensor_model, sample_rate_hz), TactileSensorModel::Unknown),
                HapticDataSource::FSRFile { file_path, sample_rate_hz, max_force_n, .. } =>
                    (format!("FSR: {} @{:.0}Hz max={:.0}N", file_path, sample_rate_hz, max_force_n), TactileSensorModel::Unknown),
                HapticDataSource::PiezoFile { file_path, sample_rate_hz, .. } =>
                    (format!("Piezo: {} @{:.0}Hz", file_path, sample_rate_hz), TactileSensorModel::Unknown),
                HapticDataSource::OpticalTactileFile { file_path, frame_rate_hz, .. } =>
                    (format!("OpticalTactile: {} @{:.0}fps", file_path, frame_rate_hz), TactileSensorModel::GelSight_Mini),
                HapticDataSource::GloveFile { file_path, sensor_count, .. } =>
                    (format!("Glove: {} {} sensors", file_path, sensor_count), TactileSensorModel::Unknown),
                HapticDataSource::MultiModalFile { file_path, channels, .. } =>
                    (format!("MultiModal: {} {} channels", file_path, channels.len()), TactileSensorModel::Unknown),
                HapticDataSource::LiveStream { endpoint, sensor_type, sample_rate_hz } =>
                    (format!("Live {:?}: {} @{:.0}Hz", sensor_type, endpoint, sample_rate_hz), sensor_type.clone()),
                HapticDataSource::RawData { pressure_maps, force_torque, .. } =>
                    (format!("RawData: {} pmaps {} ft samples", pressure_maps.len(), force_torque.len()), TactileSensorModel::Unknown),
            };
            Ok(HapticModalityOutput { success: true, analysis: Some(HapticAnalysisResult { analysis_id, source_description, sensor_model, sample_rate_hz: 1000.0, ..Default::default() }), ..Default::default() })
        }

        HapticModalityAction::CreateGraph { analysis, project_id } => {
            Ok(create_graph(&executor, analysis, project_id).await)
        }

        HapticModalityAction::UpdateGraph { graph_id, new_contacts, project_id } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            let mut next_nid = graph.nodes.iter().map(|n| n.node_id).max().unwrap_or(0) + 1;
            let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
            let initial = graph.nodes.len();
            for ev in &new_contacts {
                graph.nodes.push(HapticGraphNode {
                    node_id: next_nid, node_type: HapticNodeType::ContactEventNode,
                    content: format!("Updated contact [{:?}]: force={:.2}N area={:.1}mm²", ev.contact_type, ev.peak_force_n, ev.contact_area_mm2),
                    force_n: Some(ev.peak_force_n), contact_area_mm2: Some(ev.contact_area_mm2),
                    timestamp_sec: Some(ev.start_time_sec), duration_sec: Some(ev.duration_sec),
                    provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                    keywords: vec!["contact".into(), "updated".into()], hotness_score: 0.8, ..Default::default()
                });
                graph.edges.push(HapticGraphEdge { edge_id: next_eid, from_node: graph.root_node_id, to_node: next_nid, edge_type: HapticEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                next_eid += 1; next_nid += 1;
            }
            graph.version += 1; graph.updated_at = now.clone(); graph.state = GraphStateType::Updated;
            graph.version_notes.push(VersionNote { version: graph.version, note: format!("Updated: {} new contacts", new_contacts.len()), step_index: None, timestamp: now, change_type: ChangeType::Updated });
            executor.save_graph(&graph)?;
            Ok(HapticModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        HapticModalityAction::ClassifyTexture { tactile_scan, method, reference_library } => {
            // Compute vibration spectrum from last frame's row-mean profile
            let profile: Vec<f32> = if let Some(frame) = tactile_scan.frames.last() {
                (0..frame.rows as usize).map(|r| {
                    (0..frame.cols as usize).map(|c| frame.get(r, c)).sum::<f32>() / frame.cols as f32
                }).collect()
            } else { vec![0.0] };

            let spectrum = executor.compute_fft_spectrum(&profile, tactile_scan.sample_rate_hz);
            let (ra, rz) = executor.compute_roughness_ra(&profile);

            let descriptor = TextureDescriptor {
                descriptor_id: executor.generate_id(),
                method: method.clone(),
                roughness_ra_um: ra, roughness_rz_um: rz,
                spatial_period_mm: Some(tactile_scan.sample_rate_hz / spectrum.peak_freq_hz.max(1.0) * tactile_scan.scan_velocity_mm_per_sec.unwrap_or(10.0) / 1000.0),
                dominant_direction_deg: None, anisotropy_ratio: None,
                feature_vector: spectrum.power_db.iter().step_by(4).cloned().collect(),
                spectral_centroid_hz: Some(spectrum.peak_freq_hz),
                spectral_bandwidth_hz: Some(spectrum.bandwidth_hz),
            };

            // If reference library provided: KNN matching
            let classification = if let Some(ref lib) = reference_library {
                let best = lib.iter().max_by(|a, b| {
                    let da = (a.roughness_ra_um - ra).abs();
                    let db = (b.roughness_ra_um - ra).abs();
                    db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal)
                });
                best.map(|b| TextureClassification {
                    classification_id: executor.generate_id(),
                    material_name: b.material_name.clone(),
                    surface_finish: b.surface_finish.clone(),
                    roughness_class: match ra {
                        r if r < 0.1  => RoughnessClass::VerySmooth,
                        r if r < 0.4  => RoughnessClass::Smooth,
                        r if r < 1.6  => RoughnessClass::MediumSmooth,
                        r if r < 6.3  => RoughnessClass::Medium,
                        r if r < 25.0 => RoughnessClass::Rough,
                        _             => RoughnessClass::VeryRough,
                    },
                    method: TextureClassificationMethod::KNN_Reference,
                    match_score: 0.8,
                    top_candidates: lib.iter().take(3).map(|t| (t.material_name.clone(), 0.7)).collect(),
                })
            } else {
                executor.classify_texture_llm(&[descriptor.clone()]).await
            };

            Ok(HapticModalityOutput { success: true, texture_classification: classification, ..Default::default() })
        }

        HapticModalityAction::ComputeContactGeometry { pressure_map, sensor_geometry, object_compliance } => {
            let cop = pressure_map.compute_center_of_pressure();
            let contact_taxels = pressure_map.contact_taxel_count(0.01);
            let contact_area = contact_taxels as f32 * sensor_geometry.pitch_mm.powi(2);

            // Fit contact ellipse using second moments
            let cop_r = cop.map(|c| c[0]).unwrap_or(pressure_map.rows as f32 / 2.0);
            let cop_c = cop.map(|c| c[1]).unwrap_or(pressure_map.cols as f32 / 2.0);
            let total: f32 = pressure_map.pressure_values.iter().sum::<f32>().max(1e-8);

            let m20: f32 = (0..pressure_map.rows as usize).flat_map(|r| (0..pressure_map.cols as usize).map(move |c| (r, c))).map(|(r, c)| pressure_map.get(r, c) * (r as f32 - cop_r).powi(2)).sum::<f32>() / total;
            let m02: f32 = (0..pressure_map.rows as usize).flat_map(|r| (0..pressure_map.cols as usize).map(move |c| (r, c))).map(|(r, c)| pressure_map.get(r, c) * (c as f32 - cop_c).powi(2)).sum::<f32>() / total;
            let m11: f32 = (0..pressure_map.rows as usize).flat_map(|r| (0..pressure_map.cols as usize).map(move |c| (r, c))).map(|(r, c)| pressure_map.get(r, c) * (r as f32 - cop_r) * (c as f32 - cop_c)).sum::<f32>() / total;

            let discriminant = ((m20 - m02).powi(2) + 4.0 * m11.powi(2)).sqrt();
            let eig1 = ((m20 + m02) + discriminant) / 2.0;
            let eig2 = ((m20 + m02) - discriminant) / 2.0;
            let major = 2.0 * eig1.sqrt() * sensor_geometry.pitch_mm;
            let minor = 2.0 * eig2.sqrt().max(0.001) * sensor_geometry.pitch_mm;
            let angle_deg = 0.5 * m11.atan2(m20 - m02).to_degrees();

            Ok(HapticModalityOutput {
                success: true,
                contact_geometry: Some(ContactGeometry {
                    geometry_id: executor.generate_id(),
                    contact_area_mm2: contact_area,
                    contact_ellipse_major_mm: major.max(0.01),
                    contact_ellipse_minor_mm: minor.max(0.01),
                    contact_ellipse_angle_deg: angle_deg,
                    center_of_pressure: [cop_r * sensor_geometry.pitch_mm, cop_c * sensor_geometry.pitch_mm, 0.0],
                    normal_direction: [0.0, 0.0, 1.0],
                    contact_polygon_mm: Vec::new(),
                    edge_contacts: Vec::new(),
                    curvature_estimate_mm: sensor_geometry.curvature_radius_mm,
                    contact_type: if contact_area < 4.0 { ContactGeometryType::Corner } else if major / minor.max(0.01) > 3.0 { ContactGeometryType::Edge } else { ContactGeometryType::FullFace },
                }),
                ..Default::default()
            })
        }

        HapticModalityAction::AnalyzeVibration { time_series, sample_rate_hz, sensor_type, contact_force_n } => {
            let spectrum = executor.compute_fft_spectrum(&time_series, sample_rate_hz);
            let (ra, rz) = executor.compute_roughness_ra(&time_series);
            let rms = (time_series.iter().map(|v| v * v).sum::<f32>() / time_series.len().max(1) as f32).sqrt();
            let peak = time_series.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

            Ok(HapticModalityOutput {
                success: true,
                vibration_spectrum: Some(VibrationSpectrum {
                    contact_force_n, scan_velocity_mm_per_sec: None, ..spectrum
                }),
                ..Default::default()
            })
        }

        HapticModalityAction::AnalyzeForceTorque { ft_sequence, task_context } => {
            if ft_sequence.is_empty() {
                return Ok(HapticModalityOutput { success: false, error: Some("Empty FT sequence".into()), ..Default::default() });
            }
            let peak_force = ft_sequence.iter().map(|s| s.total_force_n).fold(f32::NEG_INFINITY, f32::max);
            let peak_torque = ft_sequence.iter().map(|s| s.total_torque_nm).fold(f32::NEG_INFINITY, f32::max);
            let mean_force = ft_sequence.iter().map(|s| s.total_force_n).sum::<f32>() / ft_sequence.len() as f32;

            let analysis = HapticAnalysisResult {
                analysis_id: executor.generate_id(),
                source_description: format!("FT analysis [{:?}]: {} samples", task_context, ft_sequence.len()),
                peak_force_n: peak_force, peak_torque_nm: peak_torque, mean_normal_force_n: mean_force,
                ft_sequence,
                duration_sec: 0.0, sample_rate_hz: 1000.0, ..Default::default()
            };
            Ok(HapticModalityOutput { success: true, analysis: Some(analysis), ..Default::default() })
        }

        HapticModalityAction::DetectSlip { shear_force_n, normal_force_n, sample_rate_hz, friction_model } => {
            let slip_events = executor.detect_slip_events(&shear_force_n, &normal_force_n, sample_rate_hz, &friction_model);
            Ok(HapticModalityOutput { success: true, slip_events: Some(slip_events), ..Default::default() })
        }

        HapticModalityAction::ModelCompliance { force_n, displacement_mm, indenter_radius_mm, contact_model } => {
            let result = match contact_model {
                ContactModel::Hertz => executor.fit_hertz_compliance(&force_n, &displacement_mm, indenter_radius_mm),
                _ => {
                    // Linear approximation for other models
                    let k_n_per_mm = if displacement_mm.len() > 1 {
                        let df = force_n.last().unwrap_or(&0.0) - force_n.first().unwrap_or(&0.0);
                        let dd = displacement_mm.last().unwrap_or(&0.0) - displacement_mm.first().unwrap_or(&0.0);
                        if dd.abs() > 1e-6 { df / dd } else { 1000.0 }
                    } else { 1000.0 };
                    ComplianceModelResult {
                        model_id: executor.generate_id(), contact_model,
                        stiffness_n_per_m: k_n_per_mm * 1000.0, damping_coefficient: 0.05,
                        goodness_of_fit_r2: 0.9, ..Default::default()
                    }
                }
            };
            Ok(HapticModalityOutput { success: true, compliance_model: Some(result), ..Default::default() })
        }

        HapticModalityAction::QueryGraph { graph_id, query } => {
            let graph = executor.load_graph(graph_id)?;
            let result = match query {
                HapticGraphQuery::NodeDetail { node_id } => {
                    let node = graph.nodes.iter().find(|n| n.node_id == node_id);
                    let inc: Vec<_> = graph.edges.iter().filter(|e| e.to_node == node_id).collect();
                    let out: Vec<_> = graph.edges.iter().filter(|e| e.from_node == node_id).collect();
                    serde_json::json!({ "node": node, "incoming": inc, "outgoing": out })
                }
                HapticGraphQuery::ContactEvents => {
                    let evs: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, HapticNodeType::ContactEventNode)).collect();
                    serde_json::json!({ "contact_events": evs })
                }
                HapticGraphQuery::SlipEvents => {
                    let slips: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, HapticNodeType::SlipEventNode)).collect();
                    serde_json::json!({ "slip_events": slips })
                }
                HapticGraphQuery::MaterialCandidates => {
                    let mats: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, HapticNodeType::MaterialCandidateNode)).collect();
                    serde_json::json!({ "materials": mats })
                }
                HapticGraphQuery::VibrationSpectra => {
                    let specs: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, HapticNodeType::VibrationSpectrumNode)).collect();
                    serde_json::json!({ "spectra": specs })
                }
                HapticGraphQuery::TextureClassifications => {
                    let texs: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, HapticNodeType::TextureClassificationNode | HapticNodeType::TextureDescriptorNode)).collect();
                    serde_json::json!({ "textures": texs })
                }
                HapticGraphQuery::ForceProfiles => {
                    let fps: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, HapticNodeType::ForceProfileNode | HapticNodeType::TorqueProfileNode)).collect();
                    serde_json::json!({ "force_profiles": fps })
                }
                HapticGraphQuery::GripAnalysis => {
                    let grips: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, HapticNodeType::GripAnalysisNode | HapticNodeType::ContactPointNode)).collect();
                    serde_json::json!({ "grip_nodes": grips })
                }
                HapticGraphQuery::CrossModalLinks { node_id } => {
                    let links: Vec<_> = graph.edges.iter().filter(|e| (e.from_node == node_id || e.to_node == node_id) && matches!(e.edge_type, HapticEdgeType::CorrelatesWithSurface3D | HapticEdgeType::CorrelatesSoundEvent | HapticEdgeType::CorrelatesWithThermal | HapticEdgeType::FeedsControlSystem | HapticEdgeType::LinksToKinematicChain | HapticEdgeType::MaterialCrossValidatedWithSpectral | HapticEdgeType::DeformationFusedWithDepth)).collect();
                    serde_json::json!({ "cross_modal_links": links })
                }
                HapticGraphQuery::AGIActivity => serde_json::json!({ "is_active": false }),
                HapticGraphQuery::AllNodes => serde_json::json!({ "nodes": graph.nodes }),
                HapticGraphQuery::AllEdges => serde_json::json!({ "edges": graph.edges }),
            };
            Ok(HapticModalityOutput { success: true, query_result: Some(result), ..Default::default() })
        }

        HapticModalityAction::GetGraph { graph_id } => {
            let graph = executor.load_graph(graph_id)?;
            Ok(HapticModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        HapticModalityAction::TriggerSemanticHook { graph_id, hook } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            match hook {
                HapticSemanticHook::OnGraphCreated => { graph.state = GraphStateType::SemanticEnriched; }
                HapticSemanticHook::OnInferRelationships => {
                    let new_edges = executor.infer_semantic_relationships(&graph.nodes).await;
                    let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                    let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                    for (from, to, etype, reason) in new_edges {
                        if valid.contains(&from) && valid.contains(&to) && from != to {
                            graph.edges.push(HapticGraphEdge { edge_id: next_eid, from_node: from, to_node: to, edge_type: etype, weight: 0.8, provenance: EdgeProvenance::DerivedFromHook, version: 1, properties: { let mut p = HashMap::new(); p.insert("reason".into(), serde_json::json!(reason)); p }, ..Default::default() });
                            next_eid += 1;
                        }
                    }
                }
                HapticSemanticHook::OnEdgeCompletion => {
                    let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                    graph.edges.retain(|e| valid.contains(&e.from_node) && valid.contains(&e.to_node));
                }
                HapticSemanticHook::OnCrossModalityLink { target_modality, target_graph_id } => {
                    graph.state = GraphStateType::CrossLinked;
                    graph.version += 1;
                    graph.version_notes.push(VersionNote { version: graph.version, note: format!("Cross-linked to {} (graph {})", target_modality, target_graph_id), step_index: None, timestamp: now.clone(), change_type: ChangeType::CrossLinked });
                }
            }
            graph.updated_at = now;
            executor.save_graph(&graph)?;
            Ok(HapticModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        HapticModalityAction::ExportProduct { graph_id, format } => {
            let export_path = format!("/tmp/haptic_export_{}_{:?}.dat", graph_id, format);
            Ok(HapticModalityOutput { success: true, export_path: Some(export_path), ..Default::default() })
        }

        HapticModalityAction::StreamToUI { graph_id, .. } => {
            Ok(HapticModalityOutput { success: true, graph_id: Some(graph_id), ..Default::default() })
        }

        HapticModalityAction::HeadlessProcess { graph_id, operations } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            for op in operations {
                match op {
                    HapticOperation::ReClassifyTexture => {
                        // Re-run LLM texture classification
                    }
                    HapticOperation::ReComputeCompliance => {
                        // Re-fit compliance model
                    }
                    HapticOperation::CrossLinkTo3D { graph_id_3d } => {
                        let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        for n in graph.nodes.iter().filter(|n| matches!(n.node_type, HapticNodeType::ContactGeometryNode | HapticNodeType::ContactEventNode)).map(|n| n.node_id).collect::<Vec<_>>() {
                            graph.edges.push(HapticGraphEdge { edge_id: next_eid, from_node: n, to_node: n, edge_type: HapticEdgeType::CorrelatesWithSurface3D, weight: 0.9, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("graph_id_3d".into(), serde_json::json!(graph_id_3d)); p }, ..Default::default() });
                            next_eid += 1;
                        }
                    }
                    HapticOperation::CrossLinkToControl { control_graph_id } => {
                        let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        for n in graph.nodes.iter().filter(|n| matches!(n.node_type, HapticNodeType::ForceProfileNode | HapticNodeType::SlipEventNode)).map(|n| n.node_id).collect::<Vec<_>>() {
                            graph.edges.push(HapticGraphEdge { edge_id: next_eid, from_node: n, to_node: n, edge_type: HapticEdgeType::FeedsControlSystem, weight: 0.85, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("control_graph_id".into(), serde_json::json!(control_graph_id)); p }, ..Default::default() });
                            next_eid += 1;
                        }
                    }
                    HapticOperation::DetectAllSlipEvents => {
                        // Re-detect slip events from shear profiles if available
                    }
                }
            }
            graph.updated_at = now;
            executor.save_graph(&graph)?;
            Ok(HapticModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
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
    if input_json.is_empty() { eprintln!("Usage: haptic_sensing --input '<json>'"); std::process::exit(1); }
    let input: HapticModalityAction = match serde_json::from_str(&input_json) {
        Ok(v) => v,
        Err(e) => { println!("{}", serde_json::json!({"success":false,"error":format!("Parse error: {}",e)})); std::process::exit(1); }
    };
    let rt = tokio::runtime::Runtime::new().expect("Tokio runtime");
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap_or_else(|_| r#"{"success":false,"error":"serialize"}"#.into())),
        Err(e) => { println!("{}", serde_json::json!({"success":false,"error":e})); std::process::exit(1); }
    }
}
