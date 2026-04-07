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

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

// ─────────────────────────────────────────────────────────────────────────────
// INPUT TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ControlModalityAction {
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
    pub residuals_whiteness_p_value: f64,
    pub cross_validation_fit_percent: Option<f64>,
}

// ─────────────────────────────────────────────────────────────────────────────
// SIMULATION
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SimulationInput {
    #[default] Step { amplitude: f64, step_time_sec: f64 },
    Ramp { slope: f64 },
    Sinusoidal { amplitude: f64, frequency_hz: f64 },
    ImpulseAt { time_sec: f64, amplitude: f64 },
    Arbitrary { time: Vec<f64>, values: Vec<f64> },
    White_Noise { variance: f64, seed: u64 },
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
        let input = serde_json::json!({"action":"Prompt","prompt":prompt,"max_tokens":max_tokens,"temperature":0.05,"system_context":"Control systems analysis. Return only valid JSON."});
        let out = std::process::Command::new(&self.prompt_pipeline_path).arg("--input").arg(input.to_string()).output().map_err(|e| e.to_string())?;
        let r: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
        Ok(r["response"].as_str().unwrap_or("{}").to_string())
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

Available types: Controls, Stabilizes, FeedbackFrom, MeasuredBy, ActuatedBy,
DisturbedBy, ClosedLoopWith, Affects, CausedBy, Enables, DerivedFrom,
FunctionalRole, TemporalPrecedes, SimilarTo

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
                        let etype = map_ctrl_edge_str(v["edge_type"].as_str().unwrap_or("Affects"));
                        let reason = v["reason"].as_str().unwrap_or("").to_string();
                        Some((from, to, etype, reason))
                    }).collect()
            }
            Err(_) => vec![],
        }
    }

    /// Routh-Hurwitz stability: returns true if all signs in first column are positive
    fn routh_hurwitz(coefficients: &[f64]) -> (bool, Vec<Vec<f64>>) {
        let n = coefficients.len();
        if n < 2 { return (false, vec![]); }
        let rows = n;
        let cols = (n + 1) / 2;
        let mut routh: Vec<Vec<f64>> = vec![vec![0.0; cols]; rows];

        // First two rows from coefficients
        for (i, &c) in coefficients.iter().enumerate() {
            if i % 2 == 0 { routh[0][i / 2] = c; }
            else           { routh[1][i / 2] = c; }
        }

        for row in 2..rows {
            let r1 = row - 2;
            let r2 = row - 1;
            let pivot = routh[r2][0];
            if pivot.abs() < 1e-12 {
                // Special case: replace with small epsilon
                routh[r2][0] = 1e-12;
            }
            for col in 0..cols - 1 {
                routh[row][col] = (routh[r2][0] * routh[r1][col + 1] - routh[r1][0] * routh[r2][col + 1])
                    / routh[r2][0];
            }
        }

        let stable = routh.iter().all(|row| row[0] >= 0.0);
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
                // Higher order: return estimate based on Routh array
                // In production: use eigenvalue solver (nalgebra / lapack)
                vec![]
            }
        }
    }

    /// Simulate a first/second-order LTI system using Euler integration
    fn simulate_lti(
        ss: &StateSpaceModel,
        input: &SimulationInput,
        duration_sec: f64,
        dt: f64,
    ) -> SimulationResult {
        let n_steps = (duration_sec / dt).ceil() as usize;
        let n_states = ss.a_matrix.len();
        let n_inputs = ss.b_matrix.first().map(|r| r.len()).unwrap_or(1);
        let n_outputs = ss.c_matrix.len();

        let mut x = vec![0.0f64; n_states];
        let mut time = Vec::with_capacity(n_steps);
        let mut x_traj: Vec<Vec<f64>> = Vec::with_capacity(n_steps);
        let mut y_traj: Vec<Vec<f64>> = Vec::with_capacity(n_steps);
        let mut u_traj: Vec<Vec<f64>> = Vec::with_capacity(n_steps);

        let mut diverged = false;
        let mut diverge_time = None;

        for step in 0..n_steps {
            let t = step as f64 * dt;
            time.push(t);

            // Input value at time t
            let u_val = match input {
                SimulationInput::Step { amplitude, step_time_sec } =>
                    if t >= *step_time_sec { *amplitude } else { 0.0 },
                SimulationInput::Ramp { slope } => slope * t,
                SimulationInput::Sinusoidal { amplitude, frequency_hz } =>
                    amplitude * (2.0 * std::f64::consts::PI * frequency_hz * t).sin(),
                SimulationInput::ImpulseAt { time_sec, amplitude } =>
                    if (t - time_sec).abs() < dt { amplitude / dt } else { 0.0 },
                _ => 0.0,
            };

            let u = vec![u_val; n_inputs];

            // Compute output y = C*x + D*u
            let mut y = vec![0.0f64; n_outputs];
            for i in 0..n_outputs {
                for j in 0..n_states.min(ss.c_matrix[i].len()) {
                    y[i] += ss.c_matrix[i][j] * x[j];
                }
                if i < ss.d_matrix.len() {
                    for j in 0..n_inputs.min(ss.d_matrix[i].len()) {
                        y[i] += ss.d_matrix[i][j] * u[j];
                    }
                }
            }

            x_traj.push(x.clone());
            y_traj.push(y);
            u_traj.push(u.clone());

            // Check divergence
            if x.iter().any(|&v| v.abs() > 1e6 || v.is_nan() || v.is_infinite()) {
                diverged = true;
                diverge_time = Some(t);
                break;
            }

            // Euler integration: x_dot = A*x + B*u
            let mut x_dot = vec![0.0f64; n_states];
            for i in 0..n_states {
                for j in 0..n_states.min(ss.a_matrix[i].len()) {
                    x_dot[i] += ss.a_matrix[i][j] * x[j];
                }
                if i < ss.b_matrix.len() {
                    for j in 0..n_inputs.min(ss.b_matrix[i].len()) {
                        x_dot[i] += ss.b_matrix[i][j] * u[j];
                    }
                }
                x[i] += x_dot[i] * dt;
            }
        }

        // Compute performance metrics from output trajectory
        let final_value = y_traj.last().and_then(|y| y.first()).copied().unwrap_or(0.0);
        let target = match input { SimulationInput::Step { amplitude, .. } => *amplitude, _ => final_value };
        let tolerance_band = target.abs() * 0.02; // 2%

        let rise_time = if target.abs() > 1e-12 {
            time.iter().zip(y_traj.iter()).position(|(_, y)| {
                y.first().copied().unwrap_or(0.0).abs() >= 0.9 * target.abs()
            }).map(|i| time[i])
        } else { None };

        let settling_time = if target.abs() > 1e-12 {
            time.iter().enumerate().rev().find(|(_, &t)| {
                // Find last time outside tolerance
                false
            }).map(|(_, &t)| t)
        } else { None };

        let overshoot = if target.abs() > 1e-12 {
            let peak = y_traj.iter().filter_map(|y| y.first().copied()).fold(f64::NEG_INFINITY, f64::max);
            if peak > target { Some(100.0 * (peak - target) / target.abs()) } else { Some(0.0) }
        } else { None };

        let steady_state_error = Some((final_value - target).abs());

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
                steady_state_error,
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
        hotness_score: 1.0, ..Default::default()
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
            hotness_score: 0.9,
            embedding_hint: Some(format!("plant {} domain:{:?} states:{}", plant.name, plant.physical_domain, plant.n_states)),
            ..Default::default()
        });
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
                hotness_score: 0.75, ..Default::default()
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
                edge_type: ControlEdgeType::PlantIsKinematicChain, weight: 0.85,
                provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("kinematics")); p },
                ..Default::default()
            });
            edge_id += 1;
            // Cross-modal: commands robot 3D (109)
            edges.push(ControlGraphEdge {
                edge_id, from_node: pid, to_node: pid,
                edge_type: ControlEdgeType::CommandsRobot3D, weight: 0.8,
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
            hotness_score: 0.85, ..Default::default()
        });
        ctrl_nid.insert(ctrl.controller_id, cid);
        edges.push(ControlGraphEdge { edge_id, from_node: root_id, to_node: cid, edge_type: ControlEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Cross-modal: controller code (101)
        edges.push(ControlGraphEdge {
            edge_id, from_node: cid, to_node: cid,
            edge_type: ControlEdgeType::ImplementedInCode, weight: 0.8,
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
            hotness_score: 0.7, ..Default::default()
        });
        sensor_nid.insert(sensor.sensor_id, sid);
        edges.push(ControlGraphEdge { edge_id, from_node: root_id, to_node: sid, edge_type: ControlEdgeType::Contains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Cross-modal: IMU sensor (116)
        if matches!(sensor.sensor_type, SensorType::IMU | SensorType::Accelerometer | SensorType::Gyroscope) {
            edges.push(ControlGraphEdge {
                edge_id, from_node: sid, to_node: sid,
                edge_type: ControlEdgeType::FedByIMUSensor, weight: 0.9,
                provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("imu")); p },
                ..Default::default()
            });
            edge_id += 1;
        }
        if matches!(sensor.sensor_type, SensorType::Radar) {
            edges.push(ControlGraphEdge {
                edge_id, from_node: sid, to_node: sid,
                edge_type: ControlEdgeType::TrackFromRadar, weight: 0.85,
                provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("radar")); p },
                ..Default::default()
            });
            edge_id += 1;
        }
        if matches!(sensor.sensor_type, SensorType::Sonar) {
            edges.push(ControlGraphEdge {
                edge_id, from_node: sid, to_node: sid,
                edge_type: ControlEdgeType::NavigationFromSonar, weight: 0.85,
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
            hotness_score: 0.7, ..Default::default()
        });
        actuator_nid.insert(actuator.actuator_id, aid);
        edges.push(ControlGraphEdge { edge_id, from_node: root_id, to_node: aid, edge_type: ControlEdgeType::Contains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
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
            hotness_score: 0.9, ..Default::default()
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
            edge_type: ControlEdgeType::ProvenInMath, weight: 0.75,
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
            content: format!("FreqResp [{:?}]: bw={:?}rad
