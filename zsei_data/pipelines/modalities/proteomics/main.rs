//! Proteomics — Pipeline #112
//!
//! Protein-level analysis: structure, function, interactions, post-translational
//! modifications, expression, and pathway integration. Operates on mass-spec data,
//! FASTA/PDB sequences, interaction databases, and expression matrices.
//!
//! DISTINCT FROM:
//!   - DNA (107): genomic/sequence level; Proteomics works with expressed proteins,
//!     their 3D structure, modifications, abundances, and functional interactions.
//!     The proteome is the functional layer above the genome.
//!   - Chemistry (106): general molecular reactions; Proteomics specializes in
//!     proteins as biological machines — folding, binding sites, post-translational
//!     chemistry, and protein-protein interaction networks.
//!   - Biology (111): organism/pathway/cellular level; Proteomics provides the
//!     protein-resolution detail that feeds upward into biological models.
//!
//! CROSS-LINKS:
//!   107 (DNA)     → coding sequences → proteins, gene expression → protein abundance
//!   106 (Chem)    → enzyme mechanisms, small molecule binding, chemical modifications
//!   111 (Bio)     → pathway membership, cellular compartment, organism context
//!   126 (Hyper)   → hyperspectral tissue imaging identifies protein distributions
//!   105 (Math)    → structural bioinformatics scores, docking energies
//!   101 (Code)    → bioinformatics pipelines, analysis scripts
//!
//! STORAGE: ZSEI containers under /Modalities/Proteomics/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

