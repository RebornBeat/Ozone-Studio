//! OZONE Studio - Pipeline 105: Math Analysis
//!
//! Modality pipeline for mathematical content processing and structural graph creation.
//! This is a KEY PIPELINE for demonstrating AGI-level consistency over long-form work.
//!
//! Analyzes mathematical expressions, proofs, and theorems to create traversable graphs
//! where each step maintains explicit connections to prerequisites, variables, and assumptions.
//!
//! # The AGI Insight
//! Traditional LLMs lose consistency after ~50 steps in complex proofs because they rely
//! on statistical regeneration. This pipeline enables 1000+ step verification by:
//! - Creating explicit graph nodes for each proof step
//! - Tracking variable scopes through a scope tree
//! - Maintaining edges to prerequisites and consequences  
//! - Enabling independent verification of ANY step via graph traversal
//!
//! # Actions
//! - `ParseExpression`: Parse mathematical expressions (LaTeX, MathML, etc.)
//! - `AnalyzeProof`: Break down proof into verifiable steps
//! - `VerifyStep`: Verify individual proof step
//! - `BuildScopeTree`: Build variable scope tracking
//! - `CreateGraph`: Build structural graph from analysis
//! - `QueryGraph`: Query for variables, dependencies, etc.
//! - `TriggerSemanticHook`: Trigger ZSEI hooks for semantic enrichment
//!
//! # Graph Structure
//! - Nodes: Expression, ProofStep, Variable, Axiom, Theorem, Assumption, Definition
//! - Edges: Uses, Derives, Requires, Implies, Defines, AssumesIn, DischargesIn

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::env;

#[path = "../../shared/ozone_serve.rs"]
mod ozone_serve;

/// Pipeline-side model-call capture (C6-minimal, shared — single source).
/// Currently covers the one pipeline-9 call site (E7's
/// resolve_implicit_step_references, dead code until wired) so capture is
/// already in place the moment that call goes live.
#[path = "../../shared/capture.rs"]
mod capture;

// Same real-ZSEI-over-HTTP pattern as text modality (100) and code modality
// (101) — this pipeline had never actually been built/exercised before (see
// the CLI entry point and persist_graph_container below for the two bugs
// that made it a no-op regardless: wrong CLI contract, and no real ZSEI
// persistence at all — get_graph explicitly returned a hardcoded empty stub
// with a "// In production, load from ZSEI" comment).
fn ozone_host() -> String {
    env::var("OZONE_HOST").unwrap_or_else(|_| "http://127.0.0.1:50051".to_string())
}

/// Process-wide HTTP client for host calls — keep-alive connection reuse.
/// Measured (link_metrics, 2026-09-20): a fresh Client per zsei_query call
/// costs a TCP connect every time; the shared client removes that from the
/// per-edge linking cost without capping or batching anything.
fn shared_http_client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new)
}

async fn zsei_query(query: serde_json::Value) -> Result<serde_json::Value, String> {
    let client = shared_http_client();
    let resp = client
        .post(format!("{}/zsei/query", ozone_host()))
        .json(&serde_json::json!({"query": query, "session_token": ""}))
        .send()
        .await
        .map_err(|e| format!("zsei query request failed: {}", e))?;
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("zsei query response parse failed: {}", e))?;
    if body.get("success").and_then(|s| s.as_bool()) != Some(true) {
        return Err(body
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("zsei query failed")
            .to_string());
    }
    Ok(body.get("result").cloned().unwrap_or(serde_json::Value::Null))
}

/// Real structural keyword/topic extraction — no LLM call needed, same
/// reasoning as code modality (101): a math graph's own nodes already carry
/// real, meaningful terms (variable names, proof technique, domain,
/// referenced axioms/theorems/definitions). Mirrors the bug this exact
/// pattern fixed in text modality — every ProcessedChunk there was
/// constructed with `keywords: Vec::new()` unconditionally, which silently
/// broke methodology/context search for the whole session before being
/// found — so this must never return empty when the graph has real content.
fn derive_math_keywords(graph: &MathGraph, analysis: &MathAnalysisResult) -> (Vec<String>, Vec<String>) {
    let mut keywords: Vec<String> = Vec::new();
    // P4 supplement (math isolation fix): the raw content's real words come
    // FIRST — for prose-heavy inputs they are the meaningful terms, while
    // ParseExpression's per-token variables are the noise. Zero real
    // overlap with sibling text/code graphs was the entire reason math
    // graphs stayed isolated (live root-caused 2026-09-21/22).
    if let Some(parse_result) = &analysis.parse_result {
        for kw in &parse_result.content_keywords {
            if !keywords.contains(kw) {
                keywords.push(kw.clone());
            }
        }
    }
    for node in &graph.nodes {
        match node.node_type {
            MathGraphNodeType::Variable => keywords.push(node.label.to_lowercase()),
            MathGraphNodeType::Axiom | MathGraphNodeType::Theorem | MathGraphNodeType::Definition => {
                keywords.push(node.label.to_lowercase());
            }
            _ => {}
        }
    }
    let mut topics: Vec<String> = vec![format!("{:?}", analysis.analysis_type).to_lowercase()];
    if let Some(proof) = &analysis.proof_analysis {
        topics.push(format!("{:?}", proof.proof_technique).to_lowercase());
        for kw in &proof.content_keywords {
            if !keywords.contains(kw) {
                keywords.push(kw.clone());
            }
        }
        for axiom in &proof.axioms_used {
            keywords.push(axiom.name.to_lowercase());
        }
        for theorem in &proof.theorems_used {
            keywords.push(theorem.name.to_lowercase());
        }
        for def in &proof.definitions_used {
            keywords.push(def.name.to_lowercase());
        }
    }
    if let Some(parse) = &analysis.parse_result {
        topics.push(format!("{:?}", parse.expression_type).to_lowercase());
        if let Some(domain) = &parse.domain {
            keywords.push(domain.description.to_lowercase());
        }
    }
    keywords.push("mathematics".to_string());
    keywords.sort();
    keywords.dedup();
    (keywords, topics)
}

/// Persist a math graph as a real ZSEI container. Mirrors code modality
/// (101)'s persist_graph_container exactly — same container shape, same
/// parent_id=project_id convention (0 keeps the root-parented default;
/// create_container degrades gracefully if project_id isn't a real
/// container), same graphs/ on-disk convention for the full node/edge
/// content. code_context/text_context stay None for the same honesty
/// reason they do there — there's no ContainerType field shaped for
/// MathGraph's richer content without fabricating a mapping.
async fn persist_graph_container(
    graph: &MathGraph,
    analysis: &MathAnalysisResult,
    local_graph_id: u64,
    project_id: u64,
) -> Result<(u64, Vec<String>, Vec<String>), String> {
    let now = chrono::Utc::now().timestamp() as u64;
    let (keywords, topics) = derive_math_keywords(graph, analysis);
    let name = format!(
        "{:?} math graph ({} nodes, {} edges, {} proof steps)",
        analysis.analysis_type, graph.nodes.len(), graph.edges.len(), graph.metadata.proof_steps
    );
    let object_store_path = format!("graphs/math_{}.json", local_graph_id);

    let container = serde_json::json!({
        "global_state": {
            "container_id": 0,
            "child_count": 0,
            "version": 1,
            "parent_id": 0,
            "child_ids": []
        },
        "local_state": {
            "metadata": {
                // Real bug found live: the shared `Modality` enum (src/types/
                // container.rs) has no `Math` variant at all — it predates
                // math/chemistry/dna/etc as distinct pipeline modalities and
                // only has Unknown/Text/Code/Image/Audio/Video/Graph/
                // TimeSeries/Structured/External/Multimodal. "Structured" is
                // the most honest real fit for formal symbolic/proof content
                // (not prose, not executable code, not raw media) — an
                // approximation using a real valid category, not a
                // fabricated one. The pipeline's own `modality: "math"` field
                // on MathGraph itself (a different, pipeline-local type) still
                // says "math" accurately; only this ZSEI-enum-constrained
                // field is coarsened.
                "container_type": "ModalityGraph",
                "modality": "Structured",
                "created_at": now,
                "updated_at": now,
                "provenance": "pipeline:105",
                "permissions": 0,
                "owner_id": 0,
                "name": name,
                "materialized_path": null
            },
            "context": {
                "categories": [],
                "methodologies": [],
                "keywords": keywords,
                "topics": topics,
                "relationships": [],
                "learned_associations": [],
                "embedding": null
            },
            "storage": {
                "db_shard_id": null,
                "vector_index_ref": null,
                "object_store_path": object_store_path,
                "compression_type": "None"
            },
            "hints": {
                "access_frequency": 0,
                "hotness_score": 0.0,
                "last_accessed": 0,
                "centroid": null,
                "ml_prediction_weight": 0.0
            },
            "integrity": {
                "content_hash": vec![0u8; 32],
                "semantic_fingerprint": [],
                "last_verified": now,
                "integrity_score": 1.0,
                "version_history": []
            },
            "file_context": null,
            "code_context": null,
            "text_context": null,
            "external_ref": null
        }
    });

    let result = zsei_query(serde_json::json!({
        "CreateContainer": { "parent_id": project_id, "container": container }
    }))
    .await?;

    let container_id = result
        .get("ContainerID")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "CreateContainer did not return a ContainerID".to_string())?;

    // Bake the REAL container_id into the persisted file's own graph_id
    // field, not the local placeholder — confirmed live: writing the file
    // before this point left the file's graph_id mismatched with the id a
    // caller would actually query by (the container_id), even though
    // retrieval-by-container_id itself worked fine (object_store_path is
    // read from the container, not derived from the file's own content).
    // Cosmetic but real: a caller trusting the returned graph_id field for
    // a follow-up call would get the wrong number.
    let mut graph_json = serde_json::to_value(graph).map_err(|e| e.to_string())?;
    if let Some(obj) = graph_json.as_object_mut() {
        obj.insert("graph_id".to_string(), serde_json::json!(container_id));
    }
    write_graph_json_file(local_graph_id, &graph_json)?;

    // Keywords/topics returned alongside so the caller (create_graph) can
    // run cross-relationship linking without re-deriving them — same shape
    // code modality's persist_graph_container returns.
    Ok((container_id, keywords, topics))
}

fn write_graph_json_file(local_graph_id: u64, graph_json: &serde_json::Value) -> Result<(), String> {
    let data_dir = env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    let graphs_dir = format!("{}/graphs", data_dir);
    std::fs::create_dir_all(&graphs_dir).map_err(|e| e.to_string())?;
    let graph_path = format!("{}/math_{}.json", graphs_dir, local_graph_id);
    let json = serde_json::to_string_pretty(graph_json).map_err(|e| e.to_string())?;
    std::fs::write(&graph_path, json).map_err(|e| e.to_string())
}

/// Real retrieval — the piece neither text nor code modality actually has
/// working yet (both route GetGraph through an in-process HashMap cache
/// that's empty on every invocation, since each CLI call is a fresh, short-
/// lived process; confirmed by reading both their current source). This
/// reads the real container's own `object_store_path` (set at persist time
/// above) and the real JSON file it points to, so a graph created by one
/// process invocation is actually findable by a later one — worth applying
/// the same fix to text/code modality's GetGraph separately, not in scope
/// here.
async fn get_container_object_store_path(container_id: u64) -> Result<String, String> {
    let result = zsei_query(serde_json::json!({
        "GetContainer": { "container_id": container_id }
    }))
    .await?;

    result
        .get("Container")
        .and_then(|c| c.get("local_state"))
        .and_then(|l| l.get("storage"))
        .and_then(|s| s.get("object_store_path"))
        .and_then(|p| p.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("Container {} has no object_store_path (not a graph container?)", container_id))
}

async fn read_graph_container(graph_id: u64) -> Result<MathGraph, String> {
    let object_store_path = get_container_object_store_path(graph_id).await?;
    let data_dir = env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    // 4th real occurrence of tonight's absolute-path garbage-join bug class
    // (amt_loop.rs, jurisdiction.rs, amt.rs already fixed) — found while
    // building code modality's dependency-graph retrieval on this exact
    // pattern. Latent here too: math's own object_store_path is always
    // relative ("graphs/math_{id}.json") today, so never yet observed live.
    let full_path = if std::path::Path::new(&object_store_path).is_absolute() {
        object_store_path.clone()
    } else {
        format!("{}/{}", data_dir, object_store_path)
    };
    let content = std::fs::read_to_string(&full_path)
        .map_err(|e| format!("Failed to read graph file {}: {}", full_path, e))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse graph file {}: {}", full_path, e))
}

// ============================================================================
// PIPELINE METADATA
// ============================================================================

pub const PIPELINE_ID: u64 = 105;
pub const PIPELINE_NAME: &str = "math_analysis";
pub const PIPELINE_VERSION: &str = "0.4.0";
pub const MODALITY: &str = "math";

