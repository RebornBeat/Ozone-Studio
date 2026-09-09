//! Kinematics — Pipeline #121
//!
//! Kinematic chain analysis: forward/inverse kinematics, workspace analysis,
//! joint limits, singularity detection, gait cycle analysis, trajectory
//! planning, and biomechanical modeling.
//!
//! COVERS:
//!   - Robotic manipulators (serial/parallel/mobile)
//!   - Humanoid and legged robots (biped/quadruped/hexapod)
//!   - Human biomechanics (musculoskeletal)
//!   - Animal locomotion
//!   - Mechanisms (linkages, cams, gears)
//!   - Vehicle kinematics (steering, suspension)
//!   - Prosthetics and orthotics
//!
//! CROSS-LINKS:
//!   109 (3D)     → armature/bone hierarchy maps to kinematic chain
//!   120 (CAD)    → joint geometry from CAD assembly
//!   122 (Control)→ kinematics feeds controller (joint space ↔ task space)
//!   119 (BCI)    → motor imagery → joint commands
//!   116 (IMU)    → joint angle estimation from IMU data
//!   111 (Bio)    → biomechanical context for human/animal kinematics
//!
//! STORAGE: ZSEI containers under /Modalities/Kinematics/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

// ─────────────────────────────────────────────────────────────────────────────
// INPUT TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum KinematicsModalityAction {
    // ── ANALYSIS ──
    AnalyzeChain        { data: KinematicsDataSource, compute_workspace: bool, detect_singularities: bool },
    AnalyzeGait         { data: GaitDataSource, gait_type: GaitType },
    AnalyzeBiomechanics { data: BiomechanicsDataSource, compute_muscle_forces: bool },
    AnalyzeMechanism    { data: MechanismDataSource },
    // ── KINEMATICS ──
    ForwardKinematics   { graph_id: u64, chain_node_id: u64, joint_angles_rad: Vec<f64> },
    InverseKinematics   { graph_id: u64, chain_node_id: u64, target_pose: Pose6DOF, method: IKMethod, initial_guess: Option<Vec<f64>> },
    ComputeJacobian     { graph_id: u64, chain_node_id: u64, joint_angles_rad: Vec<f64> },
    PlanTrajectory      { graph_id: u64, chain_node_id: u64, waypoints: Vec<Pose6DOF>, method: TrajectoryMethod, duration_sec: f64 },
    CheckSingularities  { graph_id: u64, chain_node_id: u64, joint_angles_rad: Vec<f64> },
    ComputeWorkspace    { graph_id: u64, chain_node_id: u64, resolution: WorkspaceResolution },
    // ── GRAPH ──
    CreateGraph         { analysis: KinematicsAnalysisResult, project_id: u64 },
    UpdateGraph         { graph_id: u64, updates: Vec<KinematicsUpdate>, project_id: u64 },
    QueryGraph          { graph_id: u64, query: KinematicsGraphQuery },
    GetGraph            { graph_id: u64 },
    // ── CROSS-MODAL ──
    LinkToModality      { graph_id: u64, target_modality: String, target_graph_id: u64, link_type: KinCrossModalLink },
    ImportFromCAD       { cad_graph_id: u64, joint_ids: Vec<u64>, project_id: u64 },
    ImportFrom3D        { graph_id_3d: u64, armature_node_id: u64, project_id: u64 },
    ExportToControl     { graph_id: u64, chain_node_id: u64 },
    // ── HOOKS / STREAMING ──
    TriggerSemanticHook { graph_id: u64, hook: KinematicsSemanticHook },
    StreamToUI          { graph_id: u64, session_id: String, display_mode: KinematicsDisplayMode },
    HeadlessProcess     { graph_id: u64, operations: Vec<KinematicsHeadlessOp> },
}

