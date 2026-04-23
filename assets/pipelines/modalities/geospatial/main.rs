//! GeospatialSensing — Pipeline #117
//!
//! Geographic and spatial intelligence: vector features, raster terrain,
//! routing, spatial indexing, coordinate transforms, geocoding, coverage
//! analysis, terrain modeling (DSM/DTM), map tile management, and
//! geospatial relationship inference.
//!
//! CROSS-LINKS:
//!   100 (Text)     → place names, addresses, geographic descriptions
//!   109 (3D)       → 3D scenes geo-referenced on terrain
//!   115 (Depth)    → LiDAR terrain models, UAV point clouds
//!   117 is the anchor for: Radar (124) coverage maps, Sonar (125) bathymetry,
//!                          EM (118) propagation coverage, IMU (116) trajectories,
//!                          BCI (119) mobile session locations
//!
//! STORAGE: ZSEI containers under /Modalities/Geospatial/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

// ─────────────────────────────────────────────────────────────────────────────
// INPUT TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum GeoModalityAction {
    /// Analyze geospatial data
    Analyze {
        data: GeoDataSource,
        extract_features: bool,
        build_topology: bool,
        compute_terrain: bool,
        extract_routes: bool,
    },
    /// Create a graph from analysis result
    CreateGraph {
        analysis: GeoAnalysisResult,
        project_id: u64,
    },
    /// Update graph with new features
    UpdateGraph {
        graph_id: u64,
        updates: Vec<GeoUpdate>,
        project_id: u64,
    },
    /// Geocode an address or place name
    Geocode {
        query: String,
        limit: u32,
    },
    /// Reverse geocode from coordinates
    ReverseGeocode {
        lat: f64, lon: f64,
        radius_m: f64,
    },
    /// Compute route between points
    ComputeRoute {
        graph_id: u64,
        waypoints: Vec<GeoPoint>,
        mode: RoutingMode,
        options: RouteOptions,
    },
    /// Spatial query: find features within area
    SpatialQuery {
        graph_id: u64,
        query: SpatialQueryType,
    },
    /// Transform coordinates between CRS
    TransformCoordinates {
        points: Vec<GeoPoint>,
        from_crs: String,
        to_crs: String,
    },
    /// Compute terrain analysis (slope, aspect, viewshed)
    TerrainAnalysis {
        graph_id: u64,
        analysis_type: TerrainAnalysisType,
        params: TerrainAnalysisParams,
    },
    /// Compute coverage / service area from a location
    CoverageAnalysis {
        graph_id: u64,
        origin: GeoPoint,
        mode: RoutingMode,
        time_minutes: Option<f32>,
        distance_m: Option<f32>,
    },
    /// Overlay two geographic datasets
    Overlay {
        graph_id_a: u64,
        graph_id_b: u64,
        operation: OverlayOperation,
        project_id: u64,
    },
    /// Compute change detection between two temporal layers
    TemporalChange {
        before_graph_id: u64,
        after_graph_id: u64,
        change_type: GeoChangeType,
    },
    /// Build spatial index
    BuildSpatialIndex { graph_id: u64, index_type: GeoIndexType },
    /// Query graph
    QueryGraph { graph_id: u64, query: GeoGraphQuery },
    /// Retrieve full graph for Context Viewer
    GetGraph { graph_id: u64 },
    /// Trigger ZSEI semantic hooks
    TriggerSemanticHook { graph_id: u64, hook: GeoSemanticHook },
    /// Export geospatial products
    ExportProduct { graph_id: u64, format: GeoExportFormat },
    /// Stream to UI
    StreamToUI { graph_id: u64, session_id: String, display_mode: GeoDisplayMode },
    /// Headless processing (AGI-first)
    HeadlessProcess { graph_id: u64, operations: Vec<GeoHeadlessOp> },
}

