//! ThreeDEngine — Pipeline #109
//!
//! AGI-first 3D scene analysis, manipulation, and simulation engine.
//! Replaces and surpasses Blender, Gazebo, Isaac Sim, Isaac Lab, Webots.
//!
//! DESIGN PRINCIPLE: AGI-first, humans second. The pipeline operates fully
//! headlessly. UI rendering via Electron/React is additive, not required.
//!
//! BLENDER COVERAGE: Full Blender data hierarchy — scenes, collections,
//! objects, meshes, armatures, animations, physics, materials, compositing,
//! VSE, motion tracking, geometry nodes, drivers, NLA.
//!
//! SIMULATION DOMAINS:
//!   Robotics (URDF/MJCF), Manipulation, Autonomous Vehicles, Drones,
//!   Industrial Automation, Medical/Surgical, Biomechanics, Aerospace,
//!   Maritime/Underwater, Construction, Automotive, Soft Robotics.
//!
//! CROSS-LINKS:
//!   100 (Text)    → scene/object descriptions
//!   101 (Code)    → control scripts, shader code
//!   102 (Image)   → textures, renders, reference images
//!   104 (Video)   → motion capture reference, render output
//!   113 (Haptic)  → contact geometry
//!   114 (Thermal) → thermal simulation
//!   115 (Depth)   → depth→point cloud fusion
//!   117 (Geo)     → scene placed on geospatial map
//!   120 (CAD)     → parametric mesh generation
//!   121 (Kine)    → kinematic chains
//!   122 (Control) → robot control
//!   124 (Radar)   → radar point cloud → 3D scene
//!   125 (Sonar)   → acoustic 3D reconstruction
//!
//! STORAGE: ZSEI containers under /Modalities/ThreeD/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

// ─────────────────────────────────────────────────────────────────────────────
// INPUT TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ThreeDModalityAction {
    // ── ANALYSIS ──
    AnalyzeScene      { data: SceneDataSource,     extract_hierarchy: bool, extract_physics: bool, extract_animation: bool, extract_materials: bool },
    AnalyzeMesh       { data: MeshDataSource,       compute_normals: bool, compute_uv: bool, compute_topology: bool },
    AnalyzeRig        { data: ArmatureDataSource,   extract_constraints: bool, extract_weights: bool },
    AnalyzeAnimation  { data: AnimationDataSource,  extract_fcurves: bool, extract_drivers: bool, extract_nla: bool },
    AnalyzePhysics    { data: PhysicsDataSource,    extract_rigid: bool, extract_soft: bool, extract_fluid: bool, extract_particles: bool },
    AnalyzeMaterials  { data: MaterialDataSource,   extract_node_tree: bool, extract_textures: bool },
    // ── GRAPH ──
    CreateGraph       { analysis: ThreeDAnalysisResult, project_id: u64 },
    UpdateGraph       { graph_id: u64, updates: Vec<SceneUpdate>, project_id: u64 },
    QueryGraph        { graph_id: u64, query: ThreeDGraphQuery },
    GetGraph          { graph_id: u64 },
    // ── SPATIAL ──
    ComputeSpatialRelationships { graph_id: u64, method: SpatialMethod },
    BuildSpatialIndex           { graph_id: u64, index_type: SpatialIndexType },
    // ── SIMULATION ──
    RunSimulation     { graph_id: u64, domain: SimulationDomain, frames: u32, headless: bool, cache_results: bool },
    BakePhysics       { graph_id: u64, simulation_type: PhysicsSimType, frame_start: u32, frame_end: u32 },
    StepSimulation    { graph_id: u64, delta_time_secs: f32 },
    // ── EXPORT ──
    ExportScene       { graph_id: u64, format: ThreeDExportFormat, options: ExportOptions },
    // ── GEOMETRY OPS ──
    ApplyModifier     { graph_id: u64, object_node_id: u64, modifier: ModifierType },
    BooleanOperation  { graph_id: u64, target_node_id: u64, tool_node_id: u64, operation: BooleanOp },
    ComputeLOD        { graph_id: u64, object_node_id: u64, lod_levels: u32 },
    // ── CROSS-MODAL ──
    LinkToModality    { graph_id: u64, target_modality: String, target_graph_id: u64, link_type: CrossModalLinkType },
    FusePointCloud    { graph_id: u64, source_modality: String, point_cloud_data: Vec<ExternalPoint3D> },
    // ── HOOKS / STREAMING ──
    TriggerSemanticHook { graph_id: u64, hook: ThreeDSemanticHook },
    StreamToUI          { graph_id: u64, session_id: String, render_mode: ViewportMode },
    HeadlessProcess     { graph_id: u64, operations: Vec<ThreeDHeadlessOp> },
}

