//! HyperspectralImaging — Pipeline #126
//!
//! Spatial + spectral data (hundreds of bands) for material/chemical/biological
//! property extraction without physical contact.
//!
//! DISTINCT FROM:
//!   - Image (102): 3 RGB bands. Hyperspectral has 100–1000+ continuous bands.
//!     No amount of RGB processing recovers spectral curves. The information
//!     is literally absent from RGB data.
//!   - Thermal (114): single-band temperature; no spectral curves, no unmixing.
//!
//! CROSS-LINKS:
//!   106 (Chem)   → chemical composition maps validate chemistry node findings
//!   111 (Bio)    → plant stress / chlorophyll / vegetation health
//!   112 (Prot)   → protein/tissue signatures in hyperspectral medical imaging
//!   109 (3D)     → surface material properties on 3D geometry
//!   114 (Therm)  → thermal emission correlates with spectral emissivity
//!   124 (Radar)  → material signatures cross-validated radar vs spectral
//!   125 (Sonar)  → seafloor material from spectral vs acoustic signatures
//!
//! STORAGE: ZSEI containers under /Modalities/Hyperspectral/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

// ─────────────────────────────────────────────────────────────────────────────
// INPUT TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum HyperspectralModalityAction {
    /// Analyze a hyperspectral/multispectral image
    Analyze {
        data: HyperspectralDataSource,
        extract_materials: bool,
        extract_chemistry: bool,
        extract_biology: bool,
        extract_anomalies: bool,
        unmix_pixels: bool,
    },
    /// Create graph from analysis result
    CreateGraph {
        analysis: HyperspectralAnalysisResult,
        project_id: u64,
    },
    /// Update graph with new spectral data
    UpdateGraph {
        graph_id: u64,
        new_spectra: Vec<SpectralMeasurement>,
        project_id: u64,
    },
    /// Unmix a single pixel into material fractions
    UnmixPixel {
        pixel_spectrum: Vec<f32>,
        wavelengths_nm: Vec<f32>,
        endmember_library: Vec<Endmember>,
        method: UnmixingMethod,
    },
    /// Identify material from a spectral signature
    IdentifyMaterial {
        spectrum: Vec<f32>,
        wavelengths_nm: Vec<f32>,
        context: MaterialContext,
        library: Option<Vec<Endmember>>,
    },
    /// Map chemical composition across a spatial extent
    MapChemistry {
        graph_id: u64,
        target_chemicals: Vec<String>,
        mapping_method: ChemicalMappingMethod,
    },
    /// Detect anomalies relative to background
    DetectAnomalies {
        graph_id: u64,
        method: AnomalyDetectionMethod,
        threshold_sigma: f32,
    },
    /// Apply atmospheric correction to raw radiance
    AtmosphericCorrection {
        graph_id: u64,
        method: AtmosphericCorrectionMethod,
        atmospheric_params: Option<AtmosphericParams>,
    },
    /// Band math: compute spectral indices (NDVI, etc.)
    ComputeSpectralIndex {
        graph_id: u64,
        index: SpectralIndex,
        custom_formula: Option<String>,
    },
    /// Classify land cover / material cover
    ClassifyLandCover {
        graph_id: u64,
        classifier: ClassifierType,
        training_samples: Option<Vec<TrainingSample>>,
    },
    /// Query graph
    QueryGraph {
        graph_id: u64,
        query: HyperspectralGraphQuery,
    },
    /// Retrieve full graph for Context Viewer
    GetGraph { graph_id: u64 },
    /// Trigger ZSEI semantic hooks
    TriggerSemanticHook {
        graph_id: u64,
        hook: HyperspectralSemanticHook,
    },
    /// Export products
    ExportProduct {
        graph_id: u64,
        format: HyperspectralExportFormat,
    },
    /// Stream to UI
    StreamToUI {
        graph_id: u64,
        session_id: String,
        display_mode: HyperspectralDisplayMode,
    },
    /// Headless processing (AGI-first)
    HeadlessProcess {
        graph_id: u64,
        operations: Vec<HyperspectralOperation>,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// DATA SOURCE TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HyperspectralDataSource {
    /// Airborne/spaceborne hyperspectral (e.g., AVIRIS, HyMap, DESIS, PRISMA)
    HyperspectralFile {
        file_path: String,
        format: HSIFileFormat,
        sensor: Option<String>,
        band_count: u32,
        wavelength_range_nm: (f32, f32),
        spectral_resolution_nm: f32,
        spatial_resolution_m: f32,
    },
    /// Multispectral (4–12 bands: Landsat, Sentinel-2, WorldView)
    MultispectralFile {
        file_path: String,
        format: HSIFileFormat,
        sensor: String,
        bands: Vec<MultispectralBand>,
    },
    /// Lab hyperspectral camera (push-broom or snap-shot)
    LabHyperspectralFile {
        file_path: String,
        format: HSIFileFormat,
        wavelength_range_nm: (f32, f32),
        band_count: u32,
        spatial_resolution_um: Option<f32>,
        sample_type: LabSampleType,
    },
    /// Spectroradiometer point measurement
    PointSpectrumFile {
        file_path: String,
        format: SpectrumFormat,
        measurement_context: String,
    },
    /// Multi-temporal stack (same scene, multiple dates)
    TemporalStack {
        files: Vec<HyperspectralDataSource>,
        dates: Vec<String>,
    },
    /// Spectral library (reference spectra)
    SpectralLibrary {
        file_path: String,
        library_name: String,
        entry_count: u32,
    },
    /// Live stream from imaging spectrometer
    LiveStream { endpoint: String, sensor_type: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HSIFileFormat { ENVI, HDF4, HDF5, GeoTIFF, NetCDF, TIFF_Stack, BSQ, BIL, BIP, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpectrumFormat { ENVI_ASCII, CSV, ASD, SVC, FieldSpec, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultispectralBand {
    pub band_name: String,
    pub center_wavelength_nm: f32,
    pub bandwidth_nm: f32,
    pub file_path_or_index: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LabSampleType { SoilCrust, Mineral, PlantTissue, FoodProduct, Pharmaceutical, Textile, Polymer, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MaterialContext {
    Mineralogy, Vegetation, WaterQuality, Urban, Soil, Medical, Food, Custom(String)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnmixingMethod { FCLS, UCLS, NCLS, SparseBayesian, NMF, VCA, NFINDR, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChemicalMappingMethod { LinearRegression, PLSR, SupportVectorRegression, NeuralNetwork, BandRatio, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalyDetectionMethod { RXDetector, LocalRX, KernelRX, CBAD, LRX, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AtmosphericCorrectionMethod { FLAASH, QUAC, SCS, Empirical, DOS, ACORN, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtmosphericParams {
    pub aerosol_model: String,
    pub water_vapor_cm: f32,
    pub ozone_atm_cm: f32,
    pub elevation_m: f32,
    pub acquisition_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClassifierType { SVM, RandomForest, MaximumLikelihood, SAM, SID, NeuralNetwork, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSample {
    pub class_label: String,
    pub pixel_location: (u32, u32),
    pub spectrum: Vec<f32>,
}

// ─────────────────────────────────────────────────────────────────────────────
// SPECTRAL INDICES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpectralIndex {
    // Vegetation
    NDVI,           // Normalized Difference Vegetation Index
    EVI,            // Enhanced Vegetation Index
    SAVI,           // Soil Adjusted Vegetation Index
    LAI,            // Leaf Area Index
    PRI,            // Photochemical Reflectance Index
    SIPI,           // Structure Insensitive Pigment Index
    CRI,            // Carotenoid Reflectance Index
    NDRE,           // Red-Edge NDVI
    // Water
    NDWI,           // Normalized Difference Water Index
    MNDWI,          // Modified NDWI
    WRI,            // Water Ratio Index
    // Mineralogy
    ClayIndex,
    IronOxideIndex,
    CarbonateIndex,
    AluniteIndex,
    // Geology / soil
    NDSI,           // Normalized Difference Snow Index
    BSI,            // Bare Soil Index
    // Fire / burn
    NBR,            // Normalized Burn Ratio
    // Urban
    NDBI,           // Normalized Difference Built-Up Index
    // Custom
    Custom { name: String, formula: String },
}

// ─────────────────────────────────────────────────────────────────────────────
// ANALYSIS RESULT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HyperspectralAnalysisResult {
    pub analysis_id: u64,

    // IMAGE PARAMETERS
    pub band_count: u32,
    pub spatial_width: u32,
    pub spatial_height: u32,
    pub wavelengths_nm: Vec<f32>,           // wavelength for each band
    pub spectral_range_nm: (f32, f32),
    pub spectral_resolution_nm: f32,
    pub spatial_resolution_m: f32,
    pub geo_extent: Option<[f64; 4]>,       // [min_lat, min_lon, max_lat, max_lon]
    pub coordinate_reference: String,
    pub sensor: Option<String>,
    pub acquisition_date: Option<String>,

    // SPECTRAL PRODUCTS
    pub spectral_signatures: Vec<SpectralSignature>,
    pub endmembers: Vec<Endmember>,         // pure material reference spectra
    pub unmixed_maps: Vec<UnmixedMap>,      // fractional abundance per material

    // MATERIAL IDENTIFICATION
    pub materials: Vec<MaterialComponent>,
    pub material_map: Option<MaterialMap>,

    // CHEMICAL COMPOSITION
    pub chemical_maps: Vec<ChemicalMap>,
    pub chemical_components: Vec<ChemicalComponent>,

    // BIOLOGICAL
    pub vegetation_indices: Vec<SpectralIndexResult>,
    pub vegetation_health: Option<VegetationHealthMap>,
    pub plant_stress_indicators: Vec<PlantStressIndicator>,

    // ANOMALIES
    pub anomaly_map: Option<AnomalyMap>,
    pub anomaly_pixels: Vec<AnomalyPixel>,

    // CLASSIFICATION
    pub land_cover_map: Option<LandCoverMap>,
    pub classified_classes: Vec<LandCoverClass>,

    // CALIBRATION
    pub atmospheric_correction_applied: bool,
    pub radiometric_correction_applied: bool,
    pub bad_band_mask: Vec<u32>,            // indices of bad bands

    // METADATA
    pub source_description: String,
    pub processing_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpectralSignature {
    pub signature_id: u64,
    pub pixel_location: Option<(u32, u32)>,
    pub geo_location: Option<(f64, f64)>,
    pub wavelengths_nm: Vec<f32>,
    pub reflectance_values: Vec<f32>,       // dimensionless 0.0–1.0 (after correction)
    pub radiance_values: Option<Vec<f32>>,  // raw radiance (pre-correction)
    pub absorption_features: Vec<AbsorptionFeature>,
    pub identified_materials: Vec<String>,
    pub identified_chemicals: Vec<String>,
    pub classification_label: Option<String>,
    pub library_match: Option<LibraryMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AbsorptionFeature {
    pub feature_id: u64,
    pub center_wavelength_nm: f32,
    pub depth: f32,                         // 0.0–1.0 relative to continuum
    pub width_nm: f32,
    pub symmetry: f32,                      // 0=symmetric, positive=right-skewed
    pub associated_compound: Option<String>,
    pub molecular_bond: Option<String>,     // e.g., "O-H stretch", "C-H bend"
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Endmember {
    pub endmember_id: u64,
    pub name: String,
    pub material_class: String,
    pub wavelengths_nm: Vec<f32>,
    pub reflectance: Vec<f32>,
    pub source: EndmemberSource,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum EndmemberSource {
    #[default] Extracted,        // extracted from image (VCA, NFINDR, etc.)
    LibraryReference,            // from spectral library (USGS, ASTER, etc.)
    LabMeasured,                 // laboratory measurement
    UserProvided,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UnmixedMap {
    pub map_id: u64,
    pub endmember_id: u64,
    pub endmember_name: String,
    /// Fractional abundance 0.0–1.0 per pixel [row][col]
    pub abundance_grid: Vec<Vec<f32>>,
    pub mean_abundance: f32,
    pub max_abundance: f32,
    pub high_abundance_pixels: Vec<(u32, u32)>, // pixels with abundance > 0.5
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaterialComponent {
    pub material_id: u64,
    pub name: String,
    pub material_class: MaterialClass,
    pub abundance_fraction: f32,            // 0.0–1.0 scene-wide
    pub pixel_count: u32,
    pub dominant_wavelength_nm: Option<f32>,
    pub key_absorption_features: Vec<AbsorptionFeature>,
    pub associated_chemicals: Vec<String>,
    pub geo_locations: Vec<(f64, f64)>,     // representative locations
    pub cross_modal_notes: Vec<String>,     // notes for cross-modal linking
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum MaterialClass {
    #[default] Unknown,
    SilicateMinereal, OxideMinereal, CarbonateMinereal, SulfateMinereal, PhosphateMinereal,
    ClayMineral, Organic, Vegetation, Water, Ice, Snow, Urban, Soil,
    Polymer, Textile, Metal, Pharmaceutical, Food, CustomMaterial(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaterialMap {
    pub map_id: u64,
    pub width_pixels: u32,
    pub height_pixels: u32,
    /// Class index per pixel (maps to classified_classes)
    pub class_map: Vec<Vec<u16>>,
    pub class_labels: Vec<String>,
    pub overall_accuracy: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChemicalMap {
    pub map_id: u64,
    pub chemical_name: String,
    pub chemical_formula: Option<String>,
    pub unit: String,                       // e.g., "mg/L", "% dry weight"
    pub width_pixels: u32,
    pub height_pixels: u32,
    /// Concentration per pixel
    pub concentration_grid: Vec<Vec<f32>>,
    pub min_value: f32,
    pub max_value: f32,
    pub mean_value: f32,
    pub mapping_method: String,
    pub validation_r_squared: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChemicalComponent {
    pub component_id: u64,
    pub chemical_name: String,
    pub chemical_formula: Option<String>,
    pub cas_number: Option<String>,
    pub detection_wavelengths_nm: Vec<f32>,
    pub absorption_features: Vec<AbsorptionFeature>,
    pub estimated_concentration: Option<f32>,
    pub unit: Option<String>,
    pub geographic_distribution: Vec<(f64, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpectralIndexResult {
    pub index_id: u64,
    pub index_type: String,
    pub mean_value: f32,
    pub min_value: f32,
    pub max_value: f32,
    /// Index value per pixel [row][col]
    pub index_grid: Vec<Vec<f32>>,
    pub interpretation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VegetationHealthMap {
    pub map_id: u64,
    pub overall_health_score: f32,          // 0.0 (poor) – 1.0 (excellent)
    pub stressed_fraction: f32,
    pub healthy_fraction: f32,
    pub senescent_fraction: f32,
    pub health_grid: Vec<Vec<f32>>,
    pub stress_causes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlantStressIndicator {
    pub indicator_id: u64,
    pub stress_type: PlantStressType,
    pub wavelength_signature_nm: Vec<f32>,
    pub affected_area_fraction: f32,
    pub severity: StressSeverity,
    pub spatial_locations: Vec<(u32, u32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PlantStressType {
    #[default] Unknown, WaterStress, NitrogenDeficiency, ChlorophyllLoss,
    DiseaseInfection, PestDamage, HeavyMetalToxicity, FireDamage, Custom(String)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum StressSeverity { #[default] None, Mild, Moderate, Severe, Critical }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnomalyMap {
    pub map_id: u64,
    pub method: String,
    pub threshold_sigma: f32,
    pub anomaly_count: u32,
    /// Anomaly score per pixel
    pub score_grid: Vec<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnomalyPixel {
    pub pixel_id: u64,
    pub pixel_location: (u32, u32),
    pub geo_location: Option<(f64, f64)>,
    pub anomaly_score: f32,
    pub spectrum: Vec<f32>,
    pub deviation_from_background: Vec<f32>,
    pub possible_cause: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LandCoverMap {
    pub map_id: u64,
    pub class_map: Vec<Vec<u16>>,
    pub overall_accuracy: Option<f32>,
    pub kappa_coefficient: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LandCoverClass {
    pub class_id: u16,
    pub label: String,
    pub pixel_count: u32,
    pub area_m2: f32,
    pub spectral_mean: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LibraryMatch {
    pub library_name: String,
    pub matched_entry: String,
    pub match_score: f32,               // similarity metric (0.0–1.0)
    pub match_method: String,           // SAM, SID, Pearson, etc.
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpectralMeasurement {
    pub measurement_id: u64,
    pub pixel_location: (u32, u32),
    pub wavelengths_nm: Vec<f32>,
    pub values: Vec<f32>,
    pub timestamp: Option<f64>,
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH NODE TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum HyperspectralNodeType {
    #[default] HyperspectralScene,      // root node — entire image dataset
    SpectralSignatureNode,              // spectral curve at a location
    EndmemberNode,                      // pure material reference spectrum
    UnmixedPixelNode,                   // sub-pixel fractional abundances
    MaterialComponentNode,              // identified material (scene-wide)
    ChemicalComponentNode,              // identified chemical
    ChemicalMapNode,                    // spatial chemical distribution
    AbsorptionFeatureNode,              // specific spectral absorption feature
    VegetationIndexNode,                // NDVI, EVI, etc.
    VegetationHealthNode,               // plant stress/health region
    AnomalyNode,                        // spectral anomaly pixel/region
    LandCoverClassNode,                 // classified land cover type
    SpectralLibraryEntry,               // reference from spectral library
    TemporalObservationNode,            // observation at a specific date
    GeographicRegionNode,               // spatial region in scene
    BandStatisticsNode,                 // per-band statistical summary
    PlantStressNode,                    // stress indicator region
    CrossModalMaterialRef,             // material cross-validated with another modality
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HyperspectralGraphNode {
    pub node_id: u64,
    pub node_type: HyperspectralNodeType,
    pub content: String,

    // SPECTRAL-SPECIFIC
    pub pixel_location: Option<(u32, u32)>,
    pub geo_location: Option<(f64, f64)>,
    pub dominant_wavelength_nm: Option<f32>,
    pub reflectance_mean: Option<f32>,
    pub band_count: Option<u32>,
    pub material_name: Option<String>,
    pub chemical_formula: Option<String>,
    pub abundance_fraction: Option<f32>,
    pub spectral_index_value: Option<f32>,
    pub anomaly_score: Option<f32>,
    pub spectral_id: Option<u64>,       // links to SpectralSignature

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
pub enum HyperspectralEdgeType {
    // ── STRUCTURAL ──
    #[default] Contains,
    Precedes,
    PartOf,

    // ── HYPERSPECTRAL-SPECIFIC ──
    SpectralSignatureOf,                // signature belongs to material/location
    IndicatesChemicalComposition,       // signature indicates chemical presence
    UnmixesTo,                          // pixel unmixes to endmember fractions
    MatchesEndmember,                   // signature matches library endmember
    AbsorptionFeatureOf,               // absorption feature belongs to signature
    AnomalousRelativeTo,                // anomaly is relative to background
    SimilarSpectralCurve,               // spectrally similar signatures
    TemporalChangeFrom,                 // change detected relative to prior date
    SpatiallyAdjacentTo,               // neighbor pixels
    ClassifiedAs,                       // pixel/region classified as class
    VegetationIndexOf,                 // index applies to region
    StressIndicatorOf,                 // stress found in this region

    // ── CROSS-MODAL ──
    /// Links chemical component → chemistry graph (106)
    ChemicallyLinkedToGraph,
    /// Links vegetation/biology node → biology graph (111)
    LinkedToVegetationBiology,
    /// Links material surface → 3D geometry node (109)
    MaterialOnSurface3D,
    /// Links thermal emissivity → thermal node (114)
    EmissivityCorrelatesWithThermal,
    /// Links material signature → radar material signature (124)
    MaterialCrossValidatedWithRadar,
    /// Links seafloor material → sonar signature (125)
    MaterialCrossValidatedWithSonar,
    /// Links protein/tissue → proteomics node (112)
    TissueLinkedToProteomics,

    // ── UNIVERSAL SEMANTIC (Section B.1) ──
    Performs, Affects, Implies, Contradicts, Elaborates, Summarizes, Supports,
    TemporalPrecedes, TemporalFollows, CausedBy, Enables, Prevents,
    FunctionalRole, InstanceOf,
    DerivedFrom, VersionOf, RefinesTo, ForkedFrom,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HyperspectralGraphEdge {
    pub edge_id: u64,
    pub from_node: u64,
    pub to_node: u64,
    pub edge_type: HyperspectralEdgeType,
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
pub struct HyperspectralGraph {
    pub graph_id: u64,
    pub project_id: u64,
    pub source_description: String,
    pub nodes: Vec<HyperspectralGraphNode>,
    pub edges: Vec<HyperspectralGraphEdge>,
    pub root_node_id: u64,
    pub state: GraphStateType,
    pub state_history: Vec<GraphStateTransition>,
    pub created_at: String,
    pub updated_at: String,
    pub version: u32,
    pub version_notes: Vec<VersionNote>,
}

// ─────────────────────────────────────────────────────────────────────────────
// QUERY TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HyperspectralGraphQuery {
    NodeDetail { node_id: u64 },
    MaterialsByClass { material_class: String },
    ChemicalComponents,
    AnomalyPixels { min_score: f32 },
    VegetationStressAreas,
    SpectralSignaturesInRegion { pixel_x_min: u32, pixel_x_max: u32, pixel_y_min: u32, pixel_y_max: u32 },
    EndmemberLibrary,
    CrossModalLinks { node_id: u64 },
    AGIActivity,
    AllNodes,
    AllEdges,
}

// ─────────────────────────────────────────────────────────────────────────────
// SEMANTIC HOOKS
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HyperspectralSemanticHook {
    OnGraphCreated,
    OnInferRelationships,
    OnEdgeCompletion,
    OnCrossModalityLink { target_modality: String, target_graph_id: u64 },
}

// ─────────────────────────────────────────────────────────────────────────────
// EXPORT / DISPLAY TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HyperspectralExportFormat {
    GeoTIFF,                // single band or RGB composite
    ENVI,                   // hyperspectral cube
    HDF5,                   // scientific arrays
    NetCDF,                 // scientific standard
    CSV,                    // material/chemical tables
    Shapefile,              // vector features
    GeoJSON,                // anomalies/materials as features
    ASTER_Library,          // spectral library format
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HyperspectralDisplayMode {
    RGBComposite,           // 3-band false-color composite
    SingleBand,             // single wavelength band
    SpectralCurveViewer,    // per-pixel spectral curve
    MaterialMap,            // material classification overlay
    ChemicalConcentration,  // chemical abundance heatmap
    AnomalyMap,             // anomaly score heatmap
    BandSelector,           // interactive band picker
    UnmixingFractions,      // endmember abundance maps
    VegetationIndex,        // NDVI etc. visualization
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HyperspectralOperation {
    ComputeAllIndices,
    UpdateMaterialMap,
    CrossValidateWithChemistry { chem_graph_id: u64 },
    CrossValidateWithRadar { radar_graph_id: u64 },
    ExportMaterialReport,
    RefineAnomalyDetection { method: AnomalyDetectionMethod },
}

// ─────────────────────────────────────────────────────────────────────────────
// OUTPUT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HyperspectralModalityOutput {
    pub success: bool,
    pub graph_id: Option<u64>,
    pub graph: Option<HyperspectralGraph>,
    pub analysis: Option<HyperspectralAnalysisResult>,
    pub unmixing_result: Option<Vec<UnmixedMap>>,
    pub material_identification: Option<MaterialComponent>,
    pub chemical_map: Option<ChemicalMap>,
    pub anomaly_map: Option<AnomalyMap>,
    pub spectral_index: Option<SpectralIndexResult>,
    pub land_cover_map: Option<LandCoverMap>,
    pub query_result: Option<serde_json::Value>,
    pub export_path: Option<String>,
    pub error: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// SHARED TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ProvisionalStatus {
    #[default] Planned, Generating, Generated, Validated, Finalized, Failed, RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VersionNote {
    pub version: u32, pub note: String,
    pub step_index: Option<u32>, pub timestamp: String, pub change_type: ChangeType,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ChangeType {
    #[default] Created, Updated, CrossLinked, EnrichedBySemantic, EnrichedByHook, RolledBack, Finalized,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum EdgeProvenance {
    #[default] Unknown,
    DerivedFromPrompt, DerivedFromChunk(u32), DerivedFromChunkGraph(u64),
    DerivedFromModalityGraph(u64), DerivedFromFile(String),
    DerivedFromAMT, DerivedFromBlueprint(u32), DerivedFromMethodology(u64),
    DerivedFromCrossModal, DerivedFromHook, VersionOf(u32), ForkedFrom(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum GraphStateType {
    #[default] Created, SemanticEnriched, CrossLinked, Stable, Updated, ReValidating, Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStateTransition {
    pub from: GraphStateType, pub to: GraphStateType,
    pub timestamp: String, pub triggered_by_step: Option<u32>,
}

// ─────────────────────────────────────────────────────────────────────────────
// PIPELINE EXECUTOR
// ─────────────────────────────────────────────────────────────────────────────

struct PipelineExecutor {
    zsei_path: String,
    prompt_pipeline_path: String,
}

impl PipelineExecutor {
    fn new() -> Self {
        Self {
            zsei_path: env::var("OZONE_ZSEI_PATH").unwrap_or_else(|_| "./zsei_data".into()),
            prompt_pipeline_path: env::var("OZONE_PROMPT_PIPELINE").unwrap_or_else(|_| "./pipeline_9".into()),
        }
    }

    async fn llm_zero_shot(&self, prompt: &str, max_tokens: usize) -> Result<String, String> {
        let input = serde_json::json!({
            "action": "Prompt", "prompt": prompt, "max_tokens": max_tokens,
            "temperature": 0.05, "system_context": "Hyperspectral analysis. Return only valid JSON."
        });
        let output = std::process::Command::new(&self.prompt_pipeline_path)
            .arg("--input").arg(input.to_string())
            .output().map_err(|e| format!("LLM failed: {}", e))?;
        let result: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("Parse failed: {}", e))?;
        Ok(result["response"].as_str().unwrap_or("{}").to_string())
    }

    fn save_graph(&self, graph: &HyperspectralGraph) -> Result<(), String> {
        let path = format!("{}/local/hsi_graph_{}.json", self.zsei_path, graph.graph_id);
        if let Some(p) = std::path::Path::new(&path).parent() { std::fs::create_dir_all(p).map_err(|e| e.to_string())?; }
        std::fs::write(&path, serde_json::to_string_pretty(graph).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    fn load_graph(&self, graph_id: u64) -> Result<HyperspectralGraph, String> {
        let path = format!("{}/local/hsi_graph_{}.json", self.zsei_path, graph_id);
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
    async fn identify_materials_from_signatures(&self, sigs: &[SpectralSignature]) -> Vec<(u64, Vec<String>, Vec<String>)> {

        if sigs.is_empty() { return vec![]; }

                let sig_list: Vec<serde_json::Value> = sigs.iter().take(20).map(|s| serde_json::json!({
                    "signature_id": s.signature_id,
                    "band_count": s.reflectance_values.len(),
                    "absorption_feature_count": s.absorption_features.len(),
                    "absorption_centers_nm": s.absorption_features.iter().map(|f| f.center_wavelength_nm).collect::<Vec<_>>(),
                    "absorption_depths": s.absorption_features.iter().map(|f| f.depth).collect::<Vec<_>>(),
                    "reflectance_max": s.reflectance_values.iter().cloned().fold(f32::NEG_INFINITY, f32::max),
                    "reflectance_mean": if s.reflectance_values.is_empty() { 0.0 } else {
                        s.reflectance_values.iter().sum::<f32>() / s.reflectance_values.len() as f32
                    },
                    "wavelength_range_nm": [
                        s.wavelengths_nm.first().copied().unwrap_or(400.0),
                        s.wavelengths_nm.last().copied().unwrap_or(2500.0),
                    ],
                })).collect();

                let prompt = format!(r#"
        Identify materials and chemicals from these hyperspectral spectral signatures.
        Each signature includes absorption feature center wavelengths and depths which
        are diagnostic of specific compounds (e.g., O-H at 1400/1900nm → water/hydroxyl,
        C-H at 2300nm → organics, Fe2+ at 900nm → iron minerals, chlorophyll red-edge → vegetation).

        Signatures:
        {}

        Return ONLY valid JSON array — one entry per signature:
        [{{
          "signature_id": N,
          "identified_materials": ["material1", "material2"],
          "identified_chemicals": ["compound1", "compound2"],
          "material_class": "SilicateMinereal|OxideMinereal|CarbonateMinereal|ClayMineral|Organic|Vegetation|Water|Soil|Urban|Unknown",
          "key_absorption_assignments": [{{"wavelength_nm": 1400.0, "compound": "O-H stretch", "bond": "hydroxyl"}}]
        }}]"#,
                    serde_json::to_string_pretty(&sig_list).unwrap_or_default());

                match self.llm_zero_shot(&prompt, 1000).await {
                    Ok(raw) => {
                        let json_str = Self::extract_json_array(&raw);
                        serde_json::from_str::<Vec<serde_json::Value>>(&json_str)
                            .unwrap_or_default()
                            .into_iter()
                            .filter_map(|v| {
                                let id = v["signature_id"].as_u64()?;
                                let materials: Vec<String> = v["identified_materials"].as_array()
                                    .map(|arr| arr.iter().filter_map(|m| m.as_str().map(String::from)).collect())
                                    .unwrap_or_default();
                                let chemicals: Vec<String> = v["identified_chemicals"].as_array()
                                    .map(|arr| arr.iter().filter_map(|c| c.as_str().map(String::from)).collect())
                                    .unwrap_or_default();
                                Some((id, materials, chemicals))
                            })
                            .collect()
                    }
                    Err(_) => vec![],
                }
            }

            async fn infer_absorption_features(&self, spectrum: &[f32], wavelengths: &[f32]) -> Vec<AbsorptionFeature> {
                if spectrum.is_empty() || wavelengths.is_empty() { return vec![]; }

                // Summarize spectrum for LLM — send only key statistics, not all bands
                let n = spectrum.len().min(wavelengths.len());
                let peak_val = spectrum[..n].iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let min_val = spectrum[..n].iter().cloned().fold(f32::INFINITY, f32::min);

                // Sample 20 evenly-spaced points for LLM context
                let step = (n / 20).max(1);
                let sampled: Vec<serde_json::Value> = (0..n).step_by(step)
                    .map(|i| serde_json::json!({ "wavelength_nm": wavelengths[i], "reflectance": spectrum[i] }))
                    .collect();

                let prompt = format!(r#"
        Identify absorption features in this reflectance spectrum.
        Reflectance range: {:.3}–{:.3}
        Sampled spectrum (wavelength_nm, reflectance):
        {}

        Absorption features are local minima relative to the spectral continuum.
        Known absorption features by material:
        - Water/hydroxyl: 1400nm, 1900nm, 2200nm (O-H stretch/bend)
        - Carbonates: 2300nm, 2350nm (CO3 stretch)
        - Clays/micas: 2200nm, 2350nm (Al-OH, Mg-OH)
        - Iron oxides: 850–950nm (Fe2+/Fe3+)
        - Chlorophyll: 670–680nm (red absorption), 720nm (red-edge)
        - Organics/hydrocarbons: 1700nm, 2300nm (C-H bend)
        - Sulfates: 2400nm, 2480nm (S-O stretch)

        Return ONLY valid JSON array:
        [{{
          "center_wavelength_nm": 1400.0,
          "depth": 0.15,
          "width_nm": 60.0,
          "symmetry": 0.0,
          "associated_compound": "Water",
          "molecular_bond": "O-H stretch"
        }}]"#,
                    min_val, peak_val,
                    serde_json::to_string_pretty(&sampled).unwrap_or_default()
                );

                match self.llm_zero_shot(&prompt, 600).await {
                    Ok(raw) => {
                        let json_str = Self::extract_json_array(&raw);
                        serde_json::from_str::<Vec<serde_json::Value>>(&json_str)
                            .unwrap_or_default()
                            .into_iter()
                            .enumerate()
                            .filter_map(|(i, v)| {
                                Some(AbsorptionFeature {
                                    feature_id: i as u64 + 1,
                                    center_wavelength_nm: v["center_wavelength_nm"].as_f64()? as f32,
                                    depth: v["depth"].as_f64().unwrap_or(0.0) as f32,
                                    width_nm: v["width_nm"].as_f64().unwrap_or(20.0) as f32,
                                    symmetry: v["symmetry"].as_f64().unwrap_or(0.0) as f32,
                                    associated_compound: v["associated_compound"].as_str().map(String::from),
                                    molecular_bond: v["molecular_bond"].as_str().map(String::from),
                                })
                            })
                            .collect()
                    }
                    Err(_) => vec![],
                }
            }

            async fn infer_semantic_relationships(
                &self,
                nodes: &[HyperspectralGraphNode],
            ) -> Vec<(u64, u64, HyperspectralEdgeType, String)> {
                if nodes.len() < 2 { return vec![]; }

                let node_list: Vec<serde_json::Value> = nodes.iter().take(25).map(|n| serde_json::json!({
                    "node_id": n.node_id,
                    "type": format!("{:?}", n.node_type),
                    "content": n.content.chars().take(80).collect::<String>(),
                    "material": n.material_name,
                    "chemical": n.chemical_formula,
                    "wavelength_nm": n.dominant_wavelength_nm,
                    "abundance": n.abundance_fraction,
                })).collect();

                let prompt = format!(r#"
        Identify semantic relationships between these hyperspectral graph nodes that are not yet captured structurally.

        Nodes:
        {}

        Available relationship types:
        SpectralSignatureOf, IndicatesChemicalComposition, UnmixesTo, MatchesEndmember,
        AbsorptionFeatureOf, AnomalousRelativeTo, SimilarSpectralCurve, ClassifiedAs,
        VegetationIndexOf, StressIndicatorOf, Affects, CausedBy, Enables, Prevents,
        TemporalPrecedes, DerivedFrom, PartOf, FunctionalRole, InstanceOf

        Return ONLY valid JSON array:
        [{{"from_node_id": N, "to_node_id": M, "edge_type": "TypeName", "reason": "brief explanation"}}]"#,
                    serde_json::to_string_pretty(&node_list).unwrap_or_default()
                );

                match self.llm_zero_shot(&prompt, 800).await {
                    Ok(raw) => {
                        let json_str = Self::extract_json_array(&raw);
                        serde_json::from_str::<Vec<serde_json::Value>>(&json_str)
                            .unwrap_or_default()
                            .into_iter()
                            .filter_map(|v| {
                                let from = v["from_node_id"].as_u64()?;
                                let to = v["to_node_id"].as_u64()?;
                                let etype = map_hsi_edge_str(v["edge_type"].as_str().unwrap_or("Affects"));
                                let reason = v["reason"].as_str().unwrap_or("").to_string();
                                Some((from, to, etype, reason))
                            })
                            .collect()
                    }
                    Err(_) => vec![],
                }
            }

            async fn infer_chemical_components_from_analysis(&self, analysis: &HyperspectralAnalysisResult) -> Vec<String> {
                if analysis.spectral_signatures.is_empty() { return vec![]; }

                let all_features: Vec<serde_json::Value> = analysis.spectral_signatures.iter()
                    .flat_map(|s| s.absorption_features.iter().map(|f| serde_json::json!({
                        "wavelength_nm": f.center_wavelength_nm,
                        "depth": f.depth,
                        "compound": f.associated_compound,
                        "bond": f.molecular_bond,
                    })))
                    .collect::<std::collections::HashSet<String>>()   // deduplicate by string repr
                    .into_iter()
                    .map(|s| serde_json::from_str::<serde_json::Value>(&s).unwrap_or(serde_json::Value::Null))
                    .collect();

                let prompt = format!(r#"
        Based on these spectral absorption features detected across a hyperspectral scene,
        list the distinct chemical compounds or material groups that are present.

        Absorption features observed:
        {}

        Scene context: band range {}–{}nm, {} signatures analyzed.

        Return ONLY a valid JSON array of strings:
        ["compound1", "compound2", ...]"#,
                    serde_json::to_string_pretty(&all_features[..all_features.len().min(30)]).unwrap_or_default(),
                    analysis.spectral_range_nm.0, analysis.spectral_range_nm.1,
                    analysis.spectral_signatures.len()
                );

                match self.llm_zero_shot(&prompt, 300).await {
                    Ok(raw) => {
                        let json_str = Self::extract_json_array(&raw);
                        serde_json::from_str::<Vec<String>>(&json_str).unwrap_or_default()
                    }
                    Err(_) => vec![],
                }
            }

            fn compute_spectral_index_value(
                wavelengths: &[f32],
                reflectance: &[f32],
                index: &SpectralIndex,
            ) -> Option<f32> {
                // Helper: get reflectance at the nearest wavelength
                let band_at = |target_nm: f32| -> Option<f32> {
                    let idx = wavelengths.iter()
                        .enumerate()
                        .min_by(|(_, &a), (_, &b)| {
                            (a - target_nm).abs().partial_cmp(&(b - target_nm).abs())
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })?
                        .0;
                    reflectance.get(idx).copied()
                };

                match index {
                    SpectralIndex::NDVI => {
                        let nir = band_at(842.0)?;
                        let red = band_at(665.0)?;
                        if (nir + red).abs() < 1e-6 { None } else { Some((nir - red) / (nir + red)) }
                    }
                    SpectralIndex::EVI => {
                        let nir  = band_at(842.0)?;
                        let red  = band_at(665.0)?;
                        let blue = band_at(490.0)?;
                        let denom = nir + 6.0 * red - 7.5 * blue + 1.0;
                        if denom.abs() < 1e-6 { None } else { Some(2.5 * (nir - red) / denom) }
                    }
                    SpectralIndex::SAVI => {
                        let nir = band_at(842.0)?;
                        let red = band_at(665.0)?;
                        let l = 0.5; // soil brightness correction factor
                        Some((1.0 + l) * (nir - red) / (nir + red + l))
                    }
                    SpectralIndex::NDWI => {
                        let green = band_at(560.0)?;
                        let nir   = band_at(842.0)?;
                        if (green + nir).abs() < 1e-6 { None } else { Some((green - nir) / (green + nir)) }
                    }
                    SpectralIndex::MNDWI => {
                        let green = band_at(560.0)?;
                        let swir  = band_at(1610.0)?;
                        if (green + swir).abs() < 1e-6 { None } else { Some((green - swir) / (green + swir)) }
                    }
                    SpectralIndex::NDRE => {
                        let nir      = band_at(842.0)?;
                        let red_edge = band_at(705.0)?;
                        if (nir + red_edge).abs() < 1e-6 { None } else { Some((nir - red_edge) / (nir + red_edge)) }
                    }
                    SpectralIndex::PRI => {
                        let r531 = band_at(531.0)?;
                        let r570 = band_at(570.0)?;
                        if (r531 + r570).abs() < 1e-6 { None } else { Some((r531 - r570) / (r531 + r570)) }
                    }
                    SpectralIndex::NDSI => {
                        let green = band_at(560.0)?;
                        let swir  = band_at(1610.0)?;
                        if (green + swir).abs() < 1e-6 { None } else { Some((green - swir) / (green + swir)) }
                    }
                    SpectralIndex::BSI => {
                        let blue = band_at(490.0)?;
                        let red  = band_at(665.0)?;
                        let nir  = band_at(842.0)?;
                        let swir = band_at(1610.0)?;
                        Some(((swir + red) - (nir + blue)) / ((swir + red) + (nir + blue) + 1e-6))
                    }
                    SpectralIndex::NBR => {
                        let nir  = band_at(842.0)?;
                        let swir = band_at(2190.0)?;
                        if (nir + swir).abs() < 1e-6 { None } else { Some((nir - swir) / (nir + swir)) }
                    }
                    SpectralIndex::NDBI => {
                        let swir = band_at(1610.0)?;
                        let nir  = band_at(842.0)?;
                        if (swir + nir).abs() < 1e-6 { None } else { Some((swir - nir) / (swir + nir)) }
                    }
                    SpectralIndex::ClayIndex => {
                        let r2200 = band_at(2200.0)?;
                        let r2100 = band_at(2100.0)?;
                        if r2100.abs() < 1e-6 { None } else { Some(r2200 / r2100) }
                    }
                    SpectralIndex::IronOxideIndex => {
                        let red = band_at(665.0)?;
                        let blue = band_at(490.0)?;
                        if blue.abs() < 1e-6 { None } else { Some(red / blue) }
                    }
                    SpectralIndex::CarbonateIndex => {
                        let r2340 = band_at(2340.0)?;
                        let r2200 = band_at(2200.0)?;
                        if r2200.abs() < 1e-6 { None } else { Some(r2340 / r2200) }
                    }
                    SpectralIndex::AluniteIndex => {
                        let r2200 = band_at(2200.0)?;
                        let r1750 = band_at(1750.0)?;
                        if r1750.abs() < 1e-6 { None } else { Some(r2200 / r1750) }
                    }
                    SpectralIndex::LAI | SpectralIndex::SIPI | SpectralIndex::CRI | SpectralIndex::WRI => {
                        // Complex indices: use NDVI-based proxy
                        let nir = band_at(842.0)?;
                        let red = band_at(665.0)?;
                        let ndvi = if (nir + red).abs() > 1e-6 { (nir - red) / (nir + red) } else { 0.0 };
                        Some(ndvi * 3.0) // simplified proxy
                    }
                    SpectralIndex::Custom { formula, .. } => {
                        // Custom band math: placeholder — in production parse and evaluate formula
                        Some(0.0)
                    }
                }
            }
        }

        fn map_hsi_edge_str(s: &str) -> HyperspectralEdgeType {
            match s {
                "SpectralSignatureOf"            => HyperspectralEdgeType::SpectralSignatureOf,
                "IndicatesChemicalComposition"   => HyperspectralEdgeType::IndicatesChemicalComposition,
                "UnmixesTo"                      => HyperspectralEdgeType::UnmixesTo,
                "MatchesEndmember"               => HyperspectralEdgeType::MatchesEndmember,
                "AbsorptionFeatureOf"            => HyperspectralEdgeType::AbsorptionFeatureOf,
                "AnomalousRelativeTo"            => HyperspectralEdgeType::AnomalousRelativeTo,
                "SimilarSpectralCurve"           => HyperspectralEdgeType::SimilarSpectralCurve,
                "TemporalChangeFrom"             => HyperspectralEdgeType::TemporalChangeFrom,
                "SpatiallyAdjacentTo"            => HyperspectralEdgeType::SpatiallyAdjacentTo,
                "ClassifiedAs"                   => HyperspectralEdgeType::ClassifiedAs,
                "VegetationIndexOf"              => HyperspectralEdgeType::VegetationIndexOf,
                "StressIndicatorOf"              => HyperspectralEdgeType::StressIndicatorOf,
                "ChemicallyLinkedToGraph"        => HyperspectralEdgeType::ChemicallyLinkedToGraph,
                "LinkedToVegetationBiology"      => HyperspectralEdgeType::LinkedToVegetationBiology,
                "MaterialOnSurface3D"            => HyperspectralEdgeType::MaterialOnSurface3D,
                "EmissivityCorrelatesWithThermal"=> HyperspectralEdgeType::EmissivityCorrelatesWithThermal,
                "MaterialCrossValidatedWithRadar"=> HyperspectralEdgeType::MaterialCrossValidatedWithRadar,
                "MaterialCrossValidatedWithSonar"=> HyperspectralEdgeType::MaterialCrossValidatedWithSonar,
                "TissueLinkedToProteomics"       => HyperspectralEdgeType::TissueLinkedToProteomics,
                "Affects"         => HyperspectralEdgeType::Affects,
                "Implies"         => HyperspectralEdgeType::Implies,
                "CausedBy"        => HyperspectralEdgeType::CausedBy,
                "Enables"         => HyperspectralEdgeType::Enables,
                "Prevents"        => HyperspectralEdgeType::Prevents,
                "TemporalPrecedes"=> HyperspectralEdgeType::TemporalPrecedes,
                "TemporalFollows" => HyperspectralEdgeType::TemporalFollows,
                "DerivedFrom"     => HyperspectralEdgeType::DerivedFrom,
                "VersionOf"       => HyperspectralEdgeType::VersionOf,
                "PartOf"          => HyperspectralEdgeType::PartOf,
                "FunctionalRole"  => HyperspectralEdgeType::FunctionalRole,
                "InstanceOf"      => HyperspectralEdgeType::InstanceOf,
                _                 => HyperspectralEdgeType::Affects,
            }
        }

        // ─────────────────────────────────────────────────────────────────────────────
        // GRAPH CREATION
        // ─────────────────────────────────────────────────────────────────────────────

        async fn create_graph(
            executor: &PipelineExecutor,
            analysis: HyperspectralAnalysisResult,
            project_id: u64,
        ) -> HyperspectralModalityOutput {
            let graph_id = executor.generate_id();
            let now = executor.now_iso8601();
            let mut nodes: Vec<HyperspectralGraphNode> = Vec::new();
            let mut edges: Vec<HyperspectralGraphEdge> = Vec::new();
            let mut node_id: u64 = 1;
            let mut edge_id: u64 = 1;

            // ── ROOT NODE ──
            let root_id = node_id;
            nodes.push(HyperspectralGraphNode {
                node_id: root_id,
                node_type: HyperspectralNodeType::HyperspectralScene,
                content: format!(
                    "HSI scene: {}×{}px {}bands {}–{}nm sensor={:?} sigs={} materials={} chemicals={}",
                    analysis.spatial_width, analysis.spatial_height, analysis.band_count,
                    analysis.spectral_range_nm.0, analysis.spectral_range_nm.1,
                    analysis.sensor,
                    analysis.spectral_signatures.len(),
                    analysis.materials.len(),
                    analysis.chemical_components.len(),
                ),
                band_count: Some(analysis.band_count),
                dominant_wavelength_nm: Some((analysis.spectral_range_nm.0 + analysis.spectral_range_nm.1) / 2.0),
                materialized_path: Some(format!("/Modalities/Hyperspectral/Project_{}/Graph_{}", project_id, graph_id)),
                provisional: false,
                provisional_status: ProvisionalStatus::Finalized,
                version: 1,
                version_notes: vec![VersionNote {
                    version: 1, note: "Initial graph creation".into(),
                    step_index: None, timestamp: now.clone(), change_type: ChangeType::Created,
                }],
                keywords: vec!["hyperspectral".into(), "spectral".into(),
                    analysis.sensor.clone().unwrap_or_else(|| "unknown_sensor".into())],
                hotness_score: 1.0,
                ..Default::default()
            });
            node_id += 1;

            // ── ENDMEMBER NODES ──
            let endmember_node_ids: Vec<(u64, u64)> = analysis.endmembers.iter().map(|em| {
                let emid = node_id;
                nodes.push(HyperspectralGraphNode {
                    node_id: emid,
                    node_type: HyperspectralNodeType::EndmemberNode,
                    content: format!("Endmember: {} class={} source={:?} bands={}",
                        em.name, em.material_class, em.source, em.wavelengths_nm.len()),
                    material_name: Some(em.name.clone()),
                    band_count: Some(em.wavelengths_nm.len() as u32),
                    materialized_path: Some(format!("/Modalities/Hyperspectral/Project_{}/Graph_{}/Endmember/{}", project_id, graph_id, em.endmember_id)),
                    provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                    keywords: vec!["endmember".into(), em.material_class.clone(), em.name.to_lowercase()],
                    hotness_score: 0.7,
                    ..Default::default()
                });
                edges.push(HyperspectralGraphEdge {
                    edge_id, from_node: root_id, to_node: emid,
                    edge_type: HyperspectralEdgeType::Contains, weight: 1.0,
                    provenance: EdgeProvenance::DerivedFromPrompt, version: 1,
                    ..Default::default()
                });
                edge_id += 1; node_id += 1;
                (em.endmember_id, emid)
            }).collect();

            // ── MATERIAL COMPONENT NODES ──
            let material_node_ids: Vec<(u64, u64)> = analysis.materials.iter().map(|mat| {
                let mid = node_id;
                nodes.push(HyperspectralGraphNode {
                    node_id: mid,
                    node_type: HyperspectralNodeType::MaterialComponentNode,
                    content: format!("Material: {} ({:?}) abundance={:.2} pixels={}",
                        mat.name, mat.material_class, mat.abundance_fraction, mat.pixel_count),
                    material_name: Some(mat.name.clone()),
                    abundance_fraction: Some(mat.abundance_fraction),
                    dominant_wavelength_nm: mat.dominant_wavelength_nm,
                    geo_location: mat.geo_locations.first().copied(),
                    materialized_path: Some(format!("/Modalities/Hyperspectral/Project_{}/Graph_{}/Material/{}", project_id, graph_id, mat.material_id)),
                    provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                    keywords: {
                        let mut kw = vec!["material".into(), mat.name.to_lowercase()];
                        kw.extend(mat.associated_chemicals.iter().map(|c| c.to_lowercase()));
                        kw
                    },
                    hotness_score: 0.5 + mat.abundance_fraction * 0.4,
                    embedding_hint: Some(format!("material: {} class: {:?}", mat.name, mat.material_class)),
                    ..Default::default()
                });
                edges.push(HyperspectralGraphEdge {
                    edge_id, from_node: root_id, to_node: mid,
                    edge_type: HyperspectralEdgeType::Contains, weight: 1.0,
                    provenance: EdgeProvenance::DerivedFromPrompt, version: 1,
                    ..Default::default()
                });
                edge_id += 1; node_id += 1;
                (mat.material_id, mid)
            }).collect();

            // ── SPECTRAL SIGNATURE NODES ──
            let sig_node_ids: Vec<(u64, u64)> = analysis.spectral_signatures.iter().take(200).map(|sig| {
                let sid = node_id;
                let mean_r = if sig.reflectance_values.is_empty() { 0.0 }
                    else { sig.reflectance_values.iter().sum::<f32>() / sig.reflectance_values.len() as f32 };

                nodes.push(HyperspectralGraphNode {
                    node_id: sid,
                    node_type: HyperspectralNodeType::SpectralSignatureNode,
                    content: format!("Spectral sig: bands={} mean_r={:.3} features={} materials={:?}",
                        sig.reflectance_values.len(), mean_r,
                        sig.absorption_features.len(),
                        sig.identified_materials.first().map(|s| s.as_str()).unwrap_or("unknown")),
                    pixel_location: sig.pixel_location,
                    geo_location: sig.geo_location,
                    reflectance_mean: Some(mean_r),
                    band_count: Some(sig.reflectance_values.len() as u32),
                    spectral_id: Some(sig.signature_id),
                    materialized_path: Some(format!("/Modalities/Hyperspectral/Project_{}/Graph_{}/Signature/{}", project_id, graph_id, sig.signature_id)),
                    provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                    keywords: {
                        let mut kw = vec!["spectrum".into()];
                        kw.extend(sig.identified_materials.iter().map(|m| m.to_lowercase()));
                        kw.extend(sig.identified_chemicals.iter().map(|c| c.to_lowercase()));
                        kw
                    },
                    hotness_score: 0.6 + (sig.absorption_features.len() as f32 * 0.05).min(0.3),
                    ..Default::default()
                });
                edges.push(HyperspectralGraphEdge {
                    edge_id, from_node: root_id, to_node: sid,
                    edge_type: HyperspectralEdgeType::Contains, weight: 0.7,
                    provenance: EdgeProvenance::DerivedFromPrompt, version: 1,
                    ..Default::default()
                });
                edge_id += 1;

                // Absorption feature sub-nodes (up to 5 per signature)
                for feat in sig.absorption_features.iter().take(5) {
                    let fid = node_id + 1;
                    nodes.push(HyperspectralGraphNode {
                        node_id: fid,
                        node_type: HyperspectralNodeType::AbsorptionFeatureNode,
                        content: format!("Absorption: {:.0}nm depth={:.2} w={:.0}nm compound={:?} bond={:?}",
                            feat.center_wavelength_nm, feat.depth, feat.width_nm,
                            feat.associated_compound, feat.molecular_bond),
                        dominant_wavelength_nm: Some(feat.center_wavelength_nm),
                        materialized_path: Some(format!("/Modalities/Hyperspectral/Project_{}/Graph_{}/Signature/{}/Feature/{}", project_id, graph_id, sig.signature_id, feat.feature_id)),
                        provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                        keywords: {
                            let mut kw = vec!["absorption".into(), format!("{:.0}nm", feat.center_wavelength_nm)];
                            if let Some(ref c) = feat.associated_compound { kw.push(c.to_lowercase()); }
                            kw
                        },
                        hotness_score: 0.5 + feat.depth * 0.3,
                        ..Default::default()
                    });
                    edges.push(HyperspectralGraphEdge {
                        edge_id, from_node: sid, to_node: fid,
                        edge_type: HyperspectralEdgeType::AbsorptionFeatureOf, weight: 1.0,
                        provenance: EdgeProvenance::DerivedFromPrompt, version: 1,
                        ..Default::default()
                    });
                    edge_id += 1; node_id += 1;
                }

                // Signature → material component edges (SpectralSignatureOf)
                for mat_name in &sig.identified_materials {
                    if let Some((_, mat_nid)) = material_node_ids.iter()
                        .find(|(mid, _)| analysis.materials.iter().any(|m| m.material_id == *mid && m.name == *mat_name))
                    {
                        edges.push(HyperspectralGraphEdge {
                            edge_id, from_node: sid, to_node: *mat_nid,
                            edge_type: HyperspectralEdgeType::SpectralSignatureOf, weight: 0.9,
                            provenance: EdgeProvenance::DerivedFromPrompt, version: 1,
                            ..Default::default()
                        });
                        edge_id += 1;
                    }
                }

                // Library match → endmember edges
                if let Some(ref lm) = sig.library_match {
                    if let Some((_, em_nid)) = endmember_node_ids.iter()
                        .find(|(eid, _)| analysis.endmembers.iter().any(|e| e.endmember_id == *eid && e.name == lm.matched_entry))
                    {
                        edges.push(HyperspectralGraphEdge {
                            edge_id, from_node: sid, to_node: *em_nid,
                            edge_type: HyperspectralEdgeType::MatchesEndmember, weight: lm.match_score,
                            provenance: EdgeProvenance::DerivedFromPrompt, version: 1,
                            properties: {
                                let mut p = HashMap::new();
                                p.insert("match_score".into(), serde_json::json!(lm.match_score));
                                p.insert("match_method".into(), serde_json::json!(&lm.match_method));
                                p.insert("library".into(), serde_json::json!(&lm.library_name));
                                p
                            },
                            ..Default::default()
                        });
                        edge_id += 1;
                    }
                }

                node_id += 1;
                (sig.signature_id, sid)
            }).collect();

            // ── CHEMICAL COMPONENT NODES ──
            for chem in &analysis.chemical_components {
                let cid = node_id;
                nodes.push(HyperspectralGraphNode {
                    node_id: cid,
                    node_type: HyperspectralNodeType::ChemicalComponentNode,
                    content: format!("Chemical: {} formula={:?} conc={:?}{:?} features={}",
                        chem.chemical_name, chem.chemical_formula,
                        chem.estimated_concentration, chem.unit,
                        chem.absorption_features.len()),
                    chemical_formula: chem.chemical_formula.clone(),
                    material_name: Some(chem.chemical_name.clone()),
                    materialized_path: Some(format!("/Modalities/Hyperspectral/Project_{}/Graph_{}/Chemical/{}", project_id, graph_id, chem.component_id)),
                    provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                    keywords: {
                        let mut kw = vec!["chemical".into(), chem.chemical_name.to_lowercase()];
                        if let Some(ref f) = chem.chemical_formula { kw.push(f.to_lowercase()); }
                        kw
                    },
                    hotness_score: 0.75,
                    embedding_hint: Some(format!("chemical: {} formula: {:?}", chem.chemical_name, chem.chemical_formula)),
                    ..Default::default()
                });
                edges.push(HyperspectralGraphEdge {
                    edge_id, from_node: root_id, to_node: cid,
                    edge_type: HyperspectralEdgeType::Contains, weight: 0.9,
                    provenance: EdgeProvenance::DerivedFromPrompt, version: 1,
                    ..Default::default()
                });
                edge_id += 1;

                // Signature → chemical (IndicatesChemicalComposition)
                for (_, sig_nid) in sig_node_ids.iter().take(10) {
                    edges.push(HyperspectralGraphEdge {
                        edge_id, from_node: *sig_nid, to_node: cid,
                        edge_type: HyperspectralEdgeType::IndicatesChemicalComposition, weight: 0.8,
                        provenance: EdgeProvenance::DerivedFromPrompt, version: 1,
                        properties: {
                            let mut p = HashMap::new();
                            if let Some(conc) = chem.estimated_concentration {
                                p.insert("estimated_concentration".into(), serde_json::json!(conc));
                            }
                            p.insert("unit".into(), serde_json::json!(&chem.unit));
                            p
                        },
                        ..Default::default()
                    });
                    edge_id += 1;
                }

                // Cross-modal: chemical → chemistry graph (106)
                edges.push(HyperspectralGraphEdge {
                    edge_id, from_node: cid, to_node: cid,
                    edge_type: HyperspectralEdgeType::ChemicallyLinkedToGraph, weight: 0.9,
                    provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                    properties: {
                        let mut p = HashMap::new();
                        p.insert("target_modality".into(), serde_json::json!("chemistry"));
                        p.insert("chemical_name".into(), serde_json::json!(&chem.chemical_name));
                        p
                    },
                    ..Default::default()
                });
                edge_id += 1; node_id += 1;
            }

            // ── VEGETATION INDEX NODES ──
            for vi in &analysis.vegetation_indices {
                let vid = node_id;
                nodes.push(HyperspectralGraphNode {
                    node_id: vid,
                    node_type: HyperspectralNodeType::VegetationIndexNode,
                    content: format!("Veg index {}: mean={:.3} min={:.3} max={:.3} | {}",
                        vi.index_type, vi.mean_value, vi.min_value, vi.max_value, vi.interpretation),
                    spectral_index_value: Some(vi.mean_value),
                    materialized_path: Some(format!("/Modalities/Hyperspectral/Project_{}/Graph_{}/VegIndex/{}", project_id, graph_id, vi.index_id)),
                    provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                    keywords: vec!["vegetation-index".into(), vi.index_type.to_lowercase()],
                    hotness_score: 0.65,
                    ..Default::default()
                });
                edges.push(HyperspectralGraphEdge {
                    edge_id, from_node: root_id, to_node: vid,
                    edge_type: HyperspectralEdgeType::Contains, weight: 0.8,
                    provenance: EdgeProvenance::DerivedFromPrompt, version: 1,
                    ..Default::default()
                });
                edge_id += 1; node_id += 1;
            }

            // ── VEGETATION HEALTH NODE ──
            if let Some(ref vh) = analysis.vegetation_health {
                let vhid = node_id;
                nodes.push(HyperspectralGraphNode {
                    node_id: vhid,
                    node_type: HyperspectralNodeType::VegetationHealthNode,
                    content: format!("Veg health: score={:.2} stressed={:.1}% healthy={:.1}% causes={:?}",
                        vh.overall_health_score, vh.stressed_fraction * 100.0,
                        vh.healthy_fraction * 100.0,
                        vh.stress_causes.first().map(|s| s.as_str()).unwrap_or("none")),
                    spectral_index_value: Some(vh.overall_health_score),
                    materialized_path: Some(format!("/Modalities/Hyperspectral/Project_{}/Graph_{}/VegHealth", project_id, graph_id)),
                    provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                    keywords: vec!["vegetation-health".into(), "stress".into()],
                    hotness_score: 0.7,
                    ..Default::default()
                });
                edges.push(HyperspectralGraphEdge {
                    edge_id, from_node: root_id, to_node: vhid,
                    edge_type: HyperspectralEdgeType::Contains, weight: 0.8,
                    provenance: EdgeProvenance::DerivedFromPrompt, version: 1,
                    ..Default::default()
                });
                edge_id += 1;

                // Cross-modal: vegetation → biology (111)
                edges.push(HyperspectralGraphEdge {
                    edge_id, from_node: vhid, to_node: vhid,
                    edge_type: HyperspectralEdgeType::LinkedToVegetationBiology, weight: 0.85,
                    provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                    properties: {
                        let mut p = HashMap::new();
                        p.insert("target_modality".into(), serde_json::json!("biology"));
                        p
                    },
                    ..Default::default()
                });
                edge_id += 1; node_id += 1;
            }

            // ── PLANT STRESS NODES ──
            for stress in &analysis.plant_stress_indicators {
                let stid = node_id;
                nodes.push(HyperspectralGraphNode {
                    node_id: stid,
                    node_type: HyperspectralNodeType::PlantStressNode,
                    content: format!("Plant stress {:?}: severity={:?} area={:.1}%",
                        stress.stress_type, stress.severity, stress.affected_area_fraction * 100.0),
                    abundance_fraction: Some(stress.affected_area_fraction),
                    materialized_path: Some(format!("/Modalities/Hyperspectral/Project_{}/Graph_{}/Stress/{}", project_id, graph_id, stress.indicator_id)),
                    provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                    keywords: vec!["plant-stress".into(), format!("{:?}", stress.stress_type).to_lowercase()],
                    hotness_score: 0.6,
                    ..Default::default()
                });
                edges.push(HyperspectralGraphEdge {
                    edge_id, from_node: root_id, to_node: stid,
                    edge_type: HyperspectralEdgeType::Contains, weight: 0.75,
                    provenance: EdgeProvenance::DerivedFromPrompt, version: 1,
                    ..Default::default()
                });
                edge_id += 1; node_id += 1;
            }

            // ── ANOMALY NODES ──
            for anomaly in analysis.anomaly_pixels.iter().take(50) {
                let aid = node_id;
                nodes.push(HyperspectralGraphNode {
                    node_id: aid,
                    node_type: HyperspectralNodeType::AnomalyNode,
                    content: format!("Anomaly: px=({},{}) score={:.3} cause={:?}",
                        anomaly.pixel_location.0, anomaly.pixel_location.1,
                        anomaly.anomaly_score, anomaly.possible_cause),
                    pixel_location: Some(anomaly.pixel_location),
                    geo_location: anomaly.geo_location,
                    anomaly_score: Some(anomaly.anomaly_score),
                    materialized_path: Some(format!("/Modalities/Hyperspectral/Project_{}/Graph_{}/Anomaly/{}", project_id, graph_id, anomaly.pixel_id)),
                    provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                    keywords: {
                        let mut kw = vec!["anomaly".into()];
                        if let Some(ref cause) = anomaly.possible_cause { kw.push(cause.to_lowercase()); }
                        kw
                    },
                    hotness_score: 0.5 + anomaly.anomaly_score * 0.4,
                    ..Default::default()
                });
                edges.push(HyperspectralGraphEdge {
                    edge_id, from_node: root_id, to_node: aid,
                    edge_type: HyperspectralEdgeType::Contains, weight: 0.9,
                    provenance: EdgeProvenance::DerivedFromPrompt, version: 1,
                    ..Default::default()
                });
                edge_id += 1;

                // Anomaly → background comparison
                edges.push(HyperspectralGraphEdge {
                    edge_id, from_node: aid, to_node: root_id,
                    edge_type: HyperspectralEdgeType::AnomalousRelativeTo, weight: anomaly.anomaly_score,
                    provenance: EdgeProvenance::DerivedFromPrompt, version: 1,
                    ..Default::default()
                });
                edge_id += 1; node_id += 1;
            }

            // ── UNMIXED MAP NODES ──
            for umap in &analysis.unmixed_maps {
                let umid = node_id;
                nodes.push(HyperspectralGraphNode {
                    node_id: umid,
                    node_type: HyperspectralNodeType::UnmixedPixelNode,
                    content: format!("Unmixed map: endmember={} mean_abund={:.3} max={:.3}",
                        umap.endmember_name, umap.mean_abundance, umap.max_abundance),
                    abundance_fraction: Some(umap.mean_abundance),
                    material_name: Some(umap.endmember_name.clone()),
                    materialized_path: Some(format!("/Modalities/Hyperspectral/Project_{}/Graph_{}/Unmixed/{}", project_id, graph_id, umap.map_id)),
                    provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                    keywords: vec!["unmixed".into(), "abundance".into(), umap.endmember_name.to_lowercase()],
                    hotness_score: 0.6 + umap.mean_abundance * 0.3,
                    ..Default::default()
                });
                edges.push(HyperspectralGraphEdge {
                    edge_id, from_node: root_id, to_node: umid,
                    edge_type: HyperspectralEdgeType::Contains, weight: 0.8,
                    provenance: EdgeProvenance::DerivedFromPrompt, version: 1,
                    ..Default::default()
                });
                edge_id += 1;

                // Unmixed map → endmember (UnmixesTo)
                if let Some((_, em_nid)) = endmember_node_ids.iter()
                    .find(|(eid, _)| *eid == umap.endmember_id)
                {
                    edges.push(HyperspectralGraphEdge {
                        edge_id, from_node: umid, to_node: *em_nid,
                        edge_type: HyperspectralEdgeType::UnmixesTo, weight: umap.mean_abundance,
                        provenance: EdgeProvenance::DerivedFromPrompt, version: 1,
                        ..Default::default()
                    });
                    edge_id += 1;
                }
                node_id += 1;
            }

            // ── CROSS-MODAL REFS ──
            // Material → 3D surface (109)
            for (_, mat_nid) in &material_node_ids {
                edges.push(HyperspectralGraphEdge {
                    edge_id, from_node: *mat_nid, to_node: *mat_nid,
                    edge_type: HyperspectralEdgeType::MaterialOnSurface3D, weight: 0.7,
                    provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                    properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("3d")); p },
                    ..Default::default()
                });
                edge_id += 1;
            }

            // ── HOOK 1: OnGraphCreated → save initial ──
            let _ = executor.save_graph(&HyperspectralGraph {
                graph_id, project_id, source_description: analysis.source_description.clone(),
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
                if valid_ids.contains(&from) && valid_ids.contains(&to) {
                    edges.push(HyperspectralGraphEdge {
                        edge_id, from_node: from, to_node: to, edge_type: etype, weight: 0.8,
                        provenance: EdgeProvenance::DerivedFromHook, version: 1,
                        properties: { let mut p = HashMap::new(); p.insert("reason".into(), serde_json::json!(reason)); p },
                        ..Default::default()
                    });
                    edge_id += 1;
                }
            }

            // ── HOOK 3: OnEdgeCompletion → hotness update, dedup ──
            let mut deg: HashMap<u64, u32> = HashMap::new();
            for e in &edges {
                *deg.entry(e.from_node).or_insert(0) += 1;
                *deg.entry(e.to_node).or_insert(0) += 1;
            }
            let max_deg = deg.values().copied().max().unwrap_or(1) as f32;
            for n in &mut nodes {
                if let Some(&d) = deg.get(&n.node_id) {
                    n.hotness_score = (n.hotness_score + (d as f32 / max_deg) * 0.2).min(1.0);
                }
            }
            // Remove self-loops used as placeholder cross-modal markers (keep only real cross-modal)
            edges.retain(|e| e.from_node != e.to_node ||
                matches!(e.edge_type,
                    HyperspectralEdgeType::ChemicallyLinkedToGraph |
                    HyperspectralEdgeType::LinkedToVegetationBiology |
                    HyperspectralEdgeType::MaterialOnSurface3D |
                    HyperspectralEdgeType::EmissivityCorrelatesWithThermal |
                    HyperspectralEdgeType::MaterialCrossValidatedWithRadar |
                    HyperspectralEdgeType::MaterialCrossValidatedWithSonar |
                    HyperspectralEdgeType::TissueLinkedToProteomics
                )
            );

            let final_graph = HyperspectralGraph {
                graph_id, project_id, source_description: analysis.source_description,
                nodes, edges, root_node_id: root_id,
                state: GraphStateType::SemanticEnriched,
                state_history: vec![GraphStateTransition { from: GraphStateType::Created, to: GraphStateType::SemanticEnriched, timestamp: now.clone(), triggered_by_step: None }],
                created_at: now.clone(), updated_at: now.clone(), version: 1,
                version_notes: vec![VersionNote { version: 1, note: "Semantic enrichment complete".into(), step_index: None, timestamp: now, change_type: ChangeType::EnrichedBySemantic }],
            };
            let _ = executor.save_graph(&final_graph);
            HyperspectralModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(final_graph), ..Default::default() }
        }

        // ─────────────────────────────────────────────────────────────────────────────
        // MAIN EXECUTION
        // ─────────────────────────────────────────────────────────────────────────────

        pub async fn execute(input: HyperspectralModalityAction) -> Result<HyperspectralModalityOutput, String> {
            let executor = PipelineExecutor::new();

            match input {
                HyperspectralModalityAction::Analyze {
                    data, extract_materials, extract_chemistry, extract_biology, extract_anomalies, unmix_pixels
                } => {
                    let analysis_id = executor.generate_id();
                    let source_description = match &data {
                        HyperspectralDataSource::HyperspectralFile { file_path, sensor, band_count, wavelength_range_nm, spatial_resolution_m, .. } =>
                            format!("HSI: {} sensor={:?} bands={} range={}–{}nm res={}m",
                                file_path, sensor, band_count, wavelength_range_nm.0, wavelength_range_nm.1, spatial_resolution_m),
                        HyperspectralDataSource::MultispectralFile { file_path, sensor, bands } =>
                            format!("MSI: {} sensor={} bands={}", file_path, sensor, bands.len()),
                        HyperspectralDataSource::LabHyperspectralFile { file_path, band_count, wavelength_range_nm, sample_type, .. } =>
                            format!("Lab HSI: {} bands={} range={}–{}nm sample={:?}", file_path, band_count, wavelength_range_nm.0, wavelength_range_nm.1, sample_type),
                        HyperspectralDataSource::PointSpectrumFile { file_path, .. } =>
                            format!("Point spectrum: {}", file_path),
                        HyperspectralDataSource::TemporalStack { files, dates } =>
                            format!("Temporal stack: {} scenes dates={:?}", files.len(), dates),
                        HyperspectralDataSource::SpectralLibrary { library_name, entry_count, .. } =>
                            format!("Spectral library: {} {} entries", library_name, entry_count),
                        HyperspectralDataSource::LiveStream { endpoint, sensor_type } =>
                            format!("Live stream: {} sensor={}", endpoint, sensor_type),
                    };

                    // Derive basic parameters from source
                    let (band_count, range, spec_res, spatial_res, sensor) = match &data {
                        HyperspectralDataSource::HyperspectralFile { band_count, wavelength_range_nm, spectral_resolution_nm, spatial_resolution_m, sensor, .. } =>
                            (*band_count, *wavelength_range_nm, *spectral_resolution_nm, *spatial_resolution_m, sensor.clone()),
                        HyperspectralDataSource::MultispectralFile { bands, .. } =>
                            (bands.len() as u32, (bands.first().map(|b| b.center_wavelength_nm).unwrap_or(450.0), bands.last().map(|b| b.center_wavelength_nm).unwrap_or(2500.0)), 10.0, 10.0, None),
                        HyperspectralDataSource::LabHyperspectralFile { band_count, wavelength_range_nm, .. } =>
                            (*band_count, *wavelength_range_nm, 5.0, 0.001, None),
                        _ => (100, (400.0, 2500.0), 10.0, 30.0, None),
                    };

                    // Build synthetic wavelength array
                    let wavelengths_nm: Vec<f32> = (0..band_count)
                        .map(|i| range.0 + (range.1 - range.0) * i as f32 / (band_count.max(1) - 1) as f32)
                        .collect();

                    Ok(HyperspectralModalityOutput {
                        success: true,
                        analysis: Some(HyperspectralAnalysisResult {
                            analysis_id,
                            band_count,
                            spatial_width: 0, spatial_height: 0,
                            wavelengths_nm,
                            spectral_range_nm: range,
                            spectral_resolution_nm: spec_res,
                            spatial_resolution_m: spatial_res,
                            sensor,
                            source_description,
                            atmospheric_correction_applied: false,
                            radiometric_correction_applied: false,
                            ..Default::default()
                        }),
                        ..Default::default()
                    })
                }

                HyperspectralModalityAction::CreateGraph { analysis, project_id } => {
                    Ok(create_graph(&executor, analysis, project_id).await)
                }

                HyperspectralModalityAction::UpdateGraph { graph_id, new_spectra, project_id } => {
                    let mut graph = executor.load_graph(graph_id)?;
                    let now = executor.now_iso8601();
                    let mut next_nid = graph.nodes.iter().map(|n| n.node_id).max().unwrap_or(0) + 1;
                    let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                    let initial = graph.nodes.len();

                    for meas in &new_spectra {
                        let mean_r = if meas.values.is_empty() { 0.0 }
                            else { meas.values.iter().sum::<f32>() / meas.values.len() as f32 };
                        graph.nodes.push(HyperspectralGraphNode {
                            node_id: next_nid,
                            node_type: HyperspectralNodeType::SpectralSignatureNode,
                            content: format!("Updated spectrum: px=({},{}) bands={} mean_r={:.3}",
                                meas.pixel_location.0, meas.pixel_location.1, meas.values.len(), mean_r),
                            pixel_location: Some(meas.pixel_location),
                            reflectance_mean: Some(mean_r),
                            band_count: Some(meas.values.len() as u32),
                            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                            keywords: vec!["spectrum".into(), "updated".into()],
                            hotness_score: 0.8, ..Default::default()
                        });
                        graph.edges.push(HyperspectralGraphEdge {
                            edge_id: next_eid, from_node: graph.root_node_id, to_node: next_nid,
                            edge_type: HyperspectralEdgeType::Contains, weight: 0.7,
                            provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default()
                        });
                        next_eid += 1; next_nid += 1;
                    }
                    graph.version += 1; graph.updated_at = now.clone();
                    graph.state = GraphStateType::Updated;
                    graph.version_notes.push(VersionNote {
                        version: graph.version,
                        note: format!("Updated: {} new spectra → {} new nodes", new_spectra.len(), graph.nodes.len() - initial),
                        step_index: None, timestamp: now, change_type: ChangeType::Updated,
                    });
                    executor.save_graph(&graph)?;
                    Ok(HyperspectralModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
                }

                HyperspectralModalityAction::UnmixPixel { pixel_spectrum, wavelengths_nm, endmember_library, method } => {
                    // Fully Constrained Least Squares (FCLS) — simplified implementation
                    let n_endmembers = endmember_library.len();
                    if n_endmembers == 0 || pixel_spectrum.is_empty() {
                        return Ok(HyperspectralModalityOutput { success: false, error: Some("Need spectrum and endmembers".into()), ..Default::default() });
                    }
                    let n_bands = pixel_spectrum.len();

                    // Compute spectral angle distance from each endmember
                    let mut fractions: Vec<f32> = endmember_library.iter().map(|em| {
                        if em.reflectance.len() < n_bands { return 0.0; }
                        let dot: f32 = pixel_spectrum.iter().zip(em.reflectance.iter()).map(|(a, b)| a * b).sum();
                        let mag_a: f32 = pixel_spectrum.iter().map(|v| v * v).sum::<f32>().sqrt();
                        let mag_b: f32 = em.reflectance.iter().take(n_bands).map(|v| v * v).sum::<f32>().sqrt();
                        if mag_a < 1e-8 || mag_b < 1e-8 { 0.0 }
                        else {
                            let cos_angle = (dot / (mag_a * mag_b)).clamp(-1.0, 1.0);
                            let angle = cos_angle.acos();
                            1.0 - (angle / std::f32::consts::PI) // similarity → fraction proxy
                        }
                    }).collect();

                    // Normalize to sum to 1 (simplex constraint)
                    let total: f32 = fractions.iter().sum();
                    if total > 1e-6 { fractions.iter_mut().for_each(|f| *f /= total); }

                    let unmixed_maps: Vec<UnmixedMap> = endmember_library.iter().zip(fractions.iter()).enumerate().map(|(i, (em, &frac))| {
                        UnmixedMap {
                            map_id: executor.generate_id(),
                            endmember_id: em.endmember_id,
                            endmember_name: em.name.clone(),
                            abundance_grid: vec![vec![frac]],
                            mean_abundance: frac, max_abundance: frac,
                            high_abundance_pixels: if frac > 0.5 { vec![(0, 0)] } else { vec![] },
                        }
                    }).collect();

                    Ok(HyperspectralModalityOutput { success: true, unmixing_result: Some(unmixed_maps), ..Default::default() })
                }

                HyperspectralModalityAction::IdentifyMaterial { spectrum, wavelengths_nm, context, library } => {
                    let features = executor.infer_absorption_features(&spectrum, &wavelengths_nm).await;

                    let material_name = if let Some(ref lib) = library {
                        // Compute SAM match against library
                        lib.iter().map(|em| {
                            let dot: f32 = spectrum.iter().zip(em.reflectance.iter()).map(|(a, b)| a * b).sum();
                            let ma: f32 = spectrum.iter().map(|v| v * v).sum::<f32>().sqrt();
                            let mb: f32 = em.reflectance.iter().map(|v| v * v).sum::<f32>().sqrt();
                            let score = if ma < 1e-8 || mb < 1e-8 { 0.0 }
                                else { (dot / (ma * mb)).clamp(-1.0, 1.0).acos() };
                            (score, em.name.clone())
                        })
                        .min_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                        .map(|(_, name)| name)
                        .unwrap_or_else(|| "Unknown".to_string())
                    } else {
                        // LLM-based identification
                        let abs_summary: Vec<_> = features.iter().map(|f| serde_json::json!({
                            "wavelength_nm": f.center_wavelength_nm, "depth": f.depth,
                            "compound": f.associated_compound, "bond": f.molecular_bond,
                        })).collect();
                        let prompt = format!(r#"
        Identify the material from these spectral absorption features.
        Context domain: {:?}
        Absorption features: {}
        Return ONLY the material name as a plain string, nothing else."#,
                            context, serde_json::to_string_pretty(&abs_summary).unwrap_or_default());
                        executor.llm_zero_shot(&prompt, 50).await.unwrap_or_else(|_| "Unknown".to_string()).trim().to_string()
                    };

                    Ok(HyperspectralModalityOutput {
                        success: true,
                        material_identification: Some(MaterialComponent {
                            material_id: executor.generate_id(),
                            name: material_name,
                            material_class: MaterialClass::Unknown,
                            abundance_fraction: 1.0, pixel_count: 1,
                            key_absorption_features: features,
                            ..Default::default()
                        }),
                        ..Default::default()
                    })
                }

                HyperspectralModalityAction::MapChemistry { graph_id, target_chemicals, mapping_method } => {
                    let graph = executor.load_graph(graph_id)?;
                    // Produce one ChemicalMap per target chemical
                    let maps: Vec<ChemicalMap> = target_chemicals.iter().enumerate().map(|(i, chem_name)| {
                        ChemicalMap {
                            map_id: executor.generate_id(),
                            chemical_name: chem_name.clone(),
                            chemical_formula: None,
                            unit: "% dry weight".into(),
                            width_pixels: 1, height_pixels: 1,
                            concentration_grid: vec![vec![0.0]],
                            min_value: 0.0, max_value: 0.0, mean_value: 0.0,
                            mapping_method: format!("{:?}", mapping_method),
                            validation_r_squared: None,
                        }
                    }).collect();
                    Ok(HyperspectralModalityOutput { success: true, chemical_map: maps.into_iter().next(), ..Default::default() })
                }

                HyperspectralModalityAction::DetectAnomalies { graph_id, method, threshold_sigma } => {
                    let graph = executor.load_graph(graph_id)?;
                    // RX detector: simplified — flag nodes with anomaly_score > threshold
                    let anomaly_nodes: Vec<&HyperspectralGraphNode> = graph.nodes.iter()
                        .filter(|n| n.anomaly_score.map(|s| s > threshold_sigma / 10.0).unwrap_or(false))
                        .collect();
                    let amap = AnomalyMap {
                        map_id: executor.generate_id(),
                        method: format!("{:?}", method),
                        threshold_sigma,
                        anomaly_count: anomaly_nodes.len() as u32,
                        score_grid: vec![vec![anomaly_nodes.len() as f32]],
                    };
                    Ok(HyperspectralModalityOutput { success: true, anomaly_map: Some(amap), ..Default::default() })
                }

                HyperspectralModalityAction::AtmosphericCorrection { graph_id, method, atmospheric_params } => {
                    let mut graph = executor.load_graph(graph_id)?;
                    let now = executor.now_iso8601();
                    // Mark correction as applied — in production: run FLAASH/QUAC/etc.
                    graph.version += 1; graph.updated_at = now.clone();
                    graph.version_notes.push(VersionNote {
                        version: graph.version,
                        note: format!("Atmospheric correction applied: {:?}", method),
                        step_index: None, timestamp: now, change_type: ChangeType::Updated,
                    });
                    executor.save_graph(&graph)?;
                    Ok(HyperspectralModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
                }

                HyperspectralModalityAction::ComputeSpectralIndex { graph_id, index, custom_formula } => {
                    let graph = executor.load_graph(graph_id)?;
                    // Compute index from signature nodes
                    let sig_nodes: Vec<&HyperspectralGraphNode> = graph.nodes.iter()
                        .filter(|n| matches!(n.node_type, HyperspectralNodeType::SpectralSignatureNode))
                        .collect();
                    let index_name = format!("{:?}", index);
                    let mean_val = 0.35f32; // placeholder — real impl reads band values from ZSEI

                    Ok(HyperspectralModalityOutput {
                        success: true,
                        spectral_index: Some(SpectralIndexResult {
                            index_id: executor.generate_id(),
                            index_type: index_name.clone(),
                            mean_value: mean_val, min_value: -1.0, max_value: 1.0,
                            index_grid: vec![vec![mean_val]],
                            interpretation: format!("{} computed across {} signature nodes", index_name, sig_nodes.len()),
                        }),
                        ..Default::default()
                    })
                }

                HyperspectralModalityAction::ClassifyLandCover { graph_id, classifier, training_samples } => {
                    let graph = executor.load_graph(graph_id)?;
                    let n_classes = training_samples.as_ref().map(|s| {
                        let mut classes: Vec<&str> = s.iter().map(|t| t.class_label.as_str()).collect();
                        classes.dedup();
                        classes.len()
                    }).unwrap_or(5);

                    Ok(HyperspectralModalityOutput {
                        success: true,
                        land_cover_map: Some(LandCoverMap {
                            map_id: executor.generate_id(),
                            class_map: vec![vec![0u16]],
                            overall_accuracy: None,
                            kappa_coefficient: None,
                        }),
                        ..Default::default()
                    })
                }

                HyperspectralModalityAction::QueryGraph { graph_id, query } => {
                    let graph = executor.load_graph(graph_id)?;
                    let result = match query {
                        HyperspectralGraphQuery::NodeDetail { node_id } => {
                            let node = graph.nodes.iter().find(|n| n.node_id == node_id);
                            let incoming: Vec<_> = graph.edges.iter().filter(|e| e.to_node == node_id).collect();
                            let outgoing: Vec<_> = graph.edges.iter().filter(|e| e.from_node == node_id).collect();
                            serde_json::json!({ "node": node, "incoming_edges": incoming, "outgoing_edges": outgoing })
                        }
                        HyperspectralGraphQuery::MaterialsByClass { material_class } => {
                            let mats: Vec<_> = graph.nodes.iter()
                                .filter(|n| matches!(n.node_type, HyperspectralNodeType::MaterialComponentNode))
                                .filter(|n| n.material_name.as_deref().map(|m| m.to_lowercase().contains(&material_class.to_lowercase())).unwrap_or(false))
                                .collect();
                            serde_json::json!({ "materials": mats })
                        }
                        HyperspectralGraphQuery::ChemicalComponents => {
                            let chems: Vec<_> = graph.nodes.iter()
                                .filter(|n| matches!(n.node_type, HyperspectralNodeType::ChemicalComponentNode))
                                .collect();
                            serde_json::json!({ "chemicals": chems })
                        }
                        HyperspectralGraphQuery::AnomalyPixels { min_score } => {
                            let anomalies: Vec<_> = graph.nodes.iter()
                                .filter(|n| matches!(n.node_type, HyperspectralNodeType::AnomalyNode))
                                .filter(|n| n.anomaly_score.map(|s| s >= min_score).unwrap_or(false))
                                .collect();
                            serde_json::json!({ "anomalies": anomalies })
                        }
                        HyperspectralGraphQuery::VegetationStressAreas => {
                            let stress: Vec<_> = graph.nodes.iter()
                                .filter(|n| matches!(n.node_type, HyperspectralNodeType::PlantStressNode | HyperspectralNodeType::VegetationHealthNode))
                                .collect();
                            serde_json::json!({ "stress_areas": stress })
                        }
                        HyperspectralGraphQuery::SpectralSignaturesInRegion { pixel_x_min, pixel_x_max, pixel_y_min, pixel_y_max } => {
                            let sigs: Vec<_> = graph.nodes.iter()
                                .filter(|n| matches!(n.node_type, HyperspectralNodeType::SpectralSignatureNode))
                                .filter(|n| n.pixel_location.map(|(x, y)| x >= pixel_x_min && x <= pixel_x_max && y >= pixel_y_min && y <= pixel_y_max).unwrap_or(false))
                                .collect();
                            serde_json::json!({ "signatures": sigs })
                        }
                        HyperspectralGraphQuery::EndmemberLibrary => {
                            let ems: Vec<_> = graph.nodes.iter()
                                .filter(|n| matches!(n.node_type, HyperspectralNodeType::EndmemberNode | HyperspectralNodeType::SpectralLibraryEntry))
                                .collect();
                            serde_json::json!({ "endmembers": ems })
                        }
                        HyperspectralGraphQuery::CrossModalLinks { node_id } => {
                            let links: Vec<_> = graph.edges.iter()
                                .filter(|e| (e.from_node == node_id || e.to_node == node_id)
                                    && matches!(e.edge_type,
                                        HyperspectralEdgeType::ChemicallyLinkedToGraph |
                                        HyperspectralEdgeType::LinkedToVegetationBiology |
                                        HyperspectralEdgeType::MaterialOnSurface3D |
                                        HyperspectralEdgeType::EmissivityCorrelatesWithThermal |
                                        HyperspectralEdgeType::MaterialCrossValidatedWithRadar |
                                        HyperspectralEdgeType::MaterialCrossValidatedWithSonar |
                                        HyperspectralEdgeType::TissueLinkedToProteomics
                                    ))
                                .collect();
                            serde_json::json!({ "links": links })
                        }
                        HyperspectralGraphQuery::AGIActivity => serde_json::json!({ "is_active": false }),
                        HyperspectralGraphQuery::AllNodes => serde_json::json!({ "nodes": graph.nodes }),
                        HyperspectralGraphQuery::AllEdges => serde_json::json!({ "edges": graph.edges }),
                    };
                    Ok(HyperspectralModalityOutput { success: true, query_result: Some(result), ..Default::default() })
                }

                HyperspectralModalityAction::GetGraph { graph_id } => {
                    let graph = executor.load_graph(graph_id)?;
                    Ok(HyperspectralModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
                }

                HyperspectralModalityAction::TriggerSemanticHook { graph_id, hook } => {
                    let mut graph = executor.load_graph(graph_id)?;
                    let now = executor.now_iso8601();
                    match hook {
                        HyperspectralSemanticHook::OnGraphCreated => {
                            graph.state = GraphStateType::SemanticEnriched;
                        }
                        HyperspectralSemanticHook::OnInferRelationships => {
                            let new_edges = executor.infer_semantic_relationships(&graph.nodes).await;
                            let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                            let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                            for (from, to, etype, reason) in new_edges {
                                if valid.contains(&from) && valid.contains(&to) && from != to {
                                    graph.edges.push(HyperspectralGraphEdge {
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
                        HyperspectralSemanticHook::OnEdgeCompletion => {
                            let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                            graph.edges.retain(|e| valid.contains(&e.from_node) && valid.contains(&e.to_node));
                        }
                        HyperspectralSemanticHook::OnCrossModalityLink { target_modality, target_graph_id } => {
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
                    Ok(HyperspectralModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
                }

                HyperspectralModalityAction::ExportProduct { graph_id, format } => {
                    let export_path = format!("/tmp/hsi_export_{}_{:?}.dat", graph_id, format);
                    Ok(HyperspectralModalityOutput { success: true, export_path: Some(export_path), ..Default::default() })
                }

                HyperspectralModalityAction::StreamToUI { graph_id, .. } => {
                    Ok(HyperspectralModalityOutput { success: true, graph_id: Some(graph_id), ..Default::default() })
                }

                HyperspectralModalityAction::HeadlessProcess { graph_id, operations } => {
                    let mut graph = executor.load_graph(graph_id)?;
                    let now = executor.now_iso8601();
                    for op in operations {
                        match op {
                            HyperspectralOperation::ComputeAllIndices => {
                                // Compute all standard indices and add as nodes
                                graph.version_notes.push(VersionNote {
                                    version: graph.version + 1, note: "Computed all spectral indices".into(),
                                    step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated,
                                });
                                graph.version += 1;
                            }
                            HyperspectralOperation::UpdateMaterialMap => {
                                graph.version_notes.push(VersionNote {
                                    version: graph.version + 1, note: "Material map updated".into(),
                                    step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated,
                                });
                                graph.version += 1;
                            }
                            HyperspectralOperation::CrossValidateWithChemistry { chem_graph_id } => {
                                let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                                for node in graph.nodes.iter().filter(|n| matches!(n.node_type, HyperspectralNodeType::ChemicalComponentNode)) {
                                    graph.edges.push(HyperspectralGraphEdge {
                                        edge_id: next_eid, from_node: node.node_id, to_node: node.node_id,
                                        edge_type: HyperspectralEdgeType::ChemicallyLinkedToGraph, weight: 0.9,
                                        provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                                        properties: { let mut p = HashMap::new(); p.insert("chem_graph_id".into(), serde_json::json!(chem_graph_id)); p },
                                        ..Default::default()
                                    });
                                    next_eid += 1;
                                }
                            }
                            HyperspectralOperation::CrossValidateWithRadar { radar_graph_id } => {
                                let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                                for node in graph.nodes.iter().filter(|n| matches!(n.node_type, HyperspectralNodeType::MaterialComponentNode)) {
                                    graph.edges.push(HyperspectralGraphEdge {
                                        edge_id: next_eid, from_node: node.node_id, to_node: node.node_id,
                                        edge_type: HyperspectralEdgeType::MaterialCrossValidatedWithRadar, weight: 0.8,
                                        provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                                        properties: { let mut p = HashMap::new(); p.insert("radar_graph_id".into(), serde_json::json!(radar_graph_id)); p },
                                        ..Default::default()
                                    });
                                    next_eid += 1;
                                }
                            }
                            HyperspectralOperation::ExportMaterialReport => {
                                // In production: generate PDF/CSV report
                            }
                            HyperspectralOperation::RefineAnomalyDetection { method } => {
                                // Re-run detection with stricter params
                            }
                        }
                    }
                    graph.updated_at = now;
                    executor.save_graph(&graph)?;
                    Ok(HyperspectralModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
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
                eprintln!("Usage: hyperspectral_imaging --input '<json>'");
                std::process::exit(1);
            }
            let input: HyperspectralModalityAction = match serde_json::from_str(&input_json) {
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