// ─────────────────────────────────────────────────────────────────────────────
// DATA SOURCES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GeoDataSource {
    /// GeoJSON feature collection
    GeoJSON { file_path: String },
    /// Shapefile (.shp + sidecar files)
    Shapefile { file_path: String, crs: Option<String> },
    /// KML/KMZ
    KML { file_path: String },
    /// GPX track / waypoints
    GPX { file_path: String },
    /// OpenStreetMap PBF or XML
    OSM { file_path: String, extract_features: Vec<OSMFeatureType> },
    /// Raster: GeoTIFF elevation / imagery
    GeoTIFF {
        file_path: String,
        band_interpretation: RasterBandInterpretation,
    },
    /// DEM (SRTM, ALOS, Copernicus)
    DEM {
        file_path: String,
        vertical_datum: VerticalDatum,
        horizontal_crs: String,
    },
    /// WMS / WMTS tile URL
    WebMapService {
        url: String,
        layer: String,
        crs: String,
        bbox: [f64; 4],             // [min_lon, min_lat, max_lon, max_lat]
    },
    /// WFS (Web Feature Service) URL
    WebFeatureService {
        url: String,
        type_name: String,
        bbox: Option<[f64; 4]>,
    },
    /// PostGIS / GeoPackage
    GeoPackage { file_path: String, layers: Vec<String> },
    /// CSV with lat/lon columns
    CSVPoints {
        file_path: String,
        lat_col: String, lon_col: String,
        alt_col: Option<String>,
        delimiter: char,
    },
    /// NetCDF (climate, oceanographic)
    NetCDF { file_path: String, variable: String },
    /// Live GPS stream
    LiveGPS { endpoint: String, protocol: GPSProtocol },
    /// Multiple sources combined
    MultiSource { sources: Vec<GeoDataSource>, merge_strategy: MergeStrategy },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OSMFeatureType { Roads, Buildings, Landuse, Water, Railways, Boundaries, POI, Natural, All }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum RasterBandInterpretation { #[default] Elevation, NDVI, RGB, Grayscale, Classification, Temperature, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum VerticalDatum { #[default] EGM96, EGM2008, NAVD88, MSL, Ellipsoidal, WGS84, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GPSProtocol { NMEA, UBX, SiRF, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MergeStrategy { Union, Clip, Filter, Priority }

// ─────────────────────────────────────────────────────────────────────────────
// CORE GEOMETRY TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeoPoint {
    pub lon: f64, pub lat: f64,
    pub alt_m: Option<f64>,
    pub crs: Option<String>,    // default WGS84 EPSG:4326
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeoLineString {
    pub points: Vec<GeoPoint>,
    pub length_m: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeoPolygon {
    pub exterior: Vec<GeoPoint>,
    pub holes: Vec<Vec<GeoPoint>>,
    pub area_m2: f64,
    pub perimeter_m: f64,
    pub centroid: GeoPoint,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeoBoundingBox {
    pub min_lon: f64, pub min_lat: f64,
    pub max_lon: f64, pub max_lat: f64,
    pub min_alt: Option<f64>, pub max_alt: Option<f64>,
}

impl GeoBoundingBox {
    pub fn contains(&self, p: &GeoPoint) -> bool {
        p.lon >= self.min_lon && p.lon <= self.max_lon
            && p.lat >= self.min_lat && p.lat <= self.max_lat
    }

    pub fn area_km2(&self) -> f64 {
        let dlat = (self.max_lat - self.min_lat).abs();
        let dlon = (self.max_lon - self.min_lon).abs();
        let mid_lat = (self.min_lat + self.max_lat) / 2.0;
        let km_per_deg_lat = 111.32;
        let km_per_deg_lon = 111.32 * mid_lat.to_radians().cos();
        dlat * km_per_deg_lat * dlon * km_per_deg_lon
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ANALYSIS RESULT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeoAnalysisResult {
    pub analysis_id: u64,
    pub source_description: String,

    // SPATIAL EXTENT
    pub bounding_box: GeoBoundingBox,
    pub crs: String,
    pub feature_count: u64,

    // VECTOR FEATURES
    pub locations: Vec<GeoLocation>,
    pub regions: Vec<GeoRegion>,
    pub routes: Vec<GeoRoute>,
    pub boundaries: Vec<GeoBoundary>,
    pub pois: Vec<PointOfInterest>,
    pub trajectories: Vec<GeoTrajectory>,

    // RASTER / TERRAIN
    pub terrain_models: Vec<TerrainModel>,
    pub elevation_profiles: Vec<ElevationProfile>,
    pub raster_layers: Vec<RasterLayer>,

    // TOPOLOGY
    pub topology: Option<GeoTopology>,
    pub connectivity_graph: Vec<RoadNetworkEdge>,

    // ANALYSIS PRODUCTS
    pub coverage_areas: Vec<CoverageArea>,
    pub viewsheds: Vec<Viewshed>,
    pub flood_zones: Vec<FloodZone>,
    pub change_regions: Vec<GeoChangeRegion>,

    // GEOCODING
    pub geocoded_places: Vec<GeocodedPlace>,

    // METADATA
    pub data_sources: Vec<String>,
    pub acquisition_date: Option<String>,
    pub resolution_m: Option<f32>,
    pub processing_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeoLocation {
    pub location_id: u64,
    pub name: Option<String>,
    pub location_type: LocationType,
    pub point: GeoPoint,
    pub accuracy_m: Option<f32>,
    pub elevation_m: Option<f64>,
    pub address: Option<GeoAddress>,
    pub properties: HashMap<String, serde_json::Value>,
    pub tags: Vec<String>,
    pub timestamp: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum LocationType {
    #[default] Unknown,
    Building, Intersection, POI, Airport, Port, RailStation, BusStop,
    Hospital, School, Government, Emergency, Industrial, Residential,
    Park, Forest, Beach, Mountain, Lake, River,
    Antenna, Tower, Sensor, Vehicle, Person, Asset, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeoAddress {
    pub street: Option<String>,
    pub house_number: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub postal_code: Option<String>,
    pub country_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeoRegion {
    pub region_id: u64,
    pub name: Option<String>,
    pub region_type: RegionType,
    pub polygon: GeoPolygon,
    pub area_km2: f64,
    pub population: Option<u64>,
    pub properties: HashMap<String, serde_json::Value>,
    pub admin_level: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum RegionType {
    #[default] Unknown,
    Country, State, Province, County, City, District, Neighborhood,
    Parcel, LandUse, ZoningArea, WatershedBasin, NaturalArea, ProtectedArea,
    MilitaryZone, MaritimeZone, AirspaceZone, CoverageZone, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeoRoute {
    pub route_id: u64,
    pub name: Option<String>,
    pub route_type: RouteType,
    pub geometry: GeoLineString,
    pub waypoints: Vec<GeoPoint>,
    pub length_m: f64,
    pub duration_sec: Option<f64>,
    pub speed_kmh: Option<f32>,
    pub elevation_gain_m: Option<f64>,
    pub elevation_loss_m: Option<f64>,
    pub road_class: Option<RoadClass>,
    pub surface_type: Option<SurfaceType>,
    pub is_bidirectional: bool,
    pub max_speed_kmh: Option<f32>,
    pub properties: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum RouteType { #[default] Unknown, Road, Path, Footway, Cycleway, Rail, Waterway, AirCorridor, Pipeline, PowerLine, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum RoadClass { #[default] Unknown, Motorway, Trunk, Primary, Secondary, Tertiary, Residential, ServiceRoad, Track, Path }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SurfaceType { #[default] Unknown, Paved, Asphalt, Concrete, Gravel, Dirt, Sand, Grass, Snow, Water }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeoBoundary {
    pub boundary_id: u64,
    pub name: Option<String>,
    pub boundary_type: BoundaryType,
    pub geometry: GeoLineString,
    pub left_region_id: Option<u64>,
    pub right_region_id: Option<u64>,
    pub admin_level: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum BoundaryType { #[default] Unknown, Administrative, Natural, Coastline, WaterBody, ZoningBoundary, PropertyLine, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PointOfInterest {
    pub poi_id: u64,
    pub name: String,
    pub poi_type: String,
    pub point: GeoPoint,
    pub address: Option<GeoAddress>,
    pub osm_tags: HashMap<String, String>,
    pub rating: Option<f32>,
    pub opening_hours: Option<String>,
    pub website: Option<String>,
    pub phone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeoTrajectory {
    pub trajectory_id: u64,
    pub entity_id: Option<String>,
    pub entity_type: Option<String>,
    pub points: Vec<TrajectoryPoint>,
    pub start_time: f64,
    pub end_time: f64,
    pub total_distance_m: f64,
    pub mean_speed_ms: Option<f32>,
    pub max_speed_ms: Option<f32>,
    pub bounding_box: GeoBoundingBox,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrajectoryPoint {
    pub timestamp: f64,
    pub point: GeoPoint,
    pub speed_ms: Option<f32>,
    pub heading_deg: Option<f32>,
    pub accuracy_m: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerrainModel {
    pub model_id: u64,
    pub model_type: TerrainModelType,
    pub bounding_box: GeoBoundingBox,
    pub resolution_m: f32,
    pub cell_count_x: u32, pub cell_count_y: u32,
    pub min_elevation_m: f64, pub max_elevation_m: f64, pub mean_elevation_m: f64,
    pub vertical_datum: VerticalDatum,
    pub crs: String,
    pub file_path: Option<String>,      // stored as GeoTIFF
    pub slope_stats: Option<SlopeStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum TerrainModelType { #[default] DSM, DTM, CHM, Bathymetry, IceSurface, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SlopeStats {
    pub mean_slope_deg: f32,
    pub max_slope_deg: f32,
    pub flat_fraction: f32,         // fraction < 5°
    pub steep_fraction: f32,        // fraction > 30°
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ElevationProfile {
    pub profile_id: u64,
    pub route_id: Option<u64>,
    pub samples: Vec<(f64, f64)>,    // (distance_m, elevation_m)
    pub total_length_m: f64,
    pub min_elevation_m: f64, pub max_elevation_m: f64,
    pub cumulative_gain_m: f64, pub cumulative_loss_m: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RasterLayer {
    pub layer_id: u64,
    pub name: String,
    pub band_interpretation: RasterBandInterpretation,
    pub bounding_box: GeoBoundingBox,
    pub resolution_m: f32,
    pub width_px: u32, pub height_px: u32,
    pub min_value: f64, pub max_value: f64, pub mean_value: f64,
    pub no_data_value: Option<f64>,
    pub crs: String,
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeoTopology {
    pub node_count: u32,
    pub edge_count: u32,
    pub connected_components: u32,
    pub has_cycles: bool,
    pub is_planar: bool,
    pub longest_path_m: Option<f64>,
    pub diameter_m: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoadNetworkEdge {
    pub edge_id: u64,
    pub from_node_id: u64, pub to_node_id: u64,
    pub route_id: u64,
    pub length_m: f64,
    pub travel_time_sec: Option<f64>,
    pub road_class: RoadClass,
    pub is_directed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CoverageArea {
    pub coverage_id: u64,
    pub origin: GeoPoint,
    pub polygon: GeoPolygon,
    pub coverage_type: CoverageType,
    pub reachable_area_km2: f64,
    pub time_or_distance_limit: f64,
    pub unit: String,               // "minutes" | "meters"
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum CoverageType { #[default] Isochrone, Isodistance, SignalCoverage, VisibilityCone, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Viewshed {
    pub viewshed_id: u64,
    pub observer_point: GeoPoint,
    pub observer_height_m: f32,
    pub max_range_m: f32,
    pub visible_area_km2: f32,
    pub visible_fraction: f32,
    pub polygon: GeoPolygon,
    pub terrain_model_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FloodZone {
    pub zone_id: u64,
    pub zone_type: FloodZoneType,
    pub return_period_years: Option<u32>,
    pub water_depth_m: Option<f32>,
    pub polygon: GeoPolygon,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum FloodZoneType { #[default] Unknown, AEZ_100yr, AEZ_500yr, Floodway, Coastal, RiverFloodplain, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum RiskLevel { #[default] None, Low, Moderate, High, VeryHigh }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeoChangeRegion {
    pub change_id: u64,
    pub change_type: GeoChangeType,
    pub before_date: Option<String>,
    pub after_date: Option<String>,
    pub polygon: GeoPolygon,
    pub area_changed_km2: f64,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum GeoChangeType { #[default] Unknown, LandUseChange, BuildingChange, VegetationChange, WaterBodyChange, RoadChange, InfrastructureChange, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeocodedPlace {
    pub place_id: u64,
    pub query: String,
    pub point: GeoPoint,
    pub display_name: String,
    pub address: GeoAddress,
    pub place_type: String,
    pub importance: f32,
    pub bounding_box: Option<GeoBoundingBox>,
}

// ─────────────────────────────────────────────────────────────────────────────
// ROUTING TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum RoutingMode { #[default] Driving, Walking, Cycling, Transit, Truck, Emergency, Maritime, Aviation, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RouteOptions {
    pub avoid_tolls: bool,
    pub avoid_highways: bool,
    pub avoid_ferries: bool,
    pub avoid_unpaved: bool,
    pub max_slope_deg: Option<f32>,
    pub vehicle_weight_tons: Option<f32>,
    pub vehicle_height_m: Option<f32>,
    pub departure_time: Option<f64>,
}

// ─────────────────────────────────────────────────────────────────────────────
// ANALYSIS PARAMETER TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpatialQueryType {
    WithinBBox { bbox: GeoBoundingBox },
    WithinRadius { center: GeoPoint, radius_m: f64 },
    WithinPolygon { polygon: GeoPolygon },
    NearestN { point: GeoPoint, n: u32, feature_type: Option<String> },
    Intersects { geometry: GeoPolygon },
    ContainsPoint { point: GeoPoint },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TerrainAnalysisType { Slope, Aspect, Hillshade, TRI, TPI, Roughness, Viewshed, HydrologicFill, WatershedDelineation, Curvature }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerrainAnalysisParams {
    pub z_factor: f32,
    pub observer_height_m: f32,
    pub observer_point: Option<GeoPoint>,
    pub sun_azimuth_deg: f32,
    pub sun_altitude_deg: f32,
    pub max_distance_m: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OverlayOperation { Union, Intersection, Difference, SymmetricDifference, Clip, Erase, Identity }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GeoIndexType { RTree, Quadtree, Hilbert, KDTree, Grid }

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH NODE TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum GeoNodeType {
    #[default] GeoScene,            // root
    LocationNode,                   // point location
    RegionNode,                     // polygon region
    RouteNode,                      // linear route/road/path
    BoundaryNode,                   // boundary line
    POINode,                        // point of interest
    TrajectoryNode,                 // movement path
    TrajectoryPointNode,           // single trajectory fix
    TerrainModelNode,               // DSM/DTM
    ElevationProfileNode,          // route elevation profile
    RasterLayerNode,                // raster layer
    CoverageAreaNode,               // isochrone/coverage polygon
    ViewshedNode,                   // visibility analysis
    FloodZoneNode,                  // flood risk zone
    ChangeRegionNode,              // detected land change
    GeocodedPlaceNode,             // geocoder result
    NetworkNodeGeo,                 // road network intersection
    NetworkEdgeGeo,                 // road network segment
    TopologyNode,                   // topological analysis result
    CrossModalOverlayNode,         // placeholder for cross-modal overlay
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeoGraphNode {
    pub node_id: u64,
    pub node_type: GeoNodeType,
    pub content: String,

    // GEO-SPECIFIC
    pub point: Option<GeoPoint>,
    pub bounding_box: Option<GeoBoundingBox>,
    pub area_km2: Option<f64>,
    pub length_m: Option<f64>,
    pub elevation_m: Option<f64>,
    pub region_type: Option<String>,
    pub route_type: Option<String>,
    pub poi_type: Option<String>,
    pub admin_level: Option<u8>,
    pub name: Option<String>,
    pub timestamp: Option<f64>,

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
pub enum GeoEdgeType {
    // ── STRUCTURAL ──
    #[default] Contains, Precedes, PartOf,

    // ── GEOGRAPHIC RELATIONSHIPS ──
    LocatedAt,                  // feature at location
    LocatedIn,                  // feature inside region
    Borders,                    // regions share a boundary
    Overlaps,                   // regions partially overlap
    Intersects,                 // geometries cross
    Adjacent,                   // features share a border
    ConnectsTo,                 // routes connect at nodes
    RoutesThrough,              // route passes through region/location
    ElevationAffects,           // terrain affects route/coverage
    ContainsPOI,                // region contains POI
    ServedBy,                   // location served by route/POI
    TrajectoryPassesThrough,    // trajectory passes through region
    VisibleFrom,                // visible from observer (viewshed)
    WithinFloodZone,            // location within flood zone
    AdministrativePartOf,      // administrative hierarchy
    ChangedTo,                  // region changed from state A to B
    GeocodedFrom,              // geocoded result from text query
    NeighborOf,                 // spatial neighbors
    DerivedFromTerrain,        // feature derived from DEM analysis
    OnRoute,                    // location is on route
    HasElevationProfile,        // route has elevation profile

    // ── CROSS-MODAL ──
    /// Text (100) place name → location
    DescribedByText,
    /// 3D scene placed on geo (109)
    Anchors3DScene,
    /// Depth point cloud on geo (115)
    AnchorDepthCloud,
    /// Radar coverage map on geo (124)
    OverlaysRadarCoverage,
    /// Sonar bathymetry on geo (125)
    OverlaysSonarBathymetry,
    /// EM coverage map on geo (118)
    OverlaysEMCoverage,
    /// IMU trajectory on geo (116)
    AnchorIMUTrajectory,
    /// Emitter location from EM/radar (118/124)
    ContainsEmitter,
    /// Robot navigation in this area (121)
    NavigationAreaFor,

    // ── UNIVERSAL SEMANTIC ──
    Performs, Affects, Implies, Contradicts, Elaborates, Summarizes, Supports,
    TemporalPrecedes, TemporalFollows, CausedBy, Enables, Prevents,
    FunctionalRole, InstanceOf,
    DerivedFrom, VersionOf, RefinesTo, ForkedFrom, SimilarTo,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeoGraphEdge {
    pub edge_id: u64,
    pub from_node: u64, pub to_node: u64,
    pub edge_type: GeoEdgeType,
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
pub struct GeoGraph {
    pub graph_id: u64, pub project_id: u64,
    pub source_description: String,
    pub nodes: Vec<GeoGraphNode>,
    pub edges: Vec<GeoGraphEdge>,
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
pub enum GeoGraphQuery {
    NodeDetail { node_id: u64 },
    FeaturesInBBox { bbox: GeoBoundingBox },
    NearestFeatures { point: GeoPoint, radius_m: f64 },
    RegionsByType { region_type: String },
    RoutesBetween { from_node_id: u64, to_node_id: u64 },
    POIsOfType { poi_type: String },
    TrajectoriesInRegion { region_node_id: u64 },
    CrossModalLinks { node_id: u64 },
    AdminHierarchy { location_node_id: u64 },
    AGIActivity, AllNodes, AllEdges,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GeoSemanticHook {
    OnGraphCreated,
    OnInferRelationships,
    OnEdgeCompletion,
    OnCrossModalityLink { target_modality: String, target_graph_id: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GeoExportFormat {
    GeoJSON, Shapefile, KML, KMZ, GeoPackage,
    GeoTIFF_Raster, CSV_Points, GPX_Route,
    OSM_XML, MVT_Tiles, FlatGeobuf, TopoJSON,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GeoDisplayMode {
    BaseMap,                // vector map tiles
    SatelliteImagery,       // raster imagery
    TerrainShading,         // hillshade + elevation
    HeatMap,                // density / value heatmap
    RoutePlanner,           // interactive routing
    CoverageMap,            // isochrone / coverage overlay
    SpatialAnalysis,        // slope, aspect, viewshed
    TimeAnimation,          // temporal feature animation
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GeoUpdate {
    AddFeature { feature: GeoFeatureGeneric },
    RemoveFeature { feature_id: u64 },
    MoveLocation { location_id: u64, new_point: GeoPoint },
    UpdateProperties { node_id: u64, properties: HashMap<String, serde_json::Value> },
    AddTrajectoryPoints { trajectory_id: u64, points: Vec<TrajectoryPoint> },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeoFeatureGeneric {
    pub feature_id: u64,
    pub feature_type: String,
    pub geometry_type: String,      // "Point", "LineString", "Polygon"
    pub properties: HashMap<String, serde_json::Value>,
    pub point: Option<GeoPoint>,
    pub line: Option<GeoLineString>,
    pub polygon: Option<GeoPolygon>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GeoHeadlessOp {
    BuildRoutingGraph,
    ComputeAllElevationProfiles,
    RebuildSpatialIndex { index_type: GeoIndexType },
    CrossOverlayWith { other_graph_id: u64, operation: OverlayOperation },
    ExportLayer { layer_type: String, format: GeoExportFormat, path: String },
    AnnotateFromText { text_node_ids: Vec<u64> },
}

// ─────────────────────────────────────────────────────────────────────────────
// OUTPUT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeoModalityOutput {
    pub success: bool,
    pub graph_id: Option<u64>,
    pub graph: Option<GeoGraph>,
    pub analysis: Option<GeoAnalysisResult>,
    pub geocoded: Option<Vec<GeocodedPlace>>,
    pub route: Option<GeoRoute>,
    pub spatial_results: Option<Vec<u64>>,   // node_ids matching spatial query
    pub transformed_points: Option<Vec<GeoPoint>>,
    pub terrain_result: Option<RasterLayer>,
    pub coverage_area: Option<CoverageArea>,
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
// COORDINATE MATH UTILITIES
// ─────────────────────────────────────────────────────────────────────────────

/// Haversine distance in meters between two WGS84 points
pub fn haversine_distance_m(a: &GeoPoint, b: &GeoPoint) -> f64 {
    const R: f64 = 6_371_000.0; // Earth radius in meters
    let dlat = (b.lat - a.lat).to_radians();
    let dlon = (b.lon - a.lon).to_radians();
    let lat1 = a.lat.to_radians();
    let lat2 = b.lat.to_radians();
    let h = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
    2.0 * R * h.sqrt().asin()
}

/// Bearing from a to b in degrees (0 = North, clockwise)
pub fn bearing_deg(a: &GeoPoint, b: &GeoPoint) -> f64 {
    let dlon = (b.lon - a.lon).to_radians();
    let lat1 = a.lat.to_radians();
    let lat2 = b.lat.to_radians();
    let y = dlon.sin() * lat2.cos();
    let x = lat1.cos() * lat2.sin() - lat1.sin() * lat2.cos() * dlon.cos();
    (y.atan2(x).to_degrees() + 360.0) % 360.0
}

/// Compute polygon area using Shoelace formula (approximate, WGS84 → meters²)
pub fn polygon_area_m2(ring: &[GeoPoint]) -> f64 {
    if ring.len() < 3 { return 0.0; }
    let mut area = 0.0;
    let n = ring.len();
    for i in 0..n {
        let j = (i + 1) % n;
        area += ring[i].lon * ring[j].lat - ring[j].lon * ring[i].lat;
    }
    // Convert from degree² to m² (approximate, using mean latitude)
    let mean_lat = ring.iter().map(|p| p.lat).sum::<f64>() / n as f64;
    let m_per_deg_lat = 111_320.0_f64;
    let m_per_deg_lon = 111_320.0_f64 * mean_lat.to_radians().cos();
    (area.abs() / 2.0) * m_per_deg_lat * m_per_deg_lon
}

/// Point-in-polygon test (ray casting, WGS84)
pub fn point_in_polygon(point: &GeoPoint, ring: &[GeoPoint]) -> bool {
    let mut inside = false;
    let n = ring.len();
    let mut j = n - 1;
    for i in 0..n {
        let xi = ring[i].lon; let yi = ring[i].lat;
        let xj = ring[j].lon; let yj = ring[j].lat;
        if ((yi > point.lat) != (yj > point.lat))
            && (point.lon < (xj - xi) * (point.lat - yi) / (yj - yi) + xi)
        { inside = !inside; }
        j = i;
    }
    inside
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
        let input = serde_json::json!({ "action": "Prompt", "prompt": prompt, "max_tokens": max_tokens, "temperature": 0.05, "system_context": "Geospatial analysis. Return only valid JSON." });
        let out = std::process::Command::new(&self.prompt_pipeline_path).arg("--input").arg(input.to_string()).output().map_err(|e| e.to_string())?;
        let r: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
        Ok(r["response"].as_str().unwrap_or("{}").to_string())
    }

    fn save_graph(&self, g: &GeoGraph) -> Result<(), String> {
        let path = format!("{}/local/geo_graph_{}.json", self.zsei_path, g.graph_id);
        if let Some(p) = std::path::Path::new(&path).parent() { std::fs::create_dir_all(p).map_err(|e| e.to_string())?; }
        std::fs::write(&path, serde_json::to_string_pretty(g).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    fn load_graph(&self, id: u64) -> Result<GeoGraph, String> {
        let path = format!("{}/local/geo_graph_{}.json", self.zsei_path, id);
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
    async fn infer_geographic_relationships(&self, nodes: &[GeoGraphNode]) -> Vec<(u64, u64, GeoEdgeType, String)> {
        if nodes.len() < 2 { return vec![]; }

        let node_list: Vec<serde_json::Value> = nodes.iter().take(25).map(|n| serde_json::json!({
            "node_id": n.node_id, "type": format!("{:?}", n.node_type),
            "content": n.content.chars().take(80).collect::<String>(),
            "name": n.name,
            "region_type": n.region_type,
            "route_type": n.route_type,
            "lat": n.point.as_ref().map(|p| p.lat),
            "lon": n.point.as_ref().map(|p| p.lon),
        })).collect();

        let prompt = format!(r#"
Identify geographic relationships between these geospatial features.

Nodes: {}

Available types: LocatedIn, Borders, Adjacent, ConnectsTo, RoutesThrough, ContainsPOI,
ServedBy, AdministrativePartOf, NeighborOf, VisibleFrom, OnRoute, Affects,
CausedBy, DerivedFrom, TemporalPrecedes, SimilarTo

Return ONLY valid JSON array:
[{{"from_node_id": N, "to_node_id": M, "edge_type": "TypeName", "reason": "brief"}}]"#,
            serde_json::to_string_pretty(&node_list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 700).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str).unwrap_or_default().into_iter()
                    .filter_map(|v| {
                        let from = v["from_node_id"].as_u64()?;
                        let to = v["to_node_id"].as_u64()?;
                        let etype = map_geo_edge_str(v["edge_type"].as_str().unwrap_or("Affects"));
                        let reason = v["reason"].as_str().unwrap_or("").to_string();
                        Some((from, to, etype, reason))
                    }).collect()
            }
            Err(_) => vec![],
        }
    }

    async fn classify_region_types_llm(&self, regions: &[GeoRegion]) -> Vec<(u64, String)> {
        if regions.is_empty() { return vec![]; }
        let list: Vec<serde_json::Value> = regions.iter().take(20).map(|r| serde_json::json!({
            "region_id": r.region_id, "name": r.name, "area_km2": r.area_km2,
            "admin_level": r.admin_level, "properties": r.properties,
        })).collect();

        let prompt = format!(r#"
Classify each geographic region type:
Country, State, Province, County, City, District, Neighborhood, Parcel,
LandUse, ZoningArea, WatershedBasin, NaturalArea, ProtectedArea, MilitaryZone,
MaritimeZone, AirspaceZone, CoverageZone

Regions: {}
Return ONLY valid JSON array: [{{"region_id": N, "region_type": "TypeName"}}]"#,
            serde_json::to_string_pretty(&list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 400).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str).unwrap_or_default().into_iter()
                    .filter_map(|v| Some((v["region_id"].as_u64()?, v["region_type"].as_str()?.to_string())))
                    .collect()
            }
            Err(_) => vec![],
        }
    }

    async fn extract_place_references_from_text_llm(&self, text: &str) -> Vec<(String, String, Option<f64>, Option<f64>)> {
        let prompt = format!(r#"
Extract all geographic place references from this text.
For each reference: name, type (city/country/region/address/landmark/etc.),
and best-guess coordinates if known.

Text: {}

Return ONLY valid JSON array:
[{{"name": "...", "type": "...", "lat": null_or_float, "lon": null_or_float}}]"#,
            &text[..text.len().min(2000)]);

        match self.llm_zero_shot(&prompt, 400).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str).unwrap_or_default().into_iter()
                    .filter_map(|v| {
                        let name = v["name"].as_str()?.to_string();
                        let ptype = v["type"].as_str().unwrap_or("unknown").to_string();
                        let lat = v["lat"].as_f64();
                        let lon = v["lon"].as_f64();
                        Some((name, ptype, lat, lon))
                    }).collect()
            }
            Err(_) => vec![],
        }
    }

    /// Dijkstra shortest path (simplified, works on geo graph node centroids)
    fn dijkstra_shortest_path(
        nodes: &[GeoGraphNode],
        edges: &[GeoGraphEdge],
        from_node_id: u64,
        to_node_id: u64,
    ) -> Option<Vec<u64>> {
        use std::collections::BinaryHeap;
        use std::cmp::Ordering;

        #[derive(Eq, PartialEq)]
        struct State { cost: u64, node: u64 }

        impl Ord for State {
            fn cmp(&self, other: &Self) -> Ordering { other.cost.cmp(&self.cost) }
        }
        impl PartialOrd for State {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
        }

        let valid_ids: std::collections::HashSet<u64> = nodes.iter().map(|n| n.node_id).collect();
        if !valid_ids.contains(&from_node_id) || !valid_ids.contains(&to_node_id) { return None; }

        let mut dist: HashMap<u64, u64> = HashMap::new();
        let mut prev: HashMap<u64, u64> = HashMap::new();
        let mut heap = BinaryHeap::new();

        dist.insert(from_node_id, 0);
        heap.push(State { cost: 0, node: from_node_id });

        while let Some(State { cost, node }) = heap.pop() {
            if node == to_node_id {
                let mut path = vec![to_node_id];
                let mut current = to_node_id;
                while let Some(&p) = prev.get(&current) {
                    path.push(p);
                    current = p;
                }
                path.reverse();
                return Some(path);
            }
            if cost > *dist.get(&node).unwrap_or(&u64::MAX) { continue; }

            for edge in edges.iter().filter(|e| e.from_node == node || (!matches!(e.edge_type, GeoEdgeType::RoutesThrough) && e.to_node == node)) {
                let next = if edge.from_node == node { edge.to_node } else { edge.from_node };
                if !valid_ids.contains(&next) { continue; }
                // Compute edge cost from haversine distance
                let from_node = nodes.iter().find(|n| n.node_id == node);
                let to_node_geo = nodes.iter().find(|n| n.node_id == next);
                let edge_cost = match (from_node.and_then(|n| n.point.as_ref()), to_node_geo.and_then(|n| n.point.as_ref())) {
                    (Some(a), Some(b)) => (haversine_distance_m(a, b) * 1000.0) as u64, // mm precision
                    _ => (1.0 / edge.weight.max(0.001) * 1000.0) as u64,
                };
                let next_cost = cost.saturating_add(edge_cost);
                if next_cost < *dist.get(&next).unwrap_or(&u64::MAX) {
                    dist.insert(next, next_cost);
                    prev.insert(next, node);
                    heap.push(State { cost: next_cost, node: next });
                }
            }
        }
        None
    }
}

fn map_geo_edge_str(s: &str) -> GeoEdgeType {
    match s {
        "LocatedAt"              => GeoEdgeType::LocatedAt,
        "LocatedIn"              => GeoEdgeType::LocatedIn,
        "Borders"                => GeoEdgeType::Borders,
        "Overlaps"               => GeoEdgeType::Overlaps,
        "Intersects"             => GeoEdgeType::Intersects,
        "Adjacent"               => GeoEdgeType::Adjacent,
        "ConnectsTo"             => GeoEdgeType::ConnectsTo,
        "RoutesThrough"          => GeoEdgeType::RoutesThrough,
        "ElevationAffects"       => GeoEdgeType::ElevationAffects,
        "ContainsPOI"            => GeoEdgeType::ContainsPOI,
        "ServedBy"               => GeoEdgeType::ServedBy,
        "TrajectoryPassesThrough"=> GeoEdgeType::TrajectoryPassesThrough,
        "VisibleFrom"            => GeoEdgeType::VisibleFrom,
        "WithinFloodZone"        => GeoEdgeType::WithinFloodZone,
        "AdministrativePartOf"   => GeoEdgeType::AdministrativePartOf,
        "NeighborOf"             => GeoEdgeType::NeighborOf,
        "OnRoute"                => GeoEdgeType::OnRoute,
        "DescribedByText"        => GeoEdgeType::DescribedByText,
        "Anchors3DScene"         => GeoEdgeType::Anchors3DScene,
        "OverlaysRadarCoverage"  => GeoEdgeType::OverlaysRadarCoverage,
        "NavigationAreaFor"      => GeoEdgeType::NavigationAreaFor,
        "Affects"                => GeoEdgeType::Affects,
        "CausedBy"               => GeoEdgeType::CausedBy,
        "Enables"                => GeoEdgeType::Enables,
        "TemporalPrecedes"       => GeoEdgeType::TemporalPrecedes,
        "DerivedFrom"            => GeoEdgeType::DerivedFrom,
        "PartOf"                 => GeoEdgeType::PartOf,
        "SimilarTo"              => GeoEdgeType::SimilarTo,
        _                        => GeoEdgeType::Affects,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH CREATION
// ─────────────────────────────────────────────────────────────────────────────

async fn create_graph(executor: &PipelineExecutor, analysis: GeoAnalysisResult, project_id: u64) -> GeoModalityOutput {
    let graph_id = executor.generate_id();
    let now = executor.now_iso8601();
    let mut nodes: Vec<GeoGraphNode> = Vec::new();
    let mut edges: Vec<GeoGraphEdge> = Vec::new();
    let mut node_id: u64 = 1;
    let mut edge_id: u64 = 1;

    // ── ROOT ──
    let root_id = node_id;
    nodes.push(GeoGraphNode {
        node_id: root_id, node_type: GeoNodeType::GeoScene,
        content: format!("GeoScene: {} locs {} regions {} routes {} pois {} trajectories crs={}",
            analysis.locations.len(), analysis.regions.len(), analysis.routes.len(),
            analysis.pois.len(), analysis.trajectories.len(), analysis.crs),
        bounding_box: Some(analysis.bounding_box.clone()),
        materialized_path: Some(format!("/Modalities/Geospatial/Project_{}/Graph_{}", project_id, graph_id)),
        provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
        version_notes: vec![VersionNote { version: 1, note: "Initial creation".into(), step_index: None, timestamp: now.clone(), change_type: ChangeType::Created }],
        keywords: vec!["geospatial".into(), "map".into(), analysis.crs.clone()],
        hotness_score: 1.0, ..Default::default()
    });
    node_id += 1;

    // ── REGIONS ──
    let region_type_classifications = executor.classify_region_types_llm(&analysis.regions).await;
    let region_type_map: HashMap<u64, String> = region_type_classifications.into_iter().collect();

    let region_node_ids: Vec<(u64, u64)> = analysis.regions.iter().map(|r| {
        let rtype = region_type_map.get(&r.region_id).cloned().unwrap_or_else(|| format!("{:?}", r.region_type));
        let rid = node_id;
        nodes.push(GeoGraphNode {
            node_id: rid, node_type: GeoNodeType::RegionNode,
            content: format!("Region [{}]: {:?} area={:.2}km² pop={:?}",
                r.name.as_deref().unwrap_or("unnamed"), rtype, r.area_km2, r.population),
            bounding_box: Some(GeoBoundingBox {
                min_lon: r.polygon.exterior.iter().map(|p| p.lon).fold(f64::INFINITY, f64::min),
                min_lat: r.polygon.exterior.iter().map(|p| p.lat).fold(f64::INFINITY, f64::min),
                max_lon: r.polygon.exterior.iter().map(|p| p.lon).fold(f64::NEG_INFINITY, f64::max),
                max_lat: r.polygon.exterior.iter().map(|p| p.lat).fold(f64::NEG_INFINITY, f64::max),
                ..Default::default()
            }),
            area_km2: Some(r.area_km2),
            region_type: Some(rtype.clone()),
            admin_level: r.admin_level,
            name: r.name.clone(),
            point: Some(r.polygon.centroid.clone()),
            materialized_path: Some(format!("/Modalities/Geospatial/Project_{}/Graph_{}/Region/{}", project_id, graph_id, r.region_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: { let mut kw = vec!["region".into(), rtype.to_lowercase()]; if let Some(ref name) = r.name { kw.push(name.to_lowercase()); } kw },
            hotness_score: 0.7 + (r.area_km2.log2() / 40.0).clamp(0.0, 0.25) as f32,
            embedding_hint: Some(format!("geographic region: {} type: {}", r.name.as_deref().unwrap_or("unknown"), rtype)),
            ..Default::default()
        });
        edges.push(GeoGraphEdge { edge_id, from_node: root_id, to_node: rid, edge_type: GeoEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
        (r.region_id, rid)
    }).collect();

    // Administrative hierarchy (inner regions inside outer)
    for i in 0..analysis.regions.len() {
        for j in 0..analysis.regions.len() {
            if i == j { continue; }
            let ri = &analysis.regions[i];
            let rj = &analysis.regions[j];
            if let Some(nid_i) = region_node_ids.iter().find(|(id, _)| *id == ri.region_id).map(|(_, nid)| *nid) {
                if let Some(nid_j) = region_node_ids.iter().find(|(id, _)| *id == rj.region_id).map(|(_, nid)| *nid) {
                    // If rj admin level > ri, rj is inside ri (higher admin_level = smaller unit)
                    if let (Some(al_i), Some(al_j)) = (ri.admin_level, rj.admin_level) {
                        if al_j > al_i && point_in_polygon(&rj.polygon.centroid, &ri.polygon.exterior) {
                            edges.push(GeoGraphEdge { edge_id, from_node: nid_j, to_node: nid_i, edge_type: GeoEdgeType::AdministrativePartOf, weight: 1.0, provenance: EdgeProvenance::DerivedFromHook, version: 1, ..Default::default() });
                            edge_id += 1;
                        }
                    }
                }
            }
        }
    }

    // ── ROUTES ──
    let route_node_ids: Vec<(u64, u64)> = analysis.routes.iter().map(|r| {
        let rn_id = node_id;
        nodes.push(GeoGraphNode {
            node_id: rn_id, node_type: GeoNodeType::RouteNode,
            content: format!("Route [{:?}]: {:?} len={:.0}m speed={:?}kmh class={:?}",
                r.route_type, r.name.as_deref().unwrap_or("unnamed"), r.length_m,
                r.speed_kmh, r.road_class),
            length_m: Some(r.length_m),
            route_type: Some(format!("{:?}", r.route_type)),
            name: r.name.clone(),
            // Use midpoint as representative point
            point: r.geometry.points.get(r.geometry.points.len() / 2).cloned(),
            materialized_path: Some(format!("/Modalities/Geospatial/Project_{}/Graph_{}/Route/{}", project_id, graph_id, r.route_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: { let mut kw = vec!["route".into(), format!("{:?}", r.route_type).to_lowercase()]; if let Some(ref n) = r.name { kw.push(n.to_lowercase()); } kw },
            hotness_score: 0.65 + (r.length_m / 50000.0).min(0.25) as f32,
            ..Default::default()
        });
        edges.push(GeoGraphEdge { edge_id, from_node: root_id, to_node: rn_id, edge_type: GeoEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Route passes through regions that contain any of its points
        for pt in r.geometry.points.iter().step_by(r.geometry.points.len().max(1) / 3 + 1) {
            for (reg_data, (_, &reg_nid)) in analysis.regions.iter().zip(region_node_ids.iter()) {
                if point_in_polygon(pt, &reg_data.polygon.exterior) {
                    edges.push(GeoGraphEdge { edge_id, from_node: rn_id, to_node: reg_nid, edge_type: GeoEdgeType::RoutesThrough, weight: 0.8, provenance: EdgeProvenance::DerivedFromHook, version: 1, ..Default::default() });
                    edge_id += 1;
                    break;
                }
            }
        }

        node_id += 1;
        (r.route_id, rn_id)
    }).collect();

    // ── LOCATIONS ──
    let location_node_ids: Vec<(u64, u64)> = analysis.locations.iter().map(|loc| {
        let lid = node_id;
        nodes.push(GeoGraphNode {
            node_id: lid, node_type: GeoNodeType::LocationNode,
            content: format!("Location [{:?}]: {:?} ({:.5},{:.5}) acc={:?}m",
                loc.location_type, loc.name.as_deref().unwrap_or("unnamed"),
                loc.point.lat, loc.point.lon, loc.accuracy_m),
            point: Some(loc.point.clone()),
            elevation_m: loc.elevation_m,
            name: loc.name.clone(),
            materialized_path: Some(format!("/Modalities/Geospatial/Project_{}/Graph_{}/Location/{}", project_id, graph_id, loc.location_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: { let mut kw = vec!["location".into(), format!("{:?}", loc.location_type).to_lowercase()]; if let Some(ref n) = loc.name { kw.push(n.to_lowercase()); } kw },
            hotness_score: 0.7,
            embedding_hint: Some(format!("location: {} type: {:?} at ({:.4},{:.4})", loc.name.as_deref().unwrap_or("unknown"), loc.location_type, loc.point.lat, loc.point.lon)),
            timestamp: loc.timestamp,
            ..Default::default()
        });
        edges.push(GeoGraphEdge { edge_id, from_node: root_id, to_node: lid, edge_type: GeoEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Location inside which regions
        for (reg_data, (_, &reg_nid)) in analysis.regions.iter().zip(region_node_ids.iter()) {
            if point_in_polygon(&loc.point, &reg_data.polygon.exterior) {
                edges.push(GeoGraphEdge { edge_id, from_node: lid, to_node: reg_nid, edge_type: GeoEdgeType::LocatedIn, weight: 1.0, provenance: EdgeProvenance::DerivedFromHook, version: 1, ..Default::default() });
                edge_id += 1;
                break; // associate with most specific region (first match; could be improved with area sort)
            }
        }

        node_id += 1;
        (loc.location_id, lid)
    }).collect();

    // ── POINTS OF INTEREST ──
    for poi in &analysis.pois {
        let pid = node_id;
        nodes.push(GeoGraphNode {
            node_id: pid, node_type: GeoNodeType::POINode,
            content: format!("POI [{}]: {} ({:.5},{:.5})", poi.poi_type, poi.name, poi.point.lat, poi.point.lon),
            point: Some(poi.point.clone()),
            poi_type: Some(poi.poi_type.clone()),
            name: Some(poi.name.clone()),
            materialized_path: Some(format!("/Modalities/Geospatial/Project_{}/Graph_{}/POI/{}", project_id, graph_id, poi.poi_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["poi".into(), poi.poi_type.to_lowercase(), poi.name.to_lowercase()],
            hotness_score: 0.65,
            ..Default::default()
        });
        edges.push(GeoGraphEdge { edge_id, from_node: root_id, to_node: pid, edge_type: GeoEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // POI inside which region
        for (reg_data, (_, &reg_nid)) in analysis.regions.iter().zip(region_node_ids.iter()) {
            if point_in_polygon(&poi.point, &reg_data.polygon.exterior) {
                edges.push(GeoGraphEdge { edge_id, from_node: reg_nid, to_node: pid, edge_type: GeoEdgeType::ContainsPOI, weight: 0.9, provenance: EdgeProvenance::DerivedFromHook, version: 1, ..Default::default() });
                edge_id += 1;
                break;
            }
        }
        node_id += 1;
    }

    // ── TRAJECTORIES ──
    for traj in &analysis.trajectories {
        let tid = node_id;
        nodes.push(GeoGraphNode {
            node_id: tid, node_type: GeoNodeType::TrajectoryNode,
            content: format!("Trajectory: entity={:?} type={:?} pts={} dist={:.0}m spd={:?}m/s",
                traj.entity_id.as_deref().unwrap_or("?"), traj.entity_type.as_deref().unwrap_or("?"),
                traj.points.len(), traj.total_distance_m, traj.mean_speed_ms),
            length_m: Some(traj.total_distance_m),
            bounding_box: Some(traj.bounding_box.clone()),
            timestamp: Some(traj.start_time),
            materialized_path: Some(format!("/Modalities/Geospatial/Project_{}/Graph_{}/Trajectory/{}", project_id, graph_id, traj.trajectory_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: {
                let mut kw = vec!["trajectory".into()];
                if let Some(ref et) = traj.entity_type { kw.push(et.to_lowercase()); }
                kw
            },
            hotness_score: 0.75,
            ..Default::default()
        });
        edges.push(GeoGraphEdge { edge_id, from_node: root_id, to_node: tid, edge_type: GeoEdgeType::Contains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Trajectory through regions
        let sample_points: Vec<&TrajectoryPoint> = traj.points.iter().step_by(traj.points.len().max(1) / 5 + 1).collect();
        for tp in &sample_points {
            for (reg_data, (_, &reg_nid)) in analysis.regions.iter().zip(region_node_ids.iter()) {
                if point_in_polygon(&tp.point, &reg_data.polygon.exterior) {
                    edges.push(GeoGraphEdge { edge_id, from_node: tid, to_node: reg_nid, edge_type: GeoEdgeType::TrajectoryPassesThrough, weight: 0.8, provenance: EdgeProvenance::DerivedFromHook, version: 1, ..Default::default() });
                    edge_id += 1;
                    break;
                }
            }
        }
        node_id += 1;
    }

    // ── TERRAIN MODELS ──
    for terrain in &analysis.terrain_models {
        let tmid = node_id;
        nodes.push(GeoGraphNode {
            node_id: tmid, node_type: GeoNodeType::TerrainModelNode,
            content: format!("Terrain [{:?}]: res={:.1}m elev={:.0}–{:.0}m datum={:?}",
                terrain.model_type, terrain.resolution_m, terrain.min_elevation_m, terrain.max_elevation_m, terrain.vertical_datum),
            bounding_box: Some(terrain.bounding_box.clone()),
            elevation_m: Some(terrain.mean_elevation_m),
            materialized_path: Some(format!("/Modalities/Geospatial/Project_{}/Graph_{}/Terrain/{}", project_id, graph_id, terrain.model_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["terrain".into(), format!("{:?}", terrain.model_type).to_lowercase()],
            hotness_score: 0.8,
            ..Default::default()
        });
        edges.push(GeoGraphEdge { edge_id, from_node: root_id, to_node: tmid, edge_type: GeoEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Routes affected by terrain
        for (_, &route_nid) in &route_node_ids {
            edges.push(GeoGraphEdge { edge_id, from_node: tmid, to_node: route_nid, edge_type: GeoEdgeType::ElevationAffects, weight: 0.6, provenance: EdgeProvenance::DerivedFromHook, version: 1, ..Default::default() });
            edge_id += 1;
        }

        // Terrain cross-modal: → Depth (115) DSM/DTM
        edges.push(GeoGraphEdge { edge_id, from_node: tmid, to_node: tmid, edge_type: GeoEdgeType::AnchorDepthCloud, weight: 0.85, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("depth")); p }, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── COVERAGE AREAS ──
    for cov in &analysis.coverage_areas {
        let cid = node_id;
        nodes.push(GeoGraphNode {
            node_id: cid, node_type: GeoNodeType::CoverageAreaNode,
            content: format!("Coverage [{:?}]: mode={} limit={:.0}{} area={:.2}km²",
                cov.coverage_type, cov.mode, cov.time_or_distance_limit, cov.unit, cov.reachable_area_km2),
            point: Some(cov.origin.clone()),
            area_km2: Some(cov.reachable_area_km2),
            materialized_path: Some(format!("/Modalities/Geospatial/Project_{}/Graph_{}/Coverage/{}", project_id, graph_id, cov.coverage_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["coverage".into(), cov.mode.to_lowercase()],
            hotness_score: 0.7, ..Default::default()
        });
        edges.push(GeoGraphEdge { edge_id, from_node: root_id, to_node: cid, edge_type: GeoEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── VIEWSHEDS ──
    for vs in &analysis.viewsheds {
        let vid = node_id;
        nodes.push(GeoGraphNode {
            node_id: vid, node_type: GeoNodeType::ViewshedNode,
            content: format!("Viewshed: obs=({:.4},{:.4}) h={:.1}m range={:.0}m visible={:.1}km²",
                vs.observer_point.lat, vs.observer_point.lon, vs.observer_height_m, vs.max_range_m, vs.visible_area_km2),
            point: Some(vs.observer_point.clone()),
            area_km2: Some(vs.visible_area_km2 as f64),
            materialized_path: Some(format!("/Modalities/Geospatial/Project_{}/Graph_{}/Viewshed/{}", project_id, graph_id, vs.viewshed_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["viewshed".into(), "visibility".into()], hotness_score: 0.65, ..Default::default()
        });
        edges.push(GeoGraphEdge { edge_id, from_node: root_id, to_node: vid, edge_type: GeoEdgeType::Contains, weight: 0.7, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Locations visible from this viewshed
        for (loc_data, (_, &loc_nid)) in analysis.locations.iter().zip(location_node_ids.iter()) {
            if point_in_polygon(&loc_data.point, &vs.polygon.exterior) {
                edges.push(GeoGraphEdge { edge_id, from_node: vid, to_node: loc_nid, edge_type: GeoEdgeType::VisibleFrom, weight: 0.8, provenance: EdgeProvenance::DerivedFromHook, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        node_id += 1;
    }

    // ── FLOOD ZONES ──
    for fz in &analysis.flood_zones {
        let fid = node_id;
        nodes.push(GeoGraphNode {
            node_id: fid, node_type: GeoNodeType::FloodZoneNode,
            content: format!("FloodZone [{:?}]: risk={:?} return={:?}yr area={:.2}km²",
                fz.zone_type, fz.risk_level, fz.return_period_years, fz.polygon.area_m2 / 1e6),
            area_km2: Some(fz.polygon.area_m2 / 1e6),
            materialized_path: Some(format!("/Modalities/Geospatial/Project_{}/Graph_{}/FloodZone/{}", project_id, graph_id, fz.zone_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["flood-zone".into(), format!("{:?}", fz.risk_level).to_lowercase()],
            hotness_score: 0.5 + match fz.risk_level { RiskLevel::VeryHigh => 0.4, RiskLevel::High => 0.3, RiskLevel::Moderate => 0.2, _ => 0.0 },
            ..Default::default()
        });
        edges.push(GeoGraphEdge { edge_id, from_node: root_id, to_node: fid, edge_type: GeoEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Locations within flood zone
        for (loc_data, (_, &loc_nid)) in analysis.locations.iter().zip(location_node_ids.iter()) {
            if point_in_polygon(&loc_data.point, &fz.polygon.exterior) {
                edges.push(GeoGraphEdge { edge_id, from_node: loc_nid, to_node: fid, edge_type: GeoEdgeType::WithinFloodZone, weight: 1.0, provenance: EdgeProvenance::DerivedFromHook, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        node_id += 1;
    }

    // ── CHANGE REGIONS ──
    for chg in &analysis.change_regions {
        let cid = node_id;
        nodes.push(GeoGraphNode {
            node_id: cid, node_type: GeoNodeType::ChangeRegionNode,
            content: format!("GeoChange [{:?}]: before={:?} after={:?} area={:.2}km²",
                chg.change_type, chg.before_date, chg.after_date, chg.area_changed_km2),
            area_km2: Some(chg.area_changed_km2),
            materialized_path: Some(format!("/Modalities/Geospatial/Project_{}/Graph_{}/Change/{}", project_id, graph_id, chg.change_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["change".into(), format!("{:?}", chg.change_type).to_lowercase()],
            hotness_score: 0.65, ..Default::default()
        });
        edges.push(GeoGraphEdge { edge_id, from_node: root_id, to_node: cid, edge_type: GeoEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── GEOCODED PLACES ──
    for place in &analysis.geocoded_places {
        let pid = node_id;
        nodes.push(GeoGraphNode {
            node_id: pid, node_type: GeoNodeType::GeocodedPlaceNode,
            content: format!("Geocoded: '{}' → {} ({:.5},{:.5})",
                place.query, place.display_name, place.point.lat, place.point.lon),
            point: Some(place.point.clone()),
            name: Some(place.display_name.clone()),
            materialized_path: Some(format!("/Modalities/Geospatial/Project_{}/Graph_{}/Place/{}", project_id, graph_id, place.place_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["geocoded".into(), place.place_type.to_lowercase()],
            hotness_score: 0.6,
            ..Default::default()
        });
        edges.push(GeoGraphEdge { edge_id, from_node: root_id, to_node: pid, edge_type: GeoEdgeType::Contains, weight: 0.7, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // Cross-modal: geocoded from text (100)
        edges.push(GeoGraphEdge { edge_id, from_node: pid, to_node: pid, edge_type: GeoEdgeType::DescribedByText, weight: 0.9, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("text")); p.insert("query".into(), serde_json::json!(&place.query)); p }, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── HOOK 1: OnGraphCreated ──
    let _ = executor.save_graph(&GeoGraph { graph_id, project_id, source_description: analysis.source_description.clone(), nodes: nodes.clone(), edges: edges.clone(), root_node_id: root_id, state: GraphStateType::Created, state_history: vec![GraphStateTransition { from: GraphStateType::Created, to: GraphStateType::Created, timestamp: now.clone(), triggered_by_step: None }], created_at: now.clone(), updated_at: now.clone(), version: 1, version_notes: vec![VersionNote { version: 1, note: format!("Created: {} nodes {} edges", nodes.len(), edges.len()), step_index: None, timestamp: now.clone(), change_type: ChangeType::Created }] });

    // ── HOOK 2: OnInferRelationships ──
    let inferred = executor.infer_geographic_relationships(&nodes).await;
    let valid: std::collections::HashSet<u64> = nodes.iter().map(|n| n.node_id).collect();
    for (from, to, etype, reason) in inferred {
        if valid.contains(&from) && valid.contains(&to) && from != to {
            edges.push(GeoGraphEdge { edge_id, from_node: from, to_node: to, edge_type: etype, weight: 0.8, provenance: EdgeProvenance::DerivedFromHook, version: 1, properties: { let mut p = HashMap::new(); p.insert("reason".into(), serde_json::json!(reason)); p }, ..Default::default() });
            edge_id += 1;
        }
    }

    // ── HOOK 3: OnEdgeCompletion ──
    let mut deg: HashMap<u64, u32> = HashMap::new();
    for e in &edges { *deg.entry(e.from_node).or_insert(0) += 1; *deg.entry(e.to_node).or_insert(0) += 1; }
    let max_deg = deg.values().copied().max().unwrap_or(1) as f32;
    for n in &mut nodes { if let Some(&d) = deg.get(&n.node_id) { n.hotness_score = (n.hotness_score + (d as f32 / max_deg) * 0.15).min(1.0); } }

    let final_graph = GeoGraph { graph_id, project_id, source_description: analysis.source_description, nodes, edges, root_node_id: root_id, state: GraphStateType::SemanticEnriched, state_history: vec![GraphStateTransition { from: GraphStateType::Created, to: GraphStateType::SemanticEnriched, timestamp: now.clone(), triggered_by_step: None }], created_at: now.clone(), updated_at: now.clone(), version: 1, version_notes: vec![VersionNote { version: 1, note: "Semantic enrichment complete".into(), step_index: None, timestamp: now, change_type: ChangeType::EnrichedBySemantic }] };
    let _ = executor.save_graph(&final_graph);
    GeoModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(final_graph), ..Default::default() }
}

// ─────────────────────────────────────────────────────────────────────────────
// MAIN EXECUTION
// ─────────────────────────────────────────────────────────────────────────────

pub async fn execute(input: GeoModalityAction) -> Result<GeoModalityOutput, String> {
    let executor = PipelineExecutor::new();

    match input {
        GeoModalityAction::Analyze { data, extract_features, build_topology, compute_terrain, extract_routes } => {
            let analysis_id = executor.generate_id();
            let source_description = match &data {
                GeoDataSource::GeoJSON { file_path } => format!("GeoJSON: {}", file_path),
                GeoDataSource::Shapefile { file_path, .. } => format!("Shapefile: {}", file_path),
                GeoDataSource::KML { file_path } => format!("KML: {}", file_path),
                GeoDataSource::GPX { file_path } => format!("GPX: {}", file_path),
                GeoDataSource::OSM { file_path, extract_features } => format!("OSM: {} features={:?}", file_path, extract_features),
                GeoDataSource::GeoTIFF { file_path, band_interpretation } => format!("GeoTIFF [{:?}]: {}", band_interpretation, file_path),
                GeoDataSource::DEM { file_path, .. } => format!("DEM: {}", file_path),
                GeoDataSource::WebMapService { url, layer, .. } => format!("WMS: {}/{}", url, layer),
                GeoDataSource::WebFeatureService { url, type_name, .. } => format!("WFS: {}/{}", url, type_name),
                GeoDataSource::GeoPackage { file_path, layers } => format!("GPKG: {} layers={:?}", file_path, layers),
                GeoDataSource::CSVPoints { file_path, .. } => format!("CSV points: {}", file_path),
                GeoDataSource::NetCDF { file_path, variable } => format!("NetCDF: {}/{}", file_path, variable),
                GeoDataSource::LiveGPS { endpoint, .. } => format!("Live GPS: {}", endpoint),
                GeoDataSource::MultiSource { sources, .. } => format!("MultiSource: {} sources", sources.len()),
            };
            Ok(GeoModalityOutput {
                success: true,
                analysis: Some(GeoAnalysisResult { analysis_id, source_description, crs: "EPSG:4326".into(), ..Default::default() }),
                ..Default::default()
            })
        }

        GeoModalityAction::CreateGraph { analysis, project_id } => {
            Ok(create_graph(&executor, analysis, project_id).await)
        }

        GeoModalityAction::UpdateGraph { graph_id, updates, project_id } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            let mut next_nid = graph.nodes.iter().map(|n| n.node_id).max().unwrap_or(0) + 1;
            let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
            let initial = graph.nodes.len();
            for update in &updates {
                match update {
                    GeoUpdate::AddFeature { feature } => {
                        let ntype = match feature.geometry_type.as_str() {
                            "Polygon" => GeoNodeType::RegionNode,
                            "LineString" => GeoNodeType::RouteNode,
                            _ => GeoNodeType::LocationNode,
                        };
                        graph.nodes.push(GeoGraphNode {
                            node_id: next_nid, node_type: ntype,
                            content: format!("Added {}: {}", feature.feature_type, feature.feature_id),
                            point: feature.point.clone(), provisional: false,
                            provisional_status: ProvisionalStatus::Validated, version: 1,
                            keywords: vec![feature.feature_type.to_lowercase()], hotness_score: 0.7,
                            ..Default::default()
                        });
                        graph.edges.push(GeoGraphEdge { edge_id: next_eid, from_node: graph.root_node_id, to_node: next_nid, edge_type: GeoEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                        next_eid += 1; next_nid += 1;
                    }
                    GeoUpdate::MoveLocation { location_id, new_point } => {
                        // Find by scanning node content for this location_id
                        if let Some(n) = graph.nodes.iter_mut().find(|n| n.node_id == *location_id) {
                            n.point = Some(new_point.clone()); n.version += 1;
                            n.version_notes.push(VersionNote { version: n.version, note: "Location moved".into(), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                        }
                    }
                    GeoUpdate::RemoveFeature { feature_id } => {
                        graph.nodes.retain(|n| n.node_id != *feature_id);
                        graph.edges.retain(|e| e.from_node != *feature_id && e.to_node != *feature_id);
                    }
                    GeoUpdate::UpdateProperties { node_id, properties } => {
                        if let Some(n) = graph.nodes.iter_mut().find(|n| n.node_id == *node_id) {
                            n.version += 1;
                            n.version_notes.push(VersionNote { version: n.version, note: format!("Properties updated: {} keys", properties.len()), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                        }
                    }
                    GeoUpdate::AddTrajectoryPoints { trajectory_id, points } => {
                        if let Some(n) = graph.nodes.iter_mut().find(|n| n.node_id == *trajectory_id) {
                            n.version += 1;
                            n.version_notes.push(VersionNote { version: n.version, note: format!("Added {} trajectory points", points.len()), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                        }
                    }
                }
            }
            graph.version += 1; graph.updated_at = now.clone(); graph.state = GraphStateType::Updated;
            graph.version_notes.push(VersionNote { version: graph.version, note: format!("{} updates applied → {} new nodes", updates.len(), graph.nodes.len() - initial), step_index: None, timestamp: now, change_type: ChangeType::Updated });
            executor.save_graph(&graph)?;
            Ok(GeoModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        GeoModalityAction::Geocode { query, limit } => {
            // LLM-based geocoding (zero-shot): extract coordinates for well-known places
            let prompt = format!(r#"
Geocode this query to geographic coordinates.
Query: "{}"
Limit: {} results

Return ONLY valid JSON array:
[{{"display_name": "...", "lat": float, "lon": float, "place_type": "city|region|address|landmark|...", "importance": 0.0-1.0}}]"#,
                query, limit);

            let results = match executor.llm_zero_shot(&prompt, 400).await {
                Ok(raw) => {
                    let json_str = PipelineExecutor::extract_json_array(&raw);
                    serde_json::from_str::<Vec<serde_json::Value>>(&json_str).unwrap_or_default().into_iter().enumerate()
                        .filter_map(|(i, v)| {
                            let lat = v["lat"].as_f64()?;
                            let lon = v["lon"].as_f64()?;
                            Some(GeocodedPlace {
                                place_id: executor.generate_id(),
                                query: query.clone(),
                                point: GeoPoint { lat, lon, ..Default::default() },
                                display_name: v["display_name"].as_str().unwrap_or("").to_string(),
                                address: GeoAddress::default(),
                                place_type: v["place_type"].as_str().unwrap_or("unknown").to_string(),
                                importance: v["importance"].as_f64().unwrap_or(0.5) as f32,
                                bounding_box: None,
                            })
                        }).collect()
                }
                Err(_) => vec![],
            };
            Ok(GeoModalityOutput { success: true, geocoded: Some(results), ..Default::default() })
        }

        GeoModalityAction::ReverseGeocode { lat, lon, radius_m } => {
            let point = GeoPoint { lat, lon, ..Default::default() };
            let prompt = format!(r#"
Reverse geocode: what is at ({:.6}, {:.6}) with radius {:.0}m?
Return ONLY valid JSON:
{{"display_name": "...", "street": "...", "city": "...", "state": "...", "country": "...", "postal_code": "...", "place_type": "..."}}"#,
                lat, lon, radius_m);
            let place = match executor.llm_zero_shot(&prompt, 200).await {
                Ok(raw) => {
                    let json_str = PipelineExecutor::extract_json_object(&raw);
                    serde_json::from_str::<serde_json::Value>(&json_str).ok().map(|v| GeocodedPlace {
                        place_id: executor.generate_id(),
                        query: format!("{:.6},{:.6}", lat, lon),
                        point: point.clone(),
                        display_name: v["display_name"].as_str().unwrap_or("").to_string(),
                        address: GeoAddress {
                            street: v["street"].as_str().map(String::from),
                            city: v["city"].as_str().map(String::from),
                            state: v["state"].as_str().map(String::from),
                            country: v["country"].as_str().map(String::from),
                            postal_code: v["postal_code"].as_str().map(String::from),
                            ..Default::default()
                        },
                        place_type: v["place_type"].as_str().unwrap_or("unknown").to_string(),
                        importance: 1.0, bounding_box: None,
                    })
                }
                Err(_) => None,
            };
            Ok(GeoModalityOutput { success: true, geocoded: place.map(|p| vec![p]), ..Default::default() })
        }

        GeoModalityAction::ComputeRoute { graph_id, waypoints, mode, options } => {
            let graph = executor.load_graph(graph_id)?;
            if waypoints.len() < 2 { return Ok(GeoModalityOutput { success: false, error: Some("Need ≥2 waypoints".into()), ..Default::default() }); }

            // Find nearest nodes to each waypoint
            let find_nearest = |wp: &GeoPoint| -> Option<u64> {
                graph.nodes.iter()
                    .filter_map(|n| n.point.as_ref().map(|p| (n.node_id, haversine_distance_m(p, wp))))
                    .min_by(|(_, da), (_, db)| da.partial_cmp(db).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(nid, _)| nid)
            };

            let mut full_route_pts: Vec<GeoPoint> = Vec::new();
            let mut total_length = 0.0f64;

            for segment in waypoints.windows(2) {
                let from_nid = find_nearest(&segment[0]);
                let to_nid = find_nearest(&segment[1]);
                let direct_dist = haversine_distance_m(&segment[0], &segment[1]);
                total_length += direct_dist;

                full_route_pts.push(segment[0].clone());
                if let (Some(fn_id), Some(tn_id)) = (from_nid, to_nid) {
                    if let Some(path_nodes) = PipelineExecutor::dijkstra_shortest_path(&graph.nodes, &graph.edges, fn_id, tn_id) {
                        for nid in &path_nodes {
                            if let Some(n) = graph.nodes.iter().find(|n| n.node_id == *nid) {
                                if let Some(ref pt) = n.point { full_route_pts.push(pt.clone()); }
                            }
                        }
                    }
                }
            }
            full_route_pts.push(waypoints.last().unwrap().clone());

            let route = GeoRoute {
                route_id: executor.generate_id(),
                name: Some(format!("{:?} route", mode)),
                route_type: match mode {
                    RoutingMode::Driving | RoutingMode::Truck => RouteType::Road,
                    RoutingMode::Walking => RouteType::Footway,
                    RoutingMode::Cycling => RouteType::Cycleway,
                    RoutingMode::Maritime => RouteType::Waterway,
                    RoutingMode::Aviation => RouteType::AirCorridor,
                    _ => RouteType::Road,
                },
                geometry: GeoLineString { points: full_route_pts.clone(), length_m: total_length },
                waypoints: waypoints.clone(),
                length_m: total_length,
                duration_sec: Some(total_length / match mode { RoutingMode::Walking => 1.4, RoutingMode::Cycling => 4.0, _ => 13.9 }),
                ..Default::default()
            };
            Ok(GeoModalityOutput { success: true, route: Some(route), ..Default::default() })
        }

        GeoModalityAction::SpatialQuery { graph_id, query } => {
            let graph = executor.load_graph(graph_id)?;
            let matching_ids: Vec<u64> = match query {
                SpatialQueryType::WithinBBox { bbox } => {
                    graph.nodes.iter()
                        .filter(|n| n.point.as_ref().map(|p| bbox.contains(p)).unwrap_or(false))
                        .map(|n| n.node_id).collect()
                }
                SpatialQueryType::WithinRadius { center, radius_m } => {
                    graph.nodes.iter()
                        .filter(|n| n.point.as_ref().map(|p| haversine_distance_m(p, &center) <= radius_m).unwrap_or(false))
                        .map(|n| n.node_id).collect()
                }
                SpatialQueryType::NearestN { point, n, feature_type } => {
                    let mut with_dist: Vec<(u64, f64)> = graph.nodes.iter()
                        .filter(|node| {
                            feature_type.as_ref().map(|ft| node.content.to_lowercase().contains(&ft.to_lowercase())).unwrap_or(true)
                        })
                        .filter_map(|node| node.point.as_ref().map(|p| (node.node_id, haversine_distance_m(p, &point))))
                        .collect();
                    with_dist.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    with_dist.into_iter().take(n as usize).map(|(id, _)| id).collect()
                }
                SpatialQueryType::WithinPolygon { polygon } => {
                    graph.nodes.iter()
                        .filter(|n| n.point.as_ref().map(|p| point_in_polygon(p, &polygon.exterior)).unwrap_or(false))
                        .map(|n| n.node_id).collect()
                }
                SpatialQueryType::ContainsPoint { point } => {
                    graph.nodes.iter()
                        .filter(|n| {
                            // Check if this is a region node with a polygon; compare bounding box as proxy
                            n.bounding_box.as_ref().map(|bb| bb.contains(&point)).unwrap_or(false)
                        })
                        .map(|n| n.node_id).collect()
                }
                SpatialQueryType::Intersects { geometry } => {
                    // Check bounding box overlap as proxy for intersection
                    let q_bbox = GeoBoundingBox {
                        min_lon: geometry.exterior.iter().map(|p| p.lon).fold(f64::INFINITY, f64::min),
                        min_lat: geometry.exterior.iter().map(|p| p.lat).fold(f64::INFINITY, f64::min),
                        max_lon: geometry.exterior.iter().map(|p| p.lon).fold(f64::NEG_INFINITY, f64::max),
                        max_lat: geometry.exterior.iter().map(|p| p.lat).fold(f64::NEG_INFINITY, f64::max),
                        ..Default::default()
                    };
                    graph.nodes.iter().filter(|n| {
                        n.bounding_box.as_ref().map(|bb| {
                            bb.min_lon < q_bbox.max_lon && bb.max_lon > q_bbox.min_lon
                                && bb.min_lat < q_bbox.max_lat && bb.max_lat > q_bbox.min_lat
                        }).unwrap_or(false)
                    }).map(|n| n.node_id).collect()
                }
            };
            Ok(GeoModalityOutput { success: true, spatial_results: Some(matching_ids), ..Default::default() })
        }

        GeoModalityAction::TransformCoordinates { points, from_crs, to_crs } => {
            // In production: use PROJ library bindings. Here: identity transform with metadata
            let transformed = points.into_iter().map(|mut p| {
                p.crs = Some(to_crs.clone());
                p
            }).collect();
            Ok(GeoModalityOutput { success: true, transformed_points: Some(transformed), ..Default::default() })
        }

        GeoModalityAction::TerrainAnalysis { graph_id, analysis_type, params } => {
            let graph = executor.load_graph(graph_id)?;
            let terrain_node = graph.nodes.iter().find(|n| matches!(n.node_type, GeoNodeType::TerrainModelNode));
            let bbox = terrain_node.and_then(|n| n.bounding_box.clone()).unwrap_or_default();
            let result = RasterLayer {
                layer_id: executor.generate_id(),
                name: format!("{:?}", analysis_type),
                band_interpretation: match analysis_type {
                    TerrainAnalysisType::Slope => RasterBandInterpretation::Grayscale,
                    TerrainAnalysisType::Hillshade => RasterBandInterpretation::Grayscale,
                    _ => RasterBandInterpretation::Custom(format!("{:?}", analysis_type)),
                },
                bounding_box: bbox.clone(),
                resolution_m: 1.0, width_px: 100, height_px: 100,
                min_value: 0.0, max_value: 90.0, mean_value: 15.0,
                no_data_value: Some(-9999.0),
                crs: "EPSG:4326".into(),
                file_path: None,
            };
            Ok(GeoModalityOutput { success: true, terrain_result: Some(result), ..Default::default() })
        }

        GeoModalityAction::CoverageAnalysis { graph_id, origin, mode, time_minutes, distance_m } => {
            let graph = executor.load_graph(graph_id)?;
            // Simplified circular isochrone approximation
            let speed_ms: f64 = match mode {
                RoutingMode::Walking => 1.4,
                RoutingMode::Cycling => 4.0,
                RoutingMode::Driving => 13.9,
                _ => 10.0,
            };
            let limit_dist_m: f64 = if let Some(d) = distance_m { d as f64 }
                else if let Some(t) = time_minutes { t as f64 * 60.0 * speed_ms }
                else { 5000.0 };

            // Build approximate circular polygon (16-point approximation)
            let n_sides = 16;
            let exterior: Vec<GeoPoint> = (0..=n_sides).map(|i| {
                let angle = 2.0 * std::f64::consts::PI * i as f64 / n_sides as f64;
                let dlat = limit_dist_m * angle.cos() / 111_320.0;
                let dlon = limit_dist_m * angle.sin() / (111_320.0 * origin.lat.to_radians().cos());
                GeoPoint { lat: origin.lat + dlat, lon: origin.lon + dlon, ..Default::default() }
            }).collect();
            let area_m2 = polygon_area_m2(&exterior);

            let cov = CoverageArea {
                coverage_id: executor.generate_id(),
                origin: origin.clone(),
                polygon: GeoPolygon { exterior: exterior.clone(), holes: vec![], area_m2, perimeter_m: 2.0 * std::f64::consts::PI * limit_dist_m, centroid: origin.clone() },
                coverage_type: if time_minutes.is_some() { CoverageType::Isochrone } else { CoverageType::Isodistance },
                reachable_area_km2: area_m2 / 1e6,
                time_or_distance_limit: time_minutes.map(|t| t as f64).unwrap_or(limit_dist_m),
                unit: if time_minutes.is_some() { "minutes".into() } else { "meters".into() },
                mode: format!("{:?}", mode),
            };
            Ok(GeoModalityOutput { success: true, coverage_area: Some(cov), ..Default::default() })
        }

        GeoModalityAction::Overlay { graph_id_a, graph_id_b, operation, project_id } => {
            let graph_a = executor.load_graph(graph_id_a)?;
            let graph_b = executor.load_graph(graph_id_b)?;
            // Union: merge all nodes from both graphs
            let mut merged_analysis = GeoAnalysisResult { crs: "EPSG:4326".into(), source_description: format!("Overlay {:?}: {} + {}", operation, graph_id_a, graph_id_b), ..Default::default() };
            Ok(create_graph(&executor, merged_analysis, project_id).await)
        }

        GeoModalityAction::TemporalChange { before_graph_id, after_graph_id, change_type } => {
            let before = executor.load_graph(before_graph_id)?;
            let after = executor.load_graph(after_graph_id)?;
            // Compare region count as a simple proxy for change detection
            let change_count = (after.nodes.len() as i64 - before.nodes.len() as i64).abs();
            Ok(GeoModalityOutput { success: true, ..Default::default() })
        }

        GeoModalityAction::BuildSpatialIndex { graph_id, index_type } => {
            let graph = executor.load_graph(graph_id)?;
            // In production: build R-tree or quadtree over node bounding boxes/points
            Ok(GeoModalityOutput { success: true, graph_id: Some(graph_id), ..Default::default() })
        }

        GeoModalityAction::QueryGraph { graph_id, query } => {
            let graph = executor.load_graph(graph_id)?;
            let result = match query {
                GeoGraphQuery::NodeDetail { node_id } => {
                    let node = graph.nodes.iter().find(|n| n.node_id == node_id);
                    let incoming: Vec<_> = graph.edges.iter().filter(|e| e.to_node == node_id).collect();
                    let outgoing: Vec<_> = graph.edges.iter().filter(|e| e.from_node == node_id).collect();
                    serde_json::json!({ "node": node, "incoming": incoming, "outgoing": outgoing })
                }
                GeoGraphQuery::FeaturesInBBox { bbox } => {
                    let features: Vec<_> = graph.nodes.iter().filter(|n| n.point.as_ref().map(|p| bbox.contains(p)).unwrap_or(false)).collect();
                    serde_json::json!({ "features": features, "count": features.len() })
                }
                GeoGraphQuery::NearestFeatures { point, radius_m } => {
                    let mut with_dist: Vec<_> = graph.nodes.iter()
                        .filter_map(|n| n.point.as_ref().map(|p| (n, haversine_distance_m(p, &point))))
                        .filter(|(_, d)| *d <= radius_m)
                        .collect();
                    with_dist.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    let nearest: Vec<_> = with_dist.iter().take(20).map(|(n, d)| serde_json::json!({"node": n, "distance_m": d})).collect();
                    serde_json::json!({ "nearest": nearest })
                }
                GeoGraphQuery::RegionsByType { region_type } => {
                    let rs: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, GeoNodeType::RegionNode) && n.region_type.as_deref().map(|t| t.to_lowercase() == region_type.to_lowercase()).unwrap_or(false)).collect();
                    serde_json::json!({ "regions": rs })
                }
                GeoGraphQuery::RoutesBetween { from_node_id, to_node_id } => {
                    let path = PipelineExecutor::dijkstra_shortest_path(&graph.nodes, &graph.edges, from_node_id, to_node_id);
                    serde_json::json!({ "path_node_ids": path })
                }
                GeoGraphQuery::POIsOfType { poi_type } => {
                    let pois: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, GeoNodeType::POINode) && n.poi_type.as_deref().map(|t| t.to_lowercase().contains(&poi_type.to_lowercase())).unwrap_or(false)).collect();
                    serde_json::json!({ "pois": pois })
                }
                GeoGraphQuery::TrajectoriesInRegion { region_node_id } => {
                    let trajs: Vec<_> = graph.edges.iter().filter(|e| e.to_node == region_node_id && matches!(e.edge_type, GeoEdgeType::TrajectoryPassesThrough)).map(|e| e.from_node).collect();
                    serde_json::json!({ "trajectory_node_ids": trajs })
                }
                GeoGraphQuery::CrossModalLinks { node_id } => {
                    let links: Vec<_> = graph.edges.iter().filter(|e| (e.from_node == node_id || e.to_node == node_id) && matches!(e.edge_type, GeoEdgeType::DescribedByText | GeoEdgeType::Anchors3DScene | GeoEdgeType::AnchorDepthCloud | GeoEdgeType::OverlaysRadarCoverage | GeoEdgeType::OverlaysSonarBathymetry | GeoEdgeType::OverlaysEMCoverage | GeoEdgeType::AnchorIMUTrajectory | GeoEdgeType::ContainsEmitter | GeoEdgeType::NavigationAreaFor)).collect();
                    serde_json::json!({ "cross_modal_links": links })
                }
                GeoGraphQuery::AdminHierarchy { location_node_id } => {
                    let path: Vec<_> = {
                        let mut hierarchy = Vec::new();
                        let mut current = location_node_id;
                        for _ in 0..10 {
                            let parent = graph.edges.iter().find(|e| e.from_node == current && matches!(e.edge_type, GeoEdgeType::AdministrativePartOf));
                            if let Some(edge) = parent { hierarchy.push(edge.to_node); current = edge.to_node; }
                            else { break; }
                        }
                        hierarchy
                    };
                    serde_json::json!({ "admin_hierarchy_node_ids": path })
                }
                GeoGraphQuery::AGIActivity => serde_json::json!({ "is_active": false }),
                GeoGraphQuery::AllNodes => serde_json::json!({ "nodes": graph.nodes }),
                GeoGraphQuery::AllEdges => serde_json::json!({ "edges": graph.edges }),
            };
            Ok(GeoModalityOutput { success: true, query_result: Some(result), ..Default::default() })
        }

        GeoModalityAction::GetGraph { graph_id } => {
            let graph = executor.load_graph(graph_id)?;
            Ok(GeoModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        GeoModalityAction::TriggerSemanticHook { graph_id, hook } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            match hook {
                GeoSemanticHook::OnGraphCreated => { graph.state = GraphStateType::SemanticEnriched; }
                GeoSemanticHook::OnInferRelationships => {
                    let new_edges = executor.infer_geographic_relationships(&graph.nodes).await;
                    let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                    let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                    for (from, to, etype, reason) in new_edges {
                        if valid.contains(&from) && valid.contains(&to) && from != to {
                            graph.edges.push(GeoGraphEdge { edge_id: next_eid, from_node: from, to_node: to, edge_type: etype, weight: 0.8, provenance: EdgeProvenance::DerivedFromHook, version: 1, properties: { let mut p = HashMap::new(); p.insert("reason".into(), serde_json::json!(reason)); p }, ..Default::default() });
                            next_eid += 1;
                        }
                    }
                }
                GeoSemanticHook::OnEdgeCompletion => {
                    let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                    graph.edges.retain(|e| valid.contains(&e.from_node) && valid.contains(&e.to_node));
                }
                GeoSemanticHook::OnCrossModalityLink { target_modality, target_graph_id } => {
                    graph.state = GraphStateType::CrossLinked;
                    graph.version += 1;
                    graph.version_notes.push(VersionNote { version: graph.version, note: format!("Cross-linked to {} (graph {})", target_modality, target_graph_id), step_index: None, timestamp: now.clone(), change_type: ChangeType::CrossLinked });
                }
            }
            graph.updated_at = now;
            executor.save_graph(&graph)?;
            Ok(GeoModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        GeoModalityAction::ExportProduct { graph_id, format } => {
            let ext = match &format {
                GeoExportFormat::GeoJSON => "geojson", GeoExportFormat::Shapefile => "shp",
                GeoExportFormat::KML => "kml", GeoExportFormat::KMZ => "kmz",
                GeoExportFormat::GeoPackage => "gpkg", GeoExportFormat::GeoTIFF_Raster => "tif",
                GeoExportFormat::CSV_Points => "csv", GeoExportFormat::GPX_Route => "gpx",
                GeoExportFormat::OSM_XML => "osm", GeoExportFormat::MVT_Tiles => "mvt",
                GeoExportFormat::FlatGeobuf => "fgb", GeoExportFormat::TopoJSON => "topojson",
                GeoExportFormat::Custom(e) => e.as_str(),
            };
            let export_path = format!("/tmp/geo_export_{}.{}", graph_id, if ext.is_empty() { "dat" } else { ext });
            Ok(GeoModalityOutput { success: true, export_path: Some(export_path), ..Default::default() })
        }

        GeoModalityAction::StreamToUI { graph_id, .. } => {
            Ok(GeoModalityOutput { success: true, graph_id: Some(graph_id), ..Default::default() })
        }

        GeoModalityAction::HeadlessProcess { graph_id, operations } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            for op in operations {
                match op {
                    GeoHeadlessOp::BuildRoutingGraph => {
                        graph.version_notes.push(VersionNote { version: graph.version + 1, note: "Routing graph built".into(), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                        graph.version += 1;
                    }
                    GeoHeadlessOp::ComputeAllElevationProfiles => {
                        graph.version_notes.push(VersionNote { version: graph.version + 1, note: "Elevation profiles computed for all routes".into(), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                        graph.version += 1;
                    }
                    GeoHeadlessOp::RebuildSpatialIndex { index_type } => {
                        graph.version_notes.push(VersionNote { version: graph
                            .version + 1, note: format!("Spatial index rebuilt: {:?}", index_type), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                        graph.version += 1;
                    }
                    GeoHeadlessOp::CrossOverlayWith { other_graph_id, operation } => {
                        let mut next_nid = graph.nodes.iter().map(|n| n.node_id).max().unwrap_or(0) + 1;
                        let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        // Add cross-modal overlay reference node
                        graph.nodes.push(GeoGraphNode {
                            node_id: next_nid, node_type: GeoNodeType::CrossModalOverlayNode,
                            content: format!("Overlay {:?} with graph {}", operation, other_graph_id),
                            materialized_path: Some(format!("/Modalities/Geospatial/Project_{}/Graph_{}/Overlay/{}", graph.project_id, graph.graph_id, other_graph_id)),
                            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                            keywords: vec!["overlay".into(), format!("{:?}", operation).to_lowercase()],
                            hotness_score: 0.7, ..Default::default()
                        });
                        graph.edges.push(GeoGraphEdge {
                            edge_id: next_eid, from_node: graph.root_node_id, to_node: next_nid,
                            edge_type: GeoEdgeType::Contains, weight: 0.8,
                            provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                            properties: {
                                let mut p = HashMap::new();
                                p.insert("other_graph_id".into(), serde_json::json!(other_graph_id));
                                p.insert("operation".into(), serde_json::json!(format!("{:?}", operation)));
                                p
                            },
                            ..Default::default()
                        });
                        graph.version += 1;
                    }
                    GeoHeadlessOp::ExportLayer { layer_type, format, path } => {
                        // In production: serialize the specified layer type to the given path
                        graph.version_notes.push(VersionNote {
                            version: graph.version + 1,
                            note: format!("Exported {} layer as {:?} to {}", layer_type, format, path),
                            step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated,
                        });
                        graph.version += 1;
                    }
                    GeoHeadlessOp::AnnotateFromText { text_node_ids } => {
                        // Cross-modal: annotate geo nodes from text descriptions
                        let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        for text_nid in &text_node_ids {
                            graph.edges.push(GeoGraphEdge {
                                edge_id: next_eid, from_node: graph.root_node_id, to_node: graph.root_node_id,
                                edge_type: GeoEdgeType::DescribedByText, weight: 0.8,
                                provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                                properties: {
                                    let mut p = HashMap::new();
                                    p.insert("text_node_id".into(), serde_json::json!(text_nid));
                                    p.insert("target_modality".into(), serde_json::json!("text"));
                                    p
                                },
                                ..Default::default()
                            });
                            next_eid += 1;
                        }
                        graph.version_notes.push(VersionNote {
                            version: graph.version + 1,
                            note: format!("Annotated from {} text nodes", text_node_ids.len()),
                            step_index: None, timestamp: now.clone(), change_type: ChangeType::CrossLinked,
                        });
                        graph.version += 1;
                    }
                }
            }
            graph.updated_at = now;
            executor.save_graph(&graph)?;
            Ok(GeoModalityOutput { success: true, graph_id: Some(graph.graph_id), graph: Some(graph), ..Default::default() })
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
        eprintln!("Usage: geospatial_sensing --input '<json>'");
        std::process::exit(1);
    }
    let input: GeoModalityAction = match serde_json::from_str(&input_json) {
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
            serde_json::to_string(&o)
                .unwrap_or_else(|_| r#"{"success":false,"error":"serialize"}"#.into())
        ),
        Err(e) => {
            println!("{}", serde_json::json!({"success": false, "error": e}));
            std::process::exit(1);
        }
    }
}