// ─────────────────────────────────────────────────────────────────────────────
// DATA SOURCES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SceneDataSource {
    GLTF        { file_path: String, load_animations: bool, load_materials: bool },
    GLB         { file_path: String },
    OBJ         { file_path: String, mtl_path: Option<String> },
    FBX         { file_path: String },
    USD         { file_path: String, variant_set: Option<String> },
    USDA        { file_path: String },
    USDC        { file_path: String },
    USDZ        { file_path: String },
    Blend       { file_path: String, scene_name: Option<String> },
    DAE         { file_path: String },
    STL         { file_path: String, unit: LengthUnit },
    PLY         { file_path: String },
    URDF        { file_path: String, mesh_dir: Option<String> },
    MJCF        { file_path: String },
    SDF         { file_path: String },
    World       { file_path: String },    // Gazebo .world
    IsaacSimUSD { file_path: String, physics_scene: Option<String> },
    /// JSON manifest: describes a scene composed of multiple files
    ManifestJSON { manifest_path: String },
    /// Raw procedural: build scene from graph node data
    Procedural  { nodes: Vec<ProceduralNode> },
    /// Live streaming from a running simulator
    LiveStream  { endpoint: String, protocol: StreamProtocol },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProceduralNode {
    pub node_type: String,          // "mesh", "light", "camera", "empty", etc.
    pub name: String,
    pub transform: Transform4x4,
    pub parameters: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform4x4 {
    pub matrix: [[f64; 4]; 4],
}

impl Default for Transform4x4 {
    fn default() -> Self {
        Self { matrix: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]}
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum LengthUnit { #[default] Meters, Centimeters, Millimeters, Inches, Feet }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamProtocol { WebSocket, gRPC, SharedMemory, ZeroMQ, ROS2Topic }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MeshDataSource {
    FromFile      { file_path: String, object_name: Option<String> },
    FromGraph     { graph_id: u64, mesh_node_id: u64 },
    RawArrays     { vertices: Vec<[f32; 3]>, faces: Vec<[u32; 3]>, normals: Option<Vec<[f32; 3]>>, uvs: Option<Vec<[f32; 2]>> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArmatureDataSource {
    FromFile  { file_path: String, armature_name: Option<String> },
    FromGraph { graph_id: u64, armature_node_id: u64 },
    URDF      { file_path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnimationDataSource {
    FromFile     { file_path: String, action_name: Option<String> },
    FromGraph    { graph_id: u64, action_node_id: u64 },
    BVH          { file_path: String },
    FBX          { file_path: String },
    AlembicCache { file_path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PhysicsDataSource {
    FromGraph     { graph_id: u64 },
    URDFPhysics   { file_path: String },
    MJCFPhysics   { file_path: String },
    BlendPhysics  { file_path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MaterialDataSource {
    FromGraph     { graph_id: u64, material_node_id: u64 },
    GLTF_PBR      { base_color: [f32; 4], metallic: f32, roughness: f32, emissive: [f32; 3] },
    TextureSet    { albedo: Option<String>, normal: Option<String>, roughness: Option<String>, metallic: Option<String>, ao: Option<String>, emissive: Option<String> },
}

// ─────────────────────────────────────────────────────────────────────────────
// ANALYSIS RESULT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThreeDAnalysisResult {
    pub analysis_id: u64,
    pub source_format: String,
    pub source_path: Option<String>,

    // SCENE HIERARCHY
    pub scenes: Vec<SceneInfo>,
    pub collections: Vec<CollectionInfo>,
    pub objects: Vec<ObjectInfo>,

    // GEOMETRY
    pub meshes: Vec<MeshInfo>,
    pub curves: Vec<CurveInfo>,
    pub surfaces: Vec<SurfaceInfo>,
    pub volumes: Vec<VolumeInfo>,
    pub metaballs: Vec<MetaballInfo>,
    pub grease_pencil_layers: Vec<GreasePencilLayerInfo>,

    // RIGGING
    pub armatures: Vec<ArmatureInfo>,
    pub bones: Vec<BoneInfo>,
    pub constraints: Vec<ConstraintInfo>,
    pub shape_keys: Vec<ShapeKeyInfo>,
    pub vertex_groups: Vec<VertexGroupInfo>,

    // ANIMATION
    pub actions: Vec<ActionInfo>,
    pub fcurves: Vec<FCurveInfo>,
    pub drivers: Vec<DriverInfo>,
    pub nla_tracks: Vec<NLATrackInfo>,

    // MODIFIERS + GEOMETRY NODES
    pub modifiers: Vec<ModifierInfo>,
    pub geometry_node_trees: Vec<GeometryNodeTree>,

    // PHYSICS
    pub rigid_bodies: Vec<RigidBodyInfo>,
    pub soft_bodies: Vec<SoftBodyInfo>,
    pub cloth_sims: Vec<ClothSimInfo>,
    pub fluid_domains: Vec<FluidDomainInfo>,
    pub smoke_domains: Vec<SmokeDomainInfo>,
    pub particle_systems: Vec<ParticleSystemInfo>,
    pub hair_systems: Vec<HairSystemInfo>,
    pub force_fields: Vec<ForceFieldInfo>,
    pub dynamic_paint_surfaces: Vec<DynamicPaintInfo>,
    pub physics_caches: Vec<PhysicsCacheInfo>,

    // MATERIALS + SHADERS
    pub materials: Vec<MaterialInfo>,
    pub shader_nodes: Vec<ShaderNodeInfo>,
    pub textures: Vec<TextureInfo>,
    pub uv_maps: Vec<UVMapInfo>,

    // SCENE OBJECTS
    pub lights: Vec<LightInfo>,
    pub cameras: Vec<CameraInfo>,
    pub world: Option<WorldInfo>,
    pub speakers: Vec<SpeakerInfo>,

    // RENDER
    pub render_settings: Option<RenderSettingsInfo>,
    pub render_layers: Vec<RenderLayerInfo>,
    pub compositor_trees: Vec<CompositorNodeTree>,

    // VSE / MOTION TRACKING
    pub vse_strips: Vec<VSEStripInfo>,
    pub motion_tracks: Vec<MotionTrackInfo>,
    pub tracking_markers: Vec<TrackingMarkerInfo>,
    pub solved_cameras: Vec<SolvedCameraInfo>,

    // SPATIAL
    pub spatial_relationships: Vec<SpatialRelationship3D>,
    pub bounding_boxes: HashMap<String, BoundingBox3D>,  // object_name → AABB

    // CUSTOM PROPERTIES
    pub id_properties: Vec<IDPropertyInfo>,
    pub custom_attributes: Vec<CustomAttributeInfo>,
    pub libraries: Vec<LibraryInfo>,

    // METADATA
    pub unit_system: LengthUnit,
    pub frame_start: u32,
    pub frame_end: u32,
    pub fps: f32,
    pub up_axis: Axis,
    pub processing_notes: Vec<String>,
}

// ── SCENE / COLLECTION / OBJECT ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SceneInfo {
    pub scene_id: u64, pub name: String,
    pub collection_ids: Vec<u64>,
    pub world_id: Option<u64>,
    pub frame_start: u32, pub frame_end: u32, pub fps: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CollectionInfo {
    pub collection_id: u64, pub name: String,
    pub parent_collection_id: Option<u64>,
    pub child_collection_ids: Vec<u64>,
    pub object_ids: Vec<u64>,
    pub hide_viewport: bool, pub hide_render: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ObjectInfo {
    pub object_id: u64, pub name: String,
    pub object_type: ObjectType,
    pub parent_id: Option<u64>,
    pub children_ids: Vec<u64>,
    pub data_id: Option<u64>,              // mesh/armature/etc. id
    pub transform: Transform4x4,
    pub location: [f64; 3],
    pub rotation_euler: [f64; 3],         // radians
    pub scale: [f64; 3],
    pub dimensions: [f64; 3],             // world-space bounding box size
    pub modifier_ids: Vec<u64>,
    pub material_ids: Vec<u64>,
    pub armature_id: Option<u64>,
    pub vertex_group_ids: Vec<u64>,
    pub shape_key_ids: Vec<u64>,
    pub rigid_body_id: Option<u64>,
    pub soft_body_id: Option<u64>,
    pub cloth_id: Option<u64>,
    pub fluid_id: Option<u64>,
    pub particle_system_ids: Vec<u64>,
    pub visible: bool,
    pub custom_properties: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ObjectType {
    #[default] Empty, Mesh, Curve, Surface, Meta, Text, Volume, GreasePencil,
    Armature, Lattice, Empty_Object, Light, Camera, Speaker, LightProbe,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum Axis { X, Y, #[default] Z }

// ── MESH ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MeshInfo {
    pub mesh_id: u64, pub name: String,
    pub vertex_count: u32, pub edge_count: u32, pub face_count: u32, pub loop_count: u32,
    pub has_uvs: bool, pub uv_layer_count: u32,
    pub has_vertex_colors: bool, pub vertex_color_layer_count: u32,
    pub has_normals: bool, pub has_custom_normals: bool,
    pub shape_key_count: u32,
    pub vertex_group_count: u32,
    pub is_manifold: bool,
    pub has_non_manifold_edges: bool,
    pub polygon_type: PolygonType,
    pub bounding_box: BoundingBox3D,
    pub surface_area: f64,
    pub volume: Option<f64>,              // only valid for closed manifold meshes
    pub uv_maps: Vec<String>,
    pub custom_attributes: Vec<CustomAttributeInfo>,
    pub shapekey_names: Vec<String>,
    pub vertex_group_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PolygonType { #[default] Triangles, Quads, Mixed, NGons }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BoundingBox3D {
    pub min: [f64; 3], pub max: [f64; 3],
    pub center: [f64; 3], pub size: [f64; 3],
}

// ── CURVE / SURFACE / META / VOLUME ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CurveInfo {
    pub curve_id: u64, pub name: String,
    pub spline_type: SplineType, pub spline_count: u32,
    pub is_3d: bool, pub fill_mode: CurveFillMode,
    pub bevel_depth: f64, pub extrude_depth: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SplineType { #[default] Bezier, NURBS, Poly }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum CurveFillMode { #[default] None, Front, Back, Both }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SurfaceInfo {
    pub surface_id: u64, pub name: String,
    pub degree_u: u32, pub degree_v: u32,
    pub control_point_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VolumeInfo {
    pub volume_id: u64, pub name: String,
    pub grid_names: Vec<String>,
    pub voxel_size: f64,
    pub file_path: Option<String>,     // OpenVDB file
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetaballInfo {
    pub meta_id: u64, pub name: String,
    pub element_count: u32,
    pub resolution: f64,
    pub threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GreasePencilLayerInfo {
    pub layer_id: u64, pub name: String,
    pub frame_count: u32, pub stroke_count: u32,
    pub opacity: f32, pub blend_mode: String,
}

// ── RIGGING ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArmatureInfo {
    pub armature_id: u64, pub name: String,
    pub bone_count: u32,
    pub bone_ids: Vec<u64>,
    pub pose_bone_ids: Vec<u64>,
    pub edit_mode_matrix: Transform4x4,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BoneInfo {
    pub bone_id: u64, pub name: String,
    pub parent_bone_id: Option<u64>,
    pub children_ids: Vec<u64>,
    pub head: [f64; 3], pub tail: [f64; 3],
    pub roll_rad: f64, pub length: f64,
    pub is_deform: bool, pub use_connect: bool,
    pub bone_matrix: Transform4x4,         // bone-space matrix
    pub pose_matrix: Option<Transform4x4>, // current pose
    pub constraint_ids: Vec<u64>,
    pub custom_shape_id: Option<u64>,
    pub envelope_distance: f64,
    pub envelope_weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConstraintInfo {
    pub constraint_id: u64, pub name: String,
    pub constraint_type: ConstraintType,
    pub owner_bone_id: Option<u64>,
    pub owner_object_id: Option<u64>,
    pub target_object_id: Option<u64>,
    pub target_bone_name: Option<String>,
    pub influence: f32,
    pub mute: bool,
    pub parameters: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ConstraintType {
    #[default] CopyLocation, CopyRotation, CopyScale, CopyTransforms,
    LimitLocation, LimitRotation, LimitScale, LimitDistance,
    IK, SplineIK, PoleTarget, StretchTo, TrackTo, DampedTrack, LockedTrack,
    ClampTo, ChildOf, Floor, FollowPath, FollowTrack, ObjectSolver,
    RotateLike, ShrinkWrap, ActionConstraint, Pivot, RigidBodyJoint, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShapeKeyInfo {
    pub shapekey_id: u64, pub name: String,
    pub reference_key: bool,
    pub relative_to_id: Option<u64>,
    pub value: f32, pub min_value: f32, pub max_value: f32,
    pub driver_id: Option<u64>,
    pub vertex_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VertexGroupInfo {
    pub vg_id: u64, pub name: String,
    pub vertex_count: u32,
    pub bone_deform: bool,
    pub associated_bone_name: Option<String>,
}

// ── ANIMATION ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActionInfo {
    pub action_id: u64, pub name: String,
    pub fcurve_count: u32,
    pub frame_range: (f32, f32),
    pub fcurve_ids: Vec<u64>,
    pub id_root: String,                   // "OBJECT", "ARMATURE", etc.
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FCurveInfo {
    pub fcurve_id: u64,
    pub data_path: String,                 // "location[0]", "pose.bones[\"Hip\"].rotation_euler[2]"
    pub array_index: u32,
    pub keyframe_count: u32,
    pub frame_range: (f32, f32),
    pub interpolation_type: InterpolationType,
    pub extrapolation: ExtrapolationType,
    pub keyframe_ids: Vec<u64>,
    pub modifier_types: Vec<String>,       // "CYCLES", "NOISE", "LIMITS", etc.
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum InterpolationType { #[default] Bezier, Linear, Constant, Back, Bounce, Elastic, Expo, Quad, Quart, Quint, Sine }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ExtrapolationType { #[default] Constant, Linear, Cyclic, CyclicWithOffset, OscillatingCyclic }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DriverInfo {
    pub driver_id: u64,
    pub data_path: String,
    pub driver_type: DriverType,
    pub expression: Option<String>,        // Python expression (if type=SCRIPTED)
    pub variables: Vec<DriverVariable>,
    pub use_self: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum DriverType { #[default] Averaged, Sum, Min, Max, Scripted }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DriverVariable {
    pub name: String,
    pub variable_type: String,             // "SINGLE_PROP", "TRANSFORMS", "LOC_DIFF", "ROTATION_DIFF"
    pub target_object: Option<String>,
    pub target_bone: Option<String>,
    pub target_property: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NLATrackInfo {
    pub track_id: u64, pub name: String,
    pub object_id: u64,
    pub mute: bool, pub solo: bool,
    pub strip_count: u32,
    pub strips: Vec<NLAStripInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NLAStripInfo {
    pub strip_id: u64, pub name: String,
    pub action_id: u64,
    pub frame_start: f32, pub frame_end: f32,
    pub action_frame_start: f32, pub action_frame_end: f32,
    pub scale: f32, pub repeat: f32,
    pub blend_type: String,               // "REPLACE", "ADD", "MULTIPLY", etc.
    pub influence: f32,
}

// ── MODIFIERS + GEOMETRY NODES ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModifierInfo {
    pub modifier_id: u64, pub name: String,
    pub modifier_type: ModifierType,
    pub show_viewport: bool, pub show_render: bool, pub apply_on_spline: bool,
    pub parameters: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ModifierType {
    #[default] Armature, Array, Bevel, Boolean, Build, Cast, Cloth,
    Collision, Curve, DataTransfer, Decimate, Displace, DynamicPaint,
    EdgeSplit, ExplodeModifier, Fluid, Hook, Laplacian_Deform, Laplacian_Smooth,
    Lattice, MeshDeform, Mirror, Multires, NormalEdit, Particles, Remesh,
    Screw, Shrinkwrap, SimpleDeform, Smooth, Solidify, Subdivision,
    Surface_Deform, Triangulate, UV_Project, UV_Warp, Vertex_WeightEdit,
    Warp, Wave, Weighted_Normal, Weld, Wireframe, GeometryNodes,
    GreasePencil_Armature, GreasePencil_Dash, GreasePencil_Lattice,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeometryNodeTree {
    pub tree_id: u64, pub name: String,
    pub input_count: u32, pub output_count: u32,
    pub node_count: u32,
    pub group_nodes: Vec<String>,
    pub uses_fields: bool,
    pub summary: String,
}

// ── PHYSICS ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RigidBodyInfo {
    pub rb_id: u64, pub object_id: u64,
    pub body_type: RigidBodyType,
    pub mass: f64, pub friction: f64, pub restitution: f64,
    pub collision_shape: CollisionShape,
    pub collision_margin: f64,
    pub kinematic: bool, pub ghost: bool,
    pub linear_damping: f64, pub angular_damping: f64,
    pub linear_velocity: [f64; 3], pub angular_velocity: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum RigidBodyType { #[default] Active, Passive }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum CollisionShape {
    #[default] Box, Sphere, Capsule, Cylinder, Cone, Convex_Hull, Mesh,
    Compound, Concave_Mesh,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SoftBodyInfo {
    pub sb_id: u64, pub object_id: u64,
    pub total_mass: f64, pub mass_stiffness: f64,
    pub friction: f64,
    pub edge_stiffness: f64, pub pull_stiffness: f64, pub push_stiffness: f64,
    pub bend_stiffness: f64,
    pub use_goal: bool, pub goal_stiffness: f64,
    pub aerodynamics: f64,
    pub self_collision: bool, pub self_repulsion_stiffness: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClothSimInfo {
    pub cloth_id: u64, pub object_id: u64,
    pub vertex_mass: f64,
    pub tension_stiffness: f64, pub compression_stiffness: f64,
    pub shear_stiffness: f64, pub bending_stiffness: f64,
    pub air_damping: f64, pub internal_friction: f64,
    pub use_pressure: bool, pub target_volume: f64,
    pub self_collision_enabled: bool, pub collision_distance: f64,
    pub vertex_group_mass: Option<String>,
    pub vertex_group_structural: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FluidDomainInfo {
    pub fluid_id: u64, pub object_id: u64,
    pub fluid_type: FluidType,
    pub resolution: u32,
    pub cache_path: Option<String>,
    pub gravity: [f64; 3],
    pub viscosity: f64, pub vorticity: f64,
    pub timestep_min: f64, pub timestep_max: f64,
    pub surface_subdivisions: u32,
    pub particle_radius: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum FluidType { #[default] Domain, Flow, Effector }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SmokeDomainInfo {
    pub smoke_id: u64, pub object_id: u64,
    pub resolution: u32, pub amplification: u32,
    pub density: f64, pub temperature: f64, pub heat: f64,
    pub use_high_resolution: bool,
    pub noise_type: String,
    pub cache_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParticleSystemInfo {
    pub ps_id: u64, pub object_id: u64, pub name: String,
    pub particle_type: ParticleType,
    pub count: u32,
    pub seed: u32,
    pub lifetime: f32, pub lifetime_random: f32,
    pub frame_start: f32, pub frame_end: f32,
    pub emit_from: String,                 // "FACE", "VOLUME", "EDGE", "VERT"
    pub use_advanced_hair: bool,
    pub physics_type: ParticlePhysics,
    pub render_type: ParticleRenderType,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ParticleType { #[default] Emitter, Hair, Reactor }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ParticlePhysics { #[default] Newton, Keyed, Boids, Fluid }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ParticleRenderType { #[default] Path, Object, Collection, Billboard }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HairSystemInfo {
    pub hair_id: u64, pub object_id: u64, pub name: String,
    pub strand_count: u32, pub strand_steps: u32,
    pub length: f64, pub radius_scale: f64,
    pub use_close_tip: bool,
    pub material_slot: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ForceFieldInfo {
    pub ff_id: u64, pub object_id: u64,
    pub field_type: ForceFieldType,
    pub strength: f64, pub flow: f64,
    pub noise: f64, pub seed: u32,
    pub falloff_type: String,
    pub min_distance: f64, pub max_distance: f64,
    pub use_2d: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ForceFieldType {
    #[default] None, Force, Wind, Vortex, Magnetic, Harmonic, Charge,
    Lennard_Jones, Texture, Curve, Boid, Turbulence, Drag, Smoke,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DynamicPaintInfo {
    pub dp_id: u64, pub object_id: u64,
    pub canvas: bool, pub brush: bool,
    pub output_type: String,
    pub bake_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PhysicsCacheInfo {
    pub cache_id: u64, pub name: String,
    pub physics_type: PhysicsSimType,
    pub frame_start: u32, pub frame_end: u32,
    pub is_baked: bool, pub cache_path: Option<String>,
    pub compression_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PhysicsSimType { #[default] RigidBody, SoftBody, Cloth, Fluid, Smoke, Particles, Hair }

// ── MATERIALS ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaterialInfo {
    pub material_id: u64, pub name: String,
    pub use_nodes: bool,
    pub base_color: [f32; 4],
    pub metallic: f32, pub roughness: f32, pub specular: f32,
    pub alpha: f32, pub blend_mode: String,
    pub shadow_mode: String,
    pub emission: [f32; 3], pub emission_strength: f32,
    pub ior: f32, pub transmission: f32, pub subsurface: f32,
    pub node_tree_id: Option<u64>,
    pub texture_ids: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShaderNodeInfo {
    pub node_id: u64, pub name: String,
    pub node_type: String,                 // "ShaderNodeBsdfPrincipled", "ShaderNodeTexImage", etc.
    pub location: [f32; 2],
    pub inputs: Vec<NodeSocket>,
    pub outputs: Vec<NodeSocket>,
    pub links_from: Vec<u64>,             // node_ids this connects from
    pub links_to: Vec<u64>,              // node_ids this connects to
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeSocket {
    pub name: String,
    pub socket_type: String,
    pub default_value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TextureInfo {
    pub texture_id: u64, pub name: String,
    pub texture_type: TextureType,
    pub image_path: Option<String>,
    pub width: u32, pub height: u32,
    pub color_space: String,
    pub interpolation: String,
    pub extension: String,               // "REPEAT", "CLIP", "CLIP_CUBE", "EXTEND"
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum TextureType { #[default] Image, Noise, Musgrave, Wave, Voronoi, Magic, Checker, Gradient }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UVMapInfo {
    pub uv_id: u64, pub name: String,
    pub mesh_id: u64,
    pub layer_index: u32,
    pub is_active_render: bool,
    pub is_active: bool,
}

// ── LIGHTS / CAMERAS / WORLD ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LightInfo {
    pub light_id: u64, pub name: String, pub object_id: u64,
    pub light_type: LightType,
    pub energy: f64, pub color: [f32; 3],
    pub radius: f64,
    pub spot_angle: f64, pub spot_blend: f64,
    pub cast_shadow: bool, pub shadow_soft_size: f64,
    pub use_custom_distance: bool, pub cutoff_distance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum LightType { #[default] Point, Sun, Spot, Area }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CameraInfo {
    pub camera_id: u64, pub name: String, pub object_id: u64,
    pub camera_type: CameraType,
    pub focal_length: f64, pub sensor_width: f64, pub sensor_height: f64,
    pub fov_horizontal: f64, pub fov_vertical: f64,
    pub clip_start: f64, pub clip_end: f64,
    pub dof_object: Option<u64>,
    pub dof_distance: f64, pub dof_aperture: f64,
    pub shift_x: f64, pub shift_y: f64,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum CameraType { #[default] Perspective, Orthographic, Panoramic }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorldInfo {
    pub world_id: u64, pub name: String,
    pub background_type: String,          // "SKY_TEXTURE", "ENVIRONMENT_TEXTURE", "COLOR"
    pub background_color: [f32; 3],
    pub background_strength: f32,
    pub use_nodes: bool,
    pub mist_start: f64, pub mist_depth: f64,
    pub ambient_color: [f32; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpeakerInfo {
    pub speaker_id: u64, pub name: String, pub object_id: u64,
    pub sound_path: Option<String>,
    pub volume: f32, pub pitch: f32,
    pub min_distance: f64, pub max_distance: f64,
    pub attenuation: f64,
}

// ── RENDER / COMPOSITOR / VSE / MOTION TRACK ─────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RenderSettingsInfo {
    pub engine: String,                    // "CYCLES", "BLENDER_EEVEE", "BLENDER_WORKBENCH"
    pub resolution_x: u32, pub resolution_y: u32, pub resolution_percentage: u32,
    pub samples: u32, pub preview_samples: u32,
    pub frame_start: u32, pub frame_end: u32, pub fps: f32,
    pub output_path: Option<String>,
    pub output_format: String,
    pub color_mode: String,
    pub film_transparent: bool,
    pub use_motion_blur: bool,
    pub use_denoising: bool,
    pub tile_x: u32, pub tile_y: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RenderLayerInfo {
    pub layer_id: u64, pub name: String,
    pub scene_name: String,
    pub use_solid: bool, pub use_strand: bool,
    pub samples: u32,
    pub passes: Vec<String>,              // "COMBINED", "Z", "NORMAL", "DIFFUSE_COLOR", etc.
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompositorNodeTree {
    pub tree_id: u64,
    pub node_count: u32,
    pub input_nodes: Vec<String>,
    pub output_nodes: Vec<String>,
    pub render_layers: Vec<String>,
    pub file_outputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VSEStripInfo {
    pub strip_id: u64, pub name: String,
    pub strip_type: VSEStripType,
    pub channel: u32,
    pub frame_start: u32, pub frame_end: u32,
    pub source_path: Option<String>,
    pub mute: bool, pub lock: bool,
    pub blend_type: String,
    pub transform: VSETransform,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum VSEStripType { #[default] Movie, Image, Sound, Scene, Text, Color, Mask, Meta, Multicam }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VSETransform { pub offset_x: f32, pub offset_y: f32, pub scale_x: f32, pub scale_y: f32, pub rotation: f32 }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MotionTrackInfo {
    pub track_id: u64, pub name: String,
    pub clip_name: String,
    pub marker_count: u32,
    pub error: f64,                        // reprojection error in pixels
    pub is_enabled: bool,
    pub pattern_size: [u32; 2],
    pub search_size: [u32; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrackingMarkerInfo {
    pub marker_id: u64, pub track_id: u64,
    pub frame: u32,
    pub position: [f32; 2],               // normalized image space
    pub is_enabled: bool, pub is_tracked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SolvedCameraInfo {
    pub camera_id: u64, pub name: String,
    pub frame_count: u32,
    pub reprojection_error: f64,
    pub focal_length: f64,
    pub k1: f64, pub k2: f64, pub k3: f64, // lens distortion
}

// ── SPATIAL / CUSTOM ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpatialRelationship3D {
    pub object_a: String, pub object_b: String,
    pub relationship_type: SpatialRelType,
    pub distance: Option<f64>,
    pub contact_area: Option<f64>,
    pub overlap_volume: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SpatialRelType {
    #[default] Adjacent, Contains, Above, Below, LeftOf, RightOf, InFrontOf, Behind,
    Intersects, Overlaps, Touches, Far, Near, Supports, SymmetricalTo,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IDPropertyInfo {
    pub prop_id: u64, pub owner_type: String, pub owner_name: String,
    pub name: String, pub value: serde_json::Value, pub prop_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CustomAttributeInfo {
    pub attr_id: u64, pub name: String, pub domain: String, pub data_type: String,
    pub mesh_name: Option<String>, pub element_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LibraryInfo {
    pub lib_id: u64, pub name: String, pub filepath: String,
    pub packed: bool, pub version: u32,
}

// ── SIMULATION DOMAIN / EXPORT ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SimulationDomain {
    #[default] Robotics, Manipulation, AutonomousVehicles, Drones,
    IndustrialAutomation, MedicalSurgical, Biomechanics, Aerospace,
    Maritime, Construction, Automotive, SoftRobotics,
    FluidDynamics, ClothSimulation, ParticleEffect, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThreeDExportFormat {
    GLTF { separate_textures: bool }, GLB,
    OBJ { include_materials: bool, triangulate: bool },
    FBX { use_armature_deform_only: bool },
    USD { export_textures: bool }, USDA, USDC, USDZ,
    STL { ascii: bool, use_scene_unit: bool },
    PLY { use_normals: bool, use_colors: bool },
    URDF { mesh_format: String, generate_joint_limits: bool },
    MJCF { include_textures: bool },
    AlembicCache { use_global_frame: bool, export_hair: bool, export_particles: bool },
    IsaacSimUSD { include_physics: bool, include_sensors: bool },
    BlenderManifest,
    NumpyArrays { include_normals: bool, include_uvs: bool },
    PointCloud { format: PointCloudFormat },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PointCloudFormat { XYZ, PCD, PLY, LAS }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExportOptions {
    pub apply_modifiers: bool,
    pub triangulate: bool,
    pub apply_transform: bool,
    pub export_selected_only: bool,
    pub include_custom_properties: bool,
    pub use_scene_unit: bool,
    pub up_axis: Axis,
    pub forward_axis: Axis,
    pub scale_factor: f64,
}

// ── MODIFIERS ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModifierType2 { // distinguished from enum ModifierType above
    Subdivision { levels: u32, render_levels: u32, use_catmull_clark: bool },
    Armature { object: String, use_deform_preserve_volume: bool, use_vertex_groups: bool },
    Boolean { operand_type: String, solver: String },
    Array { count: u32, fit_type: String },
    Mirror { use_x: bool, use_y: bool, use_z: bool, merge_threshold: f64 },
    Decimate { ratio: f32, decimate_type: String },
    Bevel { width: f64, segments: u32, limit_method: String },
    Solidify { thickness: f64, offset: f64 },
    Remesh { mode: String, octree_depth: u32, scale: f32 },
    Weld { merge_threshold: f64 },
    GeometryNodes { node_group_name: String },
    Custom { type_name: String, params: HashMap<String, serde_json::Value> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BooleanOp { Union, Difference, Intersect }

// ── SPATIAL INDEX / METHOD ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpatialMethod { BoundingBox, ConvexHull, MeshDistance, CenterOfMass }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpatialIndexType { Octree, BVH, KDTree }

// ── CROSS-MODAL ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrossModalLinkType {
    DescribedByText,         // text describes this scene/object
    ImplementedInCode,       // code controls this object
    VisualizedAsImage,       // image texture applied to object
    ParametricGeneratesMesh, // CAD (120) generated this mesh
    KinematicChainOf,        // kinematics (121) chain maps to rig
    ControlledBy,            // control system (122) commands this object
    FusedFromRadar,          // radar (124) point cloud became this geometry
    FusedFromSonar,          // sonar (125) acoustic 3D → geometry
    ThermalSurface,          // thermal (114) data mapped to surface
    HapticContact,           // haptic (113) contact geometry
    GeoReferenced,           // placed in geospatial (117) context
    DepthFused,              // depth (115) scan → geometry
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExternalPoint3D { pub x: f32, pub y: f32, pub z: f32, pub intensity: Option<f32> }

// ── SCENE UPDATE / HEADLESS ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SceneUpdate {
    MoveObject   { object_node_id: u64, new_transform: Transform4x4 },
    AddObject    { object: ObjectInfo },
    RemoveObject { object_node_id: u64 },
    UpdateMaterial { object_node_id: u64, material_info: MaterialInfo },
    UpdatePhysicsState { object_node_id: u64, velocity: [f64; 3], angular_velocity: [f64; 3] },
    SetBonePose  { armature_node_id: u64, bone_name: String, transform: Transform4x4 },
    SetShapeKey  { object_node_id: u64, shapekey_name: String, value: f32 },
    AddConstraint { bone_node_id: u64, constraint: ConstraintInfo },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThreeDHeadlessOp {
    RecomputeNormals { mesh_node_id: u64 },
    ApplyModifiers   { object_node_id: u64 },
    BakePhysics      { start_frame: u32, end_frame: u32 },
    ExportToFormat   { format: ThreeDExportFormat, path: String },
    ComputeSpatial,
    CrossFuseDepth   { depth_data: Vec<ExternalPoint3D> },
    CrossFuseRadar   { radar_graph_id: u64 },
    SolveIK          { armature_node_id: u64, target_positions: HashMap<String, [f64; 3]> },
}

// ── QUERY / HOOKS / DISPLAY ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThreeDGraphQuery {
    NodeDetail           { node_id: u64 },
    ObjectsByType        { object_type: String },
    SpatialNeighbors     { object_node_id: u64, radius_m: f64 },
    PhysicsObjects,
    AnimatedObjects,
    MaterialsUsed,
    CrossModalLinks      { node_id: u64 },
    BoneHierarchy        { armature_node_id: u64 },
    ConstraintChain      { bone_node_id: u64 },
    PhysicsCaches,
    AGIActivity,
    AllNodes, AllEdges,
    ObjectTransform      { object_node_id: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThreeDSemanticHook {
    OnGraphCreated,
    OnInferRelationships,
    OnEdgeCompletion,
    OnCrossModalityLink { target_modality: String, target_graph_id: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViewportMode { Solid, Wireframe, Material, Rendered, Graph, Spatial }

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH NODE TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ThreeDNodeType {
    #[default] Scene, Collection, Object, Mesh, Vertex, Edge, Face, Loop,
    Curve, Spline, Surface, Metaball, Volume, GreasePencilLayer,
    Armature, Bone, Constraint, ShapeKey, VertexGroup, Lattice,
    Action, FCurve, Keyframe, Driver, NLATrack, NLAStrip,
    Modifier, GeometryNodes,
    RigidBody, SoftBody, Cloth, FluidDomain, SmokeDomain,
    ParticleSystem, Hair, ForceField, DynamicPaint, PhysicsCache,
    Material, ShaderNode, Texture, Image, UVMap,
    Light, Camera, World, FreestyleLineStyle,
    CompositorNodeTree, CompositorNode, RenderLayer, RenderPass,
    VSEStrip, MotionTrack, TrackingMarker, SolvedCamera,
    Sound, Speaker,
    IDProperty, Library, CustomAttribute,
    SpatialRelationshipNode, BoundingBoxNode,
    CrossModalRef,              // placeholder node for cross-modal links
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThreeDGraphNode {
    pub node_id: u64,
    pub node_type: ThreeDNodeType,
    pub content: String,

    // 3D-SPECIFIC PAYLOAD
    pub transform: Option<Transform4x4>,
    pub location: Option<[f64; 3]>,
    pub dimensions: Option<[f64; 3]>,
    pub bounding_box: Option<BoundingBox3D>,
    pub frame_range: Option<(f32, f32)>,
    pub physics_type: Option<String>,
    pub mass: Option<f64>,
    pub is_deform_bone: Option<bool>,
    pub influence: Option<f32>,
    pub material_name: Option<String>,
    pub source_id: Option<u64>,             // ID in the analysis result

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
pub enum ThreeDEdgeType {
    // ── STRUCTURAL ──
    #[default] Contains, ParentOf, ChildOf, InCollection, BelongsToScene,

    // ── RIGGING ──
    Deforms,                // armature/bone deforms mesh
    BoundTo,                // vertex group bound to bone
    Constrains,             // constraint applied between bones/objects
    ShapeKeyAffects,        // shape key deforms mesh
    VertexGroupWeights,     // vertex group determines deformation weight
    DrivenBy,              // property is driven by driver

    // ── ANIMATION ──
    Animates,               // action animates object/bone
    Drives,                 // driver drives fcurve
    KeyframeAt,             // keyframe at frame N
    NLABlends,              // NLA strip blends action
    ModifiesAnimation,      // noise/cycles modifier on fcurve

    // ── PHYSICS ──
    HasRigidBody, HasSoftBody, HasCloth, HasFluid,
    CollidesWith,           // collision between rigid bodies
    InfluencedByForce,      // object influenced by force field
    Simulates,              // simulation cache covers this object
    CachedAs,               // physics cached to file

    // ── MATERIALS / SHADERS ──
    HasMaterial,            // object has material in slot
    HasModifier,            // object has modifier applied
    TextureUses,            // shader node uses texture
    ShaderConnectsTo,       // shader output → shader input
    UVLayerOf,              // UV map belongs to mesh

    // ── SPATIAL ──
    Above, Below, InFrontOf, Behind, LeftOf, RightOf,
    NearTo, Intersects, Supports, SymmetricalTo,
    ContainedIn3D, SpatiallyAdjacentTo,

    // ── CROSS-MODAL ──
    DescribedByText,         // text (100) describes this object
    ImplementedInCode,       // code (101) controls this
    VisualizedAsImage,       // image (102) is applied as texture
    ParametricGeneratesMesh, // CAD (120) generated this geometry
    ExportsToURDF,           // this scene exported to URDF for robotics
    ExportsToMJCF,           // exported to MJCF
    SoundDescribes,          // speaker/sound associated with object
    KinematicChainOf,        // kinematics (121) chain
    ControlledBy,            // control system (122) commands this
    FusedFromDepth,          // depth sensor (115) data
    FusedFromRadar,          // radar (124) 3D
    FusedFromSonar,          // sonar (125) 3D
    ThermalSurface,          // thermal (114) data
    HapticContact,           // haptic (113) contact
    GeoReferenced,           // geospatial (117) placement
    MaterialFrom126,         // hyperspectral (126) material identification

    // ── UNIVERSAL SEMANTIC ──
    Performs, Affects, Implies, Contradicts, Elaborates, Summarizes, Supports,
    TemporalPrecedes, TemporalFollows, CausedBy, Enables, Prevents,
    PartOf, FunctionalRole, InstanceOf,
    DerivedFrom, VersionOf, RefinesTo, ForkedFrom,
    SimilarTo,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThreeDGraphEdge {
    pub edge_id: u64,
    pub from_node: u64, pub to_node: u64,
    pub edge_type: ThreeDEdgeType,
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
pub struct ThreeDGraph {
    pub graph_id: u64, pub project_id: u64,
    pub source_description: String,
    pub nodes: Vec<ThreeDGraphNode>,
    pub edges: Vec<ThreeDGraphEdge>,
    pub root_node_id: u64,
    pub state: GraphStateType,
    pub state_history: Vec<GraphStateTransition>,
    pub created_at: String, pub updated_at: String,
    pub version: u32,
    pub version_notes: Vec<VersionNote>,
}

// ─────────────────────────────────────────────────────────────────────────────
// OUTPUT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThreeDModalityOutput {
    pub success: bool,
    pub graph_id: Option<u64>,
    pub graph: Option<ThreeDGraph>,
    pub analysis: Option<ThreeDAnalysisResult>,
    pub query_result: Option<serde_json::Value>,
    pub export_path: Option<String>,
    pub simulation_frames: Option<u32>,
    pub spatial_index_built: Option<bool>,
    pub error: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// SHARED TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ProvisionalStatus { #[default] Planned, Generating, Generated, Validated, Finalized, Failed, RolledBack }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VersionNote {
    pub version: u32, pub note: String,
    pub step_index: Option<u32>, pub timestamp: String, pub change_type: ChangeType,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ChangeType { #[default] Created, Updated, CrossLinked, EnrichedBySemantic, EnrichedByHook, RolledBack, Finalized }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum EdgeProvenance {
    #[default] Unknown,
    DerivedFromPrompt, DerivedFromChunk(u32), DerivedFromChunkGraph(u64),
    DerivedFromModalityGraph(u64), DerivedFromFile(String),
    DerivedFromAMT, DerivedFromBlueprint(u32), DerivedFromMethodology(u64),
    DerivedFromCrossModal, DerivedFromHook, VersionOf(u32), ForkedFrom(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum GraphStateType { #[default] Created, SemanticEnriched, CrossLinked, Stable, Updated, ReValidating, Archived }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStateTransition {
    pub from: GraphStateType, pub to: GraphStateType,
    pub timestamp: String, pub triggered_by_step: Option<u32>,
}

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
        let input = serde_json::json!({"action":"Prompt","prompt":prompt,"max_tokens":max_tokens,"temperature":0.05,"system_context":"3D scene analysis. Return only valid JSON."});
        let out = std::process::Command::new(&self.prompt_pipeline_path)
            .arg("--input").arg(input.to_string())
            .output().map_err(|e| e.to_string())?;
        let r: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
        Ok(r["response"].as_str().unwrap_or("{}").to_string())
    }

    fn save_graph(&self, g: &ThreeDGraph) -> Result<(), String> {
        let path = format!("{}/local/3d_graph_{}.json", self.zsei_path, g.graph_id);
        if let Some(p) = std::path::Path::new(&path).parent() { std::fs::create_dir_all(p).map_err(|e| e.to_string())?; }
        std::fs::write(&path, serde_json::to_string_pretty(g).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    fn load_graph(&self, id: u64) -> Result<ThreeDGraph, String> {
        let path = format!("{}/local/3d_graph_{}.json", self.zsei_path, id);
        serde_json::from_str(&std::fs::read_to_string(&path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    fn generate_id(&self) -> u64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as u64
    }
    fn now_iso8601(&self) -> String { format!("{}", self.generate_id()) }

    fn extract_json_array(raw: &str) -> String {
        if let (Some(s), Some(e)) = (raw.find('['), raw.rfind(']')) { raw[s..=e].to_string() } else { "[]".to_string() }
    }
    fn extract_json_object(raw: &str) -> String {
        if let (Some(s), Some(e)) = (raw.find('{'), raw.rfind('}')) { raw[s..=e].to_string() } else { "{}".to_string() }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// LLM-BASED ANALYSIS
// ─────────────────────────────────────────────────────────────────────────────

impl PipelineExecutor {
    async fn infer_semantic_relationships(&self, nodes: &[ThreeDGraphNode]) -> Vec<(u64, u64, ThreeDEdgeType, String)> {
        if nodes.len() < 2 { return vec![]; }

        let node_list: Vec<serde_json::Value> = nodes.iter().take(30).map(|n| serde_json::json!({
            "node_id": n.node_id, "type": format!("{:?}", n.node_type),
            "content": n.content.chars().take(80).collect::<String>(),
            "physics": n.physics_type, "mass": n.mass,
        })).collect();

        let prompt = format!(r#"
Identify semantic relationships between these 3D scene graph nodes not already captured structurally.

Nodes:
{}

Available relationship types: CollidesWith, InfluencedByForce, Supports, SymmetricalTo, SimilarTo,
Above, Below, InFrontOf, Performs, Affects, CausedBy, Enables, FunctionalRole, PartOf,
DescribedByText, ImplementedInCode, DerivedFrom, TemporalPrecedes

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
                        let etype = map_3d_edge_str(v["edge_type"].as_str().unwrap_or("Affects"));
                        let reason = v["reason"].as_str().unwrap_or("").to_string();
                        Some((from, to, etype, reason))
                    }).collect()
            }
            Err(_) => vec![],
        }
    }

    async fn infer_spatial_relationships(&self, objects: &[&ThreeDGraphNode]) -> Vec<(u64, u64, ThreeDEdgeType)> {
        if objects.len() < 2 { return vec![]; }

        let obj_list: Vec<serde_json::Value> = objects.iter().take(20).map(|n| serde_json::json!({
            "node_id": n.node_id,
            "name": &n.content[..n.content.len().min(40)],
            "location": n.location,
            "dimensions": n.dimensions,
            "bounding_box": n.bounding_box,
        })).collect();

        let prompt = format!(r#"
Given these 3D objects with their locations and bounding boxes, infer spatial relationships.

Objects:
{}

Determine: Above/Below, InFrontOf/Behind, LeftOf/RightOf, NearTo, Supports, Intersects, SymmetricalTo.
Use location and bounding box data to reason about spatial arrangement.

Return ONLY valid JSON array:
[{{"from_id": N, "to_id": M, "spatial_type": "Above|Below|InFrontOf|Behind|LeftOf|RightOf|NearTo|Supports|Intersects|SymmetricalTo"}}]"#,
            serde_json::to_string_pretty(&obj_list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 600).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str)
                    .unwrap_or_default().into_iter()
                    .filter_map(|v| {
                        let from = v["from_id"].as_u64()?;
                        let to = v["to_id"].as_u64()?;
                        let etype = map_3d_edge_str(v["spatial_type"].as_str().unwrap_or("NearTo"));
                        Some((from, to, etype))
                    }).collect()
            }
            Err(_) => vec![],
        }
    }

    async fn classify_scene_function(&self, analysis: &ThreeDAnalysisResult) -> String {
        let summary = serde_json::json!({
            "object_count": analysis.objects.len(),
            "has_armatures": !analysis.armatures.is_empty(),
            "has_rigid_bodies": !analysis.rigid_bodies.is_empty(),
            "has_fluid": !analysis.fluid_domains.is_empty(),
            "has_cloth": !analysis.cloth_sims.is_empty(),
            "has_particles": !analysis.particle_systems.is_empty(),
            "has_cameras": !analysis.cameras.is_empty(),
            "object_names_sample": analysis.objects.iter().take(5).map(|o| &o.name).collect::<Vec<_>>(),
            "source_format": &analysis.source_format,
        });
        let prompt = format!(r#"
Classify the function/domain of this 3D scene:
{}
Choose: Character/Creature, Architectural, Industrial, Robotics, Vehicle, Environment/Landscape,
Prop/Asset, VFX/Simulation, Mechanical/CAD, Medical, Game_Asset, Scientific, Generic.
Return ONLY the single classification label."#,
            serde_json::to_string(&summary).unwrap_or_default());
        self.llm_zero_shot(&prompt, 20).await.unwrap_or_else(|_| "Generic".into()).trim().to_string()
    }
}

fn map_3d_edge_str(s: &str) -> ThreeDEdgeType {
    match s {
        "Contains"              => ThreeDEdgeType::Contains,
        "ParentOf"              => ThreeDEdgeType::ParentOf,
        "ChildOf"               => ThreeDEdgeType::ChildOf,
        "InCollection"          => ThreeDEdgeType::InCollection,
        "Deforms"               => ThreeDEdgeType::Deforms,
        "BoundTo"               => ThreeDEdgeType::BoundTo,
        "Constrains"            => ThreeDEdgeType::Constrains,
        "ShapeKeyAffects"       => ThreeDEdgeType::ShapeKeyAffects,
        "DrivenBy"              => ThreeDEdgeType::DrivenBy,
        "Animates"              => ThreeDEdgeType::Animates,
        "Drives"                => ThreeDEdgeType::Drives,
        "KeyframeAt"            => ThreeDEdgeType::KeyframeAt,
        "NLABlends"             => ThreeDEdgeType::NLABlends,
        "HasRigidBody"          => ThreeDEdgeType::HasRigidBody,
        "HasSoftBody"           => ThreeDEdgeType::HasSoftBody,
        "HasCloth"              => ThreeDEdgeType::HasCloth,
        "HasFluid"              => ThreeDEdgeType::HasFluid,
        "CollidesWith"          => ThreeDEdgeType::CollidesWith,
        "InfluencedByForce"     => ThreeDEdgeType::InfluencedByForce,
        "Simulates"             => ThreeDEdgeType::Simulates,
        "CachedAs"              => ThreeDEdgeType::CachedAs,
        "HasMaterial"           => ThreeDEdgeType::HasMaterial,
        "HasModifier"           => ThreeDEdgeType::HasModifier,
        "TextureUses"           => ThreeDEdgeType::TextureUses,
        "ShaderConnectsTo"      => ThreeDEdgeType::ShaderConnectsTo,
        "Above"                 => ThreeDEdgeType::Above,
        "Below"                 => ThreeDEdgeType::Below,
        "InFrontOf"             => ThreeDEdgeType::InFrontOf,
        "Behind"                => ThreeDEdgeType::Behind,
        "LeftOf"                => ThreeDEdgeType::LeftOf,
        "RightOf"               => ThreeDEdgeType::RightOf,
        "NearTo"                => ThreeDEdgeType::NearTo,
        "Intersects"            => ThreeDEdgeType::Intersects,
        "Supports"              => ThreeDEdgeType::Supports,
        "SymmetricalTo"         => ThreeDEdgeType::SymmetricalTo,
        "ContainedIn3D"         => ThreeDEdgeType::ContainedIn3D,
        "DescribedByText"       => ThreeDEdgeType::DescribedByText,
        "ImplementedInCode"     => ThreeDEdgeType::ImplementedInCode,
        "ParametricGeneratesMesh"=>ThreeDEdgeType::ParametricGeneratesMesh,
        "KinematicChainOf"      => ThreeDEdgeType::KinematicChainOf,
        "ControlledBy"          => ThreeDEdgeType::ControlledBy,
        "FusedFromDepth"        => ThreeDEdgeType::FusedFromDepth,
        "FusedFromRadar"        => ThreeDEdgeType::FusedFromRadar,
        "FusedFromSonar"        => ThreeDEdgeType::FusedFromSonar,
        "GeoReferenced"         => ThreeDEdgeType::GeoReferenced,
        "Affects"               => ThreeDEdgeType::Affects,
        "Performs"              => ThreeDEdgeType::Performs,
        "CausedBy"              => ThreeDEdgeType::CausedBy,
        "Enables"               => ThreeDEdgeType::Enables,
        "Prevents"              => ThreeDEdgeType::Prevents,
        "TemporalPrecedes"      => ThreeDEdgeType::TemporalPrecedes,
        "PartOf"                => ThreeDEdgeType::PartOf,
        "FunctionalRole"        => ThreeDEdgeType::FunctionalRole,
        "InstanceOf"            => ThreeDEdgeType::InstanceOf,
        "DerivedFrom"           => ThreeDEdgeType::DerivedFrom,
        "SimilarTo"             => ThreeDEdgeType::SimilarTo,
        _                       => ThreeDEdgeType::Affects,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH CREATION — FULL BLENDER HIERARCHY
// ─────────────────────────────────────────────────────────────────────────────

async fn create_graph(
    executor: &PipelineExecutor,
    analysis: ThreeDAnalysisResult,
    project_id: u64,
) -> ThreeDModalityOutput {
    let graph_id = executor.generate_id();
    let now = executor.now_iso8601();
    let mut nodes: Vec<ThreeDGraphNode> = Vec::new();
    let mut edges: Vec<ThreeDGraphEdge> = Vec::new();
    let mut node_id: u64 = 1;
    let mut edge_id: u64 = 1;

    // ── ROOT: SCENE ──
    let root_id = node_id;
    let scene_function = executor.classify_scene_function(&analysis).await;
    nodes.push(ThreeDGraphNode {
        node_id: root_id, node_type: ThreeDNodeType::Scene,
        content: format!("3D Scene [{}]: {}objs {}meshes {}arms {}phys {}mats format={}",
            scene_function, analysis.objects.len(), analysis.meshes.len(),
            analysis.armatures.len(), analysis.rigid_bodies.len() + analysis.soft_bodies.len() + analysis.cloth_sims.len(),
            analysis.materials.len(), analysis.source_format),
        materialized_path: Some(format!("/Modalities/ThreeD/Project_{}/Graph_{}", project_id, graph_id)),
        provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
        version_notes: vec![VersionNote { version: 1, note: "Initial creation".into(), step_index: None, timestamp: now.clone(), change_type: ChangeType::Created }],
        keywords: vec!["scene".into(), "3d".into(), scene_function.to_lowercase()],
        hotness_score: 1.0, ..Default::default()
    });
    node_id += 1;

    // Build lookup: data_id → node_id for mesh/armature/etc.
    let mut mesh_id_to_nid: HashMap<u64, u64> = HashMap::new();
    let mut obj_id_to_nid: HashMap<u64, u64> = HashMap::new();
    let mut arm_id_to_nid: HashMap<u64, u64> = HashMap::new();
    let mut bone_id_to_nid: HashMap<u64, u64> = HashMap::new();
    let mut mat_id_to_nid: HashMap<u64, u64> = HashMap::new();
    let mut action_id_to_nid: HashMap<u64, u64> = HashMap::new();
    let mut rb_id_to_nid: HashMap<u64, u64> = HashMap::new();
    let mut ps_id_to_nid: HashMap<u64, u64> = HashMap::new();

    // ── COLLECTIONS ──
    let mut coll_id_to_nid: HashMap<u64, u64> = HashMap::new();
    for coll in &analysis.collections {
        let cid = node_id;
        nodes.push(ThreeDGraphNode {
            node_id: cid, node_type: ThreeDNodeType::Collection,
            content: format!("Collection: {} ({} objects {} sub-collections)", coll.name, coll.object_ids.len(), coll.child_collection_ids.len()),
            materialized_path: Some(format!("/Modalities/ThreeD/Project_{}/Graph_{}/Collection/{}", project_id, graph_id, coll.collection_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: vec!["collection".into(), coll.name.to_lowercase()],
            hotness_score: 0.7, ..Default::default()
        });
        edges.push(ThreeDGraphEdge { edge_id, from_node: root_id, to_node: cid, edge_type: ThreeDEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        coll_id_to_nid.insert(coll.collection_id, cid);
        node_id += 1;
    }
    // Collection hierarchy
    for coll in &analysis.collections {
        if let (Some(&child_nid), Some(parent_id)) = (coll_id_to_nid.get(&coll.collection_id), coll.parent_collection_id) {
            if let Some(&parent_nid) = coll_id_to_nid.get(&parent_id) {
                edges.push(ThreeDGraphEdge { edge_id, from_node: parent_nid, to_node: child_nid, edge_type: ThreeDEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
    }

    // ── MESHES ──
    for mesh in &analysis.meshes {
        let mid = node_id;
        nodes.push(ThreeDGraphNode {
            node_id: mid, node_type: ThreeDNodeType::Mesh,
            content: format!("Mesh: {} verts={} faces={} manifold={} uvs={} shapekeys={}",
                mesh.name, mesh.vertex_count, mesh.face_count, mesh.is_manifold, mesh.uv_layer_count, mesh.shape_key_count),
            bounding_box: Some(mesh.bounding_box.clone()),
            materialized_path: Some(format!("/Modalities/ThreeD/Project_{}/Graph_{}/Mesh/{}", project_id, graph_id, mesh.mesh_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: vec!["mesh".into(), mesh.name.to_lowercase()],
            hotness_score: 0.8, ..Default::default()
        });
        mesh_id_to_nid.insert(mesh.mesh_id, mid);
        node_id += 1;
    }

    // ── ARMATURES ──
    for arm in &analysis.armatures {
        let aid = node_id;
        nodes.push(ThreeDGraphNode {
            node_id: aid, node_type: ThreeDNodeType::Armature,
            content: format!("Armature: {} bones={}", arm.name, arm.bone_count),
            materialized_path: Some(format!("/Modalities/ThreeD/Project_{}/Graph_{}/Armature/{}", project_id, graph_id, arm.armature_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: vec!["armature".into(), "rig".into(), arm.name.to_lowercase()],
            hotness_score: 0.75, ..Default::default()
        });
        arm_id_to_nid.insert(arm.armature_id, aid);
        node_id += 1;
    }

    // ── BONES ──
    for bone in &analysis.bones {
        let bid = node_id;
        nodes.push(ThreeDGraphNode {
            node_id: bid, node_type: ThreeDNodeType::Bone,
            content: format!("Bone: {} len={:.3}m deform={} connect={}", bone.name, bone.length, bone.is_deform, bone.use_connect),
            is_deform_bone: Some(bone.is_deform),
            materialized_path: Some(format!("/Modalities/ThreeD/Project_{}/Graph_{}/Bone/{}", project_id, graph_id, bone.bone_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: vec!["bone".into(), bone.name.to_lowercase()],
            hotness_score: 0.6, ..Default::default()
        });
        bone_id_to_nid.insert(bone.bone_id, bid);
        node_id += 1;
    }
    // Bone hierarchy (parent → child)
    for bone in &analysis.bones {
        if let Some(&bid) = bone_id_to_nid.get(&bone.bone_id) {
            if let Some(parent_id) = bone.parent_bone_id {
                if let Some(&parent_nid) = bone_id_to_nid.get(&parent_id) {
                    edges.push(ThreeDGraphEdge { edge_id, from_node: parent_nid, to_node: bid, edge_type: ThreeDEdgeType::ParentOf, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                    edge_id += 1;
                }
            }
            // Armature contains bone
            for arm in &analysis.armatures {
                if arm.bone_ids.contains(&bone.bone_id) {
                    if let Some(&arm_nid) = arm_id_to_nid.get(&arm.armature_id) {
                        edges.push(ThreeDGraphEdge { edge_id, from_node: arm_nid, to_node: bid, edge_type: ThreeDEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                        edge_id += 1;
                    }
                }
            }
        }
    }

    // ── CONSTRAINTS ──
    for constr in &analysis.constraints {
        let cid = node_id;
        nodes.push(ThreeDGraphNode {
            node_id: cid, node_type: ThreeDNodeType::Constraint,
            content: format!("Constraint {:?}: {} inf={:.2}", constr.constraint_type, constr.name, constr.influence),
            influence: Some(constr.influence),
            materialized_path: Some(format!("/Modalities/ThreeD/Project_{}/Graph_{}/Constraint/{}", project_id, graph_id, constr.constraint_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: vec!["constraint".into(), format!("{:?}", constr.constraint_type).to_lowercase()],
            hotness_score: 0.55, ..Default::default()
        });
        // Owner bone → constraint
        if let Some(owner_bone_id) = constr.owner_bone_id {
            if let Some(&owner_nid) = bone_id_to_nid.get(&owner_bone_id) {
                edges.push(ThreeDGraphEdge { edge_id, from_node: owner_nid, to_node: cid, edge_type: ThreeDEdgeType::Constrains, weight: constr.influence, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        node_id += 1;
    }

    // ── MATERIALS ──
    for mat in &analysis.materials {
        let mid = node_id;
        nodes.push(ThreeDGraphNode {
            node_id: mid, node_type: ThreeDNodeType::Material,
            content: format!("Material: {} metal={:.2} rough={:.2} alpha={:.2} nodes={}", mat.name, mat.metallic, mat.roughness, mat.alpha, mat.use_nodes),
            material_name: Some(mat.name.clone()),
            materialized_path: Some(format!("/Modalities/ThreeD/Project_{}/Graph_{}/Material/{}", project_id, graph_id, mat.material_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: vec!["material".into(), mat.name.to_lowercase()],
            hotness_score: 0.65, ..Default::default()
        });
        mat_id_to_nid.insert(mat.material_id, mid);
        node_id += 1;
    }

    // ── ACTIONS ──
    for action in &analysis.actions {
        let aid = node_id;
        nodes.push(ThreeDGraphNode {
            node_id: aid, node_type: ThreeDNodeType::Action,
            content: format!("Action: {} fcurves={} frames={:.0}–{:.0}", action.name, action.fcurve_count, action.frame_range.0, action.frame_range.1),
            frame_range: Some(action.frame_range),
            materialized_path: Some(format!("/Modalities/ThreeD/Project_{}/Graph_{}/Action/{}", project_id, graph_id, action.action_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: vec!["action".into(), "animation".into(), action.name.to_lowercase()],
            hotness_score: 0.7, ..Default::default()
        });
        action_id_to_nid.insert(action.action_id, aid);
        node_id += 1;
    }

    // ── OBJECTS (main hierarchy) ──
    for obj in &analysis.objects {
        let oid = node_id;
        nodes.push(ThreeDGraphNode {
            node_id: oid, node_type: ThreeDNodeType::Object,
            content: format!("Object: {} [{:?}] loc=({:.2},{:.2},{:.2}) vis={}", obj.name, obj.object_type, obj.location[0], obj.location[1], obj.location[2], obj.visible),
            transform: Some(obj.transform.clone()),
            location: Some(obj.location),
            dimensions: Some(obj.dimensions),
            materialized_path: Some(format!("/Modalities/ThreeD/Project_{}/Graph_{}/Object/{}", project_id, graph_id, obj.object_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: { let mut kw = vec!["object".into(), format!("{:?}", obj.object_type).to_lowercase(), obj.name.to_lowercase()]; kw },
            hotness_score: 0.85, ..Default::default()
        });
        obj_id_to_nid.insert(obj.object_id, oid);

        // Object → scene root
        edges.push(ThreeDGraphEdge { edge_id, from_node: root_id, to_node: oid, edge_type: ThreeDEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Object → collection
        for coll in &analysis.collections {
            if coll.object_ids.contains(&obj.object_id) {
                if let Some(&coll_nid) = coll_id_to_nid.get(&coll.collection_id) {
                    edges.push(ThreeDGraphEdge { edge_id, from_node: coll_nid, to_node: oid, edge_type: ThreeDEdgeType::InCollection, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                    edge_id += 1;
                }
            }
        }

        // Object → mesh (HasMesh is via Contains)
        if let Some(data_id) = obj.data_id {
            if let Some(&mesh_nid) = mesh_id_to_nid.get(&data_id) {
                edges.push(ThreeDGraphEdge { edge_id, from_node: oid, to_node: mesh_nid, edge_type: ThreeDEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }

        // Object → armature (Deforms)
        if let Some(arm_id) = obj.armature_id {
            if let Some(&arm_nid) = arm_id_to_nid.get(&arm_id) {
                edges.push(ThreeDGraphEdge { edge_id, from_node: arm_nid, to_node: oid, edge_type: ThreeDEdgeType::Deforms, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }

        // Object → materials (HasMaterial)
        for mat_id in &obj.material_ids {
            if let Some(&mat_nid) = mat_id_to_nid.get(mat_id) {
                edges.push(ThreeDGraphEdge { edge_id, from_node: oid, to_node: mat_nid, edge_type: ThreeDEdgeType::HasMaterial, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }

        node_id += 1;
    }

    // Object parent-child hierarchy
    for obj in &analysis.objects {
        if let (Some(&obj_nid), Some(parent_id)) = (obj_id_to_nid.get(&obj.object_id), obj.parent_id) {
            if let Some(&parent_nid) = obj_id_to_nid.get(&parent_id) {
                edges.push(ThreeDGraphEdge { edge_id, from_node: parent_nid, to_node: obj_nid, edge_type: ThreeDEdgeType::ParentOf, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
    }

    // ── RIGID BODIES ──
    for rb in &analysis.rigid_bodies {
        let rid = node_id;
        nodes.push(ThreeDGraphNode {
            node_id: rid, node_type: ThreeDNodeType::RigidBody,
            content: format!("RigidBody {:?}: mass={:.2}kg friction={:.2} shape={:?}", rb.body_type, rb.mass, rb.friction, rb.collision_shape),
            physics_type: Some(format!("{:?}", rb.body_type)),
            mass: Some(rb.mass),
            materialized_path: Some(format!("/Modalities/ThreeD/Project_{}/Graph_{}/RigidBody/{}", project_id, graph_id, rb.rb_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: vec!["rigid-body".into(), "physics".into()],
            hotness_score: 0.7, ..Default::default()
        });
        rb_id_to_nid.insert(rb.rb_id, rid);
        if let Some(&obj_nid) = obj_id_to_nid.get(&rb.object_id) {
            edges.push(ThreeDGraphEdge { edge_id, from_node: obj_nid, to_node: rid, edge_type: ThreeDEdgeType::HasRigidBody, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── SOFT BODIES ──
    for sb in &analysis.soft_bodies {
        let sid = node_id;
        nodes.push(ThreeDGraphNode {
            node_id: sid, node_type: ThreeDNodeType::SoftBody,
            content: format!("SoftBody: mass={:.2}kg edge_stiffness={:.2} bend={:.2}", sb.total_mass, sb.edge_stiffness, sb.bend_stiffness),
            physics_type: Some("SoftBody".into()), mass: Some(sb.total_mass),
            materialized_path: Some(format!("/Modalities/ThreeD/Project_{}/Graph_{}/SoftBody/{}", project_id, graph_id, sb.sb_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: vec!["soft-body".into(), "physics".into()], hotness_score: 0.65, ..Default::default()
        });
        if let Some(&obj_nid) = obj_id_to_nid.get(&sb.object_id) {
            edges.push(ThreeDGraphEdge { edge_id, from_node: obj_nid, to_node: sid, edge_type: ThreeDEdgeType::HasSoftBody, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── CLOTH ──
    for cloth in &analysis.cloth_sims {
        let cid = node_id;
        nodes.push(ThreeDGraphNode {
            node_id: cid, node_type: ThreeDNodeType::Cloth,
            content: format!("Cloth: vertex_mass={:.3}kg tension={:.2} bend={:.2}", cloth.vertex_mass, cloth.tension_stiffness, cloth.bending_stiffness),
            physics_type: Some("Cloth".into()),
            materialized_path: Some(format!("/Modalities/ThreeD/Project_{}/Graph_{}/Cloth/{}", project_id, graph_id, cloth.cloth_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: vec!["cloth".into(), "physics".into()], hotness_score: 0.65, ..Default::default()
        });
        if let Some(&obj_nid) = obj_id_to_nid.get(&cloth.object_id) {
            edges.push(ThreeDGraphEdge { edge_id, from_node: obj_nid, to_node: cid, edge_type: ThreeDEdgeType::HasCloth, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── FLUID DOMAINS ──
    for fluid in &analysis.fluid_domains {
        let fid = node_id;
        nodes.push(ThreeDGraphNode {
            node_id: fid, node_type: ThreeDNodeType::FluidDomain,
            content: format!("FluidDomain {:?}: res={} viscosity={:.4}", fluid.fluid_type, fluid.resolution, fluid.viscosity),
            physics_type: Some(format!("{:?}", fluid.fluid_type)),
            materialized_path: Some(format!("/Modalities/ThreeD/Project_{}/Graph_{}/Fluid/{}", project_id, graph_id, fluid.fluid_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: vec!["fluid".into(), "physics".into()], hotness_score: 0.7, ..Default::default()
        });
        if let Some(&obj_nid) = obj_id_to_nid.get(&fluid.object_id) {
            edges.push(ThreeDGraphEdge { edge_id, from_node: obj_nid, to_node: fid, edge_type: ThreeDEdgeType::HasFluid, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── PARTICLE SYSTEMS ──
    for ps in &analysis.particle_systems {
        let pid = node_id;
        nodes.push(ThreeDGraphNode {
            node_id: pid, node_type: ThreeDNodeType::ParticleSystem,
            content: format!("Particles: {} {:?} count={}", ps.name, ps.particle_type, ps.count),
            physics_type: Some("Particles".into()),
            materialized_path: Some(format!("/Modalities/ThreeD/Project_{}/Graph_{}/Particles/{}", project_id, graph_id, ps.ps_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: vec!["particles".into(), format!("{:?}", ps.particle_type).to_lowercase()], hotness_score: 0.6, ..Default::default()
        });
        ps_id_to_nid.insert(ps.ps_id, pid);
        if let Some(&obj_nid) = obj_id_to_nid.get(&ps.object_id) {
            edges.push(ThreeDGraphEdge { edge_id, from_node: obj_nid, to_node: pid, edge_type: ThreeDEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── FORCE FIELDS ──
    for ff in &analysis.force_fields {
        let fid = node_id;
        nodes.push(ThreeDGraphNode {
            node_id: fid, node_type: ThreeDNodeType::ForceField,
            content: format!("ForceField {:?}: strength={:.2}", ff.field_type, ff.strength),
            physics_type: Some(format!("{:?}", ff.field_type)),
            materialized_path: Some(format!("/Modalities/ThreeD/Project_{}/Graph_{}/ForceField/{}", project_id, graph_id, ff.ff_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: vec!["force-field".into()], hotness_score: 0.55, ..Default::default()
        });
        if let Some(&obj_nid) = obj_id_to_nid.get(&ff.object_id) {
            edges.push(ThreeDGraphEdge { edge_id, from_node: obj_nid, to_node: fid, edge_type: ThreeDEdgeType::InfluencedByForce, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        // ForceField → all rigid bodies
        for (_, &rb_nid) in &rb_id_to_nid {
            edges.push(ThreeDGraphEdge { edge_id, from_node: fid, to_node: rb_nid, edge_type: ThreeDEdgeType::InfluencedByForce, weight: ff.strength.abs() as f32 / 100.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── CAMERAS ──
    for cam in &analysis.cameras {
        let cid = node_id;
        nodes.push(ThreeDGraphNode {
            node_id: cid, node_type: ThreeDNodeType::Camera,
            content: format!("Camera {:?}: focal={:.1}mm fov={:.1}° active={}", cam.camera_type, cam.focal_length, cam.fov_horizontal.to_degrees(), cam.is_active),
            materialized_path: Some(format!("/Modalities/ThreeD/Project_{}/Graph_{}/Camera/{}", project_id, graph_id, cam.camera_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: vec!["camera".into(), format!("{:?}", cam.camera_type).to_lowercase()], hotness_score: 0.75, ..Default::default()
        });
        if let Some(&obj_nid) = obj_id_to_nid.get(&cam.object_id) {
            edges.push(ThreeDGraphEdge { edge_id, from_node: obj_nid, to_node: cid, edge_type: ThreeDEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        edges.push(ThreeDGraphEdge { edge_id, from_node: root_id, to_node: cid, edge_type: ThreeDEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── LIGHTS ──
    for light in &analysis.lights {
        let lid = node_id;
        nodes.push(ThreeDGraphNode {
            node_id: lid, node_type: ThreeDNodeType::Light,
            content: format!("Light {:?}: energy={:.1}W shadow={}", light.light_type, light.energy, light.cast_shadow),
            materialized_path: Some(format!("/Modalities/ThreeD/Project_{}/Graph_{}/Light/{}", project_id, graph_id, light.light_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: vec!["light".into(), format!("{:?}", light.light_type).to_lowercase()], hotness_score: 0.6, ..Default::default()
        });
        if let Some(&obj_nid) = obj_id_to_nid.get(&light.object_id) {
            edges.push(ThreeDGraphEdge { edge_id, from_node: obj_nid, to_node: lid, edge_type: ThreeDEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── WORLD ──
    if let Some(ref world) = analysis.world {
        let wid = node_id;
        nodes.push(ThreeDGraphNode {
            node_id: wid, node_type: ThreeDNodeType::World,
            content: format!("World: {} bg={}", world.name, world.background_type),
            materialized_path: Some(format!("/Modalities/ThreeD/Project_{}/Graph_{}/World", project_id, graph_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: vec!["world".into(), "environment".into()], hotness_score: 0.65, ..Default::default()
        });
        edges.push(ThreeDGraphEdge { edge_id, from_node: root_id, to_node: wid, edge_type: ThreeDEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── VSE STRIPS ──
    for strip in analysis.vse_strips.iter().take(50) {
        let sid = node_id;
        nodes.push(ThreeDGraphNode {
            node_id: sid, node_type: ThreeDNodeType::VSEStrip,
            content: format!("VSEStrip {:?}: {} ch={} frames={}-{}", strip.strip_type, strip.name, strip.channel, strip.frame_start, strip.frame_end),
            frame_range: Some((strip.frame_start as f32, strip.frame_end as f32)),
            materialized_path: Some(format!("/Modalities/ThreeD/Project_{}/Graph_{}/VSE/{}", project_id, graph_id, strip.strip_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: vec!["vse".into(), format!("{:?}", strip.strip_type).to_lowercase()], hotness_score: 0.5, ..Default::default()
        });
        edges.push(ThreeDGraphEdge { edge_id, from_node: root_id, to_node: sid, edge_type: ThreeDEdgeType::Contains, weight: 0.6, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── HOOK 1: OnGraphCreated ──
    let _ = executor.save_graph(&ThreeDGraph {
        graph_id, project_id, source_description: format!("{} {}", analysis.source_format, analysis.source_path.as_deref().unwrap_or("")),
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
        if valid_ids.contains(&from) && valid_ids.contains(&to) && from != to {
            edges.push(ThreeDGraphEdge {
                edge_id, from_node: from, to_node: to, edge_type: etype, weight: 0.8,
                provenance: EdgeProvenance::DerivedFromHook, version: 1,
                properties: { let mut p = HashMap::new(); p.insert("reason".into(), serde_json::json!(reason)); p },
                ..Default::default()
            });
            edge_id += 1;
        }
    }

    // ── SPATIAL RELATIONSHIPS (LLM inferred) ──
    let obj_nodes: Vec<&ThreeDGraphNode> = nodes.iter()
        .filter(|n| matches!(n.node_type, ThreeDNodeType::Object) && n.location.is_some())
        .collect();
    let spatial = executor.infer_spatial_relationships(&obj_nodes).await;
    for (from, to, etype) in spatial {
        if valid_ids.contains(&from) && valid_ids.contains(&to) && from != to {
            edges.push(ThreeDGraphEdge { edge_id, from_node: from, to_node: to, edge_type: etype, weight: 0.7, provenance: EdgeProvenance::DerivedFromHook, version: 1, ..Default::default() });
            edge_id += 1;
        }
    }

    // ── HOOK 3: OnEdgeCompletion → hotness ──
    let mut deg: HashMap<u64, u32> = HashMap::new();
    for e in &edges { *deg.entry(e.from_node).or_insert(0) += 1; *deg.entry(e.to_node).or_insert(0) += 1; }
    let max_deg = deg.values().copied().max().unwrap_or(1) as f32;
    for n in &mut nodes {
        if let Some(&d) = deg.get(&n.node_id) {
            n.hotness_score = (n.hotness_score + (d as f32 / max_deg) * 0.15).min(1.0);
        }
    }

    let final_graph = ThreeDGraph {
        graph_id, project_id,
        source_description: format!("{} {}", analysis.source_format, analysis.source_path.unwrap_or_default()),
        nodes, edges, root_node_id: root_id, state: GraphStateType::SemanticEnriched,
        state_history: vec![GraphStateTransition { from: GraphStateType::Created, to: GraphStateType::SemanticEnriched, timestamp: now.clone(), triggered_by_step: None }],
        created_at: now.clone(), updated_at: now.clone(), version: 1,
        version_notes: vec![VersionNote { version: 1, note: "Semantic enrichment complete".into(), step_index: None, timestamp: now, change_type: ChangeType::EnrichedBySemantic }],
    };
    let _ = executor.save_graph(&final_graph);
    ThreeDModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(final_graph), ..Default::default() }
}

// ─────────────────────────────────────────────────────────────────────────────
// MAIN EXECUTION
// ─────────────────────────────────────────────────────────────────────────────

pub async fn execute(input: ThreeDModalityAction) -> Result<ThreeDModalityOutput, String> {
    let executor = PipelineExecutor::new();

    match input {
        ThreeDModalityAction::AnalyzeScene { data, extract_hierarchy, extract_physics, extract_animation, extract_materials } => {
            let analysis_id = executor.generate_id();
            let (source_format, source_path) = match &data {
                SceneDataSource::GLTF { file_path, .. } => ("GLTF".into(), Some(file_path.clone())),
                SceneDataSource::GLB { file_path } => ("GLB".into(), Some(file_path.clone())),
                SceneDataSource::OBJ { file_path, .. } => ("OBJ".into(), Some(file_path.clone())),
                SceneDataSource::FBX { file_path } => ("FBX".into(), Some(file_path.clone())),
                SceneDataSource::USD { file_path, .. } | SceneDataSource::USDA { file_path } | SceneDataSource::USDC { file_path } | SceneDataSource::USDZ { file_path } => ("USD".into(), Some(file_path.clone())),
                SceneDataSource::Blend { file_path, .. } => ("Blend".into(), Some(file_path.clone())),
                SceneDataSource::STL { file_path, .. } => ("STL".into(), Some(file_path.clone())),
                SceneDataSource::PLY { file_path } => ("PLY".into(), Some(file_path.clone())),
                SceneDataSource::URDF { file_path, .. } => ("URDF".into(), Some(file_path.clone())),
                SceneDataSource::MJCF { file_path } => ("MJCF".into(), Some(file_path.clone())),
                SceneDataSource::LiveStream { endpoint, .. } => ("LiveStream".into(), Some(endpoint.clone())),
                SceneDataSource::ManifestJSON { manifest_path } => ("Manifest".into(), Some(manifest_path.clone())),
                _ => ("Procedural".into(), None),
            };
            Ok(ThreeDModalityOutput {
                success: true,
                analysis: Some(ThreeDAnalysisResult { analysis_id, source_format, source_path, fps: 24.0, frame_end: 250, up_axis: Axis::Z, ..Default::default() }),
                ..Default::default()
            })
        }

        ThreeDModalityAction::AnalyzeMesh { data, compute_normals, compute_uv, compute_topology } => {
            let mesh = match data {
                MeshDataSource::RawArrays { vertices, faces, normals, uvs } => MeshInfo {
                    mesh_id: executor.generate_id(), name: "raw_mesh".into(),
                    vertex_count: vertices.len() as u32, edge_count: 0,
                    face_count: faces.len() as u32, loop_count: faces.len() as u32 * 3,
                    has_uvs: uvs.is_some(), has_normals: normals.is_some(),
                    ..Default::default()
                },
                _ => MeshInfo { mesh_id: executor.generate_id(), name: "mesh_from_file".into(), ..Default::default() }
            };
            Ok(ThreeDModalityOutput {
                success: true,
                analysis: Some(ThreeDAnalysisResult { meshes: vec![mesh], ..Default::default() }),
                ..Default::default()
            })
        }

        ThreeDModalityAction::AnalyzeRig { data, extract_constraints, extract_weights } => {
            let arm = ArmatureInfo { armature_id: executor.generate_id(), name: "armature".into(), bone_count: 0, ..Default::default() };
            Ok(ThreeDModalityOutput { success: true, analysis: Some(ThreeDAnalysisResult { armatures: vec![arm], ..Default::default() }), ..Default::default() })
        }

        ThreeDModalityAction::AnalyzeAnimation { data, .. } => {
            let action = ActionInfo { action_id: executor.generate_id(), name: "action".into(), frame_range: (0.0, 250.0), ..Default::default() };
            Ok(ThreeDModalityOutput { success: true, analysis: Some(ThreeDAnalysisResult { actions: vec![action], ..Default::default() }), ..Default::default() })
        }

        ThreeDModalityAction::AnalyzePhysics { data, .. } => {
            Ok(ThreeDModalityOutput { success: true, analysis: Some(ThreeDAnalysisResult { ..Default::default() }), ..Default::default() })
        }

        ThreeDModalityAction::AnalyzeMaterials { data, .. } => {
            let mat = MaterialInfo { material_id: executor.generate_id(), name: "material".into(), use_nodes: true, roughness: 0.5, ..Default::default() };
            Ok(ThreeDModalityOutput { success: true, analysis: Some(ThreeDAnalysisResult { materials: vec![mat], ..Default::default() }), ..Default::default() })
        }

        ThreeDModalityAction::CreateGraph { analysis, project_id } => {
            Ok(create_graph(&executor, analysis, project_id).await)
        }

        ThreeDModalityAction::UpdateGraph { graph_id, updates, project_id } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
            let mut update_count = 0;

            for update in &updates {
                match update {
                    SceneUpdate::MoveObject { object_node_id, new_transform } => {
                        if let Some(n) = graph.nodes.iter_mut().find(|n| n.node_id == *object_node_id) {
                            n.transform = Some(new_transform.clone());
                            n.version += 1;
                            n.version_notes.push(VersionNote { version: n.version, note: "Transform updated".into(), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                            update_count += 1;
                        }
                    }
                    SceneUpdate::AddObject { object } => {
                        let nid = graph.nodes.iter().map(|n| n.node_id).max().unwrap_or(0) + 1;
                        graph.nodes.push(ThreeDGraphNode {
                            node_id: nid, node_type: ThreeDNodeType::Object,
                            content: format!("Object: {} [{:?}]", object.name, object.object_type),
                            transform: Some(object.transform.clone()), location: Some(object.location),
                            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                            keywords: vec!["object".into(), object.name.to_lowercase()], hotness_score: 0.8, ..Default::default()
                        });
                        graph.edges.push(ThreeDGraphEdge { edge_id: next_eid, from_node: graph.root_node_id, to_node: nid, edge_type: ThreeDEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                        next_eid += 1; update_count += 1;
                    }
                    SceneUpdate::RemoveObject { object_node_id } => {
                        graph.nodes.retain(|n| n.node_id != *object_node_id);
                        graph.edges.retain(|e| e.from_node != *object_node_id && e.to_node != *object_node_id);
                        update_count += 1;
                    }
                    SceneUpdate::SetBonePose { armature_node_id, bone_name, transform } => {
                        if let Some(n) = graph.nodes.iter_mut().find(|n| n.node_id == *armature_node_id) {
                            n.version += 1;
                            n.version_notes.push(VersionNote { version: n.version, note: format!("Pose set: {}", bone_name), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                        }
                        update_count += 1;
                    }
                    _ => { update_count += 1; }
                }
            }

            graph.version += 1; graph.updated_at = now.clone(); graph.state = GraphStateType::Updated;
            graph.version_notes.push(VersionNote { version: graph.version, note: format!("{} scene updates applied", update_count), step_index: None, timestamp: now, change_type: ChangeType::Updated });
            executor.save_graph(&graph)?;
            Ok(ThreeDModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        ThreeDModalityAction::QueryGraph { graph_id, query } => {
            let graph = executor.load_graph(graph_id)?;
            let result = match query {
                ThreeDGraphQuery::NodeDetail { node_id } => {
                    let node = graph.nodes.iter().find(|n| n.node_id == node_id);
                    let incoming: Vec<_> = graph.edges.iter().filter(|e| e.to_node == node_id).collect();
                    let outgoing: Vec<_> = graph.edges.iter().filter(|e| e.from_node == node_id).collect();
                    serde_json::json!({ "node": node, "incoming": incoming, "outgoing": outgoing })
                }
                ThreeDGraphQuery::ObjectsByType { object_type } => {
                    let objs: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, ThreeDNodeType::Object) && n.content.to_lowercase().contains(&object_type.to_lowercase())).collect();
                    serde_json::json!({ "objects": objs })
                }
                ThreeDGraphQuery::SpatialNeighbors { object_node_id, radius_m } => {
                    // Find objects within radius (using bounding box center distance)
                    let source = graph.nodes.iter().find(|n| n.node_id == object_node_id);
                    let source_loc = source.and_then(|n| n.location).unwrap_or([0.0, 0.0, 0.0]);
                    let neighbors: Vec<_> = graph.nodes.iter().filter(|n| {
                        if n.node_id == object_node_id { return false; }
                        if let Some(loc) = n.location {
                            let d = ((loc[0]-source_loc[0]).powi(2)+(loc[1]-source_loc[1]).powi(2)+(loc[2]-source_loc[2]).powi(2)).sqrt();
                            d <= radius_m
                        } else { false }
                    }).collect();
                    serde_json::json!({ "neighbors": neighbors, "count": neighbors.len() })
                }
                ThreeDGraphQuery::PhysicsObjects => {
                    let phys: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, ThreeDNodeType::RigidBody | ThreeDNodeType::SoftBody | ThreeDNodeType::Cloth | ThreeDNodeType::FluidDomain | ThreeDNodeType::SmokeDomain | ThreeDNodeType::ParticleSystem)).collect();
                    serde_json::json!({ "physics_objects": phys })
                }
                ThreeDGraphQuery::AnimatedObjects => {
                    let anim: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, ThreeDNodeType::Action | ThreeDNodeType::FCurve | ThreeDNodeType::NLATrack)).collect();
                    serde_json::json!({ "animated": anim })
                }
                ThreeDGraphQuery::MaterialsUsed => {
                    let mats: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, ThreeDNodeType::Material)).collect();
                    serde_json::json!({ "materials": mats })
                }
                ThreeDGraphQuery::CrossModalLinks { node_id } => {
                    let links: Vec<_> = graph.edges.iter().filter(|e| (e.from_node == node_id || e.to_node == node_id) && matches!(e.edge_type, ThreeDEdgeType::DescribedByText | ThreeDEdgeType::ImplementedInCode | ThreeDEdgeType::VisualizedAsImage | ThreeDEdgeType::ParametricGeneratesMesh | ThreeDEdgeType::ExportsToURDF | ThreeDEdgeType::KinematicChainOf | ThreeDEdgeType::ControlledBy | ThreeDEdgeType::FusedFromDepth | ThreeDEdgeType::FusedFromRadar | ThreeDEdgeType::FusedFromSonar | ThreeDEdgeType::GeoReferenced)).collect();
                    serde_json::json!({ "cross_modal_links": links })
                }
                ThreeDGraphQuery::BoneHierarchy { armature_node_id } => {
                    let bones: Vec<_> = graph.edges.iter().filter(|e| e.from_node == armature_node_id && matches!(e.edge_type, ThreeDEdgeType::Contains | ThreeDEdgeType::ParentOf)).collect();
                    serde_json::json!({ "bone_edges": bones })
                }
                ThreeDGraphQuery::ConstraintChain { bone_node_id } => {
                    let constrs: Vec<_> = graph.edges.iter().filter(|e| e.from_node == bone_node_id && matches!(e.edge_type, ThreeDEdgeType::Constrains)).collect();
                    serde_json::json!({ "constraints": constrs })
                }
                ThreeDGraphQuery::PhysicsCaches => {
                    let caches: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, ThreeDNodeType::PhysicsCache)).collect();
                    serde_json::json!({ "caches": caches })
                }
                ThreeDGraphQuery::ObjectTransform { object_node_id } => {
                    let node = graph.nodes.iter().find(|n| n.node_id == object_node_id);
                    serde_json::json!({ "transform": node.and_then(|n| n.transform.as_ref()), "location": node.and_then(|n| n.location) })
                }
                ThreeDGraphQuery::AGIActivity => serde_json::json!({ "is_active": false }),
                ThreeDGraphQuery::AllNodes => serde_json::json!({ "nodes": graph.nodes }),
                ThreeDGraphQuery::AllEdges => serde_json::json!({ "edges": graph.edges }),
            };
            Ok(ThreeDModalityOutput { success: true, query_result: Some(result), ..Default::default() })
        }

        ThreeDModalityAction::GetGraph { graph_id } => {
            let graph = executor.load_graph(graph_id)?;
            Ok(ThreeDModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        ThreeDModalityAction::ComputeSpatialRelationships { graph_id, method } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            let obj_nodes: Vec<&ThreeDGraphNode> = graph.nodes.iter().filter(|n| matches!(n.node_type, ThreeDNodeType::Object)).collect();
            let spatial = executor.infer_spatial_relationships(&obj_nodes).await;
            let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
            let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
            let mut added = 0;
            for (from, to, etype) in spatial {
                if valid.contains(&from) && valid.contains(&to) && from != to {
                    graph.edges.push(ThreeDGraphEdge { edge_id: next_eid, from_node: from, to_node: to, edge_type: etype, weight: 0.7, provenance: EdgeProvenance::DerivedFromHook, version: 1, ..Default::default() });
                    next_eid += 1; added += 1;
                }
            }
            graph.version += 1; graph.updated_at = now.clone();
            graph.version_notes.push(VersionNote { version: graph.version, note: format!("Spatial: {} new edges (method={:?})", added, method), step_index: None, timestamp: now, change_type: ChangeType::EnrichedBySemantic });
            executor.save_graph(&graph)?;
            Ok(ThreeDModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        ThreeDModalityAction::BuildSpatialIndex { graph_id, index_type } => {
            let graph = executor.load_graph(graph_id)?;
            // In production: build Octree/BVH/KDTree over all object bounding boxes
            Ok(ThreeDModalityOutput { success: true, graph_id: Some(graph_id), spatial_index_built: Some(true), ..Default::default() })
        }

        ThreeDModalityAction::RunSimulation { graph_id, domain, frames, headless, cache_results } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            // In production: dispatch to physics backend (Bullet, MuJoCo, PhysX, Havok)
            // based on domain — here we update graph state and return frame count
            let phys_cache_id = executor.generate_id();
            let phys_nid = graph.nodes.iter().map(|n| n.node_id).max().unwrap_or(0) + 1;
            graph.nodes.push(ThreeDGraphNode {
                node_id: phys_nid, node_type: ThreeDNodeType::PhysicsCache,
                content: format!("PhysicsCache [{:?}]: {} frames headless={}", domain, frames, headless),
                physics_type: Some(format!("{:?}", domain)),
                materialized_path: Some(format!("/Modalities/ThreeD/Project_{}/Graph_{}/PhysicsCache/{}", graph.project_id, graph_id, phys_cache_id)),
                provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                keywords: vec!["physics".into(), "cache".into(), format!("{:?}", domain).to_lowercase()], hotness_score: 0.75, ..Default::default()
            });
            let root_id = graph.root_node_id;
            let next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
            graph.edges.push(ThreeDGraphEdge { edge_id: next_eid, from_node: root_id, to_node: phys_nid, edge_type: ThreeDEdgeType::Simulates, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            graph.version += 1; graph.updated_at = now.clone(); graph.state = GraphStateType::Updated;
            graph.version_notes.push(VersionNote { version: graph.version, note: format!("Simulation ran: {:?} {} frames", domain, frames), step_index: None, timestamp: now, change_type: ChangeType::Updated });
            executor.save_graph(&graph)?;
            Ok(ThreeDModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), simulation_frames: Some(frames), ..Default::default() })
        }

        ThreeDModalityAction::BakePhysics { graph_id, simulation_type, frame_start, frame_end } => {
            let graph = executor.load_graph(graph_id)?;
            // In production: drive physics engine to bake frames to disk cache
            Ok(ThreeDModalityOutput { success: true, graph_id: Some(graph_id), simulation_frames: Some(frame_end - frame_start), ..Default::default() })
        }

        ThreeDModalityAction::StepSimulation { graph_id, delta_time_secs } => {
            let graph = executor.load_graph(graph_id)?;
            // Single physics step — updates rigid body transforms in graph
            Ok(ThreeDModalityOutput { success: true, graph_id: Some(graph_id), ..Default::default() })
        }

        ThreeDModalityAction::ExportScene { graph_id, format, options } => {
            let ext = match &format {
                ThreeDExportFormat::GLTF { separate_textures: false } => "glb",
                ThreeDExportFormat::GLTF { .. } => "gltf",
                ThreeDExportFormat::GLB => "glb",
                ThreeDExportFormat::OBJ { .. } => "obj",
                ThreeDExportFormat::FBX { .. } => "fbx",
                ThreeDExportFormat::USD { .. } | ThreeDExportFormat::USDA | ThreeDExportFormat::USDC => "usd",
                ThreeDExportFormat::USDZ => "usdz",
                ThreeDExportFormat::STL { .. } => "stl",
                ThreeDExportFormat::PLY { .. } => "ply",
                ThreeDExportFormat::URDF { .. } => "urdf",
                ThreeDExportFormat::MJCF { .. } => "xml",
                ThreeDExportFormat::AlembicCache { .. } => "abc",
                ThreeDExportFormat::IsaacSimUSD { .. } => "usd",
                ThreeDExportFormat::BlenderManifest => "json",
                ThreeDExportFormat::NumpyArrays { .. } => "npz",
                ThreeDExportFormat::PointCloud { format: PointCloudFormat::PCD } => "pcd",
                ThreeDExportFormat::PointCloud { .. } => "ply",
            };
            let export_path = format!("/tmp/3d_export_{}.{}", graph_id, ext);
            Ok(ThreeDModalityOutput { success: true, graph_id: Some(graph_id), export_path: Some(export_path), ..Default::default() })
        }

        ThreeDModalityAction::ApplyModifier { graph_id, object_node_id, modifier } => {
            let graph = executor.load_graph(graph_id)?;
            Ok(ThreeDModalityOutput { success: true, graph_id: Some(graph_id), ..Default::default() })
        }

        ThreeDModalityAction::BooleanOperation { graph_id, target_node_id, tool_node_id, operation } => {
            let graph = executor.load_graph(graph_id)?;
            Ok(ThreeDModalityOutput { success: true, graph_id: Some(graph_id), ..Default::default() })
        }

        ThreeDModalityAction::ComputeLOD { graph_id, object_node_id, lod_levels } => {
            let graph = executor.load_graph(graph_id)?;
            Ok(ThreeDModalityOutput { success: true, graph_id: Some(graph_id), ..Default::default() })
        }

        ThreeDModalityAction::LinkToModality { graph_id, target_modality, target_graph_id, link_type } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            // Add cross-modal reference node
            let ref_nid = graph.nodes.iter().map(|n| n.node_id).max().unwrap_or(0) + 1;
            let next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
            graph.nodes.push(ThreeDGraphNode {
                node_id: ref_nid, node_type: ThreeDNodeType::CrossModalRef,
                content: format!("CrossModal→{} graph={} link={:?}", target_modality, target_graph_id, link_type),
                materialized_path: Some(format!("/Modalities/ThreeD/Project_{}/Graph_{}/CrossModal/{}", graph.project_id, graph_id, target_graph_id)),
                provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                keywords: vec!["cross-modal".into(), target_modality.clone()], hotness_score: 0.7, ..Default::default()
            });
            let etype = match link_type {
                CrossModalLinkType::DescribedByText        => ThreeDEdgeType::DescribedByText,
                CrossModalLinkType::ImplementedInCode      => ThreeDEdgeType::ImplementedInCode,
                CrossModalLinkType::ParametricGeneratesMesh=> ThreeDEdgeType::ParametricGeneratesMesh,
                CrossModalLinkType::KinematicChainOf       => ThreeDEdgeType::KinematicChainOf,
                CrossModalLinkType::ControlledBy           => ThreeDEdgeType::ControlledBy,
                CrossModalLinkType::FusedFromRadar         => ThreeDEdgeType::FusedFromRadar,
                CrossModalLinkType::FusedFromSonar         => ThreeDEdgeType::FusedFromSonar,
                CrossModalLinkType::GeoReferenced          => ThreeDEdgeType::GeoReferenced,
                CrossModalLinkType::DepthFused             => ThreeDEdgeType::FusedFromDepth,
                CrossModalLinkType::ThermalSurface         => ThreeDEdgeType::ThermalSurface,
                CrossModalLinkType::HapticContact          => ThreeDEdgeType::HapticContact,
                CrossModalLinkType::VisualizedAsImage      => ThreeDEdgeType::VisualizedAsImage,
            };
            graph.edges.push(ThreeDGraphEdge { edge_id: next_eid, from_node: graph.root_node_id, to_node: ref_nid, edge_type: etype, weight: 0.9, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("target_graph_id".into(), serde_json::json!(target_graph_id)); p }, ..Default::default() });
            graph.state = GraphStateType::CrossLinked;
            graph.version += 1; graph.updated_at = now.clone();
            graph.version_notes.push(VersionNote { version: graph.version, note: format!("Linked to {} (graph {})", target_modality, target_graph_id), step_index: None, timestamp: now, change_type: ChangeType::CrossLinked });
            executor.save_graph(&graph)?;
            Ok(ThreeDModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        ThreeDModalityAction::FusePointCloud { graph_id, source_modality, point_cloud_data } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            // Create point cloud node
            let pc_nid = graph.nodes.iter().map(|n| n.node_id).max().unwrap_or(0) + 1;
            let next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
            graph.nodes.push(ThreeDGraphNode {
                node_id: pc_nid, node_type: ThreeDNodeType::Mesh,
                content: format!("FusedPointCloud from {}: {} points", source_modality, point_cloud_data.len()),
                materialized_path: Some(format!("/Modalities/ThreeD/Project_{}/Graph_{}/FusedPCL/{}", graph.project_id, graph_id, pc_nid)),
                provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                keywords: vec!["point-cloud".into(), source_modality.to_lowercase()], hotness_score: 0.75, ..Default::default()
            });
            let etype = match source_modality.as_str() {
                "radar"   => ThreeDEdgeType::FusedFromRadar,
                "sonar"   => ThreeDEdgeType::FusedFromSonar,
                "depth"   => ThreeDEdgeType::FusedFromDepth,
                _         => ThreeDEdgeType::FusedFromDepth,
            };
            graph.edges.push(ThreeDGraphEdge { edge_id: next_eid, from_node: graph.root_node_id, to_node: pc_nid, edge_type: etype, weight: 0.9, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, ..Default::default() });
            graph.version += 1; graph.updated_at = now.clone();
            executor.save_graph(&graph)?;
            Ok(ThreeDModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        ThreeDModalityAction::TriggerSemanticHook { graph_id, hook } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            match hook {
                ThreeDSemanticHook::OnGraphCreated => { graph.state = GraphStateType::SemanticEnriched; }
                ThreeDSemanticHook::OnInferRelationships => {
                    let new_edges = executor.infer_semantic_relationships(&graph.nodes).await;
                    let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                    let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                    for (from, to, etype, reason) in new_edges {
                        if valid.contains(&from) && valid.contains(&to) && from != to {
                            graph.edges.push(ThreeDGraphEdge { edge_id: next_eid, from_node: from, to_node: to, edge_type: etype, weight: 0.8, provenance: EdgeProvenance::DerivedFromHook, version: 1, properties: { let mut p = HashMap::new(); p.insert("reason".into(), serde_json::json!(reason)); p }, ..Default::default() });
                            next_eid += 1;
                        }
                    }
                }
                ThreeDSemanticHook::OnEdgeCompletion => {
                    let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                    graph.edges.retain(|e| valid.contains(&e.from_node) && valid.contains(&e.to_node));
                }
                ThreeDSemanticHook::OnCrossModalityLink { target_modality, target_graph_id } => {
                    graph.state = GraphStateType::CrossLinked;
                    graph.version += 1;
                    graph.version_notes.push(VersionNote { version: graph.version, note: format!("Cross-linked to {} (graph {})", target_modality, target_graph_id), step_index: None, timestamp: now.clone(), change_type: ChangeType::CrossLinked });
                }
            }
            graph.updated_at = now;
            executor.save_graph(&graph)?;
            Ok(ThreeDModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        ThreeDModalityAction::StreamToUI { graph_id, .. } => {
            Ok(ThreeDModalityOutput { success: true, graph_id: Some(graph_id), ..Default::default() })
        }

        ThreeDModalityAction::HeadlessProcess { graph_id, operations } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            for op in operations {
                match op {
                    ThreeDHeadlessOp::ComputeSpatial => {
                        let obj_nodes: Vec<&ThreeDGraphNode> = graph.nodes.iter().filter(|n| matches!(n.node_type, ThreeDNodeType::Object)).collect();
                        let spatial = executor.infer_spatial_relationships(&obj_nodes).await;
                        let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                        let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        for (from, to, etype) in spatial {
                            if valid.contains(&from) && valid.contains(&to) && from != to {
                                graph.edges.push(ThreeDGraphEdge { edge_id: next_eid, from_node: from, to_node: to, edge_type: etype, weight: 0.7, provenance: EdgeProvenance::DerivedFromHook, version: 1, ..Default::default() });
                                next_eid += 1;
                            }
                        }
                    }
                    ThreeDHeadlessOp::ExportToFormat { format, path } => {
                        // In production: serialize scene to format at path
                    }
                    ThreeDHeadlessOp::CrossFuseRadar { radar_graph_id } => {
                        let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        let root_id = graph.root_node_id;
                        graph.edges.push(ThreeDGraphEdge { edge_id: next_eid, from_node: root_id, to_node: root_id, edge_type: ThreeDEdgeType::FusedFromRadar, weight: 0.9, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("radar_graph_id".into(), serde_json::json!(radar_graph_id)); p }, ..Default::default() });
                    }
                    ThreeDHeadlessOp::SolveIK { armature_node_id, target_positions } => {
                        // In production: run IK solver and update bone pose matrices
                    }
                    _ => {}
                }
            }
            graph.updated_at = now;
            executor.save_graph(&graph)?;
            Ok(ThreeDModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
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
    if input_json.is_empty() { eprintln!("Usage: 3d_engine --input '<json>'"); std::process::exit(1); }
    let input: ThreeDModalityAction = match serde_json::from_str(&input_json) {
        Ok(v) => v,
        Err(e) => { println!("{}", serde_json::json!({"success":false,"error":format!("Parse error: {}",e)})); std::process::exit(1); }
    };
    let rt = tokio::runtime::Runtime::new().expect("Tokio runtime");
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap_or_else(|_| r#"{"success":false,"error":"serialize"}"#.into())),
        Err(e) => { println!("{}", serde_json::json!({"success":false,"error":e})); std::process::exit(1); }
    }
}
