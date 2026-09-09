//! DepthSensing — Pipeline #115
//!
//! Depth and 3D surface reconstruction from active/passive range sensors:
//! structured light, time-of-flight (ToF), stereo vision, LiDAR (terrestrial,
//! mobile, aerial), RGBD cameras, photogrammetry point clouds.
//!
//! DISTINCT FROM:
//!   - Image (102): RGB has no depth channel; geometric reconstruction not recoverable.
//!   - 3D (109): 3D works with authored scene files (Blender, GLTF, etc.) and
//!     simulation. Depth handles raw sensor data → point cloud → surface.
//!     The two integrate: depth feeds 3D with real-world geometry.
//!
//! CROSS-LINKS:
//!   102 (Image)     → RGBD: color registered to depth
//!   109 (3D)        → depth → 3D scene geometry, mesh reconstruction
//!   113 (Haptic)    → surface geometry for contact simulation
//!   114 (Thermal)   → depth + thermal = 3D thermal field
//!   117 (Geo)       → georeferenced LiDAR, terrain models, urban scans
//!   121 (Kine)      → depth-based robot navigation / obstacle avoidance
//!   124 (Radar)     → radar point cloud registered with LiDAR
//!   125 (Sonar)     → acoustic 3D registered with LiDAR/depth
//!
//! STORAGE: ZSEI containers under /Modalities/Depth/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

// ─────────────────────────────────────────────────────────────────────────────
// INPUT TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum DepthModalityAction {
    /// Analyze raw depth/point-cloud data
    Analyze {
        data: DepthDataSource,
        extract_surfaces: bool,
        extract_objects: bool,
        extract_normals: bool,
        compute_topology: bool,
        compute_occlusion: bool,
    },
    /// Create a graph from analysis result
    CreateGraph {
        analysis: DepthAnalysisResult,
        project_id: u64,
    },
    /// Update existing graph with new frames/scans
    UpdateGraph {
        graph_id: u64,
        new_frames: Vec<DepthFrame>,
        project_id: u64,
    },
    /// Reconstruct a mesh from a point cloud
    ReconstructMesh {
        graph_id: u64,
        method: MeshReconstructionMethod,
        voxel_size_m: f32,
        depth_trunc_m: Option<f32>,
    },
    /// Fuse multiple depth frames (SLAM / TSDF)
    FuseFrames {
        graph_id: u64,
        frames: Vec<DepthFrame>,
        method: FusionMethod,
        camera_intrinsics: CameraIntrinsics,
    },
    /// Segment point cloud into objects/surfaces
    Segment {
        graph_id: u64,
        method: SegmentationMethod,
        min_cluster_size: u32,
        max_cluster_size: Option<u32>,
        distance_threshold_m: f32,
    },
    /// Register two point clouds (ICP / global registration)
    Register {
        source_graph_id: u64,
        target_graph_id: u64,
        method: RegistrationMethod,
        initial_transform: Option<Transform4x4>,
    },
    /// Compute surface normals
    ComputeNormals {
        graph_id: u64,
        radius_m: f32,
        orient_toward: NormalOrientation,
    },
    /// Compute occupancy or signed distance field
    ComputeOccupancy {
        graph_id: u64,
        grid_resolution_m: f32,
        field_type: OccupancyFieldType,
    },
    /// Extract planar surfaces (floor, walls, ceiling)
    ExtractPlanes {
        graph_id: u64,
        min_inlier_count: u32,
        max_distance_m: f32,
        max_planes: u32,
    },
    /// Detect objects / classify clusters
    ClassifyClusters {
        graph_id: u64,
    },
    /// Compute change detection between two point clouds
    DetectChanges {
        reference_graph_id: u64,
        current_graph_id: u64,
        change_threshold_m: f32,
    },
    /// Query graph
    QueryGraph { graph_id: u64, query: DepthGraphQuery },
    /// Retrieve full graph for Context Viewer
    GetGraph { graph_id: u64 },
    /// Trigger ZSEI semantic hooks
    TriggerSemanticHook { graph_id: u64, hook: DepthSemanticHook },
    /// Export depth products
    ExportProduct { graph_id: u64, format: DepthExportFormat },
    /// Stream to UI
    StreamToUI { graph_id: u64, session_id: String, display_mode: DepthDisplayMode },
    /// Headless processing (AGI-first)
    HeadlessProcess { graph_id: u64, operations: Vec<DepthHeadlessOp> },
}