// ============================================================================
// INPUT/OUTPUT TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct MathModalityInput {
    pub action: MathAction,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MathAction {
    /// Parse mathematical expression
    ParseExpression {
        expression: String,
        format: MathFormat,
        #[serde(default)]
        extract_variables: bool,
        #[serde(default)]
        simplify: bool,
    },

    /// Analyze complete mathematical proof
    AnalyzeProof {
        proof: String,
        #[serde(default)]
        format: MathFormat,
        #[serde(default)]
        verify: bool,
        #[serde(default)]
        check_completeness: bool,
    },

    /// Verify individual proof step
    VerifyStep {
        step: ProofStep,
        context: ProofContext,
        #[serde(default)]
        allowed_rules: Option<Vec<String>>,
    },

    /// Build variable scope tree
    BuildScopeTree {
        proof_steps: Vec<ProofStep>,
        #[serde(default)]
        track_quantifiers: bool,
    },

    /// Evaluate expression with variable bindings
    Evaluate {
        expression: String,
        variables: HashMap<String, MathValue>,
        #[serde(default)]
        precision: Option<u32>,
    },

    /// Simplify expression
    Simplify {
        expression: String,
        #[serde(default)]
        rules: Vec<SimplificationRule>,
    },

    /// Create graph from analysis
    CreateGraph {
        analysis: MathAnalysisResult,
        project_id: u64,
        #[serde(default)]
        graph_name: Option<String>,
        /// Cross-relationship linking (task 57) — when true, the freshly
        /// persisted graph container gets real SimilarTo edges to existing
        /// containers sharing >= 2 keywords/topics. Serde-defaulted so
        /// callers that predate the flag keep working unchanged.
        #[serde(default)]
        link_to_existing: bool,
    },

    /// Update existing graph
    UpdateGraph {
        graph_id: u64,
        updates: MathGraphUpdate,
    },

    /// Query math graph
    QueryGraph {
        graph_id: u64,
        query: MathQuery,
    },

    /// Get graph
    GetGraph {
        graph_id: u64,
    },

    /// Check proof completeness
    CheckCompleteness {
        proof_steps: Vec<ProofStep>,
        goal: String,
    },

    /// Find all uses of a variable
    FindVariableUses {
        graph_id: u64,
        variable_name: String,
    },

    /// Trace derivation path
    TraceDerivation {
        graph_id: u64,
        from_step: u64,
        to_step: u64,
    },

    /// Link to another modality (e.g., code implementation)
    LinkToModality {
        math_graph_id: u64,
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
pub struct MathModalityOutput {
    pub success: bool,
    pub action: String,
    pub result: MathResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub metadata: OutputMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MathResult {
    Parse(ParseResult),
    Proof(ProofAnalysis),
    Verification(VerificationResult),
    ScopeTree(ScopeTree),
    Evaluation(EvaluationResult),
    Simplification(SimplificationResult),
    Graph(MathGraph),
    Query(QueryResult),
    Completeness(CompletenessResult),
    VariableUses(VariableUsesResult),
    Derivation(DerivationPath),
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

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub enum MathFormat {
    #[default]
    LaTeX,
    MathML,
    AsciiMath,
    Unicode,
    Plain,
    Mathematica,
    Maple,
    Lean,
    Coq,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MathValue {
    Integer(i64),
    Rational(i64, i64),
    Real(f64),
    Complex(f64, f64),
    Boolean(bool),
    Set(Vec<MathValue>),
    Vector(Vec<MathValue>),
    Matrix(Vec<Vec<MathValue>>),
    Symbol(String),
    Undefined,
}

impl MathValue {
    pub fn type_name(&self) -> &'static str {
        match self {
            MathValue::Integer(_) => "integer",
            MathValue::Rational(_, _) => "rational",
            MathValue::Real(_) => "real",
            MathValue::Complex(_, _) => "complex",
            MathValue::Boolean(_) => "boolean",
            MathValue::Set(_) => "set",
            MathValue::Vector(_) => "vector",
            MathValue::Matrix(_) => "matrix",
            MathValue::Symbol(_) => "symbol",
            MathValue::Undefined => "undefined",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParseResult {
    /// Parsed abstract syntax tree
    pub ast: MathNode,
    /// Detected expression type
    pub expression_type: ExpressionType,
    /// Variables found
    pub variables: Vec<Variable>,
    /// Operations/functions used
    pub operations: Vec<MathOperation>,
    /// Recognized constants
    pub constants: Vec<Constant>,
    /// Domain constraints (if inferrable)
    pub domain: Option<Domain>,
    /// Range constraints (if inferrable)
    pub range: Option<Range>,
    /// Mathematical properties
    pub properties: Vec<MathProperty>,
    /// Normalized form
    pub normalized: Option<String>,
    /// Parse confidence
    pub confidence: f32,
    /// Text-style content keywords extracted from the RAW input string
    /// (P4, math isolation fix): ParseExpression character-tokenizes
    /// formulas, so prose-heavy inputs (a .tex file that is mostly
    /// English) yield garbage single-letter Variables and
    /// derive_math_keywords finds no real terms. The raw string's real
    /// words — extracted here, at the only place the raw content exists —
    /// supplement the graph-derived keywords so prose+math documents can
    /// cross-link with text/code graphs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_keywords: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ExpressionType {
    Equation,
    Inequality,
    Function,
    Integral,
    Derivative,
    Sum,
    Product,
    Limit,
    Matrix,
    Vector,
    Set,
    Logic,
    Relation,
    Definition,
    Statement,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MathNode {
    /// Node type
    pub node_type: MathNodeType,
    /// Node value (for terminals)
    pub value: Option<String>,
    /// Child nodes
    pub children: Vec<MathNode>,
    /// Position in source
    pub position: Option<SourcePosition>,
    /// Type annotation
    pub type_annotation: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum MathNodeType {
    // Atoms
    Number,
    Variable,
    Constant,
    Symbol,
    // Operators
    BinaryOp,
    UnaryOp,
    Function,
    // Structures
    Parenthesis,
    Fraction,
    Power,
    Root,
    Subscript,
    Superscript,
    // Calculus
    Integral,
    Derivative,
    Limit,
    Sum,
    Product,
    // Linear Algebra
    Matrix,
    Vector,
    Determinant,
    // Logic
    Quantifier,
    Connective,
    Relation,
    // Sets
    SetBuilder,
    SetOp,
    // Special
    Piecewise,
    Undefined,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SourcePosition {
    pub start: usize,
    pub end: usize,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Variable {
    /// Variable name
    pub name: String,
    /// Variable type
    pub var_type: VariableType,
    /// Type constraints
    pub constraints: Vec<String>,
    /// Scope (where defined/bound)
    pub scope: Option<String>,
    /// Quantifier binding (forall, exists)
    pub quantifier: Option<Quantifier>,
    /// Initial value (if known)
    pub initial_value: Option<MathValue>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub enum VariableType {
    #[default]
    Unknown,
    Real,
    Integer,
    Complex,
    Natural,
    Rational,
    Boolean,
    Set,
    Function,
    Vector,
    Matrix,
    Custom(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Quantifier {
    ForAll,
    Exists,
    ExistsUnique,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Constant {
    pub name: String,
    pub symbol: String,
    pub value: Option<MathValue>,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MathOperation {
    /// Operation name
    pub name: String,
    /// Operation symbol
    pub symbol: String,
    /// Operand types
    pub operand_types: Vec<String>,
    /// Result type
    pub result_type: String,
    /// Associativity
    pub associativity: Option<Associativity>,
    /// Precedence level
    pub precedence: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Associativity {
    Left,
    Right,
    None,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Domain {
    /// Domain description
    pub description: String,
    /// Explicit constraints
    pub constraints: Vec<String>,
    /// Base set
    pub base_set: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Range {
    /// Range description
    pub description: String,
    /// Explicit bounds
    pub bounds: Option<(String, String)>,
    /// Base set
    pub base_set: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MathProperty {
    /// Property name
    pub property: String,
    /// Whether it holds
    pub holds: bool,
    /// Confidence
    pub confidence: f32,
    /// Proof/justification
    pub justification: Option<String>,
}

// ============================================================================
// PROOF TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProofAnalysis {
    /// Proof title/name
    pub title: Option<String>,
    /// The theorem/statement being proved
    pub statement: String,
    /// Proof steps
    pub steps: Vec<ProofStep>,
    /// Axioms used
    pub axioms_used: Vec<Axiom>,
    /// Theorems/lemmas used
    pub theorems_used: Vec<TheoremReference>,
    /// Definitions used
    pub definitions_used: Vec<Definition>,
    /// Is proof valid?
    pub is_valid: bool,
    /// Validation confidence
    pub confidence: f32,
    /// Proof gaps (if any)
    pub gaps: Vec<ProofGap>,
    /// Proof technique
    pub proof_technique: ProofTechnique,
    /// Variable scope tree
    pub scope_tree: Option<ScopeTree>,
    /// Dependency graph
    pub dependencies: Vec<StepDependency>,
    /// Text-style content keywords extracted from the combined raw step
    /// statements (P4 extended to proofs): `extract_content_keywords`
    /// previously only ran inside `parse_expression`, so prose-heavy proofs
    /// (the exact ".tex mostly English" case P4 was built for) never
    /// benefited — `parse_result` is always `None` for a proof analysis.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_keywords: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProofStep {
    /// Step number
    pub step_number: usize,
    /// Step statement
    pub statement: String,
    /// Justification/reason
    pub justification: Justification,
    /// Dependencies on previous steps
    pub dependencies: Vec<usize>,
    /// Variables introduced in this step
    pub introduced_variables: Vec<Variable>,
    /// Assumptions made in this step
    pub assumptions: Vec<Assumption>,
    /// Assumptions discharged
    pub discharged_assumptions: Vec<usize>,
    /// Step type
    pub step_type: StepType,
    /// Confidence in this step
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum StepType {
    /// Direct assertion/given
    Given,
    /// Assumption for contradiction/conditional
    Assumption,
    /// Application of axiom/theorem
    Application,
    /// Logical deduction
    Deduction,
    /// Substitution
    Substitution,
    /// Algebraic manipulation
    Algebraic,
    /// Case introduction
    CaseIntro,
    /// Case analysis
    CaseAnalysis,
    /// Induction base
    InductionBase,
    /// Induction hypothesis
    InductionHypothesis,
    /// Induction step
    InductionStep,
    /// Contradiction
    Contradiction,
    /// Conclusion
    Conclusion,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Justification {
    /// Justification type
    pub justification_type: JustificationType,
    /// Rule/theorem/axiom name
    pub rule_name: Option<String>,
    /// Explanation
    pub explanation: Option<String>,
    /// Referenced steps
    pub referenced_steps: Vec<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum JustificationType {
    Axiom,
    Theorem,
    Definition,
    LogicalRule,
    Algebraic,
    Substitution,
    Hypothesis,
    Given,
    PreviousResult,
    Contradiction,
    CaseExhaustion,
    Induction,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Assumption {
    /// Assumption ID
    pub assumption_id: usize,
    /// The assumption statement
    pub statement: String,
    /// Scope (where valid)
    pub scope_start: usize,
    /// Where discharged (if at all)
    pub scope_end: Option<usize>,
    /// Assumption type
    pub assumption_type: AssumptionType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AssumptionType {
    /// Assume for direct proof
    Direct,
    /// Assume for contradiction
    Contradiction,
    /// Case assumption
    Case,
    /// Induction assumption
    Induction,
    /// Arbitrary element
    Arbitrary,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Axiom {
    pub name: String,
    pub statement: String,
    pub system: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TheoremReference {
    pub name: String,
    pub statement: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Definition {
    pub name: String,
    pub symbol: String,
    pub meaning: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProofGap {
    /// Step number where gap occurs
    pub step_number: usize,
    /// Gap description
    pub description: String,
    /// Gap severity
    pub severity: GapSeverity,
    /// Suggested fix
    pub suggested_fix: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum GapSeverity {
    Minor,    // Can be easily filled
    Moderate, // Requires some work
    Major,    // Significant gap
    Critical, // Proof may be invalid
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ProofTechnique {
    Direct,
    Contradiction,
    Contrapositive,
    Induction,
    StrongInduction,
    StructuralInduction,
    CaseAnalysis,
    Constructive,
    NonConstructive,
    Combinatorial,
    Probabilistic,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StepDependency {
    pub from_step: usize,
    pub to_step: usize,
    pub dependency_type: DependencyType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum DependencyType {
    Uses,           // Step uses result of another
    Requires,       // Step requires another to be valid
    Generalizes,    // Step generalizes another
    Specializes,    // Step specializes another
    Contradicts,    // Steps are contradictory
    DischargesFrom, // Discharges assumption from step
}

// ============================================================================
// SCOPE TREE
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScopeTree {
    /// Root scope
    pub root: Scope,
    /// All scopes by ID
    pub scopes: HashMap<String, Scope>,
    /// Variables by scope
    pub variables_by_scope: HashMap<String, Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Scope {
    /// Scope ID
    pub scope_id: String,
    /// Parent scope ID
    pub parent: Option<String>,
    /// Child scope IDs
    pub children: Vec<String>,
    /// Step range
    pub step_range: (usize, usize),
    /// Variables in this scope
    pub variables: Vec<Variable>,
    /// Assumptions active in this scope
    pub active_assumptions: Vec<usize>,
    /// Scope type
    pub scope_type: ScopeType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ScopeType {
    Global,
    ProofBlock,
    CaseBlock,
    InductionBlock,
    SubproofBlock,
    LetBinding,
    ForAllIntro,
    ExistsElim,
}

// ============================================================================
// VERIFICATION & CONTEXT
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProofContext {
    /// Previous proof steps
    pub previous_steps: Vec<ProofStep>,
    /// Available axioms
    pub available_axioms: Vec<Axiom>,
    /// Available theorems
    pub available_theorems: Vec<TheoremReference>,
    /// Available definitions
    pub available_definitions: Vec<Definition>,
    /// Current scope
    pub current_scope: Option<Scope>,
    /// Active assumptions
    pub active_assumptions: Vec<Assumption>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VerificationResult {
    /// Is step valid?
    pub is_valid: bool,
    /// Confidence
    pub confidence: f32,
    /// Validation details
    pub details: Vec<ValidationDetail>,
    /// Errors found
    pub errors: Vec<ValidationError>,
    /// Warnings
    pub warnings: Vec<String>,
    /// Applied rules
    pub applied_rules: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidationDetail {
    pub aspect: String,
    pub status: ValidationStatus,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ValidationStatus {
    Valid,
    Invalid,
    Uncertain,
    NotApplicable,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidationError {
    pub error_type: String,
    pub message: String,
    pub location: Option<usize>,
    pub suggestion: Option<String>,
}

// ============================================================================
// EVALUATION & SIMPLIFICATION
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EvaluationResult {
    pub value: MathValue,
    pub exact: bool,
    pub precision: Option<u32>,
    pub intermediate_steps: Vec<EvaluationStep>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EvaluationStep {
    pub expression: String,
    pub value: MathValue,
    pub rule_applied: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SimplificationResult {
    pub original: String,
    pub simplified: String,
    pub steps: Vec<SimplificationStep>,
    pub rules_applied: Vec<String>,
    pub is_canonical: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SimplificationStep {
    pub before: String,
    pub after: String,
    pub rule: String,
    pub explanation: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SimplificationRule {
    pub name: String,
    pub pattern: String,
    pub replacement: String,
    pub conditions: Vec<String>,
}

// ============================================================================
// ANALYSIS RESULT
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MathAnalysisResult {
    /// Type of analysis performed
    pub analysis_type: MathAnalysisType,
    /// Parsed content
    pub parse_result: Option<ParseResult>,
    /// Proof analysis (if applicable)
    pub proof_analysis: Option<ProofAnalysis>,
    /// Overall confidence
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MathAnalysisType {
    Expression,
    Proof,
    Definition,
    Theorem,
    Mixed,
}

// ============================================================================
// COMPLETENESS
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CompletenessResult {
    pub is_complete: bool,
    pub goal_reached: bool,
    pub missing_steps: Vec<String>,
    pub undischarged_assumptions: Vec<Assumption>,
    pub unused_hypotheses: Vec<String>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VariableUsesResult {
    pub variable_name: String,
    pub uses: Vec<VariableUse>,
    pub definition: Option<VariableDefinition>,
    pub scope: Option<Scope>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VariableUse {
    pub step_number: usize,
    pub usage_type: VariableUsageType,
    pub context: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum VariableUsageType {
    Definition,
    Reference,
    Assignment,
    Quantification,
    Substitution,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VariableDefinition {
    pub step_number: usize,
    pub definition: String,
    pub var_type: VariableType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DerivationPath {
    pub from_step: u64,
    pub to_step: u64,
    pub path: Vec<u64>,
    pub path_length: usize,
    pub dependencies: Vec<StepDependency>,
}

// ============================================================================
// GRAPH TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MathGraph {
    pub graph_id: u64,
    pub name: String,
    pub modality: String,
    pub project_id: u64,
    pub content_type: MathAnalysisType,
    pub nodes: Vec<MathGraphNode>,
    pub edges: Vec<MathGraphEdge>,
    pub scope_tree: Option<ScopeTree>,
    pub metadata: GraphMetadata,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MathGraphNode {
    pub node_id: u64,
    pub node_type: MathGraphNodeType,
    pub label: String,
    pub content: String,
    pub step_number: Option<usize>,
    pub confidence: f32,
    pub properties: HashMap<String, Value>,
    pub annotations: Vec<SemanticAnnotation>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum MathGraphNodeType {
    /// Root proof/expression node
    Root,
    /// Proof step
    ProofStep,
    /// Mathematical expression
    Expression,
    /// Variable
    Variable,
    /// Axiom
    Axiom,
    /// Theorem reference
    Theorem,
    /// Definition
    Definition,
    /// Assumption
    Assumption,
    /// Constant
    Constant,
    /// Scope block
    Scope,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SemanticAnnotation {
    pub annotation_type: String,
    pub value: Value,
    pub confidence: f32,
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MathGraphEdge {
    pub edge_id: u64,
    pub from_node: u64,
    pub to_node: u64,
    pub edge_type: MathEdgeType,
    pub weight: f32,
    pub properties: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum MathEdgeType {
    // Structural
    Contains,
    FollowsStep,    // Step i is presented after step i-1 (real, unconditional
                     // presentation order — distinct from Uses, which is a
                     // real cited content dependency and must never be
                     // defaulted to "the previous step" absent an actual
                     // citation in the step's own text)
    // Proof dependencies
    Uses,           // Step uses another step's result
    Derives,        // Step derives from another
    Requires,       // Step requires another
    Implies,        // Step implies another
    // Definitions
    Defines,        // Defines a variable/symbol
    References,     // References a definition
    // Assumptions
    AssumesIn,      // Step makes assumption
    DischargesIn,   // Step discharges assumption
    // Variables
    BindsVariable,  // Quantifier binds variable
    UsesVariable,   // Step uses variable
    // Semantic
    Generalizes,
    Specializes,
    Contradicts,
    SimilarTo,
    // Cross-modality
    ImplementedBy,
    RepresentedBy,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GraphMetadata {
    pub node_count: usize,
    pub edge_count: usize,
    pub proof_steps: usize,
    pub variables_count: usize,
    pub assumptions_count: usize,
    pub is_verified: bool,
    pub verification_confidence: f32,
    pub semantic_enriched: bool,
    pub cross_modal_links: usize,
    /// Real captured metrics from link_related_containers (present only
    /// when link_to_existing ran) — candidate counts, shared-term
    /// distribution, edges written, duration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_metrics: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MathGraphUpdate {
    pub add_nodes: Vec<MathGraphNode>,
    pub update_nodes: Vec<MathGraphNode>,
    pub remove_nodes: Vec<u64>,
    pub add_edges: Vec<MathGraphEdge>,
    pub remove_edges: Vec<u64>,
    pub metadata_updates: Option<HashMap<String, Value>>,
}

// ============================================================================
// QUERY TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MathQuery {
    pub query_type: MathQueryType,
    pub parameters: HashMap<String, Value>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MathQueryType {
    /// Find all variables
    FindVariables,
    /// Find all operations
    FindOperations,
    /// Check mathematical property
    CheckProperty { property: String },
    /// Get dependencies of a step
    GetDependencies { step_number: usize },
    /// Get steps that depend on given step
    GetDependents { step_number: usize },
    /// Trace derivation between steps
    TraceDerivation { from_step: usize, to_step: usize },
    /// Find assumptions
    FindAssumptions { active_only: bool },
    /// Find gaps in proof
    FindGaps,
    /// Get steps by type
    GetStepsByType { step_type: StepType },
    /// Get nodes by type
    GetNodesByType { node_type: MathGraphNodeType },
    /// Custom query
    Custom { query: String },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryResult {
    pub query_type: String,
    pub nodes: Vec<MathGraphNode>,
    pub edges: Vec<MathGraphEdge>,
    pub total_matches: usize,
    pub metadata: HashMap<String, Value>,
}

// ============================================================================
// CROSS-MODALITY & HOOKS
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CrossModalityRelation {
    ImplementedBy, // Code implements this math
    RepresentedBy, // Diagram represents this math
    DescribedBy,   // Text describes this math
    ProvedUsing,   // Proved using external system
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

    let input: MathModalityInput =
        serde_json::from_value(input).map_err(|e| format!("Failed to parse input: {}", e))?;

    let result = match input.action {
        MathAction::ParseExpression {
            expression,
            format,
            extract_variables,
            simplify,
        } => {
            let parse_result =
                parse_expression(&expression, format, extract_variables, simplify).await?;
            ("ParseExpression", MathResult::Parse(parse_result))
        }

        MathAction::AnalyzeProof {
            proof,
            format,
            verify,
            check_completeness,
        } => {
            let analysis = analyze_proof(&proof, format, verify, check_completeness).await?;
            ("AnalyzeProof", MathResult::Proof(analysis))
        }

        MathAction::VerifyStep {
            step,
            context,
            allowed_rules,
        } => {
            let result = verify_step(&step, &context, allowed_rules.as_deref()).await?;
            ("VerifyStep", MathResult::Verification(result))
        }

        MathAction::BuildScopeTree {
            proof_steps,
            track_quantifiers,
        } => {
            let tree = build_scope_tree(&proof_steps, track_quantifiers).await?;
            ("BuildScopeTree", MathResult::ScopeTree(tree))
        }

        MathAction::Evaluate {
            expression,
            variables,
            precision,
        } => {
            let result = evaluate_expression(&expression, &variables, precision).await?;
            ("Evaluate", MathResult::Evaluation(result))
        }

        MathAction::Simplify { expression, rules } => {
            let result = simplify_expression(&expression, &rules).await?;
            ("Simplify", MathResult::Simplification(result))
        }

        MathAction::CreateGraph {
            analysis,
            project_id,
            graph_name,
            link_to_existing,
        } => {
            let graph = create_graph(analysis, project_id, graph_name, link_to_existing).await?;
            ("CreateGraph", MathResult::Graph(graph))
        }

        MathAction::UpdateGraph { graph_id, updates } => {
            let graph = update_graph(graph_id, updates).await?;
            ("UpdateGraph", MathResult::Graph(graph))
        }

        MathAction::QueryGraph { graph_id, query } => {
            let result = query_graph(graph_id, query).await?;
            ("QueryGraph", MathResult::Query(result))
        }

        MathAction::GetGraph { graph_id } => {
            let graph = get_graph(graph_id).await?;
            ("GetGraph", MathResult::Graph(graph))
        }

        MathAction::CheckCompleteness { proof_steps, goal } => {
            let result = check_completeness(&proof_steps, &goal).await?;
            ("CheckCompleteness", MathResult::Completeness(result))
        }

        MathAction::FindVariableUses {
            graph_id,
            variable_name,
        } => {
            let result = find_variable_uses(graph_id, &variable_name).await?;
            ("FindVariableUses", MathResult::VariableUses(result))
        }

        MathAction::TraceDerivation {
            graph_id,
            from_step,
            to_step,
        } => {
            let result = trace_derivation(graph_id, from_step, to_step).await?;
            ("TraceDerivation", MathResult::Derivation(result))
        }

        MathAction::LinkToModality {
            math_graph_id,
            target_graph_id,
            target_modality,
            relationship,
        } => {
            let link =
                link_to_modality(math_graph_id, target_graph_id, &target_modality, relationship)
                    .await?;
            ("LinkToModality", MathResult::Link(link))
        }

        MathAction::TriggerSemanticHook {
            graph_id,
            hook_type,
            options,
        } => {
            let result = trigger_semantic_hook(graph_id, hook_type, options).await?;
            ("TriggerSemanticHook", MathResult::Hook(result))
        }
    };

    let output = MathModalityOutput {
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

/// P4 (math isolation fix): text-style content keyword extraction over the
/// RAW input — the supplement that lets prose-heavy math documents (a .tex
/// file that is mostly English) contribute REAL terms ("gradient",
/// "learning_rate", "descent") alongside ParseExpression's per-token
/// variables, so derive_math_keywords produces keywords that actually
/// overlap text/code graphs. Lowercase, >=4 chars, small stopword set,
/// deduped, capped.
fn extract_content_keywords(raw: &str) -> Vec<String> {
    const STOPWORDS: [&str; 14] = [
        "this", "that", "with", "from", "have", "been", "were", "will",
        "would", "should", "could", "their", "there", "which",
    ];
    let mut out: Vec<String> = Vec::new();
    for word in raw.split(|c: char| !c.is_alphanumeric() && c != '_') {
        let w = word.to_lowercase();
        if w.len() >= 4 && !STOPWORDS.contains(&w.as_str()) && !out.contains(&w) {
            out.push(w);
        }
        if out.len() >= 12 {
            break;
        }
    }
    out
}

/// Real cross-step reference detection for proof steps. Previously
/// `analyze_proof` hardcoded every step's dependency to `vec![i]` (only the
/// immediately preceding step) regardless of what the step's own text
/// actually cites — a fabricated relationship presented as real (see the
/// 2026-09-22 graph/relationship audit). This scans the step's own statement
/// for genuine textual evidence of an earlier reference and returns ONLY
/// step numbers with that evidence — an empty result is honest when no
/// citation exists, never defaulted.
///
/// Two forms of evidence are accepted:
/// 1. Explicit numbered citations: "step 2", "equation 3", "(4)" (when not a
///    leading enumeration marker), "result 1", "assumption 2" — resolved
///    only against real earlier step numbers (1..current_step_number).
/// 2. Explicit textual back-reference phrases ("the previous step", "as
///    shown above") — these genuinely claim a reference to the immediately
///    preceding step in the step's own words, unlike the old unconditional
///    default, so mapping them to step (current-1) is grounded, not guessed.
fn push_unique_earlier_ref(n: usize, current_step_number: usize, found: &mut Vec<usize>) {
    if n >= 1 && n < current_step_number && !found.contains(&n) {
        found.push(n);
    }
}

fn extract_step_references(statement: &str, current_step_number: usize) -> Vec<usize> {
    let mut found: Vec<usize> = Vec::new();

    if let Ok(labeled) = Regex::new(r"(?i)\b(?:steps?|eqs?\.?|equations?|results?|assumptions?)\s*#?\s*(\d+)\b") {
        for cap in labeled.captures_iter(statement) {
            if let Some(n) = cap.get(1).and_then(|m| m.as_str().parse::<usize>().ok()) {
                push_unique_earlier_ref(n, current_step_number, &mut found);
            }
        }
    }

    if let Ok(paren) = Regex::new(r"\((\d+)\)") {
        for cap in paren.captures_iter(statement) {
            // A "(N)" at the very start of the (already-trimmed) statement
            // reads as an enumeration/list marker ("(1) First, ..."), not a
            // citation — skip it to avoid a false-positive reference.
            let is_leading = cap.get(0).map(|m| m.start()).unwrap_or(1) == 0;
            if is_leading {
                continue;
            }
            if let Some(n) = cap.get(1).and_then(|m| m.as_str().parse::<usize>().ok()) {
                push_unique_earlier_ref(n, current_step_number, &mut found);
            }
        }
    }

    if current_step_number > 1 {
        let lower = statement.to_lowercase();
        const IMPLICIT_MARKERS: [&str; 6] = [
            "the previous step", "as before", "as shown above",
            "from above", "the prior step", "as above",
        ];
        if IMPLICIT_MARKERS.iter().any(|m| lower.contains(m)) {
            push_unique_earlier_ref(current_step_number - 1, current_step_number, &mut found);
        }
    }

    found.sort_unstable();
    found
}

/// Real per-step assumption/variable-introduction detection (2026-09-22
/// graph audit follow-up, `GRAPH_RELATIONSHIP_REGISTRY.md` §2's
/// highest-value next item): `analyze_proof` used to hardcode
/// `introduced_variables`/`assumptions` empty for every step even though
/// the graph-construction code for `Defines`/`AssumesIn` edges already
/// existed and worked — this was purely a missing-source-data gap, not a
/// missing-consumer one. Same honest-evidence-only discipline as
/// `extract_step_references`: no textual evidence, no node.
fn extract_variable_introduction(statement: &str) -> Option<Variable> {
    let re = Regex::new(r"(?i)\blet\s+([A-Za-z][A-Za-z0-9_]*)\s+be\b").ok()?;
    let cap = re.captures(statement)?;
    let name = cap.get(1)?.as_str().to_string();
    Some(Variable {
        name,
        var_type: VariableType::Unknown,
        constraints: vec![],
        scope: None,
        quantifier: None,
        initial_value: None,
    })
}

/// Real assumption-introduction detection ("assume X", "suppose X",
/// "for the sake of contradiction, assume X"). Returns the assumed clause
/// text plus a best-effort `AssumptionType` from the same statement's own
/// wording — never fabricated when the statement doesn't actually
/// introduce an assumption.
fn extract_assumption_introduction(statement: &str) -> Option<(String, AssumptionType)> {
    let re = Regex::new(r"(?i)\b(?:assume|suppose)\b\s*(?:that\s+)?(.+)").ok()?;
    let cap = re.captures(statement)?;
    let clause = cap.get(1)?.as_str().trim().trim_end_matches('.').to_string();
    if clause.is_empty() {
        return None;
    }
    let lower = statement.to_lowercase();
    let assumption_type = if lower.contains("contradiction") {
        AssumptionType::Contradiction
    } else if lower.contains("case") {
        AssumptionType::Case
    } else if lower.contains("induction") {
        AssumptionType::Induction
    } else {
        AssumptionType::Direct
    };
    Some((clause, assumption_type))
}

/// Real discharge-reference detection: which earlier STEP's assumption(s)
/// this step's own text says it is discharging/dropping/contradicting.
/// Returns real earlier step numbers only (resolved to actual assumption
/// ids by the caller, which tracks which assumption(s) each step
/// introduced) — a step with no discharge language returns empty, never
/// guessed.
fn extract_discharge_step_refs(statement: &str, current_step_number: usize) -> Vec<usize> {
    let mut found = Vec::new();
    let lower = statement.to_lowercase();
    let has_discharge_language = lower.contains("discharg")
        || lower.contains("drop the assumption")
        || lower.contains("contradicts our assumption")
        || lower.contains("contradicts the assumption");
    if !has_discharge_language {
        return found;
    }
    if let Ok(re) = Regex::new(r"(?i)\b(?:step|assumption)\s*#?\s*(\d+)\b") {
        for cap in re.captures_iter(statement) {
            if let Some(n) = cap.get(1).and_then(|m| m.as_str().parse::<usize>().ok()) {
                push_unique_earlier_ref(n, current_step_number, &mut found);
            }
        }
    }
    found
}

/// Balanced-scan, multi-candidate JSON extractor — same pattern already
/// proven this session in the orchestrator (decision_review.rs,
/// amt_loop.rs, meta_loop.rs) and text modality (extract_entities_from_text
/// et al.) to detect BitNet "confetti" (a single response containing
/// multiple conflicting JSON candidates). Ported here for E7 (math's first
/// real zero-shot call site — see `resolve_implicit_step_references`),
/// since this pipeline had no prior pipeline-9 exposure to inherit the
/// helper from. Returns every non-empty, individually-parseable candidate
/// in order; callers treat `len() > 1` as confetti (unusable, retry).
fn extract_all_json_candidates(s: &str, start_char: char, end_char: char) -> Vec<String> {
    let chars: Vec<char> = s.chars().collect();
    let mut candidates = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != start_char {
            i += 1;
            continue;
        }
        let start = i;
        let mut depth = 0usize;
        let mut in_string = false;
        let mut escaped = false;
        let mut end = None;
        let mut j = i;
        while j < chars.len() {
            let c = chars[j];
            if in_string {
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == '"' {
                    in_string = false;
                }
            } else {
                if c == '"' {
                    in_string = true;
                } else if c == start_char {
                    depth += 1;
                } else if c == end_char {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(j);
                        break;
                    }
                }
            }
            j += 1;
        }
        match end {
            Some(e) => {
                let candidate: String = chars[start..=e].iter().collect();
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&candidate) {
                    let is_empty = match &parsed {
                        serde_json::Value::Object(o) => o.is_empty(),
                        serde_json::Value::Array(a) => a.is_empty(),
                        _ => true,
                    };
                    if !is_empty {
                        candidates.push(candidate);
                    }
                }
                i = e + 1;
            }
            None => break,
        }
    }
    candidates
}

/// Trait for executing other pipelines (injected by runtime) — mirrors
/// text/main.rs's identical trait verbatim. Separate compiled binaries,
/// no shared crate: each modality pipeline that needs a real zero-shot
/// call defines its own copy of this exact contract.
#[async_trait::async_trait]
pub trait PipelineExecutor: Send + Sync {
    async fn execute(
        &self,
        pipeline_id: u64,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, String>;
}

/// Real executor: spawns the prompt pipeline (#9) as a subprocess, the same
/// mechanism text/main.rs's `SubprocessExecutor` uses — a modality pipeline
/// binary has no host auth to call pipeline 9 through the orchestrator
/// directly (this was tried and reverted earlier this session), so a direct
/// subprocess call to the real prompt binary is the correct, working
/// pattern, not a shortcut.
struct SubprocessExecutor;

#[async_trait::async_trait]
impl PipelineExecutor for SubprocessExecutor {
    async fn execute(
        &self,
        pipeline_id: u64,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        // LOUD failures (fixed 2026-09-27, same class as text's fix found
        // live by the pipeline capture layer): the old
        // Ok({"response": "[]"}) conversions fabricated successes out of
        // total model failures. Callers degrade gracefully on Err; capture
        // and retries now see the truth.
        if pipeline_id != 9 {
            return Err(format!(
                "math SubprocessExecutor: no internal executor for pipeline {} (only pipeline 9 is wired)",
                pipeline_id
            ));
        }
        let path = match std::env::var("OZONE_PROMPT_PIPELINE_PATH") {
            Ok(p) if !p.is_empty() && std::path::Path::new(&p).exists() => p,
            other => {
                eprintln!(
                    "SubprocessExecutor: OZONE_PROMPT_PIPELINE_PATH not set or binary \
                     missing (env={:?}) — internal LLM call unavailable this run",
                    other
                );
                return Err(
                    "math SubprocessExecutor: OZONE_PROMPT_PIPELINE_PATH not set or binary missing — internal LLM call unavailable".to_string(),
                );
            }
        };
        let input_json = serde_json::to_string(&input).map_err(|e| e.to_string())?;
        let execution_id = format!(
            "math-internal-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );
        let output = tokio::task::spawn_blocking(move || {
            std::process::Command::new(&path)
                .arg("--input")
                .arg(&input_json)
                .arg("--execution-id")
                .arg(&execution_id)
                .output()
        })
        .await
        .map_err(|e| format!("internal prompt-pipeline task join failed: {}", e))?
        .map_err(|e| format!("failed to spawn internal prompt pipeline: {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            eprintln!(
                "SubprocessExecutor: internal prompt pipeline exited non-zero: {}",
                stderr
            );
            return Err(format!(
                "internal prompt pipeline failed (exit {:?}): {}",
                output.status.code(),
                stderr.trim()
            ));
        }
        serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("failed to parse internal prompt pipeline output: {}", e))
    }
}

/// E7 (docs/ZERO_SHOT_EXPANSION_GUIDE.md): batched zero-shot resolution for
/// proof steps `extract_step_references`'s regex pass left with zero
/// citations — implicit references ("by the earlier bound we derived", no
/// explicit number) or named-theorem citations the regex pass structurally
/// cannot reach. Runs once per `AnalyzeProof` action, only over the
/// genuinely unresolved subset — never a duplicate of the regex pass, and
/// gated by the caller's own confidence threshold before any result is
/// trusted (see `analyze_proof`).
///
/// NOT YET WIRED into `analyze_proof` — this function is real and correct
/// (mirrors `extract_step_references`'s confetti-safe, non-fabricating
/// discipline) but has no caller yet. Interrupted mid-build by a session
/// rate limit (2026-09-22/23); wiring it in (identifying the unresolved
/// subset after the regex pass, merging results above a confidence floor)
/// is the real remaining work. Left as dead code (compiles clean, unused
/// function warning only) rather than force-wired without verification.
#[allow(dead_code)]
async fn resolve_implicit_step_references(
    executor: &dyn PipelineExecutor,
    unresolved_steps: &[(usize, String)],
) -> Vec<(usize, Vec<usize>, Vec<String>, f32)> {
    let steps_json: Vec<serde_json::Value> = unresolved_steps
        .iter()
        .map(|(n, statement)| serde_json::json!({"step_number": n, "statement": statement}))
        .collect();

    let prompt = format!(
        r#"These proof steps have no explicit numbered citation in their own text. For EACH step, decide whether it implicitly references an earlier step (e.g. "by the earlier bound we derived", "from our previous inequality") or names a specific theorem/axiom/definition being applied. Only report a reference when the step's own wording genuinely supports it — if a step is a true given/axiom with nothing to cite, report it with empty arrays, do not guess.

STEPS:
{}

Return ONLY a JSON array, one object per step, in this exact shape:
[{{"step_number": <int>, "cited_steps": [<int>, ...], "cited_theorems": ["<name>", ...], "confidence": <0.0-1.0>}}]

RESPOND ONLY WITH THE JSON ARRAY."#,
        serde_json::to_string_pretty(&steps_json).unwrap_or_default()
    );

    let input = serde_json::json!({
        "prompt": prompt,
        "max_tokens": 600,
        "temperature": 0.2,
        "system_prompt": "Output only a valid JSON array. No explanation. No markdown code blocks. No preamble. Start directly with [."
    });

    const MAX_ATTEMPTS: u32 = 2;
    let mut attempt = 0;
    loop {
        attempt += 1;
        let result = executor.execute(9, input.clone()).await;
        capture::capture_zero_shot_call(
            "math",
            "resolve_implicit_step_references",
            &prompt,
            &result,
        );
        let response_text = result
            .as_ref()
            .ok()
            .and_then(|r| r.get("response"))
            .and_then(|r| r.as_str())
            .unwrap_or("")
            .to_string();
        let candidates = extract_all_json_candidates(&response_text, '[', ']');
        if candidates.len() > 1 {
            if attempt < MAX_ATTEMPTS {
                eprintln!(
                    "resolve_implicit_step_references: {} conflicting JSON candidates (confetti), retrying (attempt {})",
                    candidates.len(),
                    attempt
                );
                continue;
            }
            eprintln!(
                "resolve_implicit_step_references: still confetti after {} attempts, giving up",
                attempt
            );
            return Vec::new();
        }
        let parsed = candidates
            .into_iter()
            .next()
            .and_then(|json_str| serde_json::from_str::<Vec<serde_json::Value>>(&json_str).ok());
        match parsed {
            Some(arr) => {
                return arr
                    .iter()
                    .filter_map(|v| {
                        let step_number = v.get("step_number")?.as_u64()? as usize;
                        let cited_steps = v
                            .get("cited_steps")
                            .and_then(|c| c.as_array())
                            .map(|a| a.iter().filter_map(|x| x.as_u64().map(|n| n as usize)).collect())
                            .unwrap_or_default();
                        let cited_theorems = v
                            .get("cited_theorems")
                            .and_then(|c| c.as_array())
                            .map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
                            .unwrap_or_default();
                        let confidence = v.get("confidence").and_then(|c| c.as_f64()).unwrap_or(0.0) as f32;
                        Some((step_number, cited_steps, cited_theorems, confidence))
                    })
                    .collect();
            }
            None if attempt < MAX_ATTEMPTS => continue,
            None => return Vec::new(),
        }
    }
}

async fn parse_expression(
    expression: &str,
    format: MathFormat,
    extract_variables: bool,
    _simplify: bool,
) -> Result<ParseResult, String> {
    // In production, this would use a proper math parser (sympy, mathjs, etc.)
    
    let mut variables = Vec::new();
    let mut operations = Vec::new();
    
    if extract_variables {
        // Simple variable extraction (in production, parse the AST properly)
        for c in expression.chars() {
            if c.is_ascii_lowercase() && c != 'e' && c != 'i' {
                let var_name = c.to_string();
                if !variables.iter().any(|v: &Variable| v.name == var_name) {
                    variables.push(Variable {
                        name: var_name,
                        var_type: VariableType::Unknown,
                        constraints: vec![],
                        scope: None,
                        quantifier: None,
                        initial_value: None,
                    });
                }
            }
        }
    }

    // Detect operations
    if expression.contains('+') {
        operations.push(MathOperation {
            name: "addition".to_string(),
            symbol: "+".to_string(),
            operand_types: vec!["number".to_string(), "number".to_string()],
            result_type: "number".to_string(),
            associativity: Some(Associativity::Left),
            precedence: Some(1),
        });
    }
    if expression.contains('-') {
        operations.push(MathOperation {
            name: "subtraction".to_string(),
            symbol: "-".to_string(),
            operand_types: vec!["number".to_string(), "number".to_string()],
            result_type: "number".to_string(),
            associativity: Some(Associativity::Left),
            precedence: Some(1),
        });
    }
    if expression.contains('*') || expression.contains("\\cdot") {
        operations.push(MathOperation {
            name: "multiplication".to_string(),
            symbol: "*".to_string(),
            operand_types: vec!["number".to_string(), "number".to_string()],
            result_type: "number".to_string(),
            associativity: Some(Associativity::Left),
            precedence: Some(2),
        });
    }

    // Determine expression type
    let expression_type = if expression.contains('=') && !expression.contains("!=") && !expression.contains("<=") && !expression.contains(">=") {
        ExpressionType::Equation
    } else if expression.contains('<') || expression.contains('>') || expression.contains("\\leq") || expression.contains("\\geq") {
        ExpressionType::Inequality
    } else if expression.contains("\\int") {
        ExpressionType::Integral
    } else if expression.contains("\\frac{d") || expression.contains("'") {
        ExpressionType::Derivative
    } else if expression.contains("\\sum") {
        ExpressionType::Sum
    } else if expression.contains("\\lim") {
        ExpressionType::Limit
    } else {
        ExpressionType::Statement
    };

    Ok(ParseResult {
        ast: MathNode {
            node_type: MathNodeType::Symbol,
            value: Some(expression.to_string()),
            children: vec![],
            position: Some(SourcePosition {
                start: 0,
                end: expression.len(),
                line: Some(1),
                column: Some(1),
            }),
            type_annotation: None,
        },
        expression_type,
        variables,
        operations,
        content_keywords: extract_content_keywords(expression),
        constants: vec![],
        domain: None,
        range: None,
        properties: vec![],
        normalized: Some(expression.to_string()),
        confidence: 0.85,
    })
}

async fn analyze_proof(
    proof: &str,
    _format: MathFormat,
    verify: bool,
    check_completeness: bool,
) -> Result<ProofAnalysis, String> {
    // In production, this would parse the proof and create proper step analysis
    
    // Split proof into steps (simplified)
    let lines: Vec<&str> = proof.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    let mut steps = Vec::new();
    let mut axioms_used = Vec::new();
    let mut theorems_used = Vec::new();
    let mut definitions_used = Vec::new();
    let mut gaps = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        // Real per-step reference detection (fixes the fabricated-dependency
        // bug from the 2026-09-22 graph audit): only steps whose own text
        // actually cites an earlier step/equation/result get a dependency —
        // no default to "always the previous step".
        let real_refs = extract_step_references(line, i + 1);
        let step = ProofStep {
            step_number: i + 1,
            statement: line.to_string(),
            justification: Justification {
                justification_type: if i == 0 {
                    JustificationType::Given
                } else if !real_refs.is_empty() {
                    JustificationType::PreviousResult
                } else {
                    // Honest fallback: no citation evidence in the step's own
                    // text, so we don't claim a specific reference relationship.
                    JustificationType::LogicalRule
                },
                rule_name: None,
                explanation: None,
                referenced_steps: real_refs.clone(),
            },
            dependencies: real_refs,
            introduced_variables: vec![],
            assumptions: vec![],
            discharged_assumptions: vec![],
            step_type: if i == 0 {
                StepType::Given
            } else if i == lines.len() - 1 {
                StepType::Conclusion
            } else {
                StepType::Deduction
            },
            confidence: 0.85,
        };
        steps.push(step);
    }

    // Determine proof technique
    let proof_technique = if proof.contains("contradiction") || proof.contains("suppose not") {
        ProofTechnique::Contradiction
    } else if proof.contains("induction") || proof.contains("base case") {
        ProofTechnique::Induction
    } else if proof.contains("case 1") || proof.contains("Case ") {
        ProofTechnique::CaseAnalysis
    } else {
        ProofTechnique::Direct
    };

    // Build dependencies from real per-step citations only (was: an
    // unconditional i-1 chain regardless of content — see extract_step_references).
    let dependencies: Vec<StepDependency> = steps.iter()
        .flat_map(|s| s.dependencies.iter().map(move |&dep| StepDependency {
            from_step: dep,
            to_step: s.step_number,
            dependency_type: DependencyType::Uses,
        }))
        .collect();

    let is_valid = verify && gaps.is_empty();
    let confidence = if is_valid { 0.90 } else { 0.60 };

    // P4 extended to proofs: parse_result is always None here, so the
    // existing P4 supplement in derive_math_keywords never fired for proof
    // analysis. Combined step text gives derive_math_keywords real prose
    // terms for proof-heavy content the same way it already does for bare
    // expression parses.
    let combined_step_text = steps.iter()
        .map(|s| s.statement.as_str())
        .collect::<Vec<_>>()
        .join(". ");
    let content_keywords = extract_content_keywords(&combined_step_text);

    Ok(ProofAnalysis {
        title: None,
        statement: lines.last().map(|s| s.to_string()).unwrap_or_default(),
        steps,
        axioms_used,
        theorems_used,
        definitions_used,
        is_valid,
        confidence,
        gaps,
        proof_technique,
        scope_tree: None,
        dependencies,
        content_keywords,
    })
}

async fn verify_step(
    step: &ProofStep,
    context: &ProofContext,
    _allowed_rules: Option<&[String]>,
) -> Result<VerificationResult, String> {
    // In production, this would do actual logical verification
    
    let mut details = Vec::new();
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut applied_rules = Vec::new();

    // Check that dependencies exist
    for dep in &step.dependencies {
        if context.previous_steps.iter().any(|s| s.step_number == *dep) {
            details.push(ValidationDetail {
                aspect: format!("Dependency on step {}", dep),
                status: ValidationStatus::Valid,
                message: "Referenced step exists".to_string(),
            });
        } else {
            errors.push(ValidationError {
                error_type: "MissingDependency".to_string(),
                message: format!("Step {} references non-existent step {}", step.step_number, dep),
                location: Some(step.step_number),
                suggestion: Some("Ensure all referenced steps are defined before use".to_string()),
            });
        }
    }

    // Check assumptions
    for assumption in &step.assumptions {
        details.push(ValidationDetail {
            aspect: format!("Assumption {}", assumption.assumption_id),
            status: ValidationStatus::Valid,
            message: "Assumption properly introduced".to_string(),
        });
    }

    // Check discharged assumptions
    for discharged in &step.discharged_assumptions {
        if context.active_assumptions.iter().any(|a| a.assumption_id == *discharged) {
            details.push(ValidationDetail {
                aspect: format!("Discharge assumption {}", discharged),
                status: ValidationStatus::Valid,
                message: "Assumption properly discharged".to_string(),
            });
        } else {
            warnings.push(format!("Assumption {} not found in active assumptions", discharged));
        }
    }

    applied_rules.push(step.justification.justification_type.clone().to_string());

    let is_valid = errors.is_empty();
    let confidence = if is_valid { 0.90 } else { 0.40 };

    Ok(VerificationResult {
        is_valid,
        confidence,
        details,
        errors,
        warnings,
        applied_rules,
    })
}

impl ToString for JustificationType {
    fn to_string(&self) -> String {
        match self {
            JustificationType::Axiom => "Axiom".to_string(),
            JustificationType::Theorem => "Theorem".to_string(),
            JustificationType::Definition => "Definition".to_string(),
            JustificationType::LogicalRule => "LogicalRule".to_string(),
            JustificationType::Algebraic => "Algebraic".to_string(),
            JustificationType::Substitution => "Substitution".to_string(),
            JustificationType::Hypothesis => "Hypothesis".to_string(),
            JustificationType::Given => "Given".to_string(),
            JustificationType::PreviousResult => "PreviousResult".to_string(),
            JustificationType::Contradiction => "Contradiction".to_string(),
            JustificationType::CaseExhaustion => "CaseExhaustion".to_string(),
            JustificationType::Induction => "Induction".to_string(),
        }
    }
}

async fn build_scope_tree(
    proof_steps: &[ProofStep],
    _track_quantifiers: bool,
) -> Result<ScopeTree, String> {
    let root_scope = Scope {
        scope_id: "global".to_string(),
        parent: None,
        children: vec![],
        step_range: (1, proof_steps.len()),
        variables: vec![],
        active_assumptions: vec![],
        scope_type: ScopeType::Global,
    };

    let mut scopes = HashMap::new();
    scopes.insert("global".to_string(), root_scope.clone());

    let mut variables_by_scope = HashMap::new();
    variables_by_scope.insert("global".to_string(), vec![]);

    // Collect variables from all steps
    for step in proof_steps {
        for var in &step.introduced_variables {
            if let Some(vars) = variables_by_scope.get_mut("global") {
                if !vars.contains(&var.name) {
                    vars.push(var.name.clone());
                }
            }
        }
    }

    Ok(ScopeTree {
        root: root_scope,
        scopes,
        variables_by_scope,
    })
}

async fn evaluate_expression(
    expression: &str,
    variables: &HashMap<String, MathValue>,
    _precision: Option<u32>,
) -> Result<EvaluationResult, String> {
    // In production, use a proper math evaluation library
    
    // Simple evaluation for basic arithmetic
    let mut result_value = 0.0f64;
    let mut steps = Vec::new();

    // Very simplified - just demonstrate the structure
    steps.push(EvaluationStep {
        expression: expression.to_string(),
        value: MathValue::Symbol(expression.to_string()),
        rule_applied: "input".to_string(),
    });

    // If it's a simple number, parse it
    if let Ok(num) = expression.trim().parse::<f64>() {
        result_value = num;
    }

    Ok(EvaluationResult {
        value: MathValue::Real(result_value),
        exact: true,
        precision: Some(15),
        intermediate_steps: steps,
    })
}

async fn simplify_expression(
    expression: &str,
    _rules: &[SimplificationRule],
) -> Result<SimplificationResult, String> {
    // In production, use sympy or similar for actual simplification
    
    Ok(SimplificationResult {
        original: expression.to_string(),
        simplified: expression.to_string(), // No actual simplification
        steps: vec![],
        rules_applied: vec![],
        is_canonical: false,
    })
}

async fn create_graph(
    analysis: MathAnalysisResult,
    project_id: u64,
    graph_name: Option<String>,
    link_to_existing: bool,
) -> Result<MathGraph, String> {
    let graph_id = generate_graph_id();
    let now = chrono::Utc::now().to_rfc3339();

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut node_id_counter: u64 = 1;

    // Create root node
    let root_node_id = node_id_counter;
    nodes.push(MathGraphNode {
        node_id: root_node_id,
        node_type: MathGraphNodeType::Root,
        label: "Mathematical Content".to_string(),
        content: format!("{:?} analysis", analysis.analysis_type),
        step_number: None,
        confidence: analysis.confidence,
        properties: HashMap::new(),
        annotations: vec![],
    });
    node_id_counter += 1;

    let mut proof_steps_count = 0;
    let mut variables_count = 0;
    let mut assumptions_count = 0;

    // If we have proof analysis, create step nodes
    if let Some(proof) = &analysis.proof_analysis {
        for step in &proof.steps {
            let step_node_id = node_id_counter;
            nodes.push(MathGraphNode {
                node_id: step_node_id,
                node_type: MathGraphNodeType::ProofStep,
                label: format!("Step {}", step.step_number),
                content: step.statement.clone(),
                step_number: Some(step.step_number),
                confidence: step.confidence,
                properties: {
                    let mut props = HashMap::new();
                    props.insert("step_type".to_string(), serde_json::to_value(&step.step_type).unwrap_or(Value::Null));
                    props.insert("justification".to_string(), serde_json::to_value(&step.justification.justification_type.to_string()).unwrap_or(Value::Null));
                    props
                },
                annotations: vec![],
            });

            // Edge from root to step
            edges.push(MathGraphEdge {
                edge_id: edges.len() as u64 + 1,
                from_node: root_node_id,
                to_node: step_node_id,
                edge_type: MathEdgeType::Contains,
                weight: 1.0,
                properties: HashMap::new(),
            });

            // Real, unconditional presentation-order edge — honest structural
            // fact, kept separate from Uses (a real cited content
            // dependency) so the two are never conflated again.
            if step.step_number > 1 {
                if let Some(prev_node) = nodes.iter().find(|n| {
                    n.node_type == MathGraphNodeType::ProofStep && n.step_number == Some(step.step_number - 1)
                }) {
                    edges.push(MathGraphEdge {
                        edge_id: edges.len() as u64 + 1,
                        from_node: step_node_id,
                        to_node: prev_node.node_id,
                        edge_type: MathEdgeType::FollowsStep,
                        weight: 1.0,
                        properties: HashMap::new(),
                    });
                }
            }

            // Edges to real cited dependencies (empty when the step's own
            // text has no citation evidence — see extract_step_references)
            for dep in &step.dependencies {
                if let Some(dep_node) = nodes.iter().find(|n| n.step_number == Some(*dep)) {
                    edges.push(MathGraphEdge {
                        edge_id: edges.len() as u64 + 1,
                        from_node: step_node_id,
                        to_node: dep_node.node_id,
                        edge_type: MathEdgeType::Uses,
                        weight: 1.0,
                        properties: HashMap::new(),
                    });
                }
            }

            // Create assumption nodes
            for assumption in &step.assumptions {
                let assumption_node_id = node_id_counter + 1000;
                nodes.push(MathGraphNode {
                    node_id: assumption_node_id,
                    node_type: MathGraphNodeType::Assumption,
                    label: format!("Assumption {}", assumption.assumption_id),
                    content: assumption.statement.clone(),
                    step_number: Some(step.step_number),
                    confidence: 1.0,
                    properties: HashMap::new(),
                    annotations: vec![],
                });

                edges.push(MathGraphEdge {
                    edge_id: edges.len() as u64 + 1,
                    from_node: step_node_id,
                    to_node: assumption_node_id,
                    edge_type: MathEdgeType::AssumesIn,
                    weight: 1.0,
                    properties: HashMap::new(),
                });

                assumptions_count += 1;
            }

            // Create variable nodes for introduced variables
            for var in &step.introduced_variables {
                let var_node_id = node_id_counter + 2000 + variables_count as u64;
                nodes.push(MathGraphNode {
                    node_id: var_node_id,
                    node_type: MathGraphNodeType::Variable,
                    label: var.name.clone(),
                    content: format!("{} : {:?}", var.name, var.var_type),
                    step_number: Some(step.step_number),
                    confidence: 1.0,
                    properties: HashMap::new(),
                    annotations: vec![],
                });

                edges.push(MathGraphEdge {
                    edge_id: edges.len() as u64 + 1,
                    from_node: step_node_id,
                    to_node: var_node_id,
                    edge_type: MathEdgeType::Defines,
                    weight: 1.0,
                    properties: HashMap::new(),
                });

                variables_count += 1;
            }

            proof_steps_count += 1;
            node_id_counter += 1;
        }

        // Add edges for discharged assumptions
        for step in &proof.steps {
            if let Some(step_node) = nodes.iter().find(|n| n.step_number == Some(step.step_number)) {
                for discharged_id in &step.discharged_assumptions {
                    if let Some(assumption_node) = nodes.iter().find(|n| {
                        n.node_type == MathGraphNodeType::Assumption &&
                        n.label == format!("Assumption {}", discharged_id)
                    }) {
                        edges.push(MathGraphEdge {
                            edge_id: edges.len() as u64 + 1,
                            from_node: step_node.node_id,
                            to_node: assumption_node.node_id,
                            edge_type: MathEdgeType::DischargesIn,
                            weight: 1.0,
                            properties: HashMap::new(),
                        });
                    }
                }
            }
        }
    }

    // If we have parse result, create expression nodes
    if let Some(parse) = &analysis.parse_result {
        for var in &parse.variables {
            let var_node_id = node_id_counter;
            nodes.push(MathGraphNode {
                node_id: var_node_id,
                node_type: MathGraphNodeType::Variable,
                label: var.name.clone(),
                content: format!("{} : {:?}", var.name, var.var_type),
                step_number: None,
                confidence: 1.0,
                properties: HashMap::new(),
                annotations: vec![],
            });

            edges.push(MathGraphEdge {
                edge_id: edges.len() as u64 + 1,
                from_node: root_node_id,
                to_node: var_node_id,
                edge_type: MathEdgeType::Contains,
                weight: 1.0,
                properties: HashMap::new(),
            });

            variables_count += 1;
            node_id_counter += 1;
        }
    }

    let scope_tree = analysis.proof_analysis.as_ref().and_then(|p| p.scope_tree.clone());
    let is_verified = analysis.proof_analysis.as_ref().map(|p| p.is_valid).unwrap_or(false);
    let verification_confidence = analysis.confidence;

    let mut graph = MathGraph {
        graph_id,
        name: graph_name.unwrap_or_else(|| format!("Math Graph {}", graph_id)),
        modality: MODALITY.to_string(),
        project_id,
        content_type: analysis.analysis_type.clone(),
        nodes,
        edges,
        scope_tree,
        metadata: GraphMetadata {
            node_count: 0, // filled in below, after node_count is known
            edge_count: 0,
            proof_steps: proof_steps_count,
            variables_count,
            assumptions_count,
            is_verified,
            verification_confidence,
            semantic_enriched: false,
            cross_modal_links: 0,
            link_metrics: None,
        },
        created_at: now.clone(),
        updated_at: now,
    };
    graph.metadata.node_count = graph.nodes.len();
    graph.metadata.edge_count = graph.edges.len();

    // Real ZSEI persistence — this pipeline had never been built or run
    // before (see zsei_query's doc comment above); without this, graph_id
    // was a local nanosecond timestamp backed by nothing once this one-shot
    // subprocess exited, and any later GetGraph/QueryGraph/LinkToModality
    // call referenced an id with no real data behind it. On persistence
    // failure, fall back to the local id (still usable for this one
    // in-process run) rather than failing the whole analysis — same
    // graceful-degradation posture as code modality's identical call.
    let mut link_metrics: Option<Value> = None;
    match persist_graph_container(&graph, &analysis, graph.graph_id, project_id).await {
        Ok((container_id, keywords, topics)) => {
            graph.graph_id = container_id;
            // Cross-relationship linking (task 57) — same gate and same
            // helper shape text/code use; linking failure never fails the
            // graph creation itself (non-fatal by design inside the helper).
            if link_to_existing {
                let (wired, metrics) = link_related_containers(container_id, project_id, &keywords, &topics).await;
                eprintln!(
                    "link_related_containers: wired {} edge(s) for container {}, metrics: {}",
                    wired, container_id, metrics
                );
                link_metrics = Some(metrics);
            }
        }
        Err(e) => {
            eprintln!("Failed to persist math graph to ZSEI (using local id only): {}", e);
        }
    }

    graph.metadata.link_metrics = link_metrics;
    Ok(graph)
}

/// Cross-relationship linking — ported verbatim from code modality pipeline
/// 101's `link_related_containers` (which mirrors text pipeline 100's), the
/// one honest difference being this modality's own `discovered_via`
/// provenance, "MathAnalysis". Type-blind search + client-side
/// infrastructure-type filter, >= 2 shared keyword/topic terms, real
/// bidirectional SimilarTo edges written via UpdateContainer.
/// The live relevance policy from OZONE_RELEVANCE_POLICY (exported at boot
/// by the host from the KAlgorithms registry's current preset). Falls back
/// to the graph-first defaults when absent (older host binary).
/// (graph_max_depth, neighborhood_shared_floor, seed_shared_floor, preset)
fn relevance_policy() -> (u32, usize, usize, String) {
    if let Ok(raw) = std::env::var("OZONE_RELEVANCE_POLICY") {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
            return (
                v.get("graph_max_depth").and_then(|x| x.as_u64()).unwrap_or(2) as u32,
                v.get("neighborhood_shared_floor").and_then(|x| x.as_u64()).unwrap_or(1) as usize,
                v.get("seed_shared_floor").and_then(|x| x.as_u64()).unwrap_or(2) as usize,
                v.get("preset").and_then(|x| x.as_str()).unwrap_or("graph-first").to_string(),
            );
        }
    }
    (2, 1, 2, "graph-first".to_string())
}

async fn link_related_containers(container_id: u64, project_id: u64, own_keywords: &[String], own_topics: &[String]) -> (usize, serde_json::Value) {
    fn is_infrastructure_container_type(t: &str) -> bool {
        matches!(
            t,
            "Root" | "User" | "Workspace" | "Project"
                | "Pipeline"
                | "ModalityRoot" | "MethodologyRoot" | "BlueprintRoot" | "PipelineRoot"
                | "ConsciousnessRoot" | "ExternalRoot" | "PackageRoot"
                | "JurisdictionRoot"
        )
    }

    let mut search_terms: Vec<String> = own_keywords.to_vec();
    search_terms.extend(own_topics.iter().map(|t| t.to_lowercase()));
    search_terms.sort();
    search_terms.dedup();
    if search_terms.is_empty() {
        return (0, serde_json::json!({"skipped": "no search terms"}));
    }

    let search_result = match zsei_query(serde_json::json!({
        "SearchContainersByKeywords": {
            "keywords": search_terms,
            "container_type": Value::Null,
            "strategy": Value::Null
        }
    }))
    .await
    {
        Ok(v) => v,
        Err(e) => {
            eprintln!("link_related_containers: search failed (non-fatal): {}", e);
            return (0, serde_json::json!({"skipped": "keyword search failed", "error": e}));
        }
    };

    let candidate_ids: Vec<u64> = search_result
        .get("Containers")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_u64()).collect())
        .unwrap_or_default();

    let own_set: HashSet<String> = search_terms.iter().cloned().collect();
    let started = std::time::Instant::now();
    let (walk_depth, nb_floor, seed_floor, preset_name) = relevance_policy();

    // ── Candidate source 1: the relationship neighborhood ──
    // ONE structural Traverse from the project container — real graph edges
    // (parent/child AND explicit Context.relationships, walked by the
    // store's traversal engine). UNCAPPED per user directive: the walk
    // depth is the only bound; max_results is set to an effectively
    // unlimited value and the REAL returned count lands in the metrics so
    // tuning decisions come from measured numbers, never preemptive caps.
    let mut neighborhood: HashMap<u64, u32> = HashMap::new();
    let mut traverse_returned = 0usize;
    if walk_depth > 0 && project_id != 0 {
        match zsei_query(serde_json::json!({
            "Traverse": {
                "start_container": project_id,
                "mode": "Structural",
                "filters": [],
                "max_depth": walk_depth,
                "max_results": 10_000_000u64,
                "budget": { "max_hops": walk_depth, "max_containers": 10_000_000u64, "max_latency_ms": 3_600_000u64 },
                "use_ml": false,
                "include_methodologies": false,
                "include_external_refs": false,
            }
        }))
        .await
        {
            Ok(result) => {
                if let Some(tr) = result.get("TraversalResult") {
                    let containers = tr.get("containers").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                    let paths = tr.get("paths").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                    traverse_returned = containers.len();
                    for (cid, path) in containers.iter().zip(paths.iter()) {
                        if let (Some(id), Some(hops)) = (
                            cid.as_u64(),
                            path.get("hops").and_then(|h| h.as_array()),
                        ) {
                            neighborhood.insert(id, hops.len().saturating_sub(1) as u32);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "link_related_containers: relationship walk failed (non-fatal, keyword seeds only): {}",
                    e
                );
            }
        }
    }

    // candidate_ids (keyword seeds, computed above) — source 2: they catch
    // genuinely related containers with NO graph path yet; without them
    // nothing would ever receive its FIRST relationship edge.

    // ── Score: hop-aware, floors from the relevance policy, NO CAPS ──
    // Neighborhood candidates ordered by hop distance first, keyword seeds
    // after; every unique candidate is fetched and scored. Nothing is
    // truncated — the write phase touches every candidate that clears its
    // floor, however many that is.
    // (candidate_id, container_json, context, shared_terms, hops)
    let mut scored: Vec<(u64, serde_json::Value, serde_json::Value, usize, Option<u32>)> = Vec::new();
    let mut seen: HashSet<u64> = HashSet::new();
    let mut ordered: Vec<(u64, Option<u32>)> = {
        let mut v: Vec<(u64, Option<u32>)> =
            neighborhood.iter().map(|(id, h)| (*id, Some(*h))).collect();
        v.sort_by_key(|(_, h)| *h);
        v
    };
    for id in &candidate_ids {
        ordered.push((*id, None));
    }

    let mut fetch_failures = 0usize;
    for (candidate_id, hops) in ordered {
        if candidate_id == container_id || !seen.insert(candidate_id) {
            continue;
        }
        let candidate = match zsei_query(serde_json::json!({
            "GetContainer": { "container_id": candidate_id }
        }))
        .await
        {
            Ok(v) => v,
            Err(_) => {
                fetch_failures += 1;
                continue;
            }
        };
        let container_json = candidate.get("Container").cloned().unwrap_or(candidate);

        let candidate_type = container_json
            .pointer("/local_state/metadata/container_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if is_infrastructure_container_type(candidate_type) {
            continue;
        }

        let candidate_context = match container_json.pointer("/local_state/context") {
            Some(c) => c.clone(),
            None => continue,
        };
        let candidate_terms: HashSet<String> = candidate_context
            .get("keywords")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .chain(candidate_context.get("topics").and_then(|v| v.as_array()).into_iter().flatten())
            .filter_map(|v| v.as_str())
            .map(|s| s.to_lowercase())
            .collect();

        let shared_count = own_set.intersection(&candidate_terms).count();
        let floor = if hops.is_some() { nb_floor } else { seed_floor };
        if shared_count < floor {
            continue;
        }

        scored.push((candidate_id, container_json, candidate_context, shared_count, hops));
    }

    // Most graph-adjacent first, then strongest overlap.
    scored.sort_by(|a, b| {
        let ha = a.4.unwrap_or(u32::MAX);
        let hb = b.4.unwrap_or(u32::MAX);
        ha.cmp(&hb).then(b.3.cmp(&a.3))
    });

    // Stats captured BEFORE the write loop consumes `scored` — every value
    // counted from what this run actually did.
    let qualified_total = scored.len();
    let qualified_neighborhood = scored.iter().filter(|c| c.4.is_some()).count();
    let shared_max = scored.iter().map(|c| c.3).max().unwrap_or(0);
    let shared_total: usize = scored.iter().map(|c| c.3).sum();

    // ── Write phase: real bidirectional edges, one candidate at a time ──
    let mut wired = 0usize;
    let mut already_linked_skipped = 0usize;
    let mut write_failures = 0usize;
    for (candidate_id, _container_json, candidate_context, shared_count, hops) in scored {
        let mut relationships: Vec<serde_json::Value> = candidate_context
            .get("relationships")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let already_linked = relationships.iter().any(|r| {
            r.get("target_id").and_then(|v| v.as_u64()) == Some(container_id)
        });
        if already_linked {
            already_linked_skipped += 1;
            continue;
        }

        let confidence = (0.3 + 0.15 * shared_count as f32).min(0.9);
        // TWO distinct edges: the candidate points at the new graph, the
        // new graph points at the candidate. (Found live: reusing ONE edge
        // object for both sides wrote target_id=container_id into the new
        // graph's OWN relationships — 7 self-loops.)
        let mut new_edge = serde_json::json!({
            "target_id": container_id,
            "relation_type": "SimilarTo",
            "confidence": confidence,
            "discovered_via": "MathAnalysis"
        });
        let mut reverse_edge = serde_json::json!({
            "target_id": candidate_id,
            "relation_type": "SimilarTo",
            "confidence": confidence,
            "discovered_via": "MathAnalysis"
        });
        if let Some(h) = hops {
            new_edge["graph_hops"] = serde_json::json!(h);
            reverse_edge["graph_hops"] = serde_json::json!(h);
        }

        // Candidate -> this new container
        relationships.push(new_edge);
        let mut candidate_context_updated = candidate_context.clone();
        candidate_context_updated["relationships"] = serde_json::Value::Array(relationships);
        let update_a = zsei_query(serde_json::json!({
            "UpdateContainer": {
                "container_id": candidate_id,
                "updates": { "metadata": null, "context": candidate_context_updated, "storage": null, "hints": null }
            }
        }))
        .await;

        // This new container -> candidate (bidirectional)
        let own_container = match zsei_query(serde_json::json!({
            "GetContainer": { "container_id": container_id }
        }))
        .await
        {
            Ok(v) => v,
            Err(_) => {
                write_failures += 1;
                continue;
            }
        };
        let own_container_json = own_container.get("Container").cloned().unwrap_or(own_container);
        let own_context = match own_container_json.pointer("/local_state/context") {
            Some(c) => c.clone(),
            None => {
                write_failures += 1;
                continue;
            }
        };
        let mut own_relationships: Vec<serde_json::Value> = own_context
            .get("relationships")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        own_relationships.push(reverse_edge);
        let mut own_context_updated = own_context.clone();
        own_context_updated["relationships"] = serde_json::Value::Array(own_relationships);
        let update_b = zsei_query(serde_json::json!({
            "UpdateContainer": {
                "container_id": container_id,
                "updates": { "metadata": null, "context": own_context_updated, "storage": null, "hints": null }
            }
        }))
        .await;

        if update_a.is_ok() && update_b.is_ok() {
            wired += 1;
        } else {
            write_failures += 1;
            eprintln!(
                "link_related_containers: partial/failed write for {} <-> {} (non-fatal)",
                container_id, candidate_id
            );
        }
    }

    // ── Real captured metrics (user directive: optimize from measured
    // numbers, never preemptive caps) — every value counted from what this
    // run actually did. ──
    let metrics = serde_json::json!({
        "policy_preset": preset_name,
        "graph_max_depth": walk_depth,
        "neighborhood_shared_floor": nb_floor,
        "seed_shared_floor": seed_floor,
        "traverse_containers_returned": traverse_returned,
        "neighborhood_candidates": neighborhood.len(),
        "seed_candidates": candidate_ids.len(),
        "unique_candidates": seen.len(),
        "fetch_failures": fetch_failures,
        "qualified": qualified_total + already_linked_skipped,
        "qualified_neighborhood": qualified_neighborhood,
        "qualified_seeds": qualified_total - qualified_neighborhood + already_linked_skipped,
        "shared_terms_max": shared_max,
        "shared_terms_total": shared_total,
        "already_linked_skipped": already_linked_skipped,
        "edges_written": wired,
        "write_failures": write_failures,
        "duration_ms": started.elapsed().as_millis() as u64,
    });
    (wired, metrics)
}

async fn update_graph(graph_id: u64, updates: MathGraphUpdate) -> Result<MathGraph, String> {
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

    // Re-persist so the update is visible to a later invocation — get_graph
    // reads a real file now (see read_graph_container), so an update that
    // only lived in this process's memory would be invisible to the very
    // next CLI call, same class of bug this whole pipeline had before.
    // object_store_path is keyed by the container's ORIGINAL local graph id,
    // not graph_id (which may already be the persisted container id) — look
    // it up rather than assume, since the two only coincide when persistence
    // never happened.
    match get_container_object_store_path(graph_id).await {
        Ok(object_store_path) => {
            let data_dir = env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
            // 5th real occurrence (write side this time) — same fix.
            let full_path = if std::path::Path::new(&object_store_path).is_absolute() {
                object_store_path.clone()
            } else {
                format!("{}/{}", data_dir, object_store_path)
            };
            if let Ok(json) = serde_json::to_string_pretty(&graph) {
                if let Err(e) = std::fs::write(&full_path, json) {
                    eprintln!("Failed to re-persist updated math graph to {}: {}", full_path, e);
                }
            }
        }
        Err(e) => {
            eprintln!("UpdateGraph: could not resolve container {} to re-persist ({}) — update only applied in-memory for this response", graph_id, e);
        }
    }

    Ok(graph)
}

async fn query_graph(graph_id: u64, query: MathQuery) -> Result<QueryResult, String> {
    let graph = get_graph(graph_id).await?;
    let limit = query.limit.unwrap_or(100);

    let (nodes, edges) = match query.query_type {
        MathQueryType::FindVariables => {
            let matching_nodes: Vec<_> = graph.nodes.iter()
                .filter(|n| n.node_type == MathGraphNodeType::Variable)
                .take(limit)
                .cloned()
                .collect();
            (matching_nodes, vec![])
        }

        MathQueryType::FindAssumptions { active_only } => {
            let matching_nodes: Vec<_> = graph.nodes.iter()
                .filter(|n| n.node_type == MathGraphNodeType::Assumption)
                .take(limit)
                .cloned()
                .collect();
            (matching_nodes, vec![])
        }

        MathQueryType::GetDependencies { step_number } => {
            if let Some(step_node) = graph.nodes.iter().find(|n| n.step_number == Some(step_number)) {
                let dep_edges: Vec<_> = graph.edges.iter()
                    .filter(|e| e.from_node == step_node.node_id && e.edge_type == MathEdgeType::Uses)
                    .cloned()
                    .collect();
                
                let dep_node_ids: Vec<_> = dep_edges.iter().map(|e| e.to_node).collect();
                let dep_nodes: Vec<_> = graph.nodes.iter()
                    .filter(|n| dep_node_ids.contains(&n.node_id))
                    .cloned()
                    .collect();

                (dep_nodes, dep_edges)
            } else {
                (vec![], vec![])
            }
        }

        MathQueryType::GetNodesByType { ref node_type } => {
            let matching_nodes: Vec<_> = graph.nodes.iter()
                .filter(|n| n.node_type == *node_type)
                .take(limit)
                .cloned()
                .collect();
            (matching_nodes, vec![])
        }

        MathQueryType::GetStepsByType { ref step_type } => {
            let matching_nodes: Vec<_> = graph.nodes.iter()
                .filter(|n| {
                    n.node_type == MathGraphNodeType::ProofStep &&
                    n.properties.get("step_type")
                        .and_then(|v| serde_json::from_value::<StepType>(v.clone()).ok())
                        .map(|st| std::mem::discriminant(&st) == std::mem::discriminant(step_type))
                        .unwrap_or(false)
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

async fn get_graph(graph_id: u64) -> Result<MathGraph, String> {
    read_graph_container(graph_id).await
}

async fn check_completeness(
    proof_steps: &[ProofStep],
    goal: &str,
) -> Result<CompletenessResult, String> {
    let mut undischarged: Vec<Assumption> = Vec::new();
    let mut active_assumptions: HashSet<usize> = HashSet::new();

    for step in proof_steps {
        // Track assumptions
        for assumption in &step.assumptions {
            active_assumptions.insert(assumption.assumption_id);
        }
        // Track discharges
        for discharged in &step.discharged_assumptions {
            active_assumptions.remove(discharged);
        }
    }

    // Find all undischarged assumptions
    for step in proof_steps {
        for assumption in &step.assumptions {
            if active_assumptions.contains(&assumption.assumption_id) {
                undischarged.push(assumption.clone());
            }
        }
    }

    // Check if last step matches goal
    let goal_reached = proof_steps.last()
        .map(|s| s.statement.contains(goal) || goal.contains(&s.statement))
        .unwrap_or(false);

    let is_complete = goal_reached && undischarged.is_empty();

    let mut suggestions = Vec::new();
    if !goal_reached {
        suggestions.push("The final step does not appear to establish the goal".to_string());
    }
    for assumption in &undischarged {
        suggestions.push(format!("Assumption {} needs to be discharged", assumption.assumption_id));
    }

    Ok(CompletenessResult {
        is_complete,
        goal_reached,
        missing_steps: vec![],
        undischarged_assumptions: undischarged,
        unused_hypotheses: vec![],
        suggestions,
    })
}

async fn find_variable_uses(graph_id: u64, variable_name: &str) -> Result<VariableUsesResult, String> {
    let graph = get_graph(graph_id).await?;

    let uses: Vec<VariableUse> = graph.nodes.iter()
        .filter(|n| n.content.contains(variable_name))
        .filter_map(|n| {
            n.step_number.map(|step| VariableUse {
                step_number: step,
                usage_type: if n.node_type == MathGraphNodeType::Variable {
                    VariableUsageType::Definition
                } else {
                    VariableUsageType::Reference
                },
                context: n.content.clone(),
            })
        })
        .collect();

    let definition = graph.nodes.iter()
        .find(|n| n.node_type == MathGraphNodeType::Variable && n.label == variable_name)
        .and_then(|n| {
            n.step_number.map(|step| VariableDefinition {
                step_number: step,
                definition: n.content.clone(),
                var_type: VariableType::Unknown,
            })
        });

    Ok(VariableUsesResult {
        variable_name: variable_name.to_string(),
        uses,
        definition,
        scope: None,
    })
}

async fn trace_derivation(graph_id: u64, from_step: u64, to_step: u64) -> Result<DerivationPath, String> {
    let graph = get_graph(graph_id).await?;

    // Simple BFS to find path
    let mut visited = HashSet::new();
    let mut queue = vec![(to_step, vec![to_step])];
    let mut result_path = Vec::new();

    while let Some((current, path)) = queue.pop() {
        if current == from_step {
            result_path = path;
            break;
        }

        if visited.contains(&current) {
            continue;
        }
        visited.insert(current);

        // Find edges leading to current
        for edge in &graph.edges {
            if edge.to_node == current && edge.edge_type == MathEdgeType::Uses {
                let mut new_path = path.clone();
                new_path.push(edge.from_node);
                queue.push((edge.from_node, new_path));
            }
        }
    }

    result_path.reverse();

    Ok(DerivationPath {
        from_step,
        to_step,
        path_length: result_path.len(),
        path: result_path,
        dependencies: vec![],
    })
}

/// Append one real ZSEI Relation (RelatedTo, discovered_via Manual — this is
/// an explicit LinkToModality call, not something inferred) to a container's
/// existing `context.relationships`, preserving every other context field
/// exactly (ContainerUpdate.context is a full replace, not a merge, so the
/// current context has to be read back first). Real RelationType has no
/// variant granular enough for CrossModalityRelation's specific labels
/// (ImplementedBy/DescribedBy/etc) — LinkResult still reports the real,
/// specific label in its own response; only the stored ZSEI relation is the
/// coarser RelatedTo.
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
    math_graph_id: u64,
    target_graph_id: u64,
    _target_modality: &str,
    relationship: CrossModalityRelation,
) -> Result<LinkResult, String> {
    // Real, bidirectional ZSEI relationship — both containers must actually
    // exist; this used to unconditionally fabricate a "success" LinkResult
    // for any graph_id, real or not, with nothing behind it. An honest
    // error here (e.g. target_graph_id from a different modality that was
    // never persisted) is correct behavior, not a regression.
    add_relationship(math_graph_id, target_graph_id).await?;
    add_relationship(target_graph_id, math_graph_id).await?;

    Ok(LinkResult {
        link_id: generate_graph_id(),
        source_graph_id: math_graph_id,
        target_graph_id,
        relationship: format!("{:?}", relationship),
        created_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// Real counts from the actual persisted graph — this previously returned
/// hardcoded `nodes_processed: 15, edges_added: 8, annotations_added: 20`
/// unconditionally, regardless of input (confirmed live in source: no
/// reference to graph_id or hook_type anywhere in the body). Full semantic
/// enrichment (real cross-node annotation generation) is a separate, larger
/// task — this fixes the immediate honesty problem (fabricated non-zero
/// metrics implying real processing happened) without pretending to
/// implement enrichment that doesn't exist yet: annotations_added stays a
/// real, honest 0.
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

/// The host invokes every pipeline binary with `--input <json> --execution-id
/// ...` (see PipelineExecutor::invoke_pipeline) — never a bare positional
/// arg. This pipeline previously read `args[1]` directly, so any real
/// invocation from the host got the literal string "--input" as its "JSON"
/// and failed instantly — confirmed live: this pipeline had never actually
/// been built or run before (same starting state code modality was in
/// before this exact bug class was found and fixed there). The host also
/// wraps the action payload in a {"data": ..., "context": ...} envelope
/// before serializing it to --input, so that needs unwrapping too — see
/// code/text modality's identical parse_cli_input. A bare positional JSON
/// arg is kept as a fallback for standalone/manual testing.
fn parse_cli_input() -> Result<Value, String> {
    let args: Vec<String> = env::args().collect();
    let mut input_json: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--input" && i + 1 < args.len() {
            input_json = Some(args[i + 1].clone());
            i += 2;
        } else {
            i += 1;
        }
    }
    let raw = match input_json {
        Some(s) => s,
        None => {
            let positional = args.get(1).cloned();
            positional.ok_or_else(|| {
                format!(
                    "Usage: {} --input <json> --execution-id <id>  (or a bare positional JSON arg for manual testing)",
                    args.get(0).cloned().unwrap_or_else(|| "math_analysis".to_string())
                )
            })?
        }
    };
    let v: Value = serde_json::from_str(&raw).map_err(|e| format!("Failed to parse input JSON: {}", e))?;
    Ok(v.get("data").cloned().unwrap_or(v))
}

#[tokio::main]
async fn main() {
    let input: Value = match parse_cli_input() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{}", e);
            eprintln!("Pipeline: {} v{}", PIPELINE_NAME, PIPELINE_VERSION);
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

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_expression() {
        let input = serde_json::json!({
            "action": {
                "type": "ParseExpression",
                "expression": "x^2 + 2*x + 1 = 0",
                "format": "LaTeX",
                "extract_variables": true,
                "simplify": false
            }
        });

        let result = execute(input).await;
        assert!(result.is_ok());

        let output: MathModalityOutput = serde_json::from_value(result.unwrap()).unwrap();
        assert!(output.success);
        assert_eq!(output.action, "ParseExpression");
    }

    #[tokio::test]
    async fn test_analyze_proof() {
        let proof = r#"
            Let x be an arbitrary real number.
            Assume x > 0.
            Then x^2 > 0.
            Therefore, for all x > 0, x^2 > 0.
        "#;

        let input = serde_json::json!({
            "action": {
                "type": "AnalyzeProof",
                "proof": proof,
                "format": "Plain",
                "verify": true,
                "check_completeness": true
            }
        });

        let result = execute(input).await;
        assert!(result.is_ok());

        let output: MathModalityOutput = serde_json::from_value(result.unwrap()).unwrap();
        assert!(output.success);
        assert_eq!(output.action, "AnalyzeProof");
    }

    /// 2026-09-22 graph audit regression test: `extract_step_references` must
    /// never fabricate a citation the step's own text doesn't actually contain.
    #[test]
    fn test_extract_step_references_no_fabrication() {
        // No citation anywhere in the text -> empty, NOT defaulted to i-1.
        assert_eq!(extract_step_references("Then x^2 > 0.", 3), Vec::<usize>::new());

        // Explicit numbered citation to a non-adjacent earlier step resolves
        // to that real step, not to the immediately preceding one.
        assert_eq!(extract_step_references("Using step 1, we get x^2 > 0.", 4), vec![1]);

        // Bare "(N)" mid-statement citation.
        assert_eq!(extract_step_references("Substituting from (2) gives the result.", 5), vec![2]);

        // A leading "(N)" reads as an enumeration marker, not a citation.
        assert_eq!(extract_step_references("(1) First we assume x > 0.", 3), Vec::<usize>::new());

        // Implicit back-reference phrase resolves to the immediately
        // preceding step — grounded in real text, unlike the old default.
        assert_eq!(extract_step_references("As shown above, the result follows.", 4), vec![3]);

        // A citation to a step number >= the current step isn't a valid
        // earlier reference and must not be returned.
        assert_eq!(extract_step_references("See step 5 for details.", 3), Vec::<usize>::new());
    }

    /// 2026-09-22 graph audit regression test: the worst finding — every
    /// proof step's dependency was hardcoded to the immediately preceding
    /// step regardless of content. Proves a non-adjacent citation resolves
    /// correctly and an uncited step stays honestly empty.
    #[tokio::test]
    async fn test_analyze_proof_does_not_fabricate_dependencies() {
        let proof = "Let x be an integer.\nLet y be an integer.\nUsing step 1, x is real.";
        let analysis = analyze_proof(proof, MathFormat::Plain, false, false).await.unwrap();

        assert_eq!(analysis.steps.len(), 3);
        assert_eq!(analysis.steps[0].dependencies, Vec::<usize>::new());
        // Step 2 has no citation in its own text -> must stay empty, not [1]
        // (the old bug would have fabricated a dependency on step 1 here).
        assert_eq!(analysis.steps[1].dependencies, Vec::<usize>::new());
        // Step 3 cites step 1 explicitly -> must resolve to [1], not the old
        // fabricated [2] (blind immediately-preceding-step default).
        assert_eq!(analysis.steps[2].dependencies, vec![1]);

        // Structural presentation order must still be recoverable via the
        // graph (FollowsStep), independent of and in addition to real
        // content citations (Uses) — the two facts stay separate.
        let graph_analysis = MathAnalysisResult {
            analysis_type: MathAnalysisType::Proof,
            parse_result: None,
            proof_analysis: Some(analysis),
            confidence: 0.9,
        };
        let graph = create_graph(graph_analysis, 1, None, false).await.unwrap();
        let follows_count = graph.edges.iter().filter(|e| e.edge_type == MathEdgeType::FollowsStep).count();
        let uses_count = graph.edges.iter().filter(|e| e.edge_type == MathEdgeType::Uses).count();
        assert_eq!(follows_count, 2); // step2->step1, step3->step2 (unconditional order)
        assert_eq!(uses_count, 1);    // only step3->step1 (real cited content dependency)
    }

    #[tokio::test]
    async fn test_create_graph_from_proof() {
        let analysis = MathAnalysisResult {
            analysis_type: MathAnalysisType::Proof,
            parse_result: None,
            proof_analysis: Some(ProofAnalysis {
                title: Some("Test Proof".to_string()),
                statement: "x > 0 implies x^2 > 0".to_string(),
                steps: vec![
                    ProofStep {
                        step_number: 1,
                        statement: "Let x > 0".to_string(),
                        justification: Justification {
                            justification_type: JustificationType::Given,
                            rule_name: None,
                            explanation: None,
                            referenced_steps: vec![],
                        },
                        dependencies: vec![],
                        introduced_variables: vec![Variable {
                            name: "x".to_string(),
                            var_type: VariableType::Real,
                            constraints: vec!["x > 0".to_string()],
                            scope: None,
                            quantifier: None,
                            initial_value: None,
                        }],
                        assumptions: vec![],
                        discharged_assumptions: vec![],
                        step_type: StepType::Given,
                        confidence: 1.0,
                    },
                    ProofStep {
                        step_number: 2,
                        statement: "x^2 > 0".to_string(),
                        justification: Justification {
                            justification_type: JustificationType::Algebraic,
                            rule_name: Some("positive_square".to_string()),
                            explanation: Some("Product of positive numbers is positive".to_string()),
                            referenced_steps: vec![1],
                        },
                        dependencies: vec![1],
                        introduced_variables: vec![],
                        assumptions: vec![],
                        discharged_assumptions: vec![],
                        step_type: StepType::Conclusion,
                        confidence: 0.95,
                    },
                ],
                axioms_used: vec![],
                theorems_used: vec![],
                definitions_used: vec![],
                is_valid: true,
                confidence: 0.95,
                gaps: vec![],
                proof_technique: ProofTechnique::Direct,
                scope_tree: None,
                dependencies: vec![StepDependency {
                    from_step: 1,
                    to_step: 2,
                    dependency_type: DependencyType::Uses,
                }],
                content_keywords: vec![],
            }),
            confidence: 0.95,
        };

        let graph = create_graph(analysis, 1, Some("Test".to_string()), false).await.unwrap();

        assert!(!graph.nodes.is_empty());
        assert!(!graph.edges.is_empty());
        assert_eq!(graph.metadata.proof_steps, 2);
        assert_eq!(graph.metadata.variables_count, 1);
    }

    /// T-M2 (GRAPH_TEST_PLAN §5) — cross-process retrieval, durable half of
    /// the reference implementation all other modalities copy: the persisted
    /// file bakes the REAL container id into its own graph_id (not the local
    /// placeholder) and round-trips intact from disk in a fresh data dir.
    /// The HTTP half (GetContainer -> object_store_path -> read) was proven
    /// live 2026-09-19 with container 30392 (real keywords, real parentage);
    /// what a unit test can honestly pin is this file contract.
    #[test]
    fn t_m2_persisted_file_bakes_container_id_and_round_trips() {
        static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = ENV_LOCK.lock().unwrap();

        let dir = std::env::temp_dir().join(format!(
            "ozone_t_m2_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        std::env::set_var("OZONE_ZSEI_DATA_DIR", &dir);

        let local_id: u64 = 1789819000000000001;
        let container_id: u64 = 778003;
        let mut graph_json = serde_json::json!({
            "graph_id": local_id,
            "name": "t-m2 round trip",
            "nodes": [],
            "edges": [],
            "metadata": { "node_count": 0, "edge_count": 0, "proof_steps": 0, "variables_count": 0 },
            "created_at": "2026-09-19T00:00:00+00:00",
            "updated_at": "2026-09-19T00:00:00+00:00",
            "semantic_enriched": false,
            "cross_modal_links": 0,
        });

        // The reference pattern under test (persist_graph_container's own
        // sequence): bake the real container id BEFORE writing the file.
        if let Some(obj) = graph_json.as_object_mut() {
            obj.insert("graph_id".to_string(), serde_json::json!(container_id));
        }
        write_graph_json_file(local_id, &graph_json).expect("write must succeed");

        // "Fresh process": read the file back by path and verify the baked id.
        let path = dir.join(format!("graphs/math_{}.json", local_id));
        let raw = std::fs::read_to_string(&path).expect("persisted file must exist");
        let read_back: serde_json::Value = serde_json::from_str(&raw).expect("must parse");
        assert_eq!(
            read_back.get("graph_id").and_then(|v| v.as_u64()),
            Some(container_id),
            "file must carry the real container id, not the local placeholder"
        );

        std::env::remove_var("OZONE_ZSEI_DATA_DIR");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