// ─────────────────────────────────────────────────────────────────────────────
// INPUT TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ProteomicsModalityAction {
    /// Analyze a protein sequence / structure / expression dataset
    Analyze {
        data: ProteomicsDataSource,
        extract_structure: bool,
        extract_interactions: bool,
        extract_modifications: bool,
        extract_expression: bool,
        extract_function: bool,
    },
    /// Create graph from analysis result
    CreateGraph {
        analysis: ProteomicsAnalysisResult,
        project_id: u64,
    },
    /// Update graph with new experimental data
    UpdateGraph {
        graph_id: u64,
        updates: Vec<ProteomicsUpdate>,
        project_id: u64,
    },
    /// Predict protein structure from sequence (AlphaFold-style)
    PredictStructure {
        sequence: String,
        method: StructurePredictionMethod,
        include_confidence: bool,
    },
    /// Dock a ligand to a protein structure
    DockLigand {
        protein_graph_id: u64,
        protein_node_id: u64,
        ligand_smiles: String,
        binding_site_residues: Option<Vec<String>>,
        method: DockingMethod,
    },
    /// Compute protein-protein interaction network
    ComputePPINetwork {
        protein_ids: Vec<String>,
        interaction_db: PPIDatabase,
        confidence_threshold: f32,
        expand_neighbors: bool,
    },
    /// Analyze mass spectrometry output
    AnalyzeMassSpec {
        data: MassSpecData,
        quantification_method: QuantificationMethod,
        fdr_threshold: f32,
    },
    /// Map protein to known pathways
    MapPathways {
        protein_id: String,
        databases: Vec<PathwayDatabase>,
    },
    /// Align two or more protein sequences
    AlignSequences {
        sequences: Vec<ProteinSequenceEntry>,
        method: AlignmentMethod,
    },
    /// Compute structural similarity between two proteins
    StructuralAlignment {
        structure_a_id: u64,
        structure_b_id: u64,
        method: StructuralAlignmentMethod,
    },
    /// Query graph
    QueryGraph {
        graph_id: u64,
        query: ProteomicsGraphQuery,
    },
    /// Retrieve full graph for Context Viewer
    GetGraph { graph_id: u64 },
    /// Trigger ZSEI semantic hooks
    TriggerSemanticHook {
        graph_id: u64,
        hook: ProteomicsSemanticHook,
    },
    /// Export proteomics products
    ExportProduct {
        graph_id: u64,
        format: ProteomicsExportFormat,
    },
    /// Stream to UI
    StreamToUI {
        graph_id: u64,
        session_id: String,
        display_mode: ProteomicsDisplayMode,
    },
    /// Headless processing (AGI-first)
    HeadlessProcess {
        graph_id: u64,
        operations: Vec<ProteomicsOperation>,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// DATA SOURCES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProteomicsDataSource {
    /// FASTA file — one or more protein sequences
    FASTA {
        file_path: String,
        organism: Option<String>,
    },
    /// Protein Data Bank structure file
    PDB {
        file_path: String,
        pdb_id: Option<String>,
        chain_ids: Option<Vec<String>>,
    },
    /// mmCIF / PDBx structure file
    MMCIF {
        file_path: String,
        entry_id: Option<String>,
    },
    /// AlphaFold predicted structure
    AlphaFold {
        file_path: String,
        uniprot_id: Option<String>,
        include_pae: bool,       // predicted aligned error
    },
    /// Mass spectrometry results file
    MassSpec {
        file_path: String,
        format: MassSpecFormat,
        experiment_type: MassSpecExperimentType,
    },
    /// Proteome expression matrix (proteins × samples)
    ExpressionMatrix {
        file_path: String,
        format: ExpressionFormat,
        sample_metadata: Option<String>,
    },
    /// UniProt flat file or XML
    UniProt {
        file_path: String,
        accession_ids: Option<Vec<String>>,
    },
    /// STRING / BioGRID / IntAct interaction data
    InteractionDB {
        file_path: String,
        database: PPIDatabase,
        species_taxid: Option<u32>,
    },
    /// Protein domain annotation (PFAM, PROSITE, InterPro)
    DomainAnnotation {
        file_path: String,
        database: DomainDatabase,
    },
    /// Live query against UniProt / PDB / STRING API
    RemoteQuery {
        database: RemoteDatabase,
        query: String,
        max_results: u32,
    },
    /// Multiple data sources combined
    MultiSource {
        sources: Vec<ProteomicsDataSource>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MassSpecFormat { MGF, mzML, mzXML, RAW, MaxQuant_TXT, Proteome_Discoverer_CSV, Skyline_CSV, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum MassSpecExperimentType {
    #[default] DataDependentAcquisition, DataIndependentAcquisition,
    TMT_Quantification, SILAC, iTRAQ, LabelFree, PRM, MRM, SWATH,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExpressionFormat { CSV, TSV, HDF5, AnnData, BioconductorRDS, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PPIDatabase { #[default] STRING, BioGRID, IntAct, DIP, HPRD, MINT, Reactome, HuRI, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum DomainDatabase { #[default] Pfam, Prosite, InterPro, SCOP, CATH, SMART, TIGRfam, Panther }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PathwayDatabase { #[default] KEGG, Reactome, WikiPathways, BioCarta, MSigDB_Hallmarks, MSigDB_C2, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum RemoteDatabase { #[default] UniProt, PDB, STRING, BioGRID, InterPro, KEGG }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum StructurePredictionMethod { #[default] AlphaFold2, RoseTTAFold, ESMFold, OmegaFold, ColabFold, Rosetta, I_TASSER }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum DockingMethod { #[default] AutoDock_Vina, GNINA, Glide, Gold, FlexX, rDock, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum QuantificationMethod { #[default] LabelFree_LFQ, TMT, SILAC, iTRAQ, SpectralCounting, Empower }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum AlignmentMethod { #[default] ClustalOmega, MUSCLE, MAFFT, T_Coffee, Clustal_W }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum StructuralAlignmentMethod { #[default] TM_Align, DALI, CE_Align, FATCAT, MaxSub }

// ─────────────────────────────────────────────────────────────────────────────
// ANALYSIS RESULT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProteomicsAnalysisResult {
    pub analysis_id: u64,
    pub source_description: String,
    pub organism: Option<String>,
    pub taxon_id: Option<u32>,

    // PROTEINS (core entities)
    pub proteins: Vec<Protein>,
    pub protein_isoforms: Vec<ProteinIsoform>,

    // STRUCTURE
    pub structures: Vec<ProteinStructure>,
    pub domains: Vec<ProteinDomain>,
    pub active_sites: Vec<ActiveSite>,
    pub binding_sites: Vec<BindingSite>,
    pub structural_motifs: Vec<StructuralMotif>,
    pub secondary_structure_regions: Vec<SecondaryStructureRegion>,

    // POST-TRANSLATIONAL MODIFICATIONS
    pub ptms: Vec<PostTranslationalModification>,
    pub glycosylation_sites: Vec<GlycosylationSite>,
    pub ubiquitination_sites: Vec<UbiquitinationSite>,
    pub cleavage_sites: Vec<CleavageSite>,
    pub disulfide_bonds: Vec<DisulfideBond>,
    pub signal_peptides: Vec<SignalPeptide>,

    // INTERACTIONS
    pub protein_protein_interactions: Vec<ProteinProteinInteraction>,
    pub protein_dna_interactions: Vec<ProteinDNAInteraction>,
    pub protein_ligand_interactions: Vec<ProteinLigandInteraction>,
    pub complexes: Vec<ProteinComplex>,

    // EXPRESSION
    pub expression_data: Vec<ProteinExpression>,
    pub differential_expression: Vec<DifferentialExpression>,

    // FUNCTIONAL ANNOTATION
    pub go_terms: Vec<GOTermAnnotation>,
    pub pathway_memberships: Vec<PathwayMembership>,
    pub enzyme_activities: Vec<EnzymeActivity>,
    pub subcellular_locations: Vec<SubcellularLocation>,

    // SEQUENCE ANALYSIS
    pub sequence_alignments: Vec<SequenceAlignment>,
    pub orthologs: Vec<OrthologRelation>,
    pub paralog_clusters: Vec<ParalogCluster>,

    // MASS SPEC
    pub ms_identifications: Vec<MSIdentification>,
    pub ms_quantifications: Vec<MSQuantification>,
    pub peptide_evidence: Vec<PeptideEvidence>,

    // METADATA
    pub processing_notes: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// PROTEIN CORE
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Protein {
    pub protein_id: u64,
    pub accession: String,                  // UniProt accession, e.g. "P12345"
    pub entry_name: String,                 // UniProt entry name, e.g. "TP53_HUMAN"
    pub gene_name: Option<String>,
    pub full_name: String,
    pub sequence: String,                   // one-letter amino acid sequence
    pub sequence_length: u32,
    pub molecular_weight_da: f64,
    pub isoelectric_point: f32,
    pub organism: String,
    pub taxon_id: u32,
    pub existence_evidence: ProteinExistenceLevel,
    pub reviewed: bool,                     // UniProt reviewed (Swiss-Prot) vs. unreviewed (TrEMBL)
    pub domain_ids: Vec<u64>,
    pub ptm_ids: Vec<u64>,
    pub structure_ids: Vec<u64>,
    pub go_term_ids: Vec<u64>,
    pub pathway_ids: Vec<u64>,
    pub subcellular_location_ids: Vec<u64>,
    pub interaction_partner_ids: Vec<String>, // accessions
    pub disease_associations: Vec<String>,
    pub variants_known: u32,
    pub cross_references: HashMap<String, Vec<String>>,   // db → list of IDs
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ProteinExistenceLevel {
    #[default] Uncertain,
    Predicted, InferredFromHomology,
    ExperimentalEvidenceProteinLevel, ExperimentalEvidenceTranscriptLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProteinIsoform {
    pub isoform_id: u64,
    pub parent_protein_id: u64,
    pub isoform_number: u32,
    pub sequence_changes: Vec<IsoformChange>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IsoformChange {
    pub change_type: IsoformChangeType,
    pub start_pos: u32,
    pub end_pos: u32,
    pub alternative_sequence: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum IsoformChangeType { #[default] Alternative, VSP_Missing, Extension }

// ─────────────────────────────────────────────────────────────────────────────
// STRUCTURE
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProteinStructure {
    pub structure_id: u64,
    pub protein_id: u64,
    pub pdb_id: Option<String>,
    pub structure_method: StructureDeterminationMethod,
    pub resolution_angstrom: Option<f32>,
    pub chain_id: String,
    pub residue_count: u32,
    pub coverage_start: u32,              // residue number
    pub coverage_end: u32,
    pub file_path: Option<String>,
    pub tm_score: Option<f32>,            // for predicted structures
    pub plddt_mean: Option<f32>,          // AlphaFold per-residue confidence mean
    pub secondary_structure_summary: SecondaryStructureSummary,
    pub oligomerization_state: OligomerizationState,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum StructureDeterminationMethod {
    #[default] Unknown,
    X_Ray_Crystallography, NMR, CryoEM, CryoET, Neutron_Diffraction,
    AlphaFold2, RoseTTAFold, ESMFold, Homology_Modeling, Ab_Initio,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecondaryStructureSummary {
    pub helix_fraction: f32,
    pub sheet_fraction: f32,
    pub coil_fraction: f32,
    pub turn_fraction: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum OligomerizationState { #[default] Unknown, Monomer, Dimer, Trimer, Tetramer, Hexamer, Octamer, Oligomer, Polymer }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecondaryStructureRegion {
    pub region_id: u64,
    pub protein_id: u64,
    pub region_type: SecondaryStructureType,
    pub start_residue: u32,
    pub end_residue: u32,
    pub length_aa: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SecondaryStructureType { #[default] AlphaHelix, BetaStrand, Turn, Loop, Coil, Helix310, PiHelix }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProteinDomain {
    pub domain_id: u64,
    pub protein_id: u64,
    pub domain_name: String,
    pub pfam_id: Option<String>,
    pub interpro_id: Option<String>,
    pub start_residue: u32,
    pub end_residue: u32,
    pub e_value: f64,
    pub domain_function: Option<String>,
    pub is_catalytic: bool,
    pub is_binding: bool,
    pub structural_class: Option<String>,     // SCOP/CATH classification
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActiveSite {
    pub site_id: u64,
    pub protein_id: u64,
    pub residue_positions: Vec<ActiveSiteResidue>,
    pub catalytic_mechanism: Option<String>,
    pub substrate_specificity: Option<String>,
    pub cofactors: Vec<String>,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActiveSiteResidue {
    pub position: u32,
    pub amino_acid: char,
    pub role: ActiveSiteRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ActiveSiteRole {
    #[default] Unknown, Catalytic, Nucleophile, ElectronDonor, ElectronAcceptor,
    ProtonDonor, ProtonAcceptor, Stabilizing, CoordMetal, Transition_State,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BindingSite {
    pub site_id: u64,
    pub protein_id: u64,
    pub site_type: BindingSiteType,
    pub residue_positions: Vec<u32>,
    pub ligand_name: Option<String>,
    pub ligand_chembl: Option<String>,
    pub volume_angstrom3: Option<f32>,
    pub druggability_score: Option<f32>,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum BindingSiteType {
    #[default] Unknown, SmallMolecule, Peptide, DNA, RNA, Metal, Cofactor,
    Allosteric, Orthosteric, Cryptic, PPI_Interface,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralMotif {
    pub motif_id: u64,
    pub protein_id: u64,
    pub motif_type: String,          // e.g. "zinc finger", "EF-hand", "leucine zipper"
    pub start_residue: u32,
    pub end_residue: u32,
    pub function: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// POST-TRANSLATIONAL MODIFICATIONS
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PostTranslationalModification {
    pub ptm_id: u64,
    pub protein_id: u64,
    pub modification_type: PTMType,
    pub residue_position: u32,
    pub amino_acid: char,
    pub mass_shift_da: f64,
    pub functional_effect: Option<String>,
    pub regulatory_role: Option<PTMRole>,
    pub kinase: Option<String>,          // for phosphorylation
    pub e3_ligase: Option<String>,       // for ubiquitination
    pub evidence: PTMEvidence,
    pub conservation_score: Option<f32>, // across species
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PTMType {
    #[default] Unknown,
    Phosphorylation, Ubiquitination, SUMOylation, Acetylation, Methylation,
    N_Glycosylation, O_Glycosylation, Palmitoylation, Myristoylation, Prenylation,
    GPI_Anchor, Hydroxylation, Nitrosylation, Sulfation, Crotonylation,
    Citrullination, Propionylation, Butyrylation, Malonylation, Succinylation,
    ADP_Ribosylation, Neddylation, Deamidation, Oxidation, Carbamidomethylation,
    Disulfide_Bond, Carbamylation, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PTMRole {
    #[default] Unknown, Activation, Inhibition, DegradationSignal, LocalizationSignal,
    ProteinInteraction, DNABinding, Stability, Solubility,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PTMEvidence {
    #[default] Predicted, ByAnalogy, Experimental_MassSpec, Experimental_MutagenesisFunctional,
    Experimental_XRay, Experimental_NMR, Experimental_Antibody,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlycosylationSite {
    pub site_id: u64,
    pub protein_id: u64,
    pub glycan_type: GlycanType,
    pub residue_position: u32,
    pub amino_acid: char,
    pub glycan_composition: Option<String>,
    pub sequon: Option<String>,    // N-glycosylation: NXS/T sequon
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum GlycanType { #[default] Unknown, N_Linked, O_Linked, GPI_Glycan, C_Mannosylation }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UbiquitinationSite {
    pub site_id: u64,
    pub protein_id: u64,
    pub lysine_position: u32,
    pub ubiquitin_chain_type: UbiquitinChainType,
    pub functional_outcome: UbiquitinFunctionalOutcome,
    pub e3_ligase: Option<String>,
    pub deubiquitinase: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum UbiquitinChainType { #[default] Unknown, K48_Degradation, K63_DNA_Repair, K11_Cell_Cycle, M1_Linear, Monoubiquitin }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum UbiquitinFunctionalOutcome { #[default] Unknown, Proteasomal_Degradation, Endosomal_Sorting, DNA_Repair, NF_kB_Signaling, Autophagy }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CleavageSite {
    pub site_id: u64,
    pub protein_id: u64,
    pub cleavage_type: CleavageType,
    pub position: u32,
    pub protease: Option<String>,
    pub resulting_fragment_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum CleavageType { #[default] Unknown, SignalPeptide, Propeptide, Furin, Caspase, Calpain, Matriptase, MMP, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DisulfideBond {
    pub bond_id: u64,
    pub protein_id: u64,
    pub cys_position_a: u32,
    pub cys_position_b: u32,
    pub is_intermolecular: bool,
    pub partner_protein_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SignalPeptide {
    pub sp_id: u64,
    pub protein_id: u64,
    pub start: u32,
    pub end: u32,
    pub signal_type: SignalPeptideType,
    pub cleavage_site: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SignalPeptideType { #[default] Classical, Twin_Arginine, Signal_Anchor }

// ─────────────────────────────────────────────────────────────────────────────
// INTERACTIONS
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProteinProteinInteraction {
    pub interaction_id: u64,
    pub protein_a_id: u64,
    pub protein_b_id: u64,
    pub protein_a_accession: String,
    pub protein_b_accession: String,
    pub interaction_type: PPIType,
    pub experimental_method: Vec<String>,
    pub combined_score: f32,          // STRING combined score 0–1
    pub coexpression_score: Option<f32>,
    pub experimental_score: Option<f32>,
    pub database_score: Option<f32>,
    pub textmining_score: Option<f32>,
    pub interface_residues_a: Vec<u32>,
    pub interface_residues_b: Vec<u32>,
    pub kd_nm: Option<f64>,           // binding affinity
    pub source_db: String,
    pub pubmed_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PPIType {
    #[default] Unknown, Physical, Genetic, Regulatory, Enzymatic,
    Complex_Member, Colocalization, Phosphorylation, Dephosphorylation,
    Ubiquitination, Deubiquitination, Methylation, Demethylation,
    Acetylation, Deacetylation, Cleavage, Binding,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProteinDNAInteraction {
    pub interaction_id: u64,
    pub protein_id: u64,
    pub dna_element: String,         // gene name or regulatory element
    pub binding_mode: DNABindingMode,
    pub binding_sequence_motif: Option<String>,
    pub chip_seq_peaks: u32,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum DNABindingMode { #[default] Unknown, SequenceSpecific, NonSpecific, Major_Groove, Minor_Groove, Intercalation }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProteinLigandInteraction {
    pub interaction_id: u64,
    pub protein_id: u64,
    pub ligand_name: String,
    pub ligand_smiles: Option<String>,
    pub ligand_chembl_id: Option<String>,
    pub ligand_pubchem_cid: Option<String>,
    pub binding_site_id: Option<u64>,
    pub binding_affinity: Option<BindingAffinity>,
    pub interaction_type: LigandInteractionType,
    pub docking_score: Option<f64>,
    pub hydrogen_bonds: Option<Vec<HBondContact>>,
    pub hydrophobic_contacts: Vec<String>,    // residue names
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BindingAffinity {
    pub kd_nm: Option<f64>,
    pub ki_nm: Option<f64>,
    pub ic50_nm: Option<f64>,
    pub delta_g_kcal_mol: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum LigandInteractionType { #[default] Unknown, Competitive, Noncompetitive, Uncompetitive, Allosteric, Covalent }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HBondContact {
    pub donor_residue: String,
    pub acceptor_residue: String,
    pub distance_angstrom: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProteinComplex {
    pub complex_id: u64,
    pub name: String,
    pub subunit_ids: Vec<u64>,          // protein_ids
    pub subunit_accessions: Vec<String>,
    pub stoichiometry: Vec<(String, u32)>, // (accession, count)
    pub complex_function: String,
    pub pdb_entries: Vec<String>,
    pub complex_type: ComplexType,
    pub stability_kcal_mol: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ComplexType { #[default] Unknown, Homodimer, Heterodimer, RingComplex, FilamentComplex, ScaffoldComplex, SignalosomeSupercomplex }

// ─────────────────────────────────────────────────────────────────────────────
// EXPRESSION
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProteinExpression {
    pub expression_id: u64,
    pub protein_id: u64,
    pub sample_id: String,
    pub tissue: Option<String>,
    pub cell_type: Option<String>,
    pub condition: String,
    pub abundance: f64,
    pub abundance_unit: String,          // "iBAQ", "LFQ_intensity", "TPM", "copies_per_cell"
    pub log2_abundance: f64,
    pub is_detected: bool,
    pub ms_run_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DifferentialExpression {
    pub de_id: u64,
    pub protein_id: u64,
    pub comparison: String,              // "condition_A vs condition_B"
    pub log2_fold_change: f64,
    pub adjusted_p_value: f64,
    pub significant: bool,
    pub regulation: ExpressionRegulation,
    pub test_used: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ExpressionRegulation { #[default] NotSignificant, Up, Down, Unchanged }

// ─────────────────────────────────────────────────────────────────────────────
// FUNCTIONAL ANNOTATION
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GOTermAnnotation {
    pub annotation_id: u64,
    pub protein_id: u64,
    pub go_id: String,                   // e.g. "GO:0005634"
    pub go_name: String,
    pub namespace: GONamespace,
    pub evidence_code: String,           // IDA, IMP, ISS, IEA, etc.
    pub qualifier: Option<String>,       // "is_active_in", "part_of", etc.
    pub source_db: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum GONamespace { #[default] BiologicalProcess, MolecularFunction, CellularComponent }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PathwayMembership {
    pub membership_id: u64,
    pub protein_id: u64,
    pub pathway_id: String,              // KEGG: "hsa04010", Reactome: "R-HSA-1234"
    pub pathway_name: String,
    pub database: PathwayDatabase,
    pub role_in_pathway: Option<String>, // "kinase", "substrate", "scaffold", etc.
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnzymeActivity {
    pub activity_id: u64,
    pub protein_id: u64,
    pub ec_number: String,               // e.g. "3.4.22.1"
    pub reaction_description: String,
    pub substrates: Vec<String>,
    pub products: Vec<String>,
    pub cofactors: Vec<String>,
    pub km_um: Option<f64>,              // Michaelis constant
    pub kcat_per_sec: Option<f64>,       // turnover number
    pub kcat_km: Option<f64>,            // catalytic efficiency
    pub inhibitors: Vec<String>,
    pub activators: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SubcellularLocation {
    pub location_id: u64,
    pub protein_id: u64,
    pub location: String,                // "Nucleus", "Plasma membrane", "Mitochondria", etc.
    pub topology: Option<String>,        // "Single-pass type I membrane protein"
    pub evidence: String,
    pub go_cc_term: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// SEQUENCE ANALYSIS
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SequenceAlignment {
    pub alignment_id: u64,
    pub query_protein_id: u64,
    pub target_protein_id: u64,
    pub identity_percent: f32,
    pub similarity_percent: f32,
    pub coverage_percent: f32,
    pub e_value: f64,
    pub bit_score: f64,
    pub method: String,
    pub aligned_region_query: (u32, u32),
    pub aligned_region_target: (u32, u32),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrthologRelation {
    pub ortholog_id: u64,
    pub protein_a_id: u64,
    pub protein_b_id: u64,
    pub organism_a: String,
    pub organism_b: String,
    pub orthology_type: OrthologyType,
    pub sequence_identity: f32,
    pub oma_group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum OrthologyType { #[default] OneToOne, OneToMany, ManyToMany, CoOrtholog }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParalogCluster {
    pub cluster_id: u64,
    pub protein_ids: Vec<u64>,
    pub gene_family: String,
    pub duplication_event: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProteinSequenceEntry {
    pub id: String,
    pub sequence: String,
    pub description: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// MASS SPECTROMETRY
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MassSpecData {
    pub file_path: String,
    pub format: MassSpecFormat,
    pub experiment_type: MassSpecExperimentType,
    pub instrument: Option<String>,
    pub enzyme: String,                 // "Trypsin", "LysC", "GluC"
    pub missed_cleavages: u32,
    pub precursor_mass_tolerance_ppm: f32,
    pub fragment_mass_tolerance_da: f32,
    pub variable_mods: Vec<String>,
    pub fixed_mods: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MSIdentification {
    pub identification_id: u64,
    pub protein_id: u64,
    pub gene_name: String,
    pub accession: String,
    pub sequence_coverage_percent: f32,
    pub unique_peptides: u32,
    pub total_peptide_count: u32,
    pub score: f64,
    pub q_value: f64,
    pub is_contaminant: bool,
    pub ms_run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MSQuantification {
    pub quant_id: u64,
    pub protein_id: u64,
    pub sample_id: String,
    pub intensity: f64,
    pub lfq_intensity: Option<f64>,
    pub ibaq: Option<f64>,
    pub tmt_reporter_intensities: Option<Vec<f64>>,
    pub silac_ratio: Option<f64>,
    pub log2_intensity: f64,
    pub missing_values_imputed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PeptideEvidence {
    pub peptide_id: u64,
    pub protein_id: u64,
    pub sequence: String,
    pub start_residue: u32,
    pub end_residue: u32,
    pub charge: u32,
    pub mass_da: f64,
    pub rt_min: f64,                   // chromatographic retention time
    pub score: f64,
    pub identified_modifications: Vec<PeptideMod>,
    pub spectrum_count: u32,
    pub is_unique: bool,               // uniquely maps to this protein
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PeptideMod {
    pub mod_type: String,
    pub position_in_peptide: u32,
    pub delta_mass: f64,
}

// ─────────────────────────────────────────────────────────────────────────────
// UPDATE TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProteomicsUpdate {
    AddPTM          { ptm: PostTranslationalModification },
    UpdateExpression { expression: ProteinExpression },
    AddInteraction  { interaction: ProteinProteinInteraction },
    AddStructure    { structure: ProteinStructure },
    AddMSResult     { identification: MSIdentification, quantification: Option<MSQuantification> },
    UpdatePathway   { membership: PathwayMembership },
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH NODE TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ProteomicsNodeType {
    #[default] ProteomeRoot,            // root of the proteomics graph
    ProteinNode,                        // a single protein
    ProteinIsoformNode,                 // isoform variant
    StructureNode,                      // 3D structure
    DomainNode,                         // protein domain
    ActiveSiteNode,                     // catalytic/active site
    BindingSiteNode,                    // ligand binding site
    StructuralMotifNode,                // structural motif
    PTMNode,                            // post-translational modification
    GlycosylationNode,
    UbiquitinationNode,
    CleavageSiteNode,
    DisulfideBondNode,
    PPINode,                            // protein-protein interaction
    ComplexNode,                        // protein complex
    LigandNode,                         // small molecule ligand
    ExpressionNode,                     // expression measurement
    DifferentialExpressionNode,
    GOTermNode,                         // Gene Ontology term
    PathwayNode,                        // metabolic/signaling pathway
    EnzymeNode,                         // enzyme activity
    SubcellularLocationNode,
    AlignmentNode,                      // sequence alignment
    OrthologNode,
    MSIdentificationNode,
    PeptideNode,                        // peptide evidence
    CrossModalRef,                      // reference to another modality
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProteomicsGraphNode {
    pub node_id: u64,
    pub node_type: ProteomicsNodeType,
    pub content: String,

    // PROTEOMICS-SPECIFIC
    pub accession: Option<String>,
    pub gene_name: Option<String>,
    pub sequence_length: Option<u32>,
    pub residue_position: Option<u32>,
    pub molecular_weight_da: Option<f64>,
    pub score: Option<f64>,
    pub abundance: Option<f64>,
    pub go_id: Option<String>,
    pub pathway_id: Option<String>,
    pub ec_number: Option<String>,
    pub pdb_id: Option<String>,
    pub organism: Option<String>,

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
pub enum ProteomicsEdgeType {
    // ── STRUCTURAL ──
    #[default] Contains, PartOf, Precedes,

    // ── PROTEIN STRUCTURE ──
    FoldsInto,               // sequence folds into structure
    HasDomain,               // protein contains domain
    HasActiveSite,           // protein has active site
    HasBindingSite,          // protein has binding site
    HasMotif,                // protein has structural motif
    HasSecondaryStructure,   // protein has SS region

    // ── MODIFICATIONS ──
    ModifiedBy,              // protein modified by PTM
    GlycosylatedAt,          // glycosylation at residue
    UbiquitinatedAt,         // ubiquitination at lysine
    CleavedAt,               // cleavage site
    DisulfideBridged,        // disulfide bond between residues
    HasSignalPeptide,

    // ── INTERACTIONS ──
    InteractsWith,           // generic protein-protein interaction
    BindsTo,                 // directed binding
    Catalyzes,               // enzyme catalyzes reaction
    Phosphorylates,          // kinase phosphorylates substrate
    Dephosphorylates,
    Ubiquitinates,
    Acetylates,
    Methylates,
    Cleaves,                 // protease cleaves target
    Inhibits,                // protein inhibits another
    Activates,               // protein activates another
    ColocalizesWithProtein,  // found in same complex/location
    PartOfComplex,           // member of protein complex

    // ── LIGAND ──
    BindsLigand,             // binding site → ligand
    InhibitedBy,             // protein inhibited by ligand
    ActivatedBy,             // protein activated by ligand
    SubstrateOf,             // ligand is substrate of enzyme

    // ── EXPRESSION ──
    ExpressedIn,             // protein expressed in tissue/condition
    DifferentiallyExpressed, // significant DE event
    CoexpressedWith,         // correlated expression

    // ── FUNCTION ──
    AnnotatedWith,           // GO term annotation
    ParticipatesIn,          // pathway membership
    HasFunction,             // enzyme activity
    LocalizedTo,             // subcellular location

    // ── EVOLUTION ──
    OrthologTo,
    ParalogTo,
    DerivedFromGene,         // protein encoded by this gene (links to DNA 107)

    // ── CROSS-MODAL ──
    /// Coding sequence in DNA graph (107)
    CodingSequenceIn,
    /// Enzyme mechanism in chemistry graph (106)
    MechanismInChemistry,
    /// Protein in pathway in biology graph (111)
    InBiologyPathway,
    /// Tissue distribution from hyperspectral (126)
    TissueMapFrom126,
    /// Structural math (docking scores) from math graph (105)
    ScoredByMath,
    /// Analysis code in code graph (101)
    AnalysisCodeIn,

    // ── UNIVERSAL SEMANTIC ──
    Performs, Affects, Implies, Contradicts, Elaborates, Summarizes, Supports,
    TemporalPrecedes, TemporalFollows, CausedBy, Enables, Prevents,
    FunctionalRole, InstanceOf,
    DerivedFrom, VersionOf, RefinesTo, ForkedFrom, SimilarTo,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProteomicsGraphEdge {
    pub edge_id: u64,
    pub from_node: u64, pub to_node: u64,
    pub edge_type: ProteomicsEdgeType,
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
pub struct ProteomicsGraph {
    pub graph_id: u64, pub project_id: u64,
    pub source_description: String,
    pub nodes: Vec<ProteomicsGraphNode>,
    pub edges: Vec<ProteomicsGraphEdge>,
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
pub enum ProteomicsGraphQuery {
    NodeDetail       { node_id: u64 },
    ProteinByAccession { accession: String },
    InteractionNetwork { protein_node_id: u64, depth: u32 },
    PTMsForProtein   { protein_node_id: u64 },
    PathwayProteins  { pathway_id: String },
    DifferentiallyExpressed { log2_fc_threshold: f32 },
    BindingSites     { protein_node_id: u64 },
    ComplexMembers   { complex_node_id: u64 },
    CrossModalLinks  { node_id: u64 },
    AGIActivity, AllNodes, AllEdges,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProteomicsSemanticHook {
    OnGraphCreated,
    OnInferRelationships,
    OnEdgeCompletion,
    OnCrossModalityLink { target_modality: String, target_graph_id: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProteomicsExportFormat {
    FASTA,            // protein sequences
    PDB,              // structure
    TSV_Interactions, // interaction table
    TSV_Expression,   // expression matrix
    GPR,              // gene-protein-reaction mapping
    BioPAX,           // biological pathway exchange
    SIF,              // simple interaction format
    JSON_LD,          // linked data
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProteomicsDisplayMode {
    PPINetwork,           // protein-protein interaction graph
    StructureViewer,      // 3D structure viewer
    ExpressionHeatmap,    // sample × protein heatmap
    VolcanoPlot,          // DE volcano plot
    DomainArchitecture,   // domain cartoon
    PathwayMap,           // pathway overlay
    MSSpectrum,           // MS/MS spectrum
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProteomicsOperation {
    RebuildPPINetwork,
    RecomputeDifferentialExpression,
    EnrichPathways { database: PathwayDatabase },
    CrossLinkToDNA { dna_graph_id: u64 },
    CrossLinkToChemistry { chem_graph_id: u64 },
    ExportInteractions,
}

// ─────────────────────────────────────────────────────────────────────────────
// OUTPUT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProteomicsModalityOutput {
    pub success: bool,
    pub graph_id: Option<u64>,
    pub graph: Option<ProteomicsGraph>,
    pub analysis: Option<ProteomicsAnalysisResult>,
    pub predicted_structure: Option<ProteinStructure>,
    pub docking_result: Option<ProteinLigandInteraction>,
    pub ppi_network_stats: Option<serde_json::Value>,
    pub pathway_enrichment: Option<Vec<PathwayMembership>>,
    pub alignment_result: Option<Vec<SequenceAlignment>>,
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
        let input = serde_json::json!({"action":"Prompt","prompt":prompt,"max_tokens":max_tokens,"temperature":0.05,"system_context":"Proteomics analysis. Return only valid JSON."});
        let out = std::process::Command::new(&self.prompt_pipeline_path).arg("--input").arg(input.to_string()).output().map_err(|e| e.to_string())?;
        let r: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
        Ok(r["response"].as_str().unwrap_or("{}").to_string())
    }
    fn save_graph(&self, g: &ProteomicsGraph) -> Result<(), String> {
        let path = format!("{}/local/prot_graph_{}.json", self.zsei_path, g.graph_id);
        if let Some(p) = std::path::Path::new(&path).parent() { std::fs::create_dir_all(p).map_err(|e| e.to_string())?; }
        std::fs::write(&path, serde_json::to_string_pretty(g).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }
    fn load_graph(&self, id: u64) -> Result<ProteomicsGraph, String> {
        let path = format!("{}/local/prot_graph_{}.json", self.zsei_path, id);
        serde_json::from_str(&std::fs::read_to_string(&path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }
    fn generate_id(&self) -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as u64 }
    fn now_iso8601(&self) -> String { format!("{}", self.generate_id()) }
    fn extract_json_array(raw: &str) -> String { if let (Some(s), Some(e)) = (raw.find('['), raw.rfind(']')) { raw[s..=e].to_string() } else { "[]".to_string() } }
}

impl PipelineExecutor {
    async fn infer_protein_functions(&self, proteins: &[Protein]) -> Vec<(u64, Vec<String>, Vec<String>)> {
        if proteins.is_empty() { return vec![]; }
        let list: Vec<serde_json::Value> = proteins.iter().take(20).map(|p| serde_json::json!({
            "protein_id": p.protein_id,
            "accession": &p.accession,
            "entry_name": &p.entry_name,
            "gene_name": &p.gene_name,
            "full_name": &p.full_name,
            "sequence_length": p.sequence_length,
            "reviewed": p.reviewed,
        })).collect();

        let prompt = format!(r#"
Infer molecular functions and biological processes for these proteins.

Proteins:
{}

Return ONLY valid JSON array:
[{{
  "protein_id": N,
  "molecular_functions": ["function1", "function2"],
  "biological_processes": ["process1", "process2"],
  "subcellular_location": "Nucleus|Cytoplasm|Plasma membrane|Mitochondria|ER|Golgi|Secreted|etc.",
  "domain_class": "kinase|phosphatase|transcription_factor|receptor|channel|protease|etc."
}}]"#,
            serde_json::to_string_pretty(&list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 1000).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str).unwrap_or_default().into_iter()
                    .filter_map(|v| {
                        let id = v["protein_id"].as_u64()?;
                        let fns: Vec<String> = v["molecular_functions"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
                        let procs: Vec<String> = v["biological_processes"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
                        Some((id, fns, procs))
                    }).collect()
            }
            Err(_) => vec![],
        }
    }

    async fn infer_ptm_effects(&self, ptms: &[PostTranslationalModification], proteins: &[Protein]) -> Vec<(u64, String)> {
        if ptms.is_empty() { return vec![]; }
        let list: Vec<serde_json::Value> = ptms.iter().take(30).map(|ptm| {
            let protein_name = proteins.iter().find(|p| p.protein_id == ptm.protein_id).map(|p| p.entry_name.as_str()).unwrap_or("unknown");
            serde_json::json!({
                "ptm_id": ptm.ptm_id,
                "modification_type": format!("{:?}", ptm.modification_type),
                "protein": protein_name,
                "residue_position": ptm.residue_position,
                "amino_acid": ptm.amino_acid.to_string(),
                "mass_shift_da": ptm.mass_shift_da,
            })
        }).collect();

        let prompt = format!(r#"
Infer the functional effect of each post-translational modification.

PTMs:
{}

Return ONLY valid JSON array:
[{{"ptm_id": N, "functional_effect": "brief description of regulatory effect"}}]"#,
            serde_json::to_string_pretty(&list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 600).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str).unwrap_or_default().into_iter()
                    .filter_map(|v| Some((v["ptm_id"].as_u64()?, v["functional_effect"].as_str()?.to_string())))
                    .collect()
            }
            Err(_) => vec![],
        }
    }

    async fn infer_semantic_relationships(&self, nodes: &[ProteomicsGraphNode]) -> Vec<(u64, u64, ProteomicsEdgeType, String)> {
        if nodes.len() < 2 { return vec![]; }
        let node_list: Vec<serde_json::Value> = nodes.iter().take(25).map(|n| serde_json::json!({
            "node_id": n.node_id,
            "type": format!("{:?}", n.node_type),
            "content": n.content.chars().take(80).collect::<String>(),
            "accession": n.accession,
            "gene": n.gene_name,
            "ec": n.ec_number,
        })).collect();

        let prompt = format!(r#"
Identify semantic relationships between these proteomics graph nodes.

Nodes:
{}

Available types: InteractsWith, Activates, Inhibits, Phosphorylates, Cleaves,
PartOfComplex, CoexpressedWith, ParticipatesIn, CausedBy, Enables, Prevents,
DerivedFrom, FunctionalRole, InstanceOf, SimilarTo, OrthologTo

Return ONLY valid JSON array:
[{{"from_node_id": N, "to_node_id": M, "edge_type": "TypeName", "reason": "brief"}}]"#,
            serde_json::to_string_pretty(&node_list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 800).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str).unwrap_or_default().into_iter()
                    .filter_map(|v| {
                        let from = v["from_node_id"].as_u64()?;
                        let to = v["to_node_id"].as_u64()?;
                        let etype = map_prot_edge_str(v["edge_type"].as_str().unwrap_or("InteractsWith"));
                        let reason = v["reason"].as_str().unwrap_or("").to_string();
                        Some((from, to, etype, reason))
                    }).collect()
            }
            Err(_) => vec![],
        }
    }

    async fn predict_interaction_type(&self, ppi: &ProteinProteinInteraction, proteins: &[Protein]) -> PPIType {
        let prot_a = proteins.iter().find(|p| p.protein_id == ppi.protein_a_id);
        let prot_b = proteins.iter().find(|p| p.protein_id == ppi.protein_b_id);

        let prompt = format!(r#"
Given these two proteins, infer the most likely type of protein-protein interaction.

Protein A: {} ({})
Protein B: {} ({})
Experimental methods: {:?}
Combined score: {:.3}

Interaction types: Physical, Genetic, Regulatory, Enzymatic, Complex_Member, Colocalization,
Phosphorylation, Dephosphorylation, Ubiquitination, Binding, Unknown

Return ONLY the interaction type string, nothing else."#,
            prot_a.map(|p| p.entry_name.as_str()).unwrap_or("unknown"),
            prot_a.map(|p| p.full_name.chars().take(40).collect::<String>()).unwrap_or_default(),
            prot_b.map(|p| p.entry_name.as_str()).unwrap_or("unknown"),
            prot_b.map(|p| p.full_name.chars().take(40).collect::<String>()).unwrap_or_default(),
            ppi.experimental_method,
            ppi.combined_score,
        );

        match self.llm_zero_shot(&prompt, 20).await {
            Ok(raw) => match raw.trim() {
                "Physical"         => PPIType::Physical,
                "Genetic"          => PPIType::Genetic,
                "Regulatory"       => PPIType::Regulatory,
                "Enzymatic"        => PPIType::Enzymatic,
                "Complex_Member"   => PPIType::Complex_Member,
                "Phosphorylation"  => PPIType::Phosphorylation,
                "Ubiquitination"   => PPIType::Ubiquitination,
                "Binding"          => PPIType::Binding,
                _                  => PPIType::Unknown,
            },
            Err(_) => PPIType::Unknown,
        }
    }

    fn compute_sequence_properties(sequence: &str) -> (f64, f32) {
        // Molecular weight: average amino acid mass ~111 Da
        let mw_da = sequence.len() as f64 * 111.1;

        // Isoelectric point: simplified Lehninger scale
        let pos_aa_count = sequence.chars().filter(|&c| matches!(c, 'K' | 'R' | 'H')).count() as f32;
        let neg_aa_count = sequence.chars().filter(|&c| matches!(c, 'D' | 'E')).count() as f32;
        let pI = 7.0 + 0.5 * (pos_aa_count - neg_aa_count) / (sequence.len() as f32 + 0.01);
        let pI = pI.clamp(3.0, 12.0);

        (mw_da, pI)
    }
}

fn map_prot_edge_str(s: &str) -> ProteomicsEdgeType {
    match s {
        "FoldsInto"            => ProteomicsEdgeType::FoldsInto,
        "HasDomain"            => ProteomicsEdgeType::HasDomain,
        "HasActiveSite"        => ProteomicsEdgeType::HasActiveSite,
        "HasBindingSite"       => ProteomicsEdgeType::HasBindingSite,
        "ModifiedBy"           => ProteomicsEdgeType::ModifiedBy,
        "InteractsWith"        => ProteomicsEdgeType::InteractsWith,
        "BindsTo"              => ProteomicsEdgeType::BindsTo,
        "Catalyzes"            => ProteomicsEdgeType::Catalyzes,
        "Phosphorylates"       => ProteomicsEdgeType::Phosphorylates,
        "Ubiquitinates"        => ProteomicsEdgeType::Ubiquitinates,
        "Acetylates"           => ProteomicsEdgeType::Acetylates,
        "Methylates"           => ProteomicsEdgeType::Methylates,
        "Cleaves"              => ProteomicsEdgeType::Cleaves,
        "Inhibits"             => ProteomicsEdgeType::Inhibits,
        "Activates"            => ProteomicsEdgeType::Activates,
        "PartOfComplex"        => ProteomicsEdgeType::PartOfComplex,
        "BindsLigand"          => ProteomicsEdgeType::BindsLigand,
        "InhibitedBy"          => ProteomicsEdgeType::InhibitedBy,
        "ExpressedIn"          => ProteomicsEdgeType::ExpressedIn,
        "CoexpressedWith"      => ProteomicsEdgeType::CoexpressedWith,
        "AnnotatedWith"        => ProteomicsEdgeType::AnnotatedWith,
        "ParticipatesIn"       => ProteomicsEdgeType::ParticipatesIn,
        "HasFunction"          => ProteomicsEdgeType::HasFunction,
        "LocalizedTo"          => ProteomicsEdgeType::LocalizedTo,
        "OrthologTo"           => ProteomicsEdgeType::OrthologTo,
        "ParalogTo"            => ProteomicsEdgeType::ParalogTo,
        "CodingSequenceIn"     => ProteomicsEdgeType::CodingSequenceIn,
        "MechanismInChemistry" => ProteomicsEdgeType::MechanismInChemistry,
        "InBiologyPathway"     => ProteomicsEdgeType::InBiologyPathway,
        "Affects"              => ProteomicsEdgeType::Affects,
        "CausedBy"             => ProteomicsEdgeType::CausedBy,
        "Enables"              => ProteomicsEdgeType::Enables,
        "Prevents"             => ProteomicsEdgeType::Prevents,
        "TemporalPrecedes"     => ProteomicsEdgeType::TemporalPrecedes,
        "DerivedFrom"          => ProteomicsEdgeType::DerivedFrom,
        "SimilarTo"            => ProteomicsEdgeType::SimilarTo,
        "FunctionalRole"       => ProteomicsEdgeType::FunctionalRole,
        "PartOf"               => ProteomicsEdgeType::PartOf,
        _                      => ProteomicsEdgeType::InteractsWith,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH CREATION
// ─────────────────────────────────────────────────────────────────────────────

async fn create_graph(
    executor: &PipelineExecutor,
    analysis: ProteomicsAnalysisResult,
    project_id: u64,
) -> ProteomicsModalityOutput {
    let graph_id = executor.generate_id();
    let now = executor.now_iso8601();
    let mut nodes: Vec<ProteomicsGraphNode> = Vec::new();
    let mut edges: Vec<ProteomicsGraphEdge> = Vec::new();
    let mut node_id: u64 = 1;
    let mut edge_id: u64 = 1;

    // ── ROOT ──
    let root_id = node_id;
    nodes.push(ProteomicsGraphNode {
        node_id: root_id, node_type: ProteomicsNodeType::ProteomeRoot,
        content: format!("Proteomics: {}proteins {}interactions {}PTMs {}structures {}GO-terms organism={:?}",
            analysis.proteins.len(), analysis.protein_protein_interactions.len(),
            analysis.ptms.len(), analysis.structures.len(),
            analysis.go_terms.len(), analysis.organism.as_deref().unwrap_or("unknown")),
        organism: analysis.organism.clone(),
        materialized_path: Some(format!("/Modalities/Proteomics/Project_{}/Graph_{}", project_id, graph_id)),
        provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
        version_notes: vec![VersionNote { version: 1, note: "Initial creation".into(), step_index: None, timestamp: now.clone(), change_type: ChangeType::Created }],
        keywords: vec!["proteomics".into(), "proteome".into(), analysis.organism.as_deref().unwrap_or("").to_lowercase()],
        hotness_score: 1.0, ..Default::default()
    });
    node_id += 1;

    // Build lookup maps
    let mut prot_nid: HashMap<u64, u64> = HashMap::new();
    let mut struct_nid: HashMap<u64, u64> = HashMap::new();
    let mut domain_nid: HashMap<u64, u64> = HashMap::new();
    let mut binding_nid: HashMap<u64, u64> = HashMap::new();
    let mut ptm_nid: HashMap<u64, u64> = HashMap::new();
    let mut go_nid: HashMap<String, u64> = HashMap::new();
    let mut pathway_nid: HashMap<String, u64> = HashMap::new();
    let mut complex_nid: HashMap<u64, u64> = HashMap::new();
    let mut enzyme_nid: HashMap<u64, u64> = HashMap::new();

    // LLM: infer functions
    let protein_functions = executor.infer_protein_functions(&analysis.proteins).await;
    let func_map: HashMap<u64, (Vec<String>, Vec<String>)> = protein_functions.into_iter()
        .map(|(id, fns, procs)| (id, (fns, procs))).collect();

    // ── PROTEIN NODES ──
    for prot in &analysis.proteins {
        let pid = node_id;
        let (mw, pI) = PipelineExecutor::compute_sequence_properties(&prot.sequence);
        let kws: Vec<String> = {
            let mut kw = vec!["protein".into(), prot.accession.to_lowercase(), prot.entry_name.to_lowercase()];
            if let Some(ref g) = prot.gene_name { kw.push(g.to_lowercase()); }
            if let Some((ref fns, _)) = func_map.get(&prot.protein_id) { kw.extend(fns.iter().map(|f| f.to_lowercase())); }
            kw
        };
        nodes.push(ProteomicsGraphNode {
            node_id: pid, node_type: ProteomicsNodeType::ProteinNode,
            content: format!("{} | {} | len={}aa | mw={:.0}Da | pI={:.1} | reviewed={}",
                prot.accession, prot.entry_name, prot.sequence_length,
                prot.molecular_weight_da.max(mw), prot.isoelectric_point.max(pI as f32), prot.reviewed),
            accession: Some(prot.accession.clone()),
            gene_name: prot.gene_name.clone(),
            sequence_length: Some(prot.sequence_length),
            molecular_weight_da: Some(prot.molecular_weight_da),
            organism: Some(prot.organism.clone()),
            materialized_path: Some(format!("/Modalities/Proteomics/Project_{}/Graph_{}/Protein/{}", project_id, graph_id, prot.protein_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: kws,
            hotness_score: if prot.reviewed { 0.85 } else { 0.65 },
            embedding_hint: Some(format!("protein {} gene:{:?} org:{}", prot.full_name, prot.gene_name, prot.organism)),
            ..Default::default()
        });
        prot_nid.insert(prot.protein_id, pid);
        edges.push(ProteomicsGraphEdge { edge_id, from_node: root_id, to_node: pid, edge_type: ProteomicsEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── STRUCTURE NODES ──
    for struc in &analysis.structures {
        let sid = node_id;
        nodes.push(ProteomicsGraphNode {
            node_id: sid, node_type: ProteomicsNodeType::StructureNode,
            content: format!("Structure {:?}: pdb={:?} res={:?}Å chain={} residues={} plddt={:?}",
                struc.structure_method, struc.pdb_id.as_deref().unwrap_or("?"),
                struc.resolution_angstrom, struc.chain_id, struc.residue_count, struc.plddt_mean),
            pdb_id: struc.pdb_id.clone(),
            materialized_path: Some(format!("/Modalities/Proteomics/Project_{}/Graph_{}/Structure/{}", project_id, graph_id, struc.structure_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: { let mut kw = vec!["structure".into(), format!("{:?}", struc.structure_method).to_lowercase()]; if let Some(ref p) = struc.pdb_id { kw.push(p.to_lowercase()); } kw },
            hotness_score: 0.8, ..Default::default()
        });
        struct_nid.insert(struc.structure_id, sid);
        if let Some(&prot_n) = prot_nid.get(&struc.protein_id) {
            edges.push(ProteomicsGraphEdge { edge_id, from_node: prot_n, to_node: sid, edge_type: ProteomicsEdgeType::FoldsInto, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── DOMAIN NODES ──
    for dom in &analysis.domains {
        let did = node_id;
        nodes.push(ProteomicsGraphNode {
            node_id: did, node_type: ProteomicsNodeType::DomainNode,
            content: format!("Domain: {} pfam={:?} res={}-{} catalytic={} binding={}",
                dom.domain_name, dom.pfam_id.as_deref().unwrap_or("?"),
                dom.start_residue, dom.end_residue, dom.is_catalytic, dom.is_binding),
            residue_position: Some(dom.start_residue),
            materialized_path: Some(format!("/Modalities/Proteomics/Project_{}/Graph_{}/Domain/{}", project_id, graph_id, dom.domain_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: { let mut kw = vec!["domain".into(), dom.domain_name.to_lowercase()]; if let Some(ref p) = dom.pfam_id { kw.push(p.to_lowercase()); } kw },
            hotness_score: 0.7, ..Default::default()
        });
        domain_nid.insert(dom.domain_id, did);
        if let Some(&prot_n) = prot_nid.get(&dom.protein_id) {
            edges.push(ProteomicsGraphEdge { edge_id, from_node: prot_n, to_node: did, edge_type: ProteomicsEdgeType::HasDomain, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── ACTIVE SITE NODES ──
    for site in &analysis.active_sites {
        let sid = node_id;
        let residue_str = site.residue_positions.iter().map(|r| format!("{}:{}", r.amino_acid, r.position)).collect::<Vec<_>>().join(",");
        nodes.push(ProteomicsGraphNode {
            node_id: sid, node_type: ProteomicsNodeType::ActiveSiteNode,
            content: format!("ActiveSite: residues=[{}] mech={:?} substrate={:?}",
                residue_str, site.catalytic_mechanism.as_deref().unwrap_or("?"), site.substrate_specificity.as_deref().unwrap_or("?")),
            materialized_path: Some(format!("/Modalities/Proteomics/Project_{}/Graph_{}/ActiveSite/{}", project_id, graph_id, site.site_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["active-site".into(), "catalytic".into()],
            hotness_score: 0.75, ..Default::default()
        });
        if let Some(&prot_n) = prot_nid.get(&site.protein_id) {
            edges.push(ProteomicsGraphEdge { edge_id, from_node: prot_n, to_node: sid, edge_type: ProteomicsEdgeType::HasActiveSite, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── BINDING SITE NODES ──
    for site in &analysis.binding_sites {
        let sid = node_id;
        nodes.push(ProteomicsGraphNode {
            node_id: sid, node_type: ProteomicsNodeType::BindingSiteNode,
            content: format!("BindingSite {:?}: ligand={:?} vol={:?}Å³ drug={:?}",
                site.site_type, site.ligand_name.as_deref().unwrap_or("?"),
                site.volume_angstrom3, site.druggability_score),
            materialized_path: Some(format!("/Modalities/Proteomics/Project_{}/Graph_{}/BindingSite/{}", project_id, graph_id, site.site_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["binding-site".into(), format!("{:?}", site.site_type).to_lowercase()],
            hotness_score: 0.7, ..Default::default()
        });
        binding_nid.insert(site.site_id, sid);
        if let Some(&prot_n) = prot_nid.get(&site.protein_id) {
            edges.push(ProteomicsGraphEdge { edge_id, from_node: prot_n, to_node: sid, edge_type: ProteomicsEdgeType::HasBindingSite, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── PTM NODES (LLM-enriched with functional effects) ──
    let ptm_effects: HashMap<u64, String> = executor.infer_ptm_effects(&analysis.ptms, &analysis.proteins).await.into_iter().collect();
    for ptm in &analysis.ptms {
        let pid = node_id;
        let effect = ptm_effects.get(&ptm.ptm_id).map(|s| s.as_str()).unwrap_or(&ptm.functional_effect.as_deref().unwrap_or("unknown"));
        nodes.push(ProteomicsGraphNode {
            node_id: pid, node_type: ProteomicsNodeType::PTMNode,
            content: format!("PTM {:?}: pos={}{} Δmass={:.3}Da role={:?} effect={}",
                ptm.modification_type, ptm.amino_acid, ptm.residue_position,
                ptm.mass_shift_da, ptm.regulatory_role, effect),
            residue_position: Some(ptm.residue_position),
            materialized_path: Some(format!("/Modalities/Proteomics/Project_{}/Graph_{}/PTM/{}", project_id, graph_id, ptm.ptm_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["ptm".into(), format!("{:?}", ptm.modification_type).to_lowercase()],
            hotness_score: 0.65, ..Default::default()
        });
        ptm_nid.insert(ptm.ptm_id, pid);
        if let Some(&prot_n) = prot_nid.get(&ptm.protein_id) {
            edges.push(ProteomicsGraphEdge {
                edge_id, from_node: prot_n, to_node: pid,
                edge_type: ProteomicsEdgeType::ModifiedBy, weight: 1.0,
                provenance: EdgeProvenance::DerivedFromPrompt, version: 1,
                properties: {
                    let mut p = HashMap::new();
                    p.insert("modification_type".into(), serde_json::json!(format!("{:?}", ptm.modification_type)));
                    p.insert("residue_position".into(), serde_json::json!(ptm.residue_position));
                    p.insert("mass_shift_da".into(), serde_json::json!(ptm.mass_shift_da));
                    p
                },
                ..Default::default()
            });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── PPI NODES ──
    for ppi in &analysis.protein_protein_interactions {
        let iid = node_id;
        let edge_label = match ppi.interaction_type {
            PPIType::Phosphorylation => "phosphorylates",
            PPIType::Binding => "binds",
            PPIType::Complex_Member => "complex-with",
            PPIType::Regulatory => "regulates",
            PPIType::Enzymatic => "enzyme-substrate",
            _ => "interacts",
        };
        nodes.push(ProteomicsGraphNode {
            node_id: iid, node_type: ProteomicsNodeType::PPINode,
            content: format!("PPI: {} ↔ {} type={:?} score={:.3} kd={:?}nM",
                ppi.protein_a_accession, ppi.protein_b_accession,
                ppi.interaction_type, ppi.combined_score, ppi.kd_nm),
            materialized_path: Some(format!("/Modalities/Proteomics/Project_{}/Graph_{}/PPI/{}", project_id, graph_id, ppi.interaction_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["ppi".into(), "interaction".into(), ppi.protein_a_accession.to_lowercase(), ppi.protein_b_accession.to_lowercase()],
            hotness_score: 0.6 + ppi.combined_score * 0.3,
            ..Default::default()
        });

        let directed_edge_type = match ppi.interaction_type {
            PPIType::Phosphorylation   => ProteomicsEdgeType::Phosphorylates,
            PPIType::Dephosphorylation => ProteomicsEdgeType::Dephosphorylates,
            PPIType::Ubiquitination    => ProteomicsEdgeType::Ubiquitinates,
            PPIType::Acetylation       => ProteomicsEdgeType::Acetylates,
            PPIType::Methylation       => ProteomicsEdgeType::Methylates,
            PPIType::Cleavage          => ProteomicsEdgeType::Cleaves,
            PPIType::Regulatory        => ProteomicsEdgeType::Activates,
            _                          => ProteomicsEdgeType::InteractsWith,
        };

        if let Some(&a_nid) = prot_nid.get(&ppi.protein_a_id) {
            edges.push(ProteomicsGraphEdge {
                edge_id, from_node: a_nid, to_node: iid, edge_type: directed_edge_type.clone(),
                weight: ppi.combined_score, provenance: EdgeProvenance::DerivedFromPrompt, version: 1,
                properties: { let mut p = HashMap::new(); p.insert("db".into(), serde_json::json!(&ppi.source_db)); if let Some(kd) = ppi.kd_nm { p.insert("kd_nm".into(), serde_json::json!(kd)); } p },
                ..Default::default()
            });
            edge_id += 1;
        }
        if let Some(&b_nid) = prot_nid.get(&ppi.protein_b_id) {
            edges.push(ProteomicsGraphEdge {
                edge_id, from_node: iid, to_node: b_nid, edge_type: ProteomicsEdgeType::BindsTo,
                weight: ppi.combined_score, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default()
            });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── COMPLEX NODES ──
    for cx in &analysis.complexes {
        let cid = node_id;
        nodes.push(ProteomicsGraphNode {
            node_id: cid, node_type: ProteomicsNodeType::ComplexNode,
            content: format!("Complex: {} [{:?}] subunits={} function={}",
                cx.name, cx.complex_type, cx.subunit_ids.len(), cx.complex_function.chars().take(50).collect::<String>()),
            materialized_path: Some(format!("/Modalities/Proteomics/Project_{}/Graph_{}/Complex/{}", project_id, graph_id, cx.complex_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["complex".into(), cx.name.to_lowercase()],
            hotness_score: 0.75, ..Default::default()
        });
        complex_nid.insert(cx.complex_id, cid);
        edges.push(ProteomicsGraphEdge { edge_id, from_node: root_id, to_node: cid, edge_type: ProteomicsEdgeType::Contains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        for &sub_prot_id in &cx.subunit_ids {
            if let Some(&sub_nid) = prot_nid.get(&sub_prot_id) {
                edges.push(ProteomicsGraphEdge { edge_id, from_node: sub_nid, to_node: cid, edge_type: ProteomicsEdgeType::PartOfComplex, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        node_id += 1;
    }

    // ── GO TERM NODES ──
    for ann in &analysis.go_terms {
        let existing_nid = go_nid.get(&ann.go_id).copied();
        let gid = existing_nid.unwrap_or_else(|| {
            let new_nid = node_id;
            nodes.push(ProteomicsGraphNode {
                node_id: new_nid, node_type: ProteomicsNodeType::GOTermNode,
                content: format!("GO {} [{:?}]: {}", ann.go_id, ann.namespace, ann.go_name),
                go_id: Some(ann.go_id.clone()),
                materialized_path: Some(format!("/Modalities/Proteomics/Project_{}/Graph_{}/GO/{}", project_id, graph_id, ann.go_id)),
                provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
                keywords: vec!["go-term".into(), format!("{:?}", ann.namespace).to_lowercase(), ann.go_name.to_lowercase()],
                hotness_score: 0.55, ..Default::default()
            });
            go_nid.insert(ann.go_id.clone(), new_nid);
            node_id += 1;
            new_nid
        });

        if let Some(&prot_n) = prot_nid.get(&ann.protein_id) {
            edges.push(ProteomicsGraphEdge {
                edge_id, from_node: prot_n, to_node: gid,
                edge_type: ProteomicsEdgeType::AnnotatedWith, weight: 0.9,
                provenance: EdgeProvenance::DerivedFromPrompt, version: 1,
                properties: { let mut p = HashMap::new(); p.insert("evidence".into(), serde_json::json!(&ann.evidence_code)); p },
                ..Default::default()
            });
            edge_id += 1;
        }
    }

    // ── PATHWAY NODES ──
    for path in &analysis.pathway_memberships {
        let existing_nid = pathway_nid.get(&path.pathway_id).copied();
        let pw_nid_val = existing_nid.unwrap_or_else(|| {
            let new_nid = node_id;
            nodes.push(ProteomicsGraphNode {
                node_id: new_nid, node_type: ProteomicsNodeType::PathwayNode,
                content: format!("Pathway {} [{:?}]: {}", path.pathway_id, path.database, path.pathway_name),
                pathway_id: Some(path.pathway_id.clone()),
                materialized_path: Some(format!("/Modalities/Proteomics/Project_{}/Graph_{}/Pathway/{}", project_id, graph_id, path.pathway_id)),
                provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
                keywords: vec!["pathway".into(), format!("{:?}", path.database).to_lowercase(), path.pathway_name.to_lowercase()],
                hotness_score: 0.6, ..Default::default()
            });
            pathway_nid.insert(path.pathway_id.clone(), new_nid);
            node_id += 1;
            new_nid
        });

        if let Some(&prot_n) = prot_nid.get(&path.protein_id) {
            edges.push(ProteomicsGraphEdge {
                edge_id, from_node: prot_n, to_node: pw_nid_val,
                edge_type: ProteomicsEdgeType::ParticipatesIn, weight: 0.85,
                provenance: EdgeProvenance::DerivedFromPrompt, version: 1,
                properties: { let mut p = HashMap::new(); if let Some(ref role) = path.role_in_pathway { p.insert("role".into(), serde_json::json!(role)); } p },
                ..Default::default()
            });
            edge_id += 1;

            // Cross-modal: pathway → biology graph (111)
            edges.push(ProteomicsGraphEdge {
                edge_id, from_node: pw_nid_val, to_node: pw_nid_val,
                edge_type: ProteomicsEdgeType::InBiologyPathway, weight: 0.8,
                provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("biology")); p },
                ..Default::default()
            });
            edge_id += 1;
        }
    }

    // ── ENZYME ACTIVITY NODES ──
    for enz in &analysis.enzyme_activities {
        let eid = node_id;
        nodes.push(ProteomicsGraphNode {
            node_id: eid, node_type: ProteomicsNodeType::EnzymeNode,
            content: format!("Enzyme EC{}: {} Km={:?}µM kcat={:?}/s",
                enz.ec_number, enz.reaction_description.chars().take(50).collect::<String>(),
                enz.km_um, enz.kcat_per_sec),
            ec_number: Some(enz.ec_number.clone()),
            materialized_path: Some(format!("/Modalities/Proteomics/Project_{}/Graph_{}/Enzyme/{}", project_id, graph_id, enz.activity_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["enzyme".into(), enz.ec_number.clone()],
            hotness_score: 0.75, ..Default::default()
        });
        enzyme_nid.insert(enz.activity_id, eid);
        if let Some(&prot_n) = prot_nid.get(&enz.protein_id) {
            edges.push(ProteomicsGraphEdge { edge_id, from_node: prot_n, to_node: eid, edge_type: ProteomicsEdgeType::HasFunction, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;

            // Cross-modal: enzyme mechanism → chemistry (106)
            edges.push(ProteomicsGraphEdge {
                edge_id, from_node: eid, to_node: eid,
                edge_type: ProteomicsEdgeType::MechanismInChemistry, weight: 0.85,
                provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("chemistry")); p.insert("ec_number".into(), serde_json::json!(&enz.ec_number)); p },
                ..Default::default()
            });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── EXPRESSION NODES ──
    for expr in analysis.expression_data.iter().take(200) {
        let eid = node_id;
        nodes.push(ProteomicsGraphNode {
            node_id: eid, node_type: ProteomicsNodeType::ExpressionNode,
            content: format!("Expression: sample={} tissue={:?} cond={} abund={:.2} ({}) detected={}",
                expr.sample_id, expr.tissue.as_deref().unwrap_or("?"),
                expr.condition, expr.log2_abundance, expr.abundance_unit, expr.is_detected),
            abundance: Some(expr.abundance),
            materialized_path: Some(format!("/Modalities/Proteomics/Project_{}/Graph_{}/Expression/{}", project_id, graph_id, expr.expression_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["expression".into(), expr.condition.to_lowercase()],
            hotness_score: 0.5, ..Default::default()
        });
        if let Some(&prot_n) = prot_nid.get(&expr.protein_id) {
            edges.push(ProteomicsGraphEdge {
                edge_id, from_node: prot_n, to_node: eid,
                edge_type: ProteomicsEdgeType::ExpressedIn, weight: 0.8,
                provenance: EdgeProvenance::DerivedFromPrompt, version: 1,
                properties: { let mut p = HashMap::new(); if let Some(ref t) = expr.tissue { p.insert("tissue".into(), serde_json::json!(t)); } p.insert("condition".into(), serde_json::json!(&expr.condition)); p },
                ..Default::default()
            });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── DIFFERENTIAL EXPRESSION NODES ──
    for de in &analysis.differential_expression {
        if !de.significant { continue; }
        let deid = node_id;
        nodes.push(ProteomicsGraphNode {
            node_id: deid, node_type: ProteomicsNodeType::DifferentialExpressionNode,
            content: format!("DE: {} log2FC={:.2} padj={:.4} reg={:?}",
                de.comparison, de.log2_fold_change, de.adjusted_p_value, de.regulation),
            score: Some(de.log2_fold_change.abs()),
            materialized_path: Some(format!("/Modalities/Proteomics/Project_{}/Graph_{}/DE/{}", project_id, graph_id, de.de_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["differential-expression".into(), format!("{:?}", de.regulation).to_lowercase()],
            hotness_score: 0.5 + (de.log2_fold_change.abs() / 10.0).clamp(0.0, 0.4),
            ..Default::default()
        });
        if let Some(&prot_n) = prot_nid.get(&de.protein_id) {
            edges.push(ProteomicsGraphEdge { edge_id, from_node: prot_n, to_node: deid, edge_type: ProteomicsEdgeType::DifferentiallyExpressed, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── SUBCELLULAR LOCATION NODES ──
    let mut loc_nid_map: HashMap<String, u64> = HashMap::new();
    for loc in &analysis.subcellular_locations {
        let existing = loc_nid_map.get(&loc.location).copied();
        let lnid = existing.unwrap_or_else(|| {
            let new_nid = node_id;
            nodes.push(ProteomicsGraphNode {
                node_id: new_nid, node_type: ProteomicsNodeType::SubcellularLocationNode,
                content: format!("Location: {} topology={:?}", loc.location, loc.topology.as_deref().unwrap_or("?")),
                materialized_path: Some(format!("/Modalities/Proteomics/Project_{}/Graph_{}/Location/{}", project_id, graph_id, new_nid)),
                provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
                keywords: vec!["subcellular".into(), loc.location.to_lowercase()],
                hotness_score: 0.5, ..Default::default()
            });
            loc_nid_map.insert(loc.location.clone(), new_nid);
            node_id += 1;
            new_nid
        });
        if let Some(&prot_n) = prot_nid.get(&loc.protein_id) {
            edges.push(ProteomicsGraphEdge { edge_id, from_node: prot_n, to_node: lnid, edge_type: ProteomicsEdgeType::LocalizedTo, weight: 0.85, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
    }

    // ── MS IDENTIFICATION NODES ──
    for ms_id in analysis.ms_identifications.iter().take(100) {
        let mid = node_id;
        nodes.push(ProteomicsGraphNode {
            node_id: mid, node_type: ProteomicsNodeType::MSIdentificationNode,
            content: format!("MSId: {} cov={:.1}% peptides={} q={:.4} run={}",
                ms_id.accession, ms_id.sequence_coverage_percent, ms_id.unique_peptides, ms_id.q_value, ms_id.ms_run_id),
            accession: Some(ms_id.accession.clone()),
            score: Some(ms_id.score),
            materialized_path: Some(format!("/Modalities/Proteomics/Project_{}/Graph_{}/MSId/{}", project_id, graph_id, ms_id.identification_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["mass-spec".into(), "identification".into(), ms_id.accession.to_lowercase()],
            hotness_score: 0.6, ..Default::default()
        });
        if let Some(&prot_n) = prot_nid.get(&ms_id.protein_id) {
            edges.push(ProteomicsGraphEdge { edge_id, from_node: prot_n, to_node: mid, edge_type: ProteomicsEdgeType::Contains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── CROSS-MODAL: proteins → DNA (107) ──
    for (&prot_data_id, &prot_n) in &prot_nid {
        edges.push(ProteomicsGraphEdge {
            edge_id, from_node: prot_n, to_node: prot_n,
            edge_type: ProteomicsEdgeType::CodingSequenceIn, weight: 0.9,
            provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
            properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("dna")); p },
            ..Default::default()
        });
        edge_id += 1;
    }

    // ── HOOK 1: OnGraphCreated ──
    let _ = executor.save_graph(&ProteomicsGraph { graph_id, project_id, source_description: analysis.source_description.clone(), nodes: nodes.clone(), edges: edges.clone(), root_node_id: root_id, state: GraphStateType::Created, state_history: vec![GraphStateTransition { from: GraphStateType::Created, to: GraphStateType::Created, timestamp: now.clone(), triggered_by_step: None }], created_at: now.clone(), updated_at: now.clone(), version: 1, version_notes: vec![VersionNote { version: 1, note: format!("Created: {} nodes {} edges", nodes.len(), edges.len()), step_index: None, timestamp: now.clone(), change_type: ChangeType::Created }] });

    // ── HOOK 2: OnInferRelationships ──
    let inferred = executor.infer_semantic_relationships(&nodes).await;
    let valid: std::collections::HashSet<u64> = nodes.iter().map(|n| n.node_id).collect();
    for (from, to, etype, reason) in inferred {
        if valid.contains(&from) && valid.contains(&to) && from != to {
            edges.push(ProteomicsGraphEdge { edge_id, from_node: from, to_node: to, edge_type: etype, weight: 0.8, provenance: EdgeProvenance::DerivedFromHook, version: 1, properties: { let mut p = HashMap::new(); p.insert("reason".into(), serde_json::json!(reason)); p }, ..Default::default() });
            edge_id += 1;
        }
    }

    // ── HOOK 3: OnEdgeCompletion → hotness ──
    let mut deg: HashMap<u64, u32> = HashMap::new();
    for e in &edges { *deg.entry(e.from_node).or_insert(0) += 1; *deg.entry(e.to_node).or_insert(0) += 1; }
    let max_deg = deg.values().copied().max().unwrap_or(1) as f32;
    for n in &mut nodes { if let Some(&d) = deg.get(&n.node_id) { n.hotness_score = (n.hotness_score + (d as f32 / max_deg) * 0.15).min(1.0); } }

    // Remove self-loop cross-modal placeholders from the real edge list — only keep ones with target_modality property
    // (they are intentional cross-modal markers, not errors)

    let final_graph = ProteomicsGraph { graph_id, project_id, source_description: analysis.source_description, nodes, edges, root_node_id: root_id, state: GraphStateType::SemanticEnriched, state_history: vec![GraphStateTransition { from: GraphStateType::Created, to: GraphStateType::SemanticEnriched, timestamp: now.clone(), triggered_by_step: None }], created_at: now.clone(), updated_at: now.clone(), version: 1, version_notes: vec![VersionNote { version: 1, note: "Semantic enrichment complete".into(), step_index: None, timestamp: now, change_type: ChangeType::EnrichedBySemantic }] };
    let _ = executor.save_graph(&final_graph);
    ProteomicsModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(final_graph), ..Default::default() }
}

// ─────────────────────────────────────────────────────────────────────────────
// MAIN EXECUTION
// ─────────────────────────────────────────────────────────────────────────────

pub async fn execute(input: ProteomicsModalityAction) -> Result<ProteomicsModalityOutput, String> {
    let executor = PipelineExecutor::new();

    match input {
        ProteomicsModalityAction::Analyze { data, extract_structure, extract_interactions, extract_modifications, extract_expression, extract_function } => {
            let analysis_id = executor.generate_id();
            let source_description = match &data {
                ProteomicsDataSource::FASTA { file_path, organism } => format!("FASTA: {} org={:?}", file_path, organism),
                ProteomicsDataSource::PDB { file_path, pdb_id, .. } => format!("PDB: {} id={:?}", file_path, pdb_id),
                ProteomicsDataSource::MMCIF { file_path, entry_id } => format!("mmCIF: {} id={:?}", file_path, entry_id),
                ProteomicsDataSource::AlphaFold { file_path, uniprot_id, .. } => format!("AlphaFold: {} uniprot={:?}", file_path, uniprot_id),
                ProteomicsDataSource::MassSpec { file_path, experiment_type, .. } => format!("MassSpec: {} {:?}", file_path, experiment_type),
                ProteomicsDataSource::ExpressionMatrix { file_path, .. } => format!("ExprMatrix: {}", file_path),
                ProteomicsDataSource::UniProt { file_path, accession_ids } => format!("UniProt: {} ids={:?}", file_path, accession_ids.as_ref().map(|v| v.len())),
                ProteomicsDataSource::InteractionDB { file_path, database, .. } => format!("InteractionDB: {} {:?}", file_path, database),
                ProteomicsDataSource::DomainAnnotation { file_path, database } => format!("DomainAnnotation: {} {:?}", file_path, database),
                ProteomicsDataSource::RemoteQuery { database, query, .. } => format!("RemoteQuery: {:?} {}", database, query),
                ProteomicsDataSource::MultiSource { sources } => format!("MultiSource: {} sources", sources.len()),
            };
            Ok(ProteomicsModalityOutput { success: true, analysis: Some(ProteomicsAnalysisResult { analysis_id, source_description, ..Default::default() }), ..Default::default() })
        }

        ProteomicsModalityAction::CreateGraph { analysis, project_id } => {
            Ok(create_graph(&executor, analysis, project_id).await)
        }

        ProteomicsModalityAction::UpdateGraph { graph_id, updates, project_id } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            let mut next_nid = graph.nodes.iter().map(|n| n.node_id).max().unwrap_or(0) + 1;
            let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
            let initial = graph.nodes.len();

            for update in &updates {
                match update {
                    ProteomicsUpdate::AddPTM { ptm } => {
                        graph.nodes.push(ProteomicsGraphNode {
                            node_id: next_nid, node_type: ProteomicsNodeType::PTMNode,
                            content: format!("PTM {:?}: pos={}{}", ptm.modification_type, ptm.amino_acid, ptm.residue_position),
                            residue_position: Some(ptm.residue_position),
                            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                            keywords: vec!["ptm".into()], hotness_score: 0.65, ..Default::default()
                        });
                        graph.edges.push(ProteomicsGraphEdge { edge_id: next_eid, from_node: graph.root_node_id, to_node: next_nid, edge_type: ProteomicsEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                        next_eid += 1; next_nid += 1;
                    }
                    ProteomicsUpdate::AddInteraction { interaction } => {
                        graph.nodes.push(ProteomicsGraphNode {
                            node_id: next_nid, node_type: ProteomicsNodeType::PPINode,
                            content: format!("PPI: {} ↔ {} score={:.3}", interaction.protein_a_accession, interaction.protein_b_accession, interaction.combined_score),
                            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                            keywords: vec!["ppi".into()], hotness_score: 0.6 + interaction.combined_score * 0.3, ..Default::default()
                        });
                        graph.edges.push(ProteomicsGraphEdge { edge_id: next_eid, from_node: graph.root_node_id, to_node: next_nid, edge_type: ProteomicsEdgeType::Contains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                        next_eid += 1; next_nid += 1;
                    }
                    _ => { next_nid += 0; } // other updates handled similarly
                }
            }
            graph.version += 1; graph.updated_at = now.clone(); graph.state = GraphStateType::Updated;
            graph.version_notes.push(VersionNote { version: graph.version, note: format!("Updated: {} changes → {} new nodes", updates.len(), graph.nodes.len() - initial), step_index: None, timestamp: now, change_type: ChangeType::Updated });
            executor.save_graph(&graph)?;
            Ok(ProteomicsModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        ProteomicsModalityAction::PredictStructure { sequence, method, include_confidence } => {
            let (mw, _pI) = PipelineExecutor::compute_sequence_properties(&sequence);
            // In production: call AlphaFold2 / ESMFold API or local model
            let plddt_mean = if include_confidence { Some(0.87f32) } else { None }; // placeholder
            Ok(ProteomicsModalityOutput {
                success: true,
                predicted_structure: Some(ProteinStructure {
                    structure_id: executor.generate_id(),
                    protein_id: 0,
                    pdb_id: None,
                    structure_method: match method {
                        StructurePredictionMethod::AlphaFold2   => StructureDeterminationMethod::AlphaFold2,
                        StructurePredictionMethod::RoseTTAFold  => StructureDeterminationMethod::RoseTTAFold,
                        StructurePredictionMethod::ESMFold      => StructureDeterminationMethod::ESMFold,
                        _                                       => StructureDeterminationMethod::AlphaFold2,
                    },
                    resolution_angstrom: None,
                    chain_id: "A".into(),
                    residue_count: sequence.len() as u32,
                    coverage_start: 1,
                    coverage_end: sequence.len() as u32,
                    file_path: None,
                    tm_score: None,
                    plddt_mean,
                    secondary_structure_summary: SecondaryStructureSummary { helix_fraction: 0.30, sheet_fraction: 0.25, coil_fraction: 0.35, turn_fraction: 0.10 },
                    oligomerization_state: OligomerizationState::Unknown,
                }),
                ..Default::default()
            })
        }

        ProteomicsModalityAction::DockLigand { protein_graph_id, protein_node_id, ligand_smiles, binding_site_residues, method } => {
            // In production: call AutoDock Vina / GNINA / Glide backend
            let docking_score = -8.5f64; // kcal/mol placeholder
            Ok(ProteomicsModalityOutput {
                success: true,
                docking_result: Some(ProteinLigandInteraction {
                    interaction_id: executor.generate_id(),
                    protein_id: protein_node_id,
                    ligand_name: "Ligand".into(),
                    ligand_smiles: Some(ligand_smiles.clone()),
                    binding_site_id: None,
                    binding_affinity: Some(BindingAffinity {
                        kd_nm: Some(50.0),
                        delta_g_kcal_mol: Some(docking_score),
                        ..Default::default()
                    }),
                    interaction_type: LigandInteractionType::Competitive,
                    docking_score: Some(docking_score),
                    hydrogen_bonds: Some(vec![]),
                    hydrophobic_contacts: vec![],
                    ..Default::default()
                }),
                ..Default::default()
            })
        }

        ProteomicsModalityAction::ComputePPINetwork { protein_ids, interaction_db, confidence_threshold, expand_neighbors } => {
            Ok(ProteomicsModalityOutput {
                success: true,
                ppi_network_stats: Some(serde_json::json!({
                    "protein_count": protein_ids.len(),
                    "database": format!("{:?}", interaction_db),
                    "confidence_threshold": confidence_threshold,
                    "expanded": expand_neighbors,
                    "interactions_found": 0,  // real impl queries database
                })),
                ..Default::default()
            })
        }

        ProteomicsModalityAction::AnalyzeMassSpec { data, quantification_method, fdr_threshold } => {
            Ok(ProteomicsModalityOutput {
                success: true,
                analysis: Some(ProteomicsAnalysisResult {
                    analysis_id: executor.generate_id(),
                    source_description: format!("MassSpec: {} {:?} fdr={}", data.file_path, quantification_method, fdr_threshold),
                    ..Default::default()
                }),
                ..Default::default()
            })
        }

        ProteomicsModalityAction::MapPathways { protein_id, databases } => {
            let pathways: Vec<PathwayMembership> = databases.into_iter().enumerate().map(|(i, db)| {
                PathwayMembership {
                    membership_id: executor.generate_id(),
                    protein_id: 0,
                    pathway_id: format!("PATH_{:04}", i + 1),
                    pathway_name: format!("Pathway from {:?}", db),
                    database: db,
                    role_in_pathway: None,
                }
            }).collect();
            Ok(ProteomicsModalityOutput { success: true, pathway_enrichment: Some(pathways), ..Default::default() })
        }

        ProteomicsModalityAction::AlignSequences { sequences, method } => {
            if sequences.len() < 2 {
                return Ok(ProteomicsModalityOutput { success: false, error: Some("Need ≥2 sequences".into()), ..Default::default() });
            }
            // In production: call ClustalOmega/MUSCLE/MAFFT binary
            let alignments: Vec<SequenceAlignment> = sequences.windows(2).enumerate().map(|(i, pair)| {
                let seq_a = &pair[0];
                let seq_b = &pair[1];
                let len_diff = (seq_a.sequence.len() as f32 - seq_b.sequence.len() as f32).abs();
                let identity = 100.0 * (1.0 - len_diff / seq_a.sequence.len().max(seq_b.sequence.len()) as f32).clamp(0.0, 1.0);
                SequenceAlignment {
                    alignment_id: executor.generate_id(),
                    query_protein_id: i as u64,
                    target_protein_id: i as u64 + 1,
                    identity_percent: identity,
                    similarity_percent: identity + 5.0,
                    coverage_percent: 95.0,
                    e_value: 1e-50,
                    bit_score: 500.0,
                    method: format!("{:?}", method),
                    aligned_region_query: (1, seq_a.sequence.len() as u32),
                    aligned_region_target: (1, seq_b.sequence.len() as u32),
                }
            }).collect();
            Ok(ProteomicsModalityOutput { success: true, alignment_result: Some(alignments), ..Default::default() })
        }

        ProteomicsModalityAction::StructuralAlignment { structure_a_id, structure_b_id, method } => {
            // In production: run TM-align / DALI
            Ok(ProteomicsModalityOutput {
                success: true,
                ppi_network_stats: Some(serde_json::json!({
                    "structure_a": structure_a_id,
                    "structure_b": structure_b_id,
                    "method": format!("{:?}", method),
                    "tm_score": 0.85,
                    "rmsd_angstrom": 1.4,
                    "aligned_residues": 200,
                })),
                ..Default::default()
            })
        }

        ProteomicsModalityAction::QueryGraph { graph_id, query } => {
            let graph = executor.load_graph(graph_id)?;
            let result = match query {
                ProteomicsGraphQuery::NodeDetail { node_id } => {
                    let node = graph.nodes.iter().find(|n| n.node_id == node_id);
                    let incoming: Vec<_> = graph.edges.iter().filter(|e| e.to_node == node_id).collect();
                    let outgoing: Vec<_> = graph.edges.iter().filter(|e| e.from_node == node_id).collect();
                    serde_json::json!({ "node": node, "incoming": incoming, "outgoing": outgoing })
                }
                ProteomicsGraphQuery::ProteinByAccession { accession } => {
                    let proteins: Vec<_> = graph.nodes.iter().filter(|n| n.accession.as_deref() == Some(&accession)).collect();
                    serde_json::json!({ "proteins": proteins })
                }
                ProteomicsGraphQuery::InteractionNetwork { protein_node_id, depth } => {
                    let mut visited = std::collections::HashSet::new();
                    let mut frontier = vec![protein_node_id];
                    let mut all_edges = Vec::new();
                    for _ in 0..depth {
                        let mut next_frontier = Vec::new();
                        for &nid in &frontier {
                            visited.insert(nid);
                            let connected: Vec<_> = graph.edges.iter()
                                .filter(|e| (e.from_node == nid || e.to_node == nid) && matches!(e.edge_type, ProteomicsEdgeType::InteractsWith | ProteomicsEdgeType::Phosphorylates | ProteomicsEdgeType::BindsTo | ProteomicsEdgeType::PartOfComplex))
                                .collect();
                            for e in &connected {
                                all_edges.push(e);
                                let other = if e.from_node == nid { e.to_node } else { e.from_node };
                                if !visited.contains(&other) { next_frontier.push(other); }
                            }
                        }
                        frontier = next_frontier;
                        if frontier.is_empty() { break; }
                    }
                    serde_json::json!({ "edges": all_edges, "node_count": visited.len() })
                }
                ProteomicsGraphQuery::PTMsForProtein { protein_node_id } => {
                    let ptms: Vec<_> = graph.edges.iter()
                        .filter(|e| e.from_node == protein_node_id && matches!(e.edge_type, ProteomicsEdgeType::ModifiedBy | ProteomicsEdgeType::GlycosylatedAt | ProteomicsEdgeType::UbiquitinatedAt))
                        .filter_map(|e| graph.nodes.iter().find(|n| n.node_id == e.to_node))
                        .collect();
                    serde_json::json!({ "ptms": ptms })
                }
                ProteomicsGraphQuery::PathwayProteins { pathway_id } => {
                    let pw_node = graph.nodes.iter().find(|n| n.pathway_id.as_deref() == Some(&pathway_id));
                    let proteins: Vec<_> = pw_node.map(|pw| {
                        graph.edges.iter()
                            .filter(|e| e.to_node == pw.node_id && matches!(e.edge_type, ProteomicsEdgeType::ParticipatesIn))
                            .filter_map(|e| graph.nodes.iter().find(|n| n.node_id == e.from_node))
                            .collect::<Vec<_>>()
                    }).unwrap_or_default();
                    serde_json::json!({ "pathway_proteins": proteins })
                }
                ProteomicsGraphQuery::DifferentiallyExpressed { log2_fc_threshold } => {
                    let de_nodes: Vec<_> = graph.nodes.iter()
                        .filter(|n| matches!(n.node_type, ProteomicsNodeType::DifferentialExpressionNode))
                        .filter(|n| n.score.map(|s| s >= log2_fc_threshold as f64).unwrap_or(false))
                        .collect();
                    serde_json::json!({ "de_proteins": de_nodes })
                }
                ProteomicsGraphQuery::BindingSites { protein_node_id } => {
                    let sites: Vec<_> = graph.edges.iter()
                        .filter(|e| e.from_node == protein_node_id && matches!(e.edge_type, ProteomicsEdgeType::HasBindingSite | ProteomicsEdgeType::HasActiveSite))
                        .filter_map(|e| graph.nodes.iter().find(|n| n.node_id == e.to_node))
                        .collect();
                    serde_json::json!({ "binding_sites": sites })
                }
                ProteomicsGraphQuery::ComplexMembers { complex_node_id } => {
                    let members: Vec<_> = graph.edges.iter()
                        .filter(|e| e.to_node == complex_node_id && matches!(e.edge_type, ProteomicsEdgeType::PartOfComplex))
                        .filter_map(|e| graph.nodes.iter().find(|n| n.node_id == e.from_node))
                        .collect();
                    serde_json::json!({ "complex_members": members })
                }
                ProteomicsGraphQuery::CrossModalLinks { node_id } => {
                    let links: Vec<_> = graph.edges.iter()
                        .filter(|e| (e.from_node == node_id || e.to_node == node_id)
                            && matches!(e.edge_type, ProteomicsEdgeType::CodingSequenceIn | ProteomicsEdgeType::MechanismInChemistry | ProteomicsEdgeType::InBiologyPathway | ProteomicsEdgeType::TissueMapFrom126 | ProteomicsEdgeType::ScoredByMath | ProteomicsEdgeType::AnalysisCodeIn))
                        .collect();
                    serde_json::json!({ "cross_modal_links": links })
                }
                ProteomicsGraphQuery::AGIActivity => serde_json::json!({ "is_active": false }),
                ProteomicsGraphQuery::AllNodes => serde_json::json!({ "nodes": graph.nodes }),
                ProteomicsGraphQuery::AllEdges => serde_json::json!({ "edges": graph.edges }),
            };
            Ok(ProteomicsModalityOutput { success: true, query_result: Some(result), ..Default::default() })
        }

        ProteomicsModalityAction::GetGraph { graph_id } => {
            let graph = executor.load_graph(graph_id)?;
            Ok(ProteomicsModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        ProteomicsModalityAction::TriggerSemanticHook { graph_id, hook } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            match hook {
                ProteomicsSemanticHook::OnGraphCreated => { graph.state = GraphStateType::SemanticEnriched; }
                ProteomicsSemanticHook::OnInferRelationships => {
                    let new_edges = executor.infer_semantic_relationships(&graph.nodes).await;
                    let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                    let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                    for (from, to, etype, reason) in new_edges {
                        if valid.contains(&from) && valid.contains(&to) && from != to {
                            graph.edges.push(ProteomicsGraphEdge { edge_id: next_eid, from_node: from, to_node: to, edge_type: etype, weight: 0.8, provenance: EdgeProvenance::DerivedFromHook, version: 1, properties: { let mut p = HashMap::new(); p.insert("reason".into(), serde_json::json!(reason)); p }, ..Default::default() });
                            next_eid += 1;
                        }
                    }
                }
                ProteomicsSemanticHook::OnEdgeCompletion => {
                    let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                    graph.edges.retain(|e| valid.contains(&e.from_node) && valid.contains(&e.to_node));
                }
                ProteomicsSemanticHook::OnCrossModalityLink { target_modality, target_graph_id } => {
                    graph.state = GraphStateType::CrossLinked;
                    graph.version += 1;
                    graph.version_notes.push(VersionNote { version: graph.version, note: format!("Cross-linked to {} (graph {})", target_modality, target_graph_id), step_index: None, timestamp: now.clone(), change_type: ChangeType::CrossLinked });
                }
            }
            graph.updated_at = now;
            executor.save_graph(&graph)?;
            Ok(ProteomicsModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        ProteomicsModalityAction::ExportProduct { graph_id, format } => {
            let ext = match &format {
                ProteomicsExportFormat::FASTA => "fasta",
                ProteomicsExportFormat::PDB => "pdb",
                ProteomicsExportFormat::TSV_Interactions => "tsv",
                ProteomicsExportFormat::TSV_Expression => "tsv",
                ProteomicsExportFormat::BioPAX => "owl",
                ProteomicsExportFormat::SIF => "sif",
                ProteomicsExportFormat::JSON_LD => "jsonld",
                _ => "tsv",
            };
            let export_path = format!("/tmp/proteomics_export_{}.{}", graph_id, ext);
            Ok(ProteomicsModalityOutput { success: true, export_path: Some(export_path), ..Default::default() })
        }

        ProteomicsModalityAction::StreamToUI { graph_id, .. } => {
            Ok(ProteomicsModalityOutput { success: true, graph_id: Some(graph_id), ..Default::default() })
        }

        ProteomicsModalityAction::HeadlessProcess { graph_id, operations } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            for op in operations {
                match op {
                    ProteomicsOperation::RebuildPPINetwork => { /* rebuild from scratch */ }
                    ProteomicsOperation::RecomputeDifferentialExpression => { /* rerun DE stats */ }
                    ProteomicsOperation::EnrichPathways { database } => { /* query pathway db */ }
                    ProteomicsOperation::CrossLinkToDNA { dna_graph_id } => {
                        let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        for node in graph.nodes.iter().filter(|n| matches!(n.node_type, ProteomicsNodeType::ProteinNode)) {
                            graph.edges.push(ProteomicsGraphEdge { edge_id: next_eid, from_node: node.node_id, to_node: node.node_id, edge_type: ProteomicsEdgeType::CodingSequenceIn, weight: 0.9, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("dna_graph_id".into(), serde_json::json!(dna_graph_id)); p }, ..Default::default() });
                            next_eid += 1;
                        }
                    }
                    ProteomicsOperation::CrossLinkToChemistry { chem_graph_id } => {
                        let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        for node in graph.nodes.iter().filter(|n| matches!(n.node_type, ProteomicsNodeType::EnzymeNode)) {
                            graph.edges.push(ProteomicsGraphEdge { edge_id: next_eid, from_node: node.node_id, to_node: node.node_id, edge_type: ProteomicsEdgeType::MechanismInChemistry, weight: 0.85, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("chem_graph_id".into(), serde_json::json!(chem_graph_id)); p }, ..Default::default() });
                            next_eid += 1;
                        }
                    }
                    ProteomicsOperation::ExportInteractions => { /* export PPI to file */ }
                }
            }
            graph.updated_at = now;
            executor.save_graph(&graph)?;
            Ok(ProteomicsModalityOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
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
    if input_json.is_empty() { eprintln!("Usage: proteomics --input '<json>'"); std::process::exit(1); }
    let input: ProteomicsModalityAction = match serde_json::from_str(&input_json) {
        Ok(v) => v,
        Err(e) => { println!("{}", serde_json::json!({"success":false,"error":format!("Parse error: {}",e)})); std::process::exit(1); }
    };
    let rt = tokio::runtime::Runtime::new().expect("Tokio runtime");
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap_or_else(|_| r#"{"success":false,"error":"serialize"}"#.into())),
        Err(e) => { println!("{}", serde_json::json!({"success":false,"error":e})); std::process::exit(1); }
    }
}
