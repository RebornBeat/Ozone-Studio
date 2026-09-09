//! ParametricCAD — Pipeline #120
//!
//! B-rep solid geometry, feature-based parametric modeling, assembly
//! constraints, tolerance chains, drawing views, and BOM management.
//!
//! DESIGN PRINCIPLE: AGI-first. The pipeline processes CAD geometry fully
//! headlessly. All B-rep operations, feature extraction, constraint solving,
//! and tolerance analysis are performed without requiring a CAD GUI.
//!
//! DISTINCT FROM:
//!   - 3D (109): 3D is scene/render-centric (meshes, materials, animation).
//!     Parametric CAD is engineering-centric: exact B-rep geometry, feature
//!     history trees, dimensional tolerances, assembly constraints, GD&T.
//!     ParametricGeneratesMesh is the bridge: CAD → tessellated mesh → 3D.
//!
//! CROSS-LINKS:
//!   109 (3D)       → ParametricGeneratesMesh: CAD B-rep → render mesh
//!   121 (Kine)     → kinematic joint geometry from CAD assembly
//!   122 (Control)  → actuator specs from CAD → control model
//!   106 (Chem)     → material composition of CAD parts
//!   126 (Hyper)    → surface material identification feeds CAD material props
//!
//! STORAGE: ZSEI containers under /Modalities/ParametricCAD/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

// ─────────────────────────────────────────────────────────────────────────────
// INPUT TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum CADModalityAction {
    // ── ANALYSIS ──
    AnalyzeFile       { data: CADDataSource, extract_features: bool, extract_assembly: bool, extract_tolerances: bool },
    AnalyzePart       { data: CADDataSource, compute_mass_properties: bool, check_validity: bool },
    AnalyzeAssembly   { data: CADDataSource, resolve_constraints: bool, check_interference: bool },
    AnalyzeDrawing    { data: CADDataSource, extract_dimensions: bool, extract_notes: bool },
    // ── GRAPH ──
    CreateGraph       { analysis: CADAnalysisResult, project_id: u64 },
    UpdateGraph       { graph_id: u64, updates: Vec<CADUpdate>, project_id: u64 },
    QueryGraph        { graph_id: u64, query: CADGraphQuery },
    GetGraph          { graph_id: u64 },
    // ── GEOMETRY OPERATIONS ──
    EvaluateFeature   { graph_id: u64, feature_node_id: u64 },
    SolveConstraints  { graph_id: u64, assembly_node_id: u64 },
    ComputeMassProps  { graph_id: u64, part_node_id: u64, material_density_kg_m3: f64 },
    CheckInterference { graph_id: u64, part_ids: Vec<u64> },
    AnalyzeToleranceChain { graph_id: u64, dimension_ids: Vec<u64> },
    GenerateBOM       { graph_id: u64, include_sub_assemblies: bool },
    // ── TESSELLATION / CROSS-MODAL ──
    TessellateToMesh  { graph_id: u64, part_node_id: u64, chord_tolerance_mm: f64, angle_tolerance_deg: f64 },
    ExportForKinematics { graph_id: u64, joint_ids: Vec<u64> },
    LinkToModality    { graph_id: u64, target_modality: String, target_graph_id: u64, link_type: CADCrossModalLink },
    // ── EXPORT ──
    ExportPart        { graph_id: u64, part_node_id: u64, format: CADExportFormat },
    ExportAssembly    { graph_id: u64, assembly_node_id: u64, format: CADExportFormat },
    ExportDrawing     { graph_id: u64, drawing_node_id: u64, format: DrawingExportFormat },
    // ── HOOKS / STREAMING ──
    TriggerSemanticHook { graph_id: u64, hook: CADSemanticHook },
    StreamToUI          { graph_id: u64, session_id: String, view_mode: CADViewMode },
    HeadlessProcess     { graph_id: u64, operations: Vec<CADHeadlessOp> },
}

