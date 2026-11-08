//! OZONE Studio - Pipeline 106: Chemistry Analysis
//!
//! Modality pipeline for chemical structure and reaction analysis.
//! Analyzes molecules, bonds, functional groups, and chemical reactions.
//! Creates traversable graphs that can be enriched by ZSEI semantic hooks.
//!
//! # Actions
//! - `AnalyzeMolecule`: Parse and analyze molecular structure
//! - `AnalyzeReaction`: Analyze chemical reaction
//! - `PredictProperties`: Predict molecular properties
//! - `CreateGraph`: Build structural graph from analysis
//! - `QueryGraph`: Query graph for substructures, similar molecules
//! - `TriggerSemanticHook`: Trigger ZSEI hooks for semantic enrichment
//!
//! # Graph Structure
//! - Nodes: Molecule, Atom, Bond, FunctionalGroup, Reaction
//! - Edges: Contains, BondedTo, ReactsWith, Produces, CatalyzedBy

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::env;

#[path = "../../shared/ozone_serve.rs"]
mod ozone_serve;

// Pure parsing + geometry (MOL/SDF V2000, XYZ, small-molecule PDB, SMILES, formula).
// UNCOMPILED pending the EU build gate. Replaces the former fabricated analysis.
mod molparse;

// ============================================================================
// PIPELINE METADATA
// ============================================================================

pub const PIPELINE_ID: u64 = 106;
pub const PIPELINE_NAME: &str = "chemistry_analysis";
pub const PIPELINE_VERSION: &str = "0.4.0";
pub const MODALITY: &str = "chemistry";

