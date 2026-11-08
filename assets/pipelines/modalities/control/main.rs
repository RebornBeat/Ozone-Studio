//! ControlSystems — Pipeline #122
//!
//! Classical and modern control theory: plant modeling, controller design,
//! stability analysis, transfer functions, state-space models, PID tuning,
//! optimal control (LQR/LQG/MPC), adaptive control, feedback/feedforward,
//! system identification, simulation, and verification.
//!
//! DISTINCT FROM:
//!   - Kinematics (121): geometry and motion chains; Control Systems adds
//!     the feedback loop, stability theory, controller synthesis, and
//!     closed-loop dynamics that make systems track and reject disturbances.
//!   - Network Topology (123): communication routing; Control Systems is
//!     the physics-level closed-loop regulation.
//!
//! CROSS-LINKS:
//!   109 (3D)       → robot model ↔ controller (FK/IK feeds plant model)
//!   116 (IMU)      → IMU sensor provides state feedback
//!   121 (Kine)     → kinematic chain is the plant
//!   118 (EM)       → RF link quality affects control loop timing
//!   124 (Radar)    → radar tracks target, feeds guidance controller
//!   125 (Sonar)    → sonar provides AUV navigation feedback
//!   117 (Geo)      → waypoints define reference trajectory
//!   101 (Code)     → control software implementation
//!   105 (Math)     → stability proofs, Lyapunov functions
//!
//! STORAGE: ZSEI containers under /Modalities/ControlSystems/

#[path = "../../shared/link_handler.rs"]
mod link_handler;

#[path = "../../shared/semantic_relations.rs"]
mod semantic_relations;

#[path = "../../shared/capture.rs"]
mod capture;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

#[path = "../../shared/ozone_serve.rs"]
mod ozone_serve;

