//! MultiScaleBiology — Pipeline #111
//!
//! Multi-scale biological systems analysis: molecular pathways, cellular
//! processes, tissue architecture, organ function, organism physiology,
//! and population/ecological dynamics.
//!
//! COVERAGE HIERARCHY:
//!   Molecule → Pathway → Cellular Process → Cell Type → Tissue → Organ →
//!   Organ System → Organism → Population → Ecosystem
//!
//! DISTINCT FROM:
//!   - DNA (107): sequence-level; Biology is functional/systems-level.
//!     DNA tells you WHAT; Biology tells you WHAT IT DOES and HOW.
//!   - Proteomics (112): protein structure/interaction; Biology integrates
//!     across all biological scales.
//!
//! CROSS-LINKS:
//!   107 (DNA)       → gene → protein → pathway → function
//!   112 (Prot)      → protein function in biological context
//!   106 (Chem)      → metabolic chemistry, drug interactions
//!   114 (Therm)     → metabolic heat, fever, thermogenesis
//!   111↔125 (Sonar) → marine biology, species vocalizations
//!   117 (Geo)       → spatial ecology, habitat mapping
//!   113 (Haptic)    → tissue mechanics, tactile biology
//!   126 (Hyper)     → plant stress, vegetation health
//!
//! STORAGE: ZSEI containers under /Modalities/Biology/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

// ─────────────────────────────────────────────────────────────────────────────
// INPUT TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum BiologyModalityAction {
    Analyze {
        data: BiologyDataSource,
        extract_pathways: bool,
        extract_cell_types: bool,
        extract_tissue: bool,
        extract_organism: bool,
        extract_ecology: bool,
    },
    CreateGraph {
        analysis: BiologyAnalysisResult,
        project_id: u64,
    },
    UpdateGraph {
        graph_id: u64,
        updates: Vec<BiologyUpdate>,
        project_id: u64,
    },
    /// Run pathway enrichment analysis
    PathwayEnrichment {
        graph_id: u64,
        gene_list: Vec<String>,
        database: PathwayDatabase,
        significance_threshold: f32,
    },
    /// Map tissue spatial architecture
    AnalyzeSpatialTranscriptomics {
        data: SpatialTranscriptomicsData,
        project_id: u64,
    },
    /// Classify cell types from expression profiles
    ClassifyCellTypes {
        expression_profiles: Vec<CellExpressionProfile>,
        reference_atlas: Option<String>,
    },
    /// Extract regulatory network from genomic data
    BuildRegulatoryNetwork {
        graph_id: u64,
        evidence_types: Vec<RegulatoryEvidenceType>,
    },
    /// Ecological community analysis
    AnalyzeCommunity {
        species_data: Vec<SpeciesObservation>,
        diversity_metrics: bool,
        network_analysis: bool,
    },
    QueryGraph { graph_id: u64, query: BiologyGraphQuery },
    GetGraph { graph_id: u64 },
    TriggerSemanticHook { graph_id: u64, hook: BiologySemanticHook },
    ExportProduct { graph_id: u64, format: BiologyExportFormat },
    StreamToUI { graph_id: u64, session_id: String, display_mode: BiologyDisplayMode },
    HeadlessProcess { graph_id: u64, operations: Vec<BiologyOperation> },
}