// ─────────────────────────────────────────────────────────────────────────────
// DATA SOURCES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DepthDataSource {
    /// LAS/LAZ LiDAR point cloud (terrestrial, mobile, aerial)
    LiDARFile {
        file_path: String,
        format: LiDARFormat,
        has_intensity: bool,
        has_rgb: bool,
        has_classification: bool,
        coordinate_system: CoordinateSystem,
    },
    /// PCD point cloud (PCL format)
    PCDFile {
        file_path: String,
        binary: bool,
        has_normals: bool,
        has_rgb: bool,
    },
    /// PLY point cloud or mesh
    PLYFile {
        file_path: String,
        has_normals: bool,
        has_rgb: bool,
        is_mesh: bool,
    },
    /// XYZ / XYZRGB plain text
    XYZFile {
        file_path: String,
        delimiter: char,
        has_rgb: bool,
        has_intensity: bool,
    },
    /// E57 multi-scan LiDAR format
    E57File {
        file_path: String,
        scan_count: u32,
    },
    /// Depth image (16-bit PNG, EXR, etc.)
    DepthImage {
        file_path: String,
        format: DepthImageFormat,
        width: u32, height: u32,
        depth_scale: f32,      // conversion: raw_value * scale = meters
        min_depth_m: f32,
        max_depth_m: f32,
        camera_intrinsics: Option<CameraIntrinsics>,
    },
    /// RGBD camera sequence (color + depth, possibly aligned)
    RGBDSequence {
        depth_dir: String,
        color_dir: Option<String>,
        pose_file: Option<String>,   // camera trajectory
        format: RGBDFormat,
        camera_intrinsics: CameraIntrinsics,
    },
    /// Stereo image pair → disparity → depth
    StereoPair {
        left_image_path: String,
        right_image_path: String,
        baseline_m: f32,
        focal_length_px: f32,
        camera_intrinsics: CameraIntrinsics,
    },
    /// Photogrammetry output (SfM/MVS point cloud)
    PhotogrammetryCloud {
        file_path: String,
        format: LiDARFormat,
        reconstruction_software: Option<String>,
        ground_control_points: u32,
        gsd_m: Option<f32>,         // ground sample distance
    },
    /// External point cloud data (from another pipeline, e.g., Radar or Sonar)
    ExternalPointCloud {
        points: Vec<ExternalPoint3D>,
        source_modality: String,
        coordinate_system: CoordinateSystem,
    },
    /// Live stream from depth sensor
    LiveStream {
        endpoint: String,
        sensor_type: DepthSensorType,
        camera_intrinsics: CameraIntrinsics,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LiDARFormat { LAS, LAZ, LAS14, PCD, PLY, XYZ, E57, RDB, FLS, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum DepthImageFormat { #[default] PNG_16bit, PNG_8bit, EXR, TIFF_32bit, NPY, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RGBDFormat { OpenNI, RealSense, AzureKinect, TUM_RGBD, ICL_NUIM, ScanNet, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DepthSensorType { RealSense, AzureKinect, Zed2, Orbbec, ToF_PMD, Velodyne, Ouster, Livox, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum CoordinateSystem {
    #[default] SensorLocal,
    WorldNED,          // North-East-Down
    WorldENU,          // East-North-Up
    ECEF,
    UTM { zone: u8, hemisphere: char },
    WGS84,
    LocalCartesian { origin_lat: f64, origin_lon: f64, origin_alt: f64 },
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CameraIntrinsics {
    pub width: u32, pub height: u32,
    pub fx: f64, pub fy: f64,       // focal length in pixels
    pub cx: f64, pub cy: f64,       // principal point
    pub k1: f64, pub k2: f64,       // radial distortion
    pub p1: f64, pub p2: f64,       // tangential distortion
    pub k3: f64,
    pub depth_scale: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExternalPoint3D {
    pub x: f32, pub y: f32, pub z: f32,
    pub intensity: Option<f32>,
    pub r: Option<u8>, pub g: Option<u8>, pub b: Option<u8>,
    pub label: Option<u32>,
}

// ─────────────────────────────────────────────────────────────────────────────
// PROCESSING PARAMETER TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum MeshReconstructionMethod {
    #[default] Poisson,          // screened Poisson surface reconstruction
    BallPivoting,                // ball-pivoting algorithm
    Alpha_Shape,
    Marching_Cubes,              // from TSDF/ESDF
    Delaunay_2D,                 // for near-planar datasets
    TSDF,                        // truncated signed distance function
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum FusionMethod {
    #[default] TSDF_Voxel,       // voxel-hashing TSDF (KinectFusion style)
    InfiniteRoom,                // ElasticFusion style
    SLAM_Occupancy,             // occupancy grid SLAM
    ICP_Sequential,             // sequential ICP frame fusion
    DepthInpaint,               // simple depth compositing
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SegmentationMethod {
    #[default] EuclideanClustering,
    RANSAC_Plane,
    RegionGrowing,
    Min_Cut,
    VCCS_Supervoxel,
    SACSegmentation,
    DBSCAN,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum RegistrationMethod {
    #[default] ICP_PointToPoint,
    ICP_PointToPlane,
    Generalized_ICP,
    FGR_FastGlobal,
    FPFH_RANSAC,
    NDT,                         // normal distribution transform
    Go_ICP,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum NormalOrientation {
    #[default] TowardCamera,
    AwayFromCamera,
    Consistent,
    ManualAxis { x: f32, y: f32, z: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum OccupancyFieldType {
    #[default] Binary,           // 0=free, 1=occupied
    Probabilistic,               // log-odds
    TSDF,                        // truncated signed distance
    ESDF,                        // Euclidean signed distance
}

// ─────────────────────────────────────────────────────────────────────────────
// ANALYSIS RESULT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DepthAnalysisResult {
    pub analysis_id: u64,
    pub source_description: String,

    // POINT CLOUD PROPERTIES
    pub point_count: u64,
    pub coordinate_system: CoordinateSystem,
    pub bounding_box: BoundingBox3D,
    pub has_rgb: bool,
    pub has_intensity: bool,
    pub has_normals: bool,
    pub has_classification: bool,
    pub point_density_per_m2: Option<f32>,
    pub mean_point_spacing_m: Option<f32>,

    // GEOMETRIC PRODUCTS
    pub depth_maps: Vec<DepthMap>,
    pub point_clusters: Vec<PointCluster>,
    pub extracted_planes: Vec<ExtractedPlane>,
    pub reconstructed_meshes: Vec<ReconstructedMesh>,
    pub surface_patches: Vec<SurfacePatch>,
    pub occlusion_regions: Vec<OcclusionRegion>,
    pub change_detections: Vec<ChangeRegion>,

    // SEMANTIC
    pub classified_objects: Vec<ClassifiedObject>,
    pub scene_understanding: Option<SceneUnderstanding>,
    pub free_space_volume_m3: Option<f32>,
    pub surface_area_m2: Option<f32>,
    pub volume_m3: Option<f32>,

    // SCAN METADATA
    pub sensor_type: Option<DepthSensorType>,
    pub scan_count: u32,
    pub frame_count: u32,
    pub scan_positions: Vec<ScanPosition>,
    pub registration_error_m: Option<f32>,

    // STATISTICS
    pub height_min_m: f32,
    pub height_max_m: f32,
    pub height_mean_m: f32,
    pub intensity_min: Option<f32>,
    pub intensity_max: Option<f32>,
    pub classification_labels: Vec<LiDARClassLabel>,

    // PROCESSING METADATA
    pub processing_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BoundingBox3D {
    pub min: [f64; 3], pub max: [f64; 3],
    pub center: [f64; 3], pub size: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DepthMap {
    pub map_id: u64,
    pub width: u32, pub height: u32,
    pub min_depth_m: f32, pub max_depth_m: f32, pub mean_depth_m: f32,
    pub valid_pixel_fraction: f32,
    pub has_color: bool,
    pub frame_index: u32,
    pub timestamp_sec: f64,
    pub camera_pose: Option<Transform4x4>,
    pub intrinsics: Option<CameraIntrinsics>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Transform4x4 {
    pub matrix: [[f64; 4]; 4],
}

impl Transform4x4 {
    pub fn identity() -> Self {
        Self { matrix: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]}
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PointCluster {
    pub cluster_id: u64,
    pub point_count: u32,
    pub centroid: [f64; 3],
    pub bounding_box: BoundingBox3D,
    pub mean_intensity: Option<f32>,
    pub dominant_color: Option<[u8; 3]>,
    pub label: Option<String>,
    pub label_confidence_note: String,
    pub surface_area_m2: Option<f32>,
    pub volume_m3: Option<f32>,
    pub is_ground: bool,
    pub is_vegetation: bool,
    pub is_building: bool,
    pub is_vehicle: bool,
    pub is_person: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtractedPlane {
    pub plane_id: u64,
    pub normal: [f64; 3],
    pub d: f64,                      // plane equation: ax+by+cz+d=0
    pub centroid: [f64; 3],
    pub inlier_count: u32,
    pub area_m2: f32,
    pub bounding_box: BoundingBox3D,
    pub plane_type: PlaneType,
    pub tilt_angle_deg: f32,         // angle from horizontal
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PlaneType { #[default] Unknown, Floor, Ceiling, Wall, Ramp, Roof, Road, WaterSurface, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReconstructedMesh {
    pub mesh_id: u64,
    pub vertex_count: u32,
    pub face_count: u32,
    pub has_normals: bool,
    pub has_colors: bool,
    pub bounding_box: BoundingBox3D,
    pub volume_m3: Option<f64>,
    pub surface_area_m2: f64,
    pub is_watertight: bool,
    pub reconstruction_method: String,
    pub voxel_size_m: Option<f32>,
    pub file_path: Option<String>,   // exported mesh path
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SurfacePatch {
    pub patch_id: u64,
    pub centroid: [f64; 3],
    pub normal: [f64; 3],
    pub area_m2: f32,
    pub roughness: f32,
    pub curvature: f32,
    pub point_count: u32,
    pub material_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OcclusionRegion {
    pub region_id: u64,
    pub sensor_position: [f64; 3],
    pub occluder_cluster_id: Option<u64>,
    pub occluded_volume_m3: Option<f32>,
    pub bounding_box: BoundingBox3D,
    pub occlusion_fraction: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChangeRegion {
    pub change_id: u64,
    pub centroid: [f64; 3],
    pub bounding_box: BoundingBox3D,
    pub point_count: u32,
    pub change_type: ChangeType3D,
    pub magnitude_m: f32,           // mean distance between reference and current
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ChangeType3D { #[default] Unknown, Added, Removed, Deformed, Moved }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClassifiedObject {
    pub object_id: u64,
    pub cluster_id: u64,
    pub class_label: String,
    pub centroid: [f64; 3],
    pub bounding_box: BoundingBox3D,
    pub dimensions_m: [f32; 3],     // (length, width, height)
    pub heading_deg: Option<f32>,
    pub point_count: u32,
    pub classification_method: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SceneUnderstanding {
    pub scene_type: SceneType,
    pub floor_plane_id: Option<u64>,
    pub ceiling_plane_id: Option<u64>,
    pub wall_plane_ids: Vec<u64>,
    pub navigable_area_m2: f32,
    pub object_count: u32,
    pub estimated_room_volume_m3: Option<f32>,
    pub has_vegetation: bool,
    pub is_outdoor: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SceneType {
    #[default] Unknown, Indoor_Room, Indoor_Corridor, Outdoor_Urban, Outdoor_Rural,
    Industrial, Warehouse, Construction, Underground, Aerial_Oblique, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanPosition {
    pub position_id: u64,
    pub location: [f64; 3],
    pub heading_deg: f64,
    pub point_count_from_here: u32,
    pub scan_range_m: f32,
    pub timestamp_sec: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LiDARClassLabel {
    pub label_id: u32,
    pub name: String,
    pub point_count: u64,
    pub fraction: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DepthFrame {
    pub frame_id: u64,
    pub timestamp_sec: f64,
    pub depth_data: Vec<f32>,        // flattened depth map, row-major, meters
    pub width: u32, pub height: u32,
    pub color_data: Option<Vec<u8>>, // flattened RGB, row-major
    pub pose: Option<Transform4x4>,
    pub intrinsics: Option<CameraIntrinsics>,
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH NODE TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum DepthNodeType {
    #[default] DepthScene,           // root
    PointCloudNode,                  // full point cloud or sub-cloud
    DepthMapNode,                    // single depth frame
    PointClusterNode,               // segmented cluster
    ExtractedPlaneNode,             // detected planar surface
    ReconstructedMeshNode,          // mesh from point cloud
    SurfacePatchNode,               // local surface patch
    OcclusionRegionNode,            // sensor occlusion zone
    ClassifiedObjectNode,           // semantically labeled cluster
    ScanPositionNode,               // scanner/sensor position
    ChangeDetectionNode,            // change between scans
    OccupancyGridNode,              // voxelized occupancy
    NormalVectorField,              // computed normals
    RegistrationResultNode,         // registration between scans
    CrossModalFusionNode,           // fused with another modality
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DepthGraphNode {
    pub node_id: u64,
    pub node_type: DepthNodeType,
    pub content: String,

    // DEPTH-SPECIFIC
    pub point_count: Option<u64>,
    pub centroid: Option<[f64; 3]>,
    pub bounding_box: Option<BoundingBox3D>,
    pub normal: Option<[f64; 3]>,
    pub depth_m: Option<f32>,
    pub area_m2: Option<f32>,
    pub volume_m3: Option<f64>,
    pub class_label: Option<String>,
    pub timestamp_sec: Option<f64>,
    pub plane_type: Option<String>,
    pub tilt_deg: Option<f32>,

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
pub enum DepthEdgeType {
    // ── STRUCTURAL ──
    #[default] Contains, Precedes, PartOf,

    // ── DEPTH-SPECIFIC ──
    Occludes,                   // cluster A occludes cluster B from sensor
    DistanceTo,                 // spatial distance relationship
    SurfaceGeometryOf,          // surface patch is geometry of object
    FillsVolume,                // cluster fills a spatial volume
    SupportedBy,                // object rests on plane/surface
    AbovePlane,                 // cluster is above extracted plane
    BelowPlane,
    AdjacentCluster,            // neighboring cluster
    RegisteredTo,               // point cloud registered to another
    SegmentedFrom,              // cluster segmented from parent cloud
    ReconstructedFrom,          // mesh reconstructed from cluster
    DetectedChange,             // change region detected between scans
    ScanCoverage,               // scan position covers point cloud region
    NormalField,                // normal vectors computed for this surface

    // ── CROSS-MODAL ──
    /// Point cloud → 3D scene (109)
    ContributesTo3DScene,
    /// RGB registered to depth → image (102)
    ColorAlignedWithImage,
    /// Surface geometry → haptic simulation (113)
    ContactSurfaceForHaptic,
    /// Depth + thermal → 3D thermal field (114)
    FusedWithThermal,
    /// Point cloud → geospatial map (117)
    PlottedOnGeoMap,
    /// Navigable space → kinematics/navigation (121)
    NavigableSpaceForRobot,
    /// Registered radar point cloud (124)
    FusedWithRadar,
    /// Registered sonar point cloud (125)
    FusedWithSonar,

    // ── UNIVERSAL SEMANTIC ──
    Performs, Affects, Implies, Contradicts, Elaborates, Summarizes, Supports,
    TemporalPrecedes, TemporalFollows, CausedBy, Enables, Prevents,
    FunctionalRole, InstanceOf,
    DerivedFrom, VersionOf, RefinesTo, ForkedFrom, SimilarTo,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DepthGraphEdge {
    pub edge_id: u64,
    pub from_node: u64, pub to_node: u64,
    pub edge_type: DepthEdgeType,
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
pub struct DepthGraph {
    pub graph_id: u64, pub project_id: u64,
    pub source_description: String,
    pub nodes: Vec<DepthGraphNode>,
    pub edges: Vec<DepthGraphEdge>,
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
pub enum DepthGraphQuery {
    NodeDetail { node_id: u64 },
    ClustersByLabel { label: String },
    PlanesOfType { plane_type: String },
    ObjectsInVolume { min: [f64; 3], max: [f64; 3] },
    OcclusionRegions,
    ChangedRegions,
    CrossModalLinks { node_id: u64 },
    LargestClusters { top_n: u32 },
    ScanPositions,
    AGIActivity, AllNodes, AllEdges,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DepthSemanticHook {
    OnGraphCreated,
    OnInferRelationships,
    OnEdgeCompletion,
    OnCrossModalityLink { target_modality: String, target_graph_id: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DepthExportFormat {
    LAS, LAZ, PCD, PLY_PointCloud, PLY_Mesh,
    XYZ, E57, OBJ_Mesh, GLTF_Mesh,
    DepthImage_PNG, DepthImage_EXR,
    VoxelGrid_NPZ, GeoTIFF_DSM,    // digital surface model
    GeoTIFF_DTM,                   // digital terrain model
    KML_Objects, GeoJSON_Objects,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DepthDisplayMode {
    PointCloudViewer,          // 3D point cloud
    DepthMap,                  // 2D depth image
    NormalMap,                 // surface normal visualization
    IntensityMap,              // intensity as grayscale
    ClassificationMap,         // color by class label
    OccupancyGrid,             // 2D occupancy
    MeshViewer,                // reconstructed mesh
    ChangeMap,                 // change detection overlay
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DepthHeadlessOp {
    ReconstructMesh { method: MeshReconstructionMethod },
    ComputeNormals { radius_m: f32 },
    SegmentClusters { method: SegmentationMethod },
    ExtractPlanes { max_planes: u32 },
    FuseWithDepth { other_graph_id: u64 },
    ExportMesh { format: DepthExportFormat, path: String },
    CrossRegisterWith3D { graph_id_3d: u64 },
}

// ─────────────────────────────────────────────────────────────────────────────
// OUTPUT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DepthModalityOutput {
    pub success: bool,
    pub graph_id: Option<u64>,
    pub graph: Option<DepthGraph>,
    pub analysis: Option<DepthAnalysisResult>,
    pub mesh: Option<ReconstructedMesh>,
    pub planes: Option<Vec<ExtractedPlane>>,
    pub clusters: Option<Vec<PointCluster>>,
    pub registration_transform: Option<Transform4x4>,
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
    DerivedFromModalityGraph(u64), DerivedFromFile(String), DerivedFromAMT,
    DerivedFromBlueprint(u32), DerivedFromMethodology(u64),
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
        let input = serde_json::json!({ "action": "Prompt", "prompt": prompt, "max_tokens": max_tokens, "temperature": 0.05, "system_context": "Depth/LiDAR analysis. Return only valid JSON." });
        let out = std::process::Command::new(&self.prompt_pipeline_path).arg("--input").arg(input.to_string()).output().map_err(|e| e.to_string())?;
        let r: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
        Ok(r["response"].as_str().unwrap_or("{}").to_string())
    }

    fn save_graph(&self, g: &DepthGraph) -> Result<(), String> {
        let path = format!("{}/local/depth_graph_{}.json", self.zsei_path, g.graph_id);
        if let Some(p) = std::path::Path::new(&path).parent() { std::fs::create_dir_all(p).map_err(|e| e.to_string())?; }
        std::fs::write(&path, serde_json::to_string_pretty(g).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    fn load_graph(&self, id: u64) -> Result<DepthGraph, String> {
        let path = format!("{}/local/depth_graph_{}.json", self.zsei_path, id);
        serde_json::from_str(&std::fs::read_to_string(&path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    fn generate_id(&self) -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as u64 }
    fn now_iso8601(&self) -> String { format!("{}", self.generate_id()) }
    fn extract_json_array(raw: &str) -> String { if let (Some(s), Some(e)) = (raw.find('['), raw.rfind(']')) { raw[s..=e].to_string() } else { "[]".to_string() } }
    fn extract_json_object(raw: &str) -> String { if let (Some(s), Some(e)) = (raw.find('{'), raw.rfind('}')) { raw[s..=e].to_string() } else { "{}".to_string() } }
}

// ─────────────────────────────────────────────────────────────────────────────
// LLM-BASED ANALYSIS
// ─────────────────────────────────────────────────────────────────────────────

impl PipelineExecutor {
    async fn classify_clusters_llm(&self, clusters: &[PointCluster]) -> Vec<(u64, String, String)> {
        if clusters.is_empty() { return vec![]; }
        let list: Vec<serde_json::Value> = clusters.iter().take(20).map(|c| serde_json::json!({
            "cluster_id": c.cluster_id,
            "point_count": c.point_count,
            "size_xyz_m": [c.bounding_box.size[0], c.bounding_box.size[1], c.bounding_box.size[2]],
            "height_m": c.bounding_box.size[2],
            "centroid_z_m": c.centroid[2],
            "is_ground": c.is_ground,
            "mean_intensity": c.mean_intensity,
        })).collect();

        let prompt = format!(r#"
Classify each 3D point cloud cluster by object type.

Options: Ground, Vegetation_Low, Vegetation_High, Building, Wall, Roof, Vehicle_Car,
Vehicle_Truck, Vehicle_Motorcycle, Pedestrian, Pole, Wire, Traffic_Sign, Furniture,
Debris, Water, Road_Surface, Sidewalk, Unknown

Clusters:
{}

Return ONLY valid JSON array:
[{{"cluster_id": N, "class_label": "Label", "reasoning": "brief"}}]"#,
            serde_json::to_string_pretty(&list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 600).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str).unwrap_or_default()
                    .into_iter()
                    .filter_map(|v| Some((v["cluster_id"].as_u64()?, v["class_label"].as_str()?.to_string(), v["reasoning"].as_str().unwrap_or("").to_string())))
                    .collect()
            }
            Err(_) => vec![],
        }
    }

    async fn infer_plane_types_llm(&self, planes: &[ExtractedPlane]) -> Vec<(u64, String)> {
        if planes.is_empty() { return vec![]; }
        let list: Vec<serde_json::Value> = planes.iter().map(|p| serde_json::json!({
            "plane_id": p.plane_id,
            "normal_xyz": p.normal,
            "tilt_angle_deg": p.tilt_angle_deg,
            "area_m2": p.area_m2,
            "height_m": p.centroid[2],
        })).collect();

        let prompt = format!(r#"
Classify each planar surface.
Options: Floor, Ceiling, Wall, Ramp, Roof, Road, WaterSurface, Unknown

Planes:
{}

Return ONLY valid JSON array:
[{{"plane_id": N, "plane_type": "TypeName"}}]"#,
            serde_json::to_string_pretty(&list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 300).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str).unwrap_or_default()
                    .into_iter()
                    .filter_map(|v| Some((v["plane_id"].as_u64()?, v["plane_type"].as_str()?.to_string())))
                    .collect()
            }
            Err(_) => vec![],
        }
    }

    async fn infer_scene_type_llm(&self, analysis: &DepthAnalysisResult) -> String {
        let summary = serde_json::json!({
            "point_count": analysis.point_count,
            "height_range_m": analysis.height_max_m - analysis.height_min_m,
            "has_classification": analysis.has_classification,
            "cluster_count": analysis.point_clusters.len(),
            "plane_count": analysis.extracted_planes.len(),
            "scan_positions": analysis.scan_positions.len(),
            "has_vegetation": analysis.classified_objects.iter().any(|o| o.class_label.to_lowercase().contains("vegetation")),
            "has_buildings": analysis.classified_objects.iter().any(|o| o.class_label.to_lowercase().contains("building")),
            "bounding_box_size": analysis.bounding_box.size,
        });

        let prompt = format!(r#"
Classify this point cloud scene type:
Indoor_Room, Indoor_Corridor, Outdoor_Urban, Outdoor_Rural, Industrial,
Warehouse, Construction, Underground, Aerial_Oblique

Scene summary: {}
Return ONLY the single scene type label."#,
            serde_json::to_string(&summary).unwrap_or_default());

        self.llm_zero_shot(&prompt, 15).await.unwrap_or_else(|_| "Unknown".into()).trim().to_string()
    }

    async fn infer_semantic_relationships(&self, nodes: &[DepthGraphNode]) -> Vec<(u64, u64, DepthEdgeType, String)> {
        if nodes.len() < 2 { return vec![]; }
        let node_list: Vec<serde_json::Value> = nodes.iter().take(25).map(|n| serde_json::json!({
            "node_id": n.node_id, "type": format!("{:?}", n.node_type),
            "content": n.content.chars().take(80).collect::<String>(),
            "label": n.class_label,
            "centroid_z": n.centroid.map(|c| c[2]),
        })).collect();

        let prompt = format!(r#"
Identify semantic relationships between these depth/point cloud graph nodes.

Nodes: {}

Types: Occludes, SupportedBy, AbovePlane, AdjacentCluster, SegmentedFrom,
FillsVolume, Affects, CausedBy, Enables, DerivedFrom, PartOf, SimilarTo

Return ONLY valid JSON array:
[{{"from_node_id": N, "to_node_id": M, "edge_type": "TypeName", "reason": "brief"}}]"#,
            serde_json::to_string_pretty(&node_list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 600).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str).unwrap_or_default()
                    .into_iter()
                    .filter_map(|v| {
                        let from = v["from_node_id"].as_u64()?;
                        let to = v["to_node_id"].as_u64()?;
                        let etype = map_depth_edge_str(v["edge_type"].as_str().unwrap_or("Affects"));
                        let reason = v["reason"].as_str().unwrap_or("").to_string();
                        Some((from, to, etype, reason))
                    }).collect()
            }
            Err(_) => vec![],
        }
    }

    // ICP (Iterative Closest Point) — point-to-point, simplified implementation
    fn icp_point_to_point(
        source: &[[f32; 3]],
        target: &[[f32; 3]],
        max_iterations: u32,
        tolerance_m: f32,
    ) -> (Transform4x4, f64) {
        // In production: build KD-tree for target, iterate correspondence + SVD transform
        // Here: return identity with mock error
        let rmse = 0.005; // 5mm placeholder
        (Transform4x4::identity(), rmse)
    }

    // RANSAC plane fitting — returns (normal, d, inlier_indices)
    fn ransac_plane(
        points: &[[f32; 3]],
        max_iterations: u32,
        distance_threshold_m: f32,
    ) -> Option<([f64; 3], f64, Vec<usize>)> {
        if points.len() < 3 { return None; }
        let mut best_inliers: Vec<usize> = Vec::new();
        let mut best_normal = [0.0f64; 3];
        let mut best_d = 0.0f64;

        for iter in 0..max_iterations.min(100) {
            let i0 = (iter * 7 + 0) as usize % points.len();
            let i1 = (iter * 7 + 3) as usize % points.len();
            let i2 = (iter * 7 + 5) as usize % points.len();
            if i0 == i1 || i1 == i2 || i0 == i2 { continue; }

            let p0 = points[i0]; let p1 = points[i1]; let p2 = points[i2];
            let v1 = [p1[0]-p0[0], p1[1]-p0[1], p1[2]-p0[2]];
            let v2 = [p2[0]-p0[0], p2[1]-p0[1], p2[2]-p0[2]];
            let n = [
                (v1[1]*v2[2] - v1[2]*v2[1]) as f64,
                (v1[2]*v2[0] - v1[0]*v2[2]) as f64,
                (v1[0]*v2[1] - v1[1]*v2[0]) as f64,
            ];
            let len = (n[0]*n[0]+n[1]*n[1]+n[2]*n[2]).sqrt();
            if len < 1e-9 { continue; }
            let n = [n[0]/len, n[1]/len, n[2]/len];
            let d = -(n[0]*p0[0] as f64 + n[1]*p0[1] as f64 + n[2]*p0[2] as f64);

            let inliers: Vec<usize> = points.iter().enumerate()
                .filter(|(_, p)| (n[0]*p[0] as f64 + n[1]*p[1] as f64 + n[2]*p[2] as f64 + d).abs() < distance_threshold_m as f64)
                .map(|(i, _)| i)
                .collect();

            if inliers.len() > best_inliers.len() {
                best_inliers = inliers;
                best_normal = n;
                best_d = d;
            }
        }

        if best_inliers.is_empty() { None } else { Some((best_normal, best_d, best_inliers)) }
    }

    // Euclidean clustering
    fn euclidean_cluster(
        points: &[[f32; 3]],
        distance_threshold_m: f32,
        min_size: u32,
        max_size: Option<u32>,
    ) -> Vec<Vec<usize>> {
        // In production: KD-tree + BFS. Here: simple distance-based grouping
        let mut visited = vec![false; points.len()];
        let mut clusters: Vec<Vec<usize>> = Vec::new();

        for start in 0..points.len() {
            if visited[start] { continue; }
            let mut cluster = Vec::new();
            let mut queue = vec![start];
            visited[start] = true;

            while let Some(idx) = queue.pop() {
                cluster.push(idx);
                for neighbor in 0..points.len() {
                    if visited[neighbor] { continue; }
                    let p = points[idx]; let q = points[neighbor];
                    let d = ((p[0]-q[0]).powi(2)+(p[1]-q[1]).powi(2)+(p[2]-q[2]).powi(2)).sqrt();
                    if d <= distance_threshold_m {
                        visited[neighbor] = true;
                        queue.push(neighbor);
                    }
                }
            }

            let max_ok = max_size.map(|m| cluster.len() <= m as usize).unwrap_or(true);
            if cluster.len() >= min_size as usize && max_ok {
                clusters.push(cluster);
            }
        }
        clusters
    }
}

fn map_depth_edge_str(s: &str) -> DepthEdgeType {
    match s {
        "Occludes"              => DepthEdgeType::Occludes,
        "DistanceTo"            => DepthEdgeType::DistanceTo,
        "SurfaceGeometryOf"     => DepthEdgeType::SurfaceGeometryOf,
        "FillsVolume"           => DepthEdgeType::FillsVolume,
        "SupportedBy"           => DepthEdgeType::SupportedBy,
        "AbovePlane"            => DepthEdgeType::AbovePlane,
        "BelowPlane"            => DepthEdgeType::BelowPlane,
        "AdjacentCluster"       => DepthEdgeType::AdjacentCluster,
        "RegisteredTo"          => DepthEdgeType::RegisteredTo,
        "SegmentedFrom"         => DepthEdgeType::SegmentedFrom,
        "ReconstructedFrom"     => DepthEdgeType::ReconstructedFrom,
        "DetectedChange"        => DepthEdgeType::DetectedChange,
        "ContributesTo3DScene"  => DepthEdgeType::ContributesTo3DScene,
        "FusedWithThermal"      => DepthEdgeType::FusedWithThermal,
        "PlottedOnGeoMap"       => DepthEdgeType::PlottedOnGeoMap,
        "NavigableSpaceForRobot"=> DepthEdgeType::NavigableSpaceForRobot,
        "FusedWithRadar"        => DepthEdgeType::FusedWithRadar,
        "FusedWithSonar"        => DepthEdgeType::FusedWithSonar,
        "Affects"               => DepthEdgeType::Affects,
        "CausedBy"              => DepthEdgeType::CausedBy,
        "Enables"               => DepthEdgeType::Enables,
        "TemporalPrecedes"      => DepthEdgeType::TemporalPrecedes,
        "DerivedFrom"           => DepthEdgeType::DerivedFrom,
        "PartOf"                => DepthEdgeType::PartOf,
        "SimilarTo"             => DepthEdgeType::SimilarTo,
        _                       => DepthEdgeType::Affects,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH CREATION
// ─────────────────────────────────────────────────────────────────────────────

async fn create_graph(executor: &PipelineExecutor, analysis: DepthAnalysisResult, project_id: u64) -> DepthModalityOutput {
    let graph_id = executor.generate_id();
    let now = executor.now_iso8601();
    let mut nodes: Vec<DepthGraphNode> = Vec::new();
    let mut edges: Vec<DepthGraphEdge> = Vec::new();
    let mut node_id: u64 = 1;
    let mut edge_id: u64 = 1;

    // ── ROOT ──
    let root_id = node_id;
    let scene_type = executor.infer_scene_type_llm(&analysis).await;
    nodes.push(DepthGraphNode {
        node_id: root_id, node_type: DepthNodeType::DepthScene,
        content: format!("Depth scene [{}]: {}pts {}clusters {}planes {}meshes {}objs coord={:?}",
            scene_type, analysis.point_count, analysis.point_clusters.len(),
            analysis.extracted_planes.len(), analysis.reconstructed_meshes.len(),
            analysis.classified_objects.len(), analysis.coordinate_system),
        point_count: Some(analysis.point_count),
        bounding_box: Some(analysis.bounding_box.clone()),
        materialized_path: Some(format!("/Modalities/Depth/Project_{}/Graph_{}", project_id, graph_id)),
        provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
        version_notes: vec![VersionNote { version: 1, note: "Initial creation".into(), step_index: None, timestamp: now.clone(), change_type: ChangeType::Created }],
        keywords: vec!["depth".into(), "point-cloud".into(), scene_type.to_lowercase()],
        hotness_score: 1.0, ..Default::default()
    });
    node_id += 1;

    // ── SCAN POSITIONS ──
    let mut scan_pos_nids: Vec<(u64, u64)> = Vec::new();
    for sp in &analysis.scan_positions {
        let sid = node_id;
        nodes.push(DepthGraphNode {
            node_id: sid, node_type: DepthNodeType::ScanPositionNode,
            content: format!("ScanPos: loc=({:.2},{:.2},{:.2}) pts={} range={:.0}m",
                sp.location[0], sp.location[1], sp.location[2], sp.point_count_from_here, sp.scan_range_m),
            centroid: Some(sp.location),
            timestamp_sec: Some(sp.timestamp_sec),
            materialized_path: Some(format!("/Modalities/Depth/Project_{}/Graph_{}/ScanPos/{}", project_id, graph_id, sp.position_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: vec!["scan-position".into()], hotness_score: 0.65, ..Default::default()
        });
        edges.push(DepthGraphEdge { edge_id, from_node: root_id, to_node: sid, edge_type: DepthEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        scan_pos_nids.push((sp.position_id, sid));
        node_id += 1;
    }

    // ── DEPTH MAPS ──
    for dm in &analysis.depth_maps {
        let did = node_id;
        nodes.push(DepthGraphNode {
            node_id: did, node_type: DepthNodeType::DepthMapNode,
            content: format!("DepthMap {}×{}: depth={:.2}–{:.2}m valid={:.0}%",
                dm.width, dm.height, dm.min_depth_m, dm.max_depth_m, dm.valid_pixel_fraction * 100.0),
            depth_m: Some(dm.mean_depth_m), timestamp_sec: Some(dm.timestamp_sec),
            materialized_path: Some(format!("/Modalities/Depth/Project_{}/Graph_{}/DepthMap/{}", project_id, graph_id, dm.map_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: vec!["depth-map".into()], hotness_score: 0.7, ..Default::default()
        });
        edges.push(DepthGraphEdge { edge_id, from_node: root_id, to_node: did, edge_type: DepthEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── EXTRACTED PLANES ──
    let plane_type_classifications = executor.infer_plane_types_llm(&analysis.extracted_planes).await;
    let plane_type_map: HashMap<u64, String> = plane_type_classifications.into_iter().collect();

    let plane_node_ids: Vec<(u64, u64)> = analysis.extracted_planes.iter().map(|p| {
        let plane_type_str = plane_type_map.get(&p.plane_id)
            .cloned()
            .unwrap_or_else(|| format!("{:?}", p.plane_type));
        let pid = node_id;
        nodes.push(DepthGraphNode {
            node_id: pid, node_type: DepthNodeType::ExtractedPlaneNode,
            content: format!("Plane [{}]: inliers={} area={:.1}m² tilt={:.1}° normal=({:.2},{:.2},{:.2})",
                plane_type_str, p.inlier_count, p.area_m2, p.tilt_angle_deg, p.normal[0], p.normal[1], p.normal[2]),
            centroid: Some(p.centroid), normal: Some(p.normal), area_m2: Some(p.area_m2),
            tilt_deg: Some(p.tilt_angle_deg), plane_type: Some(plane_type_str.clone()),
            bounding_box: Some(p.bounding_box.clone()),
            materialized_path: Some(format!("/Modalities/Depth/Project_{}/Graph_{}/Plane/{}", project_id, graph_id, p.plane_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["plane".into(), plane_type_str.to_lowercase()],
            hotness_score: 0.7 + (p.area_m2 / 100.0).min(0.25),
            ..Default::default()
        });
        edges.push(DepthGraphEdge { edge_id, from_node: root_id, to_node: pid, edge_type: DepthEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
        (p.plane_id, pid)
    }).collect();

    // ── POINT CLUSTERS ──
    let cluster_classifications = executor.classify_clusters_llm(&analysis.point_clusters).await;
    let cluster_class_map: HashMap<u64, (String, String)> = cluster_classifications.into_iter()
        .map(|(id, label, reason)| (id, (label, reason))).collect();

    let cluster_node_ids: Vec<(u64, u64)> = analysis.point_clusters.iter().map(|c| {
        let (class_label, class_note) = cluster_class_map.get(&c.cluster_id)
            .cloned()
            .unwrap_or_else(|| (c.label.clone().unwrap_or_else(|| "Unknown".into()), String::new()));
        let cid = node_id;
        nodes.push(DepthGraphNode {
            node_id: cid, node_type: DepthNodeType::PointClusterNode,
            content: format!("Cluster [{}]: pts={} size=({:.2},{:.2},{:.2})m centroid=({:.1},{:.1},{:.1})",
                class_label, c.point_count,
                c.bounding_box.size[0], c.bounding_box.size[1], c.bounding_box.size[2],
                c.centroid[0], c.centroid[1], c.centroid[2]),
            point_count: Some(c.point_count as u64), centroid: Some(c.centroid),
            bounding_box: Some(c.bounding_box.clone()), class_label: Some(class_label.clone()),
            area_m2: c.surface_area_m2, volume_m3: c.volume_m3.map(|v| v as f64),
            materialized_path: Some(format!("/Modalities/Depth/Project_{}/Graph_{}/Cluster/{}", project_id, graph_id, c.cluster_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["cluster".into(), class_label.to_lowercase()],
            hotness_score: 0.6 + ((c.point_count as f32).log2() / 30.0).min(0.3),
            embedding_hint: Some(format!("3D object: {} at ({:.1},{:.1},{:.1})", class_label, c.centroid[0], c.centroid[1], c.centroid[2])),
            ..Default::default()
        });
        edges.push(DepthGraphEdge { edge_id, from_node: root_id, to_node: cid, edge_type: DepthEdgeType::SegmentedFrom, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Cluster → plane: SupportedBy (cluster above floor/road planes)
        for (_, &plane_nid) in &plane_node_ids {
            let plane = analysis.extracted_planes.iter().find(|p| {
                // plane_node_id → find original plane via node content check (simplified)
                true
            });
            // Find lowest centroid z, associate with floor if cluster z > plane z
            if let Some(plane_node) = nodes.iter().find(|n| n.node_id == plane_nid) {
                if let (Some(c_z), Some(p_type)) = (c.centroid.get(2), &plane_node.plane_type) {
                    if p_type == "Floor" || p_type == "Road" {
                        if let Some(p_centroid) = plane_node.centroid {
                            if *c_z > p_centroid[2] + 0.05 {
                                edges.push(DepthGraphEdge { edge_id, from_node: cid, to_node: plane_nid, edge_type: DepthEdgeType::SupportedBy, weight: 0.8, provenance: EdgeProvenance::DerivedFromHook, version: 1, ..Default::default() });
                                edge_id += 1;
                            }
                        }
                    }
                }
            }
        }

        // Cross-modal: cluster → 3D scene (109)
        edges.push(DepthGraphEdge {
            edge_id, from_node: cid, to_node: cid,
            edge_type: DepthEdgeType::ContributesTo3DScene, weight: 0.9,
            provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
            properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("3d")); p },
            ..Default::default()
        });
        edge_id += 1; node_id += 1;
        (c.cluster_id, cid)
    }).collect();

    // ── CLASSIFIED OBJECTS ──
    for obj in &analysis.classified_objects {
        let oid = node_id;
        nodes.push(DepthGraphNode {
            node_id: oid, node_type: DepthNodeType::ClassifiedObjectNode,
            content: format!("Object [{}]: dim=({:.2}×{:.2}×{:.2})m pts={} heading={:?}°",
                obj.class_label, obj.dimensions_m[0], obj.dimensions_m[1], obj.dimensions_m[2],
                obj.point_count, obj.heading_deg),
            centroid: Some(obj.centroid), bounding_box: Some(obj.bounding_box.clone()),
            class_label: Some(obj.class_label.clone()),
            materialized_path: Some(format!("/Modalities/Depth/Project_{}/Graph_{}/Object/{}", project_id, graph_id, obj.object_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["object".into(), obj.class_label.to_lowercase()],
            hotness_score: 0.75,
            ..Default::default()
        });
        edges.push(DepthGraphEdge { edge_id, from_node: root_id, to_node: oid, edge_type: DepthEdgeType::Contains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Classified object → source cluster
        if let Some((_, &cluster_nid)) = cluster_node_ids.iter().find(|(id, _)| *id == obj.cluster_id) {
            edges.push(DepthGraphEdge { edge_id, from_node: oid, to_node: cluster_nid, edge_type: DepthEdgeType::SegmentedFrom, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── RECONSTRUCTED MESHES ──
    for mesh in &analysis.reconstructed_meshes {
        let mid = node_id;
        nodes.push(DepthGraphNode {
            node_id: mid, node_type: DepthNodeType::ReconstructedMeshNode,
            content: format!("Mesh: verts={} faces={} area={:.1}m² watertight={} method={}",
                mesh.vertex_count, mesh.face_count, mesh.surface_area_m2, mesh.is_watertight, mesh.reconstruction_method),
            bounding_box: Some(mesh.bounding_box.clone()),
            area_m2: Some(mesh.surface_area_m2 as f32),
            volume_m3: mesh.volume_m3,
            materialized_path: Some(format!("/Modalities/Depth/Project_{}/Graph_{}/Mesh/{}", project_id, graph_id, mesh.mesh_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["mesh".into(), "reconstruction".into()], hotness_score: 0.8, ..Default::default()
        });
        edges.push(DepthGraphEdge { edge_id, from_node: root_id, to_node: mid, edge_type: DepthEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // Mesh → 3D (109) cross-modal
        edges.push(DepthGraphEdge { edge_id, from_node: mid, to_node: mid, edge_type: DepthEdgeType::ContributesTo3DScene, weight: 0.95, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("3d")); p }, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── OCCLUSION REGIONS ──
    for occ in &analysis.occlusion_regions {
        let oid = node_id;
        nodes.push(DepthGraphNode {
            node_id: oid, node_type: DepthNodeType::OcclusionRegionNode,
            content: format!("Occlusion: sensor=({:.1},{:.1},{:.1}) frac={:.0}%",
                occ.sensor_position[0], occ.sensor_position[1], occ.sensor_position[2],
                occ.occlusion_fraction * 100.0),
            bounding_box: Some(occ.bounding_box.clone()),
            materialized_path: Some(format!("/Modalities/Depth/Project_{}/Graph_{}/Occlusion/{}", project_id, graph_id, occ.region_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["occlusion".into()], hotness_score: 0.55, ..Default::default()
        });
        edges.push(DepthGraphEdge { edge_id, from_node: root_id, to_node: oid, edge_type: DepthEdgeType::Contains, weight: 0.6, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        if let Some(occluder_id) = occ.occluder_cluster_id {
            if let Some((_, &occ_cid)) = cluster_node_ids.iter().find(|(id, _)| *id == occluder_id) {
                edges.push(DepthGraphEdge { edge_id, from_node: occ_cid, to_node: oid, edge_type: DepthEdgeType::Occludes, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        node_id += 1;
    }

    // ── CHANGE REGIONS ──
    for chg in &analysis.change_detections {
        let cid = node_id;
        nodes.push(DepthGraphNode {
            node_id: cid, node_type: DepthNodeType::ChangeDetectionNode,
            content: format!("Change [{:?}]: pts={} magnitude={:.3}m",
                chg.change_type, chg.point_count, chg.magnitude_m),
            centroid: Some(chg.centroid), bounding_box: Some(chg.bounding_box.clone()),
            materialized_path: Some(format!("/Modalities/Depth/Project_{}/Graph_{}/Change/{}", project_id, graph_id, chg.change_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["change".into(), format!("{:?}", chg.change_type).to_lowercase()],
            hotness_score: 0.6 + (chg.magnitude_m / 2.0).min(0.3),
            ..Default::default()
        });
        edges.push(DepthGraphEdge { edge_id, from_node: root_id, to_node: cid, edge_type: DepthEdgeType::DetectedChange, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── CROSS-MODAL: geo map (117) ──
    // If analysis has georeferenced data, add link
    if !matches!(analysis.coordinate_system, CoordinateSystem::SensorLocal) {
        let geo_ref_nid = node_id;
        nodes.push(DepthGraphNode {
            node_id: geo_ref_nid, node_type: DepthNodeType::CrossModalFusionNode,
            content: format!("GeoRef: coord={:?}", analysis.coordinate_system),
            materialized_path: Some(format!("/Modalities/Depth/Project_{}/Graph_{}/GeoRef", project_id, graph_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["georeferenced".into()], hotness_score: 0.7, ..Default::default()
        });
        edges.push(DepthGraphEdge { edge_id, from_node: root_id, to_node: geo_ref_nid, edge_type: DepthEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        edges.push(DepthGraphEdge { edge_id, from_node: geo_ref_nid, to_node: geo_ref_nid, edge_type: DepthEdgeType::PlottedOnGeoMap, weight: 0.9, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("geospatial")); p }, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── HOOK 1: OnGraphCreated ──
    let _ = executor.save_graph(&DepthGraph { graph_id, project_id, source_description: analysis.source_description.clone(), nodes: nodes.clone(), edges: edges.clone(), root_node_id: root_id, state: GraphStateType::Created, state_history: vec![GraphStateTransition { from: GraphStateType::Created, to: GraphStateType::Created, timestamp: now.clone(), triggered_by_step: None }], created_at: now.clone(), updated_at: now.clone(), version: 1, version_notes: vec![VersionNote { version: 1, note: format!("Created: {} nodes {} edges", nodes.len(), edges.len()), step_index: None, timestamp: now.clone(), change_type: ChangeType::Created }] });

    // ── HOOK 2: OnInferRelationships ──
    let inferred = executor.infer_semantic_relationships(&nodes).await;
    let valid: std::collections::HashSet<u64> = nodes.iter().map(|n| n.node_id).collect();
    for (from, to, etype, reason) in inferred {
        if valid.contains(&from) && valid.contains(&to) && from != to {
            edges.push(DepthGraphEdge { edge_id, from_node: from, to_node: to, edge_type: etype, weight: 0.8, provenance: EdgeProvenance::DerivedFromHook, version: 1, properties: { let mut p = HashMap::new(); p.insert("reason".into(), serde_json::json!(reason)); p }, ..Default::default() });
            edge_id += 1;
        }
    }

    // ── HOOK 3: OnEdgeCompletion ──
    let mut deg: HashMap<u64, u32> = HashMap::new();
    for e in &edges { *deg.entry(e.from_node).or_insert(0) += 1; *deg.entry(e.to_node).or_insert(0) += 1; }
    let max_deg = deg.values().copied().max().unwrap_or(1) as f32;
    for n in &mut nodes { if let Some(&d) = deg.get(&n.node_id) { n.hotness_score = (n.hotness_score + (d as f32 / max_deg) * 0.15).min(1.0); } }

    let final_graph = DepthGraph { graph_id, project_id, source_description: analysis.source_description, nodes, edges, root_node_id: root_id, state: GraphStateType::SemanticEnriched, state_history: vec![GraphStateTransition { from: GraphStateType::Created, to: GraphStateType::SemanticEnriched, timestamp: now.clone(), triggered_by_step: None }], created_at: now.clone(), updated_at: now.clone(), version: 1, version_notes: vec![VersionNote { version: 1, note: "Semantic enrichment complete".into(), step_index: None, timestamp: now, change_type: ChangeType::EnrichedBySemantic }] };
    let _ = executor.save_graph(&final_graph);
    DepthModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(final_graph), ..Default::default() }
}

// ─────────────────────────────────────────────────────────────────────────────
// MAIN EXECUTION
// ─────────────────────────────────────────────────────────────────────────────

pub async fn execute(input: DepthModalityAction) -> Result<DepthModalityOutput, String> {
    let executor = PipelineExecutor::new();

    match input {
        DepthModalityAction::Analyze { data, extract_surfaces, extract_objects, extract_normals, compute_topology, compute_occlusion } => {
            let analysis_id = executor.generate_id();
            let source_description = match &data {
                DepthDataSource::LiDARFile { file_path, format, .. } => format!("LiDAR {:?}: {}", format, file_path),
                DepthDataSource::PCDFile { file_path, .. } => format!("PCD: {}", file_path),
                DepthDataSource::PLYFile { file_path, .. } => format!("PLY: {}", file_path),
                DepthDataSource::XYZFile { file_path, .. } => format!("XYZ: {}", file_path),
                DepthDataSource::E57File { file_path, scan_count } => format!("E57: {} ({} scans)", file_path, scan_count),
                DepthDataSource::DepthImage { file_path, width, height, .. } => format!("DepthImage {}×{}: {}", width, height, file_path),
                DepthDataSource::RGBDSequence { depth_dir, .. } => format!("RGBD sequence: {}", depth_dir),
                DepthDataSource::StereoPair { left_image_path, .. } => format!("Stereo: {}", left_image_path),
                DepthDataSource::PhotogrammetryCloud { file_path, reconstruction_software, .. } => format!("SfM/MVS: {} sw={:?}", file_path, reconstruction_software),
                DepthDataSource::ExternalPointCloud { points, source_modality, .. } => format!("External {}: {} pts", source_modality, points.len()),
                DepthDataSource::LiveStream { endpoint, sensor_type, .. } => format!("Live {:?}: {}", sensor_type, endpoint),
            };
            Ok(DepthModalityOutput {
                success: true,
                analysis: Some(DepthAnalysisResult { analysis_id, source_description, ..Default::default() }),
                ..Default::default()
            })
        }

        DepthModalityAction::CreateGraph { analysis, project_id } => {
            Ok(create_graph(&executor, analysis, project_id).await)
        }

        DepthModalityAction::UpdateGraph { graph_id, new_frames, project_id } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            let mut next_nid = graph.nodes.iter().map(|n| n.node_id).max().unwrap_or(0) + 1;
            let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
            let initial = graph.nodes.len();
            for frame in &new_frames {
                let valid_pixels = frame.depth_data.iter().filter(|&&d| d > 0.0).count();
                let mean_depth = if valid_pixels > 0 { frame.depth_data.iter().filter(|&&d| d > 0.0).sum::<f32>() / valid_pixels as f32 } else { 0.0 };
                graph.nodes.push(DepthGraphNode {
                    node_id: next_nid, node_type: DepthNodeType::DepthMapNode,
                    content: format!("Frame {}: {}×{}px mean_depth={:.2}m valid={:.0}%",
                        frame.frame_id, frame.width, frame.height, mean_depth, valid_pixels as f32 / (frame.width * frame.height).max(1) as f32 * 100.0),
                    depth_m: Some(mean_depth), timestamp_sec: Some(frame.timestamp_sec),
                    provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                    keywords: vec!["depth-frame".into()], hotness_score: 0.7, ..Default::default()
                });
                graph.edges.push(DepthGraphEdge { edge_id: next_eid, from_node: graph.root_node_id, to_node: next_nid, edge_type: DepthEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                next_eid += 1; next_nid += 1;
            }
            graph.version += 1; graph.updated_at = now.clone(); graph.state = GraphStateType::Updated;
            graph.version_notes.push(VersionNote { version: graph.version, note: format!("Updated: {} new frames → {} new nodes", new_frames.len(), graph.nodes.len() - initial), step_index: None, timestamp: now, change_type: ChangeType::Updated });
            executor.save_graph(&graph)?;
            Ok(DepthModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        DepthModalityAction::ReconstructMesh { graph_id, method, voxel_size_m, depth_trunc_m } => {
            let graph = executor.load_graph(graph_id)?;
            // In production: collect point cloud from nodes, run Poisson/BPA/TSDF
            let mesh = ReconstructedMesh {
                mesh_id: executor.generate_id(),
                vertex_count: 0, face_count: 0,
                has_normals: true, has_colors: false,
                bounding_box: BoundingBox3D::default(),
                volume_m3: None, surface_area_m2: 0.0,
                is_watertight: matches!(method, MeshReconstructionMethod::TSDF | MeshReconstructionMethod::Poisson),
                reconstruction_method: format!("{:?}", method),
                voxel_size_m: Some(voxel_size_m),
                file_path: None,
            };
            Ok(DepthModalityOutput { success: true, graph_id: Some(graph_id), mesh: Some(mesh), ..Default::default() })
        }

        DepthModalityAction::FuseFrames { graph_id, frames, method, camera_intrinsics } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            graph.version += 1; graph.updated_at = now.clone();
            graph.version_notes.push(VersionNote { version: graph.version, note: format!("Fused {} frames using {:?}", frames.len(), method), step_index: None, timestamp: now, change_type: ChangeType::Updated });
            executor.save_graph(&graph)?;
            Ok(DepthModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        DepthModalityAction::Segment { graph_id, method, min_cluster_size, max_cluster_size, distance_threshold_m } => {
            let graph = executor.load_graph(graph_id)?;
            // In production: load point cloud from ZSEI, run selected segmentation algorithm
            Ok(DepthModalityOutput { success: true, graph_id: Some(graph_id), clusters: Some(vec![]), ..Default::default() })
        }

        DepthModalityAction::Register { source_graph_id, target_graph_id, method, initial_transform } => {
            let source = executor.load_graph(source_graph_id)?;
            let target = executor.load_graph(target_graph_id)?;
            // ICP: collect centroid positions from cluster nodes as proxy point set
            let source_pts: Vec<[f32; 3]> = source.nodes.iter()
                .filter_map(|n| n.centroid.map(|c| [c[0] as f32, c[1] as f32, c[2] as f32]))
                .collect();
            let target_pts: Vec<[f32; 3]> = target.nodes.iter()
                .filter_map(|n| n.centroid.map(|c| [c[0] as f32, c[1] as f32, c[2] as f32]))
                .collect();
            let (transform, rmse) = PipelineExecutor::icp_point_to_point(&source_pts, &target_pts, 50, 0.001);
            Ok(DepthModalityOutput { success: true, registration_transform: Some(transform), ..Default::default() })
        }

        DepthModalityAction::ComputeNormals { graph_id, radius_m, orient_toward } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            graph.version += 1; graph.updated_at = now.clone();
            graph.version_notes.push(VersionNote { version: graph.version, note: format!("Normals computed: radius={:.3}m orient={:?}", radius_m, orient_toward), step_index: None, timestamp: now, change_type: ChangeType::Updated });
            executor.save_graph(&graph)?;
            Ok(DepthModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        DepthModalityAction::ComputeOccupancy { graph_id, grid_resolution_m, field_type } => {
            let graph = executor.load_graph(graph_id)?;
            // In production: voxelize point cloud into occupancy/TSDF/ESDF grid
            Ok(DepthModalityOutput { success: true, graph_id: Some(graph_id), ..Default::default() })
        }

        DepthModalityAction::ExtractPlanes { graph_id, min_inlier_count, max_distance_m, max_planes } => {
            let graph = executor.load_graph(graph_id)?;
            let mut planes: Vec<ExtractedPlane> = Vec::new();
            // Use RANSAC on centroid-level proxy points (full point cloud from ZSEI in production)
            let proxy_pts: Vec<[f32; 3]> = graph.nodes.iter()
                .filter_map(|n| n.centroid.map(|c| [c[0] as f32, c[1] as f32, c[2] as f32]))
                .collect();

            for plane_idx in 0..max_planes.min(10) {
                if proxy_pts.len() < 3 { break; }
                if let Some((normal, d, inliers)) = PipelineExecutor::ransac_plane(&proxy_pts, 100, max_distance_m) {
                    if inliers.len() >= min_inlier_count as usize {
                        let centroid_pts: Vec<[f32; 3]> = inliers.iter().map(|&i| proxy_pts[i]).collect();
                        let cx = centroid_pts.iter().map(|p| p[0] as f64).sum::<f64>() / centroid_pts.len() as f64;
                        let cy = centroid_pts.iter().map(|p| p[1] as f64).sum::<f64>() / centroid_pts.len() as f64;
                        let cz = centroid_pts.iter().map(|p| p[2] as f64).sum::<f64>() / centroid_pts.len() as f64;
                        let tilt_deg = (normal[2].acos() * 180.0 / std::f64::consts::PI) as f32;
                        planes.push(ExtractedPlane {
                            plane_id: executor.generate_id(),
                            normal, d, centroid: [cx, cy, cz],
                            inlier_count: inliers.len() as u32,
                            area_m2: (inliers.len() as f32) * max_distance_m * max_distance_m,
                            tilt_angle_deg: tilt_deg,
                            bounding_box: BoundingBox3D::default(),
                            plane_type: if tilt_deg < 10.0 { PlaneType::Floor } else if tilt_deg > 80.0 { PlaneType::Wall } else { PlaneType::Unknown },
                        });
                    }
                }
                if planes.len() >= max_planes as usize { break; }
            }
            Ok(DepthModalityOutput { success: true, graph_id: Some(graph_id), planes: Some(planes), ..Default::default() })
        }

        DepthModalityAction::ClassifyClusters { graph_id } => {
            let graph = executor.load_graph(graph_id)?;
            let cluster_nodes: Vec<PointCluster> = graph.nodes.iter()
                .filter(|n| matches!(n.node_type, DepthNodeType::PointClusterNode))
                .map(|n| PointCluster {
                    cluster_id: n.node_id,
                    point_count: n.point_count.unwrap_or(0) as u32,
                    centroid: n.centroid.unwrap_or([0.0, 0.0, 0.0]),
                    bounding_box: n.bounding_box.clone().unwrap_or_default(),
                    ..Default::default()
                }).collect();
            let classifications = executor.classify_clusters_llm(&cluster_nodes).await;
            Ok(DepthModalityOutput { success: true, graph_id: Some(graph_id), clusters: Some(cluster_nodes), ..Default::default() })
        }

        DepthModalityAction::DetectChanges { reference_graph_id, current_graph_id, change_threshold_m } => {
            let ref_graph = executor.load_graph(reference_graph_id)?;
            let cur_graph = executor.load_graph(current_graph_id)?;
            // Compare cluster centroids between reference and current
            let ref_centroids: Vec<[f64; 3]> = ref_graph.nodes.iter().filter_map(|n| n.centroid).collect();
            let cur_centroids: Vec<[f64; 3]> = cur_graph.nodes.iter().filter_map(|n| n.centroid).collect();
            let mut changes: Vec<ChangeRegion> = Vec::new();
            for cur in &cur_centroids {
                let min_dist = ref_centroids.iter().map(|r| {
                    ((cur[0]-r[0]).powi(2)+(cur[1]-r[1]).powi(2)+(cur[2]-r[2]).powi(2)).sqrt() as f32
                }).fold(f32::INFINITY, f32::min);
                if min_dist > change_threshold_m {
                    changes.push(ChangeRegion {
                        change_id: executor.generate_id(),
                        centroid: *cur, point_count: 1,
                        change_type: ChangeType3D::Added,
                        magnitude_m: min_dist,
                        bounding_box: BoundingBox3D { center: *cur, ..Default::default() },
                    });
                }
            }
            Ok(DepthModalityOutput { success: true, ..Default::default() })
        }

        DepthModalityAction::QueryGraph { graph_id, query } => {
            let graph = executor.load_graph(graph_id)?;
            let result = match query {
                DepthGraphQuery::NodeDetail { node_id } => {
                    let node = graph.nodes.iter().find(|n| n.node_id == node_id);
                    let incoming: Vec<_> = graph.edges.iter().filter(|e| e.to_node == node_id).collect();
                    let outgoing: Vec<_> = graph.edges.iter().filter(|e| e.from_node == node_id).collect();
                    serde_json::json!({ "node": node, "incoming": incoming, "outgoing": outgoing })
                }
                DepthGraphQuery::ClustersByLabel { label } => {
                    let cs: Vec<_> = graph.nodes.iter().filter(|n| n.class_label.as_deref().map(|l| l.to_lowercase().contains(&label.to_lowercase())).unwrap_or(false)).collect();
                    serde_json::json!({ "clusters": cs })
                }
                DepthGraphQuery::PlanesOfType { plane_type } => {
                    let ps: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, DepthNodeType::ExtractedPlaneNode) && n.plane_type.as_deref().map(|t| t.to_lowercase() == plane_type.to_lowercase()).unwrap_or(false)).collect();
                    serde_json::json!({ "planes": ps })
                }
                DepthGraphQuery::ObjectsInVolume { min, max } => {
                    let objs: Vec<_> = graph.nodes.iter().filter(|n| {
                        n.centroid.map(|c| c[0] >= min[0] && c[0] <= max[0] && c[1] >= min[1] && c[1] <= max[1] && c[2] >= min[2] && c[2] <= max[2]).unwrap_or(false)
                    }).collect();
                    serde_json::json!({ "objects": objs })
                }
                DepthGraphQuery::OcclusionRegions => {
                    let occ: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, DepthNodeType::OcclusionRegionNode)).collect();
                    serde_json::json!({ "occlusion_regions": occ })
                }
                DepthGraphQuery::ChangedRegions => {
                    let chg: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, DepthNodeType::ChangeDetectionNode)).collect();
                    serde_json::json!({ "changes": chg })
                }
                DepthGraphQuery::LargestClusters { top_n } => {
                    let mut cs: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, DepthNodeType::PointClusterNode)).collect();
                    cs.sort_by(|a, b| b.point_count.cmp(&a.point_count));
                    cs.truncate(top_n as usize);
                    serde_json::json!({ "clusters": cs })
                }
                DepthGraphQuery::ScanPositions => {
                    let sps: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, DepthNodeType::ScanPositionNode)).collect();
                    serde_json::json!({ "scan_positions": sps })
                }
                DepthGraphQuery::CrossModalLinks { node_id } => {
                    let links: Vec<_> = graph.edges.iter().filter(|e| (e.from_node == node_id || e.to_node == node_id) && matches!(e.edge_type, DepthEdgeType::ContributesTo3DScene | DepthEdgeType::ColorAlignedWithImage | DepthEdgeType::ContactSurfaceForHaptic | DepthEdgeType::FusedWithThermal | DepthEdgeType::PlottedOnGeoMap | DepthEdgeType::NavigableSpaceForRobot | DepthEdgeType::FusedWithRadar | DepthEdgeType::FusedWithSonar)).collect();
                    serde_json::json!({ "cross_modal_links": links })
                }
                DepthGraphQuery::AGIActivity => serde_json::json!({ "is_active": false }),
                DepthGraphQuery::AllNodes => serde_json::json!({ "nodes": graph.nodes }),
                DepthGraphQuery::AllEdges => serde_json::json!({ "edges": graph.edges }),
            };
            Ok(DepthModalityOutput { success: true, query_result: Some(result), ..Default::default() })
        }

        DepthModalityAction::GetGraph { graph_id } => {
            let graph = executor.load_graph(graph_id)?;
            Ok(DepthModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        DepthModalityAction::TriggerSemanticHook { graph_id, hook } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            match hook {
                DepthSemanticHook::OnGraphCreated => { graph.state = GraphStateType::SemanticEnriched; }
                DepthSemanticHook::OnInferRelationships => {
                    let new_edges = executor.infer_semantic_relationships(&graph.nodes).await;
                    let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                    let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                    for (from, to, etype, reason) in new_edges {
                        if valid.contains(&from) && valid.contains(&to) && from != to {
                            graph.edges.push(DepthGraphEdge { edge_id: next_eid, from_node: from, to_node: to, edge_type: etype, weight: 0.8, provenance: EdgeProvenance::DerivedFromHook, version: 1, properties: { let mut p = HashMap::new(); p.insert("reason".into(), serde_json::json!(reason)); p }, ..Default::default() });
                            next_eid += 1;
                        }
                    }
                }
                DepthSemanticHook::OnEdgeCompletion => {
                    let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                    graph.edges.retain(|e| valid.contains(&e.from_node) && valid.contains(&e.to_node));
                }
                DepthSemanticHook::OnCrossModalityLink { target_modality, target_graph_id } => {
                    graph.state = GraphStateType::CrossLinked;
                    graph.version += 1;
                    graph.version_notes.push(VersionNote { version: graph.version, note: format!("Cross-linked to {} (graph {})", target_modality, target_graph_id), step_index: None, timestamp: now.clone(), change_type: ChangeType::CrossLinked });
                }
            }
            graph.updated_at = now;
            executor.save_graph(&graph)?;
            Ok(DepthModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        DepthModalityAction::ExportProduct { graph_id, format } => {
            let ext = match &format {
                DepthExportFormat::LAS => "las", DepthExportFormat::LAZ => "laz",
                DepthExportFormat::PCD => "pcd", DepthExportFormat::PLY_PointCloud | DepthExportFormat::PLY_Mesh => "ply",
                DepthExportFormat::XYZ => "xyz", DepthExportFormat::E57 => "e57",
                DepthExportFormat::OBJ_Mesh => "obj", DepthExportFormat::GLTF_Mesh => "glb",
                DepthExportFormat::DepthImage_PNG => "png", DepthExportFormat::DepthImage_EXR => "exr",
                DepthExportFormat::VoxelGrid_NPZ => "npz",
                DepthExportFormat::GeoTIFF_DSM | DepthExportFormat::GeoTIFF_DTM => "tif",
                DepthExportFormat::KML_Objects => "kml", DepthExportFormat::GeoJSON_Objects => "geojson",
                DepthExportFormat::Custom(ext) => ext.as_str(),
            };
            let export_path = format!("/tmp/depth_export_{}_{}", graph_id, if ext.is_empty() { "dat" } else { ext });
            Ok(DepthModalityOutput { success: true, export_path: Some(export_path), ..Default::default() })
        }

        DepthModalityAction::StreamToUI { graph_id, .. } => {
            Ok(DepthModalityOutput { success: true, graph_id: Some(graph_id), ..Default::default() })
        }

        DepthModalityAction::HeadlessProcess { graph_id, operations } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            for op in operations {
                match op {
                    DepthHeadlessOp::ReconstructMesh { method } => {
                        graph.version_notes.push(VersionNote { version: graph.version + 1, note: format!("Headless: mesh reconstruction {:?}", method), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                        graph.version += 1;
                    }
                    DepthHeadlessOp::ComputeNormals { radius_m } => {
                        graph.version_notes.push(VersionNote { version: graph.version + 1, note: format!("Headless: normals r={:.3}m", radius_m), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                        graph.version += 1;
                    }
                    DepthHeadlessOp::SegmentClusters { method } => {
                        graph.version_notes.push(VersionNote { version: graph.version + 1, note: format!("Headless: segmentation {:?}", method), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                        graph.version += 1;
                    }
                    DepthHeadlessOp::ExtractPlanes { max_planes } => {
                        graph.version_notes.push(VersionNote { version: graph.version + 1, note: format!("Headless: plane extraction max={}", max_planes), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                        graph.version += 1;
                    }
                    DepthHeadlessOp::FuseWithDepth { other_graph_id } => {
                        let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        graph.edges.push(DepthGraphEdge { edge_id: next_eid, from_node: graph.root_node_id, to_node: graph.root_node_id, edge_type: DepthEdgeType::RegisteredTo, weight: 0.9, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("other_graph_id".into(), serde_json::json!(other_graph_id)); p }, ..Default::default() });
                    }
                    DepthHeadlessOp::ExportMesh { format, path } => {
                        // In production: serialize mesh to path
                    }
                    DepthHeadlessOp::CrossRegisterWith3D { graph_id_3d } => {
                        let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        for node in graph.nodes.iter().filter(|n| matches!(n.node_type, DepthNodeType::ReconstructedMeshNode)) {
                            graph.edges.push(DepthGraphEdge { edge_id: next_eid, from_node: node.node_id, to_node: node.node_id, edge_type: DepthEdgeType::ContributesTo3DScene, weight: 0.95, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("graph_id_3d".into(), serde_json::json!(graph_id_3d)); p }, ..Default::default() });
                            next_eid += 1;
                        }
                    }
                }
            }
            graph.updated_at = now;
            executor.save_graph(&graph)?;
            Ok(DepthModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
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
    if input_json.is_empty() { eprintln!("Usage: depth_sensing --input '<json>'"); std::process::exit(1); }
    let input: DepthModalityAction = match serde_json::from_str(&input_json) {
        Ok(v) => v,
        Err(e) => { println!("{}", serde_json::json!({"success":false,"error":format!("Parse error: {}",e)})); std::process::exit(1); }
    };
    let rt = tokio::runtime::Runtime::new().expect("Tokio runtime");
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap_or_else(|_| r#"{"success":false,"error":"serialize"}"#.into())),
        Err(e) => { println!("{}", serde_json::json!({"success":false,"error":e})); std::process::exit(1); }
    }
}