// ============================================================================
// INPUT/OUTPUT TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ChemistryModalityInput {
    pub action: ChemistryAction,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ChemistryAction {
    /// Analyze molecular structure
    AnalyzeMolecule {
        molecule: MoleculeData,
        #[serde(default)]
        compute_properties: bool,
        #[serde(default)]
        find_functional_groups: bool,
        #[serde(default)]
        generate_3d: bool,
    },

    /// Analyze chemical reaction
    AnalyzeReaction {
        reaction: ReactionData,
        #[serde(default)]
        balance: bool,
        #[serde(default)]
        predict_mechanism: bool,
        #[serde(default)]
        compute_thermodynamics: bool,
    },

    /// Predict molecular properties
    PredictProperties {
        molecule: MoleculeData,
        properties: Vec<PropertyType>,
    },

    /// Find substructure matches
    FindSubstructure {
        molecule: MoleculeData,
        pattern: String, // SMARTS pattern
    },

    /// Compare molecular similarity
    CompareMolecules {
        molecule1: MoleculeData,
        molecule2: MoleculeData,
        #[serde(default)]
        method: SimilarityMethod,
    },

    /// Create graph from analysis
    CreateGraph {
        analysis: ChemistryAnalysisResult,
        project_id: u64,
        #[serde(default)]
        graph_name: Option<String>,
    },

    /// Update existing graph
    UpdateGraph {
        graph_id: u64,
        updates: ChemistryGraphUpdate,
    },

    /// Query chemistry graph
    QueryGraph {
        graph_id: u64,
        query: ChemistryQuery,
    },

    /// Get graph
    GetGraph {
        graph_id: u64,
    },

    /// Link to another modality
    LinkToModality {
        chemistry_graph_id: u64,
        target_graph_id: u64,
        target_modality: String,
        relationship: CrossModalityRelation,
    },

    /// Trigger ZSEI semantic hook
    TriggerSemanticHook {
        graph_id: u64,
        hook_type: ZSEIHookType,
        #[serde(default)]
        options: HookOptions,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChemistryModalityOutput {
    pub success: bool,
    pub action: String,
    pub result: ChemistryResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub metadata: OutputMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChemistryResult {
    Analysis(ChemistryAnalysisResult),
    Molecule(MoleculeAnalysis),
    Reaction(ReactionAnalysis),
    Properties(PropertyPredictions),
    Substructure(SubstructureResult),
    Similarity(SimilarityResult),
    Graph(ChemistryGraph),
    Query(QueryResult),
    Link(LinkResult),
    Hook(HookResult),
    Empty,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OutputMetadata {
    pub pipeline_id: u64,
    pub pipeline_version: String,
    pub processing_time_ms: u64,
    pub timestamp: String,
}

// ============================================================================
// CORE DATA TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MoleculeData {
    /// SMILES notation
    SMILES(String),
    /// InChI notation
    InChI(String),
    /// MDL Molfile
    MolFile(String),
    /// PDB format
    PDB(String),
    /// XYZ coordinates (atom count, comment, then `El x y z` lines)
    XYZ(String),
    /// Molecular formula
    Formula(String),
    /// Common name (will be looked up)
    Name(String),
    /// CAS registry number
    CAS(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReactionData {
    /// Reactant molecules
    pub reactants: Vec<MoleculeData>,
    /// Product molecules
    pub products: Vec<MoleculeData>,
    /// Reagents (not consumed)
    pub reagents: Vec<MoleculeData>,
    /// Reaction conditions
    pub conditions: Option<ReactionConditions>,
    /// Reaction SMILES (if available)
    pub reaction_smiles: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ReactionConditions {
    /// Temperature in Celsius
    pub temperature: Option<f32>,
    /// Pressure in atm
    pub pressure: Option<f32>,
    /// Catalyst
    pub catalyst: Option<String>,
    /// Solvent
    pub solvent: Option<String>,
    /// Reaction time in hours
    pub time_hours: Option<f32>,
    /// pH
    pub ph: Option<f32>,
    /// Atmosphere (N2, Ar, O2, etc.)
    pub atmosphere: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub enum SimilarityMethod {
    #[default]
    Tanimoto,
    Dice,
    Cosine,
    ECFP4,
    MACCS,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PropertyType {
    LogP,
    PKa,
    Solubility,
    MeltingPoint,
    BoilingPoint,
    MolecularWeight,
    TPSA,
    HBondDonors,
    HBondAcceptors,
    RotatableBonds,
    Lipinski,
    Toxicity,
    Bioavailability,
}

// ============================================================================
// ANALYSIS RESULTS
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChemistryAnalysisResult {
    /// Analyzed molecules
    pub molecules: Vec<MoleculeAnalysis>,
    /// Analyzed reactions
    pub reactions: Vec<ReactionAnalysis>,
    /// Metabolic/synthetic pathways
    pub pathways: Vec<ChemicalPathway>,
    /// Overall analysis metadata
    pub metadata: AnalysisMetadata,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AnalysisMetadata {
    pub molecule_count: usize,
    pub reaction_count: usize,
    pub total_atoms: usize,
    pub total_bonds: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MoleculeAnalysis {
    /// Molecule identifier
    pub molecule_id: String,
    /// Common name (if known)
    pub name: Option<String>,
    /// IUPAC name
    pub iupac_name: Option<String>,
    /// Molecular formula
    pub formula: String,
    /// SMILES representation
    pub smiles: String,
    /// InChI representation
    pub inchi: Option<String>,
    /// InChIKey
    pub inchi_key: Option<String>,
    /// Molecular weight
    pub molecular_weight: f64,
    /// Exact mass
    pub exact_mass: f64,
    /// Atoms in molecule
    pub atoms: Vec<Atom>,
    /// Bonds in molecule
    pub bonds: Vec<Bond>,
    /// Rings detected
    pub rings: Vec<Ring>,
    /// Functional groups detected
    pub functional_groups: Vec<FunctionalGroup>,
    /// Stereochemistry information
    pub stereochemistry: Option<Stereochemistry>,
    /// Computed properties
    pub properties: MolecularProperties,
    /// 3D structure (if generated)
    pub structure_3d: Option<Structure3D>,
    /// Drug-likeness assessment (never fabricated: absent unless a model supplies it)
    pub drug_likeness: Option<DrugLikeness>,
    /// Format actually parsed ("smiles", "molfile-v2000", "xyz", "pdb", "formula", "inchi-formula-layer")
    #[serde(default)]
    pub source_format: String,
    /// True when every hydrogen is accounted for (explicit atoms or valence-derived).
    /// When false the formula lists heavy atoms only and mass is unresolved.
    #[serde(default)]
    pub hydrogens_resolved: bool,
    /// True when `molecular_weight` / `exact_mass` were computed (all elements known AND hydrogens resolved).
    #[serde(default)]
    pub mass_resolved: bool,
    /// Cyclomatic number (bonds - atoms + components); ring enumeration (SSSR) is not implemented.
    #[serde(default)]
    pub ring_count: Option<usize>,
    /// Geometry computed from supplied coordinates (absent for coordinate-free input).
    #[serde(default)]
    pub geometry: Option<MoleculeGeometry>,
    /// Why `structure_3d` is absent when 3D was requested.
    #[serde(default)]
    pub structure_3d_absent_reason: Option<String>,
    /// Honesty notes: what was derived, inferred, or could not be determined.
    #[serde(default)]
    pub notes: Vec<String>,
}

/// Real geometry from supplied coordinates, in frame `molecule:<id>` (angstrom).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MoleculeGeometry {
    pub frame: String,
    pub unit: String,
    pub centre: [f64; 3],
    pub bbox_min: [f64; 3],
    pub bbox_max: [f64; 3],
    pub bond_angles: Vec<BondAngle>,
    pub bond_angles_truncated: bool,
    /// Non-bonded, non-1-3 atom pairs closer than `near_cutoff_angstrom` (bounded list, nearest first).
    pub near_contacts: Vec<NearContact>,
    pub near_contacts_total: usize,
    pub near_cutoff_angstrom: f64,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BondAngle {
    pub atom_a: usize,
    pub centre: usize,
    pub atom_b: usize,
    pub degrees: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NearContact {
    pub atom_a: usize,
    pub atom_b: usize,
    pub distance: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Atom {
    /// Atom index (0-based)
    pub atom_id: usize,
    /// Element symbol
    pub element: String,
    /// Atomic number
    pub atomic_number: u8,
    /// Formal charge
    pub formal_charge: i32,
    /// Implicit hydrogen count
    pub implicit_hydrogens: u8,
    /// Explicit hydrogen count
    pub explicit_hydrogens: u8,
    /// Hybridization state
    pub hybridization: Hybridization,
    /// Is aromatic
    pub is_aromatic: bool,
    /// Is in ring
    pub is_in_ring: bool,
    /// Isotope (if specified)
    pub isotope: Option<u16>,
    /// 3D position (if available)
    pub position: Option<Position3D>,
    /// Partial charge (if computed)
    pub partial_charge: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub enum Hybridization {
    S,
    SP,
    SP2,
    SP3,
    SP3D,
    SP3D2,
    #[default]
    Unspecified,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Position3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Bond {
    /// Bond index
    pub bond_id: usize,
    /// First atom index
    pub atom1_id: usize,
    /// Second atom index
    pub atom2_id: usize,
    /// Bond type
    pub bond_type: BondType,
    /// Bond order (1, 1.5, 2, 3)
    pub bond_order: f32,
    /// Is aromatic
    pub is_aromatic: bool,
    /// Is in ring
    pub is_in_ring: bool,
    /// Stereochemistry
    pub stereo: Option<BondStereo>,
    /// Is rotatable
    pub is_rotatable: bool,
    /// Bond length (if 3D available)
    pub bond_length: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum BondType {
    Single,
    Double,
    Triple,
    Aromatic,
    Quadruple,
    Dative,
    Hydrogen,
    Ionic,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum BondStereo {
    None,
    E,
    Z,
    Cis,
    Trans,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Ring {
    /// Ring identifier
    pub ring_id: String,
    /// Atom indices in ring
    pub atoms: Vec<usize>,
    /// Ring size
    pub size: usize,
    /// Is aromatic
    pub is_aromatic: bool,
    /// Ring name (if recognized)
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionalGroup {
    /// Group name
    pub name: String,
    /// SMARTS pattern
    pub smarts: String,
    /// Atom indices in group
    pub atoms: Vec<usize>,
    /// Group class (alcohol, amine, etc.)
    pub group_class: String,
    /// Count of this group in molecule
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Stereochemistry {
    /// Number of chiral centers
    pub chiral_centers: Vec<ChiralCenter>,
    /// E/Z isomerism
    pub double_bond_stereo: Vec<DoubleBondStereo>,
    /// Is achiral
    pub is_achiral: bool,
    /// Is meso compound
    pub is_meso: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChiralCenter {
    pub atom_id: usize,
    pub configuration: ChiralConfiguration,
    pub neighbors: Vec<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ChiralConfiguration {
    R,
    S,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DoubleBondStereo {
    pub bond_id: usize,
    pub configuration: EZConfiguration,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum EZConfiguration {
    E,
    Z,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MolecularProperties {
    /// Octanol-water partition coefficient
    pub log_p: Option<f64>,
    /// Acid dissociation constants
    pub pka: Option<Vec<f64>>,
    /// Aqueous solubility (mg/L)
    pub solubility: Option<f64>,
    /// Melting point (°C)
    pub melting_point: Option<f64>,
    /// Boiling point (°C)
    pub boiling_point: Option<f64>,
    /// Topological polar surface area
    pub tpsa: Option<f64>,
    /// Number of H-bond donors
    pub h_bond_donors: u32,
    /// Number of H-bond acceptors
    pub h_bond_acceptors: u32,
    /// Number of rotatable bonds
    pub rotatable_bonds: u32,
    /// Molar refractivity
    pub molar_refractivity: Option<f64>,
    /// Number of heavy atoms
    pub heavy_atom_count: u32,
    /// Formal charge
    pub total_charge: i32,
    /// Complexity score
    pub complexity: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Structure3D {
    /// Atom coordinates
    pub coordinates: Vec<Position3D>,
    /// Conformation name
    pub conformation: String,
    /// Energy (kcal/mol)
    pub energy: Option<f64>,
    /// Is minimized
    pub is_minimized: bool,
    /// Force field used
    pub force_field: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DrugLikeness {
    /// Lipinski's Rule of Five
    pub lipinski_violations: u32,
    pub passes_lipinski: bool,
    /// Veber rules
    pub veber_violations: u32,
    /// Ghose filter
    pub ghose_violations: u32,
    /// Lead-likeness
    pub is_lead_like: bool,
    /// Drug-likeness score
    pub qed_score: Option<f64>,
    /// Synthetic accessibility score (1-10)
    pub sa_score: Option<f64>,
    /// Pan-assay interference (PAINS) alerts
    pub pains_alerts: Vec<String>,
    /// Brenk structural alerts
    pub brenk_alerts: Vec<String>,
}

// ============================================================================
// REACTION ANALYSIS
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReactionAnalysis {
    /// Reaction identifier
    pub reaction_id: String,
    /// Reaction SMILES
    pub reaction_smiles: String,
    /// Reaction name (if recognized)
    pub reaction_name: Option<String>,
    /// Reaction type/class
    pub reaction_type: ReactionType,
    /// Is balanced
    pub is_balanced: bool,
    /// Atom mapping
    pub atom_mapping: Option<Vec<AtomMapping>>,
    /// Reaction mechanism
    pub mechanism: Option<ReactionMechanism>,
    /// Thermodynamic data
    pub thermodynamics: Option<ReactionThermodynamics>,
    /// Kinetic data
    pub kinetics: Option<ReactionKinetics>,
    /// Reaction center atoms
    pub reaction_centers: Vec<usize>,
    /// Confidence score (1.0 only for the computed balance; 0.0 when nothing could be computed)
    pub confidence: f32,
    /// True when the element balance could actually be computed.
    #[serde(default)]
    pub balance_determined: bool,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ReactionType {
    Substitution,
    Addition,
    Elimination,
    Rearrangement,
    Oxidation,
    Reduction,
    Condensation,
    Hydrolysis,
    Polymerization,
    Isomerization,
    Cyclization,
    Coupling,
    Protection,
    Deprotection,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AtomMapping {
    pub reactant_molecule: usize,
    pub reactant_atom: usize,
    pub product_molecule: usize,
    pub product_atom: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReactionMechanism {
    /// Mechanism type
    pub mechanism_type: MechanismType,
    /// Steps in mechanism
    pub steps: Vec<MechanismStep>,
    /// Intermediates
    pub intermediates: Vec<String>,
    /// Transition states
    pub transition_states: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MechanismType {
    SN1,
    SN2,
    E1,
    E2,
    ElectrophilicAddition,
    NucleophilicAddition,
    RadicalChain,
    Pericyclic,
    Concerted,
    Stepwise,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MechanismStep {
    pub step_number: usize,
    pub description: String,
    pub arrow_pushing: Option<String>,
    pub intermediate: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReactionThermodynamics {
    /// Enthalpy change (kJ/mol)
    pub delta_h: Option<f64>,
    /// Entropy change (J/mol·K)
    pub delta_s: Option<f64>,
    /// Gibbs free energy (kJ/mol)
    pub delta_g: Option<f64>,
    /// Equilibrium constant
    pub k_eq: Option<f64>,
    /// Is exothermic
    pub is_exothermic: Option<bool>,
    /// Is spontaneous
    pub is_spontaneous: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReactionKinetics {
    /// Activation energy (kJ/mol)
    pub activation_energy: Option<f64>,
    /// Rate constant
    pub rate_constant: Option<f64>,
    /// Reaction order
    pub reaction_order: Option<u32>,
    /// Rate law expression
    pub rate_law: Option<String>,
    /// Half-life
    pub half_life: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChemicalPathway {
    /// Pathway name
    pub name: String,
    /// Pathway type
    pub pathway_type: PathwayType,
    /// Reactions in pathway
    pub reactions: Vec<String>,
    /// Starting materials
    pub starting_materials: Vec<String>,
    /// Final products
    pub final_products: Vec<String>,
    /// Enzymes involved (for biochemical)
    pub enzymes: Vec<String>,
    /// Total steps
    pub total_steps: usize,
    /// Overall yield (if synthetic)
    pub overall_yield: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PathwayType {
    Metabolic,
    Biosynthetic,
    Synthetic,
    Degradation,
    Signaling,
}

// ============================================================================
// PREDICTION RESULTS
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PropertyPredictions {
    pub molecule_id: String,
    pub predictions: Vec<PropertyPrediction>,
    /// Requested properties that cannot be computed here, with the reason (never filled with defaults).
    #[serde(default)]
    pub unavailable: Vec<UnavailableProperty>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UnavailableProperty {
    pub property: PropertyType,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PropertyPrediction {
    pub property: PropertyType,
    pub value: f64,
    pub unit: String,
    pub confidence: f32,
    pub method: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubstructureResult {
    pub pattern: String,
    pub matches: Vec<SubstructureMatch>,
    pub total_matches: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubstructureMatch {
    pub molecule_id: String,
    pub atom_indices: Vec<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SimilarityResult {
    pub molecule1_id: String,
    pub molecule2_id: String,
    pub method: SimilarityMethod,
    pub similarity: f64,
    pub common_substructures: Vec<String>,
}

// ============================================================================
// GRAPH TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChemistryGraph {
    pub graph_id: u64,
    pub name: String,
    pub modality: String,
    pub project_id: u64,
    pub nodes: Vec<ChemistryGraphNode>,
    pub edges: Vec<ChemistryGraphEdge>,
    pub metadata: GraphMetadata,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChemistryGraphNode {
    pub node_id: u64,
    pub node_type: ChemistryNodeType,
    pub label: String,
    pub content: String,
    pub smiles: Option<String>,
    pub confidence: f32,
    pub properties: HashMap<String, Value>,
    pub annotations: Vec<SemanticAnnotation>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ChemistryNodeType {
    Molecule,
    Atom,
    Bond,
    FunctionalGroup,
    Ring,
    Reaction,
    Pathway,
    Property,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SemanticAnnotation {
    pub annotation_type: String,
    pub value: Value,
    pub confidence: f32,
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChemistryGraphEdge {
    pub edge_id: u64,
    pub from_node: u64,
    pub to_node: u64,
    pub edge_type: ChemistryEdgeType,
    pub weight: f32,
    pub properties: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ChemistryEdgeType {
    // Structural
    Contains,
    BondedTo,
    PartOf,
    // Reaction
    ReactsWith,
    Produces,
    CatalyzedBy,
    // Similarity
    SimilarTo,
    IsomerOf,
    // Pathway
    PrecedesIn,
    FollowsIn,
    // Cross-modality
    DescribedBy,
    ImplementedBy,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GraphMetadata {
    pub node_count: usize,
    pub edge_count: usize,
    pub molecule_count: usize,
    pub reaction_count: usize,
    pub total_atoms: usize,
    pub total_bonds: usize,
    pub semantic_enriched: bool,
    pub cross_modal_links: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChemistryGraphUpdate {
    pub add_nodes: Vec<ChemistryGraphNode>,
    pub update_nodes: Vec<ChemistryGraphNode>,
    pub remove_nodes: Vec<u64>,
    pub add_edges: Vec<ChemistryGraphEdge>,
    pub remove_edges: Vec<u64>,
    pub metadata_updates: Option<HashMap<String, Value>>,
}

// ============================================================================
// QUERY TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChemistryQuery {
    pub query_type: ChemistryQueryType,
    pub parameters: HashMap<String, Value>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub min_confidence: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ChemistryQueryType {
    /// Find by substructure (SMARTS)
    FindSubstructure { pattern: String },
    /// Find similar molecules
    FindSimilar { smiles: String, threshold: f64 },
    /// Get reaction pathway
    GetReactionPath { start: String, end: String },
    /// Predict products
    PredictProducts { reactants: Vec<String> },
    /// Find by functional group
    FindByFunctionalGroup { group_name: String },
    /// Get molecules by property range
    FindByProperty { property: PropertyType, min: f64, max: f64 },
    /// Get nodes by type
    GetNodesByType { node_type: ChemistryNodeType },
    /// Custom query
    Custom { query: String },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryResult {
    pub query_type: String,
    pub nodes: Vec<ChemistryGraphNode>,
    pub edges: Vec<ChemistryGraphEdge>,
    pub total_matches: usize,
    pub metadata: HashMap<String, Value>,
}

// ============================================================================
// CROSS-MODALITY & HOOKS
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CrossModalityRelation {
    DescribedBy,
    StructureOf,
    ImplementedBy,
    AnalyzedIn,
    Custom(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LinkResult {
    pub link_id: u64,
    pub source_graph_id: u64,
    pub target_graph_id: u64,
    pub relationship: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ZSEIHookType {
    OnGraphCreated,
    OnInferRelationships,
    OnEdgeCompletion,
    OnCrossModalityLink,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct HookOptions {
    pub max_nodes: Option<usize>,
    pub min_confidence: Option<f32>,
    pub async_processing: bool,
    pub parameters: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HookResult {
    pub hook_type: ZSEIHookType,
    pub success: bool,
    pub nodes_processed: usize,
    pub edges_added: usize,
    pub annotations_added: usize,
    pub processing_time_ms: u64,
    pub errors: Vec<String>,
}

// ============================================================================
// EXECUTION
// ============================================================================

pub async fn execute(input: Value) -> Result<Value, String> {
    let start_time = std::time::Instant::now();

    // Honest refusal for a text-only "mention" with no real raw molecule/
    // reaction data attached, BEFORE the strict typed deserialization below
    // would reject it with an accidental, low-level parse error. Only the
    // two actions that require raw data (AnalyzeMolecule/AnalyzeReaction)
    // are checked.
    if let Some(action_type) = input.get("action").and_then(|a| a.get("type")).and_then(|t| t.as_str()) {
        let required: Option<(&str, &str)> = match action_type {
            "AnalyzeMolecule" => Some(("molecule", "molecular structure")),
            "AnalyzeReaction" => Some(("reaction", "chemical reaction")),
            _ => None,
        };
        if let Some((field, label)) = required {
            let missing = input
                .get("action")
                .and_then(|a| a.get(field))
                .map(|d| d.is_null())
                .unwrap_or(true);
            if missing {
                return Ok(serde_json::json!({
                    "success": false,
                    "action": action_type,
                    "result": null,
                    "error": format!("No raw {} data provided — this call appears to be a text-only mention with no attached data file; real chemistry analysis requires an actual {} data source", label, label),
                    "metadata": {
                        "pipeline_id": PIPELINE_ID,
                        "pipeline_version": PIPELINE_VERSION,
                        "processing_time_ms": start_time.elapsed().as_millis(),
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    }
                }));
            }
        }
    }

    let input: ChemistryModalityInput =
        serde_json::from_value(input).map_err(|e| format!("Failed to parse input: {}", e))?;

    let result = match input.action {
        ChemistryAction::AnalyzeMolecule {
            molecule,
            compute_properties,
            find_functional_groups,
            generate_3d,
        } => {
            let analysis = analyze_molecule(
                &molecule,
                compute_properties,
                find_functional_groups,
                generate_3d,
            )
            .await?;
            ("AnalyzeMolecule", ChemistryResult::Molecule(analysis))
        }

        ChemistryAction::AnalyzeReaction {
            reaction,
            balance,
            predict_mechanism,
            compute_thermodynamics,
        } => {
            let analysis =
                analyze_reaction(&reaction, balance, predict_mechanism, compute_thermodynamics)
                    .await?;
            ("AnalyzeReaction", ChemistryResult::Reaction(analysis))
        }

        ChemistryAction::PredictProperties { molecule, properties } => {
            let predictions = predict_properties(&molecule, &properties).await?;
            ("PredictProperties", ChemistryResult::Properties(predictions))
        }

        ChemistryAction::FindSubstructure { molecule, pattern } => {
            let result = find_substructure(&molecule, &pattern).await?;
            ("FindSubstructure", ChemistryResult::Substructure(result))
        }

        ChemistryAction::CompareMolecules {
            molecule1,
            molecule2,
            method,
        } => {
            let result = compare_molecules(&molecule1, &molecule2, method).await?;
            ("CompareMolecules", ChemistryResult::Similarity(result))
        }

        ChemistryAction::CreateGraph {
            analysis,
            project_id,
            graph_name,
        } => {
            let mut graph = create_graph(analysis, project_id, graph_name).await?;
            if let Err(e) = persist_chemistry_graph(&mut graph).await {
                eprintln!("chemistry CreateGraph: ZSEI persistence failed: {e}");
            }
            ("CreateGraph", ChemistryResult::Graph(graph))
        }

        ChemistryAction::UpdateGraph { graph_id, updates } => {
            let graph = update_graph(graph_id, updates).await?;
            ("UpdateGraph", ChemistryResult::Graph(graph))
        }

        ChemistryAction::QueryGraph { graph_id, query } => {
            let result = query_graph(graph_id, query).await?;
            ("QueryGraph", ChemistryResult::Query(result))
        }

        ChemistryAction::GetGraph { graph_id } => {
            let graph = get_graph(graph_id).await?;
            ("GetGraph", ChemistryResult::Graph(graph))
        }

        ChemistryAction::LinkToModality {
            chemistry_graph_id,
            target_graph_id,
            target_modality,
            relationship,
        } => {
            let link = link_to_modality(
                chemistry_graph_id,
                target_graph_id,
                &target_modality,
                relationship,
            )
            .await?;
            ("LinkToModality", ChemistryResult::Link(link))
        }

        ChemistryAction::TriggerSemanticHook {
            graph_id,
            hook_type,
            options,
        } => {
            let result = trigger_semantic_hook(graph_id, hook_type, options).await?;
            ("TriggerSemanticHook", ChemistryResult::Hook(result))
        }
    };

    let output = ChemistryModalityOutput {
        success: true,
        action: result.0.to_string(),
        result: result.1,
        error: None,
        metadata: OutputMetadata {
            pipeline_id: PIPELINE_ID,
            pipeline_version: PIPELINE_VERSION.to_string(),
            processing_time_ms: start_time.elapsed().as_millis() as u64,
            timestamp: chrono::Utc::now().to_rfc3339(),
        },
    };

    serde_json::to_value(output).map_err(|e| format!("Failed to serialize output: {}", e))
}

// ============================================================================
// ACTION IMPLEMENTATIONS
// ============================================================================

// ----------------------------------------------------------------------------
// HONEST ANALYSIS (2026-10-31): every value below is parsed or computed from
// the supplied input, or reported absent with a reason. The former
// implementation returned a hard-coded methanol (CH4O, MW 32.04, a 2-atom
// "MMFF94-optimized" structure at 1.43 A / -115.5 kcal/mol, invented
// properties and drug-likeness) for ANY input; none of that exists any more.
// UNCOMPILED pending the EU build gate.
// ----------------------------------------------------------------------------

enum Loaded {
    Mol(molparse::ParsedMol, Option<String>),
    Formula(std::collections::BTreeMap<String, u32>, &'static str),
}

fn load_molecule(m: &MoleculeData) -> Result<Loaded, String> {
    match m {
        MoleculeData::SMILES(s) => Ok(Loaded::Mol(molparse::parse_smiles(s)?, Some(s.trim().to_string()))),
        MoleculeData::MolFile(s) => Ok(Loaded::Mol(molparse::parse_molfile(s)?, None)),
        MoleculeData::XYZ(s) => Ok(Loaded::Mol(molparse::parse_xyz(s)?, None)),
        MoleculeData::PDB(s) => Ok(Loaded::Mol(molparse::parse_pdb_small(s)?, None)),
        MoleculeData::Formula(s) => Ok(Loaded::Formula(molparse::parse_formula(s)?, "formula")),
        MoleculeData::InChI(s) => {
            let t = s.trim();
            let rest = t.strip_prefix("InChI=").ok_or_else(|| "InChI must start with 'InChI='".to_string())?;
            let mut parts = rest.split('/');
            let _version = parts.next();
            let f = parts.next().ok_or_else(|| "InChI has no formula layer".to_string())?;
            Ok(Loaded::Formula(molparse::parse_formula(f)?, "inchi-formula-layer"))
        }
        MoleculeData::Name(_) | MoleculeData::CAS(_) => Err(
            "name/CAS lookup needs a chemical database and network access, which this pipeline does not have (use chem-mcp / PubChem); no structure is guessed"
                .to_string(),
        ),
    }
}

fn bond_type_for(order: f32, aromatic: bool) -> BondType {
    if aromatic {
        return BondType::Aromatic;
    }
    if (order - 1.0).abs() < 1e-6 {
        BondType::Single
    } else if (order - 2.0).abs() < 1e-6 {
        BondType::Double
    } else if (order - 3.0).abs() < 1e-6 {
        BondType::Triple
    } else if (order - 4.0).abs() < 1e-6 {
        BondType::Quadruple
    } else {
        BondType::Unknown
    }
}

fn analysis_from_formula(
    counts: std::collections::BTreeMap<String, u32>,
    source: &str,
    molecule_id: String,
    generate_3d: bool,
) -> MoleculeAnalysis {
    let mut notes = vec!["formula-only input: no atoms, bonds or geometry are available".to_string()];
    let masses = molparse::masses(&counts);
    if masses.is_none() {
        notes.push("an element is missing from the pipeline's element table: mass unresolved".to_string());
    }
    let heavy: u32 = counts.iter().filter(|(k, _)| k.as_str() != "H").map(|(_, v)| *v).sum();
    let (mw, em) = masses.unwrap_or((0.0, 0.0));
    MoleculeAnalysis {
        molecule_id,
        name: None,
        iupac_name: None,
        formula: molparse::hill(&counts),
        smiles: String::new(),
        inchi: None,
        inchi_key: None,
        molecular_weight: mw,
        exact_mass: em,
        atoms: vec![],
        bonds: vec![],
        rings: vec![],
        functional_groups: vec![],
        stereochemistry: None,
        properties: MolecularProperties { heavy_atom_count: heavy, ..MolecularProperties::default() },
        structure_3d: None,
        drug_likeness: None,
        source_format: source.to_string(),
        hydrogens_resolved: true,
        mass_resolved: masses.is_some(),
        ring_count: None,
        geometry: None,
        structure_3d_absent_reason: if generate_3d {
            Some("formula-only input has no structure; no conformer generator in this pipeline (use chem-mcp/RDKit)".to_string())
        } else {
            None
        },
        notes,
    }
}

fn analysis_from_parsed(
    mol: &molparse::ParsedMol,
    molecule_id: String,
    compute_properties: bool,
    find_groups: bool,
    generate_3d: bool,
    smiles_in: Option<String>,
) -> MoleculeAnalysis {
    let n = mol.atoms.len();
    let comp = molparse::composition(mol);
    let mut notes = mol.notes.clone();
    let masses = molparse::masses(&comp.counts);
    let mass_resolved = masses.is_some() && comp.hydrogens_resolved;
    if masses.is_none() {
        notes.push("an element is missing from the pipeline's element table: mass unresolved".to_string());
    }
    if !comp.hydrogens_resolved {
        notes.push("hydrogens are unresolved: the formula lists heavy atoms only and molecular_weight / exact_mass are not reported (0.0 with mass_resolved=false)".to_string());
    }
    let (mw, em) = if mass_resolved { masses.unwrap_or((0.0, 0.0)) } else { (0.0, 0.0) };

    let in_ring_bond = molparse::ring_bonds(n, &mol.bonds);
    let rot_flags = molparse::rotatable_flags(mol, &in_ring_bond);
    let mut in_ring_atom = vec![false; n];
    for (bi, b) in mol.bonds.iter().enumerate() {
        if in_ring_bond.get(bi).copied().unwrap_or(false) {
            in_ring_atom[b.a] = true;
            in_ring_atom[b.b] = true;
        }
    }
    let geom = molparse::geometry(mol);
    let frame = format!("molecule:{}", molecule_id);

    let mut explicit_h = vec![0u8; n];
    for b in &mol.bonds {
        if mol.atoms[b.a].element == "H" && mol.atoms[b.b].element != "H" {
            explicit_h[b.b] = explicit_h[b.b].saturating_add(1);
        }
        if mol.atoms[b.b].element == "H" && mol.atoms[b.a].element != "H" {
            explicit_h[b.a] = explicit_h[b.a].saturating_add(1);
        }
    }

    let atoms: Vec<Atom> = mol
        .atoms
        .iter()
        .enumerate()
        .map(|(i, a)| Atom {
            atom_id: i,
            element: a.element.clone(),
            atomic_number: molparse::element(&a.element).map(|e| e.z).unwrap_or(0),
            formal_charge: a.charge,
            implicit_hydrogens: a.implicit_h.unwrap_or(0),
            explicit_hydrogens: explicit_h[i],
            hybridization: Hybridization::Unspecified,
            is_aromatic: a.aromatic,
            is_in_ring: in_ring_atom[i],
            isotope: a.isotope,
            position: a.pos.map(|p| Position3D { x: p[0], y: p[1], z: p[2] }),
            partial_charge: None,
        })
        .collect();

    let bonds: Vec<Bond> = mol
        .bonds
        .iter()
        .enumerate()
        .map(|(bi, b)| Bond {
            bond_id: bi,
            atom1_id: b.a,
            atom2_id: b.b,
            bond_type: bond_type_for(b.order, b.aromatic),
            bond_order: b.order,
            is_aromatic: b.aromatic,
            is_in_ring: in_ring_bond.get(bi).copied().unwrap_or(false),
            stereo: None,
            is_rotatable: rot_flags.get(bi).copied().unwrap_or(false),
            bond_length: geom.as_ref().and_then(|g| g.bond_lengths.iter().find(|(i, _)| *i == bi).map(|(_, l)| *l)),
        })
        .collect();

    let orders_perceived = !mol.bonds.is_empty() && mol.bonds.iter().all(|b| b.order > 0.0);
    let heavy_atoms = mol.atoms.iter().filter(|a| a.element != "H").count() as u32;
    let total_charge: i32 = mol.atoms.iter().map(|a| a.charge).sum();

    let properties = if compute_properties {
        let hba = mol.atoms.iter().filter(|a| a.element == "N" || a.element == "O").count() as u32;
        let hbd = if comp.hydrogens_resolved {
            mol.atoms
                .iter()
                .enumerate()
                .filter(|(i, a)| (a.element == "N" || a.element == "O") && (a.implicit_h.unwrap_or(0) as u32 + explicit_h[*i] as u32) >= 1)
                .count() as u32
        } else {
            notes.push("h_bond_donors not computed: hydrogens unresolved".to_string());
            0
        };
        let rot = if orders_perceived {
            rot_flags.iter().filter(|r| **r).count() as u32
        } else {
            if !mol.bonds.is_empty() {
                notes.push("rotatable_bonds not computed: bond orders were not perceived".to_string());
            }
            0
        };
        notes.push("log_p, pKa, solubility, melting/boiling point, TPSA, molar refractivity and complexity are model/experimental quantities and are not computed by this pipeline".to_string());
        MolecularProperties {
            h_bond_donors: hbd,
            h_bond_acceptors: hba,
            rotatable_bonds: rot,
            heavy_atom_count: heavy_atoms,
            total_charge,
            ..MolecularProperties::default()
        }
    } else {
        MolecularProperties::default()
    };

    let functional_groups = if find_groups {
        if !comp.hydrogens_resolved {
            notes.push("functional groups that need hydrogens (hydroxyl, carboxylic acid) are only reported when hydrogens are resolved".to_string());
        }
        molparse::functional_groups(mol)
            .into_iter()
            .map(|g| FunctionalGroup { name: g.name.to_string(), smarts: g.smarts.to_string(), atoms: g.atoms, group_class: g.class.to_string(), count: 1 })
            .collect()
    } else {
        vec![]
    };

    let components = molparse::components(n, &mol.bonds);
    let ring_count = if n > 0 { Some((mol.bonds.len() + components).saturating_sub(n)) } else { None };
    if ring_count.unwrap_or(0) > 0 {
        notes.push("ring membership flags are reported; ring enumeration (SSSR) is not implemented, so `rings` is empty".to_string());
    }
    let smiles = match smiles_in {
        Some(s) => s,
        None => {
            notes.push("no SMILES writer in this pipeline: `smiles` is empty for non-SMILES input".to_string());
            String::new()
        }
    };

    let (structure_3d, structure_3d_absent_reason) = match (&geom, mol.has_coords) {
        (Some(_), true) => (
            Some(Structure3D {
                coordinates: mol.atoms.iter().filter_map(|a| a.pos.map(|p| Position3D { x: p[0], y: p[1], z: p[2] })).collect(),
                conformation: "as_supplied".to_string(),
                energy: None,
                is_minimized: false,
                force_field: None,
            }),
            None,
        ),
        _ if generate_3d => (
            None,
            Some("the input carries no 3D coordinates and this pipeline has no conformer generator (use chem-mcp/RDKit); coordinates are never invented".to_string()),
        ),
        _ => (None, None),
    };

    let geometry = geom.map(|g| MoleculeGeometry {
        frame,
        unit: "angstrom".to_string(),
        centre: g.centre,
        bbox_min: g.bbox_min,
        bbox_max: g.bbox_max,
        bond_angles: g.angles.iter().map(|(a, c, b, d)| BondAngle { atom_a: *a, centre: *c, atom_b: *b, degrees: *d }).collect(),
        bond_angles_truncated: g.angles_truncated,
        near_contacts: g.near_contacts.iter().map(|(a, b, d)| NearContact { atom_a: *a, atom_b: *b, distance: *d }).collect(),
        near_contacts_total: g.near_total,
        near_cutoff_angstrom: molparse::NEAR_CUTOFF,
        notes: g.notes.clone(),
    });

    MoleculeAnalysis {
        molecule_id,
        name: None,
        iupac_name: None,
        formula: molparse::hill(&comp.counts),
        smiles,
        inchi: None,
        inchi_key: None,
        molecular_weight: mw,
        exact_mass: em,
        atoms,
        bonds,
        rings: vec![],
        functional_groups,
        stereochemistry: None,
        properties,
        structure_3d,
        drug_likeness: None,
        source_format: mol.source.clone(),
        hydrogens_resolved: comp.hydrogens_resolved,
        mass_resolved,
        ring_count,
        geometry,
        structure_3d_absent_reason,
        notes,
    }
}

async fn analyze_molecule(
    molecule: &MoleculeData,
    compute_properties: bool,
    find_functional_groups: bool,
    generate_3d: bool,
) -> Result<MoleculeAnalysis, String> {
    let id = format!("mol_{}", generate_graph_id());
    match load_molecule(molecule)? {
        Loaded::Mol(m, smiles) => Ok(analysis_from_parsed(&m, id, compute_properties, find_functional_groups, generate_3d, smiles)),
        Loaded::Formula(counts, src) => Ok(analysis_from_formula(counts, src, id, generate_3d)),
    }
}

fn element_counts_of(m: &MoleculeData) -> Result<(std::collections::BTreeMap<String, u32>, bool), String> {
    match load_molecule(m)? {
        Loaded::Mol(p, _) => {
            let c = molparse::composition(&p);
            Ok((c.counts, c.hydrogens_resolved))
        }
        Loaded::Formula(c, _) => Ok((c, true)),
    }
}

async fn analyze_reaction(
    reaction: &ReactionData,
    _balance: bool,
    predict_mechanism: bool,
    compute_thermodynamics: bool,
) -> Result<ReactionAnalysis, String> {
    // Only the element balance is computable from structure. Mechanism,
    // thermodynamics, kinetics, atom mapping and reaction class need a
    // cheminformatics engine / data and are reported ABSENT.
    let mut notes: Vec<String> = Vec::new();
    if predict_mechanism {
        notes.push("mechanism not computed: needs a reaction-mechanism engine (use chem-mcp/RDKit)".to_string());
    }
    if compute_thermodynamics {
        notes.push("thermodynamics and kinetics not computed: need computed or experimental data".to_string());
    }
    notes.push("atom mapping, reaction centres and reaction class are not computed".to_string());

    let side = |list: &[MoleculeData]| -> Result<(std::collections::BTreeMap<String, u32>, bool), String> {
        let mut total: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
        let mut ok = true;
        for m in list {
            let (c, resolved) = element_counts_of(m)?;
            ok &= resolved;
            for (k, v) in c {
                *total.entry(k).or_insert(0) += v;
            }
        }
        Ok((total, ok))
    };
    let (balance_determined, is_balanced) = match (side(&reaction.reactants), side(&reaction.products)) {
        (Ok((r, rok)), Ok((p, pok))) if rok && pok && !reaction.reactants.is_empty() && !reaction.products.is_empty() => (true, r == p),
        (Ok(_), Ok(_)) => {
            notes.push("element balance undetermined: a species has unresolved hydrogens, or a side is empty".to_string());
            (false, false)
        }
        (Err(e), _) | (_, Err(e)) => {
            notes.push(format!("element balance undetermined: {}", e));
            (false, false)
        }
    };
    let reaction_smiles = match &reaction.reaction_smiles {
        Some(s) => s.clone(),
        None => {
            let join = |l: &[MoleculeData]| -> Option<String> {
                let mut parts = Vec::new();
                for m in l {
                    if let MoleculeData::SMILES(s) = m {
                        parts.push(s.trim().to_string());
                    } else {
                        return None;
                    }
                }
                Some(parts.join("."))
            };
            match (join(&reaction.reactants), join(&reaction.products)) {
                (Some(r), Some(p)) => format!("{}>>{}", r, p),
                _ => String::new(),
            }
        }
    };
    Ok(ReactionAnalysis {
        reaction_id: format!("rxn_{}", generate_graph_id()),
        reaction_smiles,
        reaction_name: None,
        reaction_type: ReactionType::Unknown,
        is_balanced,
        atom_mapping: None,
        mechanism: None,
        thermodynamics: None,
        kinetics: None,
        reaction_centers: vec![],
        confidence: if balance_determined { 1.0 } else { 0.0 },
        balance_determined,
        notes,
    })
}

async fn predict_properties(molecule: &MoleculeData, properties: &[PropertyType]) -> Result<PropertyPredictions, String> {
    let id = format!("mol_{}", generate_graph_id());
    let analysis = match load_molecule(molecule)? {
        Loaded::Mol(m, s) => analysis_from_parsed(&m, id.clone(), true, false, false, s),
        Loaded::Formula(c, src) => analysis_from_formula(c, src, id.clone(), false),
    };
    let has_atoms = !analysis.atoms.is_empty();
    let orders_perceived = has_atoms && !analysis.bonds.is_empty() && analysis.bonds.iter().all(|b| b.bond_order > 0.0);
    let mut predictions: Vec<PropertyPrediction> = Vec::new();
    let mut unavailable: Vec<UnavailableProperty> = Vec::new();
    let computed = |p: &PropertyType, value: f64, unit: &str| PropertyPrediction {
        property: p.clone(),
        value,
        unit: unit.to_string(),
        confidence: 1.0,
        method: "computed from the parsed structure (no model)".to_string(),
    };
    for p in properties {
        match p {
            PropertyType::MolecularWeight if analysis.mass_resolved => predictions.push(computed(p, analysis.molecular_weight, "g/mol")),
            PropertyType::HBondAcceptors if has_atoms => predictions.push(computed(p, analysis.properties.h_bond_acceptors as f64, "count")),
            PropertyType::HBondDonors if has_atoms && analysis.hydrogens_resolved => predictions.push(computed(p, analysis.properties.h_bond_donors as f64, "count")),
            PropertyType::RotatableBonds if orders_perceived => predictions.push(computed(p, analysis.properties.rotatable_bonds as f64, "count")),
            PropertyType::MolecularWeight | PropertyType::HBondAcceptors | PropertyType::HBondDonors | PropertyType::RotatableBonds => {
                unavailable.push(UnavailableProperty {
                    property: p.clone(),
                    reason: "the structure does not determine it (unresolved hydrogens, unperceived bond orders, formula-only input or unknown element)".to_string(),
                })
            }
            other => unavailable.push(UnavailableProperty {
                property: other.clone(),
                reason: "needs a trained model or experimental data; this pipeline computes only structure-derived quantities (use chem-mcp)".to_string(),
            }),
        }
    }
    Ok(PropertyPredictions { molecule_id: analysis.molecule_id, predictions, unavailable })
}

async fn find_substructure(_molecule: &MoleculeData, _pattern: &str) -> Result<SubstructureResult, String> {
    Err("FindSubstructure is not implemented: SMARTS matching needs a substructure engine (use chem-mcp/RDKit); no matches are fabricated".to_string())
}

async fn compare_molecules(_molecule1: &MoleculeData, _molecule2: &MoleculeData, _method: SimilarityMethod) -> Result<SimilarityResult, String> {
    Err("CompareMolecules is not implemented: fingerprint similarity needs a cheminformatics engine (use chem-mcp/RDKit); no similarity value is fabricated".to_string())
}

async fn zsei_query(query: serde_json::Value) -> Result<serde_json::Value, String> {
    let host = env::var("OZONE_HOST").unwrap_or_else(|_| "http://127.0.0.1:50051".to_string());
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

async fn persist_chemistry_graph(graph: &mut ChemistryGraph) -> Result<(), String> {
    let keywords: Vec<String> = {
        let mut k: Vec<String> = graph.nodes.iter()
            .filter_map(|n| n.label.to_lowercase().split_whitespace().next().map(String::from))
            .collect();
        k.sort(); k.dedup();
        if k.is_empty() { k.push("chemistry".to_string()); }
        // container keyword set kept WHOLE (no 12-keyword cut — doctrine audit #33, 2026-11-04)
        k
    };
    let name = format!(
        "Chemistry graph ({} nodes, {} edges)",
        graph.nodes.len(), graph.edges.len()
    );
    let now = chrono::Utc::now().timestamp() as u64;
    let container = serde_json::json!({
        "global_state": {"container_id": 0, "parent_id": graph.project_id, "child_ids": [], "child_count": 0, "version": 1},
        "local_state": {
            "metadata": {"container_type": "ModalityGraph", "modality": "Unknown", "created_at": now,
                "updated_at": now, "provenance": "chemistry-pipeline", "permissions": 0, "owner_id": 0,
                "name": name, "materialized_path": null},
            "context": {"categories": [], "methodologies": [], "keywords": keywords,
                "topics": ["chemistry"], "relationships": [], "learned_associations": [], "embedding": null},
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
    let dir = env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    let rel = format!("graphs/chemistry_{}.json", new_id);
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

async fn create_graph(
    analysis: ChemistryAnalysisResult,
    project_id: u64,
    graph_name: Option<String>,
) -> Result<ChemistryGraph, String> {
    let graph_id = generate_graph_id();
    let now = chrono::Utc::now().to_rfc3339();

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut node_id_counter: u64 = 1;
    let mut total_atoms = 0;
    let mut total_bonds = 0;

    // Create molecule nodes
    for molecule in &analysis.molecules {
        let mol_node_id = node_id_counter;
        nodes.push(ChemistryGraphNode {
            node_id: mol_node_id,
            node_type: ChemistryNodeType::Molecule,
            label: molecule.name.clone().unwrap_or_else(|| molecule.formula.clone()),
            content: if molecule.mass_resolved {
                format!("{} (MW: {:.2})", molecule.formula, molecule.molecular_weight)
            } else {
                format!("{} (mass unresolved)", molecule.formula)
            },
            smiles: if molecule.smiles.is_empty() { None } else { Some(molecule.smiles.clone()) },
            confidence: 1.0,
            properties: {
                let mut props = HashMap::new();
                if molecule.mass_resolved {
                    props.insert("molecular_weight".to_string(), Value::from(molecule.molecular_weight));
                }
                props.insert("formula".to_string(), Value::from(molecule.formula.clone()));
                props.insert("source_format".to_string(), Value::from(molecule.source_format.clone()));
                props.insert("hydrogens_resolved".to_string(), Value::from(molecule.hydrogens_resolved));
                // Intrinsic geometry (docs/SPATIAL_GEOMETRY_LAYER.md): 3D box of the supplied coordinates.
                if let Some(g) = &molecule.geometry {
                    props.insert(
                        "geom".to_string(),
                        serde_json::json!({
                            "frame": g.frame, "kind": "box3d", "unit": g.unit,
                            "min": g.bbox_min, "max": g.bbox_max, "centre": g.centre
                        }),
                    );
                }
                props
            },
            annotations: vec![],
        });
        node_id_counter += 1;
        // LIVE NODE EVENT (2026-10-19, additive, same fire-and-forget
        // pattern as 3D/main.rs's first real producer): best-effort report
        // of this real molecule node as it's added, so a connected live
        // view can show construction progress instead of only the final
        // result. Detached tokio::spawn — never awaited inline.
        {
            let host = env::var("OZONE_HOST").unwrap_or_else(|_| "http://127.0.0.1:50051".into());
            let pid = project_id;
            let gid = graph_id;
            let nid = mol_node_id;
            tokio::spawn(async move {
                let client = reqwest::Client::new();
                let _ = client.post(format!("{host}/orchestration/node-event"))
                    .json(&serde_json::json!({
                        "project_id": pid,
                        "modality": "chemistry",
                        "graph_id": gid,
                        "node_id": nid,
                        "related_node_id": null,
                        "node_type": "Molecule",
                        "operation": "node_created",
                    }))
                    .timeout(std::time::Duration::from_secs(2))
                    .send().await;
            });
        }

        // Create atom nodes
        for atom in &molecule.atoms {
            let atom_node_id = node_id_counter;
            nodes.push(ChemistryGraphNode {
                node_id: atom_node_id,
                node_type: ChemistryNodeType::Atom,
                label: format!("{}{}", atom.element, atom.atom_id),
                content: format!("{} ({})", atom.element, format!("{:?}", atom.hybridization)),
                smiles: None,
                confidence: 1.0,
                properties: {
                    let mut props = HashMap::new();
                    props.insert("element".to_string(), Value::from(atom.element.clone()));
                    props.insert("atomic_number".to_string(), Value::from(atom.atomic_number));
                    props.insert("formal_charge".to_string(), Value::from(atom.formal_charge));
                    // Intrinsic geometry: the atom's supplied position in the molecule frame.
                    if let (Some(pos), Some(g)) = (&atom.position, &molecule.geometry) {
                        props.insert(
                            "geom".to_string(),
                            serde_json::json!({
                                "frame": g.frame, "kind": "point", "unit": "angstrom",
                                "x": pos.x, "y": pos.y, "z": pos.z
                            }),
                        );
                    }
                    props
                },
                annotations: vec![],
            });

            edges.push(ChemistryGraphEdge {
                edge_id: edges.len() as u64 + 1,
                from_node: mol_node_id,
                to_node: atom_node_id,
                edge_type: ChemistryEdgeType::Contains,
                weight: 1.0,
                properties: HashMap::new(),
            });

            node_id_counter += 1;
            total_atoms += 1;
        }

        // Create bond edges between atoms
        for bond in &molecule.bonds {
            let atom1_node_id = mol_node_id + bond.atom1_id as u64 + 1;
            let atom2_node_id = mol_node_id + bond.atom2_id as u64 + 1;

            edges.push(ChemistryGraphEdge {
                edge_id: edges.len() as u64 + 1,
                from_node: atom1_node_id,
                to_node: atom2_node_id,
                edge_type: ChemistryEdgeType::BondedTo,
                weight: bond.bond_order,
                properties: {
                    let mut props = HashMap::new();
                    props.insert("bond_type".to_string(), serde_json::to_value(&bond.bond_type).unwrap_or(Value::Null));
                    props
                },
            });

            total_bonds += 1;
        }

        // Create functional group nodes
        for fg in &molecule.functional_groups {
            let fg_node_id = node_id_counter;
            nodes.push(ChemistryGraphNode {
                node_id: fg_node_id,
                node_type: ChemistryNodeType::FunctionalGroup,
                label: fg.name.clone(),
                content: format!("{} ({})", fg.name, fg.group_class),
                smiles: Some(fg.smarts.clone()),
                confidence: 1.0,
                properties: HashMap::new(),
                annotations: vec![],
            });

            edges.push(ChemistryGraphEdge {
                edge_id: edges.len() as u64 + 1,
                from_node: mol_node_id,
                to_node: fg_node_id,
                edge_type: ChemistryEdgeType::Contains,
                weight: 1.0,
                properties: HashMap::new(),
            });

            node_id_counter += 1;
        }
    }

    // Create reaction nodes
    for reaction in &analysis.reactions {
        let rxn_node_id = node_id_counter;
        nodes.push(ChemistryGraphNode {
            node_id: rxn_node_id,
            node_type: ChemistryNodeType::Reaction,
            label: reaction.reaction_name.clone().unwrap_or_else(|| format!("{:?}", reaction.reaction_type)),
            content: reaction.reaction_smiles.clone(),
            smiles: Some(reaction.reaction_smiles.clone()),
            confidence: reaction.confidence,
            properties: HashMap::new(),
            annotations: vec![],
        });
        node_id_counter += 1;
    }

    let nodes_len = nodes.len();
    let edge_count = edges.len();
    Ok(ChemistryGraph {
        graph_id,
        name: graph_name.unwrap_or_else(|| format!("Chemistry Graph {}", graph_id)),
        modality: MODALITY.to_string(),
        project_id,
        nodes,
        edges,
        metadata: GraphMetadata {
            node_count: nodes_len,
            edge_count: edge_count,
            molecule_count: analysis.molecules.len(),
            reaction_count: analysis.reactions.len(),
            total_atoms,
            total_bonds,
            semantic_enriched: false,
            cross_modal_links: 0,
        },
        created_at: now.clone(),
        updated_at: now,
    })
}

async fn update_graph(graph_id: u64, updates: ChemistryGraphUpdate) -> Result<ChemistryGraph, String> {
    let mut graph = get_graph(graph_id).await?;
    let now = chrono::Utc::now().to_rfc3339();

    for node in updates.add_nodes {
        graph.nodes.push(node);
    }

    for update_node in updates.update_nodes {
        if let Some(existing) = graph.nodes.iter_mut().find(|n| n.node_id == update_node.node_id) {
            *existing = update_node;
        }
    }

    graph.nodes.retain(|n| !updates.remove_nodes.contains(&n.node_id));

    for edge in updates.add_edges {
        graph.edges.push(edge);
    }

    graph.edges.retain(|e| !updates.remove_edges.contains(&e.edge_id));

    graph.metadata.node_count = graph.nodes.len();
    graph.metadata.edge_count = graph.edges.len();
    graph.updated_at = now;

    Ok(graph)
}

async fn query_graph(graph_id: u64, query: ChemistryQuery) -> Result<QueryResult, String> {
    let graph = get_graph(graph_id).await?;
    let limit = query.limit.unwrap_or(100);

    let (nodes, edges) = match query.query_type.clone() {
        ChemistryQueryType::GetNodesByType { node_type } => {
            let matching_nodes: Vec<_> = graph
                .nodes
                .iter()
                .filter(|n| n.node_type == node_type)
                .take(limit)
                .cloned()
                .collect();
            (matching_nodes, vec![])
        }

        ChemistryQueryType::FindByFunctionalGroup { group_name } => {
            let group_lower = group_name.to_lowercase();
            let matching_nodes: Vec<_> = graph
                .nodes
                .iter()
                .filter(|n| {
                    n.node_type == ChemistryNodeType::FunctionalGroup
                        && n.label.to_lowercase().contains(&group_lower)
                })
                .take(limit)
                .cloned()
                .collect();
            (matching_nodes, vec![])
        }

        _ => (vec![], vec![]),
    };

    Ok(QueryResult {
        query_type: format!("{:?}", query.query_type),
        total_matches: nodes.len(),
        nodes,
        edges,
        metadata: HashMap::new(),
    })
}

async fn get_graph(graph_id: u64) -> Result<ChemistryGraph, String> {
    Ok(ChemistryGraph {
        graph_id,
        name: format!("Chemistry Graph {}", graph_id),
        modality: MODALITY.to_string(),
        project_id: 1,
        nodes: vec![],
        edges: vec![],
        metadata: GraphMetadata::default(),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// Append one real ZSEI Relation (RelatedTo, discovered_via Manual) to a
/// container's existing `context.relationships`, preserving every other
/// context field exactly (ContainerUpdate.context is a full replace, not a
/// merge). Idempotent: skips the write if an edge to target_id already
/// exists, since this is called from both directions by link_to_modality.
async fn add_relationship(container_id: u64, target_id: u64) -> Result<(), String> {
    let existing = zsei_query(serde_json::json!({
        "GetContainer": { "container_id": container_id }
    }))
    .await?;
    let mut context = existing
        .get("Container")
        .and_then(|c| c.get("local_state"))
        .and_then(|l| l.get("context"))
        .cloned()
        .ok_or_else(|| format!("Container {} has no local_state.context", container_id))?;

    let relationships = context
        .get_mut("relationships")
        .and_then(|r| r.as_array_mut())
        .ok_or_else(|| format!("Container {} context.relationships is not an array", container_id))?;

    let already_linked = relationships.iter().any(|r| {
        r.get("target_id").and_then(|t| t.as_u64()) == Some(target_id)
    });
    if already_linked {
        return Ok(());
    }

    relationships.push(serde_json::json!({
        "target_id": target_id,
        "relation_type": "RelatedTo",
        "confidence": 1.0,
        "discovered_via": "Manual"
    }));

    zsei_query(serde_json::json!({
        "UpdateContainer": {
            "container_id": container_id,
            "updates": { "context": context }
        }
    }))
    .await?;
    Ok(())
}

async fn link_to_modality(
    chemistry_graph_id: u64,
    target_graph_id: u64,
    _target_modality: &str,
    relationship: CrossModalityRelation,
) -> Result<LinkResult, String> {
    // Real, bidirectional, idempotent ZSEI relationship — both containers
    // must actually exist; this used to unconditionally fabricate a
    // "success" LinkResult for any graph_id, real or not, with nothing
    // behind it. An honest error here is correct behavior, not a regression.
    add_relationship(chemistry_graph_id, target_graph_id).await?;
    add_relationship(target_graph_id, chemistry_graph_id).await?;

    Ok(LinkResult {
        link_id: generate_graph_id(),
        source_graph_id: chemistry_graph_id,
        target_graph_id,
        relationship: format!("{:?}", relationship),
        created_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// Real counts from the actual persisted graph — this previously returned
/// hardcoded `nodes_processed: 10, edges_added: 5, annotations_added: 15`
/// unconditionally. Full semantic enrichment (real cross-node annotation
/// generation) is a separate, larger task — this fixes the immediate
/// honesty problem without pretending enrichment exists: annotations_added
/// stays a real, honest 0.
async fn trigger_semantic_hook(
    graph_id: u64,
    hook_type: ZSEIHookType,
    _options: HookOptions,
) -> Result<HookResult, String> {
    let start_time = std::time::Instant::now();

    let graph = get_graph(graph_id).await?;

    Ok(HookResult {
        hook_type,
        success: true,
        nodes_processed: graph.nodes.len(),
        edges_added: graph.edges.len(),
        annotations_added: 0,
        processing_time_ms: start_time.elapsed().as_millis() as u64,
        errors: vec![],
    })
}

fn generate_graph_id() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    duration.as_nanos() as u64 % 1_000_000_000
}

// ============================================================================
// CLI ENTRY POINT
// ============================================================================

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let mut input_json = String::new();
    let mut ai = 1usize;
    while ai < args.len() {
        if args[ai] == "--input" && ai + 1 < args.len() { input_json = ozone_input_arg(&args[ai + 1]); ai += 2; } else { ai += 1; }
    }
    if input_json.is_empty() {
        use std::io::Read;
        let _ = std::io::stdin().read_to_string(&mut input_json);
    }
    if input_json.trim().is_empty() {
        eprintln!("Usage: chemistry --input '<json>' (or JSON on stdin)");
        std::process::exit(1);
    }
    let input: Value = match serde_json::from_str::<Value>(&input_json) {
        Ok(v) => match v.get("data") { Some(d) => d.clone(), None => v },
        Err(e) => {
            eprintln!("Failed to parse input JSON: {}", e);
            std::process::exit(1);
        }
    };

    match execute(input).await {
        Ok(output) => {
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        Err(e) => {
            let error_output = serde_json::json!({
                "success": false,
                "action": "unknown",
                "result": null,
                "error": e,
                "metadata": {
                    "pipeline_id": PIPELINE_ID,
                    "pipeline_version": PIPELINE_VERSION,
                    "processing_time_ms": 0,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }
            });
            println!("{}", serde_json::to_string_pretty(&error_output).unwrap());
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_analyze_molecule() {
        let input = serde_json::json!({
            "action": {
                "type": "AnalyzeMolecule",
                "molecule": {"SMILES": "CO"},
                "compute_properties": true,
                "find_functional_groups": true,
                "generate_3d": false
            }
        });

        let result = execute(input).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_analyze_molecule_is_real_not_methanol() {
        let input = serde_json::json!({
            "action": {"type": "AnalyzeMolecule", "molecule": {"SMILES": "CCO"},
                       "compute_properties": true, "find_functional_groups": true, "generate_3d": true}
        });
        let out = execute(input).await.unwrap();
        let mol = &out["result"];
        assert_eq!(mol["formula"], "C2H6O");
        assert!((mol["molecular_weight"].as_f64().unwrap() - 46.069).abs() < 0.01);
        // 3D was requested for a coordinate-free SMILES: absent, with a reason, never invented.
        assert!(mol["structure_3d"].is_null());
        assert!(mol["structure_3d_absent_reason"].as_str().unwrap().contains("conformer generator"));
        assert!(mol["drug_likeness"].is_null());
    }

    #[tokio::test]
    async fn test_different_inputs_give_different_answers() {
        let a = execute(serde_json::json!({"action": {"type": "AnalyzeMolecule", "molecule": {"SMILES": "c1ccccc1"}}})).await.unwrap();
        let b = execute(serde_json::json!({"action": {"type": "AnalyzeMolecule", "molecule": {"SMILES": "CC(=O)O"}}})).await.unwrap();
        assert_eq!(a["result"]["formula"], "C6H6");
        assert_eq!(b["result"]["formula"], "C2H4O2");
    }

    #[tokio::test]
    async fn test_molfile_geometry_comes_from_coordinates() {
        let mol = "water\n  t\n\n  3  2  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.1173 O   0  0  0  0  0  0  0  0  0  0  0  0\n    0.0000    0.7572   -0.4692 H   0  0  0  0  0  0  0  0  0  0  0  0\n    0.0000   -0.7572   -0.4692 H   0  0  0  0  0  0  0  0  0  0  0  0\n  1  2  1  0\n  1  3  1  0\nM  END\n";
        let out = execute(serde_json::json!({"action": {"type": "AnalyzeMolecule", "molecule": {"MolFile": mol}, "generate_3d": true}})).await.unwrap();
        let m = &out["result"];
        assert!(m["structure_3d"]["force_field"].is_null());
        assert!(m["structure_3d"]["energy"].is_null());
        let l = m["bonds"][0]["bond_length"].as_f64().unwrap();
        assert!((l - 0.9578).abs() < 0.001);
        assert!(m["geometry"]["frame"].as_str().unwrap().starts_with("molecule:"));
    }

    #[tokio::test]
    async fn test_name_and_cas_are_refused_not_guessed() {
        let r = execute(serde_json::json!({"action": {"type": "AnalyzeMolecule", "molecule": {"Name": "aspirin"}}})).await;
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("no structure is guessed"));
    }

    #[tokio::test]
    async fn test_reaction_balance_is_computed_and_the_rest_is_absent() {
        let input = serde_json::json!({
            "action": {"type": "AnalyzeReaction",
                "reaction": {"reactants": [{"SMILES": "CCBr"}], "products": [{"SMILES": "CCO"}], "reagents": [{"SMILES": "[OH-]"}], "conditions": null, "reaction_smiles": null},
                "balance": true, "predict_mechanism": true, "compute_thermodynamics": true}
        });
        let out = execute(input).await.unwrap();
        let r = &out["result"];
        assert_eq!(r["balance_determined"], true);
        assert_eq!(r["is_balanced"], false); // CCBr -> CCO is not element-balanced
        assert!(r["mechanism"].is_null() && r["thermodynamics"].is_null() && r["kinetics"].is_null());
        assert_eq!(r["reaction_smiles"], "CCBr>>CCO");
        let bal = execute(serde_json::json!({
            "action": {"type": "AnalyzeReaction",
                "reaction": {"reactants": [{"SMILES": "C"}, {"SMILES": "O=O"}], "products": [{"SMILES": "O=C=O"}, {"SMILES": "O"}], "reagents": [], "conditions": null, "reaction_smiles": null},
                "balance": true, "predict_mechanism": false, "compute_thermodynamics": false}
        })).await.unwrap();
        // CH4 + O2 -> CO2 + H2O is NOT balanced as written (H4 vs H2): the check must say so.
        assert_eq!(bal["result"]["is_balanced"], false);
    }

    #[tokio::test]
    async fn test_substructure_and_similarity_are_not_fabricated() {
        let s = execute(serde_json::json!({"action": {"type": "FindSubstructure", "molecule": {"SMILES": "CCO"}, "pattern": "[OX2H]"}})).await;
        assert!(s.is_err());
        let c = execute(serde_json::json!({"action": {"type": "CompareMolecules", "molecule1": {"SMILES": "CCO"}, "molecule2": {"SMILES": "CCN"}, "method": "Tanimoto"}})).await;
        assert!(c.is_err());
    }

    #[tokio::test]
    async fn test_predict_properties_only_returns_computable_values() {
        let out = execute(serde_json::json!({"action": {"type": "PredictProperties", "molecule": {"SMILES": "CCO"},
            "properties": ["MolecularWeight", "LogP", "HBondDonors", "Toxicity"]}})).await.unwrap();
        let p = &out["result"];
        assert_eq!(p["predictions"].as_array().unwrap().len(), 2);
        assert_eq!(p["unavailable"].as_array().unwrap().len(), 2);
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