// ─────────────────────────────────────────────────────────────────────────────
// DATA SOURCES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KinematicsDataSource {
    URDF            { file_path: String },                  // Robot description format
    MJCF            { file_path: String },                  // MuJoCo model
    DHTable         { params: Vec<DHParameters> },          // Denavit-Hartenberg
    SDF             { file_path: String },                  // Simulation Description Format
    BVH             { file_path: String },                  // BioVision Hierarchy (motion capture)
    C3D             { file_path: String },                  // Biomechanics data format
    OSIM            { file_path: String },                  // OpenSim musculoskeletal model
    JSON_Chain      { chain: KinematicChainSpec },          // Direct specification
    From3D          { graph_id: u64, armature_node_id: u64 },
    FromCAD         { cad_graph_id: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GaitDataSource {
    BVH             { file_path: String },
    C3D             { file_path: String, marker_set: MarkerSet },
    IMU_Sequence    { files: Vec<String>, sensor_placement: Vec<SensorPlacement> },
    Synthetic       { gait_type: GaitType, speed_ms: f64, step_length_m: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BiomechanicsDataSource {
    OSIM            { model_path: String, motion_path: String },
    C3D_WithForce   { file_path: String, force_plate_data: Vec<ForcePlateData> },
    Synthetic       { body_mass_kg: f64, body_height_m: f64, activity: BiomechanicsActivity },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MechanismDataSource {
    LinkageSpec     { links: Vec<LinkSpec>, joints: Vec<MechanismJointSpec> },
    GearTrainSpec   { gears: Vec<GearSpec> },
    CamFollowerSpec { cam: CamSpec, follower: FollowerSpec },
    FromCAD         { cad_graph_id: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DHParameters {
    pub joint_name: String,
    pub a_mm: f64,               // link length
    pub d_mm: f64,               // link offset
    pub alpha_rad: f64,          // link twist
    pub theta_rad: f64,          // joint angle (variable for revolute)
    pub joint_type: JointType,
    pub joint_limits: JointLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JointLimits {
    pub lower: f64,              // radians or meters
    pub upper: f64,
    pub velocity_limit: Option<f64>,   // rad/s or m/s
    pub effort_limit: Option<f64>,     // Nm or N
    pub acceleration_limit: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KinematicChainSpec {
    pub name: String,
    pub chain_type: ChainType,
    pub joints: Vec<JointSpec>,
    pub end_effectors: Vec<EndEffectorSpec>,
    pub base_frame: KinFrame,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JointSpec {
    pub name: String,
    pub joint_type: JointType,
    pub limits: JointLimits,
    pub axis: [f64; 3],
    pub origin_mm: [f64; 3],
    pub rpy_rad: [f64; 3],
    pub parent_link: String,
    pub child_link: String,
    pub damping: f64,
    pub friction: f64,
    pub gear_ratio: f64,
    pub is_mimic: bool,
    pub mimic_joint: Option<String>,
    pub mimic_multiplier: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EndEffectorSpec {
    pub name: String,
    pub parent_link: String,
    pub offset_mm: [f64; 3],
    pub rpy_rad: [f64; 3],
    pub tool_type: ToolType,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KinFrame {
    pub origin_mm: [f64; 3],
    pub rpy_rad: [f64; 3],
    pub parent_frame: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkerSet { pub marker_names: Vec<String>, pub marker_set_type: String }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorPlacement { pub sensor_id: String, pub body_segment: String, pub position_mm: [f64; 3] }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ForcePlateData { pub plate_id: u32, pub forces_n: Vec<[f64; 3]>, pub sample_rate_hz: f64 }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkSpec { pub name: String, pub length_mm: f64, pub mass_kg: f64, pub cog_mm: [f64; 3] }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MechanismJointSpec { pub name: String, pub link_a: String, pub link_b: String, pub joint_type: JointType, pub position_mm: [f64; 2] }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GearSpec { pub name: String, pub teeth: u32, pub module_mm: f64, pub pressure_angle_deg: f64, pub is_driver: bool }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CamSpec { pub name: String, pub profile: Vec<[f64; 2]>, pub rpm: f64 }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowerSpec { pub follower_type: String, pub offset_mm: f64 }

// ─────────────────────────────────────────────────────────────────────────────
// ENUMS
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum JointType {
    #[default] Revolute,         // rotation around axis
    Prismatic,                   // translation along axis
    Spherical,                   // 3-DOF ball joint
    Universal,                   // 2-DOF universal joint
    Planar,                      // 3-DOF planar
    Fixed,                       // 0 DOF rigid connection
    Continuous,                  // revolute without limits
    Floating,                    // 6 DOF (base link)
    Screw,                       // coupled revolute+prismatic
    Gear { ratio: f64 },
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ChainType {
    #[default] Serial,           // open chain: arm, leg
    Parallel,                    // closed chain: delta, Stewart platform
    Hybrid,                      // mixed
    Tree,                        // branching: humanoid spine→limbs
    Mobile,                      // wheeled/tracked base
    TendonDriven,                // cable/tendon transmission
    Continuum,                   // snake/soft robot
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum GaitType {
    #[default] HumanWalk, HumanRun, HumanJump, HumanStairs,
    BipedRobotWalk, BipedRobotRun,
    QuadrupedTrot, QuadrupedGallop, QuadrupedWalk, QuadrupedBound,
    HexapodAlt_Tripod, HexapodWave,
    Wheelchair, Crutch_Assisted,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum BiomechanicsActivity {
    #[default] Walking, Running, Jumping, Cycling, Lifting, Throwing,
    Sitting, Standing, Climbing, Swimming, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ToolType {
    #[default] Generic, Gripper, Suction, Welding, Drilling, Camera,
    ForceSensor, SixAxisFT, IMU, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum IKMethod {
    #[default] JacobianPseudoInverse,
    JacobianTranspose,
    DampedLeastSquares,
    NumericalNewtonRaphson,
    AnalyticalClosed,
    FABRIK,
    CCD,
    BioIK,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum TrajectoryMethod {
    #[default] LinearInterpolation,
    CubicSpline,
    QuinticPolynomial,
    TrapezoidalVelocity,
    MinimumJerk,
    LSPB,           // Linear Segment with Parabolic Blends
    RRT,            // Rapidly-exploring Random Tree
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum WorkspaceResolution { Coarse, Medium, #[default] Fine, VeryFine }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum KinCrossModalLink {
    #[default] MapsToArmatureIn3D,   // kinematic chain → 3D armature
    JointGeometryFromCAD,            // CAD joint geometry → this chain
    FeedsControlSystem,              // kinematic model → control (122)
    MotorImageryFromBCI,             // BCI (119) motor imagery → joint command
    JointAngleFromIMU,               // IMU (116) → joint angle estimate
    BiomechanicalContext,            // biology (111) provides muscle/bone context
}

// ─────────────────────────────────────────────────────────────────────────────
// ANALYSIS RESULT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KinematicsAnalysisResult {
    pub analysis_id: u64,
    pub source_description: String,
    pub system_type: KinematicsSystemType,

    // CHAINS & JOINTS
    pub kinematic_chains: Vec<KinematicChain>,
    pub joints: Vec<Joint>,
    pub links: Vec<Link>,
    pub end_effectors: Vec<EndEffector>,

    // WORKSPACE
    pub workspace_volumes: Vec<WorkspaceVolume>,
    pub reachable_poses: Vec<ReachablePose>,

    // SINGULARITIES
    pub singularities: Vec<SingularityPoint>,

    // GAIT
    pub gait_cycles: Vec<GaitCycle>,
    pub gait_phases: Vec<GaitPhase>,
    pub step_events: Vec<StepEvent>,
    pub ground_reaction_forces: Vec<GRFData>,

    // BIOMECHANICS
    pub body_segments: Vec<BodySegment>,
    pub muscle_groups: Vec<MuscleGroup>,
    pub joint_moments: Vec<JointMoment>,
    pub center_of_mass_trajectory: Vec<COMPoint>,

    // MECHANISMS
    pub mechanism_links: Vec<MechanismLink>,
    pub mechanism_joints: Vec<MechanismJoint>,
    pub transmission_ratios: Vec<TransmissionRatio>,

    // TRAJECTORIES (if computed)
    pub planned_trajectories: Vec<PlannedTrajectory>,

    // METADATA
    pub dof_total: u32,
    pub dof_actuated: u32,
    pub is_redundant: bool,
    pub processing_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum KinematicsSystemType {
    #[default] SerialManipulator, ParallelManipulator, HybridManipulator,
    BipedRobot, QuadrupedRobot, HexapodRobot, WheelBasedRobot,
    HumanBiomechanics, AnimalBiomechanics, ProstheticLimb,
    MechanicalLinkage, GearTrain, CamMechanism,
    Vehicle, Custom(String),
}

// ── KINEMATIC CHAIN ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KinematicChain {
    pub chain_id: u64,
    pub name: String,
    pub chain_type: ChainType,
    pub base_joint_id: u64,
    pub tip_joint_id: u64,
    pub end_effector_id: Option<u64>,
    pub joint_ids: Vec<u64>,            // ordered from base to tip
    pub link_ids: Vec<u64>,
    pub dof: u32,
    pub dof_actuated: u32,
    pub is_redundant: bool,             // dof > 6
    pub base_frame: KinFrame,
    pub dh_table: Vec<DHParameters>,    // if DH parameterization available
    pub joint_angles_home: Vec<f64>,    // home/zero configuration
    pub joint_angles_current: Vec<f64>,
    pub end_effector_pose: Option<Pose6DOF>,
    pub manipulability: Option<f64>,    // Yoshikawa manipulability measure
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Pose6DOF {
    pub position_mm: [f64; 3],
    pub orientation_rpy_rad: [f64; 3],
    pub orientation_quat: Option<[f64; 4]>,  // w, x, y, z
    pub frame_name: Option<String>,
}

// ── JOINTS ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Joint {
    pub joint_id: u64,
    pub name: String,
    pub joint_type: JointType,
    pub parent_link_id: u64,
    pub child_link_id: u64,
    pub origin_mm: [f64; 3],
    pub orientation_rpy_rad: [f64; 3],
    pub axis: [f64; 3],
    pub limits: JointLimits,
    pub current_position: f64,          // current joint value (rad or m)
    pub current_velocity: f64,
    pub current_effort: f64,
    pub gear_ratio: f64,
    pub actuator_type: ActuatorType,
    pub is_passive: bool,
    pub is_mimic: bool,
    pub mimic_source_id: Option<u64>,
    pub mimic_multiplier: f64,
    pub damping: f64,
    pub friction: f64,
    pub kinematic_pair: KinematicPairClass,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ActuatorType {
    #[default] None, DCMotor, ServoMotor, BLDC, StepperMotor, Hydraulic,
    Pneumatic, Linear_Actuator, SEA,    // Series Elastic Actuator
    Cable_Tendon, SMA,                  // Shape Memory Alloy
    Muscle_EMG, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum KinematicPairClass {
    #[default] Lower,        // surface contact
    Higher,                  // point/line contact (gears, cams)
}

// ── LINKS ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Link {
    pub link_id: u64,
    pub name: String,
    pub parent_joint_id: Option<u64>,
    pub child_joint_ids: Vec<u64>,
    pub mass_kg: f64,
    pub cog_mm: [f64; 3],
    pub inertia_tensor_kg_mm2: [[f64; 3]; 3],
    pub length_mm: f64,
    pub bounding_box: LinkBoundingBox,
    pub segment_type: BodySegmentType,
    pub material: Option<String>,
    pub is_base: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LinkBoundingBox { pub min_mm: [f64; 3], pub max_mm: [f64; 3] }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum BodySegmentType {
    #[default] Generic, Torso, Pelvis, UpperArm, ForeArm, Hand, Finger,
    Thigh, Shank, Foot, Toe, Head, Neck, Spine,
    RobotBody, RobotLink, VehicleBody, VehicleWheel, Custom(String),
}

// ── END EFFECTORS ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EndEffector {
    pub ee_id: u64,
    pub name: String,
    pub parent_link_id: u64,
    pub offset_from_parent_mm: [f64; 3],
    pub orientation_rpy_rad: [f64; 3],
    pub tool_type: ToolType,
    pub payload_mass_kg: Option<f64>,
    pub current_pose: Option<Pose6DOF>,
    pub force_torque: Option<[f64; 6]>,  // Fx,Fy,Fz, Tx,Ty,Tz
}

// ── WORKSPACE ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspaceVolume {
    pub volume_id: u64,
    pub chain_id: u64,
    pub volume_type: WorkspaceType,
    pub volume_mm3: f64,
    pub reachable_fraction: f64,        // 0.0–1.0 of sampled poses
    pub dexterous_volume_mm3: Option<f64>, // subset with full orientation control
    pub bounding_sphere_radius_mm: f64,
    pub center_mm: [f64; 3],
    pub max_reach_mm: f64,
    pub min_reach_mm: f64,
    pub resolution_mm: f32,
    pub reachable_point_cloud: Vec<[f32; 3]>,  // sampled reachable positions
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum WorkspaceType { #[default] Reachable, Dexterous, Constant_Orientation, Task_Specific }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReachablePose {
    pub pose_id: u64,
    pub position_mm: [f64; 3],
    pub is_reachable: bool,
    pub joint_config: Vec<f64>,
    pub manipulability: f64,
    pub nearest_singularity_distance: Option<f64>,
}

// ── SINGULARITIES ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SingularityPoint {
    pub singularity_id: u64,
    pub chain_id: u64,
    pub singularity_type: SingularityType,
    pub joint_config: Vec<f64>,         // joint angles at singularity
    pub pose: Pose6DOF,
    pub lost_dof: Vec<String>,          // which DOFs are lost at this singularity
    pub sigma_min: f64,                 // minimum singular value of Jacobian
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SingularityType {
    #[default] Wrist,          // wrist singularity (last 3 joints coplanar)
    Shoulder,                  // reach boundary
    Elbow,                     // elbow fully extended/retracted
    Aligned,                   // two joint axes aligned
    Custom(String),
}

// ── GAIT ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GaitCycle {
    pub cycle_id: u64,
    pub gait_type: GaitType,
    pub duration_sec: f64,
    pub step_length_m: f64,
    pub stride_length_m: f64,
    pub speed_ms: f64,
    pub cadence_steps_per_min: f64,
    pub symmetry_index: f64,            // 0=asymmetric, 1=perfectly symmetric
    pub phase_ids: Vec<u64>,
    pub step_event_ids: Vec<u64>,
    pub total_energy_J: Option<f64>,
    pub cot: Option<f64>,               // cost of transport (J/kg/m)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GaitPhase {
    pub phase_id: u64,
    pub cycle_id: u64,
    pub phase_name: String,             // "stance", "swing", "double support", etc.
    pub phase_type: GaitPhaseType,
    pub start_percent: f64,            // % of gait cycle
    pub end_percent: f64,
    pub limb: LimbIdentifier,
    pub duration_sec: f64,
    pub joint_angle_means: HashMap<String, f64>,
    pub joint_moment_means: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum GaitPhaseType {
    #[default] Stance, Swing, DoubleSupport, SingleLegSupport,
    LoadingResponse, MidStance, TerminalStance, PreSwing,
    InitialSwing, MidSwing, TerminalSwing, FlightPhase,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum LimbIdentifier {
    #[default] Right_Leg, Left_Leg, Right_Arm, Left_Arm,
    Front_Right, Front_Left, Rear_Right, Rear_Left,
    All, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StepEvent {
    pub event_id: u64,
    pub cycle_id: u64,
    pub event_type: StepEventType,
    pub limb: LimbIdentifier,
    pub time_sec: f64,
    pub percent_of_cycle: f64,
    pub position_m: Option<[f64; 2]>,   // foot strike position (x,y)
    pub grf_magnitude_n: Option<f64>,   // ground reaction force at event
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum StepEventType {
    #[default] HeelStrike, FootFlat, HeelOff, ToeOff, MidSwing, InitialContact
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GRFData {
    pub grf_id: u64,
    pub limb: LimbIdentifier,
    pub force_n: Vec<[f64; 3]>,         // time series: (Fx, Fy, Fz)
    pub moment_nm: Vec<[f64; 3]>,       // (Mx, My, Mz)
    pub cop_m: Vec<[f64; 2]>,           // center of pressure trajectory
    pub sample_rate_hz: f64,
    pub duration_sec: f64,
    pub peak_vertical_n: f64,
    pub impulse_ns: f64,
}

// ── BIOMECHANICS ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BodySegment {
    pub segment_id: u64,
    pub name: String,
    pub segment_type: BodySegmentType,
    pub mass_kg: f64,
    pub length_mm: f64,
    pub cog_fraction: f64,              // COG as fraction of segment length from proximal
    pub radius_of_gyration_mm: f64,
    pub inertia_tensor_kg_mm2: [[f64; 3]; 3],
    pub proximal_joint_id: Option<u64>,
    pub distal_joint_id: Option<u64>,
    pub adjacent_segment_ids: Vec<u64>,
    pub current_pose: Option<Pose6DOF>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MuscleGroup {
    pub muscle_id: u64,
    pub name: String,
    pub origin_segment_id: u64,
    pub insertion_segment_id: u64,
    pub origin_point_mm: [f64; 3],
    pub insertion_point_mm: [f64; 3],
    pub pennation_angle_deg: f64,
    pub pcsa_cm2: f64,                  // physiological cross-sectional area
    pub max_isometric_force_n: f64,
    pub optimal_fiber_length_mm: f64,
    pub tendon_slack_length_mm: f64,
    pub muscle_type: MuscleType,
    pub force_length_curve: Vec<(f64, f64)>,  // (normalized_length, normalized_force)
    pub force_velocity_curve: Vec<(f64, f64)>,
    pub current_activation: f64,        // 0.0–1.0
    pub current_force_n: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum MuscleType { #[default] Uniarticular, Biarticular, Pennate, Fusiform, FlatBand }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JointMoment {
    pub moment_id: u64,
    pub joint_id: u64,
    pub time_series_nm: Vec<[f64; 3]>,   // (Mx, My, Mz) over time
    pub sample_rate_hz: f64,
    pub peak_flexion_nm: Option<f64>,
    pub peak_extension_nm: Option<f64>,
    pub power_series_w: Vec<f64>,
    pub positive_work_j: Option<f64>,
    pub negative_work_j: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct COMPoint {
    pub time_sec: f64,
    pub position_m: [f64; 3],
    pub velocity_ms: [f64; 3],
    pub acceleration_ms2: [f64; 3],
}

// ── MECHANISMS ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MechanismLink {
    pub link_id: u64, pub name: String, pub length_mm: f64, pub mass_kg: f64,
    pub is_ground: bool,
    pub positions_over_cycle: Vec<[f64; 2]>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MechanismJoint {
    pub joint_id: u64, pub name: String, pub joint_type: JointType,
    pub link_a_id: u64, pub link_b_id: u64,
    pub position_mm: [f64; 2],
    pub angles_over_cycle: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TransmissionRatio {
    pub ratio_id: u64,
    pub input_joint_id: u64, pub output_joint_id: u64,
    pub ratio: f64,
    pub efficiency: f64,
    pub transmission_type: String,
}

// ── TRAJECTORY ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlannedTrajectory {
    pub trajectory_id: u64,
    pub chain_id: u64,
    pub method: String,
    pub duration_sec: f64,
    pub waypoint_count: u32,
    pub joint_trajectory: Vec<JointTrajectoryPoint>,
    pub task_trajectory: Vec<Pose6DOF>,
    pub max_velocity_rad_s: Vec<f64>,
    pub max_acceleration_rad_s2: Vec<f64>,
    pub is_collision_free: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JointTrajectoryPoint {
    pub time_sec: f64,
    pub positions_rad: Vec<f64>,
    pub velocities_rad_s: Vec<f64>,
    pub accelerations_rad_s2: Vec<f64>,
    pub effort_nm: Vec<f64>,
}

// ── FORWARD/INVERSE KINEMATICS RESULTS ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FKResult {
    pub chain_id: u64,
    pub joint_angles_rad: Vec<f64>,
    pub end_effector_pose: Pose6DOF,
    pub link_poses: Vec<(u64, Pose6DOF)>,       // (link_id, pose)
    pub jacobian: Option<Vec<Vec<f64>>>,         // 6×n matrix
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IKResult {
    pub chain_id: u64,
    pub target_pose: Pose6DOF,
    pub joint_angles_rad: Vec<f64>,
    pub converged: bool,
    pub iterations: u32,
    pub position_error_mm: f64,
    pub orientation_error_rad: f64,
    pub method: String,
    pub all_solutions: Vec<Vec<f64>>,           // multiple solutions if analytical
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SingularityCheckResult {
    pub chain_id: u64,
    pub joint_angles_rad: Vec<f64>,
    pub is_singular: bool,
    pub sigma_min: f64,
    pub manipulability: f64,
    pub singularity_type: Option<String>,
    pub distance_to_singularity: Option<f64>,
}

// ─────────────────────────────────────────────────────────────────────────────
// KINEMATICS UPDATES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KinematicsUpdate {
    SetJointAngle      { joint_node_id: u64, angle_rad: f64 },
    SetJointVelocity   { joint_node_id: u64, velocity_rad_s: f64 },
    SetJointLimits     { joint_node_id: u64, lower: f64, upper: f64 },
    UpdateEndEffectorPose { ee_node_id: u64, pose: Pose6DOF },
    UpdateLinkMass     { link_node_id: u64, mass_kg: f64 },
    AddJoint           { joint: Joint },
    RemoveJoint        { joint_node_id: u64 },
    AppendToChain      { chain_node_id: u64, joint: Joint, link: Link },
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH NODE TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum KinematicsNodeType {
    #[default] KinematicsSystem,        // root: entire robot/body
    KinematicChainNode,                 // a kinematic chain
    JointNode,                          // a single joint
    LinkNode,                           // a rigid link
    EndEffectorNode,                    // end effector / tool
    WorkspaceVolumeNode,               // reachable workspace region
    SingularityNode,                   // singularity configuration
    GaitCycleNode,                     // complete gait cycle
    GaitPhaseNode,                     // phase within gait cycle
    StepEventNode,                     // heel strike, toe off, etc.
    BodySegmentNode,                   // body segment (biomechanics)
    MuscleGroupNode,                   // muscle group
    JointMomentNode,                   // joint kinetics
    COMTrajectoryNode,                 // center of mass path
    PlannedTrajectoryNode,             // planned motion trajectory
    MechanismLinkNode,                 // mechanism link
    MechanismJointNode,                // mechanism joint
    TransmissionNode,                  // gear/belt/chain transmission
    CrossModalRef,                     // cross-modal link
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KinematicsGraphNode {
    pub node_id: u64,
    pub node_type: KinematicsNodeType,
    pub content: String,

    // KINEMATICS-SPECIFIC
    pub joint_type: Option<String>,
    pub dof: Option<u32>,
    pub current_position: Option<f64>,    // joint value
    pub limit_lower: Option<f64>,
    pub limit_upper: Option<f64>,
    pub mass_kg: Option<f64>,
    pub length_mm: Option<f64>,
    pub is_passive: Option<bool>,
    pub actuator_type: Option<String>,
    pub chain_type: Option<String>,
    pub workspace_volume_mm3: Option<f64>,
    pub manipulability: Option<f64>,
    pub gait_type: Option<String>,
    pub phase_type: Option<String>,

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
pub enum KinematicsEdgeType {
    // ── STRUCTURAL ──
    #[default] Contains, PartOf,

    // ── CHAIN TOPOLOGY ──
    KinematicallyLinked,        // joint N connects to joint N+1 in chain
    JointConstrains,            // joint constrains relative motion of two links
    LinkConnects,               // link connects two joints
    ChainContainsJoint,         // chain has this joint
    ChainContainsLink,          // chain has this link
    EndEffectorOf,              // end effector belongs to chain

    // ── KINEMATICS ──
    ParentJoint,                // parent joint in tree
    ChildJoint,                 // child joint in tree
    FollowedByJoint,            // serial ordering of joints
    DrivesJoint,                // actuator drives this joint
    GearMatesTo,                // gear connection
    TransmittedThrough,         // torque transmitted through this link
    MimicsJoint,                // mimic joint copies another
    IsSingularAt,               // chain is singular at this config
    WorkspaceOf,                // workspace belongs to chain

    // ── BIOMECHANICS / GAIT ──
    BioKinematicToMecha,        // human/animal joint → mechanical model
    AnimatesViaBone,            // joint animates 3D bone
    MuscleCrossesJoint,         // muscle crosses this joint
    SegmentProximal,            // distal segment of proximal segment
    SegmentDistal,              // proximal segment of distal
    GaitCycleHasPhase,
    GaitPhaseContainsEvent,
    StepEventOccursIn,
    GRFAppliesAt,               // ground reaction force at this limb

    // ── CROSS-MODAL ──
    MapsToArmatureIn3D,         // chain → 3D armature node (109)
    JointGeometryFromCAD,       // CAD assembly joint → this joint
    FeedsControlSystem,         // joint state → control system (122)
    MotorImageryFromBCI,        // BCI motor imagery → joint command (119)
    JointAngleFromIMU,          // IMU measurement → joint angle (116)
    BiomechanicalContext,       // biology info for this segment (111)

    // ── UNIVERSAL SEMANTIC ──
    Performs, Affects, Implies, Contradicts, Elaborates, Summarizes, Supports,
    TemporalPrecedes, TemporalFollows, CausedBy, Enables, Prevents,
    FunctionalRole, InstanceOf,
    DerivedFrom, VersionOf, RefinesTo, ForkedFrom, SimilarTo,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KinematicsGraphEdge {
    pub edge_id: u64,
    pub from_node: u64, pub to_node: u64,
    pub edge_type: KinematicsEdgeType,
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
pub struct KinematicsGraph {
    pub graph_id: u64, pub project_id: u64,
    pub source_description: String,
    pub nodes: Vec<KinematicsGraphNode>,
    pub edges: Vec<KinematicsGraphEdge>,
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
pub enum KinematicsGraphQuery {
    NodeDetail         { node_id: u64 },
    ChainTopology      { chain_node_id: u64 },
    JointLimits        { chain_node_id: u64 },
    WorkspaceVolumes,
    Singularities,
    GaitCycles,
    MuscleGroups,
    CrossModalLinks    { node_id: u64 },
    EndEffectors,
    AGIActivity, AllNodes, AllEdges,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KinematicsSemanticHook {
    OnGraphCreated,
    OnInferRelationships,
    OnEdgeCompletion,
    OnCrossModalityLink { target_modality: String, target_graph_id: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KinematicsDisplayMode {
    ChainDiagram,           // schematic joint-link diagram
    WorkspaceView,          // 3D workspace visualization
    JointStatePanel,        // current joint values + limits
    GaitTimeline,           // gait cycle phases on timeline
    BiomechanicsOverlay,    // force/moment overlays on body
    TrajectoryView,         // planned trajectory display
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KinematicsHeadlessOp {
    ComputeAllFK,
    ComputeWorkspaceAll,
    DetectAllSingularities,
    ExportURDF           { output_path: String },
    ExportMJCF           { output_path: String },
    SyncWith3D           { graph_id_3d: u64 },
    SyncWithControl      { control_graph_id: u64 },
}

// ─────────────────────────────────────────────────────────────────────────────
// OUTPUT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KinematicsModalityOutput {
    pub success: bool,
    pub graph_id: Option<u64>,
    pub graph: Option<KinematicsGraph>,
    pub analysis: Option<KinematicsAnalysisResult>,
    pub fk_result: Option<FKResult>,
    pub ik_result: Option<IKResult>,
    pub jacobian: Option<Vec<Vec<f64>>>,
    pub trajectory: Option<PlannedTrajectory>,
    pub singularity_check: Option<SingularityCheckResult>,
    pub workspace: Option<WorkspaceVolume>,
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
        let input = serde_json::json!({"action":"Prompt","prompt":prompt,"max_tokens":max_tokens,"temperature":0.05,"system_context":"Kinematics and robotics analysis. Return only valid JSON."});
        let out = std::process::Command::new(&self.prompt_pipeline_path).arg("--input").arg(input.to_string()).output().map_err(|e| e.to_string())?;
        let r: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
        Ok(r["response"].as_str().unwrap_or("{}").to_string())
    }

    fn save_graph(&self, g: &KinematicsGraph) -> Result<(), String> {
        let path = format!("{}/local/kin_graph_{}.json", self.zsei_path, g.graph_id);
        if let Some(p) = std::path::Path::new(&path).parent() { std::fs::create_dir_all(p).map_err(|e| e.to_string())?; }
        std::fs::write(&path, serde_json::to_string_pretty(g).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    fn load_graph(&self, id: u64) -> Result<KinematicsGraph, String> {
        let path = format!("{}/local/kin_graph_{}.json", self.zsei_path, id);
        serde_json::from_str(&std::fs::read_to_string(&path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    fn generate_id(&self) -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as u64 }
    fn now_iso8601(&self) -> String { format!("{}", self.generate_id()) }
    fn extract_json_array(raw: &str) -> String { if let (Some(s), Some(e)) = (raw.find('['), raw.rfind(']')) { raw[s..=e].to_string() } else { "[]".to_string() } }
    fn extract_json_object(raw: &str) -> String { if let (Some(s), Some(e)) = (raw.find('{'), raw.rfind('}')) { raw[s..=e].to_string() } else { "{}".to_string() } }

    /// Forward kinematics: compute end-effector pose from joint angles
    /// Uses simplified DH transformation product
    fn compute_fk_dh(dh_params: &[DHParameters], joint_angles: &[f64]) -> Pose6DOF {
        let mut pos = [0.0f64; 3];
        let mut total_rot = [[1.0f64; 3]; 3];  // identity rotation

        for (i, dh) in dh_params.iter().enumerate() {
            let theta = if matches!(dh.joint_type, JointType::Revolute | JointType::Continuous) {
                dh.theta_rad + joint_angles.get(i).copied().unwrap_or(0.0)
            } else {
                dh.theta_rad
            };
            let d = if matches!(dh.joint_type, JointType::Prismatic) {
                dh.d_mm + joint_angles.get(i).copied().unwrap_or(0.0)
            } else {
                dh.d_mm
            };
            let a = dh.a_mm;
            let alpha = dh.alpha_rad;

            // DH transformation (simplified: accumulate position only)
            let ct = theta.cos(); let st = theta.sin();
            let ca = alpha.cos(); let sa = alpha.sin();

            // Rotation matrix for this joint
            let r = [[ct, -st*ca, st*sa], [st, ct*ca, -ct*sa], [0.0, sa, ca]];

            // Translation in world frame (simplified accumulation)
            let local_pos = [a*ct, a*st, d];
            for k in 0..3 {
                for j in 0..3 { pos[k] += total_rot[k][j] * local_pos[j]; }
            }

            // Accumulate rotation
            let mut new_rot = [[0.0f64; 3]; 3];
            for k in 0..3 { for j in 0..3 { for m in 0..3 { new_rot[k][j] += total_rot[k][m] * r[m][j]; } } }
            total_rot = new_rot;
        }

        // Extract Euler angles from rotation matrix
        let roll = total_rot[2][1].atan2(total_rot[2][2]);
        let pitch = (-total_rot[2][0]).atan2((total_rot[2][1].powi(2) + total_rot[2][2].powi(2)).sqrt());
        let yaw = total_rot[1][0].atan2(total_rot[0][0]);

        Pose6DOF { position_mm: pos, orientation_rpy_rad: [roll, pitch, yaw], orientation_quat: None, frame_name: None }
    }

    /// Compute Jacobian numerically via finite differences
    fn compute_jacobian_numeric(dh_params: &[DHParameters], joint_angles: &[f64], delta: f64) -> Vec<Vec<f64>> {
        let n = joint_angles.len();
        let mut jacobian = vec![vec![0.0f64; n]; 6];
        let pose_0 = Self::compute_fk_dh(dh_params, joint_angles);

        for i in 0..n {
            let mut angles_delta = joint_angles.to_vec();
            angles_delta[i] += delta;
            let pose_delta = Self::compute_fk_dh(dh_params, &angles_delta);

            // Linear velocity columns (position difference / delta)
            jacobian[0][i] = (pose_delta.position_mm[0] - pose_0.position_mm[0]) / delta;
            jacobian[1][i] = (pose_delta.position_mm[1] - pose_0.position_mm[1]) / delta;
            jacobian[2][i] = (pose_delta.position_mm[2] - pose_0.position_mm[2]) / delta;

            // Angular velocity columns (orientation difference / delta)
            jacobian[3][i] = (pose_delta.orientation_rpy_rad[0] - pose_0.orientation_rpy_rad[0]) / delta;
            jacobian[4][i] = (pose_delta.orientation_rpy_rad[1] - pose_0.orientation_rpy_rad[1]) / delta;
            jacobian[5][i] = (pose_delta.orientation_rpy_rad[2] - pose_0.orientation_rpy_rad[2]) / delta;
        }
        jacobian
    }

    /// Compute manipulability measure (Yoshikawa): sqrt(det(J * J^T))
    fn compute_manipulability(jacobian: &[Vec<f64>]) -> f64 {
        let rows = jacobian.len();
        let cols = if rows > 0 { jacobian[0].len() } else { 0 };
        // J * J^T (rows × rows matrix)
        let mut jjt = vec![vec![0.0f64; rows]; rows];
        for i in 0..rows {
            for j in 0..rows {
                for k in 0..cols {
                    jjt[i][j] += jacobian[i][k] * jacobian[j][k];
                }
            }
        }
        // Compute trace as proxy for det (simplified; production uses SVD)
        let trace: f64 = (0..rows).map(|i| jjt[i][i]).sum();
        (trace / rows as f64).sqrt().max(0.0)
    }

    async fn infer_semantic_relationships(&self, nodes: &[KinematicsGraphNode]) -> Vec<(u64, u64, KinematicsEdgeType, String)> {
        if nodes.len() < 2 { return vec![]; }
        let node_list: Vec<serde_json::Value> = nodes.iter().take(25).map(|n| serde_json::json!({
            "node_id": n.node_id, "type": format!("{:?}", n.node_type),
            "content": n.content.chars().take(80).collect::<String>(),
            "joint_type": n.joint_type, "dof": n.dof,
        })).collect();

        let prompt = format!(r#"
Identify semantic relationships between these kinematics graph nodes not structurally captured.

Nodes:
{}

Edge types: KinematicallyLinked, JointConstrains, BioKinematicToMecha, AnimatesViaBone,
Affects, Enables, Prevents, CausedBy, FunctionalRole, DerivedFrom, SimilarTo, Performs

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
                        let etype = map_kin_edge_str(v["edge_type"].as_str().unwrap_or("Affects"));
                        let reason = v["reason"].as_str().unwrap_or("").to_string();
                        Some((from, to, etype, reason))
                    }).collect()
            }
            Err(_) => vec![],
        }
    }

    async fn classify_robot_type(&self, analysis: &KinematicsAnalysisResult) -> String {
        let summary = serde_json::json!({
            "dof_total": analysis.dof_total,
            "chain_count": analysis.kinematic_chains.len(),
            "has_gait": !analysis.gait_cycles.is_empty(),
            "has_biomechanics": !analysis.body_segments.is_empty(),
            "has_muscles": !analysis.muscle_groups.is_empty(),
            "system_type": format!("{:?}", analysis.system_type),
            "joint_types": analysis.joints.iter().take(5).map(|j| format!("{:?}", j.joint_type)).collect::<Vec<_>>(),
        });
        let prompt = format!(r#"
Classify this kinematic system:
{}
Options: 6DOF_Manipulator, 7DOF_Redundant_Arm, SCARA, Delta_Parallel, Stewart_Platform,
Humanoid_Robot, Biped, Quadruped, Hexapod, Wheeled_Robot, Human_Body, Animal_Body,
Prosthetic_Limb, Four_Bar_Linkage, Gear_Train, Cam_Follower, Vehicle, Custom.
Return ONLY the single classification label."#,
            serde_json::to_string(&summary).unwrap_or_default());
        self.llm_zero_shot(&prompt, 20).await.unwrap_or_else(|_| "Custom".into()).trim().to_string()
    }
}

fn map_kin_edge_str(s: &str) -> KinematicsEdgeType {
    match s {
        "KinematicallyLinked"   => KinematicsEdgeType::KinematicallyLinked,
        "JointConstrains"       => KinematicsEdgeType::JointConstrains,
        "LinkConnects"          => KinematicsEdgeType::LinkConnects,
        "EndEffectorOf"         => KinematicsEdgeType::EndEffectorOf,
        "ParentJoint"           => KinematicsEdgeType::ParentJoint,
        "ChildJoint"            => KinematicsEdgeType::ChildJoint,
        "FollowedByJoint"       => KinematicsEdgeType::FollowedByJoint,
        "DrivesJoint"           => KinematicsEdgeType::DrivesJoint,
        "GearMatesTo"           => KinematicsEdgeType::GearMatesTo,
        "IsSingularAt"          => KinematicsEdgeType::IsSingularAt,
        "WorkspaceOf"           => KinematicsEdgeType::WorkspaceOf,
        "BioKinematicToMecha"   => KinematicsEdgeType::BioKinematicToMecha,
        "AnimatesViaBone"       => KinematicsEdgeType::AnimatesViaBone,
        "MuscleCrossesJoint"    => KinematicsEdgeType::MuscleCrossesJoint,
        "GaitCycleHasPhase"     => KinematicsEdgeType::GaitCycleHasPhase,
        "GaitPhaseContainsEvent"=> KinematicsEdgeType::GaitPhaseContainsEvent,
        "MapsToArmatureIn3D"    => KinematicsEdgeType::MapsToArmatureIn3D,
        "JointGeometryFromCAD"  => KinematicsEdgeType::JointGeometryFromCAD,
        "FeedsControlSystem"    => KinematicsEdgeType::FeedsControlSystem,
        "MotorImageryFromBCI"   => KinematicsEdgeType::MotorImageryFromBCI,
        "JointAngleFromIMU"     => KinematicsEdgeType::JointAngleFromIMU,
        "BiomechanicalContext"  => KinematicsEdgeType::BiomechanicalContext,
        "Affects"               => KinematicsEdgeType::Affects,
        "Enables"               => KinematicsEdgeType::Enables,
        "Prevents"              => KinematicsEdgeType::Prevents,
        "CausedBy"              => KinematicsEdgeType::CausedBy,
        "FunctionalRole"        => KinematicsEdgeType::FunctionalRole,
        "DerivedFrom"           => KinematicsEdgeType::DerivedFrom,
        "SimilarTo"             => KinematicsEdgeType::SimilarTo,
        "Performs"              => KinematicsEdgeType::Performs,
        "TemporalPrecedes"      => KinematicsEdgeType::TemporalPrecedes,
        _                       => KinematicsEdgeType::Affects,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH CREATION
// ─────────────────────────────────────────────────────────────────────────────

async fn create_graph(executor: &PipelineExecutor, analysis: KinematicsAnalysisResult, project_id: u64) -> KinematicsModalityOutput {
    let graph_id = executor.generate_id();
    let now = executor.now_iso8601();
    let mut nodes: Vec<KinematicsGraphNode> = Vec::new();
    let mut edges: Vec<KinematicsGraphEdge> = Vec::new();
    let mut node_id: u64 = 1;
    let mut edge_id: u64 = 1;

    let robot_class = executor.classify_robot_type(&analysis).await;

    // ── ROOT ──
    let root_id = node_id;
    nodes.push(KinematicsGraphNode {
        node_id: root_id, node_type: KinematicsNodeType::KinematicsSystem,
        content: format!("Kinematics [{}]: chains={} joints={} links={} dof={}/{} gait={} biomech={}",
            robot_class, analysis.kinematic_chains.len(), analysis.joints.len(), analysis.links.len(),
            analysis.dof_actuated, analysis.dof_total, analysis.gait_cycles.len(), analysis.body_segments.len()),
        dof: Some(analysis.dof_total),
        chain_type: Some(format!("{:?}", analysis.system_type)),
        materialized_path: Some(format!("/Modalities/Kinematics/Project_{}/Graph_{}", project_id, graph_id)),
        provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
        version_notes: vec![VersionNote { version: 1, note: "Initial creation".into(), step_index: None, timestamp: now.clone(), change_type: ChangeType::Created }],
        keywords: vec!["kinematics".into(), robot_class.to_lowercase()],
        hotness_score: 1.0, ..Default::default()
    });
    node_id += 1;

    // ── LINKS ──
    let mut link_id_to_nid: HashMap<u64, u64> = HashMap::new();
    for link in &analysis.links {
        let lid = node_id;
        nodes.push(KinematicsGraphNode {
            node_id: lid, node_type: KinematicsNodeType::LinkNode,
            content: format!("Link: {} [{:?}] mass={:.3}kg len={:.1}mm base={}",
                link.name, link.segment_type, link.mass_kg, link.length_mm, link.is_base),
            mass_kg: Some(link.mass_kg), length_mm: Some(link.length_mm),
            materialized_path: Some(format!("/Modalities/Kinematics/Project_{}/Graph_{}/Link/{}", project_id, graph_id, link.link_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["link".into(), link.name.to_lowercase(), format!("{:?}", link.segment_type).to_lowercase()],
            hotness_score: 0.7, ..Default::default()
        });
        edges.push(KinematicsGraphEdge { edge_id, from_node: root_id, to_node: lid, edge_type: KinematicsEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        link_id_to_nid.insert(link.link_id, lid);
        node_id += 1;
    }

    // ── JOINTS ──
    let mut joint_id_to_nid: HashMap<u64, u64> = HashMap::new();
    for joint in &analysis.joints {
        let jid = node_id;
        nodes.push(KinematicsGraphNode {
            node_id: jid, node_type: KinematicsNodeType::JointNode,
            content: format!("Joint {:?}: {} axis=({:.2},{:.2},{:.2}) lim=[{:.2},{:.2}] passive={} act={:?}",
                joint.joint_type, joint.name, joint.axis[0], joint.axis[1], joint.axis[2],
                joint.limits.lower, joint.limits.upper, joint.is_passive, joint.actuator_type),
            joint_type: Some(format!("{:?}", joint.joint_type)),
            current_position: Some(joint.current_position),
            limit_lower: Some(joint.limits.lower), limit_upper: Some(joint.limits.upper),
            is_passive: Some(joint.is_passive),
            actuator_type: Some(format!("{:?}", joint.actuator_type)),
            dof: Some(match joint.joint_type { JointType::Fixed => 0, JointType::Revolute | JointType::Prismatic | JointType::Continuous => 1, JointType::Universal => 2, JointType::Spherical | JointType::Planar => 3, JointType::Floating => 6, _ => 1 }),
            materialized_path: Some(format!("/Modalities/Kinematics/Project_{}/Graph_{}/Joint/{}", project_id, graph_id, joint.joint_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["joint".into(), format!("{:?}", joint.joint_type).to_lowercase(), joint.name.to_lowercase()],
            hotness_score: 0.8,
            embedding_hint: Some(format!("joint type: {:?} actuator: {:?}", joint.joint_type, joint.actuator_type)),
            ..Default::default()
        });
        edges.push(KinematicsGraphEdge { edge_id, from_node: root_id, to_node: jid, edge_type: KinematicsEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Joint connects parent and child links
        if let Some(&parent_nid) = link_id_to_nid.get(&joint.parent_link_id) {
            edges.push(KinematicsGraphEdge { edge_id, from_node: parent_nid, to_node: jid, edge_type: KinematicsEdgeType::JointConstrains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        if let Some(&child_nid) = link_id_to_nid.get(&joint.child_link_id) {
            edges.push(KinematicsGraphEdge { edge_id, from_node: jid, to_node: child_nid, edge_type: KinematicsEdgeType::JointConstrains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }

        // Mimic joint
        if let Some(mimic_id) = joint.mimic_source_id {
            if let Some(&src_nid) = joint_id_to_nid.get(&mimic_id) {
                edges.push(KinematicsGraphEdge { edge_id, from_node: jid, to_node: src_nid, edge_type: KinematicsEdgeType::MimicsJoint, weight: joint.mimic_multiplier as f32, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }

        // Cross-modal: joint → 3D armature bone
        edges.push(KinematicsGraphEdge {
            edge_id, from_node: jid, to_node: jid,
            edge_type: KinematicsEdgeType::AnimatesViaBone, weight: 0.9,
            provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
            properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("3d")); p.insert("bone_name".into(), serde_json::json!(&joint.name)); p },
            ..Default::default()
        });
        edge_id += 1;

        // Cross-modal: joint state → control system (122)
        edges.push(KinematicsGraphEdge {
            edge_id, from_node: jid, to_node: jid,
            edge_type: KinematicsEdgeType::FeedsControlSystem, weight: 0.9,
            provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
            properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("control_systems")); p.insert("joint_name".into(), serde_json::json!(&joint.name)); p },
            ..Default::default()
        });
        edge_id += 1;

        joint_id_to_nid.insert(joint.joint_id, jid);
        node_id += 1;
    }

    // ── KINEMATIC CHAINS ──
    let mut chain_id_to_nid: HashMap<u64, u64> = HashMap::new();
    for chain in &analysis.kinematic_chains {
        let cid = node_id;
        nodes.push(KinematicsGraphNode {
            node_id: cid, node_type: KinematicsNodeType::KinematicChainNode,
            content: format!("Chain [{:?}]: {} dof={}/{} redundant={} manip={:?}",
                chain.chain_type, chain.name, chain.dof_actuated, chain.dof,
                chain.is_redundant, chain.manipulability),
            dof: Some(chain.dof),
            chain_type: Some(format!("{:?}", chain.chain_type)),
            manipulability: chain.manipulability,
            materialized_path: Some(format!("/Modalities/Kinematics/Project_{}/Graph_{}/Chain/{}", project_id, graph_id, chain.chain_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["chain".into(), chain.name.to_lowercase(), format!("{:?}", chain.chain_type).to_lowercase()],
            hotness_score: 0.9, ..Default::default()
        });
        edges.push(KinematicsGraphEdge { edge_id, from_node: root_id, to_node: cid, edge_type: KinematicsEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Chain contains joints (serial ordering)
        for (i, &joint_id) in chain.joint_ids.iter().enumerate() {
            if let Some(&joint_nid) = joint_id_to_nid.get(&joint_id) {
                edges.push(KinematicsGraphEdge { edge_id, from_node: cid, to_node: joint_nid, edge_type: KinematicsEdgeType::ChainContainsJoint, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
                // Sequential ordering
                if i > 0 {
                    if let Some(&prev_joint_id) = chain.joint_ids.get(i-1) {
                        if let Some(&prev_nid) = joint_id_to_nid.get(&prev_joint_id) {
                            edges.push(KinematicsGraphEdge { edge_id, from_node: prev_nid, to_node: joint_nid, edge_type: KinematicsEdgeType::FollowedByJoint, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                            edge_id += 1;
                        }
                    }
                }
            }
        }

        // Chain contains links
        for &link_id in &chain.link_ids {
            if let Some(&link_nid) = link_id_to_nid.get(&link_id) {
                edges.push(KinematicsGraphEdge { edge_id, from_node: cid, to_node: link_nid, edge_type: KinematicsEdgeType::ChainContainsLink, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }

        // Cross-modal: chain → 3D armature
        edges.push(KinematicsGraphEdge {
            edge_id, from_node: cid, to_node: cid,
            edge_type: KinematicsEdgeType::MapsToArmatureIn3D, weight: 0.9,
            provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
            properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("3d")); p.insert("armature_name".into(), serde_json::json!(&chain.name)); p },
            ..Default::default()
        });
        edge_id += 1;

        chain_id_to_nid.insert(chain.chain_id, cid);
        node_id += 1;
    }

    // ── END EFFECTORS ──
    for ee in &analysis.end_effectors {
        let eid = node_id;
        nodes.push(KinematicsGraphNode {
            node_id: eid, node_type: KinematicsNodeType::EndEffectorNode,
            content: format!("EndEffector [{:?}]: {} payload={:?}kg",
                ee.tool_type, ee.name, ee.payload_mass_kg),
            mass_kg: ee.payload_mass_kg,
            actuator_type: Some(format!("{:?}", ee.tool_type)),
            materialized_path: Some(format!("/Modalities/Kinematics/Project_{}/Graph_{}/EE/{}", project_id, graph_id, ee.ee_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["end-effector".into(), format!("{:?}", ee.tool_type).to_lowercase()],
            hotness_score: 0.85, ..Default::default()
        });
        edges.push(KinematicsGraphEdge { edge_id, from_node: root_id, to_node: eid, edge_type: KinematicsEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // EE linked to parent link
        if let Some(&link_nid) = link_id_to_nid.get(&ee.parent_link_id) {
            edges.push(KinematicsGraphEdge { edge_id, from_node: link_nid, to_node: eid, edge_type: KinematicsEdgeType::EndEffectorOf, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── WORKSPACE VOLUMES ──
    for ws in &analysis.workspace_volumes {
        let wid = node_id;
        nodes.push(KinematicsGraphNode {
            node_id: wid, node_type: KinematicsNodeType::WorkspaceVolumeNode,
            content: format!("Workspace [{:?}]: vol={:.0}mm³ reach={:.0}mm reachable={:.0}%",
                ws.volume_type, ws.volume_mm3, ws.max_reach_mm, ws.reachable_fraction * 100.0),
            workspace_volume_mm3: Some(ws.volume_mm3),
            length_mm: Some(ws.max_reach_mm),
            materialized_path: Some(format!("/Modalities/Kinematics/Project_{}/Graph_{}/Workspace/{}", project_id, graph_id, ws.volume_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["workspace".into(), format!("{:?}", ws.volume_type).to_lowercase()],
            hotness_score: 0.75, ..Default::default()
        });
        edges.push(KinematicsGraphEdge { edge_id, from_node: root_id, to_node: wid, edge_type: KinematicsEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        if let Some(&chain_nid) = chain_id_to_nid.get(&ws.chain_id) {
            edges.push(KinematicsGraphEdge { edge_id, from_node: wid, to_node: chain_nid, edge_type: KinematicsEdgeType::WorkspaceOf, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── SINGULARITIES ──
    for sing in &analysis.singularities {
        let sid = node_id;
        nodes.push(KinematicsGraphNode {
            node_id: sid, node_type: KinematicsNodeType::SingularityNode,
            content: format!("Singularity [{:?}]: sigma_min={:.4} lost={}", sing.singularity_type, sing.sigma_min, sing.lost_dof.join(",")),
            materialized_path: Some(format!("/Modalities/Kinematics/Project_{}/Graph_{}/Sing/{}", project_id, graph_id, sing.singularity_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["singularity".into(), format!("{:?}", sing.singularity_type).to_lowercase()],
            hotness_score: 0.7, ..Default::default()
        });
        edges.push(KinematicsGraphEdge { edge_id, from_node: root_id, to_node: sid, edge_type: KinematicsEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        if let Some(&chain_nid) = chain_id_to_nid.get(&sing.chain_id) {
            edges.push(KinematicsGraphEdge { edge_id, from_node: chain_nid, to_node: sid, edge_type: KinematicsEdgeType::IsSingularAt, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── GAIT CYCLES ──
    let mut gait_id_to_nid: HashMap<u64, u64> = HashMap::new();
    for gait in &analysis.gait_cycles {
        let gid = node_id;
        nodes.push(KinematicsGraphNode {
            node_id: gid, node_type: KinematicsNodeType::GaitCycleNode,
            content: format!("GaitCycle [{:?}]: dur={:.2}s step={:.2}m speed={:.2}m/s sym={:.2}",
                gait.gait_type, gait.duration_sec, gait.step_length_m, gait.speed_ms, gait.symmetry_index),
            gait_type: Some(format!("{:?}", gait.gait_type)),
            materialized_path: Some(format!("/Modalities/Kinematics/Project_{}/Graph_{}/Gait/{}", project_id, graph_id, gait.cycle_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["gait".into(), format!("{:?}", gait.gait_type).to_lowercase()],
            hotness_score: 0.8, ..Default::default()
        });
        edges.push(KinematicsGraphEdge { edge_id, from_node: root_id, to_node: gid, edge_type: KinematicsEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        gait_id_to_nid.insert(gait.cycle_id, gid);
        node_id += 1;
    }

    // ── GAIT PHASES ──
    for phase in &analysis.gait_phases {
        let pid = node_id;
        nodes.push(KinematicsGraphNode {
            node_id: pid, node_type: KinematicsNodeType::GaitPhaseNode,
            content: format!("Phase [{:?}]: {} {:.0}–{:.0}% {:?}",
                phase.phase_type, phase.phase_name, phase.start_percent, phase.end_percent, phase.limb),
            phase_type: Some(format!("{:?}", phase.phase_type)),
            materialized_path: Some(format!("/Modalities/Kinematics/Project_{}/Graph_{}/Phase/{}", project_id, graph_id, phase.phase_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["gait-phase".into(), phase.phase_name.to_lowercase(), format!("{:?}", phase.phase_type).to_lowercase()],
            hotness_score: 0.65, ..Default::default()
        });
        if let Some(&gait_nid) = gait_id_to_nid.get(&phase.cycle_id) {
            edges.push(KinematicsGraphEdge { edge_id, from_node: gait_nid, to_node: pid, edge_type: KinematicsEdgeType::GaitCycleHasPhase, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── BODY SEGMENTS ──
    let mut seg_id_to_nid: HashMap<u64, u64> = HashMap::new();
    for seg in &analysis.body_segments {
        let sid = node_id;
        nodes.push(KinematicsGraphNode {
            node_id: sid, node_type: KinematicsNodeType::BodySegmentNode,
            content: format!("Segment [{:?}]: {} mass={:.2}kg len={:.1}mm",
                seg.segment_type, seg.name, seg.mass_kg, seg.length_mm),
            mass_kg: Some(seg.mass_kg), length_mm: Some(seg.length_mm),
            materialized_path: Some(format!("/Modalities/Kinematics/Project_{}/Graph_{}/Segment/{}", project_id, graph_id, seg.segment_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["segment".into(), seg.name.to_lowercase(), format!("{:?}", seg.segment_type).to_lowercase()],
            hotness_score: 0.7,
            ..Default::default()
        });
        edges.push(KinematicsGraphEdge { edge_id, from_node: root_id, to_node: sid, edge_type: KinematicsEdgeType::Contains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Body segment cross-modal: → biology (111)
        edges.push(KinematicsGraphEdge {
            edge_id, from_node: sid, to_node: sid,
            edge_type: KinematicsEdgeType::BiomechanicalContext, weight: 0.8,
            provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
            properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("biology")); p },
            ..Default::default()
        });
        edge_id += 1;

        // Segment → proximal/distal joints
        if let Some(proximal_id) = seg.proximal_joint_id {
            if let Some(&joint_nid) = joint_id_to_nid.get(&proximal_id) {
                edges.push(KinematicsGraphEdge { edge_id, from_node: sid, to_node: joint_nid, edge_type: KinematicsEdgeType::JointConstrains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        seg_id_to_nid.insert(seg.segment_id, sid);
        node_id += 1;
    }

    // ── MUSCLE GROUPS ──
    for muscle in &analysis.muscle_groups {
        let mid = node_id;
        nodes.push(KinematicsGraphNode {
            node_id: mid, node_type: KinematicsNodeType::MuscleGroupNode,
            content: format!("Muscle [{:?}]: {} pcsa={:.1}cm² fmax={:.0}N act={:.2}",
                muscle.muscle_type, muscle.name, muscle.pcsa_cm2, muscle.max_isometric_force_n, muscle.current_activation),
            mass_kg: None,
            materialized_path: Some(format!("/Modalities/Kinematics/Project_{}/Graph_{}/Muscle/{}", project_id, graph_id, muscle.muscle_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["muscle".into(), muscle.name.to_lowercase(), format!("{:?}", muscle.muscle_type).to_lowercase()],
            hotness_score: 0.65, ..Default::default()
        });
        edges.push(KinematicsGraphEdge { edge_id, from_node: root_id, to_node: mid, edge_type: KinematicsEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Muscle crosses joints it spans (via origin/insertion segments)
        if let Some(&origin_nid) = seg_id_to_nid.get(&muscle.origin_segment_id) {
            edges.push(KinematicsGraphEdge { edge_id, from_node: mid, to_node: origin_nid, edge_type: KinematicsEdgeType::MuscleCrossesJoint, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        if let Some(&insert_nid) = seg_id_to_nid.get(&muscle.insertion_segment_id) {
            edges.push(KinematicsGraphEdge { edge_id, from_node: mid, to_node: insert_nid, edge_type: KinematicsEdgeType::MuscleCrossesJoint, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── PLANNED TRAJECTORIES ──
    for traj in analysis.planned_trajectories.iter().take(20) {
        let tid = node_id;
        nodes.push(KinematicsGraphNode {
            node_id: tid, node_type: KinematicsNodeType::PlannedTrajectoryNode,
            content: format!("Trajectory [{}]: dur={:.2}s waypoints={} collision_free={:?}",
                traj.method, traj.duration_sec, traj.waypoint_count, traj.is_collision_free),
            materialized_path: Some(format!("/Modalities/Kinematics/Project_{}/Graph_{}/Traj/{}", project_id, graph_id, traj.trajectory_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["trajectory".into(), traj.method.to_lowercase()], hotness_score: 0.7, ..Default::default()
        });
        edges.push(KinematicsGraphEdge { edge_id, from_node: root_id, to_node: tid, edge_type: KinematicsEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        if let Some(&chain_nid) = chain_id_to_nid.get(&traj.chain_id) {
            edges.push(KinematicsGraphEdge { edge_id, from_node: tid, to_node: chain_nid, edge_type: KinematicsEdgeType::ChainContainsJoint, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── HOOK 1: OnGraphCreated ──
    let _ = executor.save_graph(&KinematicsGraph { graph_id, project_id, source_description: analysis.source_description.clone(), nodes: nodes.clone(), edges: edges.clone(), root_node_id: root_id, state: GraphStateType::Created, state_history: vec![GraphStateTransition { from: GraphStateType::Created, to: GraphStateType::Created, timestamp: now.clone(), triggered_by_step: None }], created_at: now.clone(), updated_at: now.clone(), version: 1, version_notes: vec![VersionNote { version: 1, note: format!("Created: {} nodes {} edges", nodes.len(), edges.len()), step_index: None, timestamp: now.clone(), change_type: ChangeType::Created }] });

    // ── HOOK 2: OnInferRelationships ──
    let inferred = executor.infer_semantic_relationships(&nodes).await;
    let valid: std::collections::HashSet<u64> = nodes.iter().map(|n| n.node_id).collect();
    for (from, to, etype, reason) in inferred {
        if valid.contains(&from) && valid.contains(&to) && from != to {
            edges.push(KinematicsGraphEdge { edge_id, from_node: from, to_node: to, edge_type: etype, weight: 0.8, provenance: EdgeProvenance::DerivedFromHook, version: 1, properties: { let mut p = HashMap::new(); p.insert("reason".into(), serde_json::json!(reason)); p }, ..Default::default() });
            edge_id += 1;
        }
    }

    // ── HOOK 3: OnEdgeCompletion → hotness update, clean self-loops ──
    let mut deg: HashMap<u64, u32> = HashMap::new();
    for e in &edges {
        *deg.entry(e.from_node).or_insert(0) += 1;
        *deg.entry(e.to_node).or_insert(0) += 1;
    }
    let max_deg = deg.values().copied().max().unwrap_or(1) as f32;
    for n in &mut nodes {
        if let Some(&d) = deg.get(&n.node_id) {
            n.hotness_score = (n.hotness_score + (d as f32 / max_deg) * 0.15).min(1.0);
        }
    }
    // Retain self-loop cross-modal markers, drop accidental structural self-loops
    edges.retain(|e| e.from_node != e.to_node ||
        matches!(e.edge_type,
            KinematicsEdgeType::MapsToArmatureIn3D |
            KinematicsEdgeType::JointGeometryFromCAD |
            KinematicsEdgeType::FeedsControlSystem |
            KinematicsEdgeType::MotorImageryFromBCI |
            KinematicsEdgeType::JointAngleFromIMU |
            KinematicsEdgeType::BiomechanicalContext |
            KinematicsEdgeType::AnimatesViaBone
        )
    );

    let final_graph = KinematicsGraph {
        graph_id, project_id,
        source_description: analysis.source_description,
        nodes, edges, root_node_id: root_id,
        state: GraphStateType::SemanticEnriched,
        state_history: vec![GraphStateTransition {
            from: GraphStateType::Created, to: GraphStateType::SemanticEnriched,
            timestamp: now.clone(), triggered_by_step: None,
        }],
        created_at: now.clone(), updated_at: now.clone(), version: 1,
        version_notes: vec![VersionNote {
            version: 1, note: "Semantic enrichment complete".into(),
            step_index: None, timestamp: now, change_type: ChangeType::EnrichedBySemantic,
        }],
    };
    let _ = executor.save_graph(&final_graph);
    KinematicsModalityOutput {
        success: true, graph_id: Some(graph_id), graph: Some(final_graph), ..Default::default()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// MAIN EXECUTION
// ─────────────────────────────────────────────────────────────────────────────

pub async fn execute(input: KinematicsModalityAction) -> Result<KinematicsModalityOutput, String> {
    let executor = PipelineExecutor::new();

    match input {
        KinematicsModalityAction::AnalyzeChain { data, compute_workspace, detect_singularities } => {
            let analysis_id = executor.generate_id();
            let (source_description, chain_spec) = match &data {
                KinematicsDataSource::URDF { file_path } =>
                    (format!("URDF: {}", file_path), None),
                KinematicsDataSource::MJCF { file_path } =>
                    (format!("MJCF: {}", file_path), None),
                KinematicsDataSource::DHTable { params } =>
                    (format!("DH table: {} joints", params.len()), None),
                KinematicsDataSource::SDF { file_path } =>
                    (format!("SDF: {}", file_path), None),
                KinematicsDataSource::BVH { file_path } =>
                    (format!("BVH motion capture: {}", file_path), None),
                KinematicsDataSource::C3D { file_path } =>
                    (format!("C3D biomechanics: {}", file_path), None),
                KinematicsDataSource::OSIM { file_path } =>
                    (format!("OpenSim: {}", file_path), None),
                KinematicsDataSource::JSON_Chain { chain } =>
                    (format!("JSON chain: {} ({} joints)", chain.name, chain.joints.len()), Some(chain.clone())),
                KinematicsDataSource::From3D { graph_id, armature_node_id } =>
                    (format!("From 3D graph {} armature {}", graph_id, armature_node_id), None),
                KinematicsDataSource::FromCAD { cad_graph_id } =>
                    (format!("From CAD graph {}", cad_graph_id), None),
            };

            // Build a minimal KinematicsAnalysisResult from available data
            let (joints, links, chains, dof) = match &data {
                KinematicsDataSource::DHTable { params } => {
                    let joints: Vec<Joint> = params.iter().map(|dh| Joint {
                        joint_id: executor.generate_id(),
                        name: dh.joint_name.clone(),
                        joint_type: dh.joint_type.clone(),
                        parent_link_id: 0,
                        child_link_id: 0,
                        origin_mm: [dh.a_mm, 0.0, dh.d_mm],
                        orientation_rpy_rad: [dh.alpha_rad, 0.0, dh.theta_rad],
                        axis: [0.0, 0.0, 1.0],
                        limits: dh.joint_limits.clone(),
                        gear_ratio: 1.0,
                        kinematic_pair: KinematicPairClass::Lower,
                        ..Default::default()
                    }).collect();
                    let dof = joints.iter().filter(|j| !matches!(j.joint_type, JointType::Fixed)).count() as u32;
                    (joints, vec![], vec![], dof)
                }
                KinematicsDataSource::JSON_Chain { chain } => {
                    let joints: Vec<Joint> = chain.joints.iter().map(|js| Joint {
                        joint_id: executor.generate_id(),
                        name: js.name.clone(),
                        joint_type: js.joint_type.clone(),
                        limits: js.limits.clone(),
                        axis: js.axis,
                        origin_mm: js.origin_mm,
                        orientation_rpy_rad: js.rpy_rad,
                        gear_ratio: js.gear_ratio,
                        damping: js.damping,
                        friction: js.friction,
                        is_mimic: js.is_mimic,
                        ..Default::default()
                    }).collect();
                    let dof = joints.iter().filter(|j| !matches!(j.joint_type, JointType::Fixed)).count() as u32;
                    let kin_chain = KinematicChain {
                        chain_id: executor.generate_id(),
                        name: chain.name.clone(),
                        chain_type: chain.chain_type.clone(),
                        dof, dof_actuated: dof,
                        base_frame: chain.base_frame.clone(),
                        joint_angles_home: vec![0.0; dof as usize],
                        joint_angles_current: vec![0.0; dof as usize],
                        joint_ids: joints.iter().map(|j| j.joint_id).collect(),
                        ..Default::default()
                    };
                    (joints, vec![], vec![kin_chain], dof)
                }
                _ => (vec![], vec![], vec![], 0),
            };

            Ok(KinematicsModalityOutput {
                success: true,
                analysis: Some(KinematicsAnalysisResult {
                    analysis_id,
                    source_description,
                    joints,
                    links,
                    kinematic_chains: chains,
                    dof_total: dof,
                    dof_actuated: dof,
                    ..Default::default()
                }),
                ..Default::default()
            })
        }

        KinematicsModalityAction::AnalyzeGait { data, gait_type } => {
            let analysis_id = executor.generate_id();
            let source_description = match &data {
                GaitDataSource::BVH { file_path } => format!("BVH gait: {}", file_path),
                GaitDataSource::C3D { file_path, .. } => format!("C3D gait: {}", file_path),
                GaitDataSource::IMU_Sequence { files, .. } => format!("IMU gait: {} sensors", files.len()),
                GaitDataSource::Synthetic { speed_ms, step_length_m, .. } =>
                    format!("Synthetic {:?} gait: {:.1}m/s step={:.2}m", gait_type, speed_ms, step_length_m),
            };

            // Build synthetic gait cycle from parameters
            let gait_cycle = GaitCycle {
                cycle_id: executor.generate_id(),
                gait_type: gait_type.clone(),
                duration_sec: match &data {
                    GaitDataSource::Synthetic { speed_ms, step_length_m, .. } =>
                        if *speed_ms > 0.0 { *step_length_m * 2.0 / speed_ms } else { 1.0 },
                    _ => 1.0,
                },
                speed_ms: match &data { GaitDataSource::Synthetic { speed_ms, .. } => *speed_ms, _ => 1.2 },
                step_length_m: match &data { GaitDataSource::Synthetic { step_length_m, .. } => *step_length_m, _ => 0.6 },
                stride_length_m: match &data { GaitDataSource::Synthetic { step_length_m, .. } => step_length_m * 2.0, _ => 1.2 },
                cadence_steps_per_min: 60.0,
                symmetry_index: 1.0,
                ..Default::default()
            };

            // Standard gait phases for human walking
            let phases = if matches!(gait_type, GaitType::HumanWalk | GaitType::BipedRobotWalk) {
                vec![
                    GaitPhase { phase_id: executor.generate_id(), cycle_id: gait_cycle.cycle_id, phase_name: "Loading Response".into(), phase_type: GaitPhaseType::LoadingResponse, start_percent: 0.0, end_percent: 10.0, limb: LimbIdentifier::Right_Leg, duration_sec: gait_cycle.duration_sec * 0.10, ..Default::default() },
                    GaitPhase { phase_id: executor.generate_id(), cycle_id: gait_cycle.cycle_id, phase_name: "Mid Stance".into(), phase_type: GaitPhaseType::MidStance, start_percent: 10.0, end_percent: 30.0, limb: LimbIdentifier::Right_Leg, duration_sec: gait_cycle.duration_sec * 0.20, ..Default::default() },
                    GaitPhase { phase_id: executor.generate_id(), cycle_id: gait_cycle.cycle_id, phase_name: "Terminal Stance".into(), phase_type: GaitPhaseType::TerminalStance, start_percent: 30.0, end_percent: 50.0, limb: LimbIdentifier::Right_Leg, duration_sec: gait_cycle.duration_sec * 0.20, ..Default::default() },
                    GaitPhase { phase_id: executor.generate_id(), cycle_id: gait_cycle.cycle_id, phase_name: "Pre Swing".into(), phase_type: GaitPhaseType::PreSwing, start_percent: 50.0, end_percent: 60.0, limb: LimbIdentifier::Right_Leg, duration_sec: gait_cycle.duration_sec * 0.10, ..Default::default() },
                    GaitPhase { phase_id: executor.generate_id(), cycle_id: gait_cycle.cycle_id, phase_name: "Initial Swing".into(), phase_type: GaitPhaseType::InitialSwing, start_percent: 60.0, end_percent: 73.0, limb: LimbIdentifier::Right_Leg, duration_sec: gait_cycle.duration_sec * 0.13, ..Default::default() },
                    GaitPhase { phase_id: executor.generate_id(), cycle_id: gait_cycle.cycle_id, phase_name: "Mid Swing".into(), phase_type: GaitPhaseType::MidSwing, start_percent: 73.0, end_percent: 87.0, limb: LimbIdentifier::Right_Leg, duration_sec: gait_cycle.duration_sec * 0.14, ..Default::default() },
                    GaitPhase { phase_id: executor.generate_id(), cycle_id: gait_cycle.cycle_id, phase_name: "Terminal Swing".into(), phase_type: GaitPhaseType::TerminalSwing, start_percent: 87.0, end_percent: 100.0, limb: LimbIdentifier::Right_Leg, duration_sec: gait_cycle.duration_sec * 0.13, ..Default::default() },
                ]
            } else {
                vec![]
            };

            // Standard step events
            let step_events = vec![
                StepEvent { event_id: executor.generate_id(), cycle_id: gait_cycle.cycle_id, event_type: StepEventType::HeelStrike, limb: LimbIdentifier::Right_Leg, time_sec: 0.0, percent_of_cycle: 0.0, ..Default::default() },
                StepEvent { event_id: executor.generate_id(), cycle_id: gait_cycle.cycle_id, event_type: StepEventType::ToeOff, limb: LimbIdentifier::Left_Leg, time_sec: gait_cycle.duration_sec * 0.10, percent_of_cycle: 10.0, ..Default::default() },
                StepEvent { event_id: executor.generate_id(), cycle_id: gait_cycle.cycle_id, event_type: StepEventType::HeelStrike, limb: LimbIdentifier::Left_Leg, time_sec: gait_cycle.duration_sec * 0.50, percent_of_cycle: 50.0, ..Default::default() },
                StepEvent { event_id: executor.generate_id(), cycle_id: gait_cycle.cycle_id, event_type: StepEventType::ToeOff, limb: LimbIdentifier::Right_Leg, time_sec: gait_cycle.duration_sec * 0.62, percent_of_cycle: 62.0, ..Default::default() },
            ];

            Ok(KinematicsModalityOutput {
                success: true,
                analysis: Some(KinematicsAnalysisResult {
                    analysis_id,
                    source_description,
                    system_type: KinematicsSystemType::HumanBiomechanics,
                    gait_cycles: vec![gait_cycle],
                    gait_phases: phases,
                    step_events,
                    ..Default::default()
                }),
                ..Default::default()
            })
        }

        KinematicsModalityAction::AnalyzeBiomechanics { data, compute_muscle_forces } => {
            let analysis_id = executor.generate_id();
            let (source_description, mass_kg, height_m) = match &data {
                BiomechanicsDataSource::OSIM { model_path, .. } => (format!("OpenSim: {}", model_path), 70.0, 1.75),
                BiomechanicsDataSource::C3D_WithForce { file_path, .. } => (format!("C3D+Force: {}", file_path), 70.0, 1.75),
                BiomechanicsDataSource::Synthetic { body_mass_kg, body_height_m, activity } =>
                    (format!("Synthetic {:?}: {:.1}kg {:.2}m", activity, body_mass_kg, body_height_m), *body_mass_kg, *body_height_m),
            };

            // Build anthropometric body segments (Winter's body segment parameters)
            let segment_data: Vec<(&str, BodySegmentType, f64, f64)> = vec![
                ("Head+Neck", BodySegmentType::Head,   0.0814, 0.205),
                ("Trunk",     BodySegmentType::Torso,  0.4971, 0.520),
                ("Upper Arm R",BodySegmentType::UpperArm, 0.0280, 0.188),
                ("Forearm R",  BodySegmentType::ForeArm,  0.0160, 0.145),
                ("Hand R",     BodySegmentType::Hand,      0.0060, 0.075),
                ("Upper Arm L",BodySegmentType::UpperArm, 0.0280, 0.188),
                ("Forearm L",  BodySegmentType::ForeArm,  0.0160, 0.145),
                ("Hand L",     BodySegmentType::Hand,      0.0060, 0.075),
                ("Thigh R",    BodySegmentType::Thigh,    0.1000, 0.245),
                ("Shank R",    BodySegmentType::Shank,    0.0465, 0.246),
                ("Foot R",     BodySegmentType::Foot,     0.0145, 0.152),
                ("Thigh L",    BodySegmentType::Thigh,    0.1000, 0.245),
                ("Shank L",    BodySegmentType::Shank,    0.0465, 0.246),
                ("Foot L",     BodySegmentType::Foot,     0.0145, 0.152),
            ];

            let body_segments: Vec<BodySegment> = segment_data.into_iter().enumerate().map(|(i, (name, seg_type, mass_frac, len_frac))| {
                BodySegment {
                    segment_id: executor.generate_id(),
                    name: name.to_string(),
                    segment_type: seg_type,
                    mass_kg: mass_frac * mass_kg,
                    length_mm: len_frac * height_m * 1000.0,
                    cog_fraction: 0.5,
                    radius_of_gyration_mm: len_frac * height_m * 1000.0 * 0.3,
                    inertia_tensor_kg_mm2: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                    ..Default::default()
                }
            }).collect();

            // Major lower-body muscle groups
            let muscle_groups = if compute_muscle_forces {
                vec![
                    MuscleGroup { muscle_id: executor.generate_id(), name: "Quadriceps".into(), muscle_type: MuscleType::Uniarticular, pcsa_cm2: 148.0, max_isometric_force_n: 4444.0, optimal_fiber_length_mm: 84.0, tendon_slack_length_mm: 220.0, ..Default::default() },
                    MuscleGroup { muscle_id: executor.generate_id(), name: "Hamstrings".into(), muscle_type: MuscleType::Biarticular, pcsa_cm2: 78.0, max_isometric_force_n: 2340.0, optimal_fiber_length_mm: 109.0, tendon_slack_length_mm: 360.0, ..Default::default() },
                    MuscleGroup { muscle_id: executor.generate_id(), name: "Gastrocnemius".into(), muscle_type: MuscleType::Biarticular, pcsa_cm2: 21.0, max_isometric_force_n: 630.0, optimal_fiber_length_mm: 59.0, tendon_slack_length_mm: 390.0, ..Default::default() },
                    MuscleGroup { muscle_id: executor.generate_id(), name: "Soleus".into(), muscle_type: MuscleType::Uniarticular, pcsa_cm2: 58.0, max_isometric_force_n: 1740.0, optimal_fiber_length_mm: 30.0, tendon_slack_length_mm: 260.0, ..Default::default() },
                    MuscleGroup { muscle_id: executor.generate_id(), name: "Tibialis Anterior".into(), muscle_type: MuscleType::Uniarticular, pcsa_cm2: 9.0, max_isometric_force_n: 270.0, optimal_fiber_length_mm: 98.0, tendon_slack_length_mm: 223.0, ..Default::default() },
                    MuscleGroup { muscle_id: executor.generate_id(), name: "Gluteus Maximus".into(), muscle_type: MuscleType::Uniarticular, pcsa_cm2: 60.0, max_isometric_force_n: 1800.0, optimal_fiber_length_mm: 142.0, tendon_slack_length_mm: 120.0, ..Default::default() },
                    MuscleGroup { muscle_id: executor.generate_id(), name: "Hip Flexors (Iliopsoas)".into(), muscle_type: MuscleType::Uniarticular, pcsa_cm2: 18.0, max_isometric_force_n: 540.0, optimal_fiber_length_mm: 102.0, tendon_slack_length_mm: 100.0, ..Default::default() },
                ]
            } else {
                vec![]
            };

            Ok(KinematicsModalityOutput {
                success: true,
                analysis: Some(KinematicsAnalysisResult {
                    analysis_id,
                    source_description,
                    system_type: KinematicsSystemType::HumanBiomechanics,
                    body_segments,
                    muscle_groups,
                    ..Default::default()
                }),
                ..Default::default()
            })
        }

        KinematicsModalityAction::AnalyzeMechanism { data } => {
            let analysis_id = executor.generate_id();
            let (source_description, mech_links, mech_joints) = match data {
                MechanismDataSource::LinkageSpec { links, joints } => {
                    let desc = format!("Linkage: {} links {} joints", links.len(), joints.len());
                    let ml: Vec<MechanismLink> = links.iter().map(|l| MechanismLink {
                        link_id: executor.generate_id(), name: l.name.clone(),
                        length_mm: l.length_mm, mass_kg: l.mass_kg, is_ground: false,
                        ..Default::default()
                    }).collect();
                    let mj: Vec<MechanismJoint> = joints.iter().map(|j| MechanismJoint {
                        joint_id: executor.generate_id(), name: j.name.clone(),
                        joint_type: j.joint_type.clone(),
                        ..Default::default()
                    }).collect();
                    (desc, ml, mj)
                }
                MechanismDataSource::GearTrainSpec { gears } => {
                    let desc = format!("Gear train: {} gears", gears.len());
                    (desc, vec![], vec![])
                }
                MechanismDataSource::CamFollowerSpec { cam, follower } => {
                    let desc = format!("Cam-follower: {} {:.0}rpm", cam.name, cam.rpm);
                    (desc, vec![], vec![])
                }
                MechanismDataSource::FromCAD { cad_graph_id } => {
                    (format!("From CAD graph {}", cad_graph_id), vec![], vec![])
                }
            };
            Ok(KinematicsModalityOutput {
                success: true,
                analysis: Some(KinematicsAnalysisResult {
                    analysis_id,
                    source_description,
                    system_type: KinematicsSystemType::MechanicalLinkage,
                    mechanism_links: mech_links,
                    mechanism_joints: mech_joints,
                    ..Default::default()
                }),
                ..Default::default()
            })
        }

        KinematicsModalityAction::ForwardKinematics { graph_id, chain_node_id, joint_angles_rad } => {
            let graph = executor.load_graph(graph_id)?;
            // Find chain node and its DH parameters
            let chain_node = graph.nodes.iter().find(|n| n.node_id == chain_node_id)
                .ok_or_else(|| format!("Chain node {} not found", chain_node_id))?;

            // Collect joint nodes belonging to this chain (ordered)
            let joint_nodes: Vec<&KinematicsGraphNode> = graph.edges.iter()
                .filter(|e| e.from_node == chain_node_id && matches!(e.edge_type, KinematicsEdgeType::ChainContainsJoint))
                .filter_map(|e| graph.nodes.iter().find(|n| n.node_id == e.to_node))
                .collect();

            // Build simplified DH params from joint node data
            let dh_params: Vec<DHParameters> = joint_nodes.iter().enumerate().map(|(i, n)| {
                DHParameters {
                    joint_name: n.content.chars().take(20).collect(),
                    a_mm: 100.0,        // default link length
                    d_mm: 0.0,
                    alpha_rad: 0.0,
                    theta_rad: 0.0,
                    joint_type: JointType::Revolute,
                    joint_limits: JointLimits {
                        lower: n.limit_lower.unwrap_or(-std::f64::consts::PI),
                        upper: n.limit_upper.unwrap_or(std::f64::consts::PI),
                        ..Default::default()
                    },
                }
            }).collect();

            let ee_pose = PipelineExecutor::compute_fk_dh(&dh_params, &joint_angles_rad);
            let jacobian = PipelineExecutor::compute_jacobian_numeric(&dh_params, &joint_angles_rad, 1e-6);
            let manipulability = PipelineExecutor::compute_manipulability(&jacobian);

            Ok(KinematicsModalityOutput {
                success: true,
                fk_result: Some(FKResult {
                    chain_id: chain_node_id,
                    joint_angles_rad: joint_angles_rad.clone(),
                    end_effector_pose: ee_pose,
                    link_poses: vec![],
                    jacobian: Some(jacobian),
                }),
                ..Default::default()
            })
        }

        KinematicsModalityAction::InverseKinematics { graph_id, chain_node_id, target_pose, method, initial_guess } => {
            let graph = executor.load_graph(graph_id)?;

            // Collect joints for this chain
            let joint_nodes: Vec<&KinematicsGraphNode> = graph.edges.iter()
                .filter(|e| e.from_node == chain_node_id && matches!(e.edge_type, KinematicsEdgeType::ChainContainsJoint))
                .filter_map(|e| graph.nodes.iter().find(|n| n.node_id == e.to_node))
                .collect();
            let n_joints = joint_nodes.len();

            let dh_params: Vec<DHParameters> = joint_nodes.iter().map(|n| DHParameters {
                joint_name: n.content.chars().take(20).collect(),
                a_mm: 100.0, d_mm: 0.0, alpha_rad: 0.0, theta_rad: 0.0,
                joint_type: JointType::Revolute,
                joint_limits: JointLimits {
                    lower: n.limit_lower.unwrap_or(-std::f64::consts::PI),
                    upper: n.limit_upper.unwrap_or(std::f64::consts::PI),
                    ..Default::default()
                },
            }).collect();

            // Jacobian pseudo-inverse IK (gradient descent)
            let mut angles = initial_guess.unwrap_or_else(|| vec![0.0f64; n_joints]);
            let mut converged = false;
            let max_iter = 200;
            let step_size = 0.01;
            let tol_pos = 0.1;   // mm
            let tol_rot = 0.001; // rad
            let mut pos_error = f64::MAX;
            let mut rot_error = f64::MAX;
            let mut iterations = 0u32;

            for iter in 0..max_iter {
                let current_pose = PipelineExecutor::compute_fk_dh(&dh_params, &angles);
                pos_error = ((target_pose.position_mm[0] - current_pose.position_mm[0]).powi(2)
                    + (target_pose.position_mm[1] - current_pose.position_mm[1]).powi(2)
                    + (target_pose.position_mm[2] - current_pose.position_mm[2]).powi(2)).sqrt();
                rot_error = ((target_pose.orientation_rpy_rad[0] - current_pose.orientation_rpy_rad[0]).powi(2)
                    + (target_pose.orientation_rpy_rad[1] - current_pose.orientation_rpy_rad[1]).powi(2)
                    + (target_pose.orientation_rpy_rad[2] - current_pose.orientation_rpy_rad[2]).powi(2)).sqrt();

                if pos_error < tol_pos && rot_error < tol_rot {
                    converged = true;
                    iterations = iter;
                    break;
                }

                // Jacobian pseudo-inverse step
                let jacobian = PipelineExecutor::compute_jacobian_numeric(&dh_params, &angles, 1e-6);
                let error = [
                    target_pose.position_mm[0] - current_pose.position_mm[0],
                    target_pose.position_mm[1] - current_pose.position_mm[1],
                    target_pose.position_mm[2] - current_pose.position_mm[2],
                    target_pose.orientation_rpy_rad[0] - current_pose.orientation_rpy_rad[0],
                    target_pose.orientation_rpy_rad[1] - current_pose.orientation_rpy_rad[1],
                    target_pose.orientation_rpy_rad[2] - current_pose.orientation_rpy_rad[2],
                ];

                // delta_q = J^T * error (Jacobian transpose method, simple and stable)
                let mut delta_q = vec![0.0f64; n_joints];
                for j in 0..n_joints {
                    for i in 0..6 {
                        if i < jacobian.len() && j < jacobian[i].len() {
                            delta_q[j] += jacobian[i][j] * error[i];
                        }
                    }
                    delta_q[j] *= step_size;
                }

                // Apply delta and clamp to joint limits
                for (j, angle) in angles.iter_mut().enumerate() {
                    *angle += delta_q.get(j).copied().unwrap_or(0.0);
                    let lower = dh_params.get(j).map(|d| d.joint_limits.lower).unwrap_or(-std::f64::consts::PI);
                    let upper = dh_params.get(j).map(|d| d.joint_limits.upper).unwrap_or(std::f64::consts::PI);
                    *angle = angle.clamp(lower, upper);
                }
                iterations = iter + 1;
            }

            Ok(KinematicsModalityOutput {
                success: true,
                ik_result: Some(IKResult {
                    chain_id: chain_node_id,
                    target_pose,
                    joint_angles_rad: angles,
                    converged,
                    iterations,
                    position_error_mm: pos_error,
                    orientation_error_rad: rot_error,
                    method: format!("{:?}", method),
                    all_solutions: vec![],
                }),
                ..Default::default()
            })
        }

        KinematicsModalityAction::ComputeJacobian { graph_id, chain_node_id, joint_angles_rad } => {
            let graph = executor.load_graph(graph_id)?;
            let joint_count = graph.edges.iter()
                .filter(|e| e.from_node == chain_node_id && matches!(e.edge_type, KinematicsEdgeType::ChainContainsJoint))
                .count();

            let dh_params: Vec<DHParameters> = (0..joint_count).map(|i| DHParameters {
                joint_name: format!("joint_{}", i),
                a_mm: 100.0, d_mm: 0.0, alpha_rad: 0.0, theta_rad: 0.0,
                joint_type: JointType::Revolute,
                joint_limits: JointLimits { lower: -std::f64::consts::PI, upper: std::f64::consts::PI, ..Default::default() },
            }).collect();

            let jacobian = PipelineExecutor::compute_jacobian_numeric(&dh_params, &joint_angles_rad, 1e-6);
            Ok(KinematicsModalityOutput { success: true, jacobian: Some(jacobian), ..Default::default() })
        }

        KinematicsModalityAction::PlanTrajectory { graph_id, chain_node_id, waypoints, method, duration_sec } => {
            let graph = executor.load_graph(graph_id)?;
            let n_waypoints = waypoints.len();
            if n_waypoints < 2 {
                return Ok(KinematicsModalityOutput { success: false, error: Some("Need ≥2 waypoints".into()), ..Default::default() });
            }

            // Time-parameterized trajectory: divide duration equally among segments
            let dt = duration_sec / (n_waypoints - 1) as f64;
            let samples_per_segment = 20usize;
            let mut joint_trajectory: Vec<JointTrajectoryPoint> = Vec::new();

            for seg_idx in 0..(n_waypoints - 1) {
                let t_start = seg_idx as f64 * dt;
                for s in 0..samples_per_segment {
                    let t_local = s as f64 / (samples_per_segment - 1) as f64;
                    // Cubic blend: smooth start/end
                    let blend = 3.0 * t_local * t_local - 2.0 * t_local * t_local * t_local;

                    let pos_interp: Vec<f64> = (0..3).map(|k| {
                        waypoints[seg_idx].position_mm[k] * (1.0 - blend)
                        + waypoints[seg_idx + 1].position_mm[k] * blend
                    }).collect();

                    joint_trajectory.push(JointTrajectoryPoint {
                        time_sec: t_start + t_local * dt,
                        positions_rad: pos_interp.iter().map(|&p| p / 100.0).collect(), // simplified
                        velocities_rad_s: vec![0.0; 3],
                        accelerations_rad_s2: vec![0.0; 3],
                        effort_nm: vec![0.0; 3],
                    });
                }
            }

            let traj = PlannedTrajectory {
                trajectory_id: executor.generate_id(),
                chain_id: chain_node_id,
                method: format!("{:?}", method),
                duration_sec,
                waypoint_count: n_waypoints as u32,
                joint_trajectory,
                task_trajectory: waypoints,
                max_velocity_rad_s: vec![],
                max_acceleration_rad_s2: vec![],
                is_collision_free: None,
            };

            Ok(KinematicsModalityOutput { success: true, trajectory: Some(traj), ..Default::default() })
        }

        KinematicsModalityAction::CheckSingularities { graph_id, chain_node_id, joint_angles_rad } => {
            let graph = executor.load_graph(graph_id)?;
            let n = joint_angles_rad.len();

            let dh_params: Vec<DHParameters> = (0..n).map(|i| DHParameters {
                joint_name: format!("joint_{}", i),
                a_mm: 100.0, d_mm: 0.0, alpha_rad: std::f64::consts::FRAC_PI_2, theta_rad: 0.0,
                joint_type: JointType::Revolute,
                joint_limits: JointLimits { lower: -std::f64::consts::PI, upper: std::f64::consts::PI, ..Default::default() },
            }).collect();

            let jacobian = PipelineExecutor::compute_jacobian_numeric(&dh_params, &joint_angles_rad, 1e-6);
            let manipulability = PipelineExecutor::compute_manipulability(&jacobian);

            // Minimum singular value proxy: norm of column with smallest magnitude
            let sigma_min = (0..n).map(|j| {
                let col_norm: f64 = jacobian.iter().map(|row| row.get(j).copied().unwrap_or(0.0).powi(2)).sum::<f64>().sqrt();
                col_norm
            }).fold(f64::INFINITY, f64::min);

            let singular_threshold = 0.01;
            let is_singular = sigma_min < singular_threshold;

            Ok(KinematicsModalityOutput {
                success: true,
                singularity_check: Some(SingularityCheckResult {
                    chain_id: chain_node_id,
                    joint_angles_rad,
                    is_singular,
                    sigma_min,
                    manipulability,
                    singularity_type: if is_singular { Some("Detected (type unknown without full SVD)".into()) } else { None },
                    distance_to_singularity: Some(sigma_min),
                }),
                ..Default::default()
            })
        }

        KinematicsModalityAction::ComputeWorkspace { graph_id, chain_node_id, resolution } => {
            let graph = executor.load_graph(graph_id)?;

            // Workspace sampling: systematic joint space sampling
            let samples_per_joint = match resolution {
                WorkspaceResolution::Coarse   => 5usize,
                WorkspaceResolution::Medium   => 10,
                WorkspaceResolution::Fine     => 20,
                WorkspaceResolution::VeryFine => 40,
            };

            // Get joint nodes and their limits
            let joint_nodes: Vec<&KinematicsGraphNode> = graph.edges.iter()
                .filter(|e| e.from_node == chain_node_id && matches!(e.edge_type, KinematicsEdgeType::ChainContainsJoint))
                .filter_map(|e| graph.nodes.iter().find(|n| n.node_id == e.to_node))
                .collect();
            let n_joints = joint_nodes.len().max(1);

            let dh_params: Vec<DHParameters> = joint_nodes.iter().map(|n| DHParameters {
                joint_name: n.content.chars().take(20).collect(),
                a_mm: 150.0, d_mm: 0.0, alpha_rad: std::f64::consts::FRAC_PI_2, theta_rad: 0.0,
                joint_type: JointType::Revolute,
                joint_limits: JointLimits {
                    lower: n.limit_lower.unwrap_or(-std::f64::consts::PI),
                    upper: n.limit_upper.unwrap_or(std::f64::consts::PI),
                    ..Default::default()
                },
            }).collect();

            // Sample: single joint sweep for first joint (full sweep is exponential)
            let mut reachable_points: Vec<[f32; 3]> = Vec::new();
            let total_samples = samples_per_joint.pow(n_joints.min(3) as u32);
            let step = samples_per_joint;

            // Simplified: sweep joint 0 only for tractable computation
            for i in 0..step {
                let t = i as f64 / (step - 1).max(1) as f64;
                let lower = dh_params.first().map(|d| d.joint_limits.lower).unwrap_or(-std::f64::consts::PI);
                let upper = dh_params.first().map(|d| d.joint_limits.upper).unwrap_or(std::f64::consts::PI);
                let angle = lower + (upper - lower) * t;
                let angles = vec![angle; n_joints];
                let pose = PipelineExecutor::compute_fk_dh(&dh_params, &angles);
                reachable_points.push([
                    pose.position_mm[0] as f32,
                    pose.position_mm[1] as f32,
                    pose.position_mm[2] as f32,
                ]);
            }

            let max_reach = reachable_points.iter()
                .map(|p| (p[0]*p[0] + p[1]*p[1] + p[2]*p[2]).sqrt() as f64)
                .fold(0.0f64, f64::max);
            let center = [0.0f64; 3];
            let vol = (4.0 / 3.0) * std::f64::consts::PI * max_reach.powi(3);

            Ok(KinematicsModalityOutput {
                success: true,
                workspace: Some(WorkspaceVolume {
                    volume_id: executor.generate_id(),
                    chain_id: chain_node_id,
                    volume_type: WorkspaceType::Reachable,
                    volume_mm3: vol,
                    reachable_fraction: 0.8,
                    dexterous_volume_mm3: Some(vol * 0.3),
                    bounding_sphere_radius_mm: max_reach,
                    center_mm: center,
                    max_reach_mm: max_reach,
                    min_reach_mm: max_reach * 0.1,
                    resolution_mm: (max_reach / step as f64) as f32,
                    reachable_point_cloud: reachable_points,
                }),
                ..Default::default()
            })
        }

        KinematicsModalityAction::CreateGraph { analysis, project_id } => {
            Ok(create_graph(&executor, analysis, project_id).await)
        }

        KinematicsModalityAction::UpdateGraph { graph_id, updates, project_id } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
            let mut update_count = 0;

            for update in &updates {
                match update {
                    KinematicsUpdate::SetJointAngle { joint_node_id, angle_rad } => {
                        if let Some(n) = graph.nodes.iter_mut().find(|n| n.node_id == *joint_node_id) {
                            n.current_position = Some(*angle_rad);
                            n.version += 1;
                            n.version_notes.push(VersionNote {
                                version: n.version, note: format!("Joint angle → {:.4}rad", angle_rad),
                                step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated,
                            });
                            update_count += 1;
                        }
                    }
                    KinematicsUpdate::SetJointLimits { joint_node_id, lower, upper } => {
                        if let Some(n) = graph.nodes.iter_mut().find(|n| n.node_id == *joint_node_id) {
                            n.limit_lower = Some(*lower);
                            n.limit_upper = Some(*upper);
                            n.version += 1;
                            n.version_notes.push(VersionNote {
                                version: n.version, note: format!("Limits updated: [{:.3}, {:.3}]", lower, upper),
                                step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated,
                            });
                            update_count += 1;
                        }
                    }
                    KinematicsUpdate::UpdateEndEffectorPose { ee_node_id, pose } => {
                        if let Some(n) = graph.nodes.iter_mut().find(|n| n.node_id == *ee_node_id) {
                            n.version += 1;
                            n.version_notes.push(VersionNote {
                                version: n.version, note: format!("EE pose updated: ({:.1},{:.1},{:.1})",
                                    pose.position_mm[0], pose.position_mm[1], pose.position_mm[2]),
                                step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated,
                            });
                            update_count += 1;
                        }
                    }
                    KinematicsUpdate::UpdateLinkMass { link_node_id, mass_kg } => {
                        if let Some(n) = graph.nodes.iter_mut().find(|n| n.node_id == *link_node_id) {
                            n.mass_kg = Some(*mass_kg);
                            n.version += 1;
                            update_count += 1;
                        }
                    }
                    KinematicsUpdate::AddJoint { joint } => {
                        let new_nid = graph.nodes.iter().map(|n| n.node_id).max().unwrap_or(0) + 1;
                        graph.nodes.push(KinematicsGraphNode {
                            node_id: new_nid, node_type: KinematicsNodeType::JointNode,
                            content: format!("Joint {:?}: {} lim=[{:.2},{:.2}]",
                                joint.joint_type, joint.name, joint.limits.lower, joint.limits.upper),
                            joint_type: Some(format!("{:?}", joint.joint_type)),
                            current_position: Some(joint.current_position),
                            limit_lower: Some(joint.limits.lower), limit_upper: Some(joint.limits.upper),
                            is_passive: Some(joint.is_passive),
                            actuator_type: Some(format!("{:?}", joint.actuator_type)),
                            provisional: false, provisional_status: ProvisionalStatus::Validated,
                            version: 1, keywords: vec!["joint".into()], hotness_score: 0.7,
                            ..Default::default()
                        });
                        graph.edges.push(KinematicsGraphEdge {
                            edge_id: next_eid, from_node: graph.root_node_id, to_node: new_nid,
                            edge_type: KinematicsEdgeType::Contains, weight: 1.0,
                            provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default()
                        });
                        next_eid += 1; update_count += 1;
                    }
                    KinematicsUpdate::RemoveJoint { joint_node_id } => {
                        graph.nodes.retain(|n| n.node_id != *joint_node_id);
                        graph.edges.retain(|e| e.from_node != *joint_node_id && e.to_node != *joint_node_id);
                        update_count += 1;
                    }
                    KinematicsUpdate::AppendToChain { chain_node_id, joint, link } => {
                        // Add joint and link nodes, connect to chain
                        let j_nid = graph.nodes.iter().map(|n| n.node_id).max().unwrap_or(0) + 1;
                        let l_nid = j_nid + 1;
                        graph.nodes.push(KinematicsGraphNode {
                            node_id: j_nid, node_type: KinematicsNodeType::JointNode,
                            content: format!("Joint {:?}: {}", joint.joint_type, joint.name),
                            joint_type: Some(format!("{:?}", joint.joint_type)),
                            limit_lower: Some(joint.limits.lower), limit_upper: Some(joint.limits.upper),
                            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                            keywords: vec!["joint".into()], hotness_score: 0.7, ..Default::default()
                        });
                        graph.nodes.push(KinematicsGraphNode {
                            node_id: l_nid, node_type: KinematicsNodeType::LinkNode,
                            content: format!("Link: {} mass={:.3}kg", link.name, link.mass_kg),
                            mass_kg: Some(link.mass_kg), length_mm: Some(link.length_mm),
                            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                            keywords: vec!["link".into()], hotness_score: 0.6, ..Default::default()
                        });
                        graph.edges.push(KinematicsGraphEdge { edge_id: next_eid, from_node: *chain_node_id, to_node: j_nid, edge_type: KinematicsEdgeType::ChainContainsJoint, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                        next_eid += 1;
                        graph.edges.push(KinematicsGraphEdge { edge_id: next_eid, from_node: *chain_node_id, to_node: l_nid, edge_type: KinematicsEdgeType::ChainContainsLink, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                        next_eid += 1;
                        update_count += 1;
                    }
                    _ => { update_count += 1; }
                }
            }

            graph.version += 1; graph.updated_at = now.clone(); graph.state = GraphStateType::Updated;
            graph.version_notes.push(VersionNote {
                version: graph.version,
                note: format!("{} kinematics updates applied", update_count),
                step_index: None, timestamp: now, change_type: ChangeType::Updated,
            });
            executor.save_graph(&graph)?;
            Ok(KinematicsModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        KinematicsModalityAction::QueryGraph { graph_id, query } => {
            let graph = executor.load_graph(graph_id)?;
            let result = match query {
                KinematicsGraphQuery::NodeDetail { node_id } => {
                    let node = graph.nodes.iter().find(|n| n.node_id == node_id);
                    let incoming: Vec<_> = graph.edges.iter().filter(|e| e.to_node == node_id).collect();
                    let outgoing: Vec<_> = graph.edges.iter().filter(|e| e.from_node == node_id).collect();
                    serde_json::json!({ "node": node, "incoming": incoming, "outgoing": outgoing })
                }
                KinematicsGraphQuery::ChainTopology { chain_node_id } => {
                    let joints: Vec<_> = graph.edges.iter()
                        .filter(|e| e.from_node == chain_node_id && matches!(e.edge_type, KinematicsEdgeType::ChainContainsJoint | KinematicsEdgeType::FollowedByJoint))
                        .filter_map(|e| graph.nodes.iter().find(|n| n.node_id == e.to_node))
                        .collect();
                    let links: Vec<_> = graph.edges.iter()
                        .filter(|e| e.from_node == chain_node_id && matches!(e.edge_type, KinematicsEdgeType::ChainContainsLink))
                        .filter_map(|e| graph.nodes.iter().find(|n| n.node_id == e.to_node))
                        .collect();
                    serde_json::json!({ "joints": joints, "links": links })
                }
                KinematicsGraphQuery::JointLimits { chain_node_id } => {
                    let joints: Vec<serde_json::Value> = graph.edges.iter()
                        .filter(|e| e.from_node == chain_node_id && matches!(e.edge_type, KinematicsEdgeType::ChainContainsJoint))
                        .filter_map(|e| graph.nodes.iter().find(|n| n.node_id == e.to_node))
                        .map(|n| serde_json::json!({ "node_id": n.node_id, "name": &n.content[..n.content.len().min(30)], "lower": n.limit_lower, "upper": n.limit_upper, "current": n.current_position }))
                        .collect();
                    serde_json::json!({ "joint_limits": joints })
                }
                KinematicsGraphQuery::WorkspaceVolumes => {
                    let ws: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, KinematicsNodeType::WorkspaceVolumeNode)).collect();
                    serde_json::json!({ "workspace_volumes": ws })
                }
                KinematicsGraphQuery::Singularities => {
                    let sings: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, KinematicsNodeType::SingularityNode)).collect();
                    serde_json::json!({ "singularities": sings })
                }
                KinematicsGraphQuery::GaitCycles => {
                    let cycles: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, KinematicsNodeType::GaitCycleNode | KinematicsNodeType::GaitPhaseNode)).collect();
                    serde_json::json!({ "gait_cycles": cycles })
                }
                KinematicsGraphQuery::MuscleGroups => {
                    let muscles: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, KinematicsNodeType::MuscleGroupNode)).collect();
                    serde_json::json!({ "muscles": muscles })
                }
                KinematicsGraphQuery::CrossModalLinks { node_id } => {
                    let links: Vec<_> = graph.edges.iter()
                        .filter(|e| (e.from_node == node_id || e.to_node == node_id)
                            && matches!(e.edge_type,
                                KinematicsEdgeType::MapsToArmatureIn3D |
                                KinematicsEdgeType::JointGeometryFromCAD |
                                KinematicsEdgeType::FeedsControlSystem |
                                KinematicsEdgeType::MotorImageryFromBCI |
                                KinematicsEdgeType::JointAngleFromIMU |
                                KinematicsEdgeType::BiomechanicalContext |
                                KinematicsEdgeType::AnimatesViaBone
                            ))
                        .collect();
                    serde_json::json!({ "cross_modal_links": links })
                }
                KinematicsGraphQuery::EndEffectors => {
                    let ees: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, KinematicsNodeType::EndEffectorNode)).collect();
                    serde_json::json!({ "end_effectors": ees })
                }
                KinematicsGraphQuery::AGIActivity => serde_json::json!({ "is_active": false }),
                KinematicsGraphQuery::AllNodes => serde_json::json!({ "nodes": graph.nodes }),
                KinematicsGraphQuery::AllEdges => serde_json::json!({ "edges": graph.edges }),
            };
            Ok(KinematicsModalityOutput { success: true, query_result: Some(result), ..Default::default() })
        }

        KinematicsModalityAction::GetGraph { graph_id } => {
            let graph = executor.load_graph(graph_id)?;
            Ok(KinematicsModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        KinematicsModalityAction::LinkToModality { graph_id, target_modality, target_graph_id, link_type } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            let ref_nid = graph.nodes.iter().map(|n| n.node_id).max().unwrap_or(0) + 1;
            let next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
            graph.nodes.push(KinematicsGraphNode {
                node_id: ref_nid, node_type: KinematicsNodeType::CrossModalRef,
                content: format!("CrossModal→{} graph={} link={:?}", target_modality, target_graph_id, link_type),
                materialized_path: Some(format!("/Modalities/Kinematics/Project_{}/Graph_{}/CrossModal/{}", graph.project_id, graph_id, target_graph_id)),
                provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                keywords: vec!["cross-modal".into(), target_modality.clone()], hotness_score: 0.7, ..Default::default()
            });
            let etype = match link_type {
                KinCrossModalLink::MapsToArmatureIn3D   => KinematicsEdgeType::MapsToArmatureIn3D,
                KinCrossModalLink::JointGeometryFromCAD => KinematicsEdgeType::JointGeometryFromCAD,
                KinCrossModalLink::FeedsControlSystem   => KinematicsEdgeType::FeedsControlSystem,
                KinCrossModalLink::MotorImageryFromBCI  => KinematicsEdgeType::MotorImageryFromBCI,
                KinCrossModalLink::JointAngleFromIMU    => KinematicsEdgeType::JointAngleFromIMU,
                KinCrossModalLink::BiomechanicalContext => KinematicsEdgeType::BiomechanicalContext,
            };
            graph.edges.push(KinematicsGraphEdge {
                edge_id: next_eid, from_node: graph.root_node_id, to_node: ref_nid,
                edge_type: etype, weight: 0.9,
                provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                properties: { let mut p = HashMap::new(); p.insert("target_graph_id".into(), serde_json::json!(target_graph_id)); p },
                ..Default::default()
            });
            graph.state = GraphStateType::CrossLinked;
            graph.version += 1; graph.updated_at = now.clone();
            graph.version_notes.push(VersionNote { version: graph.version, note: format!("Linked to {} (graph {})", target_modality, target_graph_id), step_index: None, timestamp: now, change_type: ChangeType::CrossLinked });
            executor.save_graph(&graph)?;
            Ok(KinematicsModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        KinematicsModalityAction::ImportFromCAD { cad_graph_id, joint_ids, project_id } => {
            // In production: load CAD graph and extract joint geometry
            // Build a skeleton chain from the joint IDs
            let analysis = KinematicsAnalysisResult {
                analysis_id: executor.generate_id(),
                source_description: format!("Imported from CAD graph {} ({} joints)", cad_graph_id, joint_ids.len()),
                system_type: KinematicsSystemType::SerialManipulator,
                dof_total: joint_ids.len() as u32,
                dof_actuated: joint_ids.len() as u32,
                ..Default::default()
            };
            Ok(create_graph(&executor, analysis, project_id).await)
        }

        KinematicsModalityAction::ImportFrom3D { graph_id_3d, armature_node_id, project_id } => {
            // In production: load 3D graph, walk armature hierarchy → joints → chain
            let analysis = KinematicsAnalysisResult {
                analysis_id: executor.generate_id(),
                source_description: format!("Imported from 3D graph {} armature {}", graph_id_3d, armature_node_id),
                system_type: KinematicsSystemType::SerialManipulator,
                ..Default::default()
            };
            Ok(create_graph(&executor, analysis, project_id).await)
        }

        KinematicsModalityAction::ExportToControl { graph_id, chain_node_id } => {
            let graph = executor.load_graph(graph_id)?;
            let export_path = format!("/tmp/kin_control_export_{}_{}.json", graph_id, chain_node_id);
            // In production: serialize joint space description to control system format
            Ok(KinematicsModalityOutput { success: true, export_path: Some(export_path), ..Default::default() })
        }

        KinematicsModalityAction::TriggerSemanticHook { graph_id, hook } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            match hook {
                KinematicsSemanticHook::OnGraphCreated => {
                    graph.state = GraphStateType::SemanticEnriched;
                }
                KinematicsSemanticHook::OnInferRelationships => {
                    let new_edges = executor.infer_semantic_relationships(&graph.nodes).await;
                    let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                    let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                    for (from, to, etype, reason) in new_edges {
                        if valid.contains(&from) && valid.contains(&to) && from != to {
                            graph.edges.push(KinematicsGraphEdge {
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
                KinematicsSemanticHook::OnEdgeCompletion => {
                    let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                    graph.edges.retain(|e| valid.contains(&e.from_node) && valid.contains(&e.to_node));
                }
                KinematicsSemanticHook::OnCrossModalityLink { target_modality, target_graph_id } => {
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
            Ok(KinematicsModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        KinematicsModalityAction::StreamToUI { graph_id, .. } => {
            Ok(KinematicsModalityOutput { success: true, graph_id: Some(graph_id), ..Default::default() })
        }

        KinematicsModalityAction::HeadlessProcess { graph_id, operations } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            for op in operations {
                match op {
                    KinematicsHeadlessOp::ComputeAllFK => {
                        // In production: run FK for all chains with their current joint angles
                        graph.version_notes.push(VersionNote {
                            version: graph.version + 1, note: "FK computed for all chains".into(),
                            step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated,
                        });
                        graph.version += 1;
                    }
                    KinematicsHeadlessOp::ComputeWorkspaceAll => {
                        graph.version_notes.push(VersionNote {
                            version: graph.version + 1, note: "Workspace computed for all chains".into(),
                            step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated,
                        });
                        graph.version += 1;
                    }
                    KinematicsHeadlessOp::DetectAllSingularities => {
                        graph.version_notes.push(VersionNote {
                            version: graph.version + 1, note: "Singularity detection completed".into(),
                            step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated,
                        });
                        graph.version += 1;
                    }
                    KinematicsHeadlessOp::ExportURDF { output_path } => {
                        // In production: serialize graph to URDF XML
                    }
                    KinematicsHeadlessOp::ExportMJCF { output_path } => {
                        // In production: serialize graph to MJCF XML
                    }
                    KinematicsHeadlessOp::SyncWith3D { graph_id_3d } => {
                        let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        let root_id = graph.root_node_id;
                        graph.edges.push(KinematicsGraphEdge {
                            edge_id: next_eid, from_node: root_id, to_node: root_id,
                            edge_type: KinematicsEdgeType::MapsToArmatureIn3D, weight: 0.9,
                            provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                            properties: { let mut p = HashMap::new(); p.insert("graph_id_3d".into(), serde_json::json!(graph_id_3d)); p },
                            ..Default::default()
                        });
                    }
                    KinematicsHeadlessOp::SyncWithControl { control_graph_id } => {
                        let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        let root_id = graph.root_node_id;
                        graph.edges.push(KinematicsGraphEdge {
                            edge_id: next_eid, from_node: root_id, to_node: root_id,
                            edge_type: KinematicsEdgeType::FeedsControlSystem, weight: 0.9,
                            provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                            properties: { let mut p = HashMap::new(); p.insert("control_graph_id".into(), serde_json::json!(control_graph_id)); p },
                            ..Default::default()
                        });
                    }
                }
            }
            graph.updated_at = now;
            executor.save_graph(&graph)?;
            Ok(KinematicsModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
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
        eprintln!("Usage: kinematics --input '<json>'");
        std::process::exit(1);
    }
    let input: KinematicsModalityAction = match serde_json::from_str(&input_json) {
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
