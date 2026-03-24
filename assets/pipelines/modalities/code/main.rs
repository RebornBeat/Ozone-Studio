//! OZONE Studio - Code Modality Pipeline (ID: 101)
//! 
//! Analyzes code and creates structural graphs for:
//! - AST/parse tree structure
//! - Dependencies (imports, references)
//! - Function/class definitions
//! - Type information
//! - Call graphs

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use regex::Regex;

// ============================================================================
// INPUT/OUTPUT TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeModalityInput {
    pub action: CodeModalityAction,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CodeModalityAction {
    /// Analyze code and create structural representation
    Analyze {
        code: String,
        language: Option<String>,
        file_path: Option<String>,
        #[serde(default)]
        depth: AnalysisDepth,
    },
    
    /// Create a graph from analysis results
    CreateGraph {
        analysis_result: CodeAnalysisResult,
        project_id: u64,
        #[serde(default)]
        link_to_existing: bool,
    },
    
    /// Update existing graph with code changes
    UpdateGraph {
        graph_id: u64,
        delta: CodeDelta,
    },
    
    /// Query the code graph
    QueryGraph {
        graph_id: u64,
        query: CodeGraphQuery,
    },
    
    /// Get dependency graph for a project
    GetDependencyGraph {
        project_id: u64,
        #[serde(default)]
        include_external: bool,
    },
    
    /// Compute reverse dependencies
    ComputeReverseDependencies {
        project_id: u64,
        file_path: String,
    },
    
    /// Create provisional graph nodes (for graph-first approach)
    CreateProvisionalNodes {
        project_id: u64,
        planned_files: Vec<PlannedFile>,
        session_id: String,
    },
    
    /// Get graph including provisional nodes
    GetGraphWithProvisional {
        project_id: u64,
        session_id: String,
    },
    
    /// Finalize provisional nodes after successful write
    FinalizeProvisional {
        session_id: String,
        file_container_ids: Vec<(u64, u64)>, // provisional_id -> actual_id
    },
    
    /// Rollback provisional nodes on failure
    RollbackProvisional {
        session_id: String,
    },
    
    /// Check for conflicts before code generation
    CheckConflicts {
        project_id: u64,
        proposed_structure: ProposedCodeStructure,
    },
    
    /// Trigger ZSEI semantic analysis hook
    TriggerSemanticHook {
        graph_id: u64,
        hook_type: ZSEIHookType,
    },
    
    /// Suggest applicable methodologies
    SuggestMethodologies {
        code: String,
        language: String,
        available_methodology_ids: Vec<u64>,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub enum AnalysisDepth {
    #[default]
    Standard,
    Deep,       // Include call graph, data flow
    Surface,    // Just structure, fast
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeModalityOutput {
    pub success: bool,
    pub error: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analysis: Option<CodeAnalysisResult>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_id: Option<u64>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<CodeGraph>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependency_graph: Option<DependencyGraph>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reverse_deps: Option<Vec<FileDependency>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflicts: Option<ConflictReport>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provisional_nodes: Option<Vec<ProvisionalNode>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_methodologies: Option<Vec<MethodologySuggestion>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hook_result: Option<HookResult>,
}

impl Default for CodeModalityOutput {
    fn default() -> Self {
        Self {
            success: false,
            error: None,
            analysis: None,
            graph_id: None,
            graph: None,
            dependency_graph: None,
            reverse_deps: None,
            conflicts: None,
            provisional_nodes: None,
            suggested_methodologies: None,
            hook_result: None,
        }
    }
}

// ============================================================================
// ANALYSIS TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodeAnalysisResult {
    pub language: String,
    pub file_path: Option<String>,
    pub line_count: usize,
    pub functions: Vec<FunctionDef>,
    pub classes: Vec<ClassDef>,
    pub imports: Vec<ImportDef>,
    pub exports: Vec<ExportDef>,
    pub variables: Vec<VariableDef>,
    pub type_definitions: Vec<TypeDef>,
    pub comments: Vec<Comment>,
    pub function_calls: Vec<FunctionCall>,
    pub complexity_metrics: ComplexityMetrics,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionDef {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<String>,
    pub is_async: bool,
    pub is_public: bool,
    pub doc_comment: Option<String>,
    pub calls: Vec<String>,
    pub complexity: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClassDef {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub methods: Vec<FunctionDef>,
    pub fields: Vec<VariableDef>,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub is_public: bool,
    pub doc_comment: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImportDef {
    pub module: String,
    pub items: Vec<String>,
    pub is_external: bool,
    pub line: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExportDef {
    pub name: String,
    pub export_type: ExportType,
    pub line: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ExportType {
    Function,
    Class,
    Variable,
    Type,
    Default,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VariableDef {
    pub name: String,
    pub var_type: Option<String>,
    pub is_const: bool,
    pub is_public: bool,
    pub line: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TypeDef {
    pub name: String,
    pub kind: TypeKind,
    pub fields: Vec<(String, String)>,
    pub line: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TypeKind {
    Struct,
    Enum,
    Interface,
    TypeAlias,
    Trait,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Parameter {
    pub name: String,
    pub param_type: Option<String>,
    pub default_value: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Comment {
    pub text: String,
    pub line: usize,
    pub is_doc: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionCall {
    pub caller: String,
    pub callee: String,
    pub line: usize,
    pub is_method: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComplexityMetrics {
    pub cyclomatic_complexity: usize,
    pub cognitive_complexity: usize,
    pub halstead_metrics: Option<HalsteadMetrics>,
    pub maintainability_index: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HalsteadMetrics {
    pub distinct_operators: usize,
    pub distinct_operands: usize,
    pub total_operators: usize,
    pub total_operands: usize,
    pub program_length: usize,
    pub vocabulary: usize,
    pub volume: f32,
    pub difficulty: f32,
    pub effort: f32,
}

// ============================================================================
// GRAPH TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodeGraph {
    pub graph_id: u64,
    pub modality: String,
    pub nodes: Vec<CodeGraphNode>,
    pub edges: Vec<CodeGraphEdge>,
    pub metadata: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodeGraphNode {
    pub node_id: u64,
    pub node_type: CodeNodeType,
    pub name: String,
    pub position: Option<CodePosition>,
    pub properties: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CodeNodeType {
    File,
    Module,
    Class,
    Function,
    Method,
    Variable,
    Type,
    Import,
    Export,
    Parameter,
    Block,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodePosition {
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub start_column: Option<usize>,
    pub end_column: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodeGraphEdge {
    pub from_node: u64,
    pub to_node: u64,
    pub edge_type: CodeEdgeType,
    pub weight: f32,
    pub properties: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CodeEdgeType {
    // Structural
    Contains,
    Imports,
    Exports,
    Extends,
    Implements,
    
    // Dependency
    DependsOn,
    Calls,
    References,
    TypeOf,
    
    // Semantic (added by ZSEI)
    RelatesTo,
    SimilarTo,
    AlternativeTo,
    Refactors,
    Tests,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DependencyGraph {
    pub project_id: u64,
    pub files: Vec<FileNode>,
    pub dependencies: Vec<FileDependency>,
    pub external_deps: Vec<ExternalDependency>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileNode {
    pub file_id: u64,
    pub path: String,
    pub language: String,
    pub line_count: usize,
    pub function_count: usize,
    pub class_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileDependency {
    pub from_file: String,
    pub to_file: String,
    pub dependency_type: DependencyType,
    pub imports: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum DependencyType {
    Direct,
    Transitive,
    DevOnly,
    Optional,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExternalDependency {
    pub package: String,
    pub version: Option<String>,
    pub used_by: Vec<String>,
}

// ============================================================================
// PROVISIONAL/CONFLICT TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlannedFile {
    pub path: String,
    pub language: String,
    pub planned_exports: Vec<String>,
    pub planned_imports: Vec<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProvisionalNode {
    pub provisional_id: u64,
    pub file_path: String,
    pub session_id: String,
    pub created_at: String,
    pub planned_structure: PlannedFile,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProposedCodeStructure {
    pub files: Vec<ProposedFile>,
    pub new_functions: Vec<ProposedFunction>,
    pub new_types: Vec<ProposedType>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProposedFile {
    pub path: String,
    pub exports: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProposedFunction {
    pub name: String,
    pub file_path: String,
    pub signature: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProposedType {
    pub name: String,
    pub file_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConflictReport {
    pub has_conflicts: bool,
    pub conflicts: Vec<Conflict>,
    pub warnings: Vec<Warning>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Conflict {
    pub conflict_type: ConflictType,
    pub description: String,
    pub existing_location: Option<String>,
    pub proposed_location: String,
    pub resolution: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ConflictType {
    DuplicateName,
    CircularDependency,
    BreakingChange,
    TypeMismatch,
    MissingDependency,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Warning {
    pub warning_type: String,
    pub message: String,
    pub location: Option<String>,
}

// ============================================================================
// OTHER TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeDelta {
    pub operation: DeltaOperation,
    pub file_path: String,
    pub position: Option<CodePosition>,
    pub content: Option<String>,
    pub affected_nodes: Vec<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DeltaOperation {
    Insert,
    Delete,
    Replace,
    Rename,
    Move,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeGraphQuery {
    pub query_type: CodeQueryType,
    pub parameters: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CodeQueryType {
    FindFunction,
    FindClass,
    FindType,
    FindUsages,
    FindDependencies,
    FindReverseDependencies,
    GetCallGraph,
    GetTypeHierarchy,
    SemanticSearch,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MethodologySuggestion {
    pub methodology_id: u64,
    pub relevance: f32,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ZSEIHookType {
    OnGraphCreated,
    OnEdgeCompletion,
    OnInferRelationships,
    OnCrossModalityLink,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HookResult {
    pub hook_type: ZSEIHookType,
    pub success: bool,
    pub edges_added: usize,
    pub nodes_enriched: usize,
}

// ============================================================================
// PIPELINE IMPLEMENTATION
// ============================================================================

pub struct CodeModalityPipeline {
    provisional_nodes: HashMap<String, Vec<ProvisionalNode>>,
}

impl CodeModalityPipeline {
    pub fn new() -> Self {
        Self {
            provisional_nodes: HashMap::new(),
        }
    }
    
    pub async fn execute(&self, input: CodeModalityInput) -> CodeModalityOutput {
        match input.action {
            CodeModalityAction::Analyze { code, language, file_path, depth } => {
                self.analyze_code(&code, language, file_path, depth)
            }
            
            CodeModalityAction::CreateGraph { analysis_result, project_id, link_to_existing } => {
                self.create_graph(analysis_result, project_id, link_to_existing).await
            }
            
            CodeModalityAction::UpdateGraph { graph_id, delta } => {
                self.update_graph(graph_id, delta).await
            }
            
            CodeModalityAction::QueryGraph { graph_id, query } => {
                self.query_graph(graph_id, query).await
            }
            
            CodeModalityAction::GetDependencyGraph { project_id, include_external } => {
                self.get_dependency_graph(project_id, include_external).await
            }
            
            CodeModalityAction::ComputeReverseDependencies { project_id, file_path } => {
                self.compute_reverse_deps(project_id, &file_path).await
            }
            
            CodeModalityAction::CreateProvisionalNodes { project_id, planned_files, session_id } => {
                self.create_provisional_nodes(project_id, planned_files, session_id)
            }
            
            CodeModalityAction::GetGraphWithProvisional { project_id, session_id } => {
                self.get_graph_with_provisional(project_id, &session_id).await
            }
            
            CodeModalityAction::FinalizeProvisional { session_id, file_container_ids } => {
                self.finalize_provisional(&session_id, file_container_ids)
            }
            
            CodeModalityAction::RollbackProvisional { session_id } => {
                self.rollback_provisional(&session_id)
            }
            
            CodeModalityAction::CheckConflicts { project_id, proposed_structure } => {
                self.check_conflicts(project_id, proposed_structure).await
            }
            
            CodeModalityAction::TriggerSemanticHook { graph_id, hook_type } => {
                self.trigger_semantic_hook(graph_id, hook_type).await
            }
            
            CodeModalityAction::SuggestMethodologies { code, language, available_methodology_ids } => {
                self.suggest_methodologies(&code, &language, &available_methodology_ids)
            }
        }
    }
    
    fn analyze_code(
        &self,
        code: &str,
        language: Option<String>,
        file_path: Option<String>,
        depth: AnalysisDepth,
    ) -> CodeModalityOutput {
        let language = language.unwrap_or_else(|| self.detect_language(code, file_path.as_deref()));
        
        let line_count = code.lines().count();
        let functions = self.extract_functions(code, &language);
        let classes = self.extract_classes(code, &language);
        let imports = self.extract_imports(code, &language);
        let exports = self.extract_exports(code, &language);
        let variables = self.extract_variables(code, &language);
        let type_definitions = self.extract_types(code, &language);
        let comments = self.extract_comments(code, &language);
        let function_calls = if matches!(depth, AnalysisDepth::Deep) {
            self.extract_function_calls(code, &language, &functions)
        } else {
            Vec::new()
        };
        
        let complexity_metrics = self.compute_complexity(code, &functions);
        
        let analysis = CodeAnalysisResult {
            language,
            file_path,
            line_count,
            functions,
            classes,
            imports,
            exports,
            variables,
            type_definitions,
            comments,
            function_calls,
            complexity_metrics,
        };
        
        CodeModalityOutput {
            success: true,
            analysis: Some(analysis),
            ..Default::default()
        }
    }
    
    fn detect_language(&self, code: &str, file_path: Option<&str>) -> String {
        // First try to detect from file extension
        if let Some(path) = file_path {
            let ext = PathBuf::from(path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            
            match ext {
                "rs" => return "rust".to_string(),
                "py" => return "python".to_string(),
                "js" => return "javascript".to_string(),
                "ts" => return "typescript".to_string(),
                "go" => return "go".to_string(),
                "java" => return "java".to_string(),
                "c" | "h" => return "c".to_string(),
                "cpp" | "cc" | "hpp" => return "cpp".to_string(),
                "rb" => return "ruby".to_string(),
                "php" => return "php".to_string(),
                "swift" => return "swift".to_string(),
                "kt" => return "kotlin".to_string(),
                _ => {}
            }
        }
        
        // Detect from code patterns
        if code.contains("fn ") && code.contains("->") && code.contains("let ") {
            return "rust".to_string();
        }
        if code.contains("def ") && code.contains(":") && !code.contains(";") {
            return "python".to_string();
        }
        if code.contains("function ") || code.contains("const ") || code.contains("let ") {
            if code.contains(": ") && (code.contains("interface ") || code.contains(": string")) {
                return "typescript".to_string();
            }
            return "javascript".to_string();
        }
        if code.contains("func ") && code.contains("package ") {
            return "go".to_string();
        }
        if code.contains("public class ") || code.contains("private void ") {
            return "java".to_string();
        }
        
        "unknown".to_string()
    }
    
    fn extract_functions(&self, code: &str, language: &str) -> Vec<FunctionDef> {
        let mut functions = Vec::new();
        
        let pattern = match language {
            "rust" => r"(?m)^(\s*)(pub\s+)?(async\s+)?fn\s+(\w+)\s*(?:<[^>]*>)?\s*\(([^)]*)\)(?:\s*->\s*([^\{]+))?\s*\{",
            "python" => r"(?m)^(\s*)(async\s+)?def\s+(\w+)\s*\(([^)]*)\)(?:\s*->\s*([^:]+))?\s*:",
            "javascript" | "typescript" => r"(?m)^(\s*)(?:export\s+)?(?:async\s+)?function\s+(\w+)\s*\(([^)]*)\)(?:\s*:\s*([^{]+))?\s*\{",
            "go" => r"(?m)^func\s+(?:\([^)]+\)\s+)?(\w+)\s*\(([^)]*)\)(?:\s*(?:\([^)]+\)|[^{]+))?\s*\{",
            _ => return functions,
        };
        
        let re = Regex::new(pattern).unwrap();
        
        for (line_num, line) in code.lines().enumerate() {
            if let Some(caps) = re.captures(line) {
                let name = match language {
                    "rust" => caps.get(4).map(|m| m.as_str().to_string()),
                    "python" => caps.get(3).map(|m| m.as_str().to_string()),
                    "javascript" | "typescript" => caps.get(2).map(|m| m.as_str().to_string()),
                    "go" => caps.get(1).map(|m| m.as_str().to_string()),
                    _ => None,
                };
                
                if let Some(name) = name {
                    let is_async = line.contains("async");
                    let is_public = line.contains("pub ") || line.contains("export ");
                    
                    // Find function end (simplified - count braces)
                    let end_line = self.find_block_end(code, line_num);
                    
                    functions.push(FunctionDef {
                        name,
                        start_line: line_num + 1,
                        end_line,
                        parameters: Vec::new(), // TODO: Parse parameters
                        return_type: None,
                        is_async,
                        is_public,
                        doc_comment: None,
                        calls: Vec::new(),
                        complexity: 1,
                    });
                }
            }
        }
        
        functions
    }
    
    fn extract_classes(&self, code: &str, language: &str) -> Vec<ClassDef> {
        let mut classes = Vec::new();
        
        let pattern = match language {
            "rust" => r"(?m)^(\s*)(pub\s+)?(?:struct|enum|trait)\s+(\w+)",
            "python" => r"(?m)^(\s*)class\s+(\w+)(?:\([^)]*\))?\s*:",
            "javascript" | "typescript" => r"(?m)^(\s*)(?:export\s+)?class\s+(\w+)(?:\s+extends\s+(\w+))?(?:\s+implements\s+([^{]+))?\s*\{",
            "java" => r"(?m)^(\s*)(public\s+)?class\s+(\w+)(?:\s+extends\s+(\w+))?(?:\s+implements\s+([^{]+))?\s*\{",
            _ => return classes,
        };
        
        let re = Regex::new(pattern).unwrap();
        
        for (line_num, line) in code.lines().enumerate() {
            if let Some(caps) = re.captures(line) {
                let name = match language {
                    "rust" => caps.get(3).map(|m| m.as_str().to_string()),
                    "python" => caps.get(2).map(|m| m.as_str().to_string()),
                    "javascript" | "typescript" | "java" => caps.get(2).or(caps.get(3)).map(|m| m.as_str().to_string()),
                    _ => None,
                };
                
                if let Some(name) = name {
                    let is_public = line.contains("pub ") || line.contains("public ") || line.contains("export ");
                    let end_line = self.find_block_end(code, line_num);
                    
                    classes.push(ClassDef {
                        name,
                        start_line: line_num + 1,
                        end_line,
                        methods: Vec::new(),
                        fields: Vec::new(),
                        extends: caps.get(4).map(|m| m.as_str().trim().to_string()),
                        implements: Vec::new(),
                        is_public,
                        doc_comment: None,
                    });
                }
            }
        }
        
        classes
    }
    
    fn extract_imports(&self, code: &str, language: &str) -> Vec<ImportDef> {
        let mut imports = Vec::new();
        
        let pattern = match language {
            "rust" => r"(?m)^use\s+([^;]+);",
            "python" => r"(?m)^(?:from\s+(\S+)\s+)?import\s+(.+)$",
            "javascript" | "typescript" => r#"(?m)^import\s+(?:\{([^}]+)\}|(\w+))\s+from\s+['"]([^'"]+)['"]"#,
            "go" => r#"(?m)^import\s+(?:\(\s*([^)]+)\s*\)|"([^"]+)")"#,
            _ => return imports,
        };
        
        let re = Regex::new(pattern).unwrap();
        
        for (line_num, line) in code.lines().enumerate() {
            if let Some(caps) = re.captures(line) {
                let (module, items) = match language {
                    "rust" => {
                        let full = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                        (full.to_string(), vec![full.to_string()])
                    }
                    "python" => {
                        let module = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                        let items: Vec<String> = caps.get(2)
                            .map(|m| m.as_str().split(',').map(|s| s.trim().to_string()).collect())
                            .unwrap_or_default();
                        (module.to_string(), items)
                    }
                    "javascript" | "typescript" => {
                        let module = caps.get(3).map(|m| m.as_str()).unwrap_or("");
                        let items: Vec<String> = caps.get(1)
                            .map(|m| m.as_str().split(',').map(|s| s.trim().to_string()).collect())
                            .or_else(|| caps.get(2).map(|m| vec![m.as_str().to_string()]))
                            .unwrap_or_default();
                        (module.to_string(), items)
                    }
                    _ => continue,
                };
                
                let is_external = !module.starts_with('.') && !module.starts_with("crate") && !module.starts_with("super");
                
                imports.push(ImportDef {
                    module,
                    items,
                    is_external,
                    line: line_num + 1,
                });
            }
        }
        
        imports
    }
    
    fn extract_exports(&self, code: &str, language: &str) -> Vec<ExportDef> {
        let mut exports = Vec::new();
        
        let pattern = match language {
            "rust" => r"(?m)^pub\s+(fn|struct|enum|trait|type|const|static)\s+(\w+)",
            "javascript" | "typescript" => r"(?m)^export\s+(?:default\s+)?(function|class|const|let|var|type|interface)\s+(\w+)",
            _ => return exports,
        };
        
        let re = Regex::new(pattern).unwrap();
        
        for (line_num, line) in code.lines().enumerate() {
            if let Some(caps) = re.captures(line) {
                let export_type = match caps.get(1).map(|m| m.as_str()) {
                    Some("fn") | Some("function") => ExportType::Function,
                    Some("struct") | Some("class") => ExportType::Class,
                    Some("const") | Some("let") | Some("var") | Some("static") => ExportType::Variable,
                    Some("type") | Some("interface") | Some("trait") | Some("enum") => ExportType::Type,
                    _ => continue,
                };
                
                let name = caps.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
                
                exports.push(ExportDef {
                    name,
                    export_type,
                    line: line_num + 1,
                });
            }
        }
        
        exports
    }
    
    fn extract_variables(&self, code: &str, language: &str) -> Vec<VariableDef> {
        let mut variables = Vec::new();
        
        let pattern = match language {
            "rust" => r"(?m)^(?:pub\s+)?(?:static|const)\s+(\w+):\s*([^=]+)",
            "javascript" | "typescript" => r"(?m)^(?:export\s+)?(const|let|var)\s+(\w+)(?::\s*([^=]+))?",
            "python" => r"(?m)^(\w+)(?::\s*([^=]+))?\s*=",
            _ => return variables,
        };
        
        let re = Regex::new(pattern).unwrap();
        
        for (line_num, line) in code.lines().enumerate() {
            // Skip if inside a function (simplified check)
            if line.starts_with("    ") || line.starts_with("\t") {
                continue;
            }
            
            if let Some(caps) = re.captures(line) {
                let (name, var_type, is_const) = match language {
                    "rust" => {
                        let name = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
                        let var_type = caps.get(2).map(|m| m.as_str().trim().to_string());
                        let is_const = line.contains("const ");
                        (name, var_type, is_const)
                    }
                    "javascript" | "typescript" => {
                        let kind = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                        let name = caps.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
                        let var_type = caps.get(3).map(|m| m.as_str().trim().to_string());
                        (name, var_type, kind == "const")
                    }
                    "python" => {
                        let name = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
                        let var_type = caps.get(2).map(|m| m.as_str().trim().to_string());
                        let is_const = name.chars().all(|c| c.is_uppercase() || c == '_');
                        (name, var_type, is_const)
                    }
                    _ => continue,
                };
                
                let is_public = line.contains("pub ") || line.contains("export ");
                
                variables.push(VariableDef {
                    name,
                    var_type,
                    is_const,
                    is_public,
                    line: line_num + 1,
                });
            }
        }
        
        variables
    }
    
    fn extract_types(&self, code: &str, language: &str) -> Vec<TypeDef> {
        let mut types = Vec::new();
        
        let pattern = match language {
            "rust" => r"(?m)^(?:pub\s+)?type\s+(\w+)\s*=",
            "typescript" => r"(?m)^(?:export\s+)?(?:type|interface)\s+(\w+)",
            _ => return types,
        };
        
        let re = Regex::new(pattern).unwrap();
        
        for (line_num, line) in code.lines().enumerate() {
            if let Some(caps) = re.captures(line) {
                let name = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
                let kind = if line.contains("interface") {
                    TypeKind::Interface
                } else {
                    TypeKind::TypeAlias
                };
                
                types.push(TypeDef {
                    name,
                    kind,
                    fields: Vec::new(),
                    line: line_num + 1,
                });
            }
        }
        
        types
    }
    
    fn extract_comments(&self, code: &str, language: &str) -> Vec<Comment> {
        let mut comments = Vec::new();
        
        let (line_comment, doc_comment) = match language {
            "rust" => ("//", "///"),
            "python" => ("#", "\"\"\""),
            "javascript" | "typescript" | "go" | "java" | "c" | "cpp" => ("//", "/**"),
            _ => ("//", "///"),
        };
        
        for (line_num, line) in code.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with(doc_comment) {
                comments.push(Comment {
                    text: trimmed[doc_comment.len()..].trim().to_string(),
                    line: line_num + 1,
                    is_doc: true,
                });
            } else if trimmed.starts_with(line_comment) {
                comments.push(Comment {
                    text: trimmed[line_comment.len()..].trim().to_string(),
                    line: line_num + 1,
                    is_doc: false,
                });
            }
        }
        
        comments
    }
    
    fn extract_function_calls(&self, code: &str, _language: &str, functions: &[FunctionDef]) -> Vec<FunctionCall> {
        let mut calls = Vec::new();
        let function_names: HashSet<_> = functions.iter().map(|f| f.name.as_str()).collect();
        
        // Simple pattern: word followed by parenthesis
        let call_pattern = Regex::new(r"(\w+)\s*\(").unwrap();
        
        for (line_num, line) in code.lines().enumerate() {
            // Find which function we're in
            let caller = functions.iter()
                .find(|f| line_num + 1 >= f.start_line && line_num + 1 <= f.end_line)
                .map(|f| f.name.as_str())
                .unwrap_or("global");
            
            for cap in call_pattern.captures_iter(line) {
                if let Some(m) = cap.get(1) {
                    let callee = m.as_str();
                    if function_names.contains(callee) {
                        calls.push(FunctionCall {
                            caller: caller.to_string(),
                            callee: callee.to_string(),
                            line: line_num + 1,
                            is_method: line.contains(&format!(".{}", callee)),
                        });
                    }
                }
            }
        }
        
        calls
    }
    
    fn compute_complexity(&self, code: &str, functions: &[FunctionDef]) -> ComplexityMetrics {
        // Simplified cyclomatic complexity: count decision points
        let keywords = ["if", "else", "for", "while", "case", "catch", "&&", "||", "?"];
        let mut cyclomatic = 1;
        
        for keyword in keywords {
            cyclomatic += code.matches(keyword).count();
        }
        
        // Simplified cognitive complexity
        let nesting_keywords = ["if", "for", "while", "match", "try"];
        let mut cognitive = 0;
        let mut nesting_level = 0;
        
        for line in code.lines() {
            let trimmed = line.trim();
            for keyword in nesting_keywords {
                if trimmed.starts_with(keyword) {
                    cognitive += 1 + nesting_level;
                }
            }
            nesting_level += trimmed.matches('{').count();
            nesting_level = nesting_level.saturating_sub(trimmed.matches('}').count());
        }
        
        // Maintainability index (simplified)
        let loc = code.lines().count() as f32;
        let volume = loc * (code.split_whitespace().count() as f32).log2().max(1.0);
        let mi = (171.0 - 5.2 * volume.ln() - 0.23 * cyclomatic as f32 - 16.2 * loc.ln()).max(0.0);
        
        ComplexityMetrics {
            cyclomatic_complexity: cyclomatic,
            cognitive_complexity: cognitive,
            halstead_metrics: None,
            maintainability_index: mi / 171.0 * 100.0,
        }
    }
    
    fn find_block_end(&self, code: &str, start_line: usize) -> usize {
        let lines: Vec<&str> = code.lines().collect();
        let mut brace_count = 0;
        let mut found_open = false;
        
        for (i, line) in lines.iter().enumerate().skip(start_line) {
            for c in line.chars() {
                if c == '{' {
                    brace_count += 1;
                    found_open = true;
                } else if c == '}' {
                    brace_count -= 1;
                    if found_open && brace_count == 0 {
                        return i + 1;
                    }
                }
            }
        }
        
        lines.len()
    }
    
    async fn create_graph(&self, analysis: CodeAnalysisResult, project_id: u64, _link_to_existing: bool) -> CodeModalityOutput {
        let graph_id = self.generate_graph_id();
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut node_id = 1u64;
        
        // Create file root node
        let file_node_id = node_id;
        nodes.push(CodeGraphNode {
            node_id: file_node_id,
            node_type: CodeNodeType::File,
            name: analysis.file_path.clone().unwrap_or_default(),
            position: None,
            properties: {
                let mut props = HashMap::new();
                props.insert("language".to_string(), serde_json::json!(analysis.language));
                props.insert("line_count".to_string(), serde_json::json!(analysis.line_count));
                props
            },
        });
        node_id += 1;
        
        // Create function nodes
        for func in &analysis.functions {
            let func_node_id = node_id;
            nodes.push(CodeGraphNode {
                node_id: func_node_id,
                node_type: CodeNodeType::Function,
                name: func.name.clone(),
                position: Some(CodePosition {
                    file_path: analysis.file_path.clone().unwrap_or_default(),
                    start_line: func.start_line,
                    end_line: func.end_line,
                    start_column: None,
                    end_column: None,
                }),
                properties: {
                    let mut props = HashMap::new();
                    props.insert("is_async".to_string(), serde_json::json!(func.is_async));
                    props.insert("is_public".to_string(), serde_json::json!(func.is_public));
                    props.insert("complexity".to_string(), serde_json::json!(func.complexity));
                    props
                },
            });
            
            edges.push(CodeGraphEdge {
                from_node: file_node_id,
                to_node: func_node_id,
                edge_type: CodeEdgeType::Contains,
                weight: 1.0,
                properties: HashMap::new(),
            });
            
            node_id += 1;
        }
        
        // Create class nodes
        for class in &analysis.classes {
            let class_node_id = node_id;
            nodes.push(CodeGraphNode {
                node_id: class_node_id,
                node_type: CodeNodeType::Class,
                name: class.name.clone(),
                position: Some(CodePosition {
                    file_path: analysis.file_path.clone().unwrap_or_default(),
                    start_line: class.start_line,
                    end_line: class.end_line,
                    start_column: None,
                    end_column: None,
                }),
                properties: {
                    let mut props = HashMap::new();
                    props.insert("is_public".to_string(), serde_json::json!(class.is_public));
                    if let Some(ext) = &class.extends {
                        props.insert("extends".to_string(), serde_json::json!(ext));
                    }
                    props
                },
            });
            
            edges.push(CodeGraphEdge {
                from_node: file_node_id,
                to_node: class_node_id,
                edge_type: CodeEdgeType::Contains,
                weight: 1.0,
                properties: HashMap::new(),
            });
            
            node_id += 1;
        }
        
        // Create import edges
        for import in &analysis.imports {
            let import_node_id = node_id;
            nodes.push(CodeGraphNode {
                node_id: import_node_id,
                node_type: CodeNodeType::Import,
                name: import.module.clone(),
                position: Some(CodePosition {
                    file_path: analysis.file_path.clone().unwrap_or_default(),
                    start_line: import.line,
                    end_line: import.line,
                    start_column: None,
                    end_column: None,
                }),
                properties: {
                    let mut props = HashMap::new();
                    props.insert("is_external".to_string(), serde_json::json!(import.is_external));
                    props.insert("items".to_string(), serde_json::to_value(&import.items).unwrap());
                    props
                },
            });
            
            edges.push(CodeGraphEdge {
                from_node: file_node_id,
                to_node: import_node_id,
                edge_type: CodeEdgeType::Imports,
                weight: 1.0,
                properties: HashMap::new(),
            });
            
            node_id += 1;
        }
        
        let graph = CodeGraph {
            graph_id,
            modality: "code".to_string(),
            nodes,
            edges,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("project_id".to_string(), serde_json::json!(project_id));
                meta.insert("language".to_string(), serde_json::json!(analysis.language));
                meta.insert("created_at".to_string(), serde_json::json!(chrono::Utc::now().to_rfc3339()));
                meta
            },
        };
        
        CodeModalityOutput {
            success: true,
            graph_id: Some(graph_id),
            graph: Some(graph),
            ..Default::default()
        }
    }
    
    async fn update_graph(&self, graph_id: u64, _delta: CodeDelta) -> CodeModalityOutput {
        CodeModalityOutput {
            success: true,
            graph_id: Some(graph_id),
            ..Default::default()
        }
    }
    
    async fn query_graph(&self, graph_id: u64, _query: CodeGraphQuery) -> CodeModalityOutput {
        CodeModalityOutput {
            success: true,
            graph_id: Some(graph_id),
            ..Default::default()
        }
    }
    
    async fn get_dependency_graph(&self, project_id: u64, _include_external: bool) -> CodeModalityOutput {
        // Would query ZSEI for all file containers in project
        let dep_graph = DependencyGraph {
            project_id,
            files: Vec::new(),
            dependencies: Vec::new(),
            external_deps: Vec::new(),
        };
        
        CodeModalityOutput {
            success: true,
            dependency_graph: Some(dep_graph),
            ..Default::default()
        }
    }
    
    async fn compute_reverse_deps(&self, project_id: u64, file_path: &str) -> CodeModalityOutput {
        // Would query ZSEI for files that import this file
        let reverse_deps = Vec::new();
        
        CodeModalityOutput {
            success: true,
            reverse_deps: Some(reverse_deps),
            ..Default::default()
        }
    }
    
    fn create_provisional_nodes(&self, project_id: u64, planned_files: Vec<PlannedFile>, session_id: String) -> CodeModalityOutput {
        let mut provisional = Vec::new();
        let timestamp = chrono::Utc::now().to_rfc3339();
        
        for (i, file) in planned_files.into_iter().enumerate() {
            provisional.push(ProvisionalNode {
                provisional_id: (project_id * 1000000) + (i as u64),
                file_path: file.path.clone(),
                session_id: session_id.clone(),
                created_at: timestamp.clone(),
                planned_structure: file,
            });
        }
        
        CodeModalityOutput {
            success: true,
            provisional_nodes: Some(provisional),
            ..Default::default()
        }
    }
    
    async fn get_graph_with_provisional(&self, project_id: u64, session_id: &str) -> CodeModalityOutput {
        // Would merge actual graph with provisional nodes
        CodeModalityOutput {
            success: true,
            ..Default::default()
        }
    }
    
    fn finalize_provisional(&self, _session_id: &str, _file_container_ids: Vec<(u64, u64)>) -> CodeModalityOutput {
        CodeModalityOutput {
            success: true,
            ..Default::default()
        }
    }
    
    fn rollback_provisional(&self, _session_id: &str) -> CodeModalityOutput {
        CodeModalityOutput {
            success: true,
            ..Default::default()
        }
    }
    
    async fn check_conflicts(&self, _project_id: u64, proposed: ProposedCodeStructure) -> CodeModalityOutput {
        let mut conflicts = Vec::new();
        let mut warnings = Vec::new();
        
        // Check for duplicate function names across files
        let mut function_names: HashMap<String, String> = HashMap::new();
        for func in &proposed.new_functions {
            if let Some(existing_file) = function_names.get(&func.name) {
                conflicts.push(Conflict {
                    conflict_type: ConflictType::DuplicateName,
                    description: format!("Function '{}' already defined in {}", func.name, existing_file),
                    existing_location: Some(existing_file.clone()),
                    proposed_location: func.file_path.clone(),
                    resolution: Some("Rename one of the functions or merge them".to_string()),
                });
            } else {
                function_names.insert(func.name.clone(), func.file_path.clone());
            }
        }
        
        // Check for duplicate type names
        let mut type_names: HashMap<String, String> = HashMap::new();
        for typ in &proposed.new_types {
            if let Some(existing_file) = type_names.get(&typ.name) {
                conflicts.push(Conflict {
                    conflict_type: ConflictType::DuplicateName,
                    description: format!("Type '{}' already defined in {}", typ.name, existing_file),
                    existing_location: Some(existing_file.clone()),
                    proposed_location: typ.file_path.clone(),
                    resolution: None,
                });
            } else {
                type_names.insert(typ.name.clone(), typ.file_path.clone());
            }
        }
        
        let report = ConflictReport {
            has_conflicts: !conflicts.is_empty(),
            conflicts,
            warnings,
            suggestions: Vec::new(),
        };
        
        CodeModalityOutput {
            success: true,
            conflicts: Some(report),
            ..Default::default()
        }
    }
    
    async fn trigger_semantic_hook(&self, graph_id: u64, hook_type: ZSEIHookType) -> CodeModalityOutput {
        CodeModalityOutput {
            success: true,
            graph_id: Some(graph_id),
            hook_result: Some(HookResult {
                hook_type,
                success: true,
                edges_added: 0,
                nodes_enriched: 0,
            }),
            ..Default::default()
        }
    }
    
    fn suggest_methodologies(&self, code: &str, language: &str, available_ids: &[u64]) -> CodeModalityOutput {
        let mut suggestions = Vec::new();
        
        // Clean code methodology
        if available_ids.contains(&3) {
            suggestions.push(MethodologySuggestion {
                methodology_id: 3,
                relevance: 0.95,
                reason: "Code modality always benefits from clean code principles".to_string(),
            });
        }
        
        // Security awareness if certain patterns found
        if code.contains("password") || code.contains("secret") || code.contains("token") || 
           code.contains("auth") || code.contains("encrypt") {
            if available_ids.contains(&5) {
                suggestions.push(MethodologySuggestion {
                    methodology_id: 5,
                    relevance: 0.9,
                    reason: "Security-sensitive code detected".to_string(),
                });
            }
        }
        
        // Testing methodology if test-related
        if code.contains("#[test]") || code.contains("@Test") || 
           code.contains("def test_") || code.contains("it(") || code.contains("describe(") {
            if available_ids.contains(&10) {
                suggestions.push(MethodologySuggestion {
                    methodology_id: 10,
                    relevance: 0.9,
                    reason: "Test code detected".to_string(),
                });
            }
        }
        
        // Code review methodology
        if available_ids.contains(&4) {
            suggestions.push(MethodologySuggestion {
                methodology_id: 4,
                relevance: 0.8,
                reason: "Code review best practices applicable".to_string(),
            });
        }
        
        CodeModalityOutput {
            success: true,
            suggested_methodologies: Some(suggestions),
            ..Default::default()
        }
    }
    
    fn generate_graph_id(&self) -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
}

// ============================================================================
// ENTRY POINT
// ============================================================================

#[tokio::main]
async fn main() {
    let input: CodeModalityInput = serde_json::from_reader(std::io::stdin())
        .expect("Failed to parse input");
    
    let pipeline = CodeModalityPipeline::new();
    let output = pipeline.execute(input).await;
    
    serde_json::to_writer(std::io::stdout(), &output)
        .expect("Failed to write output");
}
