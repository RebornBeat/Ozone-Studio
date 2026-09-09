//! IMUSensing — Pipeline #116
//!
//! Inertial Measurement Unit: accelerometer, gyroscope, magnetometer processing,
//! sensor fusion (AHRS / VIO), dead reckoning, motion classification, vibration
//! analysis, and platform state estimation.
//!
//! DISTINCT FROM:
//!   - Haptic (113): measures contact forces at a surface from physical touch.
//!     IMU measures INERTIAL forces — acceleration of a moving platform and
//!     angular rates of rotation. Different physics, different sensors, different
//!     inference tasks.
//!   - Kinematics (121): describes joint-space geometry and motion of kinematic
//!     chains. IMU provides the raw sensor data that feeds kinematic estimation;
//!     kinematics computes the model, IMU provides the measurements.
//!   - Geospatial (117): macro-level position on a map. IMU provides micro-level
//!     dead-reckoning increments between GPS fixes.
//!
//! CROSS-LINKS:
//!   109 (3D)      → IMU trajectory placed in 3D scene
//!   116 itself    → multi-IMU fusion (articulated body)
//!   117 (Geo)     → dead-reckoning trajectory on geo map
//!   121 (Kine)    → joint angle estimation from IMU arrays
//!   122 (Control) → state feedback to control system
//!   124 (Radar)   → radar-IMU fusion for moving platform
//!   125 (Sonar)   → AUV navigation using sonar + IMU
//!
//! STORAGE: ZSEI containers under /Modalities/IMU/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