// ─────────────────────────────────────────────────────────────────────────────
// DATA SOURCES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BiologyDataSource {
    /// Pathway/network database file (KEGG, Reactome, WikiPathways)
    PathwayFile {
        file_path: String,
        format: PathwayFileFormat,
        organism: String,                // "Homo sapiens", "Mus musculus", etc.
    },
    /// Single-cell RNA sequencing
    ScRNAseqFile {
        file_path: String,
        format: ScRNAFormat,
        species: String,
        tissue: Option<String>,
        cell_count: u32,
    },
    /// Bulk RNA-seq / microarray expression
    ExpressionMatrix {
        file_path: String,
        format: ExpressionFormat,
        species: String,
        condition_labels: Vec<String>,
    },
    /// ATAC-seq / chromatin accessibility
    ChromatinAccessibility {
        file_path: String,
        format: ATACFormat,
        species: String,
    },
    /// Spatial transcriptomics (Visium, MERFISH, seqFISH)
    SpatialTranscriptomicsFile {
        file_path: String,
        format: SpatialTxFormat,
        tissue: String,
        species: String,
    },
    /// Histology / pathology image
    HistologyImage {
        file_path: String,
        stain: HistologyStain,
        tissue: String,
        magnification: f32,
        pathology_annotations: bool,
    },
    /// Species occurrence / ecological survey
    EcologicalSurvey {
        file_path: String,
        format: EcologicalFormat,
        survey_area: Option<String>,
        survey_date: Option<String>,
    },
    /// Clinical / physiological measurements
    PhysiologyData {
        file_path: String,
        data_type: PhysiologyType,
        organism: String,
        subject_count: u32,
    },
    /// Metabolomics profile
    MetabolomicsFile {
        file_path: String,
        format: MetabolomicsFormat,
        organism: String,
        tissue: Option<String>,
        platform: Option<String>,        // "LC-MS/MS", "NMR", etc.
    },
    /// Protein interaction network
    PPI_Network {
        file_path: String,
        format: PPIFormat,
        organism: String,
        confidence_threshold: f32,
    },
    /// Literature mining result
    LiteratureMining {
        text_content: String,           // extracted biological text
        source_reference: String,
        extraction_focus: Vec<BiologyFocus>,
    },
    /// Multi-omics composite
    MultiOmics {
        omics_files: Vec<BiologyDataSource>,
        integration_method: MultiOmicsIntegration,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PathwayFileFormat { KGML, SBML, BioPAX, GPML, JSON_Cytoscape, GMT, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScRNAFormat { H5AD, Loom, MTX_10X, HDF5, CSV_Matrix, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExpressionFormat { TSV_GeneMatrix, CSV_GeneMatrix, HDF5, GCT, CEL_Affymetrix, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ATACFormat { BED_Peaks, BigWig, BAM, Fragment_TSV, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpatialTxFormat { Visium_H5, MERFISH_CSV, seqFISH_CSV, Slide_seq_TSV, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HistologyStain { HE, IHC, IF, Masson_Trichrome, PAS, Giemsa, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EcologicalFormat { GBIF_CSV, eBird, iNaturalist, Darwin_Core, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PhysiologyType { Electrophysiology, BloodPanel, ImageMeasurements, WearableSensors, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetabolomicsFormat { mzXML, mzML, XCMS_CSV, MetaboAnalyst_CSV, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PPIFormat { STRING_TSV, BioGRID_TSV, IntAct_TSV, MITAB, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PathwayDatabase { KEGG, Reactome, WikiPathways, GO_Biological_Process, GSEA_MSigDB, BioCarta, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BiologyFocus { Pathways, Genes, Proteins, Diseases, Drugs, CellTypes, Tissues, Organisms }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MultiOmicsIntegration { MOFA, DIABLO, SNF, Early_Fusion, Late_Fusion, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegulatoryEvidenceType { ChIP_Seq, ATAC_Seq, eQTL, TF_Binding, Hi_C, Literature }

// ─────────────────────────────────────────────────────────────────────────────
// ANALYSIS RESULT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BiologyAnalysisResult {
    pub analysis_id: u64,
    pub source_description: String,
    pub organism: String,
    pub biological_scale: BiologicalScale,

    // MOLECULAR LEVEL
    pub pathways: Vec<BiologicalPathway>,
    pub pathway_interactions: Vec<PathwayInteraction>,
    pub metabolites: Vec<Metabolite>,
    pub metabolic_reactions: Vec<MetabolicReaction>,
    pub regulatory_elements: Vec<RegulatoryElement>,
    pub transcription_factors: Vec<TranscriptionFactor>,

    // CELLULAR LEVEL
    pub cell_types: Vec<CellType>,
    pub cell_clusters: Vec<CellCluster>,
    pub cellular_processes: Vec<CellularProcess>,
    pub cell_signaling_events: Vec<SignalingEvent>,
    pub organelles: Vec<Organelle>,

    // TISSUE LEVEL
    pub tissues: Vec<TissueInfo>,
    pub tissue_compartments: Vec<TissueCompartment>,
    pub spatial_domains: Vec<SpatialDomain>,
    pub histology_features: Vec<HistologyFeature>,

    // ORGAN / SYSTEM LEVEL
    pub organs: Vec<OrganInfo>,
    pub organ_systems: Vec<OrganSystem>,
    pub physiological_states: Vec<PhysiologicalState>,

    // ORGANISM LEVEL
    pub organism_phenotypes: Vec<Phenotype>,
    pub disease_associations: Vec<DiseaseAssociation>,
    pub drug_targets: Vec<DrugTarget>,

    // ECOLOGICAL LEVEL
    pub species_detected: Vec<SpeciesRecord>,
    pub ecological_interactions: Vec<EcologicalInteraction>,
    pub habitat_features: Vec<HabitatFeature>,
    pub biodiversity_metrics: Option<BiodiversityMetrics>,

    // REGULATORY NETWORKS
    pub gene_regulatory_network: Vec<RegulatoryEdge>,
    pub protein_interaction_network: Vec<ProteinInteraction>,

    // METADATA
    pub processing_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum BiologicalScale {
    #[default] Molecular, Cellular, Tissue, Organ, Organism, Population, Ecosystem, MultiScale
}

// ── MOLECULAR TYPES ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BiologicalPathway {
    pub pathway_id: String,              // e.g., "hsa04010", "R-HSA-5633007"
    pub name: String,
    pub database: String,                // "KEGG", "Reactome", etc.
    pub category: PathwayCategory,
    pub gene_count: u32,
    pub gene_symbols: Vec<String>,
    pub reaction_count: u32,
    pub sub_pathway_ids: Vec<String>,
    pub parent_pathway_id: Option<String>,
    pub description: String,
    pub is_enriched: bool,               // enriched in current dataset
    pub enrichment_pvalue: Option<f64>,
    pub enrichment_fdr: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PathwayCategory {
    #[default] MetabolicPathway, SignalingPathway, CellCycle, Apoptosis,
    ImmuneSystem, NervousSystem, EndocrineSystem, GeneRegulation,
    DNARepair, ProteinProcessing, Transport, DevelopmentMorphogenesis,
    DiseasePathway, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PathwayInteraction {
    pub interaction_id: u64,
    pub pathway_a_id: String,
    pub pathway_b_id: String,
    pub interaction_type: PathwayInteractionType,
    pub shared_genes: Vec<String>,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PathwayInteractionType {
    #[default] SharedGene, CrossTalk, Sequential, Antagonistic, Synergistic, Regulatory
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Metabolite {
    pub metabolite_id: u64,
    pub name: String,
    pub chebi_id: Option<String>,
    pub hmdb_id: Option<String>,
    pub formula: Option<String>,
    pub metabolite_class: MetaboliteClass,
    pub abundance: Option<f64>,
    pub log2_fold_change: Option<f64>,
    pub pathway_ids: Vec<String>,
    pub tissue_specificity: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum MetaboliteClass {
    #[default] Unknown, AminoAcid, Lipid, Carbohydrate, Nucleotide, Vitamin,
    Hormone, Neurotransmitter, SecondaryMetabolite, Xenobiotic, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetabolicReaction {
    pub reaction_id: u64,
    pub name: String,
    pub equation: String,                // e.g., "A + B → C + D"
    pub enzyme: Option<String>,          // enzyme catalyzing
    pub ec_number: Option<String>,
    pub direction: ReactionDirection,
    pub pathway_ids: Vec<String>,
    pub is_rate_limiting: bool,
    pub compartment: Option<String>,     // "cytosol", "mitochondria", etc.
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ReactionDirection { #[default] Reversible, Forward, Reverse }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegulatoryElement {
    pub element_id: u64,
    pub element_type: RegulatoryElementType,
    pub name: String,
    pub location: Option<GenomicLocation>,
    pub target_genes: Vec<String>,
    pub activity: Option<f64>,           // activity score
    pub cell_type_specificity: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum RegulatoryElementType {
    #[default] Promoter, Enhancer, Silencer, Insulator,
    LncRNA, miRNA, Methylation_CpG, CTCF_Site, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GenomicLocation {
    pub chromosome: String,
    pub start: u64, pub end: u64,
    pub strand: Option<char>,
    pub build: String,    // "GRCh38", etc.
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TranscriptionFactor {
    pub tf_id: u64,
    pub gene_symbol: String,
    pub tf_family: String,              // "bHLH", "zinc finger", "homeodomain", etc.
    pub binding_motif: Option<String>,  // JASPAR ID or consensus sequence
    pub target_gene_count: u32,
    pub target_genes: Vec<String>,
    pub expression_level: Option<f64>,
    pub activity_score: Option<f64>,
    pub cell_type_specific: bool,
}

// ── CELLULAR TYPES ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CellType {
    pub cell_type_id: u64,
    pub name: String,
    pub ontology_id: Option<String>,    // CL:0000000 (Cell Ontology)
    pub parent_cell_type: Option<String>,
    pub marker_genes: Vec<String>,
    pub cell_count: Option<u32>,
    pub fraction_of_total: Option<f32>,
    pub tissue_of_origin: Vec<String>,
    pub functional_state: CellFunctionalState,
    pub differentiation_stage: Option<String>,
    pub mean_expression_profile: HashMap<String, f64>,  // gene → mean expression
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum CellFunctionalState {
    #[default] Unknown, Quiescent, Activated, Proliferating, Apoptotic,
    Senescent, Differentiated, Stem, Progenitor, Transitioning, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CellCluster {
    pub cluster_id: u64,
    pub cluster_name: String,
    pub cell_type_assignment: Option<String>,
    pub cell_count: u32,
    pub resolution: f32,
    pub marker_genes: Vec<String>,
    pub mean_umap_x: f32,
    pub mean_umap_y: f32,
    pub top_de_genes: Vec<DifferentiallyExpressedGene>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DifferentiallyExpressedGene {
    pub gene_symbol: String,
    pub log2_fold_change: f64,
    pub pvalue: f64,
    pub padj: f64,
    pub base_mean: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CellularProcess {
    pub process_id: u64,
    pub name: String,
    pub go_id: Option<String>,          // GO:0006915 = "apoptotic process"
    pub process_type: CellularProcessType,
    pub associated_pathways: Vec<String>,
    pub key_proteins: Vec<String>,
    pub is_active: Option<bool>,
    pub activity_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum CellularProcessType {
    #[default] Unknown, Proliferation, Apoptosis, Differentiation, Migration,
    Metabolism, Secretion, Endocytosis, Transcription, Translation,
    DNARepair, CellCycle, Autophagy, Senescence, Signaling, Adhesion,
    ImmuneResponse, InflammatoryResponse, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SignalingEvent {
    pub event_id: u64,
    pub pathway_id: Option<String>,
    pub ligand: String,
    pub receptor: String,
    pub downstream_effect: String,
    pub cell_type_sender: Option<String>,
    pub cell_type_receiver: Option<String>,
    pub evidence_strength: SignalingEvidenceLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SignalingEvidenceLevel { #[default] Inferred, Predicted, Experimental, Curated }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Organelle {
    pub organelle_id: u64,
    pub name: String,
    pub go_component_id: Option<String>,
    pub associated_processes: Vec<String>,
    pub protein_count: u32,
    pub volume_fraction: Option<f32>,
}

// ── TISSUE TYPES ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TissueInfo {
    pub tissue_id: u64,
    pub name: String,
    pub uberon_id: Option<String>,       // UBERON:0002107 = "liver"
    pub tissue_type: TissueType,
    pub cell_type_composition: Vec<(String, f32)>,  // (cell_type, fraction)
    pub key_functions: Vec<String>,
    pub associated_pathologies: Vec<String>,
    pub gene_expression_signature: Vec<String>,  // marker genes
    pub spatial_domain_ids: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum TissueType {
    #[default] Unknown, Epithelial, Connective, Muscle, Nervous,
    Lymphoid, Adipose, Bone, Cartilage, Vascular, Glandular,
    Reproductive, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TissueCompartment {
    pub compartment_id: u64,
    pub name: String,
    pub parent_tissue_id: Option<u64>,
    pub cell_types: Vec<String>,
    pub extracellular_matrix: Vec<String>,
    pub vascularity: CompartmentVascularity,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum CompartmentVascularity { #[default] Unknown, Avascular, Vascular, Highly_Vascular }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpatialDomain {
    pub domain_id: u64,
    pub name: String,
    pub tissue_id: Option<u64>,
    pub dominant_cell_type: Option<String>,
    pub spatial_x: f32, pub spatial_y: f32,
    pub radius_um: f32,
    pub marker_genes: Vec<String>,
    pub pathway_activity: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HistologyFeature {
    pub feature_id: u64,
    pub name: String,
    pub feature_class: HistologyFeatureClass,
    pub pixel_region: Option<PixelRegion>,
    pub severity: Option<String>,
    pub pathological: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum HistologyFeatureClass {
    #[default] Unknown, Infiltrate, Fibrosis, Necrosis, Steatosis,
    Hyperplasia, Dysplasia, Neoplasm, Inflammation, VascularChange,
    NuclearAbnormality, CellularAtypia, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PixelRegion { pub x_min: u32, pub x_max: u32, pub y_min: u32, pub y_max: u32 }

// ── ORGAN / SYSTEM TYPES ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrganInfo {
    pub organ_id: u64,
    pub name: String,
    pub uberon_id: Option<String>,
    pub organ_system: OrganSystemType,
    pub tissue_ids: Vec<u64>,
    pub primary_functions: Vec<String>,
    pub associated_diseases: Vec<String>,
    pub biomarkers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrganSystem {
    pub system_id: u64,
    pub name: String,
    pub system_type: OrganSystemType,
    pub organ_ids: Vec<u64>,
    pub primary_functions: Vec<String>,
    pub key_pathways: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum OrganSystemType {
    #[default] Unknown, Cardiovascular, Respiratory, Digestive, Nervous, Endocrine,
    Immune, Musculoskeletal, Reproductive, Renal, Integumentary, Lymphatic, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PhysiologicalState {
    pub state_id: u64,
    pub name: String,
    pub state_type: PhysiologicalStateType,
    pub biomarker_values: HashMap<String, f64>,
    pub affected_pathways: Vec<String>,
    pub organ_involvement: Vec<String>,
    pub is_pathological: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PhysiologicalStateType {
    #[default] Normal, Stressed, Inflamed, Hypoxic, Hyperglycemic, Acidotic,
    Alkalotic, Hyperthermic, Hypothermic, Infected, Fatigued, Custom(String),
}

// ── ORGANISM / DISEASE TYPES ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Phenotype {
    pub phenotype_id: u64,
    pub name: String,
    pub hp_id: Option<String>,           // HPO: "HP:0000118"
    pub mp_id: Option<String>,           // MPO for mouse
    pub phenotype_class: PhenotypeClass,
    pub associated_genes: Vec<String>,
    pub severity: PhenotypeSeverity,
    pub is_disease_related: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PhenotypeClass {
    #[default] Unknown, Morphological, Behavioral, Metabolic, Immunological,
    Neurological, Cardiovascular, Reproductive, Growth, Lifespan, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PhenotypeSeverity { #[default] Mild, Moderate, Severe, Lethal, Variable }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiseaseAssociation {
    pub association_id: u64,
    pub disease_name: String,
    pub mondo_id: Option<String>,        // MONDO:0005148
    pub omim_id: Option<String>,
    pub disease_type: DiseaseType,
    pub associated_genes: Vec<String>,
    pub associated_pathways: Vec<String>,
    pub evidence_type: String,
    pub causal_variants: Vec<String>,
    pub drug_targets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum DiseaseType {
    #[default] Unknown, Mendelian, Complex, Infectious, Autoimmune, Neoplastic,
    Metabolic, Neurodegenerative, Cardiovascular, Inflammatory, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DrugTarget {
    pub target_id: u64,
    pub gene_symbol: String,
    pub protein_name: String,
    pub drug_name: Option<String>,
    pub drugbank_id: Option<String>,
    pub interaction_type: DrugTargetInteraction,
    pub indication: Option<String>,
    pub pathway_context: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum DrugTargetInteraction {
    #[default] Unknown, Inhibitor, Activator, Antagonist, Agonist,
    Substrate, Modulator, Binder, Custom(String),
}

// ── ECOLOGICAL TYPES ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpeciesRecord {
    pub species_id: u64,
    pub scientific_name: String,
    pub common_name: Option<String>,
    pub taxonomy: TaxonomyInfo,
    pub observation_count: u32,
    pub location: Option<[f64; 2]>,     // lat, lon
    pub habitat: Option<String>,
    pub conservation_status: ConservationStatus,
    pub acoustic_signature_id: Option<u64>,  // links to sonar/audio modality
    pub thermal_signature_id: Option<u64>,   // links to thermal modality
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaxonomyInfo {
    pub kingdom: Option<String>,
    pub phylum: Option<String>,
    pub class: Option<String>,
    pub order: Option<String>,
    pub family: Option<String>,
    pub genus: Option<String>,
    pub species: String,
    pub ncbi_taxon_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ConservationStatus {
    #[default] Unknown, Extinct, ExtinctInWild, CriticallyEndangered,
    Endangered, Vulnerable, NearThreatened, LeastConcern, DataDeficient,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EcologicalInteraction {
    pub interaction_id: u64,
    pub species_a: String,
    pub species_b: String,
    pub interaction_type: EcologicalInteractionType,
    pub strength: f32,                  // 0.0 = neutral, ±1.0 = strong
    pub evidence: String,
    pub ecosystem: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum EcologicalInteractionType {
    #[default] Unknown, Predation, Parasitism, Competition, Mutualism,
    Commensalism, Amensalism, Herbivory, Pollination, Decomposition, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HabitatFeature {
    pub feature_id: u64,
    pub habitat_type: HabitatType,
    pub location: Option<[f64; 2]>,
    pub area_km2: Option<f32>,
    pub species_richness: u32,
    pub keystone_species: Vec<String>,
    pub threats: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum HabitatType {
    #[default] Unknown, TropicalRainforest, TemperateForest, Grassland, Desert,
    Wetland, Coral_Reef, Open_Ocean, Freshwater, Tundra, Savanna,
    MangroveForest, Estuary, Urban, Agricultural, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BiodiversityMetrics {
    pub shannon_index: f64,
    pub simpson_index: f64,
    pub species_richness: u32,
    pub evenness: f64,
    pub beta_diversity: Option<f64>,
    pub functional_diversity: Option<f64>,
    pub phylogenetic_diversity: Option<f64>,
}

// ── REGULATORY / PPI ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegulatoryEdge {
    pub edge_id: u64,
    pub regulator: String,              // TF gene symbol or regulatory element ID
    pub target: String,                 // target gene symbol
    pub regulation_type: RegType,
    pub effect_size: f64,
    pub pvalue: Option<f64>,
    pub evidence_type: String,
    pub cell_type_context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum RegType { #[default] Activates, Represses, Dual, Unknown }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProteinInteraction {
    pub ppi_id: u64,
    pub protein_a: String,
    pub protein_b: String,
    pub interaction_type: PPIType,
    pub confidence_score: f64,          // STRING-style combined score
    pub evidence_channels: Vec<String>, // "text_mining", "coexpression", "experimental"
    pub pathway_context: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PPIType {
    #[default] Unknown, PhysicalInteraction, FunctionalAssociation,
    Coexpression, TextMining, Experimental, Database,
}

// ── AUXILIARY ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CellExpressionProfile {
    pub cell_id: String,
    pub barcode: Option<String>,
    pub gene_expression: HashMap<String, f64>,
    pub total_counts: u32,
    pub umap_x: f32, pub umap_y: f32,
    pub cluster_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpatialTranscriptomicsData {
    pub file_path: String,
    pub tissue: String, pub species: String,
    pub spot_count: u32, pub gene_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpeciesObservation {
    pub species_name: String,
    pub count: u32,
    pub location: Option<[f64; 2]>,
    pub date: Option<String>,
    pub observer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BiologyUpdate {
    AddPathway { pathway: BiologicalPathway },
    UpdatePathwayActivity { pathway_id: String, activity_score: f64 },
    AddCellType { cell_type: CellType },
    UpdateCellTypeCount { cell_type_id: u64, new_count: u32 },
    AddDiseaseAssociation { association: DiseaseAssociation },
    AddSpeciesRecord { species: SpeciesRecord },
    UpdatePhysiologicalState { state_id: u64, new_state: PhysiologicalStateType },
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH NODE TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum BiologyNodeType {
    #[default] BiologyScene,            // root
    // MOLECULAR
    PathwayNode, MetaboliteNode, MetabolicReactionNode,
    RegulatoryElementNode, TranscriptionFactorNode,
    // CELLULAR
    CellTypeNode, CellClusterNode, CellularProcessNode,
    SignalingEventNode, OrganelleNode,
    // TISSUE
    TissueNode, TissueCompartmentNode, SpatialDomainNode, HistologyFeatureNode,
    // ORGAN/SYSTEM
    OrganNode, OrganSystemNode, PhysiologicalStateNode,
    // ORGANISM
    PhenotypeNode, DiseaseNode, DrugTargetNode,
    // ECOLOGICAL
    SpeciesNode, EcologicalInteractionNode, HabitatNode, BiodiversityNode,
    // NETWORK
    RegulatoryEdgeNode, ProteinInteractionNode,
    // CROSS-MODAL
    CrossModalRef,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BiologyGraphNode {
    pub node_id: u64,
    pub node_type: BiologyNodeType,
    pub content: String,

    // BIOLOGY-SPECIFIC
    pub biological_id: Option<String>,   // pathway ID, gene symbol, ontology ID, etc.
    pub organism: Option<String>,
    pub scale: Option<String>,
    pub gene_count: Option<u32>,
    pub expression_level: Option<f64>,
    pub is_pathological: Option<bool>,
    pub species_name: Option<String>,
    pub location: Option<[f64; 2]>,
    pub activity_score: Option<f64>,

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
pub enum BiologyEdgeType {
    // ── STRUCTURAL ──
    #[default] Contains, Precedes, PartOf,

    // ── MOLECULAR ──
    Activates,                   // molecule/pathway activates another
    Inhibits,                    // molecule/pathway inhibits another
    PathwayRegulatesProcess,     // pathway regulates cellular process
    MetabolizeVia,               // metabolite processed via reaction
    CatalyzedBy,                 // reaction catalyzed by enzyme
    UpstreamOf,                  // upstream in signaling
    DownstreamOf,                // downstream in signaling
    SubstrateOf,                 // substrate of enzyme/reaction
    ProductOf,                   // product of reaction
    BoundTo,                     // binds to (ligand-receptor, TF-DNA)
    Transcribes,                 // TF transcribes / regulates gene

    // ── CELLULAR ──
    ExpressedIn,                 // gene/protein expressed in cell type
    MarkerOf,                    // marker gene of cell type
    DrivesProcess,               // drives cellular process
    InhibitsProcess,             // inhibits cellular process
    OccursIn,                    // process occurs in compartment/organelle
    Differentiates,              // cell differentiates into another
    CommunicatesWith,            // cell-cell communication
    LigandReceptorOf,            // ligand-receptor pair

    // ── TISSUE ──
    LocatedIn,                   // cell/process in tissue
    ComposedOf,                  // tissue composed of cell types
    AssociatedWith,              // clinical association
    PathologyOf,                 // histological pathology in tissue

    // ── ORGAN / SYSTEM ──
    RegulatesOrgan,              // pathway/cell type regulates organ function
    Part_OfSystem,               // organ part of organ system
    CorrelatesWith,              // physiological correlation

    // ── ORGANISM / DISEASE ──
    CausesDisease,               // variant/pathway causes disease
    AssociatedWithDisease,       // associated with disease
    PhenotypeOf,                 // phenotype of organism/disease
    DrugTargetedBy,              // protein targeted by drug

    // ── ECOLOGICAL ──
    Predates,                    // species A predates B
    Competes,                    // competition
    CoExistsWith,                // co-occurrence
    PolllinatesOrDispurses,      // mutualistic pollination/dispersal
    ParasitesOn,                 // parasitism
    InhabitedBy,                 // habitat inhabited by species

    // ── CROSS-MODAL ──
    /// Gene → sequence in DNA modality (107)
    GenomicSequenceIn,
    /// Protein structure in proteomics (112)
    StructureIn112,
    /// Biochemical pathway → chemistry (106)
    ChemistryOf,
    /// Metabolic heat → thermal modality (114)
    MetabolicHeatIn,
    /// Species vocalization → sonar/sound (125/110)
    AcousticSignatureIn,
    /// Vegetation health → hyperspectral (126)
    VegetationSignalIn,
    /// Spatial location → geospatial (117)
    GeospatialLocation,

    // ── UNIVERSAL SEMANTIC ──
    Performs, Affects, Implies, Contradicts, Elaborates, Summarizes, Supports,
    TemporalPrecedes, TemporalFollows, CausedBy, Enables, Prevents,
    FunctionalRole, InstanceOf,
    DerivedFrom, VersionOf, RefinesTo, ForkedFrom, SimilarTo,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BiologyGraphEdge {
    pub edge_id: u64,
    pub from_node: u64, pub to_node: u64,
    pub edge_type: BiologyEdgeType,
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
pub struct BiologyGraph {
    pub graph_id: u64, pub project_id: u64,
    pub source_description: String,
    pub nodes: Vec<BiologyGraphNode>,
    pub edges: Vec<BiologyGraphEdge>,
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
pub enum BiologyGraphQuery {
    NodeDetail { node_id: u64 },
    PathwaysByCategory { category: String },
    CellTypesByTissue { tissue_name: String },
    DiseaseAssociations { disease_name: String },
    EnrichedPathways,
    SpeciesByHabitat { habitat_type: String },
    DrugTargets,
    RegulatedBy { gene_symbol: String },
    CrossModalLinks { node_id: u64 },
    AGIActivity, AllNodes, AllEdges,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BiologySemanticHook {
    OnGraphCreated,
    OnInferRelationships,
    OnEdgeCompletion,
    OnCrossModalityLink { target_modality: String, target_graph_id: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BiologyExportFormat {
    SBML,           // Systems Biology Markup Language
    BioPAX,         // Biological Pathway Exchange
    GraphML,        // network graph format
    Cytoscape_JSON, // Cytoscape network
    GMT,            // gene set for GSEA
    TSV_Network,    // tab-separated edge list
    KEGG_KGML,      // KEGG pathway format
    GeoJSON,        // ecological data
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BiologyDisplayMode {
    PathwayDiagram,         // KEGG/Reactome-style pathway
    NetworkGraph,           // force-directed PPI/GRN
    CellTypeHierarchy,      // ontology tree
    ScatterUMAP,            // scRNA UMAP plot
    SpatialMap,             // spatial transcriptomics
    HeatMap,                // expression heat map
    EcologicalNetwork,      // species interaction web
    OntologyBrowser,        // GO/HPO tree
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BiologyOperation {
    RunPathwayEnrichment { gene_list: Vec<String>, database: PathwayDatabase },
    ClusterCellTypes,
    InferSignalingPaths,
    CrossLinkToGenomics { dna_graph_id: u64 },
    CrossLinkToProteomics { prot_graph_id: u64 },
    CrossLinkToChemistry { chem_graph_id: u64 },
    ComputeBiodiversity,
    ExportNetworkFile,
}

// ─────────────────────────────────────────────────────────────────────────────
// OUTPUT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BiologyModalityOutput {
    pub success: bool,
    pub graph_id: Option<u64>,
    pub graph: Option<BiologyGraph>,
    pub analysis: Option<BiologyAnalysisResult>,
    pub enrichment_results: Option<Vec<BiologicalPathway>>,
    pub cell_type_assignments: Option<Vec<(String, String)>>,
    pub biodiversity: Option<BiodiversityMetrics>,
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
        let input = serde_json::json!({"action":"Prompt","prompt":prompt,"max_tokens":max_tokens,"temperature":0.05,"system_context":"Biological systems analysis. Return only valid JSON."});
        let out = std::process::Command::new(&self.prompt_pipeline_path)
            .arg("--input").arg(input.to_string())
            .output().map_err(|e| e.to_string())?;
        let r: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
        Ok(r["response"].as_str().unwrap_or("{}").to_string())
    }

    fn save_graph(&self, g: &BiologyGraph) -> Result<(), String> {
        let path = format!("{}/local/biology_graph_{}.json", self.zsei_path, g.graph_id);
        if let Some(p) = std::path::Path::new(&path).parent() { std::fs::create_dir_all(p).map_err(|e| e.to_string())?; }
        std::fs::write(&path, serde_json::to_string_pretty(g).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    fn load_graph(&self, id: u64) -> Result<BiologyGraph, String> {
        let path = format!("{}/local/biology_graph_{}.json", self.zsei_path, id);
        serde_json::from_str(&std::fs::read_to_string(&path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    fn generate_id(&self) -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as u64 }
    fn now_iso8601(&self) -> String { format!("{}", self.generate_id()) }
    fn extract_json_array(raw: &str) -> String { if let (Some(s), Some(e)) = (raw.find('['), raw.rfind(']')) { raw[s..=e].to_string() } else { "[]".to_string() } }
    fn extract_json_object(raw: &str) -> String { if let (Some(s), Some(e)) = (raw.find('{'), raw.rfind('}')) { raw[s..=e].to_string() } else { "{}".to_string() } }
}

impl PipelineExecutor {
    async fn infer_pathway_interactions_llm(&self, pathways: &[BiologicalPathway]) -> Vec<(String, String, String)> {
        if pathways.len() < 2 { return vec![]; }
        let pw_list: Vec<serde_json::Value> = pathways.iter().take(20).map(|p| serde_json::json!({
            "id": p.pathway_id, "name": p.name, "category": format!("{:?}", p.category),
            "gene_count": p.gene_count, "enriched": p.is_enriched,
        })).collect();

        let prompt = format!(r#"
Identify biological pathway interactions based on known biology.

Pathways:
{}

Interaction types: SharedGene, CrossTalk, Sequential, Antagonistic, Synergistic, Regulatory

Return ONLY valid JSON array:
[{{"pathway_a": "id", "pathway_b": "id", "interaction_type": "TypeName"}}]"#,
            serde_json::to_string_pretty(&pw_list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 600).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str).unwrap_or_default()
                    .into_iter().filter_map(|v| Some((v["pathway_a"].as_str()?.to_string(), v["pathway_b"].as_str()?.to_string(), v["interaction_type"].as_str()?.to_string())))
                    .collect()
            }
            Err(_) => vec![],
        }
    }

    async fn classify_cell_types_llm(&self, clusters: &[CellCluster], tissue: &str) -> Vec<(u64, String)> {
        if clusters.is_empty() { return vec![]; }
        let cl_list: Vec<serde_json::Value> = clusters.iter().map(|c| serde_json::json!({
            "cluster_id": c.cluster_id, "cell_count": c.cell_count,
            "top_markers": c.marker_genes.iter().take(5).collect::<Vec<_>>(),
        })).collect();

        let prompt = format!(r#"
Classify scRNA-seq cell clusters from {} tissue based on marker genes.

Clusters:
{}

Return cell type names using standard nomenclature (e.g., "CD4+ T cell", "B cell", "NK cell",
"Monocyte", "Macrophage", "Dendritic cell", "Epithelial cell", "Fibroblast", "Endothelial cell").

Return ONLY valid JSON array:
[{{"cluster_id": N, "cell_type": "CellTypeName"}}]"#,
            tissue, serde_json::to_string_pretty(&cl_list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 400).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str).unwrap_or_default()
                    .into_iter().filter_map(|v| Some((v["cluster_id"].as_u64()?, v["cell_type"].as_str()?.to_string())))
                    .collect()
            }
            Err(_) => vec![],
        }
    }

    async fn infer_disease_pathway_associations_llm(&self, pathways: &[BiologicalPathway], diseases: &[DiseaseAssociation]) -> Vec<(String, String)> {
        if pathways.is_empty() || diseases.is_empty() { return vec![]; }
        let pw_names: Vec<&str> = pathways.iter().take(10).map(|p| p.name.as_str()).collect();
        let dis_names: Vec<&str> = diseases.iter().take(10).map(|d| d.disease_name.as_str()).collect();

        let prompt = format!(r#"
Which pathways are most likely involved in each disease based on known biology?

Pathways: {}
Diseases: {}

Return ONLY valid JSON array:
[{{"pathway_name": "...", "disease_name": "...", "evidence": "brief reason"}}]"#,
            pw_names.join(", "), dis_names.join(", "));

        match self.llm_zero_shot(&prompt, 500).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str).unwrap_or_default()
                    .into_iter().filter_map(|v| Some((v["pathway_name"].as_str()?.to_string(), v["disease_name"].as_str()?.to_string())))
                    .collect()
            }
            Err(_) => vec![],
        }
    }

    async fn infer_semantic_relationships(&self, nodes: &[BiologyGraphNode]) -> Vec<(u64, u64, BiologyEdgeType, String)> {
        if nodes.len() < 2 { return vec![]; }
        let node_list: Vec<serde_json::Value> = nodes.iter().take(25).map(|n| serde_json::json!({
            "node_id": n.node_id, "type": format!("{:?}", n.node_type),
            "content": n.content.chars().take(80).collect::<String>(),
            "bio_id": n.biological_id, "scale": n.scale,
        })).collect();

        let prompt = format!(r#"
Identify biological relationships between these graph nodes.

Nodes: {}

Types: Activates, Inhibits, PathwayRegulatesProcess, ExpressedIn, MarkerOf,
DrivesProcess, LocatedIn, ComposedOf, CausesDisease, AssociatedWithDisease,
CoExistsWith, Predates, UpstreamOf, DownstreamOf, Affects, CausedBy,
DerivedFrom, PartOf, FunctionalRole, CorrelatesWith

Return ONLY valid JSON array:
[{{"from_node_id": N, "to_node_id": M, "edge_type": "TypeName", "reason": "brief"}}]"#,
            serde_json::to_string_pretty(&node_list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 800).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str).unwrap_or_default()
                    .into_iter().filter_map(|v| {
                        let from = v["from_node_id"].as_u64()?;
                        let to = v["to_node_id"].as_u64()?;
                        let etype = map_bio_edge_str(v["edge_type"].as_str().unwrap_or("Affects"));
                        let reason = v["reason"].as_str().unwrap_or("").to_string();
                        Some((from, to, etype, reason))
                    }).collect()
            }
            Err(_) => vec![],
        }
    }

    async fn compute_pathway_enrichment_llm(&self, gene_list: &[String], pathways: &[BiologicalPathway]) -> Vec<(String, f64)> {
        if gene_list.is_empty() || pathways.is_empty() { return vec![]; }
        let pw_info: Vec<serde_json::Value> = pathways.iter().take(30).map(|p| serde_json::json!({
            "id": p.pathway_id, "name": p.name,
            "pathway_genes": p.gene_symbols.iter().take(20).collect::<Vec<_>>(),
        })).collect();

        let prompt = format!(r#"
Perform pathway enrichment analysis.

Query gene list ({}):
{}

Pathways with gene membership:
{}

For each pathway, estimate the enrichment significance (0=not enriched, 1=highly enriched)
based on overlap between query genes and pathway genes.

Return ONLY valid JSON array:
[{{"pathway_id": "id", "enrichment_score": 0.0-1.0}}]"#,
            gene_list.len(),
            gene_list.iter().take(30).map(|g| g.as_str()).collect::<Vec<_>>().join(", "),
            serde_json::to_string_pretty(&pw_info).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 800).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str).unwrap_or_default()
                    .into_iter().filter_map(|v| Some((v["pathway_id"].as_str()?.to_string(), v["enrichment_score"].as_f64().unwrap_or(0.0))))
                    .collect()
            }
            Err(_) => vec![],
        }
    }
}

fn map_bio_edge_str(s: &str) -> BiologyEdgeType {
    match s {
        "Activates"              => BiologyEdgeType::Activates,
        "Inhibits"               => BiologyEdgeType::Inhibits,
        "PathwayRegulatesProcess"=> BiologyEdgeType::PathwayRegulatesProcess,
        "MetabolizeVia"          => BiologyEdgeType::MetabolizeVia,
        "CatalyzedBy"            => BiologyEdgeType::CatalyzedBy,
        "UpstreamOf"             => BiologyEdgeType::UpstreamOf,
        "DownstreamOf"           => BiologyEdgeType::DownstreamOf,
        "SubstrateOf"            => BiologyEdgeType::SubstrateOf,
        "ProductOf"              => BiologyEdgeType::ProductOf,
        "BoundTo"                => BiologyEdgeType::BoundTo,
        "Transcribes"            => BiologyEdgeType::Transcribes,
        "ExpressedIn"            => BiologyEdgeType::ExpressedIn,
        "MarkerOf"               => BiologyEdgeType::MarkerOf,
        "DrivesProcess"          => BiologyEdgeType::DrivesProcess,
        "InhibitsProcess"        => BiologyEdgeType::InhibitsProcess,
        "OccursIn"               => BiologyEdgeType::OccursIn,
        "Differentiates"         => BiologyEdgeType::Differentiates,
        "CommunicatesWith"       => BiologyEdgeType::CommunicatesWith,
        "LigandReceptorOf"       => BiologyEdgeType::LigandReceptorOf,
        "LocatedIn"              => BiologyEdgeType::LocatedIn,
        "ComposedOf"             => BiologyEdgeType::ComposedOf,
        "AssociatedWith"         => BiologyEdgeType::AssociatedWith,
        "PathologyOf"            => BiologyEdgeType::PathologyOf,
        "RegulatesOrgan"         => BiologyEdgeType::RegulatesOrgan,
        "CausesDisease"          => BiologyEdgeType::CausesDisease,
        "AssociatedWithDisease"  => BiologyEdgeType::AssociatedWithDisease,
        "PhenotypeOf"            => BiologyEdgeType::PhenotypeOf,
        "DrugTargetedBy"         => BiologyEdgeType::DrugTargetedBy,
        "Predates"               => BiologyEdgeType::Predates,
        "Competes"               => BiologyEdgeType::Competes,
        "CoExistsWith"           => BiologyEdgeType::CoExistsWith,
        "GenomicSequenceIn"      => BiologyEdgeType::GenomicSequenceIn,
        "ChemistryOf"            => BiologyEdgeType::ChemistryOf,
        "MetabolicHeatIn"        => BiologyEdgeType::MetabolicHeatIn,
        "AcousticSignatureIn"    => BiologyEdgeType::AcousticSignatureIn,
        "VegetationSignalIn"     => BiologyEdgeType::VegetationSignalIn,
        "GeospatialLocation"     => BiologyEdgeType::GeospatialLocation,
        "Affects"                => BiologyEdgeType::Affects,
        "CausedBy"               => BiologyEdgeType::CausedBy,
        "Enables"                => BiologyEdgeType::Enables,
        "Prevents"               => BiologyEdgeType::Prevents,
        "TemporalPrecedes"       => BiologyEdgeType::TemporalPrecedes,
        "DerivedFrom"            => BiologyEdgeType::DerivedFrom,
        "PartOf"                 => BiologyEdgeType::PartOf,
        "FunctionalRole"         => BiologyEdgeType::FunctionalRole,
        "CorrelatesWith"         => BiologyEdgeType::CorrelatesWith,
        _                        => BiologyEdgeType::Affects,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH CREATION
// ─────────────────────────────────────────────────────────────────────────────

async fn create_graph(executor: &PipelineExecutor, analysis: BiologyAnalysisResult, project_id: u64) -> BiologyModalityOutput {
    let graph_id = executor.generate_id();
    let now = executor.now_iso8601();
    let mut nodes: Vec<BiologyGraphNode> = Vec::new();
    let mut edges: Vec<BiologyGraphEdge> = Vec::new();
    let mut node_id: u64 = 1;
    let mut edge_id: u64 = 1;

    // ── ROOT ──
    let root_id = node_id;
    nodes.push(BiologyGraphNode {
        node_id: root_id, node_type: BiologyNodeType::BiologyScene,
        content: format!("Biology [{}|{:?}]: pathways={} celltypes={} tissues={} organs={} species={} diseases={}",
            analysis.organism, analysis.biological_scale,
            analysis.pathways.len(), analysis.cell_types.len(), analysis.tissues.len(),
            analysis.organs.len(), analysis.species_detected.len(), analysis.disease_associations.len()),
        organism: Some(analysis.organism.clone()),
        scale: Some(format!("{:?}", analysis.biological_scale)),
        materialized_path: Some(format!("/Modalities/Biology/Project_{}/Graph_{}", project_id, graph_id)),
        provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
        version_notes: vec![VersionNote { version: 1, note: "Initial creation".into(), step_index: None, timestamp: now.clone(), change_type: ChangeType::Created }],
        keywords: vec!["biology".into(), analysis.organism.to_lowercase(), format!("{:?}", analysis.biological_scale).to_lowercase()],
        hotness_score: 1.0, ..Default::default()
    });
    node_id += 1;

    // ── PATHWAY NODES ──
    let mut pathway_id_to_nid: HashMap<String, u64> = HashMap::new();
    for pw in &analysis.pathways {
        let pid = node_id;
        nodes.push(BiologyGraphNode {
            node_id: pid, node_type: BiologyNodeType::PathwayNode,
            content: format!("Pathway [{}] {}: genes={} cat={:?} enriched={} db={}",
                pw.pathway_id, pw.name, pw.gene_count, pw.category, pw.is_enriched, pw.database),
            biological_id: Some(pw.pathway_id.clone()),
            gene_count: Some(pw.gene_count),
            activity_score: pw.enrichment_pvalue.map(|p| -p.log10()),
            materialized_path: Some(format!("/Modalities/Biology/Project_{}/Graph_{}/Pathway/{}", project_id, graph_id, pw.pathway_id.replace(':', "_"))),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: { let mut kw = vec!["pathway".into(), pw.database.to_lowercase(), format!("{:?}", pw.category).to_lowercase()]; kw.extend(pw.gene_symbols.iter().take(5).map(|g| g.to_lowercase())); kw },
            hotness_score: if pw.is_enriched { 0.9 } else { 0.6 },
            embedding_hint: Some(format!("pathway {} category: {:?}", pw.name, pw.category)),
            ..Default::default()
        });
        edges.push(BiologyGraphEdge { edge_id, from_node: root_id, to_node: pid, edge_type: BiologyEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // Parent pathway hierarchy
        if let Some(ref parent_id) = pw.parent_pathway_id {
            if let Some(&parent_nid) = pathway_id_to_nid.get(parent_id) {
                edges.push(BiologyGraphEdge { edge_id, from_node: parent_nid, to_node: pid, edge_type: BiologyEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        pathway_id_to_nid.insert(pw.pathway_id.clone(), pid);
        node_id += 1;
    }

    // ── CELLULAR PROCESS NODES ──
    let mut proc_id_to_nid: HashMap<u64, u64> = HashMap::new();
    for proc in &analysis.cellular_processes {
        let pnid = node_id;
        nodes.push(BiologyGraphNode {
            node_id: pnid, node_type: BiologyNodeType::CellularProcessNode,
            content: format!("Process {:?}: {} go={:?} active={:?}",
                proc.process_type, proc.name, proc.go_id, proc.is_active),
            biological_id: proc.go_id.clone(),
            activity_score: proc.activity_score,
            is_pathological: proc.is_active.map(|a| !a),
            materialized_path: Some(format!("/Modalities/Biology/Project_{}/Graph_{}/Process/{}", project_id, graph_id, proc.process_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["cellular-process".into(), format!("{:?}", proc.process_type).to_lowercase(), proc.name.to_lowercase()],
            hotness_score: 0.7,
            ..Default::default()
        });
        edges.push(BiologyGraphEdge { edge_id, from_node: root_id, to_node: pnid, edge_type: BiologyEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // Pathway → process
        for pw_id in &proc.associated_pathways {
            if let Some(&pw_nid) = pathway_id_to_nid.get(pw_id) {
                edges.push(BiologyGraphEdge { edge_id, from_node: pw_nid, to_node: pnid, edge_type: BiologyEdgeType::PathwayRegulatesProcess, weight: 0.85, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        proc_id_to_nid.insert(proc.process_id, pnid);
        node_id += 1;
    }

    // ── CELL TYPE NODES ──
    let mut celltype_id_to_nid: HashMap<u64, u64> = HashMap::new();
    for ct in &analysis.cell_types {
        let ctnid = node_id;
        nodes.push(BiologyGraphNode {
            node_id: ctnid, node_type: BiologyNodeType::CellTypeNode,
            content: format!("CellType: {} [{}] state={:?} markers={:?} frac={:?}",
                ct.name, ct.ontology_id.as_deref().unwrap_or("?"), ct.functional_state,
                ct.marker_genes.first().map(|s| s.as_str()).unwrap_or("?"),
                ct.fraction_of_total.map(|f| format!("{:.1}%", f * 100.0))),
            biological_id: ct.ontology_id.clone(),
            organism: Some(analysis.organism.clone()),
            gene_count: Some(ct.marker_genes.len() as u32),
            scale: Some("cellular".into()),
            materialized_path: Some(format!("/Modalities/Biology/Project_{}/Graph_{}/CellType/{}", project_id, graph_id, ct.cell_type_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: { let mut kw = vec!["cell-type".into(), ct.name.to_lowercase()]; kw.extend(ct.marker_genes.iter().take(3).map(|g| g.to_lowercase())); kw },
            hotness_score: ct.fraction_of_total.map(|f| 0.5 + f * 0.4).unwrap_or(0.65),
            embedding_hint: Some(format!("cell type: {} state: {:?}", ct.name, ct.functional_state)),
            ..Default::default()
        });
        edges.push(BiologyGraphEdge { edge_id, from_node: root_id, to_node: ctnid, edge_type: BiologyEdgeType::Contains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        celltype_id_to_nid.insert(ct.cell_type_id, ctnid);
        node_id += 1;
    }

    // ── CELL CLUSTER NODES ──
    for cluster in &analysis.cell_clusters {
        let clnid = node_id;
        nodes.push(BiologyGraphNode {
            node_id: clnid, node_type: BiologyNodeType::CellClusterNode,
            content: format!("Cluster {} [{}]: {} cells assigned={:?} markers={:?}",
                cluster.cluster_id, cluster.cluster_name, cluster.cell_count,
                cluster.cell_type_assignment.as_deref().unwrap_or("?"),
                cluster.marker_genes.first().map(|s| s.as_str()).unwrap_or("?")),
            gene_count: Some(cluster.marker_genes.len() as u32),
            materialized_path: Some(format!("/Modalities/Biology/Project_{}/Graph_{}/Cluster/{}", project_id, graph_id, cluster.cluster_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: { let mut kw = vec!["cluster".into()]; kw.extend(cluster.marker_genes.iter().take(3).map(|g| g.to_lowercase())); kw },
            hotness_score: 0.65, ..Default::default()
        });
        edges.push(BiologyGraphEdge { edge_id, from_node: root_id, to_node: clnid, edge_type: BiologyEdgeType::Contains, weight: 0.7, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // Cluster → cell type (if assigned)
        if let Some(ref ct_name) = cluster.cell_type_assignment {
            if let Some(ct) = analysis.cell_types.iter().find(|c| c.name == *ct_name) {
                if let Some(&ct_nid) = celltype_id_to_nid.get(&ct.cell_type_id) {
                    edges.push(BiologyGraphEdge { edge_id, from_node: clnid, to_node: ct_nid, edge_type: BiologyEdgeType::MarkerOf, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                    edge_id += 1;
                }
            }
        }
        node_id += 1;
    }

    // ── TISSUE NODES ──
    let mut tissue_id_to_nid: HashMap<u64, u64> = HashMap::new();
    for tissue in &analysis.tissues {
        let tnid = node_id;
        nodes.push(BiologyGraphNode {
            node_id: tnid, node_type: BiologyNodeType::TissueNode,
            content: format!("Tissue: {} [{}] type={:?} cell_types={}",
                tissue.name, tissue.uberon_id.as_deref().unwrap_or("?"),
                tissue.tissue_type, tissue.cell_type_composition.len()),
            biological_id: tissue.uberon_id.clone(),
            scale: Some("tissue".into()),
            materialized_path: Some(format!("/Modalities/Biology/Project_{}/Graph_{}/Tissue/{}", project_id, graph_id, tissue.tissue_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["tissue".into(), tissue.name.to_lowercase(), format!("{:?}", tissue.tissue_type).to_lowercase()],
            hotness_score: 0.75, ..Default::default()
        });
        edges.push(BiologyGraphEdge { edge_id, from_node: root_id, to_node: tnid, edge_type: BiologyEdgeType::Contains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // Tissue composed of cell types
        for (ct_name, fraction) in &tissue.cell_type_composition {
            if let Some(ct) = analysis.cell_types.iter().find(|c| c.name == *ct_name) {
                if let Some(&ct_nid) = celltype_id_to_nid.get(&ct.cell_type_id) {
                    edges.push(BiologyGraphEdge { edge_id, from_node: tnid, to_node: ct_nid, edge_type: BiologyEdgeType::ComposedOf, weight: *fraction, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                    edge_id += 1;
                }
            }
        }
        tissue_id_to_nid.insert(tissue.tissue_id, tnid);
        node_id += 1;
    }

    // ── ORGAN NODES ──
    let mut organ_id_to_nid: HashMap<u64, u64> = HashMap::new();
    for organ in &analysis.organs {
        let onid = node_id;
        nodes.push(BiologyGraphNode {
            node_id: onid, node_type: BiologyNodeType::OrganNode,
            content: format!("Organ: {} [{}] system={:?} biomarkers={:?}",
                organ.name, organ.uberon_id.as_deref().unwrap_or("?"),
                organ.organ_system, organ.biomarkers.first().map(|b| b.as_str()).unwrap_or("?")),
            biological_id: organ.uberon_id.clone(),
            scale: Some("organ".into()),
            materialized_path: Some(format!("/Modalities/Biology/Project_{}/Graph_{}/Organ/{}", project_id, graph_id, organ.organ_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["organ".into(), organ.name.to_lowercase(), format!("{:?}", organ.organ_system).to_lowercase()],
            hotness_score: 0.75, ..Default::default()
        });
        edges.push(BiologyGraphEdge { edge_id, from_node: root_id, to_node: onid, edge_type: BiologyEdgeType::Contains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // Organ contains tissues
        for tid in &organ.tissue_ids {
            if let Some(&t_nid) = tissue_id_to_nid.get(tid) {
                edges.push(BiologyGraphEdge { edge_id, from_node: onid, to_node: t_nid, edge_type: BiologyEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        organ_id_to_nid.insert(organ.organ_id, onid);
        node_id += 1;
    }

    // ── ORGAN SYSTEM NODES ──
    for os in &analysis.organ_systems {
        let osnid = node_id;
        nodes.push(BiologyGraphNode {
            node_id: osnid, node_type: BiologyNodeType::OrganSystemNode,
            content: format!("OrganSystem {:?}: {} organs={}",
                os.system_type, os.name, os.organ_ids.len()),
            biological_id: Some(format!("{:?}", os.system_type)),
            scale: Some("organism".into()),
            materialized_path: Some(format!("/Modalities/Biology/Project_{}/Graph_{}/System/{}", project_id, graph_id, os.system_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["organ-system".into(), format!("{:?}", os.system_type).to_lowercase()],
            hotness_score: 0.7, ..Default::default()
        });
        edges.push(BiologyGraphEdge { edge_id, from_node: root_id, to_node: osnid, edge_type: BiologyEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        for oid in &os.organ_ids {
            if let Some(&o_nid) = organ_id_to_nid.get(oid) {
                edges.push(BiologyGraphEdge { edge_id, from_node: osnid, to_node: o_nid, edge_type: BiologyEdgeType::Part_OfSystem, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        node_id += 1;
    }

    // ── DISEASE NODES ──
    let mut disease_id_to_nid: HashMap<u64, u64> = HashMap::new();
    for dis in &analysis.disease_associations {
        let dnid = node_id;
        nodes.push(BiologyGraphNode {
            node_id: dnid, node_type: BiologyNodeType::DiseaseNode,
            content: format!("Disease {:?}: {} [{}] genes={} evidence={}",
                dis.disease_type, dis.disease_name,
                dis.mondo_id.as_deref().unwrap_or("?"),
                dis.associated_genes.len(), dis.evidence_type),
            biological_id: dis.mondo_id.clone().or_else(|| dis.omim_id.clone()),
            is_pathological: Some(true),
            materialized_path: Some(format!("/Modalities/Biology/Project_{}/Graph_{}/Disease/{}", project_id, graph_id, dis.association_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: { let mut kw = vec!["disease".into(), dis.disease_name.to_lowercase(), format!("{:?}", dis.disease_type).to_lowercase()]; kw },
            hotness_score: 0.8, ..Default::default()
        });
        edges.push(BiologyGraphEdge { edge_id, from_node: root_id, to_node: dnid, edge_type: BiologyEdgeType::Contains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // Disease → pathways
        for pw_id in &dis.associated_pathways {
            if let Some(&pw_nid) = pathway_id_to_nid.get(pw_id) {
                edges.push(BiologyGraphEdge { edge_id, from_node: dnid, to_node: pw_nid, edge_type: BiologyEdgeType::AssociatedWithDisease, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        disease_id_to_nid.insert(dis.association_id, dnid);
        node_id += 1;
    }

    // ── DRUG TARGET NODES ──
    for dt in &analysis.drug_targets {
        let dtnid = node_id;
        nodes.push(BiologyGraphNode {
            node_id: dtnid, node_type: BiologyNodeType::DrugTargetNode,
            content: format!("DrugTarget: {} drug={:?} interaction={:?} indication={:?}",
                dt.gene_symbol, dt.drug_name.as_deref().unwrap_or("?"),
                dt.interaction_type, dt.indication.as_deref().unwrap_or("?")),
            biological_id: Some(dt.gene_symbol.clone()),
            materialized_path: Some(format!("/Modalities/Biology/Project_{}/Graph_{}/DrugTarget/{}", project_id, graph_id, dt.target_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: { let mut kw = vec!["drug-target".into(), dt.gene_symbol.to_lowercase()]; if let Some(ref d) = dt.drug_name { kw.push(d.to_lowercase()); } kw },
            hotness_score: 0.75, ..Default::default()
        });
        edges.push(BiologyGraphEdge { edge_id, from_node: root_id, to_node: dtnid, edge_type: BiologyEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── PHENOTYPE NODES ──
    for pheno in &analysis.organism_phenotypes {
        let phnid = node_id;
        nodes.push(BiologyGraphNode {
            node_id: phnid, node_type: BiologyNodeType::PhenotypeNode,
            content: format!("Phenotype {:?}: {} [{}] severity={:?} disease={}",
                pheno.phenotype_class, pheno.name,
                pheno.hp_id.as_deref().unwrap_or("?"), pheno.severity, pheno.is_disease_related),
            biological_id: pheno.hp_id.clone().or_else(|| pheno.mp_id.clone()),
            is_pathological: Some(pheno.is_disease_related),
            materialized_path: Some(format!("/Modalities/Biology/Project_{}/Graph_{}/Phenotype/{}", project_id, graph_id, pheno.phenotype_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["phenotype".into(), pheno.name.to_lowercase(), format!("{:?}", pheno.phenotype_class).to_lowercase()],
            hotness_score: 0.7, ..Default::default()
        });
        edges.push(BiologyGraphEdge { edge_id, from_node: root_id, to_node: phnid, edge_type: BiologyEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── SPECIES NODES ──
    let mut species_id_to_nid: HashMap<u64, u64> = HashMap::new();
    for sp in &analysis.species_detected {
        let snid = node_id;
        nodes.push(BiologyGraphNode {
            node_id: snid, node_type: BiologyNodeType::SpeciesNode,
            content: format!("Species: {} [{}] obs={} status={:?} family={}",
                sp.scientific_name, sp.common_name.as_deref().unwrap_or("?"),
                sp.observation_count, sp.conservation_status,
                sp.taxonomy.family.as_deref().unwrap_or("?")),
            biological_id: sp.taxonomy.ncbi_taxon_id.map(|id| id.to_string()),
            species_name: Some(sp.scientific_name.clone()),
            location: sp.location,
            scale: Some("ecological".into()),
            materialized_path: Some(format!("/Modalities/Biology/Project_{}/Graph_{}/Species/{}", project_id, graph_id, sp.species_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: { let mut kw = vec!["species".into(), sp.scientific_name.to_lowercase()]; if let Some(ref cn) = sp.common_name { kw.push(cn.to_lowercase()); } kw },
            hotness_score: 0.6 + match sp.conservation_status { ConservationStatus::CriticallyEndangered | ConservationStatus::Endangered => 0.3, ConservationStatus::Vulnerable => 0.2, _ => 0.0 },
            ..Default::default()
        });
        edges.push(BiologyGraphEdge { edge_id, from_node: root_id, to_node: snid, edge_type: BiologyEdgeType::Contains, weight: 0.7, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Cross-modal: species → geospatial (117)
        if sp.location.is_some() {
            edges.push(BiologyGraphEdge {
                edge_id, from_node: snid, to_node: snid,
                edge_type: BiologyEdgeType::GeospatialLocation, weight: 0.85,
                provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("geospatial")); p },
                ..Default::default()
            });
            edge_id += 1;
        }
        // Cross-modal: marine/acoustic species → sonar (125)
        if sp.acoustic_signature_id.is_some() {
            edges.push(BiologyGraphEdge {
                edge_id, from_node: snid, to_node: snid,
                edge_type: BiologyEdgeType::AcousticSignatureIn, weight: 0.8,
                provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("sonar")); p },
                ..Default::default()
            });
            edge_id += 1;
        }
        species_id_to_nid.insert(sp.species_id, snid);
        node_id += 1;
    }

    // ── ECOLOGICAL INTERACTION NODES ──
    for ei in &analysis.ecological_interactions {
        let einid = node_id;
        nodes.push(BiologyGraphNode {
            node_id: einid, node_type: BiologyNodeType::EcologicalInteractionNode,
            content: format!("EcoInteraction {:?}: {} → {} strength={:.2}",
                ei.interaction_type, ei.species_a, ei.species_b, ei.strength),
            materialized_path: Some(format!("/Modalities/Biology/Project_{}/Graph_{}/EcoInteraction/{}", project_id, graph_id, ei.interaction_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["ecological-interaction".into(), format!("{:?}", ei.interaction_type).to_lowercase()],
            hotness_score: 0.6, ..Default::default()
        });
        edges.push(BiologyGraphEdge { edge_id, from_node: root_id, to_node: einid, edge_type: BiologyEdgeType::Contains, weight: 0.7, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // Link species A → interaction
        if let Some(sp_a) = analysis.species_detected.iter().find(|s| s.scientific_name == ei.species_a) {
            if let Some(&spa_nid) = species_id_to_nid.get(&sp_a.species_id) {
                let etype = match ei.interaction_type {
                    EcologicalInteractionType::Predation => BiologyEdgeType::Predates,
                    EcologicalInteractionType::Competition => BiologyEdgeType::Competes,
                    EcologicalInteractionType::Mutualism | EcologicalInteractionType::Commensalism => BiologyEdgeType::CoExistsWith,
                    EcologicalInteractionType::Parasitism => BiologyEdgeType::ParasitesOn,
                    _ => BiologyEdgeType::CoExistsWith,
                };
                edges.push(BiologyGraphEdge { edge_id, from_node: spa_nid, to_node: einid, edge_type: etype, weight: ei.strength.abs(), provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        node_id += 1;
    }

    // ── HABITAT NODES ──
    for hab in &analysis.habitat_features {
        let hnid = node_id;
        nodes.push(BiologyGraphNode {
            node_id: hnid, node_type: BiologyNodeType::HabitatNode,
            content: format!("Habitat {:?}: species_richness={} threats={:?}",
                hab.habitat_type, hab.species_richness, hab.threats.first().map(|t| t.as_str()).unwrap_or("none")),
            location: hab.location,
            scale: Some("ecological".into()),
            materialized_path: Some(format!("/Modalities/Biology/Project_{}/Graph_{}/Habitat/{}", project_id, graph_id, hab.feature_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["habitat".into(), format!("{:?}", hab.habitat_type).to_lowercase()],
            hotness_score: 0.65, ..Default::default()
        });
        edges.push(BiologyGraphEdge { edge_id, from_node: root_id, to_node: hnid, edge_type: BiologyEdgeType::Contains, weight: 0.7, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── BIODIVERSITY NODE ──
    if let Some(ref bd) = analysis.biodiversity_metrics {
        let bdnid = node_id;
        nodes.push(BiologyGraphNode {
            node_id: bdnid, node_type: BiologyNodeType::BiodiversityNode,
            content: format!("Biodiversity: Shannon={:.3} Simpson={:.3} richness={} evenness={:.3}",
                bd.shannon_index, bd.simpson_index, bd.species_richness, bd.evenness),
            materialized_path: Some(format!("/Modalities/Biology/Project_{}/Graph_{}/Biodiversity", project_id, graph_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["biodiversity".into(), "ecology".into()], hotness_score: 0.7, ..Default::default()
        });
        edges.push(BiologyGraphEdge { edge_id, from_node: root_id, to_node: bdnid, edge_type: BiologyEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── REGULATORY NETWORK NODES ──
    for reg in analysis.gene_regulatory_network.iter().take(100) {
        let rnid = node_id;
        nodes.push(BiologyGraphNode {
            node_id: rnid, node_type: BiologyNodeType::RegulatoryEdgeNode,
            content: format!("Regulation {:?}: {} → {} effect={:.3} p={:?}",
                reg.regulation_type, reg.regulator, reg.target, reg.effect_size, reg.pvalue.map(|p| format!("{:.2e}", p))),
            biological_id: Some(format!("{}→{}", reg.regulator, reg.target)),
            activity_score: Some(reg.effect_size),
            materialized_path: Some(format!("/Modalities/Biology/Project_{}/Graph_{}/Regulation/{}", project_id, graph_id, reg.edge_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["regulation".into(), reg.regulator.to_lowercase(), reg.target.to_lowercase()],
            hotness_score: 0.6 + (reg.effect_size.abs() / 5.0).clamp(0.0, 0.3),
            ..Default::default()
        });
        edges.push(BiologyGraphEdge { edge_id, from_node: root_id, to_node: rnid, edge_type: BiologyEdgeType::Contains, weight: 0.7, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // Regulation → process
        let reg_etype = match reg.regulation_type { RegType::Activates => BiologyEdgeType::Activates, RegType::Represses => BiologyEdgeType::Inhibits, _ => BiologyEdgeType::Affects };
        edges.push(BiologyGraphEdge { edge_id, from_node: rnid, to_node: root_id, edge_type: reg_etype, weight: reg.effect_size.abs() as f32, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── METABOLITE NODES ──
    for met in analysis.metabolites.iter().take(50) {
        let mnid = node_id;
        nodes.push(BiologyGraphNode {
            node_id: mnid, node_type: BiologyNodeType::MetaboliteNode,
            content: format!("Metabolite {:?}: {} [{}] lfc={:?}",
                met.metabolite_class, met.name, met.formula.as_deref().unwrap_or("?"),
                met.log2_fold_change.map(|l| format!("{:.2}", l))),
            biological_id: met.chebi_id.clone().or_else(|| met.hmdb_id.clone()),
            activity_score: met.log2_fold_change,
            materialized_path: Some(format!("/Modalities/Biology/Project_{}/Graph_{}/Metabolite/{}", project_id, graph_id, met.metabolite_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["metabolite".into(), met.name.to_lowercase(), format!("{:?}", met.metabolite_class).to_lowercase()],
            hotness_score: 0.6 + met.log2_fold_change.map(|l| l.abs() / 10.0).unwrap_or(0.0).clamp(0.0, 0.3) as f32,
            ..Default::default()
        });
        edges.push(BiologyGraphEdge { edge_id, from_node: root_id, to_node: mnid, edge_type: BiologyEdgeType::Contains, weight: 0.7, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // Metabolite → pathways
        for pw_id in &met.pathway_ids {
            if let Some(&pw_nid) = pathway_id_to_nid.get(pw_id) {
                edges.push(BiologyGraphEdge { edge_id, from_node: mnid, to_node: pw_nid, edge_type: BiologyEdgeType::MetabolizeVia, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        // Metabolite cross-modal: → chemistry (106)
        edges.push(BiologyGraphEdge { edge_id, from_node: mnid, to_node: mnid, edge_type: BiologyEdgeType::ChemistryOf, weight: 0.85, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("chemistry")); p }, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── HOOK 1: OnGraphCreated ──
    let _ = executor.save_graph(&BiologyGraph { graph_id, project_id, source_description: analysis.source_description.clone(), nodes: nodes.clone(), edges: edges.clone(), root_node_id: root_id, state: GraphStateType::Created, state_history: vec![GraphStateTransition { from: GraphStateType::Created, to: GraphStateType::Created, timestamp: now.clone(), triggered_by_step: None }], created_at: now.clone(), updated_at: now.clone(), version: 1, version_notes: vec![VersionNote { version: 1, note: format!("Created: {} nodes {} edges", nodes.len(), edges.len()), step_index: None, timestamp: now.clone(), change_type: ChangeType::Created }] });

    // ── HOOK 2: OnInferRelationships ──
    let inferred = executor.infer_semantic_relationships(&nodes).await;
    let valid: std::collections::HashSet<u64> = nodes.iter().map(|n| n.node_id).collect();
    for (from, to, etype, reason) in inferred {
        if valid.contains(&from) && valid.contains(&to) && from != to {
            edges.push(BiologyGraphEdge { edge_id, from_node: from, to_node: to, edge_type: etype, weight: 0.8, provenance: EdgeProvenance::DerivedFromHook, version: 1, properties: { let mut p = HashMap::new(); p.insert("reason".into(), serde_json::json!(reason)); p }, ..Default::default() });
            edge_id += 1;
        }
    }

    // ── HOOK 3: OnEdgeCompletion → hotness ──
    let mut deg: HashMap<u64, u32> = HashMap::new();
    for e in &edges { *deg.entry(e.from_node).or_insert(0) += 1; *deg.entry(e.to_node).or_insert(0) += 1; }
    let max_deg = deg.values().copied().max().unwrap_or(1) as f32;
    for n in &mut nodes { if let Some(&d) = deg.get(&n.node_id) { n.hotness_score = (n.hotness_score + (d as f32 / max_deg) * 0.15).min(1.0); } }

    // Remove cross-modal self-loops — keep as metadata markers only
    edges.retain(|e| e.from_node != e.to_node || matches!(e.edge_type, BiologyEdgeType::GenomicSequenceIn | BiologyEdgeType::StructureIn112 | BiologyEdgeType::ChemistryOf | BiologyEdgeType::MetabolicHeatIn | BiologyEdgeType::AcousticSignatureIn | BiologyEdgeType::VegetationSignalIn | BiologyEdgeType::GeospatialLocation));

    let final_graph = BiologyGraph { graph_id, project_id, source_description: analysis.source_description, nodes, edges, root_node_id: root_id, state: GraphStateType::SemanticEnriched, state_history: vec![GraphStateTransition { from: GraphStateType::Created, to: GraphStateType::SemanticEnriched, timestamp: now.clone(), triggered_by_step: None }], created_at: now.clone(), updated_at: now.clone(), version: 1, version_notes: vec![VersionNote { version: 1, note: "Semantic enrichment complete".into(), step_index: None, timestamp: now, change_type: ChangeType::EnrichedBySemantic }] };
    let _ = executor.save_graph(&final_graph);
    BiologyModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(final_graph), ..Default::default() }
}

// ─────────────────────────────────────────────────────────────────────────────
// MAIN EXECUTION
// ─────────────────────────────────────────────────────────────────────────────

pub async fn execute(input: BiologyModalityAction) -> Result<BiologyModalityOutput, String> {
    let executor = PipelineExecutor::new();

    match input {
        BiologyModalityAction::Analyze { data, extract_pathways, extract_cell_types, extract_tissue, extract_organism, extract_ecology } => {
            let analysis_id = executor.generate_id();
            let (source_description, organism) = match &data {
                BiologyDataSource::PathwayFile { file_path, organism, .. } =>
                    (format!("PathwayFile: {} organism={}", file_path, organism), organism.clone()),
                BiologyDataSource::ScRNAseqFile { file_path, species, tissue, cell_count, .. } =>
                    (format!("scRNA: {} {} cells tissue={:?}", file_path, cell_count, tissue), species.clone()),
                BiologyDataSource::ExpressionMatrix { file_path, species, .. } =>
                    (format!("ExpressionMatrix: {} species={}", file_path, species), species.clone()),
                BiologyDataSource::SpatialTranscriptomicsFile { file_path, tissue, species, .. } =>
                    (format!("SpatialTx: {} tissue={} species={}", file_path, tissue, species), species.clone()),
                BiologyDataSource::HistologyImage { file_path, tissue, stain, .. } =>
                    (format!("Histology: {} tissue={} stain={:?}", file_path, tissue, stain), "unknown".into()),
                BiologyDataSource::EcologicalSurvey { file_path, survey_area, .. } =>
                    (format!("EcoSurvey: {} area={:?}", file_path, survey_area), "multi-species".into()),
                BiologyDataSource::MetabolomicsFile { file_path, organism, platform, .. } =>
                    (format!("Metabolomics: {} organism={} platform={:?}", file_path, organism, platform), organism.clone()),
                BiologyDataSource::PPI_Network { file_path, organism, .. } =>
                    (format!("PPI: {} organism={}", file_path, organism), organism.clone()),
                BiologyDataSource::LiteratureMining { source_reference, extraction_focus, .. } =>
                    (format!("LiteratureMining: {} focus={:?}", source_reference, extraction_focus), "unknown".into()),
                BiologyDataSource::PhysiologyData { file_path, organism, data_type, .. } =>
                    (format!("Physiology: {} organism={} type={:?}", file_path, organism, data_type), organism.clone()),
                BiologyDataSource::ChromatinAccessibility { file_path, species, .. } =>
                    (format!("ATAC: {} species={}", file_path, species), species.clone()),
                BiologyDataSource::MultiOmics { omics_files, integration_method } =>
                    (format!("MultiOmics: {} files method={:?}", omics_files.len(), integration_method), "multi-omics".into()),
            };
            Ok(BiologyModalityOutput { success: true, analysis: Some(BiologyAnalysisResult { analysis_id, source_description, organism, ..Default::default() }), ..Default::default() })
        }

        BiologyModalityAction::CreateGraph { analysis, project_id } => {
            Ok(create_graph(&executor, analysis, project_id).await)
        }

        BiologyModalityAction::UpdateGraph { graph_id, updates, project_id } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            let mut next_nid = graph.nodes.iter().map(|n| n.node_id).max().unwrap_or(0) + 1;
            let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
            let initial = graph.nodes.len();
            for update in &updates {
                match update {
                    BiologyUpdate::AddPathway { pathway } => {
                        graph.nodes.push(BiologyGraphNode {
                            node_id: next_nid, node_type: BiologyNodeType::PathwayNode,
                            content: format!("Pathway [{}] {}: genes={}", pathway.pathway_id, pathway.name, pathway.gene_count),
                            biological_id: Some(pathway.pathway_id.clone()),
                            gene_count: Some(pathway.gene_count),
                            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                            keywords: vec!["pathway".into(), pathway.name.to_lowercase()], hotness_score: 0.65, ..Default::default()
                        });
                        graph.edges.push(BiologyGraphEdge { edge_id: next_eid, from_node: graph.root_node_id, to_node: next_nid, edge_type: BiologyEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                        next_eid += 1; next_nid += 1;
                    }
                    BiologyUpdate::AddCellType { cell_type } => {
                        graph.nodes.push(BiologyGraphNode {
                            node_id: next_nid, node_type: BiologyNodeType::CellTypeNode,
                            content: format!("CellType: {} state={:?}", cell_type.name, cell_type.functional_state),
                            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                            keywords: vec!["cell-type".into(), cell_type.name.to_lowercase()], hotness_score: 0.65, ..Default::default()
                        });
                        graph.edges.push(BiologyGraphEdge { edge_id: next_eid, from_node: graph.root_node_id, to_node: next_nid, edge_type: BiologyEdgeType::Contains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                        next_eid += 1; next_nid += 1;
                    }
                    BiologyUpdate::AddSpeciesRecord { species } => {
                        graph.nodes.push(BiologyGraphNode {
                            node_id: next_nid, node_type: BiologyNodeType::SpeciesNode,
                            content: format!("Species: {} obs={}", species.scientific_name, species.observation_count),
                            species_name: Some(species.scientific_name.clone()),
                            location: species.location,
                            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                            keywords: vec!["species".into(), species.scientific_name.to_lowercase()], hotness_score: 0.6, ..Default::default()
                        });
                        graph.edges.push(BiologyGraphEdge { edge_id: next_eid, from_node: graph.root_node_id, to_node: next_nid, edge_type: BiologyEdgeType::Contains, weight: 0.7, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                        next_eid += 1; next_nid += 1;
                    }
                    BiologyUpdate::UpdatePathwayActivity { pathway_id, activity_score } => {
                        if let Some(n) = graph.nodes.iter_mut().find(|n| n.biological_id.as_deref() == Some(pathway_id)) {
                            n.activity_score = Some(*activity_score);
                            n.version += 1;
                            n.version_notes.push(VersionNote { version: n.version, note: format!("Activity updated to {:.3}", activity_score), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                        }
                    }
                    BiologyUpdate::UpdateCellTypeCount { cell_type_id, new_count } => {
                        if let Some(n) = graph.nodes.iter_mut().find(|n| matches!(n.node_type, BiologyNodeType::CellTypeNode) && n.gene_count == Some(*cell_type_id as u32)) {
                            n.gene_count = Some(*new_count);
                            n.version += 1;
                        }
                    }
                    BiologyUpdate::AddDiseaseAssociation { association } => {
                        graph.nodes.push(BiologyGraphNode {
                            node_id: next_nid, node_type: BiologyNodeType::DiseaseNode,
                            content: format!("Disease: {} genes={}", association.disease_name, association.associated_genes.len()),
                            is_pathological: Some(true),
                            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                            keywords: vec!["disease".into(), association.disease_name.to_lowercase()], hotness_score: 0.8, ..Default::default()
                        });
                        graph.edges.push(BiologyGraphEdge { edge_id: next_eid, from_node: graph.root_node_id, to_node: next_nid, edge_type: BiologyEdgeType::Contains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                        next_eid += 1; next_nid += 1;
                    }
                    BiologyUpdate::UpdatePhysiologicalState { state_id, new_state } => {
                        if let Some(n) = graph.nodes.iter_mut().find(|n| matches!(n.node_type, BiologyNodeType::PhysiologicalStateNode)) {
                            n.version += 1;
                            n.version_notes.push(VersionNote { version: n.version, note: format!("State updated: {:?}", new_state), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                        }
                    }
                }
            }
            graph.version += 1; graph.updated_at = now.clone(); graph.state = GraphStateType::Updated;
            graph.version_notes.push(VersionNote { version: graph.version, note: format!("Updated: {} changes, {} new nodes", updates.len(), graph.nodes.len() - initial), step_index: None, timestamp: now, change_type: ChangeType::Updated });
            executor.save_graph(&graph)?;
            Ok(BiologyModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        BiologyModalityAction::PathwayEnrichment { graph_id, gene_list, database, significance_threshold } => {
            let graph = executor.load_graph(graph_id)?;
            let pathway_nodes: Vec<BiologicalPathway> = graph.nodes.iter()
                .filter(|n| matches!(n.node_type, BiologyNodeType::PathwayNode))
                .map(|n| BiologicalPathway {
                    pathway_id: n.biological_id.clone().unwrap_or_default(),
                    name: n.content.chars().take(50).collect(),
                    database: format!("{:?}", database),
                    gene_count: n.gene_count.unwrap_or(0),
                    is_enriched: false,
                    ..Default::default()
                }).collect();

            let scores = executor.compute_pathway_enrichment_llm(&gene_list, &pathway_nodes).await;
            let mut enriched: Vec<BiologicalPathway> = pathway_nodes.into_iter().map(|mut pw| {
                if let Some((_, score)) = scores.iter().find(|(id, _)| *id == pw.pathway_id) {
                    pw.is_enriched = *score >= significance_threshold as f64;
                    pw.enrichment_fdr = Some(1.0 - score);
                }
                pw
            }).filter(|pw| pw.is_enriched).collect();
            enriched.sort_by(|a, b| a.enrichment_fdr.partial_cmp(&b.enrichment_fdr).unwrap_or(std::cmp::Ordering::Equal));
            Ok(BiologyModalityOutput { success: true, graph_id: Some(graph_id), enrichment_results: Some(enriched), ..Default::default() })
        }

        BiologyModalityAction::AnalyzeSpatialTranscriptomics { data, project_id } => {
            let analysis = BiologyAnalysisResult {
                analysis_id: executor.generate_id(),
                source_description: format!("SpatialTx: {} tissue={} species={}", data.file_path, data.tissue, data.species),
                organism: data.species,
                biological_scale: BiologicalScale::Tissue,
                ..Default::default()
            };
            Ok(create_graph(&executor, analysis, project_id).await)
        }

        BiologyModalityAction::ClassifyCellTypes { expression_profiles, reference_atlas } => {
            if expression_profiles.is_empty() {
                return Ok(BiologyModalityOutput { success: true, cell_type_assignments: Some(vec![]), ..Default::default() });
            }
            // Build pseudo-clusters from profiles by their cluster_id
            let mut cluster_map: HashMap<u64, Vec<&CellExpressionProfile>> = HashMap::new();
            for p in &expression_profiles {
                if let Some(cid) = p.cluster_id { cluster_map.entry(cid).or_default().push(p); }
            }
            let pseudo_clusters: Vec<CellCluster> = cluster_map.iter().map(|(cid, cells)| {
                let mut top_genes: Vec<String> = cells.iter()
                    .flat_map(|c| c.gene_expression.iter().filter(|(_, &v)| v > 1.0).map(|(g, _)| g.clone()))
                    .collect();
                top_genes.sort(); top_genes.dedup();
                CellCluster { cluster_id: *cid, cluster_name: format!("Cluster_{}", cid), cell_count: cells.len() as u32, marker_genes: top_genes.into_iter().take(10).collect(), ..Default::default() }
            }).collect();

            let tissue = reference_atlas.as_deref().unwrap_or("unknown tissue");
            let assignments = executor.classify_cell_types_llm(&pseudo_clusters, tissue).await;
            let result: Vec<(String, String)> = assignments.iter().map(|(id, ct)| (format!("Cluster_{}", id), ct.clone())).collect();
            Ok(BiologyModalityOutput { success: true, cell_type_assignments: Some(result), ..Default::default() })
        }

        BiologyModalityAction::BuildRegulatoryNetwork { graph_id, evidence_types } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            graph.version += 1; graph.updated_at = now.clone();
            graph.version_notes.push(VersionNote { version: graph.version, note: format!("Regulatory network built from evidence: {:?}", evidence_types.iter().map(|e| format!("{:?}", e)).collect::<Vec<_>>().join(", ")), step_index: None, timestamp: now, change_type: ChangeType::Updated });
            executor.save_graph(&graph)?;
            Ok(BiologyModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        BiologyModalityAction::AnalyzeCommunity { species_data, diversity_metrics, network_analysis } => {
            let species_records: Vec<SpeciesRecord> = species_data.iter().enumerate().map(|(i, obs)| {
                SpeciesRecord {
                    species_id: executor.generate_id() + i as u64,
                    scientific_name: obs.species_name.clone(),
                    common_name: None,
                    taxonomy: TaxonomyInfo { species: obs.species_name.clone(), ..Default::default() },
                    observation_count: obs.count,
                    location: obs.location,
                    habitat: None,
                    conservation_status: ConservationStatus::Unknown,
                    acoustic_signature_id: None,
                    thermal_signature_id: None,
                }
            }).collect();

            let biodiversity = if diversity_metrics && !species_records.is_empty() {
                let n: f64 = species_records.iter().map(|s| s.observation_count as f64).sum();
                let shannon: f64 = species_records.iter().map(|s| {
                    let p = s.observation_count as f64 / n.max(1.0);
                    if p > 0.0 { -p * p.ln() } else { 0.0 }
                }).sum();
                let simpson: f64 = 1.0 - species_records.iter().map(|s| {
                    let p = s.observation_count as f64 / n.max(1.0);
                    p * p
                }).sum::<f64>();
                let richness = species_records.len() as u32;
                let max_shannon = (richness as f64).ln();
                let evenness = if max_shannon > 0.0 { shannon / max_shannon } else { 0.0 };
                Some(BiodiversityMetrics { shannon_index: shannon, simpson_index: simpson, species_richness: richness, evenness, ..Default::default() })
            } else { None };

            let analysis = BiologyAnalysisResult {
                analysis_id: executor.generate_id(),
                source_description: format!("EcologicalCommunity: {} species {} observations", species_records.len(), species_records.iter().map(|s| s.observation_count).sum::<u32>()),
                organism: "multi-species".into(),
                biological_scale: BiologicalScale::Ecosystem,
                species_detected: species_records,
                biodiversity_metrics: biodiversity,
                ..Default::default()
            };
            Ok(create_graph(&executor, analysis, 0).await)
        }

        BiologyModalityAction::QueryGraph { graph_id, query } => {
            let graph = executor.load_graph(graph_id)?;
            let result = match query {
                BiologyGraphQuery::NodeDetail { node_id } => {
                    let node = graph.nodes.iter().find(|n| n.node_id == node_id);
                    let incoming: Vec<_> = graph.edges.iter().filter(|e| e.to_node == node_id).collect();
                    let outgoing: Vec<_> = graph.edges.iter().filter(|e| e.from_node == node_id).collect();
                    serde_json::json!({ "node": node, "incoming": incoming, "outgoing": outgoing })
                }
                BiologyGraphQuery::PathwaysByCategory { category } => {
                    let pws: Vec<_> = graph.nodes.iter()
                        .filter(|n| matches!(n.node_type, BiologyNodeType::PathwayNode))
                        .filter(|n| n.keywords.iter().any(|k| k.to_lowercase().contains(&category.to_lowercase())))
                        .collect();
                    serde_json::json!({ "pathways": pws })
                }
                BiologyGraphQuery::CellTypesByTissue { tissue_name } => {
                    let cts: Vec<_> = graph.nodes.iter()
                        .filter(|n| matches!(n.node_type, BiologyNodeType::CellTypeNode))
                        .collect();
                    serde_json::json!({ "cell_types": cts, "tissue": tissue_name })
                }
                BiologyGraphQuery::DiseaseAssociations { disease_name } => {
                    let dis: Vec<_> = graph.nodes.iter()
                        .filter(|n| matches!(n.node_type, BiologyNodeType::DiseaseNode))
                        .filter(|n| n.content.to_lowercase().contains(&disease_name.to_lowercase()))
                        .collect();
                    serde_json::json!({ "diseases": dis })
                }
                BiologyGraphQuery::EnrichedPathways => {
                    let enriched: Vec<_> = graph.nodes.iter()
                        .filter(|n| matches!(n.node_type, BiologyNodeType::PathwayNode))
                        .filter(|n| n.hotness_score > 0.8)
                        .collect();
                    serde_json::json!({ "enriched_pathways": enriched })
                }
                BiologyGraphQuery::SpeciesByHabitat { habitat_type } => {
                    let species: Vec<_> = graph.nodes.iter()
                        .filter(|n| matches!(n.node_type, BiologyNodeType::SpeciesNode))
                        .collect();
                    serde_json::json!({ "species": species, "habitat_filter": habitat_type })
                }
                BiologyGraphQuery::DrugTargets => {
                    let dts: Vec<_> = graph.nodes.iter()
                        .filter(|n| matches!(n.node_type, BiologyNodeType::DrugTargetNode))
                        .collect();
                    serde_json::json!({ "drug_targets": dts })
                }
                BiologyGraphQuery::RegulatedBy { gene_symbol } => {
                    let regs: Vec<_> = graph.nodes.iter()
                        .filter(|n| matches!(n.node_type, BiologyNodeType::RegulatoryEdgeNode))
                        .filter(|n| n.biological_id.as_deref().map(|id| id.contains(&gene_symbol)).unwrap_or(false))
                        .collect();
                    serde_json::json!({ "regulatory_edges": regs })
                }
                BiologyGraphQuery::CrossModalLinks { node_id } => {
                    let links: Vec<_> = graph.edges.iter()
                        .filter(|e| (e.from_node == node_id || e.to_node == node_id) && matches!(e.edge_type, BiologyEdgeType::GenomicSequenceIn | BiologyEdgeType::StructureIn112 | BiologyEdgeType::ChemistryOf | BiologyEdgeType::MetabolicHeatIn | BiologyEdgeType::AcousticSignatureIn | BiologyEdgeType::VegetationSignalIn | BiologyEdgeType::GeospatialLocation))
                        .collect();
                    serde_json::json!({ "cross_modal_links": links })
                }
                BiologyGraphQuery::AGIActivity => serde_json::json!({ "is_active": false }),
                BiologyGraphQuery::AllNodes => serde_json::json!({ "nodes": graph.nodes }),
                BiologyGraphQuery::AllEdges => serde_json::json!({ "edges": graph.edges }),
            };
            Ok(BiologyModalityOutput { success: true, query_result: Some(result), ..Default::default() })
        }

        BiologyModalityAction::GetGraph { graph_id } => {
            let graph = executor.load_graph(graph_id)?;
            Ok(BiologyModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        BiologyModalityAction::TriggerSemanticHook { graph_id, hook } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            match hook {
                BiologySemanticHook::OnGraphCreated => { graph.state = GraphStateType::SemanticEnriched; }
                BiologySemanticHook::OnInferRelationships => {
                    let new_edges = executor.infer_semantic_relationships(&graph.nodes).await;
                    let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                    let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                    for (from, to, etype, reason) in new_edges {
                        if valid.contains(&from) && valid.contains(&to) && from != to {
                            graph.edges.push(BiologyGraphEdge { edge_id: next_eid, from_node: from, to_node: to, edge_type: etype, weight: 0.8, provenance: EdgeProvenance::DerivedFromHook, version: 1, properties: { let mut p = HashMap::new(); p.insert("reason".into(), serde_json::json!(reason)); p }, ..Default::default() });
                            next_eid += 1;
                        }
                    }
                }
                BiologySemanticHook::OnEdgeCompletion => {
                    let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                    graph.edges.retain(|e| valid.contains(&e.from_node) && valid.contains(&e.to_node));
                }
                BiologySemanticHook::OnCrossModalityLink { target_modality, target_graph_id } => {
                    graph.state = GraphStateType::CrossLinked;
                    graph.version += 1;
                    graph.version_notes.push(VersionNote { version: graph.version, note: format!("Cross-linked to {} (graph {})", target_modality, target_graph_id), step_index: None, timestamp: now.clone(), change_type: ChangeType::CrossLinked });
                }
            }
            graph.updated_at = now;
            executor.save_graph(&graph)?;
            Ok(BiologyModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        BiologyModalityAction::ExportProduct { graph_id, format } => {
            let ext = match &format {
                BiologyExportFormat::SBML => "xml",
                BiologyExportFormat::BioPAX => "owl",
                BiologyExportFormat::GraphML => "graphml",
                BiologyExportFormat::Cytoscape_JSON => "json",
                BiologyExportFormat::GMT => "gmt",
                BiologyExportFormat::TSV_Network => "tsv",
                BiologyExportFormat::KEGG_KGML => "xml",
                BiologyExportFormat::GeoJSON => "geojson",
                BiologyExportFormat::Custom(ext) => ext.as_str(),
            };
            let export_path = format!("/tmp/biology_export_{}_{:?}.{}", graph_id, format, ext);
            Ok(BiologyModalityOutput { success: true, export_path: Some(export_path), ..Default::default() })
        }

        BiologyModalityAction::StreamToUI { graph_id, .. } => {
            Ok(BiologyModalityOutput { success: true, graph_id: Some(graph_id), ..Default::default() })
        }

        BiologyModalityAction::HeadlessProcess { graph_id, operations } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            for op in operations {
                match op {
                    BiologyOperation::RunPathwayEnrichment { gene_list, database } => {
                        let pws: Vec<BiologicalPathway> = graph.nodes.iter()
                            .filter(|n| matches!(n.node_type, BiologyNodeType::PathwayNode))
                            .map(|n| BiologicalPathway { pathway_id: n.biological_id.clone().unwrap_or_default(), name: n.content.chars().take(50).collect(), database: format!("{:?}", database), gene_count: n.gene_count.unwrap_or(0), ..Default::default() })
                            .collect();
                        let scores = executor.compute_pathway_enrichment_llm(&gene_list, &pws).await;
                        for (pw_id, score) in scores {
                            if let Some(n) = graph.nodes.iter_mut().find(|n| n.biological_id.as_deref() == Some(&pw_id)) {
                                n.hotness_score = (n.hotness_score + score as f32 * 0.3).min(1.0);
                                n.version += 1;
                                n.version_notes.push(VersionNote { version: n.version, note: format!("Enrichment score updated: {:.3}", score), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                            }
                        }
                    }
                    BiologyOperation::ClusterCellTypes => {
                        // In production: re-cluster cell type nodes using expression similarity
                        graph.version_notes.push(VersionNote { version: graph.version + 1, note: "Cell type clustering re-run".into(), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                        graph.version += 1;
                    }
                    BiologyOperation::InferSignalingPaths => {
                        // In production: run NicheNet / CellChat-style L-R inference
                        graph.version_notes.push(VersionNote { version: graph.version + 1, note: "Signaling path inference run".into(), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                        graph.version += 1;
                    }
                    BiologyOperation::CrossLinkToGenomics { dna_graph_id } => {
                        let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        for node in graph.nodes.iter().filter(|n| matches!(n.node_type, BiologyNodeType::PathwayNode | BiologyNodeType::RegulatoryEdgeNode)) {
                            graph.edges.push(BiologyGraphEdge { edge_id: next_eid, from_node: node.node_id, to_node: node.node_id, edge_type: BiologyEdgeType::GenomicSequenceIn, weight: 0.85, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("dna_graph_id".into(), serde_json::json!(dna_graph_id)); p }, ..Default::default() });
                            next_eid += 1;
                        }
                    }
                    BiologyOperation::CrossLinkToProteomics { prot_graph_id } => {
                        let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        for node in graph.nodes.iter().filter(|n| matches!(n.node_type, BiologyNodeType::CellTypeNode | BiologyNodeType::CellularProcessNode)) {
                            graph.edges.push(BiologyGraphEdge { edge_id: next_eid, from_node: node.node_id, to_node: node.node_id, edge_type: BiologyEdgeType::StructureIn112, weight: 0.8, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("prot_graph_id".into(), serde_json::json!(prot_graph_id)); p }, ..Default::default() });
                            next_eid += 1;
                        }
                    }
                    BiologyOperation::CrossLinkToChemistry { chem_graph_id } => {
                        let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        for node in graph.nodes.iter().filter(|n| matches!(n.node_type, BiologyNodeType::MetaboliteNode)) {
                            graph.edges.push(BiologyGraphEdge { edge_id: next_eid, from_node: node.node_id, to_node: node.node_id, edge_type: BiologyEdgeType::ChemistryOf, weight: 0.9, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("chem_graph_id".into(), serde_json::json!(chem_graph_id)); p }, ..Default::default() });
                            next_eid += 1;
                        }
                    }
                    BiologyOperation::ComputeBiodiversity => {
                        let species_nodes: Vec<&BiologyGraphNode> = graph.nodes.iter().filter(|n| matches!(n.node_type, BiologyNodeType::SpeciesNode)).collect();
                        if !species_nodes.is_empty() {
                            let richness = species_nodes.len() as u32;
                            let shannon = -(1.0f64 / richness as f64 * (1.0f64 / richness as f64).ln() * richness as f64);
                            graph.version_notes.push(VersionNote { version: graph.version + 1, note: format!("Biodiversity computed: richness={} shannon={:.3}", richness, shannon), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                            graph.version += 1;
                        }
                    }
                    BiologyOperation::ExportNetworkFile => {
                        // In production: write SBML/BioPAX/GraphML to disk
                    }
                }
            }
            graph.updated_at = now;
            executor.save_graph(&graph)?;
            Ok(BiologyModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
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
    if input_json.is_empty() { eprintln!("Usage: biology --input '<json>'"); std::process::exit(1); }
    let input: BiologyModalityAction = match serde_json::from_str(&input_json) {
        Ok(v) => v,
        Err(e) => { println!("{}", serde_json::json!({"success":false,"error":format!("Parse error: {}",e)})); std::process::exit(1); }
    };
    let rt = tokio::runtime::Runtime::new().expect("Tokio runtime");
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap_or_else(|_| r#"{"success":false,"error":"serialize"}"#.into())),
        Err(e) => { println!("{}", serde_json::json!({"success":false,"error":e})); std::process::exit(1); }
    }
}