// ─────────────────────────────────────────────────────────────────────────────
// INPUT TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ControlModalityAction {
    /// Link this modality graph to another modality's graph (bidirectional RelatedTo, idempotent).
    LinkToModality { source_graph_id: u64, target_graph_id: u64, target_modality: String, relationship: serde_json::Value },
    /// Analyze a control system description (model, specs, data)
    Analyze {
        data: ControlDataSource,
        analyze_stability: bool,
        analyze_performance: bool,
        analyze_robustness: bool,
        identify_system: bool,
    },
    /// Create graph from analysis result
    CreateGraph {
        analysis: ControlAnalysisResult,
        project_id: u64,
    },
    /// Update graph with new measurements or design changes
    UpdateGraph {
        graph_id: u64,
        updates: Vec<ControlUpdate>,
        project_id: u64,
    },
    /// Design a controller for a given plant
    DesignController {
        plant_model: PlantModel,
        requirements: ControlRequirements,
        method: ControlDesignMethod,
    },
    /// Tune PID controller parameters
    TunePID {
        plant_model: PlantModel,
        method: PIDTuningMethod,
        performance_spec: PIDPerformanceSpec,
    },
    /// Compute transfer function from state-space model
    StateSpaceToTransferFunction {
        a_matrix: Vec<Vec<f64>>,
        b_matrix: Vec<Vec<f64>>,
        c_matrix: Vec<Vec<f64>>,
        d_matrix: Vec<Vec<f64>>,
    },
    /// Convert transfer function to state-space
    TransferFunctionToStateSpace {
        numerator: Vec<f64>,
        denominator: Vec<f64>,
        form: StateSpaceForm,
    },
    /// Analyze stability of a closed-loop system
    AnalyzeStability {
        system: SystemModel,
        method: StabilityMethod,
    },
    /// Compute frequency response (Bode / Nyquist)
    FrequencyResponse {
        system: SystemModel,
        freq_start_rad_s: f64,
        freq_end_rad_s: f64,
        points: u32,
        response_type: FrequencyResponseType,
    },
    /// Simulate system step response
    SimulateResponse {
        graph_id: u64,
        input_type: SimulationInput,
        duration_sec: f64,
        dt_sec: f64,
        initial_state: Option<Vec<f64>>,
        /// Plant to simulate. When absent, a PlantNode whose `content` is a StateSpaceModel JSON is used;
        /// otherwise the action fails honestly (no default plant is ever substituted).
        #[serde(default)]
        state_space: Option<StateSpaceModel>,
    },
    /// Design LQR optimal controller
    DesignLQR {
        a_matrix: Vec<Vec<f64>>,
        b_matrix: Vec<Vec<f64>>,
        q_matrix: Vec<Vec<f64>>,
        r_matrix: Vec<Vec<f64>>,
    },
    /// Design Model Predictive Controller
    DesignMPC {
        plant_model: PlantModel,
        prediction_horizon: u32,
        control_horizon: u32,
        constraints: MPCConstraints,
        objective: MPCObjective,
    },
    /// Identify system from input/output data
    IdentifySystem {
        input_data: Vec<f64>,
        output_data: Vec<f64>,
        dt_sec: f64,
        method: SystemIdentificationMethod,
        model_order: Option<u32>,
    },
    /// Verify control system properties (stability, performance)
    Verify {
        graph_id: u64,
        specifications: Vec<ControlSpecification>,
    },
    /// Query graph
    QueryGraph {
        graph_id: u64,
        query: ControlGraphQuery,
    },
    /// Retrieve full graph for Context Viewer
    GetGraph { graph_id: u64 },
    /// Trigger ZSEI semantic hooks
    TriggerSemanticHook {
        graph_id: u64,
        hook: ControlSemanticHook,
    },
    /// Export control system products
    ExportProduct {
        graph_id: u64,
        format: ControlExportFormat,
    },
    /// Stream to UI
    StreamToUI {
        graph_id: u64,
        session_id: String,
        display_mode: ControlDisplayMode,
    },
    /// Headless processing (AGI-first)
    HeadlessProcess {
        graph_id: u64,
        operations: Vec<ControlOperation>,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// DATA SOURCES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlDataSource {
    /// MATLAB/Octave .mat or .m file containing system matrices
    MatlabFile {
        file_path: String,
        system_variable: Option<String>,
    },
    /// Simulink model file
    Simulink {
        file_path: String,
        subsystem_path: Option<String>,
    },
    /// Modelica / FMU model
    ModelicaFMU {
        file_path: String,
        model_name: String,
    },
    /// Python control library pickle or numpy arrays
    PythonControl {
        file_path: String,
        format: PythonControlFormat,
    },
    /// Raw state-space matrices (JSON or CSV)
    StateSpaceMatrices {
        a_file: String,
        b_file: String,
        c_file: String,
        d_file: String,
        dt_sec: Option<f64>,  // None = continuous time
    },
    /// Transfer function coefficients (JSON)
    TransferFunctionCoefficients {
        file_path: String,
        is_discrete: bool,
        sample_time: Option<f64>,
    },
    /// Time-series measurement data (input/output pairs)
    TimeSeries {
        file_path: String,
        format: TimeSeriesFormat,
        input_columns: Vec<String>,
        output_columns: Vec<String>,
        sample_rate_hz: f64,
    },
    /// URDF/MJCF robot description (plant = robot)
    RobotDescription {
        file_path: String,
        format: RobotDescriptionFormat,
        end_effector: Option<String>,
    },
    /// ROS2 bag file (recorded sensor/actuator data)
    ROS2Bag {
        file_path: String,
        topics: Vec<String>,
    },
    /// Inline plant model specification
    InlinePlant { plant: PlantModel },
    /// Live telemetry stream
    LiveStream { endpoint: String, protocol: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PythonControlFormat { Pickle, Numpy_NPZ, JSON, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeSeriesFormat { CSV, HDF5, Parquet, ROS2_CSV, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RobotDescriptionFormat { URDF, MJCF, SDF, Custom(String) }

// ─────────────────────────────────────────────────────────────────────────────
// CORE MODEL TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlantModel {
    pub plant_id: u64,
    pub name: String,
    pub model_type: PlantModelType,
    pub n_states: u32,
    pub n_inputs: u32,
    pub n_outputs: u32,
    pub is_discrete: bool,
    pub sample_time_sec: Option<f64>,
    pub state_space: Option<StateSpaceModel>,
    pub transfer_functions: Vec<TransferFunctionModel>,
    pub nonlinear_description: Option<String>,
    pub operating_point: Option<OperatingPoint>,
    pub physical_domain: PhysicalDomain,
    pub units: PlantUnits,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PlantModelType {
    #[default] Unknown,
    LinearTimeInvariant,
    LinearTimeVarying,
    Nonlinear,
    PiecewiseAffine,
    HybridSystem,
    DiscreteLTI,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StateSpaceModel {
    pub a_matrix: Vec<Vec<f64>>,     // n_states × n_states
    pub b_matrix: Vec<Vec<f64>>,     // n_states × n_inputs
    pub c_matrix: Vec<Vec<f64>>,     // n_outputs × n_states
    pub d_matrix: Vec<Vec<f64>>,     // n_outputs × n_inputs
    pub state_names: Vec<String>,
    pub input_names: Vec<String>,
    pub output_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TransferFunctionModel {
    pub tf_id: u64,
    pub name: String,
    pub input_index: u32,
    pub output_index: u32,
    pub numerator: Vec<f64>,         // highest power first
    pub denominator: Vec<f64>,
    pub is_discrete: bool,
    pub sample_time: Option<f64>,
    pub poles: Vec<ComplexNumber>,
    pub zeros: Vec<ComplexNumber>,
    pub dc_gain: Option<f64>,
    pub relative_degree: i32,        // degree(denominator) - degree(numerator)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComplexNumber {
    pub real: f64,
    pub imag: f64,
}

impl ComplexNumber {
    fn magnitude(&self) -> f64 { (self.real * self.real + self.imag * self.imag).sqrt() }
    fn phase_deg(&self) -> f64 { self.imag.atan2(self.real).to_degrees() }
    fn is_stable_pole(&self) -> bool { self.real < 0.0 }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OperatingPoint {
    pub state_equilibrium: Vec<f64>,
    pub input_equilibrium: Vec<f64>,
    pub output_equilibrium: Vec<f64>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PhysicalDomain {
    #[default] Generic,
    Mechanical, Electrical, Hydraulic, Thermal, Chemical,
    Robotic, Aerospace, Automotive, ProcessControl,
    PowerElectronics, BioMedical, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlantUnits {
    pub states: Vec<String>,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum StateSpaceForm {
    #[default] Controllable, Observable, Balanced, Jordan, Custom(String),
}

// ─────────────────────────────────────────────────────────────────────────────
// CONTROLLER TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Controller {
    pub controller_id: u64,
    pub name: String,
    pub controller_type: ControllerType,
    pub n_inputs: u32,
    pub n_outputs: u32,
    pub is_discrete: bool,
    pub sample_time_sec: Option<f64>,
    pub parameters: ControllerParameters,
    pub state_space: Option<StateSpaceModel>,
    pub transfer_functions: Vec<TransferFunctionModel>,
    pub design_method: String,
    pub performance_achieved: Option<ClosedLoopPerformance>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ControllerType {
    #[default] Unknown, PID, PI, PD, P, Lead, Lag, LeadLag,
    LQR, LQG, H_Infinity, H_Two, MPC, Adaptive, Sliding_Mode,
    Feedforward, Cascade, Decoupling, Fuzzy, NeuralNetwork,
    GainScheduled, Robust_H_Inf, L1_Adaptive, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ControllerParameters {
    // PID parameters
    pub kp: Option<f64>,
    pub ki: Option<f64>,
    pub kd: Option<f64>,
    pub n_filter: Option<f64>,         // derivative filter coefficient
    pub anti_windup: Option<AntiWindupType>,
    pub integral_limit: Option<(f64, f64)>,

    // Lead/Lag parameters
    pub gain: Option<f64>,
    pub zero_rad_s: Option<f64>,
    pub pole_rad_s: Option<f64>,

    // LQR parameters
    pub q_matrix: Option<Vec<Vec<f64>>>,
    pub r_matrix: Option<Vec<Vec<f64>>>,
    pub lqr_gain: Option<Vec<Vec<f64>>>,

    // H-infinity
    pub gamma: Option<f64>,

    // Gain schedule breakpoints
    pub schedule_variable: Option<String>,
    pub schedule_breakpoints: Option<Vec<f64>>,
    pub schedule_gains: Option<Vec<Vec<f64>>>,

    // Generic custom parameters
    pub custom: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum AntiWindupType { #[default] None, Clamping, BackCalculation, ConditionalIntegration }

// ─────────────────────────────────────────────────────────────────────────────
// SENSORS AND ACTUATORS
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Sensor {
    pub sensor_id: u64,
    pub name: String,
    pub sensor_type: SensorType,
    pub measured_quantity: String,
    pub measurement_range: (f64, f64),
    pub noise_std: f64,
    pub sample_rate_hz: f64,
    pub latency_ms: f64,
    pub transfer_function: Option<TransferFunctionModel>,
    pub quantization_bits: Option<u32>,
    pub cross_modal_source: Option<String>,   // e.g. "imu:116", "radar:124", "sonar:125"
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SensorType {
    #[default] Generic, Encoder, IMU, GPS, LiDAR, Camera, ForceTorque,
    PressureTransducer, TemperatureSensor, StrainGauge, Potentiometer,
    Tachometer, Accelerometer, Gyroscope, Magnetometer, Radar, Sonar,
    ProximitySensor, CurrentSensor, VoltageSensor, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Actuator {
    pub actuator_id: u64,
    pub name: String,
    pub actuator_type: ActuatorType,
    pub controlled_quantity: String,
    pub output_range: (f64, f64),
    pub bandwidth_hz: f64,
    pub saturation_limits: (f64, f64),
    pub rate_limit: Option<f64>,
    pub dead_zone: Option<f64>,
    pub backlash: Option<f64>,
    pub transfer_function: Option<TransferFunctionModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ActuatorType {
    #[default] Generic, DCMotor, BrushlessMotor, ServoMotor, Hydraulic,
    Pneumatic, LinearActuator, Solenoid, Thruster, ControlSurface,
    Valve, Heater, Cooler, VFD, StepperMotor, PiezoActuator, Custom(String),
}

// ─────────────────────────────────────────────────────────────────────────────
// LOOP ARCHITECTURE
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FeedbackLoop {
    pub loop_id: u64,
    pub name: String,
    pub loop_type: LoopType,
    pub plant_id: u64,
    pub controller_id: u64,
    pub sensor_ids: Vec<u64>,
    pub actuator_ids: Vec<u64>,
    pub reference_signal: ReferenceSignal,
    pub disturbance_channels: Vec<DisturbanceChannel>,
    pub noise_channels: Vec<NoiseChannel>,
    pub loop_delay_ms: f64,
    pub sample_rate_hz: f64,
    pub is_inner_loop: bool,
    pub outer_loop_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum LoopType {
    #[default] SingleInput_SingleOutput,
    MultiInput_MultiOutput,
    Cascade, Feedforward, IMC, Smith_Predictor,
    Decoupled, Adaptive, Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReferenceSignal {
    pub signal_type: String,         // "step", "ramp", "sinusoidal", "trajectory"
    pub amplitude: f64,
    pub units: String,
    pub source: Option<String>,      // "operator", "trajectory_planner", "waypoint_list"
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DisturbanceChannel {
    pub name: String,
    pub entry_point: DisturbanceEntryPoint,
    pub psd_model: Option<String>,   // power spectral density model description
    pub max_amplitude: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum DisturbanceEntryPoint { #[default] PlantInput, PlantOutput, SensorOutput }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NoiseChannel {
    pub sensor_id: u64,
    pub noise_type: NoiseType,
    pub variance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum NoiseType { #[default] Gaussian_White, Colored, Quantization, Bias }

// ─────────────────────────────────────────────────────────────────────────────
// STABILITY AND PERFORMANCE
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StabilityAnalysis {
    pub analysis_id: u64,
    pub system_name: String,
    pub method: StabilityMethod,
    pub is_stable: bool,
    pub stability_type: StabilityType,
    pub poles: Vec<Pole>,
    pub gain_margin_db: Option<f64>,
    pub phase_margin_deg: Option<f64>,
    pub gain_crossover_freq_rad_s: Option<f64>,
    pub phase_crossover_freq_rad_s: Option<f64>,
    pub delay_margin_sec: Option<f64>,
    pub disk_margin: Option<f64>,
    pub lyapunov_function: Option<String>,
    pub routh_array: Option<Vec<Vec<f64>>>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum StabilityMethod {
    #[default] EigenvalueAnalysis, RouthHurwitz, NyquistCriterion, BodeAnalysis,
    LyapunovDirect, LyapunovIndirect, CircleCriterion, Popov, InputToState,
    SumOfSquares, DiskMargin, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum StabilityType {
    #[default] Unknown, AsymptoticallyStable, BIBO_Stable, MarginallySatble, Unstable,
    ExponentiallyStable, InputToStateStable,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Pole {
    pub value: ComplexNumber,
    pub is_stable: bool,
    pub natural_freq_rad_s: f64,
    pub damping_ratio: f64,
    pub decay_rate: f64,
    pub is_dominant: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClosedLoopPerformance {
    pub performance_id: u64,
    pub rise_time_sec: Option<f64>,
    pub settling_time_sec: Option<f64>,
    pub overshoot_percent: Option<f64>,
    pub peak_time_sec: Option<f64>,
    pub steady_state_error: Option<f64>,
    pub bandwidth_hz: Option<f64>,
    pub control_effort_rms: Option<f64>,
    pub ise: Option<f64>,            // integral squared error
    pub iae: Option<f64>,            // integral absolute error
    pub itae: Option<f64>,           // integral time-weighted absolute error
    pub isv: Option<f64>,            // integral of squared variation (control)
    pub meets_spec: bool,
    pub violations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FrequencyResponseData {
    pub freq_response_id: u64,
    pub frequencies_rad_s: Vec<f64>,
    pub magnitude_db: Vec<f64>,
    pub phase_deg: Vec<f64>,
    pub response_type: FrequencyResponseType,
    pub bandwidth_3db_rad_s: Option<f64>,
    pub resonant_freq_rad_s: Option<f64>,
    pub resonant_peak_db: Option<f64>,
    pub roll_off_db_per_decade: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum FrequencyResponseType { #[default] OpenLoop, ClosedLoop, Sensitivity, ComplementarySensitivity, LoadDisturbance }

// ─────────────────────────────────────────────────────────────────────────────
// DESIGN TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ControlRequirements {
    pub rise_time_max_sec: Option<f64>,
    pub settling_time_max_sec: Option<f64>,
    pub overshoot_max_percent: Option<f64>,
    pub steady_state_error_max: Option<f64>,
    pub bandwidth_min_hz: Option<f64>,
    pub gain_margin_min_db: Option<f64>,
    pub phase_margin_min_deg: Option<f64>,
    pub disturbance_rejection_db: Option<f64>,
    pub noise_attenuation_db: Option<f64>,
    pub robustness_margin: Option<f64>,
    pub sample_rate_hz: Option<f64>,
    pub actuation_limit: Option<f64>,
    pub custom: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ControlDesignMethod {
    #[default] PoleZeroPlacement, RootLocus, FrequencyDomain_Bode, Ziegler_Nichols,
    LQR, LQG, H_Infinity, H_Two, MPC_Design, Robust, GainScheduling,
    Flatness_Based, Backstepping, Sliding_Mode, Adaptive_MIT, Adaptive_MRAS,
    Reinforcement_Learning, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PIDTuningMethod {
    #[default] ZieglerNichols_Step, ZieglerNichols_Ultimate, CohenCoon,
    IMC_Based, Lambda_Tuning, ITAE_Optimal, ISE_Optimal, CHR, Tyreus_Luyben,
    AutoTune_Relay, Model_Based, Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PIDPerformanceSpec {
    pub target_settling_time_sec: Option<f64>,
    pub target_overshoot_percent: Option<f64>,
    pub target_rise_time_sec: Option<f64>,
    pub disturbance_rejection_weight: f64,
    pub noise_sensitivity_weight: f64,
    pub anti_windup: bool,
    pub derivative_filter: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// MPC TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MPCConstraints {
    pub state_lower: Option<Vec<f64>>,
    pub state_upper: Option<Vec<f64>>,
    pub input_lower: Vec<f64>,
    pub input_upper: Vec<f64>,
    pub input_rate_lower: Option<Vec<f64>>,
    pub input_rate_upper: Option<Vec<f64>>,
    pub output_lower: Option<Vec<f64>>,
    pub output_upper: Option<Vec<f64>>,
    pub terminal_constraint: bool,
    pub terminal_set: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MPCObjective {
    pub q_state_weights: Vec<f64>,       // running cost on state deviation
    pub r_input_weights: Vec<f64>,       // running cost on control effort
    pub p_terminal_weights: Vec<f64>,    // terminal cost weights
    pub delta_u_weights: Option<Vec<f64>>, // penalise control changes
    pub soft_constraint_penalty: Option<f64>,
}

// ─────────────────────────────────────────────────────────────────────────────
// SYSTEM IDENTIFICATION
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SystemIdentificationMethod {
    #[default] ARX, ARMAX, OE, BJ_Box_Jenkins, N4SID_Subspace,
    PEM, MOESP, Hammerstein, Wiener, NARX, SINDy, Neural_SS,
    CorrelationAnalysis, FrequencyDomain_Empirical,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IdentifiedModel {
    pub model_id: u64,
    pub method: SystemIdentificationMethod,
    pub model_order: u32,
    pub fit_percent: f64,          // Normalized RMSE as percentage
    pub aic: f64,                  // Akaike information criterion
    pub bic: f64,                  // Bayesian information criterion
    pub state_space: Option<StateSpaceModel>,
    pub transfer_function: Option<TransferFunctionModel>,
    /// Ljung-Box whiteness p-value; None = not computed (never a fabricated value).
    pub residuals_whiteness_p_value: Option<f64>,
    pub cross_validation_fit_percent: Option<f64>,
}

// ─────────────────────────────────────────────────────────────────────────────
// SIMULATION
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimulationInput {
    Step { amplitude: f64, step_time_sec: f64 },
    Ramp { slope: f64 },
    Sinusoidal { amplitude: f64, frequency_hz: f64 },
    ImpulseAt { time_sec: f64, amplitude: f64 },
    Arbitrary { time: Vec<f64>, values: Vec<f64> },
    White_Noise { variance: f64, seed: u64 },
}

// Manual Default: #[derive(Default)] can't mark a data-carrying variant
// (#[default] is unit-only), but the containing request struct needs one —
// a unit 1.0-amplitude step at t=0 is the canonical control-theory input.
impl Default for SimulationInput {
    fn default() -> Self {
        Self::Step { amplitude: 1.0, step_time_sec: 0.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SimulationResult {
    pub sim_id: u64,
    pub time: Vec<f64>,
    pub state_trajectory: Vec<Vec<f64>>,    // [time_step][state_index]
    pub output_trajectory: Vec<Vec<f64>>,   // [time_step][output_index]
    pub control_trajectory: Vec<Vec<f64>>,  // [time_step][input_index]
    pub performance: ClosedLoopPerformance,
    pub diverged: bool,
    pub diverge_time: Option<f64>,
}

// ─────────────────────────────────────────────────────────────────────────────
// VERIFICATION
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ControlSpecification {
    pub spec_id: u64,
    pub spec_type: ControlSpecType,
    pub parameter: String,
    pub bound_type: BoundType,
    pub bound_value: f64,
    pub units: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ControlSpecType {
    #[default] Stability, GainMargin, PhaseMargin, Bandwidth, Overshoot,
    SettlingTime, RiseTime, SteadyStateError, ControlEffort, Robustness,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum BoundType { #[default] Maximum, Minimum, Equality }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VerificationResult {
    pub spec_id: u64,
    pub passed: bool,
    pub measured_value: f64,
    pub bound_value: f64,
    pub margin: f64,
    pub notes: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// ANALYSIS RESULT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ControlAnalysisResult {
    pub analysis_id: u64,
    pub source_description: String,
    pub system_name: String,
    pub physical_domain: PhysicalDomain,

    // MODELS
    pub plants: Vec<PlantModel>,
    pub controllers: Vec<Controller>,
    pub sensors: Vec<Sensor>,
    pub actuators: Vec<Actuator>,
    pub feedback_loops: Vec<FeedbackLoop>,
    pub identified_models: Vec<IdentifiedModel>,

    // ANALYSIS
    pub stability_analyses: Vec<StabilityAnalysis>,
    pub frequency_responses: Vec<FrequencyResponseData>,
    pub simulation_results: Vec<SimulationResult>,
    pub verification_results: Vec<VerificationResult>,

    // DESIGN
    pub designed_controllers: Vec<Controller>,
    pub mpc_problems: Vec<MPCProblem>,
    pub lqr_solutions: Vec<LQRSolution>,

    // METADATA
    pub processing_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MPCProblem {
    pub mpc_id: u64,
    pub prediction_horizon: u32,
    pub control_horizon: u32,
    pub constraints: MPCConstraints,
    pub objective: MPCObjective,
    pub controller_id: u64,
    pub solve_time_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LQRSolution {
    pub lqr_id: u64,
    pub feedback_gain: Vec<Vec<f64>>,
    pub riccati_solution: Vec<Vec<f64>>,
    pub closed_loop_poles: Vec<ComplexNumber>,
    pub optimal_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemModel {
    pub name: String,
    pub state_space: Option<StateSpaceModel>,
    pub transfer_function: Option<TransferFunctionModel>,
    pub is_discrete: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH NODE TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ControlNodeType {
    #[default] ControlSystem,          // root: entire control architecture
    PlantNode,                         // physical system being controlled
    ControllerNode,                    // control law / algorithm
    SensorNode,                        // measurement device
    ActuatorNode,                      // execution device
    FeedbackLoopNode,                  // closed-loop architecture
    SetPointNode,                      // reference/desired value
    TransferFunctionNode,              // TF or state-space block
    PoleNode,                          // individual pole in complex plane
    ZeroNode,                          // individual zero in complex plane
    StabilityAnalysisNode,             // stability result
    FrequencyResponseNode,             // Bode/Nyquist data
    SimulationResultNode,              // step/impulse response
    VerificationResultNode,            // spec compliance check
    DisturbanceNode,                   // disturbance source
    NoiseSourceNode,                   // measurement noise
    IdentifiedModelNode,               // system-identified model
    LQRSolutionNode,                   // LQR optimal gain
    MPCProblemNode,                    // MPC formulation
    CrossModalRef,                     // reference to another modality
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ControlGraphNode {
    pub node_id: u64,
    pub node_type: ControlNodeType,
    pub content: String,

    // CONTROL-SPECIFIC
    pub n_states: Option<u32>,
    pub n_inputs: Option<u32>,
    pub n_outputs: Option<u32>,
    pub is_stable: Option<bool>,
    pub bandwidth_hz: Option<f64>,
    pub gain_margin_db: Option<f64>,
    pub phase_margin_deg: Option<f64>,
    pub sample_rate_hz: Option<f64>,
    pub kp: Option<f64>,
    pub ki: Option<f64>,
    pub kd: Option<f64>,
    pub is_discrete: Option<bool>,
    pub physical_domain: Option<String>,

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
pub enum ControlEdgeType {
    // ── STRUCTURAL ──
    #[default] Contains, PartOf, Precedes,

    // ── CONTROL-SPECIFIC ──
    Controls,                    // controller → plant (command signal)
    Stabilizes,                  // controller stabilizes plant
    OutputsTrajectory,           // plant outputs to feedback
    FeedbackFrom,                // feedback signal from sensor to controller
    FeedforwardTo,               // feedforward path to plant
    MeasuredBy,                  // plant/output measured by sensor
    ActuatedBy,                  // plant actuated by actuator
    DisturbedBy,                 // plant disturbed by disturbance
    NoisyFrom,                   // measurement corrupted by noise
    ClosedLoopWith,              // plant + controller form closed loop
    CascadeInner,                // inner loop in cascade
    CascadeOuter,                // outer loop in cascade
    OutputToPole,                // transfer function has this pole
    OutputToZero,                // transfer function has this zero
    SimulatedBy,                 // system simulated by result
    VerifiedBy,                  // system verified by spec check
    IdentifiedAs,                // plant identified as this model
    OptimalGainFor,              // LQR gain for this plant
    MPCSolvesFor,                // MPC controller for this plant
    SetPointFor,                 // reference signal for this loop
    GainScheduledBy,             // controller gain scheduled by variable
    AnalyzedFor,                 // stability analysis for this system

    // ── CROSS-MODAL ──
    /// Robot kinematic chain is the plant (121)
    PlantIsKinematicChain,
    /// IMU sensor feeds state feedback (116)
    FedByIMUSensor,
    /// 3D scene robot receives control commands (109)
    CommandsRobot3D,
    /// Radar target tracks feed guidance (124)
    TrackFromRadar,
    /// Sonar navigation feedback for AUV (125)
    NavigationFromSonar,
    /// Waypoint reference from geospatial (117)
    WaypointFromGeo,
    /// Controller code in code graph (101)
    ImplementedInCode,
    /// Stability proof in math graph (105)
    ProvenInMath,
    /// RF link quality from EM (118)
    LinkQualityFromEM,

    // ── UNIVERSAL SEMANTIC ──
    Performs, Affects, Implies, Contradicts, Elaborates, Summarizes, Supports,
    TemporalPrecedes, TemporalFollows, CausedBy, Enables, Prevents,
    FunctionalRole, InstanceOf,
    DerivedFrom, VersionOf, RefinesTo, ForkedFrom, SimilarTo,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ControlGraphEdge {
    pub edge_id: u64,
    pub from_node: u64, pub to_node: u64,
    pub edge_type: ControlEdgeType,
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
pub struct ControlGraph {
    pub graph_id: u64, pub project_id: u64,
    pub source_description: String,
    pub nodes: Vec<ControlGraphNode>,
    pub edges: Vec<ControlGraphEdge>,
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
pub enum ControlGraphQuery {
    NodeDetail { node_id: u64 },
    ClosedLoopPoles,
    StabilityResults,
    PerformanceSummary,
    CrossModalLinks { node_id: u64 },
    PIDControllers,
    ActuatorSaturation,
    SensorList,
    VerificationResults,
    AGIActivity, AllNodes, AllEdges,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlSemanticHook {
    OnGraphCreated,
    OnInferRelationships,
    OnEdgeCompletion,
    OnCrossModalityLink { target_modality: String, target_graph_id: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlExportFormat {
    MATLAB_M,           // MATLAB .m script
    Python_Control,     // Python control library
    Simulink_SLX,       // Simulink model
    ACADO,              // ACADO optimal control
    Casadi_Python,      // CasADi for MPC
    JSON_Schema,        // JSON serialization
    CSV_Bode,           // Bode plot data
    CSV_StepResponse,   // Step response data
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlDisplayMode {
    BlockDiagram,       // control loop block diagram
    BodePlot,           // frequency response
    NyquistPlot,        // Nyquist diagram
    RootLocusPlot,      // root locus
    StepResponse,       // time response
    PoleZeroMap,        // pole-zero plot
    StateTraj,          // state trajectory
    ControlEffort,      // actuator signal over time
    SensitivityFunctions, // S, T, CS functions
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlOperation {
    ReanalyzeStability,
    RetunePID { method: PIDTuningMethod },
    ResimulateStep,
    CrossLinkToKinematics { kin_graph_id: u64 },
    CrossLinkToIMU { imu_graph_id: u64 },
    ExportToMatlab,
}

// ─────────────────────────────────────────────────────────────────────────────
// UPDATE TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlUpdate {
    UpdatePlant       { plant: PlantModel },
    UpdateController  { controller: Controller },
    AddSensor         { sensor: Sensor },
    UpdateSimResult   { result: SimulationResult },
    AddVerification   { result: VerificationResult },
}

// ─────────────────────────────────────────────────────────────────────────────
// OUTPUT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ControlModalityOutput {
    pub success: bool,
    /// Set when a graph save failed. The graph built in this run is still returned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub save_error: Option<String>,
    pub graph_id: Option<u64>,
    pub graph: Option<ControlGraph>,
    pub analysis: Option<ControlAnalysisResult>,
    pub designed_controller: Option<Controller>,
    pub stability_result: Option<StabilityAnalysis>,
    pub frequency_response: Option<FrequencyResponseData>,
    pub simulation_result: Option<SimulationResult>,
    pub verification_results: Option<Vec<VerificationResult>>,
    pub identified_model: Option<IdentifiedModel>,
    pub lqr_solution: Option<LQRSolution>,
    pub state_space: Option<StateSpaceModel>,
    pub transfer_function: Option<TransferFunctionModel>,
    pub query_result: Option<serde_json::Value>,
    pub export_path: Option<String>,
    /// Set by the LinkToModality action (shared/link_handler.rs); None for every other action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_result: Option<serde_json::Value>,
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
            prompt_pipeline_path: env::var("OZONE_PROMPT_PIPELINE_PATH").or_else(|_| env::var("OZONE_PROMPT_PIPELINE")).unwrap_or_else(|_| "./pipeline_9".into()),
        }
    }
    async fn llm_zero_shot(&self, prompt: &str, max_tokens: usize) -> Result<String, String> {
        let input = serde_json::json!({"action":"Prompt","prompt":prompt,"max_tokens":max_tokens,"temperature":0.05,"system_prompt":"Control systems analysis. Return only valid JSON."});
        let result: Result<serde_json::Value, String> = std::process::Command::new(&self.prompt_pipeline_path)
            .arg("--input").arg(input.to_string())
            .output()
            .map_err(|e| e.to_string())
            .and_then(|out| serde_json::from_slice::<serde_json::Value>(&out.stdout).map_err(|e| e.to_string()));
        capture::capture_zero_shot_call("control", "llm_zero_shot", prompt, &result);
        let r = result?;
        r["response"].as_str().filter(|s| !s.trim().is_empty()).map(|s| s.to_string()).ok_or_else(|| "prompt pipeline returned no response (not substituting an empty object)".to_string())
    }
    fn save_graph(&self, g: &ControlGraph) -> Result<(), String> {
        let path = format!("{}/local/ctrl_graph_{}.json", self.zsei_path, g.graph_id);
        if let Some(p) = std::path::Path::new(&path).parent() { std::fs::create_dir_all(p).map_err(|e| e.to_string())?; }
        std::fs::write(&path, serde_json::to_string_pretty(g).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }
    fn load_graph(&self, id: u64) -> Result<ControlGraph, String> {
        let path = format!("{}/local/ctrl_graph_{}.json", self.zsei_path, id);
        serde_json::from_str(&std::fs::read_to_string(&path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }
    fn generate_id(&self) -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as u64 }
    fn now_iso8601(&self) -> String { format!("{}", self.generate_id()) }
    fn extract_json_array(raw: &str) -> String { if let (Some(s), Some(e)) = (raw.find('['), raw.rfind(']')) { raw[s..=e].to_string() } else { "[]".to_string() } }
    fn extract_json_object(raw: &str) -> String { if let (Some(s), Some(e)) = (raw.find('{'), raw.rfind('}')) { raw[s..=e].to_string() } else { "{}".to_string() } }
}

impl PipelineExecutor {
    async fn infer_system_description(&self, analysis: &ControlAnalysisResult) -> String {
        let summary = serde_json::json!({
            "plant_count": analysis.plants.len(),
            "controller_count": analysis.controllers.len(),
            "loop_count": analysis.feedback_loops.len(),
            "sensor_count": analysis.sensors.len(),
            "actuator_count": analysis.actuators.len(),
            "physical_domain": format!("{:?}", analysis.physical_domain),
            "plant_name": analysis.plants.first().map(|p| p.name.as_str()).unwrap_or("unknown"),
        });
        let prompt = format!(r#"
Describe in one sentence the purpose of this control system.
{}
Return ONLY the description string, nothing else."#, serde_json::to_string(&summary).unwrap_or_default());
        self.llm_zero_shot(&prompt, 80).await.unwrap_or_else(|_| "Control system".into()).trim().to_string()
    }

    async fn infer_semantic_relationships(&self, nodes: &[ControlGraphNode]) -> Vec<(u64, u64, ControlEdgeType, String)> {
        if nodes.len() < 2 { return vec![]; }
        let node_list: Vec<serde_json::Value> = nodes.iter().take(25).map(|n| serde_json::json!({
            "node_id": n.node_id, "type": format!("{:?}", n.node_type),
            "content": n.content.chars().take(80).collect::<String>(),
            "is_stable": n.is_stable, "bandwidth_hz": n.bandwidth_hz,
        })).collect();

        let prompt = format!(r#"
Identify semantic relationships between these control system graph nodes.

Nodes:
{}

Available edge_type values: Controls, Stabilizes, FeedbackFrom, MeasuredBy, ActuatedBy,
DisturbedBy, ClosedLoopWith, Affects, CausedBy, Enables, DerivedFrom,
FunctionalRole, TemporalPrecedes, SimilarTo

Also give each item "relation" (one of: {}) and "evidence" (a short reason naming what the two nodes share).

Return ONLY valid JSON array:
[{{"from_node_id": N, "to_node_id": M, "edge_type": "TypeName", "reason": "brief", "relation": "RelationName", "evidence": "short reason"}}]"#,
            serde_json::to_string_pretty(&node_list).unwrap_or_default(),
            semantic_relations::ZSEI_RELATIONS.join(", "));

        match self.llm_zero_shot(&prompt, 600).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                let items: Vec<serde_json::Value> = serde_json::from_str(&json_str).unwrap_or_default();
                let proposal = serde_json::json!({"relations": items.iter().map(|v| serde_json::json!({
                    "from": v["from_node_id"].as_u64().map(|x| x.to_string()).unwrap_or_default(),
                    "to": v["to_node_id"].as_u64().map(|x| x.to_string()).unwrap_or_default(),
                    "relation": v["relation"].as_str().unwrap_or(""),
                    "evidence": v["evidence"].as_str().unwrap_or(""),
                })).collect::<Vec<_>>()});
                let ids: Vec<String> = nodes.iter().map(|n| n.node_id.to_string()).collect();
                let validated = semantic_relations::validate_structural(&proposal, &ids);
                // Edges come only from the validated accepted set. A raw proposal
                // the validator rejected does not become an edge — endpoint
                // existence alone (the old gate) is not evidence the relation
                // itself is real.
                let accepted: std::collections::HashSet<(String, String, String)> = validated.accepted.iter()
                    .map(|a| (a.from.clone(), a.to.clone(), a.domain_relation.clone().unwrap_or_else(|| a.relation.clone())))
                    .collect();
                items.iter().filter_map(|v| {
                    let from = v["from_node_id"].as_u64()?;
                    let to = v["to_node_id"].as_u64()?;
                    let label = v["relation"].as_str().unwrap_or("").trim().to_string();
                    if !accepted.contains(&(from.to_string(), to.to_string(), label)) { return None; }
                    let etype = map_ctrl_edge_str(v["edge_type"].as_str().unwrap_or("Affects"));
                    let reason = v["reason"].as_str().unwrap_or("").to_string();
                    Some((from, to, etype, reason))
                }).collect()
            }
            Err(_) => vec![],
        }
    }

    /// Routh-Hurwitz: asymptotically stable iff every first-column entry has the sign of the leading
/// coefficient (no sign changes, no zero entries). A zero pivot or an all-zero row means a root on
/// the imaginary axis or in a symmetric pair: reported as NOT asymptotically stable.
    fn routh_hurwitz(coefficients: &[f64]) -> (bool, Vec<Vec<f64>>) {
        let n = coefficients.len();
        if n < 2 || coefficients[0].abs() < 1e-300 { return (false, vec![]); }
        let lead = coefficients[0].signum();
        let same_sign = coefficients.iter().all(|&c| c * lead > 0.0);
        let rows = n;
        let cols = (n + 1) / 2;
        let mut routh: Vec<Vec<f64>> = vec![vec![0.0; cols]; rows];
        for (i, &c) in coefficients.iter().enumerate() {
            if i % 2 == 0 { routh[0][i / 2] = c; } else { routh[1][i / 2] = c; }
        }
        let mut degenerate = false;
        for row in 2..rows {
            let r1 = row - 2;
            let r2 = row - 1;
            if routh[r2].iter().all(|v| v.abs() < 1e-12) { degenerate = true; break; }
            if routh[r2][0].abs() < 1e-12 { degenerate = true; routh[r2][0] = 1e-12; }
            for col in 0..cols.saturating_sub(1) {
                routh[row][col] = (routh[r2][0] * routh[r1][col + 1] - routh[r1][0] * routh[r2][col + 1]) / routh[r2][0];
            }
        }
        let stable = same_sign && !degenerate && routh.iter().all(|row| row[0] * lead > 0.0);
        (stable, routh)
    }

    /// Compute poles from denominator polynomial coefficients using companion matrix eigenvalues (simplified)
    fn compute_poles_from_denominator(denominator: &[f64]) -> Vec<ComplexNumber> {
        let n = denominator.len().saturating_sub(1);
        if n == 0 { return vec![]; }
        let leading = denominator[0];
        if leading.abs() < 1e-12 { return vec![]; }

        // For 1st and 2nd order, compute analytically
        match n {
            1 => {
                let p = -denominator[1] / leading;
                vec![ComplexNumber { real: p, imag: 0.0 }]
            }
            2 => {
                let a = leading;
                let b = denominator[1];
                let c = denominator[2];
                let discriminant = b * b - 4.0 * a * c;
                if discriminant >= 0.0 {
                    let sqrt_d = discriminant.sqrt();
                    vec![
                        ComplexNumber { real: (-b + sqrt_d) / (2.0 * a), imag: 0.0 },
                        ComplexNumber { real: (-b - sqrt_d) / (2.0 * a), imag: 0.0 },
                    ]
                } else {
                    let sqrt_d = (-discriminant).sqrt();
                    vec![
                        ComplexNumber { real: -b / (2.0 * a), imag:  sqrt_d / (2.0 * a) },
                        ComplexNumber { real: -b / (2.0 * a), imag: -sqrt_d / (2.0 * a) },
                    ]
                }
            }
            _ => {
                // Order ≥ 3: Durand–Kerner root finding; empty (absent) when it does not converge.
                poly_roots(denominator).unwrap_or_default()
            }
        }
    }

    /// Simulate an LTI state-space system with RK4 (input held constant per step = zero-order hold).
    /// The same scalar input signal drives every input channel. Performance metrics are computed
    /// from the output trajectory (10–90% rise time, ±2% settling, peak overshoot, ISE/IAE/ITAE for steps).
    fn simulate_lti(
        ss: &StateSpaceModel,
        input: &SimulationInput,
        duration_sec: f64,
        dt: f64,
        initial_state: Option<&[f64]>,
    ) -> SimulationResult {
        let n_steps = ((duration_sec / dt).ceil() as usize).max(1);
        let n_states = ss.a_matrix.len();
        let n_inputs = ss.b_matrix.first().map(|r| r.len()).unwrap_or(1).max(1);
        let n_outputs = ss.c_matrix.len();

        let mut x = vec![0.0f64; n_states];
        if let Some(x0) = initial_state { for (i, v) in x0.iter().enumerate().take(n_states) { x[i] = *v; } }

        // Deterministic xorshift64* → Box–Muller (seeded; reproducible white noise).
        let mut rng: u64 = match input {
            SimulationInput::White_Noise { seed, .. } => seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407) | 1,
            _ => 1,
        };
        let mut uniform = || -> f64 {
            rng ^= rng >> 12; rng ^= rng << 25; rng ^= rng >> 27;
            let v = rng.wrapping_mul(0x2545F4914F6CDD1D);
            ((v >> 11) as f64 + 1.0) / ((1u64 << 53) as f64 + 1.0)
        };

        let mut time = Vec::with_capacity(n_steps);
        let mut x_traj: Vec<Vec<f64>> = Vec::with_capacity(n_steps);
        let mut y_traj: Vec<Vec<f64>> = Vec::with_capacity(n_steps);
        let mut u_traj: Vec<Vec<f64>> = Vec::with_capacity(n_steps);
        let mut diverged = false;
        let mut diverge_time = None;

        let deriv = |x: &[f64], u: &[f64]| -> Vec<f64> {
            let mut d = vec![0.0f64; n_states];
            for i in 0..n_states {
                for j in 0..n_states.min(ss.a_matrix[i].len()) { d[i] += ss.a_matrix[i][j] * x[j]; }
                if i < ss.b_matrix.len() { for j in 0..n_inputs.min(ss.b_matrix[i].len()) { d[i] += ss.b_matrix[i][j] * u[j]; } }
            }
            d
        };

        for step in 0..n_steps {
            let t = step as f64 * dt;
            time.push(t);
            let u_val = match input {
                SimulationInput::Step { amplitude, step_time_sec } => if t >= *step_time_sec { *amplitude } else { 0.0 },
                SimulationInput::Ramp { slope } => slope * t,
                SimulationInput::Sinusoidal { amplitude, frequency_hz } => amplitude * (2.0 * std::f64::consts::PI * frequency_hz * t).sin(),
                SimulationInput::ImpulseAt { time_sec, amplitude } => if t >= *time_sec && t < *time_sec + dt { amplitude / dt } else { 0.0 },
                SimulationInput::Arbitrary { time: tt, values } => {
                    let m = tt.len().min(values.len());
                    if m == 0 { 0.0 }
                    else if t <= tt[0] { values[0] }
                    else if t >= tt[m - 1] { values[m - 1] }
                    else {
                        let k = (1..m).find(|&k| tt[k] >= t).unwrap_or(m - 1);
                        let span = tt[k] - tt[k - 1];
                        if span.abs() < 1e-300 { values[k] } else { values[k - 1] + (values[k] - values[k - 1]) * (t - tt[k - 1]) / span }
                    }
                }
                SimulationInput::White_Noise { variance, .. } => {
                    let (u1, u2) = (uniform(), uniform());
                    variance.max(0.0).sqrt() * (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
                }
            };
            let u = vec![u_val; n_inputs];

            let mut y = vec![0.0f64; n_outputs];
            for i in 0..n_outputs {
                for j in 0..n_states.min(ss.c_matrix[i].len()) { y[i] += ss.c_matrix[i][j] * x[j]; }
                if i < ss.d_matrix.len() { for j in 0..n_inputs.min(ss.d_matrix[i].len()) { y[i] += ss.d_matrix[i][j] * u[j]; } }
            }
            x_traj.push(x.clone());
            y_traj.push(y);
            u_traj.push(u.clone());

            if x.iter().any(|&v| v.abs() > 1e6 || v.is_nan() || v.is_infinite()) {
                diverged = true;
                diverge_time = Some(t);
                break;
            }

            let k1 = deriv(&x, &u);
            let x2: Vec<f64> = (0..n_states).map(|i| x[i] + 0.5 * dt * k1[i]).collect();
            let k2 = deriv(&x2, &u);
            let x3: Vec<f64> = (0..n_states).map(|i| x[i] + 0.5 * dt * k2[i]).collect();
            let k3 = deriv(&x3, &u);
            let x4: Vec<f64> = (0..n_states).map(|i| x[i] + dt * k3[i]).collect();
            let k4 = deriv(&x4, &u);
            for i in 0..n_states { x[i] += dt / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]); }
        }

        // ── performance metrics (all computed; absent when not defined for the input) ──
        let y1: Vec<f64> = y_traj.iter().map(|y| y.first().copied().unwrap_or(0.0)).collect();
        let u1v: Vec<f64> = u_traj.iter().map(|u| u.first().copied().unwrap_or(0.0)).collect();
        let final_value = y1.last().copied().unwrap_or(0.0);
        let step_amp = match input { SimulationInput::Step { amplitude, .. } => Some(*amplitude), _ => None };
        let measurable = step_amp.is_some() && !diverged && final_value.abs() > 1e-12 && y1.len() > 2;

        let (mut rise_time, mut settling_time, mut overshoot, mut peak_time) = (None, None, None, None);
        if measurable {
            let sgn = final_value.signum();
            let r = final_value.abs();
            let ys: Vec<f64> = y1.iter().map(|v| v * sgn).collect();
            let t10 = ys.iter().position(|&v| v >= 0.1 * r).map(|i| time[i]);
            let t90 = ys.iter().position(|&v| v >= 0.9 * r).map(|i| time[i]);
            rise_time = match (t10, t90) { (Some(a), Some(b)) => Some(b - a), _ => None };
            let band = 0.02 * r;
            let last_out = ys.iter().rposition(|&v| (v - r).abs() > band);
            settling_time = match last_out {
                None => Some(time[0]),
                Some(i) if i + 1 < ys.len() => Some(time[i + 1]),
                Some(_) => None, // still outside the band at the end of the run: did not settle
            };
            let (pi, pk) = ys.iter().enumerate().fold((0usize, f64::NEG_INFINITY), |(bi, bv), (i, &v)| if v > bv { (i, v) } else { (bi, bv) });
            overshoot = Some(((pk - r) / r * 100.0).max(0.0));
            peak_time = Some(time[pi]);
        }
        let (mut steady_state_error, mut ise, mut iae, mut itae) = (None, None, None, None);
        if let (Some(a), false) = (step_amp, diverged) {
            steady_state_error = Some((a - final_value).abs());
            let (mut s_ise, mut s_iae, mut s_itae) = (0.0f64, 0.0f64, 0.0f64);
            for i in 0..y1.len() { let e = a - y1[i]; s_ise += e * e * dt; s_iae += e.abs() * dt; s_itae += time[i] * e.abs() * dt; }
            ise = Some(s_ise); iae = Some(s_iae); itae = Some(s_itae);
        }
        let control_effort_rms = if u1v.is_empty() { None } else { Some((u1v.iter().map(|v| v * v).sum::<f64>() / u1v.len() as f64).sqrt()) };

        SimulationResult {
            sim_id: 0,
            time,
            state_trajectory: x_traj,
            output_trajectory: y_traj,
            control_trajectory: u_traj,
            performance: ClosedLoopPerformance {
                performance_id: 0,
                rise_time_sec: rise_time,
                settling_time_sec: settling_time,
                overshoot_percent: overshoot,
                peak_time_sec: peak_time,
                steady_state_error,
                control_effort_rms,
                ise, iae, itae,
                meets_spec: !diverged,
                violations: if diverged { vec!["System diverged".into()] } else { vec![] },
                ..Default::default()
            },
            diverged,
            diverge_time,
        }
    }

    /// Compute Bode plot data for a transfer function
    fn compute_bode(
        numerator: &[f64],
        denominator: &[f64],
        freq_start: f64,
        freq_end: f64,
        n_points: u32,
    ) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let freqs: Vec<f64> = (0..n_points).map(|i| {
            let t = i as f64 / (n_points - 1).max(1) as f64;
            freq_start * (freq_end / freq_start).powf(t)
        }).collect();

        let mut magnitudes_db = Vec::with_capacity(n_points as usize);
        let mut phases_deg = Vec::with_capacity(n_points as usize);

        for &w in &freqs {
            // Evaluate H(jw) = N(jw) / D(jw)
            let n_deg = numerator.len().saturating_sub(1);
            let d_deg = denominator.len().saturating_sub(1);

            let eval_poly = |coeffs: &[f64], jw_real: f64, jw_imag: f64| -> (f64, f64) {
                let mut re = 0.0f64;
                let mut im = 0.0f64;
                let n = coeffs.len();
                // Horner's method for complex evaluation
                for (k, &c) in coeffs.iter().enumerate() {
                    let power = (n - 1 - k) as i32;
                    // (jw)^power: we track real/imag via repeated multiplication
                    let mut pw_re = 1.0f64;
                    let mut pw_im = 0.0f64;
                    for _ in 0..power {
                        let new_re = pw_re * jw_real - pw_im * jw_imag;
                        let new_im = pw_re * jw_imag + pw_im * jw_real;
                        pw_re = new_re; pw_im = new_im;
                    }
                    re += c * pw_re;
                    im += c * pw_im;
                }
                (re, im)
            };

            let (nr, ni) = eval_poly(numerator, 0.0, w);
            let (dr, di) = eval_poly(denominator, 0.0, w);

            // H = (nr + j*ni) / (dr + j*di)
            let denom_sq = dr * dr + di * di;
            if denom_sq < 1e-30 {
                magnitudes_db.push(f64::INFINITY);
                phases_deg.push(0.0);
            } else {
                let hr = (nr * dr + ni * di) / denom_sq;
                let hi = (ni * dr - nr * di) / denom_sq;
                let magnitude = (hr * hr + hi * hi).sqrt();
                let phase = hi.atan2(hr).to_degrees();
                magnitudes_db.push(20.0 * magnitude.log10().max(-200.0));
                phases_deg.push(phase);
            }
        }

        // Unwrap the phase (atan2 wraps at ±180°; margins need the continuous curve).
        for i in 1..phases_deg.len() {
            while phases_deg[i] - phases_deg[i - 1] > 180.0 { phases_deg[i] -= 360.0; }
            while phases_deg[i] - phases_deg[i - 1] < -180.0 { phases_deg[i] += 360.0; }
        }
        (freqs, magnitudes_db, phases_deg)
    }

    /// Find gain and phase margins from Bode data
    fn find_margins(freqs: &[f64], mag_db: &[f64], phase_deg: &[f64]) -> (Option<f64>, Option<f64>, Option<f64>, Option<f64>) {
        // Phase crossover: frequency where phase = -180 deg
        let mut phase_crossover_freq = None;
        let mut gain_margin_db = None;
        for i in 1..phase_deg.len() {
            if (phase_deg[i - 1] > -180.0 && phase_deg[i] <= -180.0) || (phase_deg[i - 1] < -180.0 && phase_deg[i] >= -180.0) {
                let alpha = (phase_deg[i - 1] + 180.0) / (phase_deg[i - 1] - phase_deg[i]);
                let pc_freq = freqs[i - 1] + alpha * (freqs[i] - freqs[i - 1]);
                let gm = -(mag_db[i - 1] + alpha * (mag_db[i] - mag_db[i - 1]));
                phase_crossover_freq = Some(pc_freq);
                gain_margin_db = Some(gm);
                break;
            }
        }
        // Gain crossover: frequency where |H(jw)| = 0 dB
        let mut gain_crossover_freq = None;
        let mut phase_margin_deg = None;
        for i in 1..mag_db.len() {
            if (mag_db[i - 1] > 0.0 && mag_db[i] <= 0.0) || (mag_db[i - 1] < 0.0 && mag_db[i] >= 0.0) {
                let alpha = mag_db[i - 1] / (mag_db[i - 1] - mag_db[i]);
                let gc_freq = freqs[i - 1] + alpha * (freqs[i] - freqs[i - 1]);
                let phase_at_gc = phase_deg[i - 1] + alpha * (phase_deg[i] - phase_deg[i - 1]);
                gain_crossover_freq = Some(gc_freq);
                phase_margin_deg = Some(phase_at_gc + 180.0);
                break;
            }
        }
        (gain_margin_db, phase_margin_deg, gain_crossover_freq, phase_crossover_freq)
    }
}

fn map_ctrl_edge_str(s: &str) -> ControlEdgeType {
    match s {
        "Controls"              => ControlEdgeType::Controls,
        "Stabilizes"            => ControlEdgeType::Stabilizes,
        "OutputsTrajectory"     => ControlEdgeType::OutputsTrajectory,
        "FeedbackFrom"          => ControlEdgeType::FeedbackFrom,
        "FeedforwardTo"         => ControlEdgeType::FeedforwardTo,
        "MeasuredBy"            => ControlEdgeType::MeasuredBy,
        "ActuatedBy"            => ControlEdgeType::ActuatedBy,
        "DisturbedBy"           => ControlEdgeType::DisturbedBy,
        "NoisyFrom"             => ControlEdgeType::NoisyFrom,
        "ClosedLoopWith"        => ControlEdgeType::ClosedLoopWith,
        "CascadeInner"          => ControlEdgeType::CascadeInner,
        "CascadeOuter"          => ControlEdgeType::CascadeOuter,
        "OptimalGainFor"        => ControlEdgeType::OptimalGainFor,
        "MPCSolvesFor"          => ControlEdgeType::MPCSolvesFor,
        "SetPointFor"           => ControlEdgeType::SetPointFor,
        "AnalyzedFor"           => ControlEdgeType::AnalyzedFor,
        "PlantIsKinematicChain" => ControlEdgeType::PlantIsKinematicChain,
        "FedByIMUSensor"        => ControlEdgeType::FedByIMUSensor,
        "CommandsRobot3D"       => ControlEdgeType::CommandsRobot3D,
        "ImplementedInCode"     => ControlEdgeType::ImplementedInCode,
        "ProvenInMath"          => ControlEdgeType::ProvenInMath,
        "Affects"               => ControlEdgeType::Affects,
        "CausedBy"              => ControlEdgeType::CausedBy,
        "Enables"               => ControlEdgeType::Enables,
        "Prevents"              => ControlEdgeType::Prevents,
        "TemporalPrecedes"      => ControlEdgeType::TemporalPrecedes,
        "DerivedFrom"           => ControlEdgeType::DerivedFrom,
        "SimilarTo"             => ControlEdgeType::SimilarTo,
        "FunctionalRole"        => ControlEdgeType::FunctionalRole,
        "PartOf"                => ControlEdgeType::PartOf,
        _                       => ControlEdgeType::Affects,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH CREATION
// ─────────────────────────────────────────────────────────────────────────────

async fn create_graph(
    executor: &PipelineExecutor,
    analysis: ControlAnalysisResult,
    project_id: u64,
) -> ControlModalityOutput {
    let graph_id = executor.generate_id();
    let now = executor.now_iso8601();
    let mut nodes: Vec<ControlGraphNode> = Vec::new();
    let mut edges: Vec<ControlGraphEdge> = Vec::new();
    let mut node_id: u64 = 1;
    let mut edge_id: u64 = 1;

    let system_desc = executor.infer_system_description(&analysis).await;

    // ── ROOT ──
    let root_id = node_id;
    nodes.push(ControlGraphNode {
        node_id: root_id, node_type: ControlNodeType::ControlSystem,
        content: format!("ControlSystem [{:?}]: {} | plants={} controllers={} loops={} sensors={} actuators={}",
            analysis.physical_domain, system_desc,
            analysis.plants.len(), analysis.controllers.len(),
            analysis.feedback_loops.len(), analysis.sensors.len(), analysis.actuators.len()),
        physical_domain: Some(format!("{:?}", analysis.physical_domain)),
        materialized_path: Some(format!("/Modalities/ControlSystems/Project_{}/Graph_{}", project_id, graph_id)),
        provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
        version_notes: vec![VersionNote { version: 1, note: "Initial creation".into(), step_index: None, timestamp: now.clone(), change_type: ChangeType::Created }],
        keywords: vec!["control".into(), "system".into(), format!("{:?}", analysis.physical_domain).to_lowercase()],
        hotness_score: 0.0, ..Default::default()
    });
    node_id += 1;

    // Build lookup maps
    let mut plant_nid: HashMap<u64, u64> = HashMap::new();
    let mut ctrl_nid: HashMap<u64, u64> = HashMap::new();
    let mut sensor_nid: HashMap<u64, u64> = HashMap::new();
    let mut actuator_nid: HashMap<u64, u64> = HashMap::new();
    let mut loop_nid: HashMap<u64, u64> = HashMap::new();
    let mut stability_nid: HashMap<u64, u64> = HashMap::new();
    let mut sim_nid: HashMap<u64, u64> = HashMap::new();

    // ── PLANT NODES ──
    for plant in &analysis.plants {
        let pid = node_id;
        nodes.push(ControlGraphNode {
            node_id: pid, node_type: ControlNodeType::PlantNode,
            content: format!("Plant: {} [{:?}] states={} in={} out={} discrete={}",
                plant.name, plant.model_type, plant.n_states, plant.n_inputs, plant.n_outputs, plant.is_discrete),
            n_states: Some(plant.n_states), n_inputs: Some(plant.n_inputs), n_outputs: Some(plant.n_outputs),
            is_discrete: Some(plant.is_discrete), sample_rate_hz: plant.sample_time_sec.map(|dt| 1.0 / dt),
            physical_domain: Some(format!("{:?}", plant.physical_domain)),
            materialized_path: Some(format!("/Modalities/ControlSystems/Project_{}/Graph_{}/Plant/{}", project_id, graph_id, plant.plant_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: vec!["plant".into(), plant.name.to_lowercase(), format!("{:?}", plant.physical_domain).to_lowercase()],
            hotness_score: 0.0,
            embedding_hint: Some(format!("plant {} domain:{:?} states:{}", plant.name, plant.physical_domain, plant.n_states)),
            ..Default::default()
        });
        // LIVE NODE EVENT (2026-10-19, additive — same pattern as 3D's
        // first real producer). Fire-and-forget, detached, never slows
        // real graph construction.
        {
            let host = env::var("OZONE_HOST").unwrap_or_else(|_| "http://127.0.0.1:50051".into());
            let ev_project_id = project_id;
            let ev_graph_id = graph_id;
            let ev_node_id = pid;
            tokio::spawn(async move {
                let client = reqwest::Client::new();
                let _ = client.post(format!("{host}/orchestration/node-event"))
                    .json(&serde_json::json!({
                        "project_id": ev_project_id,
                        "modality": "control",
                        "graph_id": ev_graph_id,
                        "node_id": ev_node_id,
                        "related_node_id": serde_json::Value::Null,
                        "node_type": "PlantNode",
                        "operation": "node_created",
                    }))
                    .timeout(std::time::Duration::from_secs(2))
                    .send().await;
            });
        }
        plant_nid.insert(plant.plant_id, pid);
        edges.push(ControlGraphEdge { edge_id, from_node: root_id, to_node: pid, edge_type: ControlEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Plant transfer function nodes
        for tf in &plant.transfer_functions {
            let tf_nid = node_id + 1;
            let poles = if tf.poles.is_empty() {
                PipelineExecutor::compute_poles_from_denominator(&tf.denominator)
            } else { tf.poles.clone() };
            let all_stable = poles.iter().all(|p| p.is_stable_pole());

            nodes.push(ControlGraphNode {
                node_id: tf_nid, node_type: ControlNodeType::TransferFunctionNode,
                content: format!("TF {}/{}: num_deg={} den_deg={} dc_gain={:?} rel_deg={}",
                    tf.input_index, tf.output_index,
                    tf.numerator.len().saturating_sub(1), tf.denominator.len().saturating_sub(1),
                    tf.dc_gain, tf.relative_degree),
                is_stable: Some(all_stable),
                materialized_path: Some(format!("/Modalities/ControlSystems/Project_{}/Graph_{}/Plant/{}/TF/{}", project_id, graph_id, plant.plant_id, tf.tf_id)),
                provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                keywords: vec!["transfer-function".into(), "plant".into()],
                hotness_score: 0.0, ..Default::default()
            });
            edges.push(ControlGraphEdge { edge_id, from_node: pid, to_node: tf_nid, edge_type: ControlEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;

            // Pole nodes
            for pole in &poles {
                let pole_nid = node_id + 2;
                let wn = (pole.real * pole.real + pole.imag * pole.imag).sqrt();
                let zeta = if wn > 1e-12 { -pole.real / wn } else { 0.0 };
                nodes.push(ControlGraphNode {
                    node_id: pole_nid, node_type: ControlNodeType::PoleNode,
                    content: format!("Pole: {:.4}+{:.4}j |stable={} wn={:.3} zeta={:.3}",
                        pole.real, pole.imag, pole.is_stable_pole(), wn, zeta),
                    is_stable: Some(pole.is_stable_pole()),
                    materialized_path: Some(format!("/Modalities/ControlSystems/Project_{}/Graph_{}/Pole/{}", project_id, graph_id, pole_nid)),
                    provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                    keywords: if pole.is_stable_pole() { vec!["pole".into(), "stable".into()] } else { vec!["pole".into(), "unstable".into()] },
                    hotness_score: if pole.is_stable_pole() { 0.5 } else { 0.85 },
                    ..Default::default()
                });
                edges.push(ControlGraphEdge { edge_id, from_node: tf_nid, to_node: pole_nid, edge_type: ControlEdgeType::OutputToPole, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
                node_id += 1;
            }
            node_id += 2;
        }

        // Cross-modal: plant is kinematic chain (121)
        if matches!(plant.physical_domain, PhysicalDomain::Robotic) {
            edges.push(ControlGraphEdge {
                edge_id, from_node: pid, to_node: pid,
                edge_type: ControlEdgeType::PlantIsKinematicChain, weight: 1.0,
                provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("kinematics")); p },
                ..Default::default()
            });
            edge_id += 1;
            // Cross-modal: commands robot 3D (109)
            edges.push(ControlGraphEdge {
                edge_id, from_node: pid, to_node: pid,
                edge_type: ControlEdgeType::CommandsRobot3D, weight: 1.0,
                provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("3d")); p },
                ..Default::default()
            });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── CONTROLLER NODES ──
    for ctrl in &analysis.controllers {
        let cid = node_id;
        let kp = ctrl.parameters.kp;
        let ki = ctrl.parameters.ki;
        let kd = ctrl.parameters.kd;
        nodes.push(ControlGraphNode {
            node_id: cid, node_type: ControlNodeType::ControllerNode,
            content: format!("Controller {:?}: {} in={} out={} discrete={} Kp={:?} Ki={:?} Kd={:?}",
                ctrl.controller_type, ctrl.name, ctrl.n_inputs, ctrl.n_outputs, ctrl.is_discrete,
                kp.map(|v| format!("{:.4}",v)), ki.map(|v| format!("{:.4}",v)), kd.map(|v| format!("{:.4}",v))),
            n_inputs: Some(ctrl.n_inputs), n_outputs: Some(ctrl.n_outputs),
            is_discrete: Some(ctrl.is_discrete), kp, ki, kd,
            sample_rate_hz: ctrl.sample_time_sec.map(|dt| 1.0 / dt),
            materialized_path: Some(format!("/Modalities/ControlSystems/Project_{}/Graph_{}/Controller/{}", project_id, graph_id, ctrl.controller_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: vec!["controller".into(), format!("{:?}", ctrl.controller_type).to_lowercase()],
            hotness_score: 0.0, ..Default::default()
        });
        ctrl_nid.insert(ctrl.controller_id, cid);
        edges.push(ControlGraphEdge { edge_id, from_node: root_id, to_node: cid, edge_type: ControlEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Cross-modal: controller code (101)
        edges.push(ControlGraphEdge {
            edge_id, from_node: cid, to_node: cid,
            edge_type: ControlEdgeType::ImplementedInCode, weight: 1.0,
            provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
            properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("code")); p },
            ..Default::default()
        });
        edge_id += 1;
        node_id += 1;
    }

    // ── SENSOR NODES ──
    for sensor in &analysis.sensors {
        let sid = node_id;
        nodes.push(ControlGraphNode {
            node_id: sid, node_type: ControlNodeType::SensorNode,
            content: format!("Sensor {:?}: {} measures={} range=[{:.2},{:.2}] noise={:.4} rate={:.1}Hz",
                sensor.sensor_type, sensor.name, sensor.measured_quantity,
                sensor.measurement_range.0, sensor.measurement_range.1,
                sensor.noise_std, sensor.sample_rate_hz),
            sample_rate_hz: Some(sensor.sample_rate_hz),
            materialized_path: Some(format!("/Modalities/ControlSystems/Project_{}/Graph_{}/Sensor/{}", project_id, graph_id, sensor.sensor_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: vec!["sensor".into(), format!("{:?}", sensor.sensor_type).to_lowercase(), sensor.measured_quantity.to_lowercase()],
            hotness_score: 0.0, ..Default::default()
        });
        sensor_nid.insert(sensor.sensor_id, sid);
        edges.push(ControlGraphEdge { edge_id, from_node: root_id, to_node: sid, edge_type: ControlEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Cross-modal: IMU sensor (116)
        if matches!(sensor.sensor_type, SensorType::IMU | SensorType::Accelerometer | SensorType::Gyroscope) {
            edges.push(ControlGraphEdge {
                edge_id, from_node: sid, to_node: sid,
                edge_type: ControlEdgeType::FedByIMUSensor, weight: 1.0,
                provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("imu")); p },
                ..Default::default()
            });
            edge_id += 1;
        }
        if matches!(sensor.sensor_type, SensorType::Radar) {
            edges.push(ControlGraphEdge {
                edge_id, from_node: sid, to_node: sid,
                edge_type: ControlEdgeType::TrackFromRadar, weight: 1.0,
                provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("radar")); p },
                ..Default::default()
            });
            edge_id += 1;
        }
        if matches!(sensor.sensor_type, SensorType::Sonar) {
            edges.push(ControlGraphEdge {
                edge_id, from_node: sid, to_node: sid,
                edge_type: ControlEdgeType::NavigationFromSonar, weight: 1.0,
                provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("sonar")); p },
                ..Default::default()
            });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── ACTUATOR NODES ──
    for actuator in &analysis.actuators {
        let aid = node_id;
        nodes.push(ControlGraphNode {
            node_id: aid, node_type: ControlNodeType::ActuatorNode,
            content: format!("Actuator {:?}: {} controls={} range=[{:.2},{:.2}] bw={:.1}Hz",
                actuator.actuator_type, actuator.name, actuator.controlled_quantity,
                actuator.output_range.0, actuator.output_range.1, actuator.bandwidth_hz),
            bandwidth_hz: Some(actuator.bandwidth_hz),
            materialized_path: Some(format!("/Modalities/ControlSystems/Project_{}/Graph_{}/Actuator/{}", project_id, graph_id, actuator.actuator_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: vec!["actuator".into(), format!("{:?}", actuator.actuator_type).to_lowercase()],
            hotness_score: 0.0, ..Default::default()
        });
        actuator_nid.insert(actuator.actuator_id, aid);
        edges.push(ControlGraphEdge { edge_id, from_node: root_id, to_node: aid, edge_type: ControlEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── FEEDBACK LOOP NODES ──
    for floop in &analysis.feedback_loops {
        let lid = node_id;
        nodes.push(ControlGraphNode {
            node_id: lid, node_type: ControlNodeType::FeedbackLoopNode,
            content: format!("Loop {:?}: {} plant={} ctrl={} sensors={} delay={:.1}ms rate={:.1}Hz inner={}",
                floop.loop_type, floop.name, floop.plant_id, floop.controller_id,
                floop.sensor_ids.len(), floop.loop_delay_ms, floop.sample_rate_hz, floop.is_inner_loop),
            sample_rate_hz: Some(floop.sample_rate_hz),
            materialized_path: Some(format!("/Modalities/ControlSystems/Project_{}/Graph_{}/Loop/{}", project_id, graph_id, floop.loop_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: vec!["feedback-loop".into(), format!("{:?}", floop.loop_type).to_lowercase()],
            hotness_score: 0.0, ..Default::default()
        });
        loop_nid.insert(floop.loop_id, lid);
        edges.push(ControlGraphEdge { edge_id, from_node: root_id, to_node: lid, edge_type: ControlEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Loop → plant
        if let Some(&p_nid) = plant_nid.get(&floop.plant_id) {
            edges.push(ControlGraphEdge { edge_id, from_node: lid, to_node: p_nid, edge_type: ControlEdgeType::ClosedLoopWith, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        // Loop → controller
        if let Some(&c_nid) = ctrl_nid.get(&floop.controller_id) {
            edges.push(ControlGraphEdge { edge_id, from_node: c_nid, to_node: lid, edge_type: ControlEdgeType::Controls, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        // Loop → sensors (feedback path)
        for &sensor_id in &floop.sensor_ids {
            if let Some(&s_nid) = sensor_nid.get(&sensor_id) {
                edges.push(ControlGraphEdge { edge_id, from_node: lid, to_node: s_nid, edge_type: ControlEdgeType::FeedbackFrom, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        // Loop → actuators
        for &actuator_id in &floop.actuator_ids {
            if let Some(&a_nid) = actuator_nid.get(&actuator_id) {
                edges.push(ControlGraphEdge { edge_id, from_node: lid, to_node: a_nid, edge_type: ControlEdgeType::ActuatedBy, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        // Cascade loop hierarchy
        if let Some(outer_id) = floop.outer_loop_id {
            if let Some(&outer_nid) = loop_nid.get(&outer_id) {
                edges.push(ControlGraphEdge { edge_id, from_node: outer_nid, to_node: lid, edge_type: ControlEdgeType::CascadeInner, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        node_id += 1;
    }

    // ── STABILITY ANALYSIS NODES ──
    for stab in &analysis.stability_analyses {
        let sid = node_id;
        nodes.push(ControlGraphNode {
            node_id: sid, node_type: ControlNodeType::StabilityAnalysisNode,
            content: format!("Stability [{:?}]: {:?} GM={:?}dB PM={:?}° method={:?}",
                stab.stability_type, stab.is_stable,
                stab.gain_margin_db.map(|v| format!("{:.2}",v)),
                stab.phase_margin_deg.map(|v| format!("{:.1}",v)),
                stab.method),
            is_stable: Some(stab.is_stable),
            gain_margin_db: stab.gain_margin_db,
            phase_margin_deg: stab.phase_margin_deg,
            materialized_path: Some(format!("/Modalities/ControlSystems/Project_{}/Graph_{}/Stability/{}", project_id, graph_id, stab.analysis_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: { let mut kw = vec!["stability".into(), format!("{:?}", stab.method).to_lowercase()]; if stab.is_stable { kw.push("stable".into()); } else { kw.push("unstable".into()); } kw },
            hotness_score: if stab.is_stable { 0.6 } else { 0.95 },
            ..Default::default()
        });
        stability_nid.insert(stab.analysis_id, sid);
        edges.push(ControlGraphEdge { edge_id, from_node: root_id, to_node: sid, edge_type: ControlEdgeType::AnalyzedFor, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Cross-modal: stability proof → math (105)
        edges.push(ControlGraphEdge {
            edge_id, from_node: sid, to_node: sid,
            edge_type: ControlEdgeType::ProvenInMath, weight: 1.0,
            provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
            properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("math")); p },
            ..Default::default()
        });
        edge_id += 1; node_id += 1;
    }

    // ── FREQUENCY RESPONSE NODES ──
    for fr in &analysis.frequency_responses {
        let frid = node_id;
        nodes.push(ControlGraphNode {
            node_id: frid, node_type: ControlNodeType::FrequencyResponseNode,
            content: format!("FreqResp [{:?}]: bw={:?}rad/s resonant_peak={:?}dB points={}",
                fr.response_type,
                fr.bandwidth_3db_rad_s.map(|v| format!("{:.3}", v)),
                fr.resonant_peak_db.map(|v| format!("{:.2}", v)),
                fr.frequencies_rad_s.len()),
            bandwidth_hz: fr.bandwidth_3db_rad_s.map(|w| w / (2.0 * std::f64::consts::PI)),
            materialized_path: Some(format!("/Modalities/ControlSystems/Project_{}/Graph_{}/FreqResp/{}", project_id, graph_id, fr.freq_response_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["frequency-response".into(), format!("{:?}", fr.response_type).to_lowercase()],
            hotness_score: 0.0, ..Default::default()
        });
        edges.push(ControlGraphEdge { edge_id, from_node: root_id, to_node: frid, edge_type: ControlEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── SIMULATION RESULT NODES ──
    for sim in &analysis.simulation_results {
        let sim_node_id = node_id;
        nodes.push(ControlGraphNode {
            node_id: sim_node_id, node_type: ControlNodeType::SimulationResultNode,
            content: format!("Simulation: rise={:?}s settling={:?}s overshoot={:?}% sse={:?} diverged={}",
                sim.performance.rise_time_sec.map(|v| format!("{:.3}", v)),
                sim.performance.settling_time_sec.map(|v| format!("{:.3}", v)),
                sim.performance.overshoot_percent.map(|v| format!("{:.1}", v)),
                sim.performance.steady_state_error.map(|v| format!("{:.4}", v)),
                sim.diverged),
            is_stable: Some(!sim.diverged),
            materialized_path: Some(format!("/Modalities/ControlSystems/Project_{}/Graph_{}/Sim/{}", project_id, graph_id, sim.sim_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: if sim.diverged { vec!["simulation".into(), "diverged".into()] } else { vec!["simulation".into(), "stable".into()] },
            hotness_score: if sim.diverged { 0.9 } else { 0.6 },
            ..Default::default()
        });
        sim_nid.insert(sim.sim_id, sim_node_id);
        edges.push(ControlGraphEdge { edge_id, from_node: root_id, to_node: sim_node_id, edge_type: ControlEdgeType::SimulatedBy, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── VERIFICATION RESULT NODES ──
    for vr in &analysis.verification_results {
        let vrid = node_id;
        nodes.push(ControlGraphNode {
            node_id: vrid, node_type: ControlNodeType::VerificationResultNode,
            content: format!("Verify [{:?}]: {} {} measured={:.4} bound={:.4} margin={:.4}",
                vr.spec_type_from_spec_id(&analysis), if vr.passed { "PASS" } else { "FAIL" },
                vr.parameter_from_id(&analysis),
                vr.measured_value, vr.bound_value, vr.margin),
            is_stable: Some(vr.passed),
            materialized_path: Some(format!("/Modalities/ControlSystems/Project_{}/Graph_{}/Verify/{}", project_id, graph_id, vr.spec_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: if vr.passed { vec!["verification".into(), "pass".into()] } else { vec!["verification".into(), "fail".into()] },
            hotness_score: if vr.passed { 0.5 } else { 0.9 },
            ..Default::default()
        });
        edges.push(ControlGraphEdge { edge_id, from_node: root_id, to_node: vrid, edge_type: ControlEdgeType::VerifiedBy, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── LQR SOLUTION NODES ──
    for lqr in &analysis.lqr_solutions {
        let lqr_nid = node_id;
        let all_cl_stable = lqr.closed_loop_poles.iter().all(|p| p.real < 0.0);
        nodes.push(ControlGraphNode {
            node_id: lqr_nid, node_type: ControlNodeType::LQRSolutionNode,
            content: format!("LQR: cost={:.4} cl_poles={} all_stable={}", lqr.optimal_cost, lqr.closed_loop_poles.len(), all_cl_stable),
            is_stable: Some(all_cl_stable),
            materialized_path: Some(format!("/Modalities/ControlSystems/Project_{}/Graph_{}/LQR/{}", project_id, graph_id, lqr.lqr_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["lqr".into(), "optimal".into()],
            hotness_score: 0.0, ..Default::default()
        });
        edges.push(ControlGraphEdge { edge_id, from_node: root_id, to_node: lqr_nid, edge_type: ControlEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // LQR → first plant (OptimalGainFor)
        if let Some(&p_nid) = plant_nid.values().next() {
            edges.push(ControlGraphEdge { edge_id, from_node: lqr_nid, to_node: p_nid, edge_type: ControlEdgeType::OptimalGainFor, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── MPC PROBLEM NODES ──
    for mpc in &analysis.mpc_problems {
        let mpc_nid = node_id;
        nodes.push(ControlGraphNode {
            node_id: mpc_nid, node_type: ControlNodeType::MPCProblemNode,
            content: format!("MPC: horizon={}/{} solve={:?}ms soft={}",
                mpc.prediction_horizon, mpc.control_horizon,
                mpc.solve_time_ms.map(|t| format!("{:.2}", t)),
                mpc.constraints.terminal_constraint),
            materialized_path: Some(format!("/Modalities/ControlSystems/Project_{}/Graph_{}/MPC/{}", project_id, graph_id, mpc.mpc_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["mpc".into(), "optimal-control".into()],
            hotness_score: 0.0, ..Default::default()
        });
        edges.push(ControlGraphEdge { edge_id, from_node: root_id, to_node: mpc_nid, edge_type: ControlEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // MPC → associated controller
        if let Some(&c_nid) = ctrl_nid.get(&mpc.controller_id) {
            edges.push(ControlGraphEdge { edge_id, from_node: mpc_nid, to_node: c_nid, edge_type: ControlEdgeType::MPCSolvesFor, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── IDENTIFIED MODEL NODES ──
    for id_model in &analysis.identified_models {
        let im_nid = node_id;
        nodes.push(ControlGraphNode {
            node_id: im_nid, node_type: ControlNodeType::PlantNode,
            content: format!("IdentifiedModel [{:?}]: order={} fit={:.1}% AIC={:.2} val_fit={:?}%",
                id_model.method, id_model.model_order, id_model.fit_percent, id_model.aic,
                id_model.cross_validation_fit_percent.map(|v| format!("{:.1}", v))),
            n_states: Some(id_model.model_order),
            materialized_path: Some(format!("/Modalities/ControlSystems/Project_{}/Graph_{}/IdModel/{}", project_id, graph_id, id_model.model_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["system-identification".into(), format!("{:?}", id_model.method).to_lowercase()],
            hotness_score: 0.0, ..Default::default()
        });
        edges.push(ControlGraphEdge { edge_id, from_node: root_id, to_node: im_nid, edge_type: ControlEdgeType::IdentifiedAs, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── DISTURBANCE NODES ──
    for floop in &analysis.feedback_loops {
        for dist in &floop.disturbance_channels {
            let did = node_id;
            nodes.push(ControlGraphNode {
                node_id: did, node_type: ControlNodeType::DisturbanceNode,
                content: format!("Disturbance: {} [{:?}] max_amp={:.3}", dist.name, dist.entry_point, dist.max_amplitude),
                materialized_path: Some(format!("/Modalities/ControlSystems/Project_{}/Graph_{}/Dist/{}", project_id, graph_id, did)),
                provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                keywords: vec!["disturbance".into(), format!("{:?}", dist.entry_point).to_lowercase()],
                hotness_score: 0.0, ..Default::default()
            });
            if let Some(&l_nid) = loop_nid.get(&floop.loop_id) {
                edges.push(ControlGraphEdge { edge_id, from_node: did, to_node: l_nid, edge_type: ControlEdgeType::DisturbedBy, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
            node_id += 1;
        }
    }

    // ── HOOK 1: OnGraphCreated ──
    let mut save_errors: Vec<String> = Vec::new();
    if let Err(e) = executor.save_graph(&ControlGraph {
        graph_id, project_id, source_description: analysis.source_description.clone(),
        nodes: nodes.clone(), edges: edges.clone(), root_node_id: root_id,
        state: GraphStateType::Created,
        state_history: vec![GraphStateTransition { from: GraphStateType::Created, to: GraphStateType::Created, timestamp: now.clone(), triggered_by_step: None }],
        created_at: now.clone(), updated_at: now.clone(), version: 1,
        version_notes: vec![VersionNote { version: 1, note: format!("Created: {} nodes {} edges", nodes.len(), edges.len()), step_index: None, timestamp: now.clone(), change_type: ChangeType::Created }],
    }) {
        eprintln!("control save_graph failed: file=assets/pipelines/modalities/control/main.rs container={graph_id} stage=created error={e}");
        save_errors.push(format!("created: {e}"));
    }

    // ── HOOK 2: OnInferRelationships ──
    let inferred = executor.infer_semantic_relationships(&nodes).await;
    let valid: std::collections::HashSet<u64> = nodes.iter().map(|n| n.node_id).collect();
    for (from, to, etype, reason) in inferred {
        if valid.contains(&from) && valid.contains(&to) && from != to {
            edges.push(ControlGraphEdge {
                edge_id, from_node: from, to_node: to, edge_type: etype, weight: 1.0,
                provenance: EdgeProvenance::DerivedFromHook, version: 1,
                properties: { let mut p = HashMap::new(); p.insert("reason".into(), serde_json::json!(reason)); p },
                ..Default::default()
            });
            edge_id += 1;
        }
    }

    // ── HOOK 3: OnEdgeCompletion → hotness + prune cross-modal self-loops ──
    let mut deg: HashMap<u64, u32> = HashMap::new();
    for e in &edges { *deg.entry(e.from_node).or_insert(0) += 1; *deg.entry(e.to_node).or_insert(0) += 1; }
    let max_deg = deg.values().copied().max().unwrap_or(1) as f32;
    for n in &mut nodes {
        if let Some(&d) = deg.get(&n.node_id) {
            n.hotness_score = (n.hotness_score + (d as f32 / max_deg) * 0.15).min(1.0);
        }
    }
    // Keep self-loop cross-modal markers, remove accidental self-loops from hook
    edges.retain(|e| e.from_node != e.to_node ||
        matches!(e.edge_type,
            ControlEdgeType::PlantIsKinematicChain | ControlEdgeType::FedByIMUSensor |
            ControlEdgeType::CommandsRobot3D | ControlEdgeType::TrackFromRadar |
            ControlEdgeType::NavigationFromSonar | ControlEdgeType::WaypointFromGeo |
            ControlEdgeType::ImplementedInCode | ControlEdgeType::ProvenInMath |
            ControlEdgeType::LinkQualityFromEM
        )
    );

    let mut final_graph = ControlGraph {
        graph_id, project_id, source_description: analysis.source_description,
        nodes, edges, root_node_id: root_id, state: GraphStateType::SemanticEnriched,
        state_history: vec![GraphStateTransition { from: GraphStateType::Created, to: GraphStateType::SemanticEnriched, timestamp: now.clone(), triggered_by_step: None }],
        created_at: now.clone(), updated_at: now.clone(), version: 1,
        version_notes: vec![VersionNote { version: 1, note: "Semantic enrichment complete".into(), step_index: None, timestamp: now, change_type: ChangeType::EnrichedBySemantic }],
    };
    if let Err(e) = executor.save_graph(&final_graph) {
        eprintln!("control save_graph failed: file=assets/pipelines/modalities/control/main.rs container={} stage=enriched error={e}", final_graph.graph_id);
        save_errors.push(format!("enriched: {e}"));
    }
    if let Err(e) = persist_control_graph(&mut final_graph).await {
        eprintln!("control persist: graph built but ZSEI persistence failed: {e}");
        save_errors.push(format!("persist: {e}"));
    }
    ControlModalityOutput { success: true, graph_id: Some(final_graph.graph_id), graph: Some(final_graph), save_error: (!save_errors.is_empty()).then(|| save_errors.join("; ")), ..Default::default() }
}

/// POST /zsei/query helper — same contract the other modality pipelines use:
/// envelope `{query, session_token}`, success-gated, returns the inner `result`.
async fn zsei_query(query: serde_json::Value) -> Result<serde_json::Value, String> {
    let host = std::env::var("OZONE_HOST").unwrap_or_else(|_| "http://127.0.0.1:50051".to_string());
    let client = reqwest::Client::new();
    let body = serde_json::json!({"query": query, "session_token": ""});
    let resp = client
        .post(format!("{host}/zsei/query")).header("x-ozone-trace", std::env::var("OZONE_TRACE_ID").unwrap_or_default()).header("x-ozone-origin", std::env::var("OZONE_ORIGIN").unwrap_or_default())
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("ZSEI unreachable: {e}"))?;
    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if v.get("success").and_then(|s| s.as_bool()).unwrap_or(false) {
        Ok(v.get("result").cloned().unwrap_or(serde_json::Value::Null))
    } else {
        Err(v.get("error").and_then(|e| e.as_str()).unwrap_or("zsei query failed").to_string())
    }
}

/// Persist the built graph as a real ModalityGraph container (parent =
/// project) + node/edge payload file — the proven revival pattern.
async fn persist_control_graph(graph: &mut ControlGraph) -> Result<(), String> {
    let keywords: Vec<String> = {
        let mut k: Vec<String> = graph.nodes.iter()
            .flat_map(|n| n.keywords.iter().cloned())
            .collect();
        k.sort(); k.dedup();
        if k.is_empty() { k.push("control".to_string()); }
        // container keyword set kept WHOLE (no 12-keyword cut — doctrine audit #33, 2026-11-04)
        k
    };
    let name = format!("Control graph ({} nodes, {} edges)", graph.nodes.len(), graph.edges.len());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let container = serde_json::json!({
        "global_state": {"container_id": 0, "parent_id": graph.project_id, "child_ids": [], "child_count": 0, "version": 1},
        "local_state": {
            "metadata": {"container_type": "ModalityGraph", "modality": "Unknown", "created_at": now,
                "updated_at": now, "provenance": "control-pipeline", "permissions": 0, "owner_id": 0,
                "name": name, "materialized_path": null},
            "context": {"categories": [], "methodologies": [], "keywords": keywords,
                "topics": ["control"], "relationships": [], "learned_associations": [], "embedding": null},
            "storage": {"db_shard_id": null, "vector_index_ref": null,
                "object_store_path": null, "compression_type": "None"},
            "hints": {"access_frequency": 0, "hotness_score": 0.0, "last_accessed": 0, "centroid": null, "ml_prediction_weight": 0.0},
            "integrity": {"content_hash": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
                "semantic_fingerprint": [], "last_verified": 0, "integrity_score": 1.0, "version_history": []},
            "file_context": null, "code_context": null, "text_context": null, "external_ref": null
        }
    });
    let result = zsei_query(serde_json::json!({
        "CreateContainer": { "parent_id": graph.project_id, "container": container }
    })).await?;
    let new_id = result.get("ContainerID").and_then(|c| c.as_u64())
        .ok_or("CreateContainer returned no ContainerID")?;
    let dir = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    let rel = format!("graphs/control_{}.json", new_id);
    let abs = std::path::Path::new(&dir).join(&rel);
    if let Some(parent) = abs.parent() { let _ = std::fs::create_dir_all(parent); }
    std::fs::write(&abs, serde_json::to_string_pretty(&serde_json::json!({
        "graph_id": new_id, "nodes": graph.nodes, "edges": graph.edges
    })).map_err(|e| e.to_string())?).map_err(|e| format!("graph content write failed: {e}"))?;
    let _ = zsei_query(serde_json::json!({
        "UpdateContainer": { "container_id": new_id, "updates": { "storage": {
            "db_shard_id": null, "vector_index_ref": null, "object_store_path": rel,
            "compression_type": "None" }, "metadata": null, "context": null, "hints": null } }
    })).await;
    graph.graph_id = new_id;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// HELPER TRAIT — gives VerificationResult friendly display from analysis context
// ─────────────────────────────────────────────────────────────────────────────

trait VerificationDisplay {
    fn spec_type_from_spec_id(&self, analysis: &ControlAnalysisResult) -> String;
    fn parameter_from_id(&self, analysis: &ControlAnalysisResult) -> String;
}

impl VerificationDisplay for VerificationResult {
    fn spec_type_from_spec_id(&self, analysis: &ControlAnalysisResult) -> String {
        // Find matching spec in analysis — if not found return generic label
        "Spec".into()
    }
    fn parameter_from_id(&self, analysis: &ControlAnalysisResult) -> String {
        "parameter".into()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// MAIN EXECUTION
// ─────────────────────────────────────────────────────────────────────────────

pub async fn execute(input: ControlModalityAction) -> Result<ControlModalityOutput, String> {
    let executor = PipelineExecutor::new();

    match input {
        ControlModalityAction::LinkToModality { source_graph_id, target_graph_id, target_modality, relationship } => {
            match link_handler::link_bidirectional(&zsei_query, source_graph_id, target_graph_id).await {
                Ok(outcome) => Ok(ControlModalityOutput {
                    success: true,
                    link_result: Some(link_handler::link_result_json(source_graph_id, target_graph_id, &target_modality, &relationship, outcome)),
                    ..Default::default()
                }),
                Err(e) => Ok(ControlModalityOutput { success: false, error: Some(e), ..Default::default() }),
            }
        }
        ControlModalityAction::Analyze { data, analyze_stability, analyze_performance, analyze_robustness, identify_system } => {
            // HONESTY (doctrine audit 2026-11-04, R-FAB / docs/FABRICATED_OUTPUT_AUDIT_2026-11-04.md X3): this arm used to return `success: true` with an empty,
            // description-only analysis — a fake success that file_link then turned into a root-only graph. An analysis that did not run FAILS.
            #[allow(unreachable_code, unused_variables)]
            { return Ok(ControlModalityOutput { success: false, error: Some("control analysis is not implemented: NO analysis was performed on the data source (the previous output was a description-only placeholder presented as a successful analysis and has been removed); register a control analysis provider (MCP) to enable it".to_string()), ..Default::default() }); }
            let analysis_id = executor.generate_id();
            let source_description = match &data {
                ControlDataSource::MatlabFile { file_path, .. }        => format!("MATLAB: {}", file_path),
                ControlDataSource::Simulink { file_path, .. }          => format!("Simulink: {}", file_path),
                ControlDataSource::ModelicaFMU { file_path, model_name } => format!("FMU: {} ({})", file_path, model_name),
                ControlDataSource::PythonControl { file_path, .. }     => format!("Python: {}", file_path),
                ControlDataSource::StateSpaceMatrices { a_file, .. }   => format!("SS matrices: {}", a_file),
                ControlDataSource::TransferFunctionCoefficients { file_path, .. } => format!("TF: {}", file_path),
                ControlDataSource::TimeSeries { file_path, .. }        => format!("TimeSeries: {}", file_path),
                ControlDataSource::RobotDescription { file_path, format, .. } => format!("Robot {:?}: {}", format, file_path),
                ControlDataSource::ROS2Bag { file_path, topics }       => format!("ROS2Bag: {} ({} topics)", file_path, topics.len()),
                ControlDataSource::InlinePlant { plant }               => format!("InlinePlant: {}", plant.name),
                ControlDataSource::LiveStream { endpoint, .. }         => format!("LiveStream: {}", endpoint),
            };
            Ok(ControlModalityOutput {
                success: true,
                analysis: Some(ControlAnalysisResult { analysis_id, source_description, ..Default::default() }),
                ..Default::default()
            })
        }

        ControlModalityAction::CreateGraph { analysis, project_id } => {
            Ok(create_graph(&executor, analysis, project_id).await)
        }

        ControlModalityAction::UpdateGraph { graph_id, updates, project_id } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            let mut next_nid = graph.nodes.iter().map(|n| n.node_id).max().unwrap_or(0) + 1;
            let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
            let initial_count = graph.nodes.len();

            for update in &updates {
                match update {
                    ControlUpdate::UpdatePlant { plant } => {
                        // Find existing plant node or add new one
                        if let Some(n) = graph.nodes.iter_mut().find(|n| matches!(n.node_type, ControlNodeType::PlantNode) && n.content.contains(&plant.name)) {
                            n.n_states = Some(plant.n_states);
                            n.n_inputs = Some(plant.n_inputs);
                            n.n_outputs = Some(plant.n_outputs);
                            n.version += 1;
                            n.version_notes.push(VersionNote { version: n.version, note: format!("Plant updated: {}", plant.name), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                        } else {
                            graph.nodes.push(ControlGraphNode {
                                node_id: next_nid, node_type: ControlNodeType::PlantNode,
                                content: format!("Plant: {} states={} in={} out={}", plant.name, plant.n_states, plant.n_inputs, plant.n_outputs),
                                n_states: Some(plant.n_states), n_inputs: Some(plant.n_inputs), n_outputs: Some(plant.n_outputs),
                                provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                                keywords: vec!["plant".into(), plant.name.to_lowercase()], hotness_score: 0.0, ..Default::default()
                            });
                            graph.edges.push(ControlGraphEdge { edge_id: next_eid, from_node: graph.root_node_id, to_node: next_nid, edge_type: ControlEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                            next_eid += 1; next_nid += 1;
                        }
                    }
                    ControlUpdate::UpdateController { controller } => {
                        graph.nodes.push(ControlGraphNode {
                            node_id: next_nid, node_type: ControlNodeType::ControllerNode,
                            content: format!("Controller {:?}: {}", controller.controller_type, controller.name),
                            kp: controller.parameters.kp, ki: controller.parameters.ki, kd: controller.parameters.kd,
                            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                            keywords: vec!["controller".into(), format!("{:?}", controller.controller_type).to_lowercase()], hotness_score: 0.0, ..Default::default()
                        });
                        graph.edges.push(ControlGraphEdge { edge_id: next_eid, from_node: graph.root_node_id, to_node: next_nid, edge_type: ControlEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                        next_eid += 1; next_nid += 1;
                    }
                    ControlUpdate::AddSensor { sensor } => {
                        graph.nodes.push(ControlGraphNode {
                            node_id: next_nid, node_type: ControlNodeType::SensorNode,
                            content: format!("Sensor {:?}: {} measures={}", sensor.sensor_type, sensor.name, sensor.measured_quantity),
                            sample_rate_hz: Some(sensor.sample_rate_hz),
                            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                            keywords: vec!["sensor".into(), sensor.name.to_lowercase()], hotness_score: 0.0, ..Default::default()
                        });
                        graph.edges.push(ControlGraphEdge { edge_id: next_eid, from_node: graph.root_node_id, to_node: next_nid, edge_type: ControlEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                        next_eid += 1; next_nid += 1;
                    }
                    ControlUpdate::UpdateSimResult { result } => {
                        graph.nodes.push(ControlGraphNode {
                            node_id: next_nid, node_type: ControlNodeType::SimulationResultNode,
                            content: format!("Simulation: overshoot={:?}% settling={:?}s diverged={}", result.performance.overshoot_percent.map(|v| format!("{:.1}",v)), result.performance.settling_time_sec.map(|v| format!("{:.3}",v)), result.diverged),
                            is_stable: Some(!result.diverged),
                            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                            keywords: vec!["simulation".into()], hotness_score: if result.diverged { 0.9 } else { 0.6 }, ..Default::default()
                        });
                        graph.edges.push(ControlGraphEdge { edge_id: next_eid, from_node: graph.root_node_id, to_node: next_nid, edge_type: ControlEdgeType::SimulatedBy, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                        next_eid += 1; next_nid += 1;
                    }
                    ControlUpdate::AddVerification { result } => {
                        graph.nodes.push(ControlGraphNode {
                            node_id: next_nid, node_type: ControlNodeType::VerificationResultNode,
                            content: format!("Verify: {} measured={:.4} bound={:.4} margin={:.4}", if result.passed { "PASS" } else { "FAIL" }, result.measured_value, result.bound_value, result.margin),
                            is_stable: Some(result.passed),
                            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                            keywords: if result.passed { vec!["verification".into(), "pass".into()] } else { vec!["verification".into(), "fail".into()] },
                            hotness_score: if result.passed { 0.5 } else { 0.9 }, ..Default::default()
                        });
                        graph.edges.push(ControlGraphEdge { edge_id: next_eid, from_node: graph.root_node_id, to_node: next_nid, edge_type: ControlEdgeType::VerifiedBy, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                        next_eid += 1; next_nid += 1;
                    }
                }
            }

            graph.version += 1; graph.updated_at = now.clone(); graph.state = GraphStateType::Updated;
            graph.version_notes.push(VersionNote { version: graph.version, note: format!("{} updates applied ({} new nodes)", updates.len(), graph.nodes.len() - initial_count), step_index: None, timestamp: now, change_type: ChangeType::Updated });
            executor.save_graph(&graph)?;
            Ok(ControlModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        ControlModalityAction::DesignController { plant_model, requirements, method } => {
            // LLM-assisted controller design
            let prompt = format!(r#"
Design a controller for this plant:
Plant: {} states={} inputs={} outputs={} domain={:?}
Method: {:?}
Requirements: settling_time≤{:?}s overshoot≤{:?}% bandwidth≥{:?}Hz GM≥{:?}dB PM≥{:?}°

Return ONLY valid JSON:
{{
    "controller_type": "PID|LQR|LeadLag|H_Infinity|MPC",
    "kp": null_or_number, "ki": null_or_number, "kd": null_or_number,
    "gain": null_or_number, "zero_rad_s": null_or_number, "pole_rad_s": null_or_number,
    "design_notes": "brief design rationale"
}}"#,
                plant_model.name, plant_model.n_states, plant_model.n_inputs, plant_model.n_outputs,
                plant_model.physical_domain, method,
                requirements.settling_time_max_sec, requirements.overshoot_max_percent,
                requirements.bandwidth_min_hz, requirements.gain_margin_min_db, requirements.phase_margin_min_deg);

            let ctrl_type;
            let mut params = ControllerParameters::default();

            match executor.llm_zero_shot(&prompt, 300).await {
                Ok(raw) => {
                    let json_str = PipelineExecutor::extract_json_object_static(&raw);
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json_str) {
                        params.kp = v["kp"].as_f64();
                        params.ki = v["ki"].as_f64();
                        params.kd = v["kd"].as_f64();
                        params.gain = v["gain"].as_f64();
                        params.zero_rad_s = v["zero_rad_s"].as_f64();
                        params.pole_rad_s = v["pole_rad_s"].as_f64();
                        ctrl_type = match v["controller_type"].as_str().unwrap_or("PID") {
                            "LQR"       => ControllerType::LQR,
                            "LeadLag"   => ControllerType::LeadLag,
                            "Lead"      => ControllerType::Lead,
                            "Lag"       => ControllerType::Lag,
                            "H_Infinity"=> ControllerType::H_Infinity,
                            "MPC"       => ControllerType::MPC,
                            _           => ControllerType::PID,
                        };
                    } else {
                        return Ok(ControlModalityOutput { success: false, error: Some("controller design unavailable: the model answer was not valid JSON; no default gains are substituted".into()), ..Default::default() });
                    }
                }
                Err(_) => {
                    return Ok(ControlModalityOutput { success: false, error: Some("controller design unavailable: no model answer; no default gains are substituted".into()), ..Default::default() });
                }
            }

            let controller = Controller {
                controller_id: executor.generate_id(),
                name: format!("{:?}_for_{}", ctrl_type, plant_model.name),
                controller_type: ctrl_type,
                n_inputs: plant_model.n_outputs,
                n_outputs: plant_model.n_inputs,
                is_discrete: requirements.sample_rate_hz.is_some(),
                sample_time_sec: requirements.sample_rate_hz.map(|r| 1.0 / r),
                parameters: params,
                design_method: format!("{:?} (LLM-suggested, unverified: run AnalyzeStability/SimulateResponse before trusting)", method),
                ..Default::default()
            };
            Ok(ControlModalityOutput { success: true, designed_controller: Some(controller), ..Default::default() })
        }

        ControlModalityAction::TunePID { plant_model, method, performance_spec } => {
            // Rule-based tuning needs a plant model; nothing is guessed when it is missing.
            let tf = match plant_model.transfer_functions.first() {
                Some(tf) if tf.denominator.len() >= 2 => tf,
                _ => return Ok(ControlModalityOutput { success: false, error: Some("PID tuning needs a plant transfer function (denominator degree ≥ 1) in plant_model.transfer_functions".into()), ..Default::default() }),
            };
            let tuned: Result<(f64, f64, f64, &'static str), String> = match method {
                PIDTuningMethod::ZieglerNichols_Step => fopdt_from_tf(&tf.numerator, &tf.denominator).and_then(|(k, l, t)| {
                    if l < 1e-9 { return Err("no measurable dead time L: Ziegler–Nichols step rules (Kp = 1.2 T/(K L)) are undefined".to_string()); }
                    let kp = 1.2 * t / (k * l);
                    Ok((kp, kp / (2.0 * l), kp * 0.5 * l, "Ziegler–Nichols step (tangent-method FOPDT fit)"))
                }),
                PIDTuningMethod::ZieglerNichols_Ultimate | PIDTuningMethod::Tyreus_Luyben => {
                    let (freqs, mag, phase) = PipelineExecutor::compute_bode(&tf.numerator, &tf.denominator, 1e-3, 1e4, 4000);
                    let (gm_db, _pm, _gc, pc) = PipelineExecutor::find_margins(&freqs, &mag, &phase);
                    match (gm_db, pc) {
                        (Some(gm), Some(wpc)) if wpc > 0.0 => {
                            let ku = 10f64.powf(gm / 20.0);
                            let pu = 2.0 * std::f64::consts::PI / wpc;
                            if matches!(method, PIDTuningMethod::ZieglerNichols_Ultimate) {
                                let kp = 0.6 * ku;
                                Ok((kp, kp / (0.5 * pu), kp * pu / 8.0, "Ziegler–Nichols ultimate (frequency-response Ku, Pu)"))
                            } else {
                                let kp = ku / 2.2;
                                Ok((kp, kp / (2.2 * pu), kp * pu / 6.3, "Tyreus–Luyben (frequency-response Ku, Pu)"))
                            }
                        }
                        _ => Err("phase never crosses −180° in 1e-3..1e4 rad/s: no ultimate gain/period exists for this plant".to_string()),
                    }
                }
                PIDTuningMethod::IMC_Based | PIDTuningMethod::Lambda_Tuning => match performance_spec.target_settling_time_sec {
                    None => Err("IMC/lambda tuning needs performance_spec.target_settling_time_sec (λ = ts/4); no default is assumed".to_string()),
                    Some(ts) => fopdt_from_tf(&tf.numerator, &tf.denominator).map(|(k, l, t)| {
                        let lambda = ts / 4.0;
                        let kp = t / (k * (lambda + l));
                        (kp, kp / t, 0.0, "IMC/lambda PI on a tangent-method FOPDT fit")
                    }),
                },
                _ => Err(format!("tuning method {:?} is not implemented; no gains are guessed", method)),
            };
            let (kp, ki, kd, design_note) = match tuned {
                Ok(v) => v,
                Err(e) => return Ok(ControlModalityOutput { success: false, error: Some(e), ..Default::default() }),
            };

            let mut params = ControllerParameters::default();
            params.kp = Some(kp); params.ki = Some(ki);
            if performance_spec.derivative_filter || kd.abs() > 1e-10 { params.kd = Some(kd); }
            if performance_spec.anti_windup { params.anti_windup = Some(AntiWindupType::BackCalculation); }

            let controller = Controller {
                controller_id: executor.generate_id(),
                name: format!("PID_{:?}_{}", method, plant_model.name),
                controller_type: if ki.abs() < 1e-10 { ControllerType::PD } else if kd.abs() < 1e-10 { ControllerType::PI } else { ControllerType::PID },
                n_inputs: plant_model.n_outputs,
                n_outputs: plant_model.n_inputs,
                is_discrete: false,
                parameters: params,
                design_method: format!("{:?}: {}", method, design_note),
                ..Default::default()
            };
            Ok(ControlModalityOutput { success: true, designed_controller: Some(controller), ..Default::default() })
        }

        ControlModalityAction::StateSpaceToTransferFunction { a_matrix, b_matrix, c_matrix, d_matrix } => {
            match ss_siso_to_tf(&a_matrix, &b_matrix, &c_matrix, &d_matrix) {
                Err(e) => Ok(ControlModalityOutput { success: false, error: Some(e), ..Default::default() }),
                Ok((numerator, denominator)) => {
                    let poles = poly_roots(&denominator).unwrap_or_default();
                    let zeros = if numerator.len() > 1 { poly_roots(&numerator).unwrap_or_default() } else { vec![] };
                    let relative_degree = denominator.len() as i32 - numerator.len() as i32;
                    let dc_gain = match (numerator.last(), denominator.last()) {
                        (Some(n0), Some(d0)) if d0.abs() > 1e-12 => Some(n0 / d0),
                        _ => None,
                    };
                    Ok(ControlModalityOutput {
                        success: true,
                        transfer_function: Some(TransferFunctionModel {
                            tf_id: executor.generate_id(),
                            name: "ss2tf".into(),
                            numerator, denominator, poles, zeros, dc_gain, relative_degree,
                            ..Default::default()
                        }),
                        ..Default::default()
                    })
                }
            }
        }

        ControlModalityAction::TransferFunctionToStateSpace { numerator, denominator, form } => {
            // Controllable canonical form
            let n = denominator.len().saturating_sub(1);
            if n == 0 {
                return Ok(ControlModalityOutput { success: false, error: Some("Denominator must have degree ≥ 1".into()), ..Default::default() });
            }
            let leading = denominator[0];
            let a_coeffs: Vec<f64> = denominator[1..].iter().map(|&c| -c / leading).collect();
            let m = numerator.len().saturating_sub(1);

            // Build companion (controllable canonical) A matrix
            let mut a = vec![vec![0.0f64; n]; n];
            for j in 0..n { a[0][j] = a_coeffs.get(j).copied().unwrap_or(0.0); }
            for i in 1..n { a[i][i - 1] = 1.0; }

            // B = [1, 0, 0, ..., 0]^T
            let mut b = vec![vec![0.0f64]; n];
            b[0][0] = 1.0 / leading;

            // C and D depend on numerator relative to denominator
            let mut c = vec![vec![0.0f64; n]];
            let mut d = vec![vec![0.0f64]];
            let rel_deg = n as i32 - m as i32;
            if rel_deg > 0 {
                // Strictly proper: D = 0, C from numerator coefficients
                for j in 0..n.min(numerator.len()) {
                    c[0][n - 1 - j] = numerator[numerator.len() - 1 - j] / leading;
                }
            } else {
                // Improper or biproper: D = b0/a0
                d[0][0] = numerator[0] / leading;
                let num_adj: Vec<f64> = numerator[1..].iter().enumerate()
                    .map(|(i, &ni)| ni - d[0][0] * denominator.get(i + 1).copied().unwrap_or(0.0))
                    .collect();
                for j in 0..n.min(num_adj.len()) {
                    c[0][n - 1 - j] = num_adj[num_adj.len() - 1 - j] / leading;
                }
            }

            Ok(ControlModalityOutput {
                success: true,
                state_space: Some(StateSpaceModel {
                    a_matrix: a, b_matrix: b, c_matrix: c, d_matrix: d,
                    state_names: (0..n).map(|i| format!("x{}", i + 1)).collect(),
                    input_names: vec!["u".into()],
                    output_names: vec!["y".into()],
                }),
                ..Default::default()
            })
        }

        ControlModalityAction::AnalyzeStability { system, method } => {
            let analysis_id = executor.generate_id();
            let mut stab = StabilityAnalysis {
                analysis_id,
                system_name: system.name.clone(),
                method: method.clone(),
                ..Default::default()
            };

            match method {
                StabilityMethod::RouthHurwitz => {
                    if let Some(ref tf) = system.transfer_function {
                        let (stable, routh_array) = PipelineExecutor::routh_hurwitz(&tf.denominator);
                        stab.is_stable = stable;
                        stab.stability_type = if stable { StabilityType::AsymptoticallyStable } else { StabilityType::Unstable };
                        stab.routh_array = Some(routh_array);
                        // Count sign changes in first column
                        let sign_changes = stab.routh_array.as_ref().unwrap().iter()
                            .map(|row| row[0].signum())
                            .collect::<Vec<_>>()
                            .windows(2)
                            .filter(|w| w[0] != w[1])
                            .count();
                        if sign_changes > 0 {
                            stab.notes.push(format!("{} sign changes in Routh first column → {} unstable poles", sign_changes, sign_changes));
                        }
                        // Extract poles
                        stab.poles = PipelineExecutor::compute_poles_from_denominator(&tf.denominator)
                            .into_iter()
                            .map(|p| {
                                let wn = (p.real * p.real + p.imag * p.imag).sqrt();
                                let zeta = if wn > 1e-12 { -p.real / wn } else { 0.0 };
                                Pole { is_stable: p.is_stable_pole(), natural_freq_rad_s: wn, damping_ratio: zeta, decay_rate: -p.real, is_dominant: false, value: p }
                            }).collect();
                    }
                }
                StabilityMethod::BodeAnalysis | StabilityMethod::NyquistCriterion => {
                    if let Some(ref tf) = system.transfer_function {
                        let (freqs, mag, phase) = PipelineExecutor::compute_bode(
                            &tf.numerator, &tf.denominator, 0.001, 1000.0, 500);
                        let (gm, pm, gc_freq, pc_freq) = PipelineExecutor::find_margins(&freqs, &mag, &phase);
                        stab.gain_margin_db = gm;
                        stab.phase_margin_deg = pm;
                        stab.gain_crossover_freq_rad_s = gc_freq;
                        stab.phase_crossover_freq_rad_s = pc_freq;
                        // A missing crossover means the loop gain never reaches that condition (margin unbounded), not instability.
                        // Bode margins are a valid stability test only for an open-loop-stable loop transfer function.
                        stab.is_stable = pm.map(|p| p > 0.0).unwrap_or(true) && gm.map(|g| g > 0.0).unwrap_or(true);
                        stab.notes.push("Bode/Nyquist margins assume a stable open-loop transfer function; missing crossovers are treated as unbounded margin".into());
                        stab.stability_type = if stab.is_stable { StabilityType::AsymptoticallyStable } else { StabilityType::Unstable };
                        stab.notes.push(format!("GM={:?}dB PM={:?}°", gm.map(|v| format!("{:.2}",v)), pm.map(|v| format!("{:.1}",v))));
                    }
                }
                StabilityMethod::EigenvalueAnalysis => {
                    if let Some(ref ss) = system.state_space {
                        // Exact characteristic polynomial (Faddeev–LeVerrier) + root finding, any order.
                        let discrete = system.is_discrete;
                        match eigenvalues(&ss.a_matrix) {
                            Some(ev) => {
                                let mut poles: Vec<Pole> = ev.into_iter().map(|p| pole_from(p, discrete)).collect();
                                mark_dominant(&mut poles, discrete);
                                if discrete { stab.notes.push("discrete-time: stable inside the unit circle; ω_n/ζ need the sample time and are left 0".into()); }
                                stab.poles = poles;
                            }
                            None => stab.notes.push("eigenvalues unavailable: A is not square or root finding did not converge (e.g. repeated eigenvalues)".into()),
                        }
                        let _n = ss.a_matrix.len();
                        stab.is_stable = !stab.poles.is_empty() && stab.poles.iter().all(|p| p.is_stable);
                        stab.stability_type = if stab.poles.is_empty() { StabilityType::Unknown } else if stab.is_stable { StabilityType::AsymptoticallyStable } else { StabilityType::Unstable };
                    }
                }
                _ => {
                    // LLM-assisted for Lyapunov, Circle, etc. NOT a numerical proof: say so in the output.
                    stab.notes.push(format!("method {:?} is not implemented numerically; any verdict below is an LLM judgement, not a computed result", method));
                    let prompt = format!(r#"
Analyze stability of control system "{}" using {:?} method.
Return ONLY valid JSON: {{"is_stable": true, "stability_type": "AsymptoticallyStable|Unstable|MarginallyStable", "notes": ["note1"]}}"#,
                        system.name, method);
                    if let Ok(raw) = executor.llm_zero_shot(&prompt, 150).await {
                        let json_str = PipelineExecutor::extract_json_object_static(&raw);
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json_str) {
                            stab.is_stable = v["is_stable"].as_bool().unwrap_or(false);
                            stab.stability_type = match v["stability_type"].as_str().unwrap_or("Unknown") {
                                "AsymptoticallyStable"  => StabilityType::AsymptoticallyStable,
                                "Unstable"              => StabilityType::Unstable,
                                "MarginallySatble"      => StabilityType::MarginallySatble,
                                _                       => StabilityType::Unknown,
                            };
                            stab.notes = v["notes"].as_array().map(|arr| arr.iter().filter_map(|n| n.as_str().map(String::from)).collect()).unwrap_or_default();
                        }
                    }
                }
            }
            Ok(ControlModalityOutput { success: true, stability_result: Some(stab), ..Default::default() })
        }

        ControlModalityAction::FrequencyResponse { system, freq_start_rad_s, freq_end_rad_s, points, response_type } => {
            let fr_id = executor.generate_id();
            let pair: Option<(Vec<f64>, Vec<f64>)> = if let Some(ref tf) = system.transfer_function {
                Some((tf.numerator.clone(), tf.denominator.clone()))
            } else if let Some(ref ss) = system.state_space {
                ss_siso_to_tf(&ss.a_matrix, &ss.b_matrix, &ss.c_matrix, &ss.d_matrix).ok()
            } else { None };
            let (num_c, den_c) = match pair {
                Some(p) => p,
                None => return Ok(ControlModalityOutput { success: false, error: Some("frequency response needs a transfer function or a SISO state-space model; no response is fabricated".into()), ..Default::default() }),
            };
            let (freqs, mag, phase) = PipelineExecutor::compute_bode(&num_c, &den_c, freq_start_rad_s, freq_end_rad_s, points);

            // −3 dB bandwidth is relative to the low-frequency magnitude, not to 0 dB.
            let ref_db = mag.first().copied().unwrap_or(0.0);
            let bw = freqs.iter().zip(mag.iter()).find(|(_, &m)| m <= ref_db - 3.0).map(|(&w, _)| w);
            let (res_peak, res_freq) = mag.iter().zip(freqs.iter()).fold((f64::NEG_INFINITY, 0.0), |(mp, mf), (&m, &f)| if m > mp { (m, f) } else { (mp, mf) });
            let res_rel = res_peak - ref_db;
            // Roll-off: slope over the last decade (or the whole span when shorter), dB per decade.
            let roll_off = if mag.len() >= 2 {
                let last = mag.len() - 1;
                let target_f = freqs[last] / 10.0;
                let j = freqs.iter().position(|&f| f >= target_f).unwrap_or(0).min(last - 1);
                let decades = (freqs[last] / freqs[j]).log10();
                if decades > 1e-9 && mag[last].is_finite() && mag[j].is_finite() { Some((mag[last] - mag[j]) / decades) } else { None }
            } else { None };

            Ok(ControlModalityOutput {
                success: true,
                frequency_response: Some(FrequencyResponseData {
                    freq_response_id: fr_id,
                    frequencies_rad_s: freqs,
                    magnitude_db: mag,
                    phase_deg: phase,
                    response_type,
                    bandwidth_3db_rad_s: bw,
                    resonant_freq_rad_s: if res_rel > 1e-6 { Some(res_freq) } else { None },
                    resonant_peak_db: if res_rel > 1e-6 { Some(res_rel) } else { None },
                    roll_off_db_per_decade: roll_off,
                }),
                ..Default::default()
            })
        }

        ControlModalityAction::SimulateResponse { graph_id, input_type, duration_sec, dt_sec, initial_state, state_space } => {
            let graph = executor.load_graph(graph_id)?;
            let ss: Option<StateSpaceModel> = state_space.or_else(|| {
                graph.nodes.iter()
                    .filter(|n| matches!(n.node_type, ControlNodeType::PlantNode))
                    .find_map(|n| serde_json::from_str::<StateSpaceModel>(&n.content).ok())
            });
            let ss = match ss {
                Some(m) => m,
                None => return Ok(ControlModalityOutput { success: false, error: Some("no state-space model available: pass `state_space`, or store the plant's StateSpaceModel JSON in a PlantNode's content; no default plant is substituted".into()), ..Default::default() }),
            };
            if !(dt_sec > 0.0) || !(duration_sec > 0.0) || duration_sec / dt_sec > 2_000_000.0 {
                return Ok(ControlModalityOutput { success: false, error: Some("need duration_sec > 0, dt_sec > 0 and at most 2,000,000 steps".into()), ..Default::default() });
            }
            let mut result = PipelineExecutor::simulate_lti(&ss, &input_type, duration_sec, dt_sec, initial_state.as_deref());
            result.sim_id = executor.generate_id();
            Ok(ControlModalityOutput { success: true, simulation_result: Some(result), ..Default::default() })
        }

        ControlModalityAction::DesignLQR { a_matrix, b_matrix, q_matrix, r_matrix } => {
            let lqr_id = executor.generate_id();
            // Continuous-time LQR: Riccati ODE integrated to its stationary solution (see solve_care_lqr).
            match solve_care_lqr(&a_matrix, &b_matrix, &q_matrix, &r_matrix) {
                Err(e) => Ok(ControlModalityOutput { success: false, error: Some(e), ..Default::default() }),
                Ok((k_gain, p, cl_poles)) => {
                    // optimal_cost = trace(P): the optimal cost summed over the n unit-vector initial states
                    // (for a specific x0 the cost is x0ᵀ P x0).
                    let optimal_cost: f64 = (0..p.len()).map(|i| p[i][i]).sum();
                    Ok(ControlModalityOutput {
                        success: true,
                        lqr_solution: Some(LQRSolution { lqr_id, feedback_gain: k_gain, riccati_solution: p, closed_loop_poles: cl_poles, optimal_cost }),
                        ..Default::default()
                    })
                }
            }
        }

        ControlModalityAction::DesignMPC { plant_model, prediction_horizon, control_horizon, constraints, objective } => {
            // No QP solver exists in this pipeline: answer honestly instead of returning an empty "MPC" controller.
            let _ = (&plant_model, prediction_horizon, control_horizon, &constraints, &objective);
            Ok(ControlModalityOutput { success: false, error: Some("MPC design is not implemented: no quadratic-programming solver is available, so no controller is produced".into()), ..Default::default() })
        }

        ControlModalityAction::IdentifySystem { input_data, output_data, dt_sec, method, model_order } => {
            let model_id = executor.generate_id();
            let n = input_data.len().min(output_data.len());
            if n < 10 {
                return Ok(ControlModalityOutput { success: false, error: Some("Need ≥10 samples for system identification".into()), ..Default::default() });
            }

            let order = model_order.unwrap_or(2) as usize;

            // ARX: minimize ||Y - Phi*theta||^2
            // Build regressor matrix Phi for ARX(na, nb)
            let na = order; let nb = order;
            let n_reg = na + nb;
            let n_valid = n - na.max(nb);
            if n_valid < n_reg {
                return Ok(ControlModalityOutput { success: false, error: Some("Not enough data for this model order".into()), ..Default::default() });
            }

            // Phi rows: [-y(k-1)...-y(k-na), u(k-1)...u(k-nb)]
            let mut phi = vec![vec![0.0f64; n_reg]; n_valid];
            let mut y_vec = vec![0.0f64; n_valid];
            for k in 0..n_valid {
                let ki = k + na.max(nb);
                y_vec[k] = output_data[ki];
                for i in 0..na { phi[k][i] = -output_data[ki - i - 1]; }
                for i in 0..nb { phi[k][na + i] = input_data[ki - i - 1]; }
            }

            // Least squares: theta = (Phi'Phi)^{-1} Phi'Y
            // For small n_reg, compute via normal equations
            let mut phi_t_phi = vec![vec![0.0f64; n_reg]; n_reg];
            let mut phi_t_y = vec![0.0f64; n_reg];
            for k in 0..n_valid {
                for i in 0..n_reg {
                    phi_t_y[i] += phi[k][i] * y_vec[k];
                    for j in 0..n_reg { phi_t_phi[i][j] += phi[k][i] * phi[k][j]; }
                }
            }

            // Solve via Gaussian elimination (small system)
            let theta = match gaussian_elimination(&phi_t_phi, &phi_t_y) {
                Some(t) => t,
                None => return Ok(ControlModalityOutput { success: false, error: Some("normal equations are singular: the data are not persistently exciting for this model order; no parameters are fabricated".into()), ..Default::default() }),
            };

            // Compute fit
            let y_pred: Vec<f64> = (0..n_valid).map(|k| phi[k].iter().zip(theta.iter()).map(|(p, t)| p * t).sum()).collect();
            let y_mean = y_vec.iter().sum::<f64>() / n_valid as f64;
            let ss_tot: f64 = y_vec.iter().map(|&y| (y - y_mean).powi(2)).sum();
            let ss_res: f64 = y_vec.iter().zip(y_pred.iter()).map(|(y, yp)| (y - yp).powi(2)).sum();
            if ss_tot <= 1e-12 {
                return Ok(ControlModalityOutput { success: false, error: Some("output has no variance: fit percent is undefined".into()), ..Default::default() });
            }
            let fit = 100.0 * (1.0 - ss_res / ss_tot);

            // Convert ARX theta to transfer function: G(z) = B(z^{-1}) / A(z^{-1})
            let a_coeffs: Vec<f64> = std::iter::once(1.0).chain(theta[..na].iter().copied()).collect();
            let b_coeffs: Vec<f64> = theta[na..na+nb].to_vec();

            let n_params = n_reg as f64;
            let ss_res_pos = ss_res.max(1e-300);
            let aic = (n_valid as f64) * (ss_res_pos / n_valid as f64).ln() + 2.0 * n_params;
            let bic = (n_valid as f64) * (ss_res_pos / n_valid as f64).ln() + n_params * (n_valid as f64).ln();

            Ok(ControlModalityOutput {
                success: true,
                identified_model: Some(IdentifiedModel {
                    model_id,
                    method,
                    model_order: order as u32,
                    fit_percent: fit.max(0.0),
                    aic, bic,
                    transfer_function: Some(TransferFunctionModel {
                        tf_id: executor.generate_id(),
                        name: "identified".into(),
                        numerator: b_coeffs.clone(),
                        denominator: a_coeffs.clone(),
                        is_discrete: true,
                        sample_time: Some(dt_sec),
                        poles: poly_roots(&a_coeffs).unwrap_or_default(),
                        zeros: if b_coeffs.len() > 1 { poly_roots(&b_coeffs).unwrap_or_default() } else { vec![] },
                        ..Default::default()
                    }),
                    residuals_whiteness_p_value: None, // Ljung-Box test not implemented — absent, not fabricated
                    ..Default::default()
                }),
                ..Default::default()
            })
        }

        ControlModalityAction::Verify { graph_id, specifications } => {
            let graph = executor.load_graph(graph_id)?;
            // Pull stability/performance data from graph nodes
            let stab_node = graph.nodes.iter().find(|n| matches!(n.node_type, ControlNodeType::StabilityAnalysisNode));
            let sim_node = graph.nodes.iter().find(|n| matches!(n.node_type, ControlNodeType::SimulationResultNode));

            let results: Vec<VerificationResult> = specifications.iter().map(|spec| {
                let (measured, passed) = match spec.spec_type {
                    ControlSpecType::Stability => {
                        let v = stab_node.and_then(|n| n.is_stable).unwrap_or(false);
                        (if v { 1.0 } else { 0.0 }, v)
                    }
                    ControlSpecType::GainMargin => {
                        let gm = stab_node.and_then(|n| n.gain_margin_db).unwrap_or(0.0);
                        let bound = spec.bound_value;
                        (gm, match spec.bound_type { BoundType::Minimum => gm >= bound, BoundType::Maximum => gm <= bound, _ => (gm - bound).abs() < 1e-6 })
                    }
                    ControlSpecType::PhaseMargin => {
                        let pm = stab_node.and_then(|n| n.phase_margin_deg).unwrap_or(0.0);
                        let bound = spec.bound_value;
                        (pm, match spec.bound_type { BoundType::Minimum => pm >= bound, _ => pm <= bound })
                    }
                    ControlSpecType::Bandwidth => {
                        let bw = stab_node.and_then(|n| n.bandwidth_hz).unwrap_or(0.0);
                        let bound = spec.bound_value;
                        (bw, match spec.bound_type { BoundType::Minimum => bw >= bound, _ => bw <= bound })
                    }
                    _ => (0.0, false),
                };
                // A value the graph does not carry is NOT measured: the 0.0 above is a placeholder, never evidence.
                let available = match spec.spec_type {
                    ControlSpecType::Stability => stab_node.and_then(|n| n.is_stable).is_some(),
                    ControlSpecType::GainMargin => stab_node.and_then(|n| n.gain_margin_db).is_some(),
                    ControlSpecType::PhaseMargin => stab_node.and_then(|n| n.phase_margin_deg).is_some(),
                    ControlSpecType::Bandwidth => stab_node.and_then(|n| n.bandwidth_hz).is_some(),
                    _ => false,
                };
                let passed = passed && available;
                let margin = match spec.bound_type {
                    BoundType::Minimum => measured - spec.bound_value,
                    BoundType::Maximum => spec.bound_value - measured,
                    BoundType::Equality => -(measured - spec.bound_value).abs(),
                };
                VerificationResult { spec_id: spec.spec_id, passed, measured_value: measured, bound_value: spec.bound_value, margin, notes: if !available { "NOT MEASURED: the graph carries no value for this specification (measured_value 0.0 is a placeholder, not evidence)".to_string() } else if passed { "Specification met".to_string() } else { format!("Specification violated: measured={:.4}", measured) } }
            }).collect();

            Ok(ControlModalityOutput { success: true, verification_results: Some(results), ..Default::default() })
        }

        ControlModalityAction::QueryGraph { graph_id, query } => {
            let graph = executor.load_graph(graph_id)?;
            let result = match query {
                ControlGraphQuery::NodeDetail { node_id } => {
                    let node = graph.nodes.iter().find(|n| n.node_id == node_id);
                    let incoming: Vec<_> = graph.edges.iter().filter(|e| e.to_node == node_id).collect();
                    let outgoing: Vec<_> = graph.edges.iter().filter(|e| e.from_node == node_id).collect();
                    serde_json::json!({ "node": node, "incoming": incoming, "outgoing": outgoing })
                }
                ControlGraphQuery::ClosedLoopPoles => {
                    let poles: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, ControlNodeType::PoleNode)).collect();
                    let unstable: Vec<_> = poles.iter().filter(|n| n.is_stable == Some(false)).collect();
                    serde_json::json!({ "poles": poles, "unstable_count": unstable.len() })
                }
                ControlGraphQuery::StabilityResults => {
                    let stabs: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, ControlNodeType::StabilityAnalysisNode)).collect();
                    serde_json::json!({ "stability_analyses": stabs })
                }
                ControlGraphQuery::PerformanceSummary => {
                    let sims: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, ControlNodeType::SimulationResultNode)).collect();
                    let freq_resps: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, ControlNodeType::FrequencyResponseNode)).collect();
                    serde_json::json!({ "simulations": sims, "frequency_responses": freq_resps })
                }
                ControlGraphQuery::CrossModalLinks { node_id } => {
                    let links: Vec<_> = graph.edges.iter().filter(|e| (e.from_node == node_id || e.to_node == node_id) && matches!(e.edge_type, ControlEdgeType::PlantIsKinematicChain | ControlEdgeType::FedByIMUSensor | ControlEdgeType::CommandsRobot3D | ControlEdgeType::TrackFromRadar | ControlEdgeType::NavigationFromSonar | ControlEdgeType::WaypointFromGeo | ControlEdgeType::ImplementedInCode | ControlEdgeType::ProvenInMath | ControlEdgeType::LinkQualityFromEM)).collect();
                    serde_json::json!({ "cross_modal_links": links })
                }
                ControlGraphQuery::PIDControllers => {
                    let pids: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, ControlNodeType::ControllerNode) && (n.kp.is_some() || n.ki.is_some() || n.kd.is_some())).collect();
                    serde_json::json!({ "pid_controllers": pids })
                }
                ControlGraphQuery::ActuatorSaturation => {
                    let acts: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, ControlNodeType::ActuatorNode)).collect();
                    serde_json::json!({ "actuators": acts })
                }
                ControlGraphQuery::SensorList => {
                    let sensors: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, ControlNodeType::SensorNode)).collect();
                    serde_json::json!({ "sensors": sensors })
                }
                ControlGraphQuery::VerificationResults => {
                    let vrs: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, ControlNodeType::VerificationResultNode)).collect();
                    let fails: Vec<_> = vrs.iter().filter(|n| n.is_stable == Some(false)).collect();
                    serde_json::json!({ "results": vrs, "fail_count": fails.len() })
                }
                ControlGraphQuery::AGIActivity => serde_json::json!({ "is_active": false }),
                ControlGraphQuery::AllNodes => serde_json::json!({ "nodes": graph.nodes }),
                ControlGraphQuery::AllEdges => serde_json::json!({ "edges": graph.edges }),
            };
            Ok(ControlModalityOutput { success: true, query_result: Some(result), ..Default::default() })
        }

        ControlModalityAction::GetGraph { graph_id } => {
            let graph = executor.load_graph(graph_id)?;
            Ok(ControlModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        ControlModalityAction::TriggerSemanticHook { graph_id, hook } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            match hook {
                ControlSemanticHook::OnGraphCreated => { graph.state = GraphStateType::SemanticEnriched; }
                ControlSemanticHook::OnInferRelationships => {
                    let new_edges = executor.infer_semantic_relationships(&graph.nodes).await;
                    let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                    let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                    for (from, to, etype, reason) in new_edges {
                        if valid.contains(&from) && valid.contains(&to) && from != to {
                            graph.edges.push(ControlGraphEdge { edge_id: next_eid, from_node: from, to_node: to, edge_type: etype, weight: 1.0, provenance: EdgeProvenance::DerivedFromHook, version: 1, properties: { let mut p = HashMap::new(); p.insert("reason".into(), serde_json::json!(reason)); p }, ..Default::default() });
                            next_eid += 1;
                        }
                    }
                }
                ControlSemanticHook::OnEdgeCompletion => {
                    let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                    graph.edges.retain(|e| e.from_node == e.to_node || (valid.contains(&e.from_node) && valid.contains(&e.to_node)));
                }
                ControlSemanticHook::OnCrossModalityLink { target_modality, target_graph_id } => {
                    graph.state = GraphStateType::CrossLinked;
                    graph.version += 1;
                    graph.version_notes.push(VersionNote { version: graph.version, note: format!("Cross-linked to {} (graph {})", target_modality, target_graph_id), step_index: None, timestamp: now.clone(), change_type: ChangeType::CrossLinked });
                }
            }
            graph.updated_at = now;
            executor.save_graph(&graph)?;
            Ok(ControlModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        ControlModalityAction::ExportProduct { graph_id, format } => {
            // HONESTY (audit X1): this arm used to return a `/tmp/..` path with `success: true` WITHOUT writing anything. Nothing is written, so it FAILS.
            #[allow(unreachable_code, unused_variables)]
            { return Ok(ControlModalityOutput { success: false, error: Some("control export is not implemented: nothing was written (no file exists at any path this arm used to report)".to_string()), ..Default::default() }); }
            let ext = match &format {
                ControlExportFormat::MATLAB_M         => "m",
                ControlExportFormat::Python_Control   => "py",
                ControlExportFormat::Simulink_SLX     => "slx",
                ControlExportFormat::ACADO            => "cpp",
                ControlExportFormat::Casadi_Python    => "py",
                ControlExportFormat::JSON_Schema      => "json",
                ControlExportFormat::CSV_Bode         => "csv",
                ControlExportFormat::CSV_StepResponse => "csv",
                ControlExportFormat::Custom(s)        => "dat",
            };
            // Only the JSON export is implemented; every other format answers honestly instead of
            // returning a path to a file that was never written.
            if !matches!(format, ControlExportFormat::JSON_Schema) {
                return Ok(ControlModalityOutput { success: false, error: Some(format!("export format {:?} is not implemented; no file was written", format)), ..Default::default() });
            }
            let graph = executor.load_graph(graph_id)?;
            let export_path = std::env::temp_dir().join(format!("ctrl_export_{}.{}", graph_id, ext)).to_string_lossy().to_string();
            let body = serde_json::to_string_pretty(&graph).map_err(|e| format!("export serialization failed: {}", e))?;
            std::fs::write(&export_path, body).map_err(|e| format!("export write failed: {}", e))?;
            Ok(ControlModalityOutput { success: true, export_path: Some(export_path), ..Default::default() })
        }

        ControlModalityAction::StreamToUI { graph_id, .. } => {
            Ok(ControlModalityOutput { success: true, graph_id: Some(graph_id), ..Default::default() })
        }

        ControlModalityAction::HeadlessProcess { graph_id, operations } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            for op in operations {
                match op {
                    ControlOperation::ReanalyzeStability => {
                        graph.version_notes.push(VersionNote { version: graph.version + 1, note: "Stability re-analyzed headless".into(), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                        graph.version += 1;
                    }
                    ControlOperation::RetunePID { method } => {
                        graph.version_notes.push(VersionNote { version: graph.version + 1, note: format!("PID retuned: {:?}", method), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                        graph.version += 1;
                    }
                    ControlOperation::ResimulateStep => {
                        graph.version_notes.push(VersionNote { version: graph.version + 1, note: "Step response re-simulated".into(), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                        graph.version += 1;
                    }
                    ControlOperation::CrossLinkToKinematics { kin_graph_id } => {
                        let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        let root_id = graph.root_node_id;
                        graph.edges.push(ControlGraphEdge {
                            edge_id: next_eid, from_node: root_id, to_node: root_id,
                            edge_type: ControlEdgeType::PlantIsKinematicChain, weight: 1.0,
                            provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                            properties: { let mut p = HashMap::new(); p.insert("kin_graph_id".into(), serde_json::json!(kin_graph_id)); p },
                            ..Default::default()
                        });
                    }
                    ControlOperation::CrossLinkToIMU { imu_graph_id } => {
                        let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        let root_id = graph.root_node_id;
                        graph.edges.push(ControlGraphEdge {
                            edge_id: next_eid, from_node: root_id, to_node: root_id,
                            edge_type: ControlEdgeType::FedByIMUSensor, weight: 1.0,
                            provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                            properties: { let mut p = HashMap::new(); p.insert("imu_graph_id".into(), serde_json::json!(imu_graph_id)); p },
                            ..Default::default()
                        });
                    }
                    ControlOperation::ExportToMatlab => {
                        // Not implemented: refuse instead of silently succeeding.
                        return Ok(ControlModalityOutput { success: false, error: Some("ExportToMatlab is not implemented".into()), ..Default::default() });
                    }
                }
            }
            graph.updated_at = now;
            executor.save_graph(&graph)?;
            Ok(ControlModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// HELPER: static version of extract_json_object for use in non-async contexts
// ─────────────────────────────────────────────────────────────────────────────

impl PipelineExecutor {
    fn extract_json_object_static(raw: &str) -> String {
        if let (Some(s), Some(e)) = (raw.find('{'), raw.rfind('}')) { raw[s..=e].to_string() } else { "{}".to_string() }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GAUSSIAN ELIMINATION — for system identification least squares
// ─────────────────────────────────────────────────────────────────────────────

fn gaussian_elimination(a: &[Vec<f64>], b: &[f64]) -> Option<Vec<f64>> {
    let n = a.len();
    if n == 0 || b.len() != n { return None; }

    // Augmented matrix [A | b]
    let mut aug: Vec<Vec<f64>> = a.iter().enumerate().map(|(i, row)| {
        let mut r = row.clone();
        r.push(b[i]);
        r
    }).collect();

    for col in 0..n {
        // Partial pivot
        let max_row = (col..n).max_by(|&r1, &r2| aug[r1][col].abs().partial_cmp(&aug[r2][col].abs()).unwrap_or(std::cmp::Ordering::Equal))?;
        aug.swap(col, max_row);
        let pivot = aug[col][col];
        if pivot.abs() < 1e-12 { return None; }
        for j in col..=n { aug[col][j] /= pivot; }
        for row in 0..n {
            if row != col {
                let factor = aug[row][col];
                for j in col..=n { aug[row][j] -= factor * aug[col][j]; }
            }
        }
    }

    Some((0..n).map(|i| aug[i][n]).collect())
}


// ─────────────────────────────────────────────────────────────────────────────
// PURE-MATH HELPERS (honesty pass 2026-10-31, fork G): real linear algebra and
// polynomial root finding so no result is canned or silently empty.
// UNCOMPILED pending the EU build gate.
// ─────────────────────────────────────────────────────────────────────────────

fn mat_mul(a: &[Vec<f64>], b: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = a.len();
    let m = b.first().map(|r| r.len()).unwrap_or(0);
    let mut out = vec![vec![0.0f64; m]; n];
    for i in 0..n {
        for l in 0..b.len().min(a[i].len()) {
            let v = a[i][l];
            if v == 0.0 { continue; }
            for j in 0..m { out[i][j] += v * b[l][j]; }
        }
    }
    out
}

fn mat_transpose(a: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = a.len();
    let m = a.first().map(|r| r.len()).unwrap_or(0);
    let mut t = vec![vec![0.0f64; n]; m];
    for i in 0..n { for j in 0..m { t[j][i] = a[i][j]; } }
    t
}

/// Gauss-Jordan inverse with partial pivoting; None when singular or not square.
fn mat_inverse(a: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = a.len();
    if n == 0 || a.iter().any(|r| r.len() != n) { return None; }
    let mut aug: Vec<Vec<f64>> = (0..n).map(|i| {
        let mut r = a[i].clone();
        r.extend((0..n).map(|j| if i == j { 1.0 } else { 0.0 }));
        r
    }).collect();
    for col in 0..n {
        let mut piv = col;
        for r in (col + 1)..n { if aug[r][col].abs() > aug[piv][col].abs() { piv = r; } }
        if aug[piv][col].abs() < 1e-12 { return None; }
        aug.swap(col, piv);
        let p = aug[col][col];
        for j in 0..(2 * n) { aug[col][j] /= p; }
        for r in 0..n {
            if r != col {
                let f = aug[r][col];
                if f != 0.0 { for j in 0..(2 * n) { aug[r][j] -= f * aug[col][j]; } }
            }
        }
    }
    Some(aug.into_iter().map(|r| r[n..].to_vec()).collect())
}

fn trim_leading_zeros(mut c: Vec<f64>) -> Vec<f64> {
    let scale = c.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    let eps = 1e-12 * scale.max(1e-300);
    while c.len() > 1 && c[0].abs() <= eps { c.remove(0); }
    c
}

/// Faddeev–LeVerrier: characteristic polynomial coefficients det(sI−A) =
/// c[0] s^n + c[1] s^(n−1) + … (c[0] = 1) and the adjugate coefficient matrices
/// M_0..M_(n−1) with adj(sI−A) = Σ M_k s^(n−1−k).
fn faddeev_leverrier(a: &[Vec<f64>]) -> Option<(Vec<f64>, Vec<Vec<Vec<f64>>>)> {
    let n = a.len();
    if n == 0 || a.iter().any(|r| r.len() != n) { return None; }
    let ident: Vec<Vec<f64>> = (0..n).map(|i| (0..n).map(|j| if i == j { 1.0 } else { 0.0 }).collect()).collect();
    let mut c = vec![1.0f64];
    let mut ms: Vec<Vec<Vec<f64>>> = vec![ident];
    for k in 1..=n {
        let am = mat_mul(a, &ms[k - 1]);
        let tr: f64 = (0..n).map(|i| am[i][i]).sum();
        let ck = -tr / k as f64;
        c.push(ck);
        if k < n {
            let mut mk = am;
            for i in 0..n { mk[i][i] += ck; }
            ms.push(mk);
        }
    }
    Some((c, ms))
}

/// SISO state-space → (numerator, denominator), highest power first.
fn ss_siso_to_tf(a: &[Vec<f64>], b: &[Vec<f64>], c: &[Vec<f64>], d: &[Vec<f64>]) -> Result<(Vec<f64>, Vec<f64>), String> {
    let n = a.len();
    if n == 0 || b.len() != n || b.iter().any(|r| r.len() != 1) || c.len() != 1 || c[0].len() != n {
        return Err("only SISO state-space models (A n×n, B n×1, C 1×n) convert to one transfer function; convert MIMO per input/output channel".to_string());
    }
    let dval = d.first().and_then(|r| r.first()).copied().unwrap_or(0.0);
    let (den, ms) = faddeev_leverrier(a).ok_or_else(|| "A must be square and non-empty".to_string())?;
    let mut num = vec![0.0f64; n + 1];
    for j in 0..=n {
        num[j] = dval * den[j];
        if j >= 1 {
            let mb = mat_mul(&ms[j - 1], b);
            let cmb: f64 = (0..n).map(|i| c[0][i] * mb[i][0]).sum();
            num[j] += cmb;
        }
    }
    Ok((trim_leading_zeros(num), den))
}

/// Strictly-proper-or-proper transfer function → controllable canonical form (SISO).
fn tf_to_ss_siso(num: &[f64], den: &[f64]) -> Result<StateSpaceModel, String> {
    let den = trim_leading_zeros(den.to_vec());
    let num = trim_leading_zeros(num.to_vec());
    let n = den.len().saturating_sub(1);
    if n == 0 { return Err("denominator must have degree ≥ 1".to_string()); }
    if num.len() > den.len() { return Err("improper transfer function (numerator degree > denominator degree)".to_string()); }
    let lead = den[0];
    if lead.abs() < 1e-300 { return Err("zero leading denominator coefficient".to_string()); }
    let a_c: Vec<f64> = den[1..].iter().map(|v| v / lead).collect();
    let mut b_c = vec![0.0f64; n + 1];
    let off = (n + 1) - num.len();
    for (i, v) in num.iter().enumerate() { b_c[off + i] = v / lead; }
    let mut a_m = vec![vec![0.0f64; n]; n];
    for j in 0..n { a_m[0][j] = -a_c[j]; }
    for i in 1..n { a_m[i][i - 1] = 1.0; }
    let mut b_m = vec![vec![0.0f64]; n];
    b_m[0][0] = 1.0;
    let c_m = vec![(0..n).map(|j| b_c[j + 1] - a_c[j] * b_c[0]).collect::<Vec<f64>>()];
    let d_m = vec![vec![b_c[0]]];
    Ok(StateSpaceModel {
        a_matrix: a_m, b_matrix: b_m, c_matrix: c_m, d_matrix: d_m,
        state_names: (1..=n).map(|i| format!("x{}", i)).collect(),
        input_names: vec!["u".into()],
        output_names: vec!["y".into()],
    })
}

fn cx_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) { (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0) }
fn cx_div(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    let d = b.0 * b.0 + b.1 * b.1;
    ((a.0 * b.0 + a.1 * b.1) / d, (a.1 * b.0 - a.0 * b.1) / d)
}

/// Polynomial roots (coefficients highest power first) by Durand–Kerner.
/// None when the iteration does not converge (e.g. repeated roots) — absent, never guessed.
fn poly_roots(coeffs: &[f64]) -> Option<Vec<ComplexNumber>> {
    let mut c = trim_leading_zeros(coeffs.to_vec());
    let mut zero_roots = 0usize;
    while c.len() > 1 && c.last().map_or(false, |v| v.abs() < 1e-14) { c.pop(); zero_roots += 1; }
    let deg = c.len().saturating_sub(1);
    let mut out: Vec<ComplexNumber> = (0..zero_roots).map(|_| ComplexNumber { real: 0.0, imag: 0.0 }).collect();
    if deg == 0 { return Some(out); }
    let lead = c[0];
    if lead.abs() < 1e-300 { return None; }
    let mon: Vec<f64> = c.iter().map(|v| v / lead).collect();
    let r = 1.0 + mon[1..].iter().fold(0.0f64, |m, v| m.max(v.abs()));
    let mut z: Vec<(f64, f64)> = (0..deg).map(|k| {
        let ang = 2.0 * std::f64::consts::PI * (k as f64) / (deg as f64) + 0.4;
        (0.5 * r * ang.cos(), 0.5 * r * ang.sin())
    }).collect();
    let eval = |x: (f64, f64)| -> (f64, f64) {
        let mut acc = (1.0f64, 0.0f64);
        for &co in &mon[1..] { acc = cx_mul(acc, x); acc.0 += co; }
        acc
    };
    let mut converged = false;
    for _ in 0..3000 {
        let mut maxd = 0.0f64;
        for i in 0..deg {
            let p = eval(z[i]);
            let mut den = (1.0f64, 0.0f64);
            for j in 0..deg { if j != i { den = cx_mul(den, (z[i].0 - z[j].0, z[i].1 - z[j].1)); } }
            if den.0.abs() + den.1.abs() < 1e-300 { continue; }
            let delta = cx_div(p, den);
            z[i].0 -= delta.0;
            z[i].1 -= delta.1;
            maxd = maxd.max(delta.0.abs() + delta.1.abs());
        }
        if !maxd.is_finite() { return None; }
        if maxd < 1e-13 * r.max(1.0) { converged = true; break; }
    }
    if !converged { return None; }
    for (re, im) in z {
        out.push(ComplexNumber {
            real: if re.abs() < 1e-12 * r { 0.0 } else { re },
            imag: if im.abs() < 1e-10 * r { 0.0 } else { im },
        });
    }
    Some(out)
}

/// Eigenvalues of a square matrix via its characteristic polynomial (exact coefficients,
/// then root finding). None when A is not square or root finding did not converge.
fn eigenvalues(a: &[Vec<f64>]) -> Option<Vec<ComplexNumber>> {
    let (cp, _) = faddeev_leverrier(a)?;
    poly_roots(&cp)
}

fn pole_from(p: ComplexNumber, discrete: bool) -> Pole {
    let wn = (p.real * p.real + p.imag * p.imag).sqrt();
    if discrete {
        // z-plane pole: stable inside the unit circle; ω_n/ζ need the sample time (not in the model) → 0 and noted by the caller.
        return Pole { is_stable: wn < 1.0, natural_freq_rad_s: 0.0, damping_ratio: 0.0, decay_rate: 0.0, is_dominant: false, value: p };
    }
    let zeta = if wn > 1e-12 { -p.real / wn } else { 0.0 };
    Pole { is_stable: p.real < 0.0, natural_freq_rad_s: wn, damping_ratio: zeta, decay_rate: -p.real, is_dominant: false, value: p }
}

fn mark_dominant(poles: &mut [Pole], discrete: bool) {
    let key = |p: &Pole| if discrete { p.value.real.hypot(p.value.imag) } else { p.value.real };
    let best = poles.iter().filter(|p| p.is_stable).map(|p| key(p)).fold(f64::NEG_INFINITY, f64::max);
    if best.is_finite() { for p in poles.iter_mut() { if p.is_stable && (key(p) - best).abs() < 1e-9 { p.is_dominant = true; } } }
}

/// Continuous-time LQR: integrate the Riccati ODE P' = AᵀP + PA − P B R⁻¹ Bᵀ P + Q from P = 0
/// (RK4) until stationary. Returns (K, P, closed-loop poles) or an honest error.
fn solve_care_lqr(a: &[Vec<f64>], b: &[Vec<f64>], q: &[Vec<f64>], r: &[Vec<f64>]) -> Result<(Vec<Vec<f64>>, Vec<Vec<f64>>, Vec<ComplexNumber>), String> {
    let n = a.len();
    let m = b.first().map(|x| x.len()).unwrap_or(0);
    if n == 0 || m == 0 || a.iter().any(|x| x.len() != n) || b.len() != n || b.iter().any(|x| x.len() != m)
        || q.len() != n || q.iter().any(|x| x.len() != n) || r.len() != m || r.iter().any(|x| x.len() != m) {
        return Err("dimension mismatch: need A n×n, B n×m, Q n×n, R m×m".to_string());
    }
    let rinv = mat_inverse(r).ok_or_else(|| "R must be invertible (positive definite)".to_string())?;
    let bt = mat_transpose(b);
    let s = mat_mul(&mat_mul(b, &rinv), &bt);
    let at = mat_transpose(a);
    let deriv = |p: &Vec<Vec<f64>>| -> Vec<Vec<f64>> {
        let t1 = mat_mul(&at, p);
        let t2 = mat_mul(p, a);
        let t3 = mat_mul(&mat_mul(p, &s), p);
        let mut o = vec![vec![0.0f64; n]; n];
        for i in 0..n { for j in 0..n { o[i][j] = t1[i][j] + t2[i][j] - t3[i][j] + q[i][j]; } }
        o
    };
    let norm = a.iter().map(|row| row.iter().map(|v| v.abs()).sum::<f64>()).fold(0.0f64, f64::max);
    let h = 0.02 / (1.0 + norm);
    let max_steps = ((2.0e8 / ((n * n * n) as f64 * 12.0)) as usize).clamp(2_000, 400_000);
    let mut p = vec![vec![0.0f64; n]; n];
    let mut converged = false;
    for step in 0..max_steps {
        let k1 = deriv(&p);
        let add = |base: &Vec<Vec<f64>>, k: &Vec<Vec<f64>>, f: f64| -> Vec<Vec<f64>> {
            (0..n).map(|i| (0..n).map(|j| base[i][j] + f * k[i][j]).collect()).collect()
        };
        let k2 = deriv(&add(&p, &k1, h / 2.0));
        let k3 = deriv(&add(&p, &k2, h / 2.0));
        let k4 = deriv(&add(&p, &k3, h));
        for i in 0..n { for j in 0..n { p[i][j] += h / 6.0 * (k1[i][j] + 2.0 * k2[i][j] + 2.0 * k3[i][j] + k4[i][j]); } }
        if p.iter().any(|row| row.iter().any(|v| !v.is_finite())) { return Err("Riccati integration diverged: (A,B) is probably not stabilizable for this Q,R".to_string()); }
        if step % 50 == 0 {
            let d = deriv(&p);
            let dmax = d.iter().flat_map(|row| row.iter()).fold(0.0f64, |m, v| m.max(v.abs()));
            let pmax = p.iter().flat_map(|row| row.iter()).fold(0.0f64, |m, v| m.max(v.abs()));
            if dmax < 1e-10 * (1.0 + pmax) { converged = true; break; }
        }
    }
    if !converged { return Err(format!("Riccati iteration did not converge in {} steps (slow modes, or not stabilizable/detectable)", max_steps)); }
    let k_gain = mat_mul(&mat_mul(&rinv, &bt), &p);
    let bk = mat_mul(b, &k_gain);
    let acl: Vec<Vec<f64>> = (0..n).map(|i| (0..n).map(|j| a[i][j] - bk[i][j]).collect()).collect();
    let poles = eigenvalues(&acl).ok_or_else(|| "closed-loop eigenvalues could not be computed (root finding did not converge)".to_string())?;
    if poles.iter().any(|z| z.real >= -1e-9) { return Err("closed loop is not asymptotically stable: (A,B) not stabilizable or Q not detectable".to_string()); }
    Ok((k_gain, p, poles))
}

/// First-order-plus-dead-time (K, L, T) from the unit-step response of a stable SISO
/// transfer function, by the tangent-at-steepest-slope method.
fn fopdt_from_tf(num: &[f64], den: &[f64]) -> Result<(f64, f64, f64), String> {
    let ss = tf_to_ss_siso(num, den)?;
    let poles = poly_roots(den).ok_or_else(|| "denominator roots did not converge".to_string())?;
    let slowest = poles.iter().map(|p| -p.real).fold(f64::INFINITY, f64::min);
    if poles.is_empty() || poles.iter().any(|p| p.real >= 0.0) || !slowest.is_finite() || slowest <= 0.0 {
        return Err("step-response tuning needs a strictly stable plant (all poles in the open left half-plane)".to_string());
    }
    let tmax = 10.0 / slowest;
    let dt = tmax / 5000.0;
    let sim = PipelineExecutor::simulate_lti(&ss, &SimulationInput::Step { amplitude: 1.0, step_time_sec: 0.0 }, tmax, dt, None);
    let y: Vec<f64> = sim.output_trajectory.iter().map(|v| v.first().copied().unwrap_or(0.0)).collect();
    if y.len() < 10 { return Err("step response too short".to_string()); }
    let k = *y.last().unwrap();
    if k.abs() < 1e-9 { return Err("zero DC gain: cannot fit a first-order-plus-dead-time model".to_string()); }
    let mut best_i = 1usize;
    let mut best_s = f64::NEG_INFINITY;
    for i in 1..y.len() { let sl = (y[i] - y[i - 1]) / dt * k.signum(); if sl > best_s { best_s = sl; best_i = i; } }
    if best_s <= 1e-12 { return Err("no rising slope in the step response".to_string()); }
    let t_ip = sim.time[best_i];
    let y_ip = y[best_i];
    let slope = (y[best_i] - y[best_i - 1]) / dt;
    let l = (t_ip - y_ip / slope).max(0.0);
    let t1 = t_ip + (k - y_ip) / slope;
    let t_const = t1 - l;
    if t_const <= 0.0 { return Err("could not fit a positive time constant".to_string()); }
    Ok((k, l, t_const))
}

// ─────────────────────────────────────────────────────────────────────────────
// MAIN
// ─────────────────────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--input" && i + 1 < args.len() { input_json = ozone_input_arg(&args[i + 1]); i += 2; } else { i += 1; }
    }
    if input_json.is_empty() {
        eprintln!("Usage: control_systems --input '<json>'");
        std::process::exit(1);
    }
    // CONTRACT UNIFICATION (2026-10-14): the host serializes the
    // PipelineInput envelope {"data": {...}, "context": {...}} — unwrap
    // `data` when present (the text/math contract), else parse the payload
    // as-is. Every pipeline now speaks ONE dispatch contract.
    let input_json = {
        let probe: serde_json::Value = match serde_json::from_str(&input_json) {
            Ok(v) => v,
            Err(_) => serde_json::Value::Null,
        };
        match probe.get("data") {
            Some(inner) if probe.get("context").is_some() => inner.to_string(),
            _ => input_json,
        }
    };
    // RAW-DATA DESIGN LADDER (2026-10-16): an `Analyze` call with no `data`
    // field at all is a text-only mention with nothing real to analyze (the
    // orchestrator detected this modality in prompt text, no attached data
    // file) — a deliberate honest refusal here, not an accidental
    // type-mismatch parse error. A call that DOES include `data` but with a
    // malformed/unrecognized shape still falls through to the normal parse
    // error below, since that's a real, reportable input problem, not "no
    // data provided."
    if let Ok(probe) = serde_json::from_str::<serde_json::Value>(&input_json) {
        if probe.get("type").and_then(|t| t.as_str()) == Some("Analyze") && probe.get("data").is_none() {
            println!("{}", serde_json::json!({"success":false,"error":"No raw control-systems data provided — this call appears to be a text-only mention with no attached data file; real control-systems analysis requires an actual data file"}));
            std::process::exit(1);
        }
    }
    let input: ControlModalityAction = match serde_json::from_str(&input_json) {
        Ok(v) => v,
        Err(e) => { println!("{}", serde_json::json!({"success":false,"error":format!("Parse error: {}",e)})); std::process::exit(1); }
    };
    let rt = tokio::runtime::Runtime::new().expect("Tokio runtime");
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap_or_else(|_| r#"{"success":false,"error":"serialize"}"#.into())),
        Err(e) => { println!("{}", serde_json::json!({"success":false,"error":e})); std::process::exit(1); }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TESTS for the pure-math helpers (honesty pass 2026-10-31; written, not yet run)
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod math_tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool { (a - b).abs() < tol }

    #[test]
    fn poly_roots_real_cubic() {
        let mut r: Vec<f64> = poly_roots(&[1.0, 6.0, 11.0, 6.0]).expect("converges").iter().map(|z| z.real).collect();
        r.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!(close(r[0], -3.0, 1e-6) && close(r[1], -2.0, 1e-6) && close(r[2], -1.0, 1e-6));
    }

    #[test]
    fn poly_roots_complex_pair_and_zero_root() {
        let r = poly_roots(&[1.0, 2.0, 5.0, 0.0]).expect("converges"); // s(s²+2s+5)
        assert_eq!(r.len(), 3);
        assert!(r.iter().any(|z| z.real == 0.0 && z.imag == 0.0));
        assert!(r.iter().any(|z| close(z.real, -1.0, 1e-6) && close(z.imag.abs(), 2.0, 1e-6)));
    }

    #[test]
    fn higher_order_poles_are_no_longer_empty() {
        let p = PipelineExecutor::compute_poles_from_denominator(&[1.0, 6.0, 11.0, 6.0]);
        assert_eq!(p.len(), 3);
    }

    #[test]
    fn routh_stable_and_unstable_cubics() {
        assert!(PipelineExecutor::routh_hurwitz(&[1.0, 2.0, 3.0, 4.0]).0); // 2·3 > 1·4
        assert!(!PipelineExecutor::routh_hurwitz(&[1.0, 1.0, 2.0, 8.0]).0); // 1·2 < 8
        assert!(!PipelineExecutor::routh_hurwitz(&[1.0, 0.0, 1.0]).0); // imaginary-axis roots are not asymptotically stable
        assert!(!PipelineExecutor::routh_hurwitz(&[1.0, -1.0, 1.0]).0); // sign change
    }

    #[test]
    fn faddeev_leverrier_charpoly_and_ss2tf() {
        let a = vec![vec![0.0, 1.0], vec![-2.0, -3.0]];
        let (cp, _) = faddeev_leverrier(&a).unwrap();
        assert!(close(cp[0], 1.0, 1e-12) && close(cp[1], 3.0, 1e-12) && close(cp[2], 2.0, 1e-12));
        let (num, den) = ss_siso_to_tf(&a, &vec![vec![0.0], vec![1.0]], &vec![vec![1.0, 0.0]], &vec![vec![0.0]]).unwrap();
        assert_eq!(num.len(), 1);
        assert!(close(num[0], 1.0, 1e-9));
        assert!(close(den[1], 3.0, 1e-9) && close(den[2], 2.0, 1e-9));
    }

    #[test]
    fn ss2tf_rejects_mimo_honestly() {
        let r = ss_siso_to_tf(&vec![vec![0.0]], &vec![vec![1.0, 1.0]], &vec![vec![1.0]], &vec![vec![0.0]]);
        assert!(r.is_err());
    }

    #[test]
    fn third_order_ss2tf_has_real_denominator() {
        // companion form of s³ + 6s² + 11s + 6
        let a = vec![vec![0.0, 1.0, 0.0], vec![0.0, 0.0, 1.0], vec![-6.0, -11.0, -6.0]];
        let (cp, _) = faddeev_leverrier(&a).unwrap();
        assert!(close(cp[1], 6.0, 1e-9) && close(cp[2], 11.0, 1e-9) && close(cp[3], 6.0, 1e-9));
    }

    #[test]
    fn mat_inverse_diag() {
        let inv = mat_inverse(&[vec![2.0, 0.0], vec![0.0, 4.0]]).unwrap();
        assert!(close(inv[0][0], 0.5, 1e-12) && close(inv[1][1], 0.25, 1e-12));
        assert!(mat_inverse(&[vec![1.0, 2.0], vec![2.0, 4.0]]).is_none());
    }

    #[test]
    fn lqr_scalar_matches_closed_form() {
        // A=0, B=1, Q=1, R=1 → P = 1, K = 1, closed-loop pole −1
        let (k, p, poles) = solve_care_lqr(&[vec![0.0]], &[vec![1.0]], &[vec![1.0]], &[vec![1.0]]).unwrap();
        assert!(close(p[0][0], 1.0, 1e-6) && close(k[0][0], 1.0, 1e-6));
        assert!(close(poles[0].real, -1.0, 1e-6));
    }

    #[test]
    fn lqr_unstabilizable_fails_honestly() {
        // unstable mode not reachable (B = 0): the Riccati solution does not exist
        let r = solve_care_lqr(&[vec![1.0]], &[vec![0.0]], &[vec![1.0]], &[vec![1.0]]);
        assert!(r.is_err());
    }

    #[test]
    fn first_order_step_metrics_are_computed() {
        let ss = tf_to_ss_siso(&[1.0], &[1.0, 1.0]).unwrap();
        let sim = PipelineExecutor::simulate_lti(&ss, &SimulationInput::Step { amplitude: 1.0, step_time_sec: 0.0 }, 12.0, 0.001, None);
        assert!(!sim.diverged);
        let y = sim.output_trajectory.last().unwrap()[0];
        assert!(close(y, 1.0, 1e-3));
        let ts = sim.performance.settling_time_sec.expect("settles");
        assert!(ts > 3.7 && ts < 4.1, "settling {}", ts); // ≈ 3.91 s for 1/(s+1)
        assert!(sim.performance.overshoot_percent.unwrap() < 1e-6);
        let tr = sim.performance.rise_time_sec.unwrap();
        assert!(tr > 2.0 && tr < 2.4, "rise {}", tr); // 10–90% ≈ 2.197 s
    }

    #[test]
    fn simulation_honors_initial_state_and_arbitrary_input() {
        let ss = tf_to_ss_siso(&[1.0], &[1.0, 1.0]).unwrap();
        let sim = PipelineExecutor::simulate_lti(&ss, &SimulationInput::Arbitrary { time: vec![0.0, 1.0], values: vec![0.0, 0.0] }, 1.0, 0.01, Some(&[1.0]));
        let y_end = sim.output_trajectory.last().unwrap()[0];
        assert!(close(y_end, (-1.0f64).exp(), 5e-3)); // free decay of x0 = 1
    }

    #[test]
    fn white_noise_is_reproducible_for_a_seed() {
        let ss = tf_to_ss_siso(&[1.0], &[1.0, 1.0]).unwrap();
        let a = PipelineExecutor::simulate_lti(&ss, &SimulationInput::White_Noise { variance: 1.0, seed: 7 }, 1.0, 0.01, None);
        let b = PipelineExecutor::simulate_lti(&ss, &SimulationInput::White_Noise { variance: 1.0, seed: 7 }, 1.0, 0.01, None);
        assert_eq!(a.output_trajectory, b.output_trajectory);
    }

    #[test]
    fn fopdt_fit_on_third_order_lag() {
        let (k, l, t) = fopdt_from_tf(&[1.0], &[1.0, 3.0, 3.0, 1.0]).unwrap(); // 1/(s+1)³
        assert!(close(k, 1.0, 1e-3));
        assert!(l > 0.3 && l < 1.2, "L {}", l);
        assert!(t > 1.0 && t < 4.0, "T {}", t);
    }
}

/// `--input @<path>`: the host stages oversize inputs (> 100 KB) in a file because a
/// single argv string is capped at 128 KiB (E2BIG). Plain values pass through unchanged.
#[allow(dead_code)]
fn ozone_input_arg(raw: &str) -> String {
    match raw.strip_prefix('@') {
        Some(path) if std::path::Path::new(path).is_file() => std::fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("Failed to read staged input {}: {}", path, e);
            std::process::exit(1);
        }),
        _ => raw.to_string(),
    }
}