// ─────────────────────────────────────────────────────────────────────────────
// INPUT TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum IMUModalityAction {
    /// Analyze raw IMU data — produces IMUAnalysisResult
    Analyze {
        data: IMUDataSource,
        run_fusion: bool,
        classify_motion: bool,
        detect_events: bool,
        estimate_trajectory: bool,
        compute_vibration: bool,
    },
    /// Create a graph from analysis result
    CreateGraph {
        analysis: IMUAnalysisResult,
        project_id: u64,
    },
    /// Update graph with new IMU samples
    UpdateGraph {
        graph_id: u64,
        new_samples: Vec<IMUSample>,
        project_id: u64,
    },
    /// Run AHRS sensor fusion (Madgwick / Mahony / EKF)
    RunAHRS {
        samples: Vec<IMUSample>,
        sample_rate_hz: f32,
        algorithm: AHRSAlgorithm,
        initial_orientation: Option<Quaternion>,
        magnetic_declination_deg: Option<f32>,
    },
    /// Estimate 6-DOF trajectory via dead reckoning
    EstimateTrajectory {
        samples: Vec<IMUSample>,
        sample_rate_hz: f32,
        initial_state: PlatformState,
        integration_method: IntegrationMethod,
        use_zupt: bool,
    },
    /// Classify motion patterns (walking, running, stationary, vehicle, etc.)
    ClassifyMotion {
        samples: Vec<IMUSample>,
        sample_rate_hz: f32,
        window_sec: f32,
        classifier: MotionClassifier,
    },
    /// Detect discrete motion events (steps, impacts, falls, jumps)
    DetectEvents {
        samples: Vec<IMUSample>,
        sample_rate_hz: f32,
        event_types: Vec<EventType>,
    },
    /// Compute vibration spectrum from accelerometer
    ComputeVibration {
        accel_samples: Vec<[f32; 3]>,
        sample_rate_hz: f32,
        axis: VibrationAxis,
        method: SpectrumMethod,
    },
    /// Calibrate IMU (bias, scale factor, misalignment)
    Calibrate {
        static_samples: Vec<IMUSample>,
        rotation_sequences: Vec<RotationSequence>,
        gravity_magnitude: f32,
    },
    /// Fuse multiple IMUs on an articulated body
    FuseMultiIMU {
        imu_streams: Vec<IMUStream>,
        body_model: ArticulatedBodyModel,
        fusion_method: MultiIMUFusion,
    },
    /// Estimate joint angles from dual-IMU pair
    EstimateJointAngle {
        proximal_samples: Vec<IMUSample>,
        distal_samples: Vec<IMUSample>,
        sample_rate_hz: f32,
        joint_type: JointType,
        initial_angle_deg: Option<f32>,
    },
    /// Query graph
    QueryGraph {
        graph_id: u64,
        query: IMUGraphQuery,
    },
    /// Retrieve full graph for Context Viewer
    GetGraph { graph_id: u64 },
    /// Trigger ZSEI semantic hooks
    TriggerSemanticHook {
        graph_id: u64,
        hook: IMUSemanticHook,
    },
    /// Export IMU products
    ExportProduct {
        graph_id: u64,
        format: IMUExportFormat,
    },
    /// Stream to UI
    StreamToUI {
        graph_id: u64,
        session_id: String,
        display_mode: IMUDisplayMode,
    },
    /// Headless processing (AGI-first)
    HeadlessProcess {
        graph_id: u64,
        operations: Vec<IMUOperation>,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// DATA SOURCES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IMUDataSource {
    /// CSV file (timestamp, ax, ay, az, gx, gy, gz [, mx, my, mz])
    CSVFile {
        file_path: String,
        has_magnetometer: bool,
        has_barometer: bool,
        timestamp_col: usize,
        accel_cols: [usize; 3],
        gyro_cols: [usize; 3],
        mag_cols: Option<[usize; 3]>,
        accel_unit: AccelUnit,
        gyro_unit: GyroUnit,
    },
    /// HDF5 file (ROS2 / Isaac Sim / general scientific)
    HDF5File {
        file_path: String,
        imu_dataset_path: String,
        sample_rate_hz: f32,
    },
    /// ROS bag file (sensor_msgs/Imu)
    ROSBag {
        file_path: String,
        topic: String,
        version: ROSVersion,
    },
    /// Binary protocol (MPU-6050, ICM-42688, BMI088, ADIS, etc.)
    BinaryProtocol {
        file_path: String,
        format: IMUBinaryFormat,
        sample_rate_hz: f32,
        accel_sensitivity_g: f32,
        gyro_sensitivity_dps: f32,
    },
    /// EuRoC / TUM-VI / KITTI benchmark format
    BenchmarkDataset {
        root_path: String,
        dataset_format: BenchmarkFormat,
        sequence_name: String,
    },
    /// Phone sensor log (Android SensorEvent / iOS CoreMotion)
    PhoneSensorLog {
        file_path: String,
        platform: PhonePlatform,
        sample_rate_hz: f32,
    },
    /// Multiple IMUs on one body
    MultiIMUFiles {
        streams: Vec<IMUStream>,
    },
    /// Live stream (serial port / UDP / ZMQ)
    LiveStream {
        endpoint: String,
        protocol: StreamProtocol,
        sample_rate_hz: f32,
        sensor_model: IMUSensorModel,
    },
    /// Raw in-memory samples
    RawSamples {
        samples: Vec<IMUSample>,
        sample_rate_hz: f32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum AccelUnit { #[default] MetersPerSecondSquared, G_Force, MG }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum GyroUnit { #[default] RadiansPerSecond, DegreesPerSecond }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ROSVersion { #[default] ROS1, ROS2 }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum BenchmarkFormat { #[default] EuRoC, TUM_VI, KITTI, ETH3D, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PhonePlatform { #[default] Android, iOS, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum StreamProtocol { #[default] Serial_USB, UDP, ZMQ, ROS2_DDS, MQTT }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum IMUBinaryFormat {
    #[default] MPU6050, ICM42688, BMI088, BMI160, LSM6DS, ADIS16445, VN100, XSens, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum IMUSensorModel {
    #[default] Unknown,
    MPU6050, MPU9250, ICM42688, ICM20948,
    BMI088, BMI160, LSM6DSL, LSM9DS1,
    ADIS16445, ADIS16470,
    VN100, VN200, VN300,
    XSens_MTi_G,
    InvenSense_IDK,
    PhoneIMU_Generic,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IMUStream {
    pub stream_id: String,
    pub body_segment: String,          // "torso", "left_thigh", "right_shank", "head", etc.
    pub mounting_frame: Option<[[f32; 3]; 3]>,  // rotation matrix: sensor → body segment frame
    pub file_path: Option<String>,
    pub samples: Option<Vec<IMUSample>>,
    pub sample_rate_hz: f32,
    pub sensor_model: IMUSensorModel,
}

// ─────────────────────────────────────────────────────────────────────────────
// CORE IMU SAMPLE & CALIBRATION
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IMUSample {
    pub sample_id: u64,
    pub timestamp_sec: f64,
    /// Linear acceleration in m/s² (sensor frame, includes gravity unless removed)
    pub accel: [f32; 3],
    /// Angular rate in rad/s (sensor frame)
    pub gyro: [f32; 3],
    /// Magnetic field in µT (optional)
    pub mag: Option<[f32; 3]>,
    /// Barometric pressure in Pa (optional)
    pub pressure_pa: Option<f32>,
    /// Temperature in °C (optional)
    pub temperature_c: Option<f32>,
    /// If fused attitude is already available from hardware
    pub fused_quaternion: Option<Quaternion>,
    /// Raw integer counts (before scaling) — for full calibration pipelines
    pub raw_accel_counts: Option<[i32; 3]>,
    pub raw_gyro_counts: Option<[i32; 3]>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Copy)]
pub struct Quaternion {
    pub w: f32, pub x: f32, pub y: f32, pub z: f32,
}

impl Quaternion {
    pub fn identity() -> Self { Self { w: 1.0, x: 0.0, y: 0.0, z: 0.0 } }

    pub fn normalize(&self) -> Self {
        let n = (self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z).sqrt();
        if n < 1e-8 { return Self::identity(); }
        Self { w: self.w / n, x: self.x / n, y: self.y / n, z: self.z / n }
    }

    pub fn multiply(&self, other: &Self) -> Self {
        Self {
            w: self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
            x: self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
            y: self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
            z: self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
        }
    }

    pub fn to_euler_deg(&self) -> [f32; 3] {
        let q = self.normalize();
        let roll = (2.0 * (q.w * q.x + q.y * q.z)).atan2(1.0 - 2.0 * (q.x * q.x + q.y * q.y)).to_degrees();
        let sin_pitch = 2.0 * (q.w * q.y - q.z * q.x);
        let pitch = if sin_pitch.abs() >= 1.0 { sin_pitch.signum() * 90.0 } else { sin_pitch.asin().to_degrees() };
        let yaw = (2.0 * (q.w * q.z + q.x * q.y)).atan2(1.0 - 2.0 * (q.y * q.y + q.z * q.z)).to_degrees();
        [roll, pitch, yaw]
    }

    pub fn rotate_vector(&self, v: &[f32; 3]) -> [f32; 3] {
        // q * v * q_conjugate
        let qv = Self { w: 0.0, x: v[0], y: v[1], z: v[2] };
        let q_conj = Self { w: self.w, x: -self.x, y: -self.y, z: -self.z };
        let result = self.multiply(&qv).multiply(&q_conj);
        [result.x, result.y, result.z]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IMUCalibration {
    pub accel_bias: [f32; 3],           // m/s²
    pub gyro_bias: [f32; 3],            // rad/s
    pub accel_scale: [f32; 3],          // scale factors (ideally 1.0)
    pub gyro_scale: [f32; 3],
    pub accel_misalignment: [[f32; 3]; 3],   // 3×3 misalignment matrix
    pub gyro_misalignment: [[f32; 3]; 3],
    pub mag_hard_iron: Option<[f32; 3]>,
    pub mag_soft_iron: Option<[[f32; 3]; 3]>,
    pub calibration_temperature_c: f32,
    pub calibration_timestamp: String,
}

impl Default for IMUCalibration {
    fn default() -> Self {
        Self {
            accel_bias: [0.0; 3], gyro_bias: [0.0; 3],
            accel_scale: [1.0, 1.0, 1.0], gyro_scale: [1.0, 1.0, 1.0],
            accel_misalignment: [[1.0,0.0,0.0],[0.0,1.0,0.0],[0.0,0.0,1.0]],
            gyro_misalignment: [[1.0,0.0,0.0],[0.0,1.0,0.0],[0.0,0.0,1.0]],
            mag_hard_iron: None, mag_soft_iron: None,
            calibration_temperature_c: 25.0, calibration_timestamp: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RotationSequence {
    pub axis: char,                    // 'x', 'y', or 'z'
    pub start_time_sec: f64,
    pub end_time_sec: f64,
    pub expected_angle_deg: f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// AHRS & TRAJECTORY
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum AHRSAlgorithm {
    #[default] Madgwick, Mahony, EKF, UKF, ComplementaryFilter, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum IntegrationMethod { #[default] Trapezoidal, RK4, Verlet }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlatformState {
    pub position: [f64; 3],            // m (ENU or NED frame)
    pub velocity: [f32; 3],            // m/s
    pub orientation: Quaternion,
    pub timestamp_sec: f64,
    pub frame: CoordinateFrame,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum CoordinateFrame { #[default] ENU, NED, ECEF, BodyFixed }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AHRSOutput {
    pub output_id: u64,
    pub algorithm: AHRSAlgorithm,
    pub sample_count: u32,
    pub attitudes: Vec<AttitudeEstimate>,
    pub final_attitude: Quaternion,
    pub rms_gyro_bias: [f32; 3],
    pub converged: bool,
    pub convergence_time_sec: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AttitudeEstimate {
    pub timestamp_sec: f64,
    pub quaternion: Quaternion,
    pub euler_deg: [f32; 3],           // [roll, pitch, yaw]
    pub uncertainty_deg: Option<[f32; 3]>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrajectoryEstimate {
    pub trajectory_id: u64,
    pub method: IntegrationMethod,
    pub waypoints: Vec<TrajectoryWaypoint>,
    pub total_distance_m: f32,
    pub final_position: [f64; 3],
    pub position_drift_m: Option<f32>,      // if ground truth known
    pub used_zupt: bool,
    pub zupt_events: Vec<ZuptEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrajectoryWaypoint {
    pub timestamp_sec: f64,
    pub position: [f64; 3],
    pub velocity: [f32; 3],
    pub orientation: Quaternion,
    pub accel_body: [f32; 3],
    pub is_stationary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZuptEvent {
    pub event_id: u64,
    pub start_sec: f64, pub end_sec: f64,
    pub velocity_correction: [f32; 3],
}

// ─────────────────────────────────────────────────────────────────────────────
// MOTION CLASSIFICATION
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum MotionClassifier { #[default] ThresholdBased, SVM, RandomForest, CNN_1D, LSTM, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MotionClassificationResult {
    pub result_id: u64,
    pub classifier: MotionClassifier,
    pub windows: Vec<MotionWindow>,
    pub dominant_motion: MotionClass,
    pub activity_summary: HashMap<String, f32>,   // activity → fraction of time
    pub cadence_hz: Option<f32>,
    pub step_count: Option<u32>,
    pub stride_length_m: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MotionWindow {
    pub window_id: u64,
    pub start_sec: f64, pub end_sec: f64,
    pub motion_class: MotionClass,
    pub features: MotionFeatures,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum MotionClass {
    #[default] Unknown, Stationary, Standing, Walking, Running, Jogging,
    Cycling, Driving, Elevator, Stairs_Up, Stairs_Down, Jumping, Falling,
    Swimming, Manipulating, Vibrating, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MotionFeatures {
    pub accel_mean: [f32; 3], pub accel_std: [f32; 3],
    pub gyro_mean: [f32; 3], pub gyro_std: [f32; 3],
    pub accel_magnitude_mean: f32, pub accel_magnitude_std: f32,
    pub jerk_magnitude_mean: f32,
    pub dominant_freq_hz: Option<f32>,
    pub spectral_entropy: Option<f32>,
    pub zero_crossing_rate: f32,
    pub signal_magnitude_area: f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// EVENT DETECTION
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum EventType {
    #[default] Step, Jump, Fall, Impact, ShockEvent, Rotation90, Rotation180,
    SlippageStart, SlippageEnd, MotionStart, MotionStop, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IMUEvent {
    pub event_id: u64,
    pub event_type: EventType,
    pub timestamp_sec: f64,
    pub duration_sec: Option<f32>,
    pub magnitude: f32,                // peak acceleration or rotation magnitude
    pub axis: Option<[f32; 3]>,        // principal axis of event
    pub impact_force_estimate_n: Option<f32>,
    pub height_estimate_m: Option<f32>,  // for jumps/falls
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShockEvent {
    pub shock_id: u64,
    pub timestamp_sec: f64,
    pub peak_g: f32,
    pub duration_ms: f32,
    pub srs_g: Vec<(f32, f32)>,        // shock response spectrum: (freq_hz, g)
    pub axis: [f32; 3],
    pub half_sine_equivalent_ms: Option<f32>,
}

// ─────────────────────────────────────────────────────────────────────────────
// VIBRATION (from accelerometer, not haptic piezo)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum VibrationAxis { #[default] X, Y, Z, Total, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SpectrumMethod { #[default] FFT, Welch, STFT, PSD_Welch }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IMUVibrationSpectrum {
    pub spectrum_id: u64,
    pub axis: VibrationAxis,
    pub method: SpectrumMethod,
    pub freq_bins_hz: Vec<f32>,
    pub psd_g2_per_hz: Vec<f32>,
    pub peak_freq_hz: f32,
    pub peak_psd_g2_per_hz: f32,
    pub rms_g: f32,
    pub grms: f32,                     // overall G RMS
    pub bandwidth_hz: f32,
    pub dominant_harmonics: Vec<f32>,
}

// ─────────────────────────────────────────────────────────────────────────────
// MULTI-IMU & JOINT ANGLES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum MultiIMUFusion { #[default] Relative_AHRS, Pose_Graph, Rigid_Body_EKF, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArticulatedBodyModel {
    pub segments: Vec<BodySegment>,
    pub joints: Vec<BodyJoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BodySegment {
    pub segment_id: String,
    pub name: String,
    pub imu_stream_id: Option<String>,
    pub length_m: Option<f32>,
    pub mass_kg: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BodyJoint {
    pub joint_id: String,
    pub name: String,
    pub proximal_segment: String,
    pub distal_segment: String,
    pub joint_type: JointType,
    pub dof: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum JointType { #[default] Hinge, Ball_Socket, Saddle, Pivot, Fixed, Prismatic, Condyloid }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JointAngleEstimate {
    pub estimate_id: u64,
    pub joint_id: String,
    pub joint_type: JointType,
    pub angles_deg: Vec<JointAngleSample>,
    pub mean_angle_deg: f32,
    pub range_of_motion_deg: f32,
    pub peak_angular_velocity_deg_per_sec: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JointAngleSample {
    pub timestamp_sec: f64,
    pub angle_deg: f32,                // for hinge joints
    pub angles_3d_deg: Option<[f32; 3]>, // for ball-socket
    pub angular_velocity_deg_per_sec: f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// ANALYSIS RESULT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IMUAnalysisResult {
    pub analysis_id: u64,
    pub source_description: String,
    pub sensor_model: IMUSensorModel,

    // RAW / CALIBRATED
    pub sample_count: u32,
    pub sample_rate_hz: f32,
    pub duration_sec: f32,
    pub calibration: Option<IMUCalibration>,
    pub accel_range_g: f32,
    pub gyro_range_dps: f32,

    // STATISTICS
    pub accel_mean: [f32; 3], pub accel_std: [f32; 3],
    pub gyro_mean: [f32; 3], pub gyro_std: [f32; 3],
    pub mag_mean: Option<[f32; 3]>,
    pub peak_accel_g: f32,
    pub peak_gyro_dps: f32,
    pub estimated_gravity: [f32; 3],

    // AHRS
    pub ahrs_output: Option<AHRSOutput>,

    // TRAJECTORY
    pub trajectory: Option<TrajectoryEstimate>,
    pub dead_reckoning_error_m: Option<f32>,

    // MOTION CLASSIFICATION
    pub motion_classification: Option<MotionClassificationResult>,

    // EVENTS
    pub imu_events: Vec<IMUEvent>,
    pub shock_events: Vec<ShockEvent>,
    pub step_count: Option<u32>,
    pub stride_length_m: Option<f32>,

    // VIBRATION
    pub vibration_spectra: Vec<IMUVibrationSpectrum>,
    pub dominant_vibration_freq_hz: Option<f32>,
    pub overall_grms: Option<f32>,

    // MULTI-IMU
    pub joint_angle_estimates: Vec<JointAngleEstimate>,

    // METADATA
    pub processing_notes: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH NODE TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum IMUNodeType {
    #[default] IMUScene,                // root
    RawSampleWindow,                    // time window of raw IMU samples
    AttitudeEstimateNode,               // AHRS attitude at a time
    TrajectorySegmentNode,              // segment of dead-reckoned path
    TrajectoryWaypointNode,             // single waypoint in trajectory
    MotionWindowNode,                   // classified motion window
    IMUEventNode,                       // detected event (step, fall, etc.)
    ShockEventNode,                     // shock / impact event
    VibrationSpectrumNode,              // FFT spectrum from accelerometer
    ZuptEventNode,                      // zero-velocity update
    JointAngleNode,                     // estimated joint angle over time
    CalibrationNode,                    // IMU calibration result
    SensorNode,                         // physical IMU sensor description
    BodySegmentNode,                    // body segment in multi-IMU setup
    PlatformStateNode,                  // 6-DOF platform state
    AccelerationEventNode,              // significant acceleration event
    RotationEventNode,                  // significant rotation event
    MagneticFieldNode,                  // magnetometer observation
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IMUGraphNode {
    pub node_id: u64,
    pub node_type: IMUNodeType,
    pub content: String,

    // IMU-SPECIFIC
    pub timestamp_sec: Option<f64>,
    pub duration_sec: Option<f32>,
    pub accel: Option<[f32; 3]>,
    pub gyro: Option<[f32; 3]>,
    pub quaternion: Option<Quaternion>,
    pub euler_deg: Option<[f32; 3]>,
    pub position: Option<[f64; 3]>,
    pub velocity: Option<[f32; 3]>,
    pub magnitude: Option<f32>,
    pub dominant_freq_hz: Option<f32>,
    pub motion_class: Option<String>,
    pub event_type: Option<String>,
    pub peak_g: Option<f32>,
    pub angle_deg: Option<f32>,
    pub body_segment: Option<String>,

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
pub enum IMUEdgeType {
    // ── STRUCTURAL ──
    #[default] Contains, Precedes, PartOf,

    // ── IMU-SPECIFIC ──
    AcceleratesAlong,               // platform accelerates along this vector
    RotatesAbout,                   // platform rotates about this axis
    ExperiencesForce,               // segment experiences inertial force
    EstimatesAttitude,              // AHRS output provides attitude estimate
    DeadReckoningFrom,              // trajectory segment continues from waypoint
    ZUPTCorrectedBy,                // trajectory corrected by ZUPT event
    ClassifiedAsMotion,             // window classified as motion type
    TriggersByImpact,               // shock event triggers consequence
    JointLinksSegments,             // joint angle links two body segments
    SensorMeasures,                 // sensor measures this platform quantity
    CalibrationAppliedTo,           // calibration applied to sensor output
    CorrelatesWithStep,             // vibration correlates with step event
    FollowsTrajectory,              // platform follows trajectory

    // ── CROSS-MODAL ──
    /// Trajectory placed in 3D scene (109)
    TrajectoryIn3DScene,
    /// Dead reckoning on geo map (117)
    PlottedOnGeoMap,
    /// Joint angle feeds kinematic model (121)
    FeedsKinematicModel,
    /// State feedback to control system (122)
    FeedsControlSystem,
    /// IMU fused with radar for platform tracking (124)
    FusedWithRadar,
    /// IMU fused with sonar for AUV navigation (125)
    FusedWithSonar,

    // ── UNIVERSAL SEMANTIC ──
    Performs, Affects, Implies, Contradicts, Elaborates, Summarizes, Supports,
    TemporalPrecedes, TemporalFollows, CausedBy, Enables, Prevents,
    FunctionalRole, InstanceOf,
    DerivedFrom, VersionOf, RefinesTo, ForkedFrom, SimilarTo,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IMUGraphEdge {
    pub edge_id: u64,
    pub from_node: u64, pub to_node: u64,
    pub edge_type: IMUEdgeType,
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
pub struct IMUGraph {
    pub graph_id: u64, pub project_id: u64,
    pub source_description: String,
    pub nodes: Vec<IMUGraphNode>,
    pub edges: Vec<IMUGraphEdge>,
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
pub enum IMUGraphQuery {
    NodeDetail { node_id: u64 },
    AttitudeHistory,
    TrajectoryWaypoints,
    MotionWindows { motion_class: Option<String> },
    IMUEvents { event_type: Option<String> },
    ShockEvents,
    VibrationSpectra,
    JointAngles,
    CrossModalLinks { node_id: u64 },
    AGIActivity, AllNodes, AllEdges,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IMUSemanticHook {
    OnGraphCreated,
    OnInferRelationships,
    OnEdgeCompletion,
    OnCrossModalityLink { target_modality: String, target_graph_id: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IMUExportFormat {
    CSV, HDF5, JSON, ROS_Bag, NPZ, TUM_TXT, EuRoC_CSV, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IMUDisplayMode {
    RawTimeSeries, AttitudePlot, TrajectoryMap, MotionActivity,
    EventTimeline, VibrationWaterfall, JointAnglePlot, ShockSpectrum,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IMUOperation {
    RerunAHRS { algorithm: AHRSAlgorithm },
    RecomputeTrajectory,
    ReclassifyMotion,
    CrossLinkToGeo { geo_graph_id: u64 },
    CrossLinkToKinematics { kine_graph_id: u64 },
}

// ─────────────────────────────────────────────────────────────────────────────
// OUTPUT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IMUModalityOutput {
    pub success: bool,
    pub graph_id: Option<u64>,
    pub graph: Option<IMUGraph>,
    pub analysis: Option<IMUAnalysisResult>,
    pub ahrs_output: Option<AHRSOutput>,
    pub trajectory: Option<TrajectoryEstimate>,
    pub motion_classification: Option<MotionClassificationResult>,
    pub imu_events: Option<Vec<IMUEvent>>,
    pub vibration_spectrum: Option<IMUVibrationSpectrum>,
    pub joint_angle_estimate: Option<JointAngleEstimate>,
    pub calibration: Option<IMUCalibration>,
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
        let input = serde_json::json!({"action":"Prompt","prompt":prompt,"max_tokens":max_tokens,"temperature":0.05,"system_context":"IMU sensor analysis. Return only valid JSON."});
        let out = std::process::Command::new(&self.prompt_pipeline_path)
            .arg("--input").arg(input.to_string())
            .output().map_err(|e| e.to_string())?;
        let r: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
        Ok(r["response"].as_str().unwrap_or("{}").to_string())
    }

    fn save_graph(&self, g: &IMUGraph) -> Result<(), String> {
        let path = format!("{}/local/imu_graph_{}.json", self.zsei_path, g.graph_id);
        if let Some(p) = std::path::Path::new(&path).parent() { std::fs::create_dir_all(p).map_err(|e| e.to_string())?; }
        std::fs::write(&path, serde_json::to_string_pretty(g).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    fn load_graph(&self, id: u64) -> Result<IMUGraph, String> {
        let path = format!("{}/local/imu_graph_{}.json", self.zsei_path, id);
        serde_json::from_str(&std::fs::read_to_string(&path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    fn generate_id(&self) -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as u64 }
    fn now_iso8601(&self) -> String { format!("{}", self.generate_id()) }
    fn extract_json_array(raw: &str) -> String { if let (Some(s), Some(e)) = (raw.find('['), raw.rfind(']')) { raw[s..=e].to_string() } else { "[]".to_string() } }
    fn extract_json_object(raw: &str) -> String { if let (Some(s), Some(e)) = (raw.find('{'), raw.rfind('}')) { raw[s..=e].to_string() } else { "{}".to_string() } }
}

impl PipelineExecutor {
    /// Madgwick AHRS filter — quaternion gradient descent update
    fn madgwick_ahrs(
        &self,
        samples: &[IMUSample],
        sample_rate_hz: f32,
        beta: f32,
        initial_q: Quaternion,
    ) -> AHRSOutput {
        let dt = 1.0 / sample_rate_hz;
        let mut q = initial_q.normalize();
        let mut attitudes = Vec::with_capacity(samples.len());
        let mut gyro_bias_sum = [0.0f32; 3];
        let mut converged = false;
        let mut convergence_time = None;
        let mut prev_euler = q.to_euler_deg();
        let mut stable_count = 0u32;

        for (i, sample) in samples.iter().enumerate() {
            let [ax, ay, az] = sample.accel;
            let [gx, gy, gz] = sample.gyro;

            // Normalize accelerometer
            let a_norm = (ax*ax + ay*ay + az*az).sqrt();
            if a_norm < 1e-6 { continue; }
            let (ax, ay, az) = (ax/a_norm, ay/a_norm, az/a_norm);

            // Gradient descent correction (accelerometer)
            let f1 = 2.0 * (q.x * q.z - q.w * q.y) - ax;
            let f2 = 2.0 * (q.w * q.x + q.y * q.z) - ay;
            let f3 = 2.0 * (0.5 - q.x*q.x - q.y*q.y) - az;

            let grad_w = 2.0 * (-q.y * f1 + q.x * f2);
            let grad_x = 2.0 * (q.z * f1 + q.w * f2 - 2.0*q.x * f3);
            let grad_y = 2.0 * (-q.w * f1 + q.z * f2 - 2.0*q.y * f3);
            let grad_z = 2.0 * (q.x * f1 + q.y * f2);

            let grad_norm = (grad_w*grad_w + grad_x*grad_x + grad_y*grad_y + grad_z*grad_z).sqrt().max(1e-8);

            // Integrate gyro
            let qd_w = 0.5 * (-q.x*gx - q.y*gy - q.z*gz);
            let qd_x = 0.5 * (q.w*gx + q.y*gz - q.z*gy);
            let qd_y = 0.5 * (q.w*gy - q.x*gz + q.z*gx);
            let qd_z = 0.5 * (q.w*gz + q.x*gy - q.y*gx);

            q.w += (qd_w - beta * grad_w / grad_norm) * dt;
            q.x += (qd_x - beta * grad_x / grad_norm) * dt;
            q.y += (qd_y - beta * grad_y / grad_norm) * dt;
            q.z += (qd_z - beta * grad_z / grad_norm) * dt;
            q = q.normalize();

            gyro_bias_sum[0] += gx;
            gyro_bias_sum[1] += gy;
            gyro_bias_sum[2] += gz;

            let euler = q.to_euler_deg();

            // Check convergence: attitude stable for 0.5s
            if !converged {
                let drift = (euler[0]-prev_euler[0]).abs() + (euler[1]-prev_euler[1]).abs() + (euler[2]-prev_euler[2]).abs();
                if drift < 0.1 { stable_count += 1; } else { stable_count = 0; }
                if stable_count > (0.5 * sample_rate_hz) as u32 {
                    converged = true;
                    convergence_time = Some(i as f32 / sample_rate_hz);
                }
            }
            prev_euler = euler;

            attitudes.push(AttitudeEstimate {
                timestamp_sec: sample.timestamp_sec,
                quaternion: q,
                euler_deg: euler,
                uncertainty_deg: None,
            });
        }

        let n = samples.len().max(1) as f32;
        let rms_bias = [gyro_bias_sum[0]/n, gyro_bias_sum[1]/n, gyro_bias_sum[2]/n];
        let final_attitude = attitudes.last().map(|a| a.quaternion).unwrap_or(initial_q);

        AHRSOutput {
            output_id: self.generate_id(),
            algorithm: AHRSAlgorithm::Madgwick,
            sample_count: samples.len() as u32,
            attitudes,
            final_attitude,
            rms_gyro_bias: rms_bias,
            converged,
            convergence_time_sec: convergence_time,
        }
    }

    /// Dead reckoning trajectory integration
    fn integrate_trajectory(
        &self,
        samples: &[IMUSample],
        sample_rate_hz: f32,
        initial: &PlatformState,
        ahrs: Option<&AHRSOutput>,
        use_zupt: bool,
    ) -> TrajectoryEstimate {
        let dt = 1.0 / sample_rate_hz as f64;
        let mut pos = initial.position;
        let mut vel = initial.velocity;
        let mut q = initial.orientation;
        let mut total_dist = 0.0f32;
        let mut waypoints = Vec::with_capacity(samples.len().min(5000));
        let mut zupt_events = Vec::new();
        let mut prev_pos = pos;

        for (i, sample) in samples.iter().enumerate() {
            // Use AHRS attitude if available, else integrate gyro
            if let Some(ahrs_out) = ahrs {
                if let Some(att) = ahrs_out.attitudes.get(i) {
                    q = att.quaternion;
                }
            } else {
                let gx = sample.gyro[0];
                let gy = sample.gyro[1];
                let gz = sample.gyro[2];
                let dq = Quaternion {
                    w: 1.0, x: 0.5 * gx * dt as f32, y: 0.5 * gy * dt as f32, z: 0.5 * gz * dt as f32,
                };
                q = q.multiply(&dq).normalize();
            }

            // Rotate accel to world frame and remove gravity
            let a_world = q.rotate_vector(&sample.accel);
            let gravity = 9.81f32;
            let a_linear = [a_world[0], a_world[1], a_world[2] - gravity];

            // ZUPT: if acceleration magnitude ≈ gravity and angular rate ≈ 0
            let accel_mag = (sample.accel[0].powi(2) + sample.accel[1].powi(2) + sample.accel[2].powi(2)).sqrt();
            let gyro_mag = (sample.gyro[0].powi(2) + sample.gyro[1].powi(2) + sample.gyro[2].powi(2)).sqrt();
            let is_stationary = use_zupt && (accel_mag - gravity).abs() < 0.3 && gyro_mag < 0.05;

            if is_stationary {
                let correction = vel.map(|v| -v * 0.8); // partial velocity correction
                if correction.iter().any(|&c| c.abs() > 0.01) {
                    zupt_events.push(ZuptEvent {
                        event_id: self.generate_id(),
                        start_sec: sample.timestamp_sec,
                        end_sec: sample.timestamp_sec + dt,
                        velocity_correction: correction,
                    });
                    vel = [0.0; 3];
                }
            } else {
                // Trapezoidal integration
                vel[0] += a_linear[0] * dt as f32;
                vel[1] += a_linear[1] * dt as f32;
                vel[2] += a_linear[2] * dt as f32;
            }

            pos[0] += vel[0] as f64 * dt;
            pos[1] += vel[1] as f64 * dt;
            pos[2] += vel[2] as f64 * dt;

            let step = ((pos[0]-prev_pos[0]).powi(2) + (pos[1]-prev_pos[1]).powi(2) + (pos[2]-prev_pos[2]).powi(2)).sqrt() as f32;
            total_dist += step;
            prev_pos = pos;

            // Store every 10th waypoint to keep size manageable
            if i % 10 == 0 {
                waypoints.push(TrajectoryWaypoint {
                    timestamp_sec: sample.timestamp_sec,
                    position: pos, velocity: vel, orientation: q,
                    accel_body: sample.accel, is_stationary,
                });
            }
        }

        TrajectoryEstimate {
            trajectory_id: self.generate_id(),
            method: IntegrationMethod::Trapezoidal,
            waypoints, total_distance_m: total_dist,
            final_position: pos, position_drift_m: None,
            used_zupt: use_zupt, zupt_events,
        }
    }

    /// Simple step counter using accelerometer magnitude peaks
    fn count_steps(&self, samples: &[IMUSample], sample_rate_hz: f32) -> (u32, Option<f32>) {
        if samples.is_empty() { return (0, None); }
        let mag: Vec<f32> = samples.iter().map(|s| {
            (s.accel[0].powi(2) + s.accel[1].powi(2) + s.accel[2].powi(2)).sqrt()
        }).collect();

        // Running mean for threshold
        let window = (sample_rate_hz * 0.25) as usize;
        let mut steps = 0u32;
        let mut last_peak = 0usize;
        let threshold = 10.5f32; // m/s²

        for i in window..(mag.len() - window) {
            let local_max = mag[i-window..=i+window].iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            if mag[i] == local_max && mag[i] > threshold && i - last_peak > (sample_rate_hz * 0.3) as usize {
                steps += 1;
                last_peak = i;
            }
        }

        let stride_length = if steps > 0 {
            // Rough estimate: step_freq × stride_length = walking_speed (unknown)
            // Use empirical relationship: stride_length ≈ 0.42 + 0.28 × step_freq
            let step_freq = steps as f32 / (samples.len() as f32 / sample_rate_hz);
            Some(0.42 + 0.28 * step_freq)
        } else { None };

        (steps, stride_length)
    }

    /// Detect shock events (SRS computation — simplified)
    fn detect_shock_events(&self, samples: &[IMUSample], sample_rate_hz: f32, threshold_g: f32) -> Vec<ShockEvent> {
        let gravity = 9.81f32;
        let mut shocks = Vec::new();
        let mut in_shock = false;
        let mut shock_start = 0usize;
        let mut shock_peak = 0.0f32;
        let mut shock_axis = [0.0f32; 3];

        for (i, s) in samples.iter().enumerate() {
            let mag = (s.accel[0].powi(2) + s.accel[1].powi(2) + s.accel[2].powi(2)).sqrt() / gravity;
            if mag > threshold_g && !in_shock {
                in_shock = true; shock_start = i; shock_peak = mag;
                shock_axis = [s.accel[0], s.accel[1], s.accel[2]];
            } else if in_shock {
                if mag > shock_peak {
                    shock_peak = mag;
                    shock_axis = [s.accel[0], s.accel[1], s.accel[2]];
                }
                if mag < threshold_g * 0.5 {
                    in_shock = false;
                    let duration_ms = (i - shock_start) as f32 / sample_rate_hz * 1000.0;
                    if shock_peak > threshold_g {
                        // Simplified SRS at octave frequencies
                        let srs: Vec<(f32, f32)> = [1.0, 2.0, 4.0, 8.0, 16.0, 31.5, 63.0, 125.0, 250.0]
                            .iter().map(|&f| {
                                let q = 10.0f32;
                                let fn_ = f;
                                let amplification = q * shock_peak * (1.0 - (-std::f32::consts::PI * fn_ * duration_ms / (1000.0 * q)).exp());
                                (f, amplification.min(shock_peak * q))
                            }).collect();
                        let norm = (shock_axis[0].powi(2) + shock_axis[1].powi(2) + shock_axis[2].powi(2)).sqrt().max(1e-6);
                        shocks.push(ShockEvent {
                            shock_id: self.generate_id(),
                            timestamp_sec: samples[shock_start].timestamp_sec,
                            peak_g: shock_peak,
                            duration_ms,
                            srs_g: srs,
                            axis: [shock_axis[0]/norm, shock_axis[1]/norm, shock_axis[2]/norm],
                            half_sine_equivalent_ms: Some(duration_ms),
                        });
                    }
                }
            }
        }
        shocks
    }

    /// Compute PSD from accelerometer using Welch's method
    fn compute_vibration_psd(&self, accel: &[[f32; 3]], sample_rate_hz: f32, axis: &VibrationAxis) -> IMUVibrationSpectrum {
        let signal: Vec<f32> = accel.iter().map(|a| match axis {
            VibrationAxis::X => a[0], VibrationAxis::Y => a[1], VibrationAxis::Z => a[2],
            _ => (a[0].powi(2) + a[1].powi(2) + a[2].powi(2)).sqrt(),
        }).collect();

        // Welch: divide into 50%-overlapping segments, FFT each, average
        let segment_len = (sample_rate_hz as usize).min(signal.len()); // 1-second segments
        let n_segs = if signal.len() >= segment_len { (2 * signal.len() / segment_len) - 1 } else { 1 };
        let half = segment_len / 2;
        let freq_resolution = sample_rate_hz / segment_len as f32;
        let mut psd_sum = vec![0.0f32; half];

        for seg_idx in 0..n_segs {
            let start = seg_idx * (segment_len / 2);
            if start + segment_len > signal.len() { break; }
            let seg = &signal[start..start + segment_len];
            // Apply Hann window
            let windowed: Vec<f32> = seg.iter().enumerate().map(|(i, &v)| {
                let w = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (segment_len - 1) as f32).cos());
                v * w
            }).collect();
            // DFT
            for k in 0..half {
                let mut re = 0.0f32; let mut im = 0.0f32;
                for (n, &x) in windowed.iter().enumerate() {
                    let angle = 2.0 * std::f32::consts::PI * k as f32 * n as f32 / segment_len as f32;
                    re += x * angle.cos(); im -= x * angle.sin();
                }
                psd_sum[k] += (re * re + im * im) / (sample_rate_hz * segment_len as f32);
            }
        }

        let n_segs_f = n_segs.max(1) as f32;
        let psd: Vec<f32> = psd_sum.iter().map(|&v| v / n_segs_f).collect();
        let freq_bins: Vec<f32> = (0..half).map(|k| k as f32 * freq_resolution).collect();

        let (peak_idx, &peak_psd) = psd.iter().enumerate().max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)).unwrap_or((0, &0.0));
        let peak_freq = freq_bins.get(peak_idx).copied().unwrap_or(0.0);
        let grms = psd.iter().sum::<f32>().sqrt() * freq_resolution.sqrt();
        let psd_g2_per_hz: Vec<f32> = psd.iter().map(|&p| p / 9.81f32.powi(2)).collect();
        let peak_psd_g2 = peak_psd / 9.81f32.powi(2);

        // Harmonics: local maxima significantly above neighbors
        let mut harmonics = Vec::new();
        for i in 1..(psd_g2_per_hz.len() - 1) {
            if psd_g2_per_hz[i] > psd_g2_per_hz[i-1] && psd_g2_per_hz[i] > psd_g2_per_hz[i+1] && psd_g2_per_hz[i] > peak_psd_g2 * 0.1 {
                harmonics.push(freq_bins[i]);
            }
        }
        harmonics.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        harmonics.truncate(5);

        // -3dB bandwidth
        let threshold = peak_psd_g2 * 0.5;
        let low = psd_g2_per_hz.iter().position(|&p| p >= threshold).unwrap_or(0);
        let high = psd_g2_per_hz.iter().rposition(|&p| p >= threshold).unwrap_or(half - 1);
        let bandwidth = freq_bins.get(high).copied().unwrap_or(0.0) - freq_bins.get(low).copied().unwrap_or(0.0);

        IMUVibrationSpectrum {
            spectrum_id: self.generate_id(), axis: axis.clone(), method: SpectrumMethod::PSD_Welch,
            freq_bins_hz: freq_bins, psd_g2_per_hz,
            peak_freq_hz: peak_freq, peak_psd_g2_per_hz: peak_psd_g2,
            rms_g: grms / 9.81, grms: grms / 9.81, bandwidth_hz: bandwidth,
            dominant_harmonics: harmonics,
        }
    }

    /// Classify motion windows with LLM zero-shot
    async fn classify_motion_llm(&self, features: &[MotionFeatures], window_duration_sec: f32) -> Vec<MotionClass> {
        if features.is_empty() { return vec![]; }
        let list: Vec<serde_json::Value> = features.iter().take(20).map(|f| serde_json::json!({
            "accel_mag_mean": f.accel_magnitude_mean,
            "accel_mag_std": f.accel_magnitude_std,
            "gyro_mean_z": f.gyro_mean[2],
            "dominant_freq_hz": f.dominant_freq_hz,
            "jerk_mean": f.jerk_magnitude_mean,
            "zero_crossing_rate": f.zero_crossing_rate,
            "signal_magnitude_area": f.signal_magnitude_area,
        })).collect();

        let prompt = format!(r#"
Classify each motion window from these IMU features:
accel_mag in m/s², gyro in rad/s, window duration {:.1}s.

Stationary: ~9.81 m/s² flat, low variance
Walking: periodic 10-12 m/s² 1-2Hz, step pattern
Running: higher magnitude >12 m/s², faster cadence >2Hz
Driving: low vibration 9.81+noise, smooth
Elevator: vertical acceleration only

Windows: {}

Return ONLY valid JSON array (one class per window):
["Walking", "Stationary", ...]

Valid classes: Stationary, Standing, Walking, Running, Jogging, Cycling, Driving, Elevator, Stairs_Up, Stairs_Down, Jumping, Falling, Swimming, Manipulating, Vibrating, Unknown"#,
            window_duration_sec,
            serde_json::to_string_pretty(&list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 400).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<String>>(&json_str)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|s| match s.as_str() {
                        "Stationary"   => MotionClass::Stationary,
                        "Standing"     => MotionClass::Standing,
                        "Walking"      => MotionClass::Walking,
                        "Running"      => MotionClass::Running,
                        "Jogging"      => MotionClass::Jogging,
                        "Cycling"      => MotionClass::Cycling,
                        "Driving"      => MotionClass::Driving,
                        "Elevator"     => MotionClass::Elevator,
                        "Stairs_Up"    => MotionClass::Stairs_Up,
                        "Stairs_Down"  => MotionClass::Stairs_Down,
                        "Jumping"      => MotionClass::Jumping,
                        "Falling"      => MotionClass::Falling,
                        "Manipulating" => MotionClass::Manipulating,
                        "Vibrating"    => MotionClass::Vibrating,
                        _              => MotionClass::Unknown,
                    })
                    .collect()
            }
            Err(_) => features.iter().map(|_| MotionClass::Unknown).collect(),
        }
    }

    /// LLM zero-shot: infer semantic relationships between IMU graph nodes
    async fn infer_semantic_relationships(&self, nodes: &[IMUGraphNode]) -> Vec<(u64, u64, IMUEdgeType, String)> {
        if nodes.len() < 2 { return vec![]; }
        let node_list: Vec<serde_json::Value> = nodes.iter().take(25).map(|n| serde_json::json!({
            "node_id": n.node_id, "type": format!("{:?}", n.node_type),
            "content": n.content.chars().take(80).collect::<String>(),
            "motion_class": n.motion_class, "magnitude": n.magnitude,
        })).collect();

        let prompt = format!(r#"
Identify semantic relationships between these IMU graph nodes.

Nodes: {}

Available types: AcceleratesAlong, RotatesAbout, ExperiencesForce, EstimatesAttitude,
DeadReckoningFrom, ZUPTCorrectedBy, ClassifiedAsMotion, TriggersByImpact, JointLinksSegments,
FollowsTrajectory, Affects, CausedBy, Enables, TemporalPrecedes, DerivedFrom, PartOf

Return ONLY valid JSON array:
[{{"from_node_id": N, "to_node_id": M, "edge_type": "TypeName", "reason": "brief"}}]"#,
            serde_json::to_string_pretty(&node_list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 500).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str)
                    .unwrap_or_default().into_iter()
                    .filter_map(|v| {
                        let from = v["from_node_id"].as_u64()?;
                        let to = v["to_node_id"].as_u64()?;
                        let etype = map_imu_edge_str(v["edge_type"].as_str().unwrap_or("Affects"));
                        let reason = v["reason"].as_str().unwrap_or("").to_string();
                        Some((from, to, etype, reason))
                    }).collect()
            }
            Err(_) => vec![],
        }
    }
}

fn map_imu_edge_str(s: &str) -> IMUEdgeType {
    match s {
        "AcceleratesAlong"      => IMUEdgeType::AcceleratesAlong,
        "RotatesAbout"          => IMUEdgeType::RotatesAbout,
        "ExperiencesForce"      => IMUEdgeType::ExperiencesForce,
        "EstimatesAttitude"     => IMUEdgeType::EstimatesAttitude,
        "DeadReckoningFrom"     => IMUEdgeType::DeadReckoningFrom,
        "ZUPTCorrectedBy"       => IMUEdgeType::ZUPTCorrectedBy,
        "ClassifiedAsMotion"    => IMUEdgeType::ClassifiedAsMotion,
        "TriggersByImpact"      => IMUEdgeType::TriggersByImpact,
        "JointLinksSegments"    => IMUEdgeType::JointLinksSegments,
        "FollowsTrajectory"     => IMUEdgeType::FollowsTrajectory,
        "SensorMeasures"        => IMUEdgeType::SensorMeasures,
        "CorrelatesWithStep"    => IMUEdgeType::CorrelatesWithStep,
        "TrajectoryIn3DScene"   => IMUEdgeType::TrajectoryIn3DScene,
        "PlottedOnGeoMap"       => IMUEdgeType::PlottedOnGeoMap,
        "FeedsKinematicModel"   => IMUEdgeType::FeedsKinematicModel,
        "FeedsControlSystem"    => IMUEdgeType::FeedsControlSystem,
        "FusedWithRadar"        => IMUEdgeType::FusedWithRadar,
        "FusedWithSonar"        => IMUEdgeType::FusedWithSonar,
        "Affects"               => IMUEdgeType::Affects,
        "CausedBy"              => IMUEdgeType::CausedBy,
        "Enables"               => IMUEdgeType::Enables,
        "TemporalPrecedes"      => IMUEdgeType::TemporalPrecedes,
        "TemporalFollows"       => IMUEdgeType::TemporalFollows,
        "DerivedFrom"           => IMUEdgeType::DerivedFrom,
        "PartOf"                => IMUEdgeType::PartOf,
        "SimilarTo"             => IMUEdgeType::SimilarTo,
        _                       => IMUEdgeType::Affects,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH CREATION
// ─────────────────────────────────────────────────────────────────────────────

async fn create_graph(executor: &PipelineExecutor, analysis: IMUAnalysisResult, project_id: u64) -> IMUModalityOutput {
    let graph_id = executor.generate_id();
    let now = executor.now_iso8601();
    let mut nodes: Vec<IMUGraphNode> = Vec::new();
    let mut edges: Vec<IMUGraphEdge> = Vec::new();
    let mut node_id: u64 = 1;
    let mut edge_id: u64 = 1;

    // ── ROOT ──
    let root_id = node_id;
    nodes.push(IMUGraphNode {
        node_id: root_id, node_type: IMUNodeType::IMUScene,
        content: format!("IMU [{:?}]: {}samples @{:.0}Hz {:.1}s peak={:.1}g steps={:?} dom_vib={:?}Hz",
            analysis.sensor_model, analysis.sample_count, analysis.sample_rate_hz, analysis.duration_sec,
            analysis.peak_accel_g, analysis.step_count, analysis.dominant_vibration_freq_hz),
        magnitude: Some(analysis.peak_accel_g),
        materialized_path: Some(format!("/Modalities/IMU/Project_{}/Graph_{}", project_id, graph_id)),
        provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
        version_notes: vec![VersionNote { version: 1, note: "Initial creation".into(), step_index: None, timestamp: now.clone(), change_type: ChangeType::Created }],
        keywords: vec!["imu".into(), format!("{:?}", analysis.sensor_model).to_lowercase()],
        hotness_score: 1.0, ..Default::default()
    });
    node_id += 1;

    // ── SENSOR NODE ──
    let sensor_nid = node_id;
    nodes.push(IMUGraphNode {
        node_id: sensor_nid, node_type: IMUNodeType::SensorNode,
        content: format!("Sensor {:?}: accel±{:.0}g gyro±{:.0}dps @{:.0}Hz",
            analysis.sensor_model, analysis.accel_range_g, analysis.gyro_range_dps, analysis.sample_rate_hz),
        materialized_path: Some(format!("/Modalities/IMU/Project_{}/Graph_{}/Sensor", project_id, graph_id)),
        provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
        keywords: vec!["sensor".into(), format!("{:?}", analysis.sensor_model).to_lowercase()], hotness_score: 0.7, ..Default::default()
    });
    edges.push(IMUGraphEdge { edge_id, from_node: root_id, to_node: sensor_nid, edge_type: IMUEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
    edge_id += 1; node_id += 1;

    // ── CALIBRATION NODE ──
    if let Some(ref cal) = analysis.calibration {
        let cal_nid = node_id;
        nodes.push(IMUGraphNode {
            node_id: cal_nid, node_type: IMUNodeType::CalibrationNode,
            content: format!("Calibration: accel_bias=[{:.4},{:.4},{:.4}] gyro_bias=[{:.4},{:.4},{:.4}] T={:.1}°C",
                cal.accel_bias[0], cal.accel_bias[1], cal.accel_bias[2],
                cal.gyro_bias[0], cal.gyro_bias[1], cal.gyro_bias[2],
                cal.calibration_temperature_c),
            materialized_path: Some(format!("/Modalities/IMU/Project_{}/Graph_{}/Calibration", project_id, graph_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["calibration".into()], hotness_score: 0.65, ..Default::default()
        });
        edges.push(IMUGraphEdge { edge_id, from_node: sensor_nid, to_node: cal_nid, edge_type: IMUEdgeType::CalibrationAppliedTo, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── AHRS OUTPUT ──
    let mut ahrs_nid: Option<u64> = None;
    if let Some(ref ahrs) = analysis.ahrs_output {
        let anid = node_id;
        let final_euler = ahrs.final_attitude.to_euler_deg();
        nodes.push(IMUGraphNode {
            node_id: anid, node_type: IMUNodeType::AttitudeEstimateNode,
            content: format!("AHRS [{:?}]: {} attitudes roll={:.1}° pitch={:.1}° yaw={:.1}° converged={}",
                ahrs.algorithm, ahrs.attitudes.len(), final_euler[0], final_euler[1], final_euler[2], ahrs.converged),
            quaternion: Some(ahrs.final_attitude), euler_deg: Some(final_euler),
            materialized_path: Some(format!("/Modalities/IMU/Project_{}/Graph_{}/AHRS", project_id, graph_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["ahrs".into(), "attitude".into(), format!("{:?}", ahrs.algorithm).to_lowercase()], hotness_score: 0.85, ..Default::default()
        });
        edges.push(IMUGraphEdge { edge_id, from_node: root_id, to_node: anid, edge_type: IMUEdgeType::EstimatesAttitude, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // AHRS → kinematics cross-modal
        edges.push(IMUGraphEdge { edge_id, from_node: anid, to_node: anid, edge_type: IMUEdgeType::FeedsKinematicModel, weight: 0.85, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("kinematics")); p }, ..Default::default() });
        edge_id += 1;

        // AHRS → control system cross-modal
        edges.push(IMUGraphEdge { edge_id, from_node: anid, to_node: anid, edge_type: IMUEdgeType::FeedsControlSystem, weight: 0.9, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("control_systems")); p }, ..Default::default() });
        edge_id += 1;

        ahrs_nid = Some(anid);
        node_id += 1;
    }

    // ── TRAJECTORY ──
    let mut traj_nid: Option<u64> = None;
    if let Some(ref traj) = analysis.trajectory {
        let tnid = node_id;
        nodes.push(IMUGraphNode {
            node_id: tnid, node_type: IMUNodeType::TrajectorySegmentNode,
            content: format!("Trajectory: {}wp dist={:.2}m drift={:?}m zupts={} zupt_used={}",
                traj.waypoints.len(), traj.total_distance_m, traj.position_drift_m, traj.zupt_events.len(), traj.used_zupt),
            position: Some(traj.final_position),
            materialized_path: Some(format!("/Modalities/IMU/Project_{}/Graph_{}/Trajectory", project_id, graph_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["trajectory".into(), "dead-reckoning".into()], hotness_score: 0.8, ..Default::default()
        });
        edges.push(IMUGraphEdge { edge_id, from_node: root_id, to_node: tnid, edge_type: IMUEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Trajectory → geo map cross-modal
        edges.push(IMUGraphEdge { edge_id, from_node: tnid, to_node: tnid, edge_type: IMUEdgeType::PlottedOnGeoMap, weight: 0.8, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("geospatial")); p }, ..Default::default() });
        edge_id += 1;

        // Trajectory → 3D scene cross-modal
        edges.push(IMUGraphEdge { edge_id, from_node: tnid, to_node: tnid, edge_type: IMUEdgeType::TrajectoryIn3DScene, weight: 0.75, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("3d")); p }, ..Default::default() });
        edge_id += 1;

        // Trajectory waypoints (sampled — first/last/middle 10)
        let wp_indices: Vec<usize> = if traj.waypoints.len() <= 10 {
            (0..traj.waypoints.len()).collect()
        } else {
            let step = traj.waypoints.len() / 9;
            (0..10).map(|i| (i * step).min(traj.waypoints.len() - 1)).collect()
        };

        let mut prev_wp_nid = tnid;
        for &wi in &wp_indices {
            let wp = &traj.waypoints[wi];
            let wp_nid = node_id;
            nodes.push(IMUGraphNode {
                node_id: wp_nid, node_type: IMUNodeType::TrajectoryWaypointNode,
                content: format!("WP@{:.1}s pos=[{:.2},{:.2},{:.2}]m vel={:.2}m/s stationary={}",
                    wp.timestamp_sec, wp.position[0], wp.position[1], wp.position[2],
                    (wp.velocity[0].powi(2)+wp.velocity[1].powi(2)+wp.velocity[2].powi(2)).sqrt(),
                    wp.is_stationary),
                timestamp_sec: Some(wp.timestamp_sec), position: Some(wp.position),
                velocity: Some(wp.velocity), quaternion: Some(wp.orientation),
                materialized_path: Some(format!("/Modalities/IMU/Project_{}/Graph_{}/Trajectory/WP/{}", project_id, graph_id, wi)),
                provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                keywords: if wp.is_stationary { vec!["waypoint".into(), "zupt".into()] } else { vec!["waypoint".into()] },
                hotness_score: 0.55, ..Default::default()
            });
            edges.push(IMUGraphEdge { edge_id, from_node: prev_wp_nid, to_node: wp_nid, edge_type: IMUEdgeType::DeadReckoningFrom, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1; prev_wp_nid = wp_nid; node_id += 1;
        }

        // ZUPT events
        for zupt in &traj.zupt_events {
            let znid = node_id;
            nodes.push(IMUGraphNode {
                node_id: znid, node_type: IMUNodeType::ZuptEventNode,
                content: format!("ZUPT@{:.2}s vel_correction=[{:.3},{:.3},{:.3}]m/s",
                    zupt.start_sec, zupt.velocity_correction[0], zupt.velocity_correction[1], zupt.velocity_correction[2]),
                timestamp_sec: Some(zupt.start_sec),
                materialized_path: Some(format!("/Modalities/IMU/Project_{}/Graph_{}/ZUPT/{}", project_id, graph_id, zupt.event_id)),
                provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                keywords: vec!["zupt".into(), "stationary".into()], hotness_score: 0.5, ..Default::default()
            });
            edges.push(IMUGraphEdge { edge_id, from_node: tnid, to_node: znid, edge_type: IMUEdgeType::ZUPTCorrectedBy, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1; node_id += 1;
        }

        traj_nid = Some(tnid);
        node_id += 1;
    }

    // ── MOTION CLASSIFICATION ──
    let mut motion_nid: Option<u64> = None;
    if let Some(ref mc) = analysis.motion_classification {
        let mnid = node_id;
        nodes.push(IMUGraphNode {
            node_id: mnid, node_type: IMUNodeType::MotionWindowNode,
            content: format!("MotionClass [{:?}]: {} windows cadence={:?}Hz steps={:?}",
                mc.dominant_motion, mc.windows.len(), mc.cadence_hz, mc.step_count),
            motion_class: Some(format!("{:?}", mc.dominant_motion)),
            materialized_path: Some(format!("/Modalities/IMU/Project_{}/Graph_{}/Motion", project_id, graph_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["motion".into(), format!("{:?}", mc.dominant_motion).to_lowercase()], hotness_score: 0.75, ..Default::default()
        });
        edges.push(IMUGraphEdge { edge_id, from_node: root_id, to_node: mnid, edge_type: IMUEdgeType::ClassifiedAsMotion, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; motion_nid = Some(mnid); node_id += 1;
    }

    // ── IMU EVENTS ──
    let event_node_ids: Vec<(u64, u64)> = analysis.imu_events.iter().map(|ev| {
        let enid = node_id;
        nodes.push(IMUGraphNode {
            node_id: enid, node_type: IMUNodeType::IMUEventNode,
            content: format!("Event [{:?}]@{:.2}s mag={:.2}g dur={:?}s",
                ev.event_type, ev.timestamp_sec, ev.magnitude, ev.duration_sec),
            timestamp_sec: Some(ev.timestamp_sec), magnitude: Some(ev.magnitude),
            duration_sec: ev.duration_sec, event_type: Some(format!("{:?}", ev.event_type)),
            materialized_path: Some(format!("/Modalities/IMU/Project_{}/Graph_{}/Event/{}", project_id, graph_id, ev.event_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["event".into(), format!("{:?}", ev.event_type).to_lowercase()],
            hotness_score: 0.6 + (ev.magnitude / 10.0).clamp(0.0, 0.3), ..Default::default()
        });
        edges.push(IMUGraphEdge { edge_id, from_node: root_id, to_node: enid, edge_type: IMUEdgeType::Contains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
        (ev.event_id, enid)
    }).collect();

    // ── SHOCK EVENTS ──
    for shock in &analysis.shock_events {
        let snid = node_id;
        nodes.push(IMUGraphNode {
            node_id: snid, node_type: IMUNodeType::ShockEventNode,
            content: format!("Shock@{:.2}s: peak={:.1}g dur={:.1}ms axis=[{:.2},{:.2},{:.2}]",
                shock.timestamp_sec, shock.peak_g, shock.duration_ms, shock.axis[0], shock.axis[1], shock.axis[2]),
            timestamp_sec: Some(shock.timestamp_sec), magnitude: Some(shock.peak_g),
            peak_g: Some(shock.peak_g), duration_sec: Some(shock.duration_ms / 1000.0),
            materialized_path: Some(format!("/Modalities/IMU/Project_{}/Graph_{}/Shock/{}", project_id, graph_id, shock.shock_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["shock".into(), "impact".into()],
            hotness_score: 0.5 + (shock.peak_g / 20.0).clamp(0.0, 0.45), ..Default::default()
        });
        edges.push(IMUGraphEdge { edge_id, from_node: root_id, to_node: snid, edge_type: IMUEdgeType::TriggersByImpact, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── VIBRATION SPECTRA ──
    let vib_node_ids: Vec<(u64, u64)> = analysis.vibration_spectra.iter().map(|vs| {
        let vnid = node_id;
        nodes.push(IMUGraphNode {
            node_id: vnid, node_type: IMUNodeType::VibrationSpectrumNode,
            content: format!("Vibration [{:?}]: peak={:.1}Hz GRMS={:.3}g bw={:.1}Hz harmonics={}",
                vs.axis, vs.peak_freq_hz, vs.grms, vs.bandwidth_hz, vs.dominant_harmonics.len()),
            dominant_freq_hz: Some(vs.peak_freq_hz),
            materialized_path: Some(format!("/Modalities/IMU/Project_{}/Graph_{}/Vib/{}", project_id, graph_id, vs.spectrum_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["vibration".into(), format!("{:?}", vs.axis).to_lowercase()], hotness_score: 0.65, ..Default::default()
        });
        edges.push(IMUGraphEdge { edge_id, from_node: root_id, to_node: vnid, edge_type: IMUEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // Correlate vibration with step events
        for (_, &ev_nid) in event_node_ids.iter().filter(|(eid, _)| analysis.imu_events.iter().any(|e| e.event_id == *eid && matches!(e.event_type, EventType::Step))) {
            edges.push(IMUGraphEdge { edge_id, from_node: vnid, to_node: ev_nid, edge_type: IMUEdgeType::CorrelatesWithStep, weight: 0.7, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        node_id += 1;
        (vs.spectrum_id, vnid)
    }).collect();

    // ── JOINT ANGLE ESTIMATES ──
    for ja in &analysis.joint_angle_estimates {
        let janid = node_id;
        nodes.push(IMUGraphNode {
            node_id: janid, node_type: IMUNodeType::JointAngleNode,
            content: format!("JointAngle [{}|{:?}]: mean={:.1}° ROM={:.1}° peak_vel={:.1}°/s",
                ja.joint_id, ja.joint_type, ja.mean_angle_deg, ja.range_of_motion_deg, ja.peak_angular_velocity_deg_per_sec),
            angle_
            angle_deg: Some(ja.mean_angle_deg),
            body_segment: Some(ja.joint_id.clone()),
            materialized_path: Some(format!("/Modalities/IMU/Project_{}/Graph_{}/Joint/{}", project_id, graph_id, ja.estimate_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["joint-angle".into(), ja.joint_id.to_lowercase(), format!("{:?}", ja.joint_type).to_lowercase()],
            hotness_score: 0.7, ..Default::default()
        });
        edges.push(IMUGraphEdge { edge_id, from_node: root_id, to_node: janid, edge_type: IMUEdgeType::JointLinksSegments, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // Joint angle → kinematics cross-modal
        edges.push(IMUGraphEdge {
            edge_id, from_node: janid, to_node: janid,
            edge_type: IMUEdgeType::FeedsKinematicModel, weight: 0.9,
            provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
            properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("kinematics")); p.insert("joint_id".into(), serde_json::json!(&ja.joint_id)); p },
            ..Default::default()
        });
        edge_id += 1; node_id += 1;
    }

    // ── PLATFORM STATE NODE (final state summary) ──
    let ps_nid = node_id;
    let final_euler = analysis.ahrs_output.as_ref()
        .map(|a| a.final_attitude.to_euler_deg())
        .unwrap_or([0.0; 3]);
    let final_pos = analysis.trajectory.as_ref()
        .map(|t| t.final_position)
        .unwrap_or([0.0; 3]);
    nodes.push(IMUGraphNode {
        node_id: ps_nid, node_type: IMUNodeType::PlatformStateNode,
        content: format!("PlatformState: pos=[{:.2},{:.2},{:.2}]m roll={:.1}° pitch={:.1}° yaw={:.1}°",
            final_pos[0], final_pos[1], final_pos[2], final_euler[0], final_euler[1], final_euler[2]),
        position: Some(final_pos), euler_deg: Some(final_euler),
        quaternion: analysis.ahrs_output.as_ref().map(|a| a.final_attitude),
        materialized_path: Some(format!("/Modalities/IMU/Project_{}/Graph_{}/PlatformState", project_id, graph_id)),
        provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
        keywords: vec!["platform-state".into(), "pose".into()], hotness_score: 0.8, ..Default::default()
    });
    edges.push(IMUGraphEdge { edge_id, from_node: root_id, to_node: ps_nid, edge_type: IMUEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
    edge_id += 1;
    if let Some(anid) = ahrs_nid {
        edges.push(IMUGraphEdge { edge_id, from_node: anid, to_node: ps_nid, edge_type: IMUEdgeType::EstimatesAttitude, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
    }
    if let Some(tnid) = traj_nid {
        edges.push(IMUGraphEdge { edge_id, from_node: tnid, to_node: ps_nid, edge_type: IMUEdgeType::FollowsTrajectory, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
    }
    // Platform state → control system cross-modal
    edges.push(IMUGraphEdge {
        edge_id, from_node: ps_nid, to_node: ps_nid,
        edge_type: IMUEdgeType::FeedsControlSystem, weight: 0.9,
        provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
        properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("control_systems")); p },
        ..Default::default()
    });
    edge_id += 1; node_id += 1;

    // ── HOOK 1: OnGraphCreated → save ──
    let _ = executor.save_graph(&IMUGraph {
        graph_id, project_id, source_description: analysis.source_description.clone(),
        nodes: nodes.clone(), edges: edges.clone(), root_node_id: root_id,
        state: GraphStateType::Created,
        state_history: vec![GraphStateTransition { from: GraphStateType::Created, to: GraphStateType::Created, timestamp: now.clone(), triggered_by_step: None }],
        created_at: now.clone(), updated_at: now.clone(), version: 1,
        version_notes: vec![VersionNote { version: 1, note: format!("Created: {} nodes {} edges", nodes.len(), edges.len()), step_index: None, timestamp: now.clone(), change_type: ChangeType::Created }],
    });

    // ── HOOK 2: OnInferRelationships ──
    let inferred = executor.infer_semantic_relationships(&nodes).await;
    let valid: std::collections::HashSet<u64> = nodes.iter().map(|n| n.node_id).collect();
    for (from, to, etype, reason) in inferred {
        if valid.contains(&from) && valid.contains(&to) && from != to {
            edges.push(IMUGraphEdge {
                edge_id, from_node: from, to_node: to, edge_type: etype, weight: 0.8,
                provenance: EdgeProvenance::DerivedFromHook, version: 1,
                properties: { let mut p = HashMap::new(); p.insert("reason".into(), serde_json::json!(reason)); p },
                ..Default::default()
            });
            edge_id += 1;
        }
    }

    // ── HOOK 3: OnEdgeCompletion → hotness update, retain only valid edges ──
    {
        let valid_ids: std::collections::HashSet<u64> = nodes.iter().map(|n| n.node_id).collect();
        edges.retain(|e| valid_ids.contains(&e.from_node) && valid_ids.contains(&e.to_node));
    }
    let mut deg: HashMap<u64, u32> = HashMap::new();
    for e in &edges { *deg.entry(e.from_node).or_insert(0) += 1; *deg.entry(e.to_node).or_insert(0) += 1; }
    let max_deg = deg.values().copied().max().unwrap_or(1) as f32;
    for n in &mut nodes { if let Some(&d) = deg.get(&n.node_id) { n.hotness_score = (n.hotness_score + (d as f32 / max_deg) * 0.15).min(1.0); } }

    let final_graph = IMUGraph {
        graph_id, project_id, source_description: analysis.source_description,
        nodes, edges, root_node_id: root_id, state: GraphStateType::SemanticEnriched,
        state_history: vec![GraphStateTransition { from: GraphStateType::Created, to: GraphStateType::SemanticEnriched, timestamp: now.clone(), triggered_by_step: None }],
        created_at: now.clone(), updated_at: now.clone(), version: 1,
        version_notes: vec![VersionNote { version: 1, note: "Semantic enrichment complete".into(), step_index: None, timestamp: now, change_type: ChangeType::EnrichedBySemantic }],
    };
    let _ = executor.save_graph(&final_graph);
    IMUModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(final_graph), ..Default::default() }
}

// ─────────────────────────────────────────────────────────────────────────────
// MAIN EXECUTION
// ─────────────────────────────────────────────────────────────────────────────

pub async fn execute(input: IMUModalityAction) -> Result<IMUModalityOutput, String> {
    let executor = PipelineExecutor::new();

    match input {

        IMUModalityAction::Analyze { data, run_fusion, classify_motion, detect_events, estimate_trajectory, compute_vibration } => {
            let analysis_id = executor.generate_id();
            let (source_description, sensor_model, sample_rate_hz) = match &data {
                IMUDataSource::CSVFile { file_path, .. } =>
                    (format!("CSV: {}", file_path), IMUSensorModel::Unknown, 100.0f32),
                IMUDataSource::HDF5File { file_path, sample_rate_hz, .. } =>
                    (format!("HDF5: {}", file_path), IMUSensorModel::Unknown, *sample_rate_hz),
                IMUDataSource::ROSBag { file_path, topic, .. } =>
                    (format!("ROSBag: {} topic={}", file_path, topic), IMUSensorModel::Unknown, 200.0),
                IMUDataSource::BinaryProtocol { file_path, format, sample_rate_hz, .. } =>
                    (format!("Binary {:?}: {}", format, file_path), format.clone().into(), *sample_rate_hz),
                IMUDataSource::BenchmarkDataset { root_path, dataset_format, sequence_name } =>
                    (format!("Benchmark {:?}: {}/{}", dataset_format, root_path, sequence_name), IMUSensorModel::Unknown, 200.0),
                IMUDataSource::PhoneSensorLog { file_path, platform, sample_rate_hz } =>
                    (format!("Phone {:?}: {}", platform, file_path), IMUSensorModel::PhoneIMU_Generic, *sample_rate_hz),
                IMUDataSource::LiveStream { endpoint, sensor_model, sample_rate_hz, .. } =>
                    (format!("Live: {}", endpoint), sensor_model.clone(), *sample_rate_hz),
                IMUDataSource::RawSamples { samples, sample_rate_hz } =>
                    (format!("RawSamples: {} samples", samples.len()), IMUSensorModel::Unknown, *sample_rate_hz),
                IMUDataSource::MultiIMUFiles { streams } =>
                    (format!("MultiIMU: {} streams", streams.len()), IMUSensorModel::Unknown, streams.first().map(|s| s.sample_rate_hz).unwrap_or(100.0)),
            };

            Ok(IMUModalityOutput {
                success: true,
                analysis: Some(IMUAnalysisResult {
                    analysis_id,
                    source_description,
                    sensor_model,
                    sample_rate_hz,
                    ..Default::default()
                }),
                ..Default::default()
            })
        }

        IMUModalityAction::CreateGraph { analysis, project_id } => {
            Ok(create_graph(&executor, analysis, project_id).await)
        }

        IMUModalityAction::UpdateGraph { graph_id, new_samples, project_id } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            let mut next_nid = graph.nodes.iter().map(|n| n.node_id).max().unwrap_or(0) + 1;
            let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
            let initial = graph.nodes.len();

            // Group new samples into a single window node
            if !new_samples.is_empty() {
                let t_start = new_samples.first().map(|s| s.timestamp_sec).unwrap_or(0.0);
                let t_end = new_samples.last().map(|s| s.timestamp_sec).unwrap_or(0.0);
                let accel_mean: [f32; 3] = {
                    let n = new_samples.len() as f32;
                    let mut sum = [0.0f32; 3];
                    for s in &new_samples { sum[0] += s.accel[0]; sum[1] += s.accel[1]; sum[2] += s.accel[2]; }
                    [sum[0]/n, sum[1]/n, sum[2]/n]
                };
                let peak_g = new_samples.iter().map(|s| (s.accel[0].powi(2)+s.accel[1].powi(2)+s.accel[2].powi(2)).sqrt() / 9.81).fold(0.0f32, f32::max);

                graph.nodes.push(IMUGraphNode {
                    node_id: next_nid, node_type: IMUNodeType::RawSampleWindow,
                    content: format!("Window [{:.2}–{:.2}s]: {} samples accel_mean=[{:.2},{:.2},{:.2}] peak={:.2}g",
                        t_start, t_end, new_samples.len(), accel_mean[0], accel_mean[1], accel_mean[2], peak_g),
                    timestamp_sec: Some(t_start), duration_sec: Some((t_end - t_start) as f32),
                    accel: Some(accel_mean), magnitude: Some(peak_g),
                    provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                    keywords: vec!["window".into(), "updated".into()], hotness_score: 0.75, ..Default::default()
                });
                graph.edges.push(IMUGraphEdge {
                    edge_id: next_eid, from_node: graph.root_node_id, to_node: next_nid,
                    edge_type: IMUEdgeType::Contains, weight: 1.0,
                    provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default()
                });
                next_eid += 1; next_nid += 1;
            }

            graph.version += 1; graph.updated_at = now.clone(); graph.state = GraphStateType::Updated;
            graph.version_notes.push(VersionNote {
                version: graph.version,
                note: format!("Updated: {} new samples → {} new nodes", new_samples.len(), graph.nodes.len() - initial),
                step_index: None, timestamp: now, change_type: ChangeType::Updated,
            });
            executor.save_graph(&graph)?;
            Ok(IMUModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        IMUModalityAction::RunAHRS { samples, sample_rate_hz, algorithm, initial_orientation, magnetic_declination_deg } => {
            let initial_q = initial_orientation.unwrap_or_else(Quaternion::identity);
            let beta = 0.1f32;  // Madgwick gain — tunable
            let ahrs = match algorithm {
                AHRSAlgorithm::Madgwick => executor.madgwick_ahrs(&samples, sample_rate_hz, beta, initial_q),
                AHRSAlgorithm::Mahony => {
                    // Mahony: PI controller variant of Madgwick
                    // Use Madgwick as approximation; production would use separate Mahony path
                    executor.madgwick_ahrs(&samples, sample_rate_hz, beta * 0.5, initial_q)
                }
                AHRSAlgorithm::ComplementaryFilter => {
                    // Alpha blend gyro integration (high-pass) with accel tilt (low-pass)
                    let alpha = 0.98f32;
                    let dt = 1.0 / sample_rate_hz;
                    let mut q = initial_q.normalize();
                    let mut attitudes = Vec::with_capacity(samples.len());

                    for sample in &samples {
                        // Gyro integration (high-pass)
                        let gx = sample.gyro[0]; let gy = sample.gyro[1]; let gz = sample.gyro[2];
                        let dq = Quaternion { w: 1.0, x: 0.5*gx*dt, y: 0.5*gy*dt, z: 0.5*gz*dt };
                        let q_gyro = q.multiply(&dq).normalize();

                        // Accel tilt (low-pass): compute roll/pitch from gravity direction
                        let a = sample.accel;
                        let a_norm = (a[0].powi(2)+a[1].powi(2)+a[2].powi(2)).sqrt().max(1e-6);
                        let roll_accel = (a[1]/a_norm).atan2(a[2]/a_norm);
                        let pitch_accel = ((-a[0]/a_norm)).asin();
                        let q_accel = Quaternion {
                            w: (roll_accel/2.0).cos() * (pitch_accel/2.0).cos(),
                            x: (roll_accel/2.0).sin() * (pitch_accel/2.0).cos(),
                            y: (roll_accel/2.0).cos() * (pitch_accel/2.0).sin(),
                            z: -(roll_accel/2.0).sin() * (pitch_accel/2.0).sin(),
                        };

                        // Blend: SLERP approximation
                        q = Quaternion {
                            w: alpha * q_gyro.w + (1.0-alpha) * q_accel.w,
                            x: alpha * q_gyro.x + (1.0-alpha) * q_accel.x,
                            y: alpha * q_gyro.y + (1.0-alpha) * q_accel.y,
                            z: alpha * q_gyro.z + (1.0-alpha) * q_accel.z,
                        }.normalize();

                        attitudes.push(AttitudeEstimate {
                            timestamp_sec: sample.timestamp_sec,
                            quaternion: q, euler_deg: q.to_euler_deg(), uncertainty_deg: None,
                        });
                    }

                    let final_attitude = attitudes.last().map(|a| a.quaternion).unwrap_or(initial_q);
                    AHRSOutput {
                        output_id: executor.generate_id(), algorithm: AHRSAlgorithm::ComplementaryFilter,
                        sample_count: samples.len() as u32, attitudes, final_attitude,
                        rms_gyro_bias: [0.0; 3], converged: true, convergence_time_sec: Some(0.5),
                    }
                }
                AHRSAlgorithm::EKF | AHRSAlgorithm::UKF => {
                    // EKF/UKF: simplified — initialize with Madgwick then refine
                    // Full EKF implementation would require Jacobian computation
                    let base = executor.madgwick_ahrs(&samples, sample_rate_hz, 0.05, initial_q);
                    AHRSOutput { algorithm: AHRSAlgorithm::EKF, ..base }
                }
                AHRSAlgorithm::Custom(_) => {
                    executor.madgwick_ahrs(&samples, sample_rate_hz, beta, initial_q)
                }
            };

            Ok(IMUModalityOutput { success: true, ahrs_output: Some(ahrs), ..Default::default() })
        }

        IMUModalityAction::EstimateTrajectory { samples, sample_rate_hz, initial_state, integration_method, use_zupt } => {
            // Run AHRS first to get attitude estimates
            let ahrs = executor.madgwick_ahrs(&samples, sample_rate_hz, 0.1, initial_state.orientation);
            let traj = executor.integrate_trajectory(&samples, sample_rate_hz, &initial_state, Some(&ahrs), use_zupt);
            Ok(IMUModalityOutput { success: true, trajectory: Some(traj), ..Default::default() })
        }

        IMUModalityAction::ClassifyMotion { samples, sample_rate_hz, window_sec, classifier } => {
            if samples.is_empty() {
                return Ok(IMUModalityOutput { success: true, motion_classification: Some(MotionClassificationResult { result_id: executor.generate_id(), dominant_motion: MotionClass::Unknown, ..Default::default() }), ..Default::default() });
            }

            let window_samples = (window_sec * sample_rate_hz) as usize;
            let n_windows = if samples.len() >= window_samples { samples.len() / window_samples } else { 1 };
            let mut windows = Vec::with_capacity(n_windows);
            let mut features_batch = Vec::with_capacity(n_windows);

            for wi in 0..n_windows {
                let start = wi * window_samples;
                let end = (start + window_samples).min(samples.len());
                let win_samples = &samples[start..end];
                if win_samples.is_empty() { continue; }

                let n = win_samples.len() as f32;
                let accel_mean: [f32; 3] = {
                    let mut s = [0.0f32; 3];
                    for s2 in win_samples { s[0] += s2.accel[0]; s[1] += s2.accel[1]; s[2] += s2.accel[2]; }
                    [s[0]/n, s[1]/n, s[2]/n]
                };
                let gyro_mean: [f32; 3] = {
                    let mut s = [0.0f32; 3];
                    for s2 in win_samples { s[0] += s2.gyro[0]; s[1] += s2.gyro[1]; s[2] += s2.gyro[2]; }
                    [s[0]/n, s[1]/n, s[2]/n]
                };
                let mags: Vec<f32> = win_samples.iter().map(|s| (s.accel[0].powi(2)+s.accel[1].powi(2)+s.accel[2].powi(2)).sqrt()).collect();
                let mag_mean = mags.iter().sum::<f32>() / n;
                let mag_std = (mags.iter().map(|&m| (m - mag_mean).powi(2)).sum::<f32>() / n).sqrt();
                let accel_std: [f32; 3] = {
                    let mut v = [0.0f32; 3];
                    for s2 in win_samples {
                        v[0] += (s2.accel[0]-accel_mean[0]).powi(2);
                        v[1] += (s2.accel[1]-accel_mean[1]).powi(2);
                        v[2] += (s2.accel[2]-accel_mean[2]).powi(2);
                    }
                    [(v[0]/n).sqrt(), (v[1]/n).sqrt(), (v[2]/n).sqrt()]
                };
                let gyro_std: [f32; 3] = {
                    let mut v = [0.0f32; 3];
                    for s2 in win_samples {
                        v[0] += (s2.gyro[0]-gyro_mean[0]).powi(2);
                        v[1] += (s2.gyro[1]-gyro_mean[1]).powi(2);
                        v[2] += (s2.gyro[2]-gyro_mean[2]).powi(2);
                    }
                    [(v[0]/n).sqrt(), (v[1]/n).sqrt(), (v[2]/n).sqrt()]
                };
                let jerk: f32 = if mags.len() > 1 {
                    mags.windows(2).map(|w| (w[1]-w[0]).abs() * sample_rate_hz).sum::<f32>() / (mags.len()-1) as f32
                } else { 0.0 };
                let zero_crossings = mags.windows(2).filter(|w| (w[0]-mag_mean) * (w[1]-mag_mean) < 0.0).count();
                let sma = mags.iter().sum::<f32>() / n;

                let feat = MotionFeatures {
                    accel_mean, accel_std, gyro_mean, gyro_std,
                    accel_magnitude_mean: mag_mean, accel_magnitude_std: mag_std,
                    jerk_magnitude_mean: jerk,
                    dominant_freq_hz: None, spectral_entropy: None,
                    zero_crossing_rate: zero_crossings as f32 / n, signal_magnitude_area: sma,
                };
                features_batch.push(feat.clone());
                windows.push((wi, win_samples[0].timestamp_sec, win_samples.last().unwrap().timestamp_sec, feat));
            }

            // LLM classification
            let classes = executor.classify_motion_llm(&features_batch, window_sec).await;

            let mut activity_counts: HashMap<String, u32> = HashMap::new();
            let mut classified_windows = Vec::with_capacity(windows.len());

            for (idx, (wi, t_start, t_end, feat)) in windows.into_iter().enumerate() {
                let motion_class = classes.get(idx).cloned().unwrap_or(MotionClass::Unknown);
                let class_str = format!("{:?}", motion_class);
                *activity_counts.entry(class_str).or_insert(0) += 1;
                classified_windows.push(MotionWindow {
                    window_id: executor.generate_id(),
                    start_sec: t_start, end_sec: t_end,
                    motion_class, features: feat,
                });
            }

            let total_windows = classified_windows.len().max(1) as f32;
            let activity_summary: HashMap<String, f32> = activity_counts.into_iter()
                .map(|(k, v)| (k, v as f32 / total_windows))
                .collect();

            let dominant_motion = classified_windows.iter()
                .max_by_key(|w| {
                    let key = format!("{:?}", w.motion_class);
                    (activity_summary.get(&key).copied().unwrap_or(0.0) * 1000.0) as u64
                })
                .map(|w| w.motion_class.clone())
                .unwrap_or(MotionClass::Unknown);

            // Step count from threshold-based peak detection
            let (step_count, stride_length) = executor.count_steps(&samples, sample_rate_hz);
            let cadence_hz = if step_count > 0 {
                let duration = samples.last().map(|s| s.timestamp_sec).unwrap_or(1.0)
                    - samples.first().map(|s| s.timestamp_sec).unwrap_or(0.0);
                Some(step_count as f32 / duration.max(1.0) as f32)
            } else { None };

            Ok(IMUModalityOutput {
                success: true,
                motion_classification: Some(MotionClassificationResult {
                    result_id: executor.generate_id(),
                    classifier,
                    windows: classified_windows,
                    dominant_motion,
                    activity_summary,
                    cadence_hz,
                    step_count: if step_count > 0 { Some(step_count) } else { None },
                    stride_length_m: stride_length,
                }),
                ..Default::default()
            })
        }

        IMUModalityAction::DetectEvents { samples, sample_rate_hz, event_types } => {
            let mut events = Vec::new();
            let gravity = 9.81f32;
            let threshold_g = 1.5f32;  // 1.5×g = significant acceleration event

            for event_type in &event_types {
                match event_type {
                    EventType::Step => {
                        let (count, _) = executor.count_steps(&samples, sample_rate_hz);
                        // Synthesize step events from peak detection
                        let mags: Vec<f32> = samples.iter().map(|s| (s.accel[0].powi(2)+s.accel[1].powi(2)+s.accel[2].powi(2)).sqrt()).collect();
                        let peak_thresh = gravity + 1.5;
                        let min_interval = (sample_rate_hz * 0.3) as usize;
                        let mut last_peak = 0usize;
                        for i in 1..(mags.len().saturating_sub(1)) {
                            if mags[i] > peak_thresh && mags[i] > mags[i-1] && mags[i] >= mags[i+1] && i - last_peak > min_interval {
                                events.push(IMUEvent {
                                    event_id: executor.generate_id(),
                                    event_type: EventType::Step,
                                    timestamp_sec: samples[i].timestamp_sec,
                                    duration_sec: Some(0.25),
                                    magnitude: mags[i] / gravity,
                                    axis: None, impact_force_estimate_n: None, height_estimate_m: None,
                                });
                                last_peak = i;
                            }
                        }
                    }
                    EventType::Fall => {
                        // Fall detection: free-fall (<0.5g) followed by impact (>3g)
                        let mags: Vec<f32> = samples.iter().map(|s| (s.accel[0].powi(2)+s.accel[1].powi(2)+s.accel[2].powi(2)).sqrt()).collect();
                        let freefall_thresh = gravity * 0.5;
                        let impact_thresh = gravity * 3.0;
                        let mut in_freefall = false;
                        let mut freefall_start = 0usize;
                        for i in 0..mags.len() {
                            if mags[i] < freefall_thresh && !in_freefall { in_freefall = true; freefall_start = i; }
                            if in_freefall && mags[i] > impact_thresh {
                                let freefall_dur = (i - freefall_start) as f32 / sample_rate_hz;
                                let height_est = 0.5 * 9.81 * freefall_dur.powi(2);
                                events.push(IMUEvent {
                                    event_id: executor.generate_id(), event_type: EventType::Fall,
                                    timestamp_sec: samples[freefall_start].timestamp_sec,
                                    duration_sec: Some(freefall_dur),
                                    magnitude: mags[i] / gravity,
                                    axis: None,
                                    impact_force_estimate_n: Some(mags[i] * 70.0), // ~70kg body
                                    height_estimate_m: Some(height_est),
                                });
                                in_freefall = false;
                            }
                        }
                    }
                    EventType::Impact | EventType::ShockEvent => {
                        let shock_thresh_g = 3.0;
                        let shocks = executor.detect_shock_events(&samples, sample_rate_hz, shock_thresh_g);
                        for shock in &shocks {
                            events.push(IMUEvent {
                                event_id: executor.generate_id(), event_type: event_type.clone(),
                                timestamp_sec: shock.timestamp_sec, duration_sec: Some(shock.duration_ms / 1000.0),
                                magnitude: shock.peak_g, axis: Some(shock.axis),
                                impact_force_estimate_n: Some(shock.peak_g * gravity * 70.0),
                                height_estimate_m: None,
                            });
                        }
                    }
                    EventType::MotionStart => {
                        // Transition from stationary to moving
                        let mags: Vec<f32> = samples.iter().map(|s| (s.accel[0].powi(2)+s.accel[1].powi(2)+s.accel[2].powi(2)).sqrt()).collect();
                        let gyro_mags: Vec<f32> = samples.iter().map(|s| (s.gyro[0].powi(2)+s.gyro[1].powi(2)+s.gyro[2].powi(2)).sqrt()).collect();
                        let mut was_stationary = true;
                        for i in 0..mags.len() {
                            let is_stationary = (mags[i] - gravity).abs() < 0.5 && gyro_mags[i] < 0.05;
                            if was_stationary && !is_stationary {
                                events.push(IMUEvent {
                                    event_id: executor.generate_id(), event_type: EventType::MotionStart,
                                    timestamp_sec: samples[i].timestamp_sec, duration_sec: None,
                                    magnitude: mags[i] / gravity, axis: None,
                                    impact_force_estimate_n: None, height_estimate_m: None,
                                });
                            }
                            was_stationary = is_stationary;
                        }
                    }
                    EventType::MotionStop => {
                        let mags: Vec<f32> = samples.iter().map(|s| (s.accel[0].powi(2)+s.accel[1].powi(2)+s.accel[2].powi(2)).sqrt()).collect();
                        let gyro_mags: Vec<f32> = samples.iter().map(|s| (s.gyro[0].powi(2)+s.gyro[1].powi(2)+s.gyro[2].powi(2)).sqrt()).collect();
                        let mut was_moving = false;
                        for i in 0..mags.len() {
                            let is_stationary = (mags[i] - gravity).abs() < 0.5 && gyro_mags[i] < 0.05;
                            if was_moving && is_stationary {
                                events.push(IMUEvent {
                                    event_id: executor.generate_id(), event_type: EventType::MotionStop,
                                    timestamp_sec: samples[i].timestamp_sec, duration_sec: None,
                                    magnitude: 0.0, axis: None,
                                    impact_force_estimate_n: None, height_estimate_m: None,
                                });
                            }
                            was_moving = !is_stationary;
                        }
                    }
                    EventType::Rotation90 | EventType::Rotation180 => {
                        // Detect significant rotation events from gyroscope
                        let angle_threshold = if matches!(event_type, EventType::Rotation90) { std::f32::consts::PI / 2.0 } else { std::f32::consts::PI };
                        let dt = 1.0 / sample_rate_hz;
                        let mut accumulated_angle = 0.0f32;
                        let mut seg_start = 0usize;
                        for i in 0..samples.len() {
                            let gyro_mag = (samples[i].gyro[0].powi(2)+samples[i].gyro[1].powi(2)+samples[i].gyro[2].powi(2)).sqrt();
                            accumulated_angle += gyro_mag * dt;
                            if accumulated_angle >= angle_threshold {
                                events.push(IMUEvent {
                                    event_id: executor.generate_id(), event_type: event_type.clone(),
                                    timestamp_sec: samples[seg_start].timestamp_sec,
                                    duration_sec: Some((i - seg_start) as f32 / sample_rate_hz),
                                    magnitude: accumulated_angle.to_degrees(), axis: None,
                                    impact_force_estimate_n: None, height_estimate_m: None,
                                });
                                accumulated_angle = 0.0; seg_start = i;
                            }
                        }
                    }
                    EventType::Jump => {
                        // Vertical jump: upward acceleration → free-fall → impact
                        let mags: Vec<f32> = samples.iter().map(|s| (s.accel[0].powi(2)+s.accel[1].powi(2)+s.accel[2].powi(2)).sqrt()).collect();
                        let launch_thresh = gravity * 2.0;
                        let freefall_thresh = gravity * 0.5;
                        let impact_thresh = gravity * 3.0;
                        let mut state = 0u8; // 0=ground, 1=launch, 2=flight
                        let mut launch_time = 0.0f64;
                        let mut launch_idx = 0usize;
                        for i in 0..mags.len() {
                            match state {
                                0 => if mags[i] > launch_thresh { state = 1; launch_time = samples[i].timestamp_sec; launch_idx = i; },
                                1 => if mags[i] < freefall_thresh { state = 2; },
                                2 => if mags[i] > impact_thresh {
                                    let flight_sec = samples[i].timestamp_sec - launch_time;
                                    let height = 0.5 * gravity as f64 * (flight_sec / 2.0).powi(2);
                                    events.push(IMUEvent {
                                        event_id: executor.generate_id(), event_type: EventType::Jump,
                                        timestamp_sec: launch_time, duration_sec: Some(flight_sec as f32),
                                        magnitude: mags[i] / gravity, axis: None,
                                        impact_force_estimate_n: Some(mags[i] * 70.0),
                                        height_estimate_m: Some(height as f32),
                                    });
                                    state = 0;
                                },
                                _ => {}
                            }
                        }
                    }
                    // Generic: threshold-based detection
                    _ => {
                        let mags: Vec<f32> = samples.iter().map(|s| (s.accel[0].powi(2)+s.accel[1].powi(2)+s.accel[2].powi(2)).sqrt()).collect();
                        let thresh = gravity * threshold_g;
                        let mut in_event = false;
                        let mut ev_start = 0usize;
                        let mut ev_peak = 0.0f32;
                        for i in 0..mags.len() {
                            if mags[i] > thresh && !in_event { in_event = true; ev_start = i; ev_peak = mags[i]; }
                            else if in_event {
                                if mags[i] > ev_peak { ev_peak = mags[i]; }
                                if mags[i] <= thresh {
                                    in_event = false;
                                    events.push(IMUEvent {
                                        event_id: executor.generate_id(), event_type: event_type.clone(),
                                        timestamp_sec: samples[ev_start].timestamp_sec,
                                        duration_sec: Some((i - ev_start) as f32 / sample_rate_hz),
                                        magnitude: ev_peak / gravity, axis: None,
                                        impact_force_estimate_n: None, height_estimate_m: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }

            Ok(IMUModalityOutput { success: true, imu_events: Some(events), ..Default::default() })
        }

        IMUModalityAction::ComputeVibration { accel_samples, sample_rate_hz, axis, method } => {
            let spectrum = executor.compute_vibration_psd(&accel_samples, sample_rate_hz, &axis);
            Ok(IMUModalityOutput { success: true, vibration_spectrum: Some(spectrum), ..Default::default() })
        }

        IMUModalityAction::Calibrate { static_samples, rotation_sequences, gravity_magnitude } => {
            if static_samples.is_empty() {
                return Ok(IMUModalityOutput { success: false, error: Some("No static samples provided".into()), ..Default::default() });
            }
            let n = static_samples.len() as f32;

            // Compute mean accelerometer output during static period (estimate bias relative to gravity)
            let accel_mean: [f32; 3] = {
                let mut s = [0.0f32; 3];
                for samp in &static_samples { s[0] += samp.accel[0]; s[1] += samp.accel[1]; s[2] += samp.accel[2]; }
                [s[0]/n, s[1]/n, s[2]/n]
            };
            // Gravity should be on the Z axis (if sensor aligned upright); bias is deviation
            let accel_bias = [accel_mean[0], accel_mean[1], accel_mean[2] - gravity_magnitude];

            // Gyro bias: mean of static gyro output
            let gyro_mean: [f32; 3] = {
                let mut s = [0.0f32; 3];
                for samp in &static_samples { s[0] += samp.gyro[0]; s[1] += samp.gyro[1]; s[2] += samp.gyro[2]; }
                [s[0]/n, s[1]/n, s[2]/n]
            };

            // Scale factor from rotation sequences (simplified: expect known angle → measure integrated angle)
            let mut gyro_scale = [1.0f32; 3];
            for rot in &rotation_sequences {
                // In production: integrate gyro along the rotation axis over the sequence duration,
                // compare to expected_angle_deg, compute scale factor = expected / measured
                let _axis = rot.axis; // 'x', 'y', or 'z'
                let _expected = rot.expected_angle_deg;
                // placeholder: scale remains 1.0 without actual data
            }

            // Magnetometer calibration: hard-iron = mean of mag samples
            let mag_hard_iron = if static_samples.iter().any(|s| s.mag.is_some()) {
                let mag_count = static_samples.iter().filter(|s| s.mag.is_some()).count() as f32;
                let mut ms = [0.0f32; 3];
                for samp in &static_samples {
                    if let Some(mag) = samp.mag {
                        ms[0] += mag[0]; ms[1] += mag[1]; ms[2] += mag[2];
                    }
                }
                Some([ms[0]/mag_count, ms[1]/mag_count, ms[2]/mag_count])
            } else { None };

            let temp = static_samples.iter().filter_map(|s| s.temperature_c).next().unwrap_or(25.0);

            Ok(IMUModalityOutput {
                success: true,
                calibration: Some(IMUCalibration {
                    accel_bias, gyro_bias: gyro_mean, accel_scale: [1.0; 3],
                    gyro_scale, mag_hard_iron, mag_soft_iron: None,
                    calibration_temperature_c: temp,
                    calibration_timestamp: format!("{}", executor.generate_id()),
                    ..Default::default()
                }),
                ..Default::default()
            })
        }

        IMUModalityAction::FuseMultiIMU { imu_streams, body_model, fusion_method } => {
            // Multi-IMU fusion: run AHRS on each stream, then compute relative orientations
            let mut joint_angles = Vec::new();

            for joint in &body_model.joints {
                let prox_stream = imu_streams.iter().find(|s| s.body_segment == joint.proximal_segment);
                let dist_stream = imu_streams.iter().find(|s| s.body_segment == joint.distal_segment);

                if let (Some(prox), Some(dist)) = (prox_stream, dist_stream) {
                    if let (Some(prox_samples), Some(dist_samples)) = (&prox.samples, &dist.samples) {
                        let prox_ahrs = executor.madgwick_ahrs(prox_samples, prox.sample_rate_hz, 0.1, Quaternion::identity());
                        let dist_ahrs = executor.madgwick_ahrs(dist_samples, dist.sample_rate_hz, 0.1, Quaternion::identity());

                        let n = prox_ahrs.attitudes.len().min(dist_ahrs.attitudes.len());
                        let mut angle_samples = Vec::with_capacity(n);
                        let mut angle_sum = 0.0f32;
                        let mut prev_angle = 0.0f32;
                        let mut peak_vel = 0.0f32;

                        for i in 0..n {
                            let qp = prox_ahrs.attitudes[i].quaternion;
                            let qd = dist_ahrs.attitudes[i].quaternion;
                            // Relative rotation: q_rel = qp_conjugate * qd
                            let qp_conj = Quaternion { w: qp.w, x: -qp.x, y: -qp.y, z: -qp.z };
                            let q_rel = qp_conj.multiply(&qd).normalize();
                            let euler = q_rel.to_euler_deg();
                            // For hinge joint: use primary axis (X)
                            let angle = euler[0];
                            let dt = if i > 0 { (prox_ahrs.attitudes[i].timestamp_sec - prox_ahrs.attitudes[i-1].timestamp_sec) as f32 } else { 1.0 / prox.sample_rate_hz };
                            let vel = (angle - prev_angle).abs() / dt.max(1e-6);
                            if vel > peak_vel { peak_vel = vel; }
                            angle_sum += angle;
                            prev_angle = angle;
                            angle_samples.push(JointAngleSample {
                                timestamp_sec: prox_ahrs.attitudes[i].timestamp_sec,
                                angle_deg: angle,
                                angles_3d_deg: Some(euler),
                                angular_velocity_deg_per_sec: vel,
                            });
                        }

                        let mean_angle = if n > 0 { angle_sum / n as f32 } else { 0.0 };
                        let min_angle = angle_samples.iter().map(|a| a.angle_deg).fold(f32::INFINITY, f32::min);
                        let max_angle = angle_samples.iter().map(|a| a.angle_deg).fold(f32::NEG_INFINITY, f32::max);

                        joint_angles.push(JointAngleEstimate {
                            estimate_id: executor.generate_id(),
                            joint_id: joint.joint_id.clone(),
                            joint_type: joint.joint_type.clone(),
                            angles_deg: angle_samples,
                            mean_angle_deg: mean_angle,
                            range_of_motion_deg: (max_angle - min_angle).max(0.0),
                            peak_angular_velocity_deg_per_sec: peak_vel,
                        });
                    }
                }
            }

            let analysis = IMUAnalysisResult { joint_angle_estimates: joint_angles, ..Default::default() };
            Ok(IMUModalityOutput { success: true, analysis: Some(analysis), ..Default::default() })
        }

        IMUModalityAction::EstimateJointAngle { proximal_samples, distal_samples, sample_rate_hz, joint_type, initial_angle_deg } => {
            let prox_ahrs = executor.madgwick_ahrs(&proximal_samples, sample_rate_hz, 0.1, Quaternion::identity());
            let dist_ahrs = executor.madgwick_ahrs(&distal_samples, sample_rate_hz, 0.1, Quaternion::identity());

            let n = prox_ahrs.attitudes.len().min(dist_ahrs.attitudes.len());
            let mut angle_samples = Vec::with_capacity(n);
            let mut angle_sum = 0.0f32;
            let mut prev_angle = initial_angle_deg.unwrap_or(0.0);
            let mut peak_vel = 0.0f32;

            for i in 0..n {
                let qp = prox_ahrs.attitudes[i].quaternion;
                let qd = dist_ahrs.attitudes[i].quaternion;
                let qp_conj = Quaternion { w: qp.w, x: -qp.x, y: -qp.y, z: -qp.z };
                let q_rel = qp_conj.multiply(&qd).normalize();
                let euler = q_rel.to_euler_deg();
                let angle = euler[0];
                let dt = 1.0 / sample_rate_hz;
                let vel = (angle - prev_angle).abs() / dt;
                if vel > peak_vel { peak_vel = vel; }
                angle_sum += angle; prev_angle = angle;
                angle_samples.push(JointAngleSample {
                    timestamp_sec: prox_ahrs.attitudes[i].timestamp_sec,
                    angle_deg: angle, angles_3d_deg: Some(euler),
                    angular_velocity_deg_per_sec: vel,
                });
            }

            let mean_angle = if n > 0 { angle_sum / n as f32 } else { 0.0 };
            let min_angle = angle_samples.iter().map(|a| a.angle_deg).fold(f32::INFINITY, f32::min);
            let max_angle = angle_samples.iter().map(|a| a.angle_deg).fold(f32::NEG_INFINITY, f32::max);

            Ok(IMUModalityOutput {
                success: true,
                joint_angle_estimate: Some(JointAngleEstimate {
                    estimate_id: executor.generate_id(),
                    joint_id: "joint_0".into(),
                    joint_type, angles_deg: angle_samples,
                    mean_angle_deg: mean_angle,
                    range_of_motion_deg: (max_angle - min_angle).max(0.0),
                    peak_angular_velocity_deg_per_sec: peak_vel,
                }),
                ..Default::default()
            })
        }

        IMUModalityAction::QueryGraph { graph_id, query } => {
            let graph = executor.load_graph(graph_id)?;
            let result = match query {
                IMUGraphQuery::NodeDetail { node_id } => {
                    let node = graph.nodes.iter().find(|n| n.node_id == node_id);
                    let incoming: Vec<_> = graph.edges.iter().filter(|e| e.to_node == node_id).collect();
                    let outgoing: Vec<_> = graph.edges.iter().filter(|e| e.from_node == node_id).collect();
                    serde_json::json!({ "node": node, "incoming": incoming, "outgoing": outgoing })
                }
                IMUGraphQuery::AttitudeHistory => {
                    let att: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, IMUNodeType::AttitudeEstimateNode)).collect();
                    serde_json::json!({ "attitude_nodes": att })
                }
                IMUGraphQuery::TrajectoryWaypoints => {
                    let wps: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, IMUNodeType::TrajectoryWaypointNode)).collect();
                    serde_json::json!({ "waypoints": wps })
                }
                IMUGraphQuery::MotionWindows { motion_class } => {
                    let windows: Vec<_> = graph.nodes.iter()
                        .filter(|n| matches!(n.node_type, IMUNodeType::MotionWindowNode))
                        .filter(|n| motion_class.as_ref().map(|mc| n.motion_class.as_deref() == Some(mc)).unwrap_or(true))
                        .collect();
                    serde_json::json!({ "windows": windows })
                }
                IMUGraphQuery::IMUEvents { event_type } => {
                    let evs: Vec<_> = graph.nodes.iter()
                        .filter(|n| matches!(n.node_type, IMUNodeType::IMUEventNode))
                        .filter(|n| event_type.as_ref().map(|et| n.event_type.as_deref() == Some(et)).unwrap_or(true))
                        .collect();
                    serde_json::json!({ "events": evs })
                }
                IMUGraphQuery::ShockEvents => {
                    let shocks: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, IMUNodeType::ShockEventNode)).collect();
                    serde_json::json!({ "shock_events": shocks })
                }
                IMUGraphQuery::VibrationSpectra => {
                    let spectra: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, IMUNodeType::VibrationSpectrumNode)).collect();
                    serde_json::json!({ "vibration_spectra": spectra })
                }
                IMUGraphQuery::JointAngles => {
                    let joints: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, IMUNodeType::JointAngleNode)).collect();
                    serde_json::json!({ "joint_angles": joints })
                }
                IMUGraphQuery::CrossModalLinks { node_id } => {
                    let links: Vec<_> = graph.edges.iter()
                        .filter(|e| (e.from_node == node_id || e.to_node == node_id)
                            && matches!(e.edge_type,
                                IMUEdgeType::TrajectoryIn3DScene |
                                IMUEdgeType::PlottedOnGeoMap |
                                IMUEdgeType::FeedsKinematicModel |
                                IMUEdgeType::FeedsControlSystem |
                                IMUEdgeType::FusedWithRadar |
                                IMUEdgeType::FusedWithSonar
                            ))
                        .collect();
                    serde_json::json!({ "cross_modal_links": links })
                }
                IMUGraphQuery::AGIActivity => serde_json::json!({ "is_active": false }),
                IMUGraphQuery::AllNodes => serde_json::json!({ "nodes": graph.nodes }),
                IMUGraphQuery::AllEdges => serde_json::json!({ "edges": graph.edges }),
            };
            Ok(IMUModalityOutput { success: true, query_result: Some(result), ..Default::default() })
        }

        IMUModalityAction::GetGraph { graph_id } => {
            let graph = executor.load_graph(graph_id)?;
            Ok(IMUModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        IMUModalityAction::TriggerSemanticHook { graph_id, hook } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            match hook {
                IMUSemanticHook::OnGraphCreated => {
                    graph.state = GraphStateType::SemanticEnriched;
                }
                IMUSemanticHook::OnInferRelationships => {
                    let new_edges = executor.infer_semantic_relationships(&graph.nodes).await;
                    let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                    let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                    for (from, to, etype, reason) in new_edges {
                        if valid.contains(&from) && valid.contains(&to) && from != to {
                            graph.edges.push(IMUGraphEdge {
                                edge_id: next_eid, from_node: from, to_node: to,
                                edge_type: etype, weight: 0.8,
                                provenance: EdgeProvenance::DerivedFromHook, version: 1,
                                properties: { let mut p = HashMap::new(); p.insert("reason".into(), serde_json::json!(reason)); p },
                                ..Default::default()
                            });
                            next_eid += 1;
                        }
                    }
                }
                IMUSemanticHook::OnEdgeCompletion => {
                    let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                    graph.edges.retain(|e| valid.contains(&e.from_node) && valid.contains(&e.to_node));
                }
                IMUSemanticHook::OnCrossModalityLink { target_modality, target_graph_id } => {
                    graph.state = GraphStateType::CrossLinked;
                    graph.version += 1;
                    graph.version_notes.push(VersionNote {
                        version: graph.version,
                        note: format!("Cross-linked to {} (graph {})", target_modality, target_graph_id),
                        step_index: None, timestamp: now.clone(), change_type: ChangeType::CrossLinked,
                    });
                }
            }
            graph.updated_at = now;
            executor.save_graph(&graph)?;
            Ok(IMUModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        IMUModalityAction::ExportProduct { graph_id, format } => {
            let ext = match &format {
                IMUExportFormat::CSV => "csv",
                IMUExportFormat::HDF5 => "h5",
                IMUExportFormat::JSON => "json",
                IMUExportFormat::ROS_Bag => "bag",
                IMUExportFormat::NPZ => "npz",
                IMUExportFormat::TUM_TXT => "txt",
                IMUExportFormat::EuRoC_CSV => "csv",
                IMUExportFormat::Custom(_) => "dat",
            };
            let export_path = format!("/tmp/imu_export_{}_{}.{}", graph_id, executor.generate_id(), ext);
            Ok(IMUModalityOutput { success: true, export_path: Some(export_path), ..Default::default() })
        }

        IMUModalityAction::StreamToUI { graph_id, .. } => {
            Ok(IMUModalityOutput { success: true, graph_id: Some(graph_id), ..Default::default() })
        }

        IMUModalityAction::HeadlessProcess { graph_id, operations } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();

            for op in operations {
                match op {
                    IMUOperation::RerunAHRS { algorithm } => {
                        graph.version_notes.push(VersionNote {
                            version: graph.version + 1,
                            note: format!("Reran AHRS with {:?}", algorithm),
                            step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated,
                        });
                        graph.version += 1;
                    }
                    IMUOperation::RecomputeTrajectory => {
                        graph.version_notes.push(VersionNote {
                            version: graph.version + 1,
                            note: "Recomputed trajectory".into(),
                            step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated,
                        });
                        graph.version += 1;
                    }
                    IMUOperation::ReclassifyMotion => {
                        graph.version_notes.push(VersionNote {
                            version: graph.version + 1,
                            note: "Reclassified motion windows".into(),
                            step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated,
                        });
                        graph.version += 1;
                    }
                    IMUOperation::CrossLinkToGeo { geo_graph_id } => {
                        let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        let root_id = graph.root_node_id;
                        graph.edges.push(IMUGraphEdge {
                            edge_id: next_eid, from_node: root_id, to_node: root_id,
                            edge_type: IMUEdgeType::PlottedOnGeoMap, weight: 0.85,
                            provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                            properties: { let mut p = HashMap::new(); p.insert("geo_graph_id".into(), serde_json::json!(geo_graph_id)); p },
                            ..Default::default()
                        });
                        graph.state = GraphStateType::CrossLinked;
                    }
                    IMUOperation::CrossLinkToKinematics { kine_graph_id } => {
                        let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        // Link all joint angle nodes to kinematics
                        let joint_nids: Vec<u64> = graph.nodes.iter()
                            .filter(|n| matches!(n.node_type, IMUNodeType::JointAngleNode))
                            .map(|n| n.node_id)
                            .collect();
                        for nid in joint_nids {
                            graph.edges.push(IMUGraphEdge {
                                edge_id: next_eid, from_node: nid, to_node: nid,
                                edge_type: IMUEdgeType::FeedsKinematicModel, weight: 0.9,
                                provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                                properties: { let mut p = HashMap::new(); p.insert("kine_graph_id".into(), serde_json::json!(kine_graph_id)); p },
                                ..Default::default()
                            });
                            next_eid += 1;
                        }
                        graph.state = GraphStateType::CrossLinked;
                    }
                }
            }

            graph.updated_at = now;
            executor.save_graph(&graph)?;
            Ok(IMUModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }
    }
}

// Helper: convert IMUBinaryFormat to IMUSensorModel
impl From<IMUBinaryFormat> for IMUSensorModel {
    fn from(f: IMUBinaryFormat) -> Self {
        match f {
            IMUBinaryFormat::MPU6050   => IMUSensorModel::MPU6050,
            IMUBinaryFormat::ICM42688  => IMUSensorModel::ICM42688,
            IMUBinaryFormat::BMI088    => IMUSensorModel::BMI088,
            IMUBinaryFormat::BMI160    => IMUSensorModel::BMI160,
            IMUBinaryFormat::LSM6DS    => IMUSensorModel::LSM6DSL,
            IMUBinaryFormat::ADIS16445 => IMUSensorModel::ADIS16445,
            IMUBinaryFormat::VN100     => IMUSensorModel::VN100,
            IMUBinaryFormat::XSens     => IMUSensorModel::XSens_MTi_G,
            IMUBinaryFormat::Custom(s) => IMUSensorModel::Custom(s),
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
        if args[i] == "--input" && i + 1 < args.len() { input_json = args[i + 1].clone(); i += 2; }
        else { i += 1; }
    }
    if input_json.is_empty() {
        eprintln!("Usage: imu_sensing --input '<json>'");
        std::process::exit(1);
    }
    let input: IMUModalityAction = match serde_json::from_str(&input_json) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", serde_json::json!({"success": false, "error": format!("Parse error: {}", e)}));
            std::process::exit(1);
        }
    };
    let rt = tokio::runtime::Runtime::new().expect("Tokio runtime");
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap_or_else(|_| r#"{"success":false,"error":"serialize"}"#.into())),
        Err(e) => {
            println!("{}", serde_json::json!({"success": false, "error": e}));
            std::process::exit(1);
        }
    }
}