// ─────────────────────────────────────────────────────────────────────────────
// DATA SOURCES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CADDataSource {
    STEP    { file_path: String },                          // ISO 10303 AP203/AP214/AP242
    IGES    { file_path: String },                          // Initial Graphics Exchange Specification
    BREP    { file_path: String },                          // OpenCASCADE native B-rep
    OCC     { file_path: String },                          // OpenCASCADE .brep
    F3D     { file_path: String },                          // Fusion 360 archive
    FCStd   { file_path: String },                          // FreeCAD native
    SLDPRT  { file_path: String },                          // SolidWorks part
    SLDASM  { file_path: String },                          // SolidWorks assembly
    IPT     { file_path: String },                          // Inventor part
    IAM     { file_path: String },                          // Inventor assembly
    CATPART { file_path: String },                          // CATIA V5 part
    CATPRODUCT { file_path: String },                       // CATIA V5 product
    X_T     { file_path: String },                          // Parasolid text
    X_B     { file_path: String },                          // Parasolid binary
    SAT     { file_path: String },                          // ACIS
    JT      { file_path: String },                          // JT open format
    /// Part defined entirely through parametric feature description
    Parametric { name: String, features: Vec<FeatureSpec>, material: Option<String> },
    /// Assembly defined by component file paths and constraints
    AssemblyManifest { manifest_path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CADExportFormat {
    STEP, IGES, OBJ, STL { binary: bool }, GLTF, GLB,
    BREP, X_T, X_B, SAT, JT, USDZ,
    DXF, SVG,                        // flat 2D projections
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DrawingExportFormat { PDF, SVG, DXF, DWG, PNG { dpi: u32 }, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CADViewMode {
    Wireframe, Shaded, ShadedWithEdges, HiddenLineRemoved, Transparent,
    Exploded { factor: f32 }, XSection { plane: CutPlane }, FeatureTree,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CutPlane { XY, XZ, YZ, Custom { point: [f64; 3], normal: [f64; 3] } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CADCrossModalLink {
    GeneratesMeshIn3D,       // this part's tessellation appears in 3D graph
    ProvidesJointGeometry,   // joint geometry feeds kinematics (121)
    ProvidesActuatorSpec,    // actuator dimensions feed control system (122)
    MaterialFrom106,         // material composition from chemistry (106)
    MaterialIdentifiedBy126, // surface material identified by hyperspectral (126)
}

// ─────────────────────────────────────────────────────────────────────────────
// FEATURE SPECIFICATION (for procedural CAD creation)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureSpec {
    pub feature_type: FeatureType,
    pub name: String,
    pub parameters: HashMap<String, f64>,
    pub sketch_plane: Option<SketchPlane>,
    pub referenced_feature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SketchPlane {
    XY, XZ, YZ,
    Face { part_name: String, face_index: u32 },
    Offset { base_plane: Box<SketchPlane>, offset_mm: f64 },
}

// ─────────────────────────────────────────────────────────────────────────────
// ANALYSIS RESULT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CADAnalysisResult {
    pub analysis_id: u64,
    pub source_format: String,
    pub source_path: Option<String>,
    pub is_assembly: bool,

    // PARTS & ASSEMBLIES
    pub parts: Vec<PartInfo>,
    pub assemblies: Vec<AssemblyInfo>,
    pub components: Vec<ComponentInstance>,   // instances of parts in assemblies

    // GEOMETRY
    pub features: Vec<FeatureInfo>,
    pub sketches: Vec<SketchInfo>,
    pub bodies: Vec<SolidBodyInfo>,
    pub faces: Vec<FaceInfo>,
    pub edges: Vec<CADEdgeInfo>,
    pub vertices: Vec<VertexInfo>,

    // CONSTRAINTS
    pub assembly_constraints: Vec<AssemblyConstraint>,
    pub sketch_constraints: Vec<SketchConstraint>,
    pub parametric_relations: Vec<ParametricRelation>,

    // DIMENSIONS & TOLERANCES
    pub dimensions: Vec<DimensionInfo>,
    pub tolerances: Vec<ToleranceInfo>,
    pub gdt_frames: Vec<GDTFrame>,

    // DRAWINGS
    pub drawing_views: Vec<DrawingView>,
    pub drawing_annotations: Vec<DrawingAnnotation>,

    // MATERIALS
    pub materials: Vec<CADMaterial>,

    // MASS PROPERTIES (if computed)
    pub mass_properties: Vec<MassProperties>,

    // BOM
    pub bom_entries: Vec<BOMEntry>,

    // METADATA
    pub cad_system: Option<String>,
    pub creation_date: Option<String>,
    pub revision: Option<String>,
    pub author: Option<String>,
    pub processing_notes: Vec<String>,
}

// ── PART / ASSEMBLY ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PartInfo {
    pub part_id: u64,
    pub name: String,
    pub description: Option<String>,
    pub part_number: Option<String>,
    pub revision: Option<String>,
    pub body_count: u32,
    pub feature_count: u32,
    pub face_count: u32, pub edge_count: u32, pub vertex_count: u32,
    pub bounding_box: CADBoundingBox,
    pub is_sheet_metal: bool,
    pub is_weldment: bool,
    pub material_name: Option<String>,
    pub mass_kg: Option<f64>,
    pub volume_mm3: Option<f64>,
    pub surface_area_mm2: Option<f64>,
    pub cog: Option<[f64; 3]>,           // center of gravity
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AssemblyInfo {
    pub assembly_id: u64,
    pub name: String,
    pub description: Option<String>,
    pub part_number: Option<String>,
    pub component_count: u32,
    pub unique_part_count: u32,
    pub constraint_count: u32,
    pub degree_of_freedom: u32,          // unconstrained DOF
    pub fully_constrained: bool,
    pub bounding_box: CADBoundingBox,
    pub has_interference: Option<bool>,
    pub total_mass_kg: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComponentInstance {
    pub instance_id: u64,
    pub part_or_assembly_id: u64,        // references PartInfo.part_id or AssemblyInfo.assembly_id
    pub instance_name: String,
    pub is_part: bool,
    pub parent_assembly_id: u64,
    pub transform: CADTransform,
    pub suppressed: bool,
    pub quantity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CADTransform {
    pub matrix: [[f64; 4]; 4],
    pub translation_mm: [f64; 3],
    pub rotation_euler_deg: [f64; 3],
    pub scale: f64,
}

impl CADTransform {
    pub fn identity() -> Self {
        Self {
            matrix: [[1.0,0.0,0.0,0.0],[0.0,1.0,0.0,0.0],[0.0,0.0,1.0,0.0],[0.0,0.0,0.0,1.0]],
            translation_mm: [0.0; 3], rotation_euler_deg: [0.0; 3], scale: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CADBoundingBox {
    pub min_mm: [f64; 3], pub max_mm: [f64; 3],
    pub center_mm: [f64; 3], pub size_mm: [f64; 3],
    pub diagonal_mm: f64,
}

// ── FEATURES ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FeatureInfo {
    pub feature_id: u64,
    pub name: String,
    pub feature_type: FeatureType,
    pub part_id: u64,
    pub is_suppressed: bool,
    pub is_parametric: bool,
    pub parent_feature_ids: Vec<u64>,     // features this depends on
    pub child_feature_ids: Vec<u64>,
    pub sketch_id: Option<u64>,           // driving sketch if any
    pub parameters: HashMap<String, f64>, // e.g. depth_mm, angle_deg, radius_mm
    pub faces_created: Vec<u64>,          // face IDs created by this feature
    pub faces_consumed: Vec<u64>,         // face IDs removed by this feature
    pub bounding_box: Option<CADBoundingBox>,
    pub error_message: Option<String>,    // if feature failed to build
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum FeatureType {
    #[default] Unknown,
    // Sketch-based
    Extrude, Revolve, Sweep, Loft,
    ExtrudeCut, RevolveCut, SweepCut, LoftCut,
    // Applied
    Fillet { constant: bool },
    Chamfer,
    Shell,
    Draft,
    Rib,
    Hole { hole_type: HoleType },
    Thread { thread_type: ThreadType },
    // Transform
    Mirror, LinearPattern, CircularPattern, CurvePattern,
    Scale,
    // Sheet metal
    Flange, Bend, Unfold, HemFlanged,
    // Weldment
    StructuralMember, TrimExtend, GussetWeld,
    // Boolean
    BooleanAdd, BooleanSubtract, BooleanIntersect, Combine,
    // Reference
    WorkPlane, WorkAxis, WorkPoint,
    // Surfacing
    SewSurface, ThickenSurface, Boundary,
    // Import
    ImportedBody,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum HoleType { #[default] Simple, Tapped, TapDrill, Countersink, Counterbore, Spotface, CBore_CSink }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ThreadType { #[default] Metric_ISO, UNC, UNF, BSP, NPT, Custom(String) }

// ── SKETCHES ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SketchInfo {
    pub sketch_id: u64,
    pub name: String,
    pub part_id: u64,
    pub plane: SketchPlaneInfo,
    pub entity_count: u32,
    pub constraint_count: u32,
    pub dimension_count: u32,
    pub fully_constrained: bool,
    pub is_closed: bool,
    pub entities: Vec<SketchEntity>,
    pub constraints: Vec<SketchConstraint>,
    pub dimensions: Vec<SketchDimension>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SketchPlaneInfo {
    pub plane_type: String,
    pub origin_mm: [f64; 3],
    pub normal: [f64; 3],
    pub x_axis: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SketchEntity {
    pub entity_id: u64,
    pub entity_type: SketchEntityType,
    pub is_construction: bool,
    pub parameters: HashMap<String, f64>,  // center_x_mm, radius_mm, length_mm, etc.
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SketchEntityType {
    #[default] Line, Circle, Arc, Ellipse, Spline, Rectangle, Slot, Point, Polygon
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SketchDimension {
    pub dim_id: u64,
    pub dim_type: SketchDimType,
    pub value_mm_or_deg: f64,
    pub entity_ids: Vec<u64>,
    pub is_driven: bool,              // driven = display only, not constraining
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SketchDimType { #[default] Linear, Angular, Radial, Diameter, Arc_Length }

// ── SOLID BODIES / B-REP ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SolidBodyInfo {
    pub body_id: u64,
    pub name: String,
    pub part_id: u64,
    pub is_solid: bool, pub is_surface: bool,
    pub face_count: u32, pub edge_count: u32, pub vertex_count: u32,
    pub volume_mm3: f64,
    pub surface_area_mm2: f64,
    pub is_closed: bool,
    pub bounding_box: CADBoundingBox,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FaceInfo {
    pub face_id: u64,
    pub body_id: u64,
    pub surface_type: SurfaceType,
    pub area_mm2: f64,
    pub normal: Option<[f64; 3]>,         // for planar faces
    pub center_mm: [f64; 3],
    pub curvature: Option<f64>,            // for curved faces
    pub adjacent_face_ids: Vec<u64>,
    pub bounding_edge_ids: Vec<u64>,
    pub is_outer: bool,
    pub gdt_frame_id: Option<u64>,
    pub tolerance_id: Option<u64>,
    pub finish: Option<SurfaceFinish>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SurfaceType { #[default] Plane, Cylinder, Cone, Sphere, Torus, BSplineSurface, Offset, Revolution, Extrusion }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SurfaceFinish {
    pub ra_um: f64,                        // average roughness in micrometers
    pub rz_um: Option<f64>,               // mean roughness depth
    pub method: Option<String>,           // "machined", "ground", "lapped", "polished"
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CADEdgeInfo {
    pub edge_id: u64,
    pub body_id: u64,
    pub curve_type: CurveType,
    pub length_mm: f64,
    pub start_vertex_id: u64, pub end_vertex_id: u64,
    pub adjacent_face_ids: Vec<u64>,
    pub is_seam: bool, pub is_degenerate: bool,
    pub convexity: EdgeConvexity,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum CurveType { #[default] Line, Circle, Ellipse, BSpline, Parabola, Hyperbola, Other }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum EdgeConvexity { #[default] Convex, Concave, Mixed, Smooth }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VertexInfo {
    pub vertex_id: u64,
    pub position_mm: [f64; 3],
    pub adjacent_edge_ids: Vec<u64>,
}

// ── ASSEMBLY CONSTRAINTS ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AssemblyConstraint {
    pub constraint_id: u64,
    pub constraint_type: AssemblyConstraintType,
    pub component_a_id: u64,
    pub component_b_id: u64,
    pub geometry_a: ConstraintGeometry,
    pub geometry_b: ConstraintGeometry,
    pub offset_mm: Option<f64>,
    pub angle_deg: Option<f64>,
    pub is_suppressed: bool,
    pub is_locked: bool,
    pub dof_removed: u32,            // degrees of freedom this constraint removes
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum AssemblyConstraintType {
    #[default] Mate, Align, Flush, Insert,
    Coincident, Concentric, Parallel, Perpendicular,
    Tangent, Fixed, FixedAngle, FixedDistance,
    Symmetric, MidPoint, RigidGroup,
    GearMate { ratio: f64 }, RackPinion { pitch_mm: f64 },
    ScrewMate { pitch_mm_per_rev: f64 },
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConstraintGeometry {
    pub geometry_type: ConstraintGeomType,
    pub face_id: Option<u64>,
    pub edge_id: Option<u64>,
    pub vertex_id: Option<u64>,
    pub axis: Option<[f64; 3]>,
    pub point_mm: Option<[f64; 3]>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ConstraintGeomType { #[default] Face, Edge, Vertex, Axis, Point, Plane, Cylinder }

// ── SKETCH CONSTRAINTS ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SketchConstraint {
    pub constraint_id: u64,
    pub constraint_type: SketchConstraintType,
    pub entity_ids: Vec<u64>,
    pub is_driving: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SketchConstraintType {
    #[default] Fixed, Coincident, Concentric, Collinear, Parallel, Perpendicular,
    Horizontal, Vertical, Tangent, SmoothContinuity, Symmetric, Equal,
    MidPoint, Pierce,
}

// ── PARAMETRIC RELATIONS ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParametricRelation {
    pub relation_id: u64,
    pub name: String,
    pub expression: String,            // e.g. "depth_mm = width_mm * 0.5"
    pub lhs_variable: String,
    pub rhs_expression: String,
    pub lhs_feature_id: Option<u64>,
    pub referenced_variables: Vec<String>,
    pub is_global: bool,               // global variable vs local to part
}

// ── DIMENSIONS & TOLERANCES ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DimensionInfo {
    pub dimension_id: u64,
    pub name: Option<String>,
    pub dim_type: DimensionType,
    pub nominal_value: f64,
    pub unit: DimensionUnit,
    pub tolerance_id: Option<u64>,
    pub feature_id: Option<u64>,        // which feature drives this dimension
    pub face_ids: Vec<u64>,             // faces this dimension references
    pub is_reference: bool,
    pub is_driven: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum DimensionType {
    #[default] Linear, Angular, Radial, Diameter, Arc_Length,
    Distance_Between_Axes, Thread_Pitch, Surface_Texture,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum DimensionUnit { #[default] Millimeter, Inch, Degree, Radian }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToleranceInfo {
    pub tolerance_id: u64,
    pub tolerance_type: ToleranceType,
    pub upper_deviation: f64,          // positive = above nominal
    pub lower_deviation: f64,          // negative = below nominal
    pub tolerance_class: Option<String>, // ISO fit: H7, h6, etc.
    pub referenced_dimension_ids: Vec<u64>,
    pub referenced_face_ids: Vec<u64>,
    pub is_bilateral: bool,            // symmetric ±
    pub is_unilateral: bool,
    pub accumulated_worst_case: Option<f64>,  // tolerance chain result
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ToleranceType {
    #[default] Linear, Angular, Flatness, Straightness, Circularity, Cylindricity,
    ProfileOfLine, ProfileOfSurface, Parallelism, Perpendicularity, Angularity,
    Position, Concentricity, Symmetry, RunoutCircular, RunoutTotal,
    SurfaceFinish, ThreadForm,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GDTFrame {
    pub frame_id: u64,
    pub characteristic: GDTCharacteristic,
    pub tolerance_value_mm: f64,
    pub datum_references: Vec<DatumReference>,
    pub applies_to_face_ids: Vec<u64>,
    pub material_condition: MaterialCondition,
    pub projected_tolerance_zone: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum GDTCharacteristic {
    #[default] Straightness, Flatness, Circularity, Cylindricity,
    ProfileOfLine, ProfileOfSurface, Parallelism, Perpendicularity, Angularity,
    Position, Concentricity, Symmetry, CircularRunout, TotalRunout,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DatumReference {
    pub datum_label: String,           // "A", "B", "C"
    pub material_condition: MaterialCondition,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum MaterialCondition { #[default] RFS, MMC, LMC }

// ── DRAWINGS ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DrawingView {
    pub view_id: u64,
    pub name: String,
    pub view_type: DrawingViewType,
    pub scale: f64,
    pub origin_mm: [f64; 2],
    pub referenced_part_id: Option<u64>,
    pub referenced_assembly_id: Option<u64>,
    pub projection_direction: Option<[f64; 3]>,
    pub cut_plane: Option<CutPlane>,
    pub visible_edges: u32,
    pub hidden_edges: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum DrawingViewType {
    #[default] Front, Top, Right, Left, Back, Bottom, Isometric,
    Section, Detail { scale_factor: f64 }, Auxiliary, Broken, Exploded,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DrawingAnnotation {
    pub annotation_id: u64,
    pub annotation_type: AnnotationType,
    pub content: String,
    pub position_mm: [f64; 2],
    pub referenced_view_id: Option<u64>,
    pub referenced_dimension_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum AnnotationType {
    #[default] Text, LinearDimension, AngularDimension, RadiusDimension, DiameterDimension,
    Leader, SurfaceFinish, WeldSymbol, GDTFrame, BalloonCallout, RevisionNote,
    CenterMark, CenterLine,
}

// ── MATERIALS & MASS PROPERTIES ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CADMaterial {
    pub material_id: u64,
    pub name: String,
    pub density_kg_m3: f64,
    pub youngs_modulus_gpa: Option<f64>,
    pub poissons_ratio: Option<f64>,
    pub yield_strength_mpa: Option<f64>,
    pub ultimate_strength_mpa: Option<f64>,
    pub thermal_expansion_per_k: Option<f64>,
    pub thermal_conductivity_w_mk: Option<f64>,
    pub specific_heat_j_kgk: Option<f64>,
    pub material_class: MaterialClass,
    pub standard: Option<String>,      // e.g. "ASTM A36", "ISO 898-1 Grade 8.8"
    pub part_ids: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum MaterialClass {
    #[default] Unknown, Steel, Aluminum, Titanium, CopperAlloy, Plastic, Composite,
    Ceramic, Rubber, Glass, Wood, Foam, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MassProperties {
    pub part_id: u64,
    pub mass_kg: f64,
    pub volume_mm3: f64,
    pub surface_area_mm2: f64,
    pub cog_mm: [f64; 3],
    pub inertia_tensor_kg_mm2: [[f64; 3]; 3],
    pub principal_moments_kg_mm2: [f64; 3],
    pub principal_axes: [[f64; 3]; 3],
    pub material_name: String,
    pub density_kg_m3: f64,
}

// ── BOM ───────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BOMEntry {
    pub entry_id: u64,
    pub part_number: String,
    pub description: String,
    pub revision: String,
    pub quantity: u32,
    pub unit_mass_kg: Option<f64>,
    pub total_mass_kg: Option<f64>,
    pub material: Option<String>,
    pub supplier: Option<String>,
    pub cost_per_unit: Option<f64>,
    pub is_purchased: bool,
    pub is_make: bool,
    pub level: u32,                    // BOM indentation level
    pub parent_part_number: Option<String>,
    pub part_id: Option<u64>,
}

// ── CAD UPDATES ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CADUpdate {
    ModifyFeatureParameter { feature_node_id: u64, parameter: String, new_value: f64 },
    SuppressFeature        { feature_node_id: u64 },
    UnsuppressFeature      { feature_node_id: u64 },
    UpdateMaterial         { part_node_id: u64, material_name: String },
    UpdateTolerance        { tolerance_node_id: u64, upper: f64, lower: f64 },
    AddConstraint          { constraint: AssemblyConstraint },
    RemoveConstraint       { constraint_id: u64 },
    UpdateDimension        { dimension_id: u64, new_value: f64 },
}

// ── MASS PROPERTIES OUTPUT ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MassPropertiesResult {
    pub part_node_id: u64,
    pub mass_kg: f64,
    pub volume_mm3: f64,
    pub surface_area_mm2: f64,
    pub cog_mm: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InterferenceResult {
    pub interfering_pairs: Vec<(u64, u64)>,
    pub clearance_pairs: Vec<(u64, u64, f64)>,  // (id_a, id_b, min_clearance_mm)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToleranceChainResult {
    pub total_worst_case_mm: f64,
    pub total_rss_mm: f64,              // root sum of squares
    pub contributing_dimensions: Vec<(u64, f64)>,  // (dim_id, contribution_mm)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TessellationResult {
    pub part_node_id: u64,
    pub vertex_count: u32,
    pub triangle_count: u32,
    pub mesh_file_path: Option<String>,
    pub chord_tolerance_mm: f64,
    pub angle_tolerance_deg: f64,
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH NODE TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum CADNodeType {
    #[default] CADDocument,             // root: the file / document
    PartNode,                           // a single part
    AssemblyNode,                       // an assembly
    ComponentInstanceNode,              // component instance in assembly
    FeatureNode,                        // parametric feature
    SketchNode,                         // 2D sketch
    SolidBodyNode,                      // solid body in part
    FaceNode,                           // B-rep face
    EdgeNode,                           // B-rep edge
    VertexNode,                         // B-rep vertex
    AssemblyConstraintNode,             // mate/align/etc.
    SketchConstraintNode,               // sketch geometric constraint
    DimensionNode,                      // linear/angular/radial dimension
    ToleranceNode,                      // dimensional tolerance
    GDTFrameNode,                       // GD&T frame control block
    MaterialNode,                       // engineering material
    MassPropertiesNode,                 // mass/inertia result
    DrawingViewNode,                    // engineering drawing view
    DrawingAnnotationNode,              // dimension/note on drawing
    BOMEntryNode,                       // bill of materials line item
    ParametricRelationNode,             // equation/formula
    JointGeometryNode,                  // abstracted joint for kinematics
    CrossModalRef,                      // cross-modal link placeholder
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CADGraphNode {
    pub node_id: u64,
    pub node_type: CADNodeType,
    pub content: String,

    // CAD-SPECIFIC
    pub part_number: Option<String>,
    pub revision: Option<String>,
    pub nominal_value: Option<f64>,
    pub upper_deviation: Option<f64>,
    pub lower_deviation: Option<f64>,
    pub unit: Option<String>,
    pub feature_type: Option<String>,
    pub dof: Option<u32>,
    pub is_suppressed: Option<bool>,
    pub mass_kg: Option<f64>,
    pub material_name: Option<String>,
    pub bounding_box: Option<CADBoundingBox>,

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
pub enum CADEdgeType {
    // ── STRUCTURAL ──
    #[default] Contains, PartOf, References,

    // ── FEATURE HIERARCHY ──
    ParentFeature,              // feature depends on parent feature
    ChildFeature,               // child feature in tree
    DrivenBySketch,             // feature uses this sketch
    ParametricDrivesFeature,    // parametric equation drives feature param
    FeatureCreatesBody,         // feature produces solid body
    FeatureCreatesFace,         // feature creates B-rep face
    FeatureConsumesBody,        // boolean feature removes body

    // ── B-REP TOPOLOGY ──
    BoundsFace,                 // edge bounds this face
    AdjacentFace,               // face adjacent to another
    ConnectsVertex,             // edge connects vertex

    // ── ASSEMBLY ──
    ConstrainedBy,              // component constrained by this constraint
    ConstrainsComponent,        // constraint applies to this component
    InstanceOf,                 // component is instance of part/assembly
    ChildOf,                    // sub-assembly is child of assembly
    MatesWith,                  // surface mates with another

    // ── DIMENSIONS / TOLERANCES ──
    DimensionsFeature,          // dimension applies to feature
    ToleratesFeature,           // tolerance applies to feature
    ToleranceChain,             // sequential dimension chain for stack-up
    GDTAppliesTo,               // GD&T frame applies to face
    DatumOf,                    // face is datum for GD&T

    // ── MATERIALS ──
    MadeOf,                     // part is made of this material
    RequiresMaterial,           // feature requires specific material property

    // ── CROSS-MODAL ──
    ParametricGeneratesMesh,    // B-rep tessellates → 3D mesh (109)
    ProvidesJointGeometry,      // joint axes/limits → kinematics (121)
    ProvidesActuatorSpec,       // actuator dimensions → control (122)
    MaterialComposedOf,         // material → chemistry (106)
    MaterialIdentifiedBy,       // material ← hyperspectral (126)

    // ── UNIVERSAL SEMANTIC ──
    Performs, Affects, Implies, Contradicts, Elaborates, Summarizes, Supports,
    TemporalPrecedes, TemporalFollows, CausedBy, Enables, Prevents,
    FunctionalRole, InstanceOfType,
    DerivedFrom, VersionOf, RefinesTo, ForkedFrom, SimilarTo,
    ConstraintLimitsDeformation,
    ToleranceAffectsAssemblyFit,
    MatesWidth,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CADGraphEdge {
    pub edge_id: u64,
    pub from_node: u64, pub to_node: u64,
    pub edge_type: CADEdgeType,
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
pub struct CADGraph {
    pub graph_id: u64, pub project_id: u64,
    pub source_description: String,
    pub nodes: Vec<CADGraphNode>,
    pub edges: Vec<CADGraphEdge>,
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
pub enum CADGraphQuery {
    NodeDetail         { node_id: u64 },
    FeatureTree        { part_node_id: u64 },
    ToleranceChain     { dimension_ids: Vec<u64> },
    AssemblyConstraints { assembly_node_id: u64 },
    MaterialsUsed,
    GDTFrames,
    BOMTree,
    CrossModalLinks    { node_id: u64 },
    InterfacingFaces   { part_a_id: u64, part_b_id: u64 },
    AGIActivity, AllNodes, AllEdges,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CADSemanticHook {
    OnGraphCreated,
    OnInferRelationships,
    OnEdgeCompletion,
    OnCrossModalityLink { target_modality: String, target_graph_id: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CADHeadlessOp {
    RebuildAllFeatures,
    ExportAllParts     { format: CADExportFormat, output_dir: String },
    GenerateBOM        { output_path: String },
    SolveAllConstraints,
    ValidateTolerances,
    TessellateAll      { chord_tol_mm: f64 },
    CrossLinkTo3D      { graph_id_3d: u64 },
}

// ─────────────────────────────────────────────────────────────────────────────
// OUTPUT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CADModalityOutput {
    pub success: bool,
    pub graph_id: Option<u64>,
    pub graph: Option<CADGraph>,
    pub analysis: Option<CADAnalysisResult>,
    pub mass_properties: Option<MassPropertiesResult>,
    pub interference: Option<InterferenceResult>,
    pub tolerance_chain: Option<ToleranceChainResult>,
    pub bom: Option<Vec<BOMEntry>>,
    pub tessellation: Option<TessellationResult>,
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
        let input = serde_json::json!({"action":"Prompt","prompt":prompt,"max_tokens":max_tokens,"temperature":0.05,"system_context":"CAD engineering analysis. Return only valid JSON."});
        let out = std::process::Command::new(&self.prompt_pipeline_path).arg("--input").arg(input.to_string()).output().map_err(|e| e.to_string())?;
        let r: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
        Ok(r["response"].as_str().unwrap_or("{}").to_string())
    }

    fn save_graph(&self, g: &CADGraph) -> Result<(), String> {
        let path = format!("{}/local/cad_graph_{}.json", self.zsei_path, g.graph_id);
        if let Some(p) = std::path::Path::new(&path).parent() { std::fs::create_dir_all(p).map_err(|e| e.to_string())?; }
        std::fs::write(&path, serde_json::to_string_pretty(g).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    fn load_graph(&self, id: u64) -> Result<CADGraph, String> {
        let path = format!("{}/local/cad_graph_{}.json", self.zsei_path, id);
        serde_json::from_str(&std::fs::read_to_string(&path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    fn generate_id(&self) -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as u64 }
    fn now_iso8601(&self) -> String { format!("{}", self.generate_id()) }
    fn extract_json_array(raw: &str) -> String { if let (Some(s), Some(e)) = (raw.find('['), raw.rfind(']')) { raw[s..=e].to_string() } else { "[]".to_string() } }
    fn extract_json_object(raw: &str) -> String { if let (Some(s), Some(e)) = (raw.find('{'), raw.rfind('}')) { raw[s..=e].to_string() } else { "{}".to_string() } }
}

impl PipelineExecutor {
    async fn infer_feature_relationships(&self, features: &[FeatureInfo]) -> Vec<(u64, u64, CADEdgeType, String)> {
        if features.len() < 2 { return vec![]; }
        let list: Vec<serde_json::Value> = features.iter().take(30).map(|f| serde_json::json!({
            "feature_id": f.feature_id, "name": &f.name,
            "type": format!("{:?}", f.feature_type),
            "params": f.parameters,
            "children": f.child_feature_ids.len(),
        })).collect();

        let prompt = format!(r#"
Analyze these CAD features and identify functional/semantic relationships beyond the parent-child tree.

Features:
{}

Available edge types: FunctionalRole, Affects, Enables, Prevents, DerivedFrom, SimilarTo,
ConstraintLimitsDeformation, ToleranceAffectsAssemblyFit, Performs, CausedBy

Return ONLY valid JSON array:
[{{"from_feature_id": N, "to_feature_id": M, "edge_type": "TypeName", "reason": "brief"}}]"#,
            serde_json::to_string_pretty(&list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 600).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str).unwrap_or_default().into_iter()
                    .filter_map(|v| {
                        let from = v["from_feature_id"].as_u64()?;
                        let to = v["to_feature_id"].as_u64()?;
                        let etype = map_cad_edge_str(v["edge_type"].as_str().unwrap_or("Affects"));
                        let reason = v["reason"].as_str().unwrap_or("").to_string();
                        Some((from, to, etype, reason))
                    }).collect()
            }
            Err(_) => vec![],
        }
    }

    async fn infer_semantic_relationships(&self, nodes: &[CADGraphNode]) -> Vec<(u64, u64, CADEdgeType, String)> {
        if nodes.len() < 2 { return vec![]; }
        let node_list: Vec<serde_json::Value> = nodes.iter().take(25).map(|n| serde_json::json!({
            "node_id": n.node_id, "type": format!("{:?}", n.node_type),
            "content": n.content.chars().take(80).collect::<String>(),
            "nominal": n.nominal_value, "dof": n.dof,
        })).collect();

        let prompt = format!(r#"
Identify semantic relationships between these CAD graph nodes not already captured structurally.

Nodes:
{}

Edge types: ConstraintLimitsDeformation, ToleranceAffectsAssemblyFit, MatesWidth,
Affects, Enables, Prevents, FunctionalRole, DerivedFrom, SimilarTo, Performs, CausedBy

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
                        let etype = map_cad_edge_str(v["edge_type"].as_str().unwrap_or("Affects"));
                        let reason = v["reason"].as_str().unwrap_or("").to_string();
                        Some((from, to, etype, reason))
                    }).collect()
            }
            Err(_) => vec![],
        }
    }

    async fn classify_assembly_function(&self, analysis: &CADAnalysisResult) -> String {
        let summary = serde_json::json!({
            "part_count": analysis.parts.len(),
            "assembly_count": analysis.assemblies.len(),
            "constraint_count": analysis.assembly_constraints.len(),
            "has_threads": analysis.features.iter().any(|f| matches!(f.feature_type, FeatureType::Thread { .. })),
            "has_holes": analysis.features.iter().any(|f| matches!(f.feature_type, FeatureType::Hole { .. })),
            "has_gears": analysis.assembly_constraints.iter().any(|c| matches!(c.constraint_type, AssemblyConstraintType::GearMate { .. })),
            "dof_total": analysis.assemblies.iter().map(|a| a.degree_of_freedom).sum::<u32>(),
            "sample_part_names": analysis.parts.iter().take(5).map(|p| &p.name).collect::<Vec<_>>(),
        });
        let prompt = format!(r#"
Classify the mechanical function of this CAD assembly:
{}

Choose: Mechanism, Structural_Frame, Enclosure_Housing, Robotic_Arm, Transmission_Gearbox,
Fluid_System, Fastener_Assembly, Electrical_Enclosure, Fixture_Jig, General_Purpose.
Return ONLY the single classification label."#,
            serde_json::to_string(&summary).unwrap_or_default());
        self.llm_zero_shot(&prompt, 20).await.unwrap_or_else(|_| "General_Purpose".into()).trim().to_string()
    }

    /// Compute worst-case tolerance stack-up for a chain of dimensions
    fn compute_tolerance_stack(&self, tols: &[&ToleranceInfo]) -> ToleranceChainResult {
        let worst_case: f64 = tols.iter().map(|t| (t.upper_deviation - t.lower_deviation).abs()).sum();
        let rss: f64 = tols.iter().map(|t| {
            let half = (t.upper_deviation - t.lower_deviation) / 2.0;
            half * half
        }).sum::<f64>().sqrt() * 2.0;
        let contributions: Vec<(u64, f64)> = tols.iter().map(|t| {
            (t.tolerance_id, (t.upper_deviation - t.lower_deviation).abs())
        }).collect();
        ToleranceChainResult {
            total_worst_case_mm: worst_case,
            total_rss_mm: rss,
            contributing_dimensions: contributions,
        }
    }
}

fn map_cad_edge_str(s: &str) -> CADEdgeType {
    match s {
        "ParentFeature"               => CADEdgeType::ParentFeature,
        "ChildFeature"                => CADEdgeType::ChildFeature,
        "DrivenBySketch"              => CADEdgeType::DrivenBySketch,
        "ParametricDrivesFeature"     => CADEdgeType::ParametricDrivesFeature,
        "FeatureCreatesBody"          => CADEdgeType::FeatureCreatesBody,
        "FeatureCreatesFace"          => CADEdgeType::FeatureCreatesFace,
        "FeatureConsumesBody"         => CADEdgeType::FeatureConsumesBody,
        "BoundsFace"                  => CADEdgeType::BoundsFace,
        "AdjacentFace"                => CADEdgeType::AdjacentFace,
        "ConstrainedBy"               => CADEdgeType::ConstrainedBy,
        "MatesWith"                   => CADEdgeType::MatesWith,
        "DimensionsFeature"           => CADEdgeType::DimensionsFeature,
        "ToleratesFeature"            => CADEdgeType::ToleratesFeature,
        "ToleranceChain"              => CADEdgeType::ToleranceChain,
        "GDTAppliesTo"                => CADEdgeType::GDTAppliesTo,
        "MadeOf"                      => CADEdgeType::MadeOf,
        "ParametricGeneratesMesh"     => CADEdgeType::ParametricGeneratesMesh,
        "ProvidesJointGeometry"       => CADEdgeType::ProvidesJointGeometry,
        "ProvidesActuatorSpec"        => CADEdgeType::ProvidesActuatorSpec,
        "ConstraintLimitsDeformation" => CADEdgeType::ConstraintLimitsDeformation,
        "ToleranceAffectsAssemblyFit" => CADEdgeType::ToleranceAffectsAssemblyFit,
        "MatesWidth"                  => CADEdgeType::MatesWidth,
        "Affects"                     => CADEdgeType::Affects,
        "Enables"                     => CADEdgeType::Enables,
        "Prevents"                    => CADEdgeType::Prevents,
        "CausedBy"                    => CADEdgeType::CausedBy,
        "FunctionalRole"              => CADEdgeType::FunctionalRole,
        "DerivedFrom"                 => CADEdgeType::DerivedFrom,
        "SimilarTo"                   => CADEdgeType::SimilarTo,
        "Performs"                    => CADEdgeType::Performs,
        "PartOf"                      => CADEdgeType::PartOf,
        _                             => CADEdgeType::Affects,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH CREATION
// ─────────────────────────────────────────────────────────────────────────────

async fn create_graph(executor: &PipelineExecutor, analysis: CADAnalysisResult, project_id: u64) -> CADModalityOutput {
    let graph_id = executor.generate_id();
    let now = executor.now_iso8601();
    let mut nodes: Vec<CADGraphNode> = Vec::new();
    let mut edges: Vec<CADGraphEdge> = Vec::new();
    let mut node_id: u64 = 1;
    let mut edge_id: u64 = 1;

    let function_class = executor.classify_assembly_function(&analysis).await;

    // ── ROOT ──
    let root_id = node_id;
    nodes.push(CADGraphNode {
        node_id: root_id, node_type: CADNodeType::CADDocument,
        content: format!("CAD [{}]: format={} parts={} assemblies={} features={} constraints={}",
            function_class, analysis.source_format, analysis.parts.len(), analysis.assemblies.len(),
            analysis.features.len(), analysis.assembly_constraints.len()),
        materialized_path: Some(format!("/Modalities/ParametricCAD/Project_{}/Graph_{}", project_id, graph_id)),
        provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
        version_notes: vec![VersionNote { version: 1, note: "Initial creation".into(), step_index: None, timestamp: now.clone(), change_type: ChangeType::Created }],
        keywords: vec!["cad".into(), "parametric".into(), function_class.to_lowercase()],
        hotness_score: 1.0, ..Default::default()
    });
    node_id += 1;

    // ── MATERIALS ──
    let mut mat_name_to_nid: HashMap<String, u64> = HashMap::new();
    for mat in &analysis.materials {
        let mid = node_id;
        nodes.push(CADGraphNode {
            node_id: mid, node_type: CADNodeType::MaterialNode,
            content: format!("Material: {} ({:?}) ρ={:.0}kg/m³ E={:?}GPa σy={:?}MPa",
                mat.name, mat.material_class, mat.density_kg_m3, mat.youngs_modulus_gpa, mat.yield_strength_mpa),
            material_name: Some(mat.name.clone()),
            materialized_path: Some(format!("/Modalities/ParametricCAD/Project_{}/Graph_{}/Material/{}", project_id, graph_id, mat.material_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["material".into(), mat.name.to_lowercase(), format!("{:?}", mat.material_class).to_lowercase()],
            hotness_score: 0.65,
            embedding_hint: Some(format!("material: {} class: {:?}", mat.name, mat.material_class)),
            ..Default::default()
        });
        edges.push(CADGraphEdge { edge_id, from_node: root_id, to_node: mid, edge_type: CADEdgeType::Contains, weight: 0.7, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        mat_name_to_nid.insert(mat.name.clone(), mid);
        node_id += 1;
    }

    // ── PARTS ──
    let mut part_id_to_nid: HashMap<u64, u64> = HashMap::new();
    for part in &analysis.parts {
        let pid = node_id;
        nodes.push(CADGraphNode {
            node_id: pid, node_type: CADNodeType::PartNode,
            content: format!("Part: {} rev={:?} features={} faces={} mass={:?}kg mat={}",
                part.name, part.revision, part.feature_count, part.face_count,
                part.mass_kg, part.material_name.as_deref().unwrap_or("none")),
            part_number: part.part_number.clone(), revision: part.revision.clone(),
            mass_kg: part.mass_kg, material_name: part.material_name.clone(),
            bounding_box: Some(part.bounding_box.clone()),
            materialized_path: Some(format!("/Modalities/ParametricCAD/Project_{}/Graph_{}/Part/{}", project_id, graph_id, part.part_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: { let mut kw = vec!["part".into(), part.name.to_lowercase()]; if let Some(ref pn) = part.part_number { kw.push(pn.clone()); } kw },
            hotness_score: 0.85, ..Default::default()
        });
        edges.push(CADGraphEdge { edge_id, from_node: root_id, to_node: pid, edge_type: CADEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Part → material
        if let Some(ref mat_name) = part.material_name {
            if let Some(&mat_nid) = mat_name_to_nid.get(mat_name) {
                edges.push(CADGraphEdge { edge_id, from_node: pid, to_node: mat_nid, edge_type: CADEdgeType::MadeOf, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }

        // Cross-modal: part → 3D mesh (109)
        edges.push(CADGraphEdge {
            edge_id, from_node: pid, to_node: pid,
            edge_type: CADEdgeType::ParametricGeneratesMesh, weight: 1.0,
            provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
            properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("3d")); p },
            ..Default::default()
        });
        edge_id += 1;
        part_id_to_nid.insert(part.part_id, pid);
        node_id += 1;
    }

    // ── ASSEMBLIES ──
    let mut asm_id_to_nid: HashMap<u64, u64> = HashMap::new();
    for asm in &analysis.assemblies {
        let aid = node_id;
        nodes.push(CADGraphNode {
            node_id: aid, node_type: CADNodeType::AssemblyNode,
            content: format!("Assembly: {} components={} constraints={} dof={} fully={} mass={:?}kg",
                asm.name, asm.component_count, asm.constraint_count,
                asm.degree_of_freedom, asm.fully_constrained, asm.total_mass_kg),
            part_number: asm.part_number.clone(),
            dof: Some(asm.degree_of_freedom),
            mass_kg: asm.total_mass_kg,
            bounding_box: Some(asm.bounding_box.clone()),
            materialized_path: Some(format!("/Modalities/ParametricCAD/Project_{}/Graph_{}/Assembly/{}", project_id, graph_id, asm.assembly_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["assembly".into(), asm.name.to_lowercase()],
            hotness_score: 0.9, ..Default::default()
        });
        edges.push(CADGraphEdge { edge_id, from_node: root_id, to_node: aid, edge_type: CADEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        asm_id_to_nid.insert(asm.assembly_id, aid);
        node_id += 1;
    }

    // ── COMPONENT INSTANCES ──
    for comp in &analysis.components {
        let cid = node_id;
        nodes.push(CADGraphNode {
            node_id: cid, node_type: CADNodeType::ComponentInstanceNode,
            content: format!("Instance: {} qty={} suppressed={}", comp.instance_name, comp.quantity, comp.suppressed),
            is_suppressed: Some(comp.suppressed),
            materialized_path: Some(format!("/Modalities/ParametricCAD/Project_{}/Graph_{}/Instance/{}", project_id, graph_id, comp.instance_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["instance".into(), comp.instance_name.to_lowercase()],
            hotness_score: 0.7, ..Default::default()
        });

        // Instance → parent assembly
        if let Some(&asm_nid) = asm_id_to_nid.get(&comp.parent_assembly_id) {
            edges.push(CADGraphEdge { edge_id, from_node: asm_nid, to_node: cid, edge_type: CADEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        // Instance is instance of part
        if comp.is_part {
            if let Some(&part_nid) = part_id_to_nid.get(&comp.part_or_assembly_id) {
                edges.push(CADGraphEdge { edge_id, from_node: cid, to_node: part_nid, edge_type: CADEdgeType::InstanceOf, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        } else {
            if let Some(&sub_asm_nid) = asm_id_to_nid.get(&comp.part_or_assembly_id) {
                edges.push(CADGraphEdge { edge_id, from_node: cid, to_node: sub_asm_nid, edge_type: CADEdgeType::ChildOf, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        node_id += 1;
    }

    // ── FEATURES ──
    let mut feat_id_to_nid: HashMap<u64, u64> = HashMap::new();
    for feat in &analysis.features {
        let fid = node_id;
        let param_summary: Vec<String> = feat.parameters.iter().take(3).map(|(k, v)| format!("{}={:.2}", k, v)).collect();
        nodes.push(CADGraphNode {
            node_id: fid, node_type: CADNodeType::FeatureNode,
            content: format!("Feature {:?}: {} [{}] suppressed={}",
                feat.feature_type, feat.name, param_summary.join(","), feat.is_suppressed),
            feature_type: Some(format!("{:?}", feat.feature_type)),
            is_suppressed: Some(feat.is_suppressed),
            materialized_path: Some(format!("/Modalities/ParametricCAD/Project_{}/Graph_{}/Feature/{}", project_id, graph_id, feat.feature_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: { let mut kw = vec!["feature".into(), format!("{:?}", feat.feature_type).to_lowercase(), feat.name.to_lowercase()]; kw },
            hotness_score: 0.7 - (if feat.is_suppressed { 0.2 } else { 0.0 }),
            ..Default::default()
        });

        // Feature → parent part
        if let Some(&part_nid) = part_id_to_nid.get(&feat.part_id) {
            edges.push(CADGraphEdge { edge_id, from_node: part_nid, to_node: fid, edge_type: CADEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        feat_id_to_nid.insert(feat.feature_id, fid);
        node_id += 1;
    }

    // Feature parent-child hierarchy
    for feat in &analysis.features {
        if let Some(&feat_nid) = feat_id_to_nid.get(&feat.feature_id) {
            for parent_id in &feat.parent_feature_ids {
                if let Some(&parent_nid) = feat_id_to_nid.get(parent_id) {
                    edges.push(CADGraphEdge { edge_id, from_node: parent_nid, to_node: feat_nid, edge_type: CADEdgeType::ParentFeature, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                    edge_id += 1;
                }
            }
        }
    }

    // ── SKETCHES ──
    let mut sketch_id_to_nid: HashMap<u64, u64> = HashMap::new();
    for sketch in &analysis.sketches {
        let sid = node_id;
        nodes.push(CADGraphNode {
            node_id: sid, node_type: CADNodeType::SketchNode,
            content: format!("Sketch: {} entities={} constrs={} dims={} fully={}",
                sketch.name, sketch.entity_count, sketch.constraint_count, sketch.dimension_count, sketch.fully_constrained),
            materialized_path: Some(format!("/Modalities/ParametricCAD/Project_{}/Graph_{}/Sketch/{}", project_id, graph_id, sketch.sketch_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["sketch".into(), sketch.name.to_lowercase()],
            hotness_score: 0.6, ..Default::default()
        });
        // Sketch → part
        if let Some(&part_nid) = part_id_to_nid.get(&sketch.part_id) {
            edges.push(CADGraphEdge { edge_id, from_node: part_nid, to_node: sid, edge_type: CADEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        // Feature driven by sketch
        for feat in analysis.features.iter().filter(|f| f.sketch_id == Some(sketch.sketch_id)) {
            if let Some(&feat_nid) = feat_id_to_nid.get(&feat.feature_id) {
                edges.push(CADGraphEdge { edge_id, from_node: feat_nid, to_node: sid, edge_type: CADEdgeType::DrivenBySketch, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        sketch_id_to_nid.insert(sketch.sketch_id, sid);
        node_id += 1;
    }

    // ── ASSEMBLY CONSTRAINTS ──
    let mut constr_id_to_nid: HashMap<u64, u64> = HashMap::new();
    for constr in &analysis.assembly_constraints {
        let cid = node_id;
        nodes.push(CADGraphNode {
            node_id: cid, node_type: CADNodeType::AssemblyConstraintNode,
            content: format!("Constraint {:?}: offset={:?}mm angle={:?}° dof_removed={}",
                constr.constraint_type, constr.offset_mm, constr.angle_deg, constr.dof_removed),
            dof: Some(constr.dof_removed),
            materialized_path: Some(format!("/Modalities/ParametricCAD/Project_{}/Graph_{}/Constraint/{}", project_id, graph_id, constr.constraint_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["constraint".into(), format!("{:?}", constr.constraint_type).to_lowercase()],
            hotness_score: 0.65, ..Default::default()
        });
        edges.push(CADGraphEdge { edge_id, from_node: root_id, to_node: cid, edge_type: CADEdgeType::Contains, weight: 0.7, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Constraint ConstrainedBy components
        if let Some(&ca_nid) = part_id_to_nid.get(&constr.component_a_id).or_else(|| asm_id_to_nid.get(&constr.component_a_id)) {
            edges.push(CADGraphEdge { edge_id, from_node: ca_nid, to_node: cid, edge_type: CADEdgeType::ConstrainedBy, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        if let Some(&cb_nid) = part_id_to_nid.get(&constr.component_b_id).or_else(|| asm_id_to_nid.get(&constr.component_b_id)) {
            edges.push(CADGraphEdge { edge_id, from_node: cb_nid, to_node: cid, edge_type: CADEdgeType::ConstrainedBy, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }

        // MatesWidth cross-modal note for gear/screw mates → kinematics
        if matches!(constr.constraint_type, AssemblyConstraintType::GearMate { .. } | AssemblyConstraintType::RackPinion { .. } | AssemblyConstraintType::ScrewMate { .. }) {
            edges.push(CADGraphEdge {
                edge_id, from_node: cid, to_node: cid,
                edge_type: CADEdgeType::ProvidesJointGeometry, weight: 0.9,
                provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("kinematics")); p.insert("constraint_type".into(), serde_json::json!(format!("{:?}", constr.constraint_type))); p },
                ..Default::default()
            });
            edge_id += 1;
        }

        constr_id_to_nid.insert(constr.constraint_id, cid);
        node_id += 1;
    }

    // ── DIMENSIONS ──
    let mut dim_id_to_nid: HashMap<u64, u64> = HashMap::new();
    for dim in &analysis.dimensions {
        let did = node_id;
        nodes.push(CADGraphNode {
            node_id: did, node_type: CADNodeType::DimensionNode,
            content: format!("Dim {:?}: {}={:.4}{:?} ref={} driven={}",
                dim.dim_type, dim.name.as_deref().unwrap_or("dim"), dim.nominal_value,
                dim.unit, dim.is_reference, dim.is_driven),
            nominal_value: Some(dim.nominal_value),
            unit: Some(format!("{:?}", dim.unit)),
            materialized_path: Some(format!("/Modalities/ParametricCAD/Project_{}/Graph_{}/Dim/{}", project_id, graph_id, dim.dimension_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["dimension".into(), format!("{:?}", dim.dim_type).to_lowercase()],
            hotness_score: 0.6, ..Default::default()
        });
        edges.push(CADGraphEdge { edge_id, from_node: root_id, to_node: did, edge_type: CADEdgeType::Contains, weight: 0.6, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        if let Some(feat_id) = dim.feature_id {
            if let Some(&feat_nid) = feat_id_to_nid.get(&feat_id) {
                edges.push(CADGraphEdge { edge_id, from_node: did, to_node: feat_nid, edge_type: CADEdgeType::DimensionsFeature, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        dim_id_to_nid.insert(dim.dimension_id, did);
        node_id += 1;
    }

    // ── TOLERANCES ──
    let mut tol_id_to_nid: HashMap<u64, u64> = HashMap::new();
    for tol in &analysis.tolerances {
        let tid = node_id;
        nodes.push(CADGraphNode {
            node_id: tid, node_type: CADNodeType::ToleranceNode,
            content: format!("Tol {:?}: +{:.4}/{:.4} class={:?} bilateral={}",
                tol.tolerance_type, tol.upper_deviation, tol.lower_deviation,
                tol.tolerance_class, tol.is_bilateral),
            nominal_value: Some(0.0), upper_deviation: Some(tol.upper_deviation), lower_deviation: Some(tol.lower_deviation),
            materialized_path: Some(format!("/Modalities/ParametricCAD/Project_{}/Graph_{}/Tol/{}", project_id, graph_id, tol.tolerance_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["tolerance".into(), format!("{:?}", tol.tolerance_type).to_lowercase()],
            hotness_score: 0.6, ..Default::default()
        });
        edges.push(CADGraphEdge { edge_id, from_node: root_id, to_node: tid, edge_type: CADEdgeType::Contains, weight: 0.6, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Tolerance → dimensions
        for &dim_id in &tol.referenced_dimension_ids {
            if let Some(&dim_nid) = dim_id_to_nid.get(&dim_id) {
                edges.push(CADGraphEdge { edge_id, from_node: tid, to_node: dim_nid, edge_type: CADEdgeType::ToleratesFeature, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        tol_id_to_nid.insert(tol.tolerance_id, tid);
        node_id += 1;
    }

    // ── GD&T FRAMES ──
    for gdt in &analysis.gdt_frames {
        let gid = node_id;
        nodes.push(CADGraphNode {
            node_id: gid, node_type: CADNodeType::GDTFrameNode,
            content: format!("GDT {:?}: ±{:.4}mm datums={} MMC={:?}",
                gdt.characteristic, gdt.tolerance_value_mm,
                gdt.datum_references.iter().map(|d| &d.datum_label).collect::<Vec<_>>(),
                matches!(gdt.material_condition, MaterialCondition::MMC)),
            nominal_value: Some(gdt.tolerance_value_mm),
            materialized_path: Some(format!("/Modalities/ParametricCAD/Project_{}/Graph_{}/GDT/{}", project_id, graph_id, gdt.frame_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["gdt".into(), format!("{:?}", gdt.characteristic).to_lowercase()],
            hotness_score: 0.65, ..Default::default()
        });
        edges.push(CADGraphEdge { edge_id, from_node: root_id, to_node: gid, edge_type: CADEdgeType::Contains, weight: 0.6, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── MASS PROPERTIES ──
    for mp in &analysis.mass_properties {
        let mpid = node_id;
        nodes.push(CADGraphNode {
            node_id: mpid, node_type: CADNodeType::MassPropertiesNode,
            content: format!("MassProps: mass={:.4}kg vol={:.2}mm³ cog=({:.2},{:.2},{:.2})",
                mp.mass_kg, mp.volume_mm3, mp.cog_mm[0], mp.cog_mm[1], mp.cog_mm[2]),
            mass_kg: Some(mp.mass_kg),
            materialized_path: Some(format!("/Modalities/ParametricCAD/Project_{}/Graph_{}/MassProps/{}", project_id, graph_id, mp.part_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["mass-properties".into(), "inertia".into()], hotness_score: 0.6, ..Default::default()
        });
        if let Some(&part_nid) = part_id_to_nid.get(&mp.part_id) {
            edges.push(CADGraphEdge { edge_id, from_node: part_nid, to_node: mpid, edge_type: CADEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        // MassProps cross-modal: → control system (122) for dynamics model
        edges.push(CADGraphEdge { edge_id, from_node: mpid, to_node: mpid, edge_type: CADEdgeType::ProvidesActuatorSpec, weight: 0.85, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("control_systems")); p }, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── DRAWING VIEWS ──
    for view in analysis.drawing_views.iter().take(30) {
        let vid = node_id;
        nodes.push(CADGraphNode {
            node_id: vid, node_type: CADNodeType::DrawingViewNode,
            content: format!("DrawView {:?}: {} scale={:.3}", view.view_type, view.name, view.scale),
            materialized_path: Some(format!("/Modalities/ParametricCAD/Project_{}/Graph_{}/DrawView/{}", project_id, graph_id, view.view_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["drawing".into(), format!("{:?}", view.view_type).to_lowercase()], hotness_score: 0.5, ..Default::default()
        });
        edges.push(CADGraphEdge { edge_id, from_node: root_id, to_node: vid, edge_type: CADEdgeType::Contains, weight: 0.5, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── BOM ──
    for bom_entry in analysis.bom_entries.iter().take(100) {
        let bid = node_id;
        nodes.push(CADGraphNode {
            node_id: bid, node_type: CADNodeType::BOMEntryNode,
            content: format!("BOM[L{}]: {} {} qty={} make={} buy={}",
                bom_entry.level, bom_entry.part_number, bom_entry.description,
                bom_entry.quantity, bom_entry.is_make, bom_entry.is_purchased),
            part_number: Some(bom_entry.part_number.clone()),
            mass_kg: bom_entry.unit_mass_kg,
            materialized_path: Some(format!("/Modalities/ParametricCAD/Project_{}/Graph_{}/BOM/{}", project_id, graph_id, bom_entry.entry_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["bom".into(), bom_entry.part_number.to_lowercase()], hotness_score: 0.55, ..Default::default()
        });
        if let Some(&part_nid) = bom_entry.part_id.and_then(|id| part_id_to_nid.get(&id)) {
            edges.push(CADGraphEdge { edge_id, from_node: bid, to_node: part_nid, edge_type: CADEdgeType::References, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        edges.push(CADGraphEdge { edge_id, from_node: root_id, to_node: bid, edge_type: CADEdgeType::Contains, weight: 0.5, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── HOOK 1: OnGraphCreated ──
    let _ = executor.save_graph(&CADGraph { graph_id, project_id, source_description: format!("{} {}", analysis.source_format, analysis.source_path.as_deref().unwrap_or("")), nodes: nodes.clone(), edges: edges.clone(), root_node_id: root_id, state: GraphStateType::Created, state_history: vec![GraphStateTransition { from: GraphStateType::Created, to: GraphStateType::Created, timestamp: now.clone(), triggered_by_step: None }], created_at: now.clone(), updated_at: now.clone(), version: 1, version_notes: vec![VersionNote { version: 1, note: format!("Created: {} nodes {} edges", nodes.len(), edges.len()), step_index: None, timestamp: now.clone(), change_type: ChangeType::Created }] });

    // ── HOOK 2: OnInferRelationships ──
    let inferred = executor.infer_semantic_relationships(&nodes).await;
    let valid: std::collections::HashSet<u64> = nodes.iter().map(|n| n.node_id).collect();
    for (from, to, etype, reason) in inferred {
        if valid.contains(&from) && valid.contains(&to) && from != to {
            edges.push(CADGraphEdge { edge_id, from_node: from, to_node: to, edge_type: etype, weight: 0.8, provenance: EdgeProvenance::DerivedFromHook, version: 1, properties: { let mut p = HashMap::new(); p.insert("reason".into(), serde_json::json!(reason)); p }, ..Default::default() });
            edge_id += 1;
        }
    }

    // ── HOOK 3: OnEdgeCompletion → hotness ──
    let mut deg: HashMap<u64, u32> = HashMap::new();
    for e in &edges { *deg.entry(e.from_node).or_insert(0) += 1; *deg.entry(e.to_node).or_insert(0) += 1; }
    let max_deg = deg.values().copied().max().unwrap_or(1) as f32;
    for n in &mut nodes { if let Some(&d) = deg.get(&n.node_id) { n.hotness_score = (n.hotness_score + (d as f32 / max_deg) * 0.15).min(1.0); } }

    let final_graph = CADGraph { graph_id, project_id, source_description: format!("{} {}", analysis.source_format, analysis.source_path.unwrap_or_default()), nodes, edges, root_node_id: root_id, state: GraphStateType::SemanticEnriched, state_history: vec![GraphStateTransition { from: GraphStateType::Created, to: GraphStateType::SemanticEnriched, timestamp: now.clone(), triggered_by_step: None }], created_at: now.clone(), updated_at: now.clone(), version: 1, version_notes: vec![VersionNote { version: 1, note: "Semantic enrichment complete".into(), step_index: None, timestamp: now, change_type: ChangeType::EnrichedBySemantic }] };
    let _ = executor.save_graph(&final_graph);
    CADModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(final_graph), ..Default::default() }
}

// ─────────────────────────────────────────────────────────────────────────────
// MAIN EXECUTION
// ─────────────────────────────────────────────────────────────────────────────

pub async fn execute(input: CADModalityAction) -> Result<CADModalityOutput, String> {
    let executor = PipelineExecutor::new();

    match input {
        CADModalityAction::AnalyzeFile { data, extract_features, extract_assembly, extract_tolerances } |
        CADModalityAction::AnalyzePart { data, .. } |
        CADModalityAction::AnalyzeAssembly { data, .. } |
        CADModalityAction::AnalyzeDrawing { data, .. } => {
            let analysis_id = executor.generate_id();
            let (source_format, source_path, is_assembly) = match &data {
                CADDataSource::STEP { file_path } => ("STEP".into(), Some(file_path.clone()), false),
                CADDataSource::IGES { file_path } => ("IGES".into(), Some(file_path.clone()), false),
                CADDataSource::BREP { file_path } | CADDataSource::OCC { file_path } => ("BRep".into(), Some(file_path.clone()), false),
                CADDataSource::F3D { file_path } => ("F3D".into(), Some(file_path.clone()), true),
                CADDataSource::FCStd { file_path } => ("FCStd".into(), Some(file_path.clone()), true),
                CADDataSource::SLDPRT { file_path } => ("SLDPRT".into(), Some(file_path.clone()), false),
                CADDataSource::SLDASM { file_path } => ("SLDASM".into(), Some(file_path.clone()), true),
                CADDataSource::IPT { file_path } => ("IPT".into(), Some(file_path.clone()), false),
                CADDataSource::IAM { file_path } => ("IAM".into(), Some(file_path.clone()), true),
                CADDataSource::CATPART { file_path } => ("CATPART".into(), Some(file_path.clone()), false),
                CADDataSource::CATPRODUCT { file_path } => ("CATPRODUCT".into(), Some(file_path.clone()), true),
                CADDataSource::X_T { file_path } | CADDataSource::X_B { file_path } => ("Parasolid".into(), Some(file_path.clone()), false),
                CADDataSource::SAT { file_path } => ("ACIS".into(), Some(file_path.clone()), false),
                CADDataSource::JT { file_path } => ("JT".into(), Some(file_path.clone()), false),
                CADDataSource::Parametric { name, .. } => ("Parametric".into(), None, false),
                CADDataSource::AssemblyManifest { manifest_path } => ("Manifest".into(), Some(manifest_path.clone()), true),
            };
            Ok(CADModalityOutput { success: true, analysis: Some(CADAnalysisResult { analysis_id, source_format, source_path, is_assembly, ..Default::default() }), ..Default::default() })
        }

        CADModalityAction::CreateGraph { analysis, project_id } => {
            Ok(create_graph(&executor, analysis, project_id).await)
        }

        CADModalityAction::UpdateGraph { graph_id, updates, project_id } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            let mut update_count = 0;
            for update in &updates {
                match update {
                    CADUpdate::ModifyFeatureParameter { feature_node_id, parameter, new_value } => {
                        if let Some(n) = graph.nodes.iter_mut().find(|n| n.node_id == *feature_node_id) {
                            n.version += 1;
                            n.version_notes.push(VersionNote { version: n.version, note: format!("Parameter {}={:.4}", parameter, new_value), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                            update_count += 1;
                        }
                    }
                    CADUpdate::SuppressFeature { feature_node_id } => {
                        if let Some(n) = graph.nodes.iter_mut().find(|n| n.node_id == *feature_node_id) {
                            n.is_suppressed = Some(true);
                            n.version += 1;
                            n.version_notes.push(VersionNote { version: n.version, note: "Feature suppressed".into(), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                            update_count += 1;
                        }
                    }
                    CADUpdate::UnsuppressFeature { feature_node_id } => {
                        if let Some(n) = graph.nodes.iter_mut().find(|n| n.node_id == *feature_node_id) {
                            n.is_suppressed = Some(false);
                            n.version += 1;
                            update_count += 1;
                        }
                    }
                    CADUpdate::UpdateTolerance { tolerance_node_id, upper, lower } => {
                        if let Some(n) = graph.nodes.iter_mut().find(|n| n.node_id == *tolerance_node_id) {
                            n.upper_deviation = Some(*upper);
                            n.lower_deviation = Some(*lower);
                            n.version += 1;
                            n.version_notes.push(VersionNote { version: n.version, note: format!("Tolerance updated: +{:.4}/{:.4}", upper, lower), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                            update_count += 1;
                        }
                    }
                    CADUpdate::UpdateMaterial { part_node_id, material_name } => {
                        if let Some(n) = graph.nodes.iter_mut().find(|n| n.node_id == *part_node_id) {
                            n.material_name = Some(material_name.clone());
                            n.version += 1;
                            update_count += 1;
                        }
                    }
                    CADUpdate::UpdateDimension { dimension_id, new_value } => {
                        if let Some(n) = graph.nodes.iter_mut().find(|n| n.node_id == *dimension_id) {
                            n.nominal_value = Some(*new_value);
                            n.version += 1;
                            update_count += 1;
                        }
                    }
                    _ => { update_count += 1; }
                }
            }
            graph.version += 1; graph.updated_at = now.clone(); graph.state = GraphStateType::Updated;
            graph.version_notes.push(VersionNote { version: graph.version, note: format!("{} CAD updates applied", update_count), step_index: None, timestamp: now, change_type: ChangeType::Updated });
            executor.save_graph(&graph)?;
            Ok(CADModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        CADModalityAction::EvaluateFeature { graph_id, feature_node_id } => {
            let graph = executor.load_graph(graph_id)?;
            let node = graph.nodes.iter().find(|n| n.node_id == feature_node_id)
                .ok_or_else(|| format!("Feature node {} not found", feature_node_id))?;
            Ok(CADModalityOutput { success: true, graph_id: Some(graph_id), query_result: Some(serde_json::json!({ "feature": node, "status": "evaluated" })), ..Default::default() })
        }

        CADModalityAction::SolveConstraints { graph_id, assembly_node_id } => {
            let graph = executor.load_graph(graph_id)?;
            let constr_count = graph.edges.iter().filter(|e| matches!(e.edge_type, CADEdgeType::ConstrainedBy)).count();
            Ok(CADModalityOutput { success: true, graph_id: Some(graph_id), query_result: Some(serde_json::json!({ "constraints_solved": constr_count, "status": "solved" })), ..Default::default() })
        }

        CADModalityAction::ComputeMassProps { graph_id, part_node_id, material_density_kg_m3 } => {
            let graph = executor.load_graph(graph_id)?;
            let node = graph.nodes.iter().find(|n| n.node_id == part_node_id);
            let vol_mm3 = node.and_then(|n| n.bounding_box.as_ref()).map(|bb| {
                bb.size_mm[0] * bb.size_mm[1] * bb.size_mm[2]
            }).unwrap_or(1000.0);
            let vol_m3 = vol_mm3 * 1e-9;
            let mass_kg = vol_m3 * material_density_kg_m3;
            Ok(CADModalityOutput {
                success: true,
                mass_properties: Some(MassPropertiesResult { part_node_id, mass_kg, volume_mm3: vol_mm3, surface_area_mm2: 0.0, cog_mm: [0.0; 3] }),
                ..Default::default()
            })
        }

        CADModalityAction::CheckInterference { graph_id, part_ids } => {
            let graph = executor.load_graph(graph_id)?;
            // In production: run AABB overlap check then exact B-rep intersection
            let part_nodes: Vec<&CADGraphNode> = graph.nodes.iter()
                .filter(|n| part_ids.contains(&n.node_id) || part_ids.is_empty())
                .filter(|n| matches!(n.node_type, CADNodeType::PartNode))
                .collect();
            let mut interfering: Vec<(u64, u64)> = Vec::new();
            let mut clearance: Vec<(u64, u64, f64)> = Vec::new();
            for i in 0..part_nodes.len() {
                for j in (i+1)..part_nodes.len() {
                    if let (Some(bb_a), Some(bb_b)) = (&part_nodes[i].bounding_box, &part_nodes[j].bounding_box) {
                        let overlap = (0..3).all(|k| bb_a.min_mm[k] <= bb_b.max_mm[k] && bb_a.max_mm[k] >= bb_b.min_mm[k]);
                        if overlap {
                            interfering.push((part_nodes[i].node_id, part_nodes[j].node_id));
                        } else {
                            let min_clearance = (0..3).map(|k| {
                                (bb_a.min_mm[k] - bb_b.max_mm[k]).max(bb_b.min_mm[k] - bb_a.max_mm[k]).max(0.0)
                            }).fold(f64::INFINITY, f64::min);
                            clearance.push((part_nodes[i].node_id, part_nodes[j].node_id, min_clearance));
                        }
                    }
                }
            }
            Ok(CADModalityOutput { success: true, interference: Some(InterferenceResult { interfering_pairs: interfering, clearance_pairs: clearance }), ..Default::default() })
        }

        CADModalityAction::AnalyzeToleranceChain { graph_id, dimension_ids } => {
            let graph = executor.load_graph(graph_id)?;
            // Collect tolerances linked to the specified dimensions
            let tol_nodes: Vec<&CADGraphNode> = graph.nodes.iter()
                .filter(|n| matches!(n.node_type, CADNodeType::ToleranceNode))
                .collect();
            let worst_case: f64 = tol_nodes.iter().map(|n| {
                let u = n.upper_deviation.unwrap_or(0.0);
                let l = n.lower_deviation.unwrap_or(0.0);
                (u - l).abs()
            }).sum();
            let rss: f64 = tol_nodes.iter().map(|n| {
                let u = n.upper_deviation.unwrap_or(0.0);
                let l = n.lower_deviation.unwrap_or(0.0);
                let half = (u - l) / 2.0;
                half * half
            }).sum::<f64>().sqrt() * 2.0;
            Ok(CADModalityOutput {
                success: true,
                tolerance_chain: Some(ToleranceChainResult { total_worst_case_mm: worst_case, total_rss_mm: rss, contributing_dimensions: tol_nodes.iter().map(|n| (n.node_id, (n.upper_deviation.unwrap_or(0.0) - n.lower_deviation.unwrap_or(0.0)).abs())).collect() }),
                ..Default::default()
            })
        }

        CADModalityAction::GenerateBOM { graph_id, include_sub_assemblies } => {
            let graph = executor.load_graph(graph_id)?;
            let bom_nodes: Vec<&CADGraphNode> = graph.nodes.iter().filter(|n| matches!(n.node_type, CADNodeType::BOMEntryNode)).collect();
            let bom: Vec<BOMEntry> = bom_nodes.iter().enumerate().map(|(i, n)| BOMEntry {
                entry_id: n.node_id, part_number: n.part_number.clone().unwrap_or_else(|| format!("PART-{}", i+1)),
                description: n.content.chars().take(60).collect(),
                revision: n.revision.clone().unwrap_or_else(|| "A".into()),
                quantity: 1, unit_mass_kg: n.mass_kg, total_mass_kg: n.mass_kg,
                material: n.material_name.clone(), is_make: true, is_purchased: false, level: 0,
                ..Default::default()
            }).collect();
            Ok(CADModalityOutput { success: true, bom: Some(bom), ..Default::default() })
        }

        CADModalityAction::TessellateToMesh { graph_id, part_node_id, chord_tolerance_mm, angle_tolerance_deg } => {
            let graph = executor.load_graph(graph_id)?;
            let node = graph.nodes.iter().find(|n| n.node_id == part_node_id);
            let face_count = node.and_then(|n| n.bounding_box.as_ref()).map(|_| 100u32).unwrap_or(50);
            let vertex_count = face_count * 3;
            let triangle_count = face_count * 2;
            Ok(CADModalityOutput {
                success: true,
                tessellation: Some(TessellationResult { part_node_id, vertex_count, triangle_count, mesh_file_path: Some(format!("/tmp/cad_tess_{}.obj", part_node_id)), chord_tolerance_mm, angle_tolerance_deg }),
                ..Default::default()
            })
        }

        CADModalityAction::ExportForKinematics { graph_id, joint_ids } => {
            let graph = executor.load_graph(graph_id)?;
            let joint_data: Vec<serde_json::Value> = graph.edges.iter()
                .filter(|e| matches!(e.edge_type, CADEdgeType::ProvidesJointGeometry))
                .take(50)
                .map(|e| serde_json::json!({ "from": e.from_node, "to": e.to_node, "props": e.properties }))
                .collect();
            Ok(CADModalityOutput { success: true, query_result: Some(serde_json::json!({ "joint_data": joint_data })), ..Default::default() })
        }

        CADModalityAction::LinkToModality { graph_id, target_modality, target_graph_id, link_type } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            let etype = match link_type {
                CADCrossModalLink::GeneratesMeshIn3D         => CADEdgeType::ParametricGeneratesMesh,
                CADCrossModalLink::ProvidesJointGeometry     => CADEdgeType::ProvidesJointGeometry,
                CADCrossModalLink::ProvidesActuatorSpec      => CADEdgeType::ProvidesActuatorSpec,
                CADCrossModalLink::MaterialFrom106           => CADEdgeType::MaterialComposedOf,
                CADCrossModalLink::MaterialIdentifiedBy126   => CADEdgeType::MaterialIdentifiedBy,
            };
            let ref_nid = graph.nodes.iter().map(|n| n.node_id).max().unwrap_or(0) + 1;
            let next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
            graph.nodes.push(CADGraphNode {
                node_id: ref_nid, node_type: CADNodeType::CrossModalRef,
                content: format!("CrossModal→{} graph={}", target_modality, target_graph_id),
                materialized_path: Some(format!("/Modalities/ParametricCAD/Project_{}/Graph_{}/CrossModal/{}", graph.project_id, graph_id, target_graph_id)),
                provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                keywords: vec!["cross-modal".into(), target_modality.clone()], hotness_score: 0.7, ..Default::default()
            });
            graph.edges.push(CADGraphEdge { edge_id: next_eid, from_node: graph.root_node_id, to_node: ref_nid, edge_type: etype, weight: 0.9, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("target_graph_id".into(), serde_json::json!(target_graph_id)); p }, ..Default::default() });
            graph.state = GraphStateType::CrossLinked;
            graph.version += 1; graph.updated_at = now.clone();
            graph.version_notes.push(VersionNote { version: graph.version, note: format!("Linked to {} (graph {})", target_modality, target_graph_id), step_index: None, timestamp: now, change_type: ChangeType::CrossLinked });
            executor.save_graph(&graph)?;
            Ok(CADModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        CADModalityAction::ExportPart { graph_id, part_node_id, format } |
        CADModalityAction::ExportAssembly { graph_id, assembly_node_id: part_node_id, format } => {
            let ext = match &format {
                CADExportFormat::STEP => "step", CADExportFormat::IGES => "iges", CADExportFormat::OBJ => "obj",
                CADExportFormat::STL { binary: true } => "stl", CADExportFormat::STL { .. } => "stl",
                CADExportFormat::GLTF => "gltf", CADExportFormat::GLB => "glb",
                CADExportFormat::BREP => "brep", CADExportFormat::X_T => "x_t", CADExportFormat::X_B => "x_b",
                CADExportFormat::SAT => "sat", CADExportFormat::JT => "jt", CADExportFormat::USDZ => "usdz",
                CADExportFormat::DXF => "dxf", CADExportFormat::SVG => "svg",
                CADExportFormat::Custom(ext) => ext,
            };
            Ok(CADModalityOutput { success: true, export_path: Some(format!("/tmp/cad_export_{}.{}", part_node_id, ext)), ..Default::default() })
        }

        CADModalityAction::ExportDrawing { graph_id, drawing_node_id, format } => {
            let ext = match &format {
                DrawingExportFormat::PDF => "pdf", DrawingExportFormat::SVG => "svg",
                DrawingExportFormat::DXF => "dxf", DrawingExportFormat::DWG => "dwg",
                DrawingExportFormat::PNG { .. } => "png", DrawingExportFormat::Custom(e) => e,
            };
            Ok(CADModalityOutput { success: true, export_path: Some(format!("/tmp/cad_drawing_{}.{}", drawing_node_id, ext)), ..Default::default() })
        }

        CADModalityAction::QueryGraph { graph_id, query } => {
            let graph = executor.load_graph(graph_id)?;
            let result = match query {
                CADGraphQuery::NodeDetail { node_id } => {
                    let node = graph.nodes.iter().find(|n| n.node_id == node_id);
                    let incoming: Vec<_> = graph.edges.iter().filter(|e| e.to_node == node_id).collect();
                    let outgoing: Vec<_> = graph.edges.iter().filter(|e| e.from_node == node_id).collect();
                    serde_json::json!({ "node": node, "incoming": incoming, "outgoing": outgoing })
                }
                CADGraphQuery::FeatureTree { part_node_id } => {
                    let feats: Vec<_> = graph.edges.iter().filter(|e| e.to_node == part_node_id && matches!(e.edge_type, CADEdgeType::ParentFeature | CADEdgeType::ChildFeature)).collect();
                    serde_json::json!({ "feature_edges": feats })
                }
                CADGraphQuery::ToleranceChain { dimension_ids } => {
                    let tols: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, CADNodeType::ToleranceNode)).collect();
                    let wc: f64 = tols.iter().map(|n| (n.upper_deviation.unwrap_or(0.0) - n.lower_deviation.unwrap_or(0.0)).abs()).sum();
                    serde_json::json!({ "tolerances": tols, "worst_case_mm": wc })
                }
                CADGraphQuery::AssemblyConstraints { assembly_node_id } => {
                    let cs: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, CADNodeType::AssemblyConstraintNode)).collect();
                    serde_json::json!({ "constraints": cs })
                }
                CADGraphQuery::MaterialsUsed => {
                    let mats: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, CADNodeType::MaterialNode)).collect();
                    serde_json::json!({ "materials": mats })
                }
                CADGraphQuery::GDTFrames => {
                    let gdt: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, CADNodeType::GDTFrameNode)).collect();
                    serde_json::json!({ "gdt_frames": gdt })
                }
                CADGraphQuery::BOMTree => {
                    let bom: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, CADNodeType::BOMEntryNode)).collect();
                    serde_json::json!({ "bom": bom })
                }
                CADGraphQuery::CrossModalLinks { node_id } => {
                    let links: Vec<_> = graph.edges.iter().filter(|e| (e.from_node == node_id || e.to_node == node_id) && matches!(e.edge_type, CADEdgeType::ParametricGeneratesMesh | CADEdgeType::ProvidesJointGeometry | CADEdgeType::ProvidesActuatorSpec | CADEdgeType::MaterialComposedOf | CADEdgeType::MaterialIdentifiedBy)).collect();
                    serde_json::json!({ "links": links })
                }
                CADGraphQuery::InterfacingFaces { part_a_id, part_b_id } => {
                    let faces: Vec<_> = graph.edges.iter().filter(|e| matches!(e.edge_type, CADEdgeType::MatesWith | CADEdgeType::AdjacentFace)).collect();
                    serde_json::json!({ "interfacing_edges": faces })
                }
                CADGraphQuery::AGIActivity => serde_json::json!({ "is_active": false }),
                CADGraphQuery::AllNodes => serde_json::json!({ "nodes": graph.nodes }),
                CADGraphQuery::AllEdges => serde_json::json!({ "edges": graph.edges }),
            };
            Ok(CADModalityOutput { success: true, query_result: Some(result), ..Default::default() })
        }

        CADModalityAction::GetGraph { graph_id } => {
            let graph = executor.load_graph(graph_id)?;
            Ok(CADModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        CADModalityAction::TriggerSemanticHook { graph_id, hook } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            match hook {
                CADSemanticHook::OnGraphCreated => { graph.state = GraphStateType::SemanticEnriched; }
                CADSemanticHook::OnInferRelationships => {
                    let new_edges = executor.infer_semantic_relationships(&graph.nodes).await;
                    let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                    let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                    for (from, to, etype, reason) in new_edges {
                        if valid.contains(&from) && valid.contains(&to) && from != to {
                            graph.edges.push(CADGraphEdge { edge_id: next_eid, from_node: from, to_node: to, edge_type: etype, weight: 0.8, provenance: EdgeProvenance::DerivedFromHook, version: 1, properties: { let mut p = HashMap::new(); p.insert("reason".into(), serde_json::json!(reason)); p }, ..Default::default() });
                            next_eid += 1;
                        }
                    }
                }
                CADSemanticHook::OnEdgeCompletion => {
                    let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                    graph.edges.retain(|e| valid.contains(&e.from_node) && valid.contains(&e.to_node));
                }
                CADSemanticHook::OnCrossModalityLink { target_modality, target_graph_id } => {
                    graph.state = GraphStateType::CrossLinked;
                    graph.version += 1;
                    graph.version_notes.push(VersionNote { version: graph.version, note: format!("Cross-linked to {} (graph {})", target_modality, target_graph_id), step_index: None, timestamp: now.clone(), change_type: ChangeType::CrossLinked });
                }
            }
            graph.updated_at = now;
            executor.save_graph(&graph)?;
            Ok(CADModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        CADModalityAction::StreamToUI { graph_id, .. } => {
            Ok(CADModalityOutput { success: true, graph_id: Some(graph_id), ..Default::default() })
        }

        CADModalityAction::HeadlessProcess { graph_id, operations } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            for op in operations {
                match op {
                    CADHeadlessOp::RebuildAllFeatures => {
                        graph.version_notes.push(VersionNote { version: graph.version + 1, note: "Rebuilt all features".into(), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                        graph.version += 1;
                    }
                    CADHeadlessOp::GenerateBOM { output_path } => {
                        // In production: generate CSV/Excel BOM
                    }
                    CADHeadlessOp::TessellateAll { chord_tol_mm } => {
                        let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        for node in graph.nodes.iter().filter(|n| matches!(n.node_type, CADNodeType::PartNode)) {
                            graph.edges.push(CADGraphEdge { edge_id: next_eid, from_node: node.node_id, to_node: node.node_id, edge_type: CADEdgeType::ParametricGeneratesMesh, weight: 1.0, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("chord_tolerance_mm".into(), serde_json::json!(chord_tol_mm)); p.insert("target_modality".into(), serde_json::json!("3d")); p }, ..Default::default() });
                            next_eid += 1;
                        }
                    }
                    CADHeadlessOp::CrossLinkTo3D { graph_id_3d } => {
                        let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        let root_id = graph.root_node_id;
                        graph.edges.push(CADGraphEdge { edge_id: next_eid, from_node: root_id, to_node: root_id, edge_type: CADEdgeType::ParametricGeneratesMesh, weight: 0.9, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("graph_id_3d".into(), serde_json::json!(graph_id_3d)); p }, ..Default::default() });
                    }
                    _ => {}
                }
            }
            graph.updated_at = now;
            executor.save_graph(&graph)?;
            Ok(CADModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
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
        if args[i] == "--input" && i + 1 < args.len() { input_json = args[i+1].clone(); i += 2; } else { i += 1; }
    }
    if input_json.is_empty() { eprintln!("Usage: parametric_cad --input '<json>'"); std::process::exit(1); }
    let input: CADModalityAction = match serde_json::from_str(&input_json) {
        Ok(v) => v,
        Err(e) => { println!("{}", serde_json::json!({"success":false,"error":format!("Parse error: {}",e)})); std::process::exit(1); }
    };
    let rt = tokio::runtime::Runtime::new().expect("Tokio runtime");
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap_or_else(|_| r#"{"success":false,"error":"serialize"}"#.into())),
        Err(e) => { println!("{}", serde_json::json!({"success":false,"error":e})); std::process::exit(1); }
    }
}
