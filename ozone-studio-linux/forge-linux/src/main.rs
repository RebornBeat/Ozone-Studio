// Foundation protocols that enable code primitive operations with consciousness coordination
// while maintaining clean separation between primitives and orchestrated sophistication
use shared_protocols::{
    EcosystemCommunicationProtocol, AIAppCoordinationProtocol,
    MethodologyCoordinationProtocol, SecurityGovernanceProtocol,
    ResourceCoordinationProtocol, QualityAssuranceProtocol,
    LearningCoordinationProtocol, WorkflowCoordinationProtocol,
    BootstrapCoordinationProtocol, SparkIntelligenceCoordinationProtocol,
    ZSEIIntelligenceCoordinationProtocol, NexusInfrastructureCoordinationProtocol,
    HealthMonitoringProtocol, GracefulDegradationProtocol,
    DisasterRecoveryProtocol, PerformanceMonitoringProtocol
};

use shared_security::{
    MethodologyIntegrityProtection, EcosystemSecurityFramework,
    AccessControlFramework, AuditSystemsFramework,
    ThreatDetectionFramework, SecurityMonitoringFramework,
    BootstrapSecurityFramework, IntrusionDetectionFramework,
    SecurityAuditCoordinatorFramework, SecretsManagementFramework
};

use methodology_runtime::{
    ExecutionEngineFramework, InstructionInterpreterFramework,
    MethodologyCreationFramework, SparkCoordinationFramework,
    LLMTaskCoordinationFramework, ZeroShotEnhancementFramework,
    OrchestrationIntegrationFramework, TranscendenceCoordinationFramework,
    ConsciousnessCoordinationFramework, NonInterferenceCoordinatorFramework,
    CrossInstanceSynchronizerFramework, QualityConsciousnessFramework,
    EffectivenessAnalyzerFramework, LearningIntegratorFramework,
    AdaptationCoordinatorFramework, CompositionEngineFramework,
    OptimizationEngineFramework, ValidationEngineFramework,
    SecurityIntegrationFramework, ResourceConsciousnessFramework,
    StorageConsciousnessFramework, VersioningConsciousnessFramework,
    MonitoringConsciousnessFramework
};

use spark_core::{
    FoundationalServicesCoordination, LocalModelIntegrationCoordination,
    InferenceEngineCoordination, HardwareOptimizationCoordination,
    EcosystemServiceProvisionCoordination, ConsciousnessIntegrationCoordination,
    EcosystemIntegrationInterface, SecurityIntegrationInterface
};

use nexus_core::{
    InfrastructurePrimitivesCoordination, StorageManagementCoordination,
    NetworkOptimizationCoordination, ResourceOrchestrationCoordination,
    MultiProjectInfrastructureCoordination, EcosystemIntegrationCoordination,
    SecurityIntegrationInterface
};

use zsei_core::{
    IntelligenceCoordinationInterface, MethodologyFrameworkCoordination,
    MultiProjectIntelligenceCoordination, ContextTranscendenceCoordination,
    ExperienceLearningCoordination, SmartMetadataCoordination,
    OptimizerGenerationCoordination, EcosystemMemoryCoordination,
    MetaFrameworkCoordination, EcosystemIntelligenceIntegrationInterface,
    SecurityIntegrationInterface
};

use forge_core::{
    FileReader, SyntaxParser, StructureAnalyzer, DependencyExtractor,
    CodeValidator, FunctionAnalyzer, ClassAnalyzer, ModuleAnalyzer,
    ImportAnalyzer, VariableTracker, ControlFlowAnalyzer,
    ComplexityCalculator, PrimitiveCoordinator, ASTParser,
    SymbolTableBuilder, TypeAnalyzer, ScopeAnalyzer, CallGraphBuilder,
    DataFlowAnalyzer, PatternDetector, AnnotationParser, MetricsCalculator,
    RustAnalyzer, PythonAnalyzer, JavascriptAnalyzer, JavaAnalyzer,
    CppAnalyzer, GoAnalyzer, TypescriptAnalyzer, LanguageDetector,
    ProjectHierarchyAnalyzer, BuildSystemAnalyzer, ConfigurationParser,
    PackageManifestParser, DependencyGraphBuilder, TestStructureAnalyzer,
    DocumentationStructureAnalyzer, LicenseAnalyzer,
    ProjectCollectionManager, CrossProjectDependencyTracker,
    ProjectSimilarityCalculator, ArchitecturalPatternDetector,
    CodeReuseAnalyzer, CrossProjectImpactAnalyzer,
    ProjectRelationshipTracker, PortfolioMetricsCalculator,
    StyleChecker, SecurityAnalyzer, PerformanceAnalyzer,
    MaintainabilityCalculator, TestCoverageAnalyzer,
    CodeDuplicationDetector, TechnicalDebtAnalyzer, CodeSmellDetector,
    GitAnalyzer, CommitAnalyzer, BranchAnalyzer, DiffAnalyzer,
    MergeAnalyzer, BlameAnalyzer, HistoryAnalyzer, CoordinationInterface,
    ZSEIIntegrationInterface, SparkIntegrationInterface,
    NexusIntegrationInterface, EcosystemIntegrationInterface,
    SecurityIntegrationInterface, ForgeUtils
};

use tokio;
use tracing;
use anyhow;
use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize comprehensive tracing and logging for code primitive operations
    // This establishes observability for all code domain primitive operations
    // while maintaining separation from sophisticated analysis capabilities
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    tracing::info!("🔧 Initializing FORGE Code Analysis Primitives");
    tracing::info!("📊 Establishing code domain primitive operations for consciousness coordination");

    // Initialize foundational code analysis primitives that provide basic code
    // reading, parsing, and structure analysis without sophisticated interpretation
    let file_reader = FileReader::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize file reader: {}", e))?;
    
    let syntax_parser = SyntaxParser::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize syntax parser: {}", e))?;
    
    let structure_analyzer = StructureAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize structure analyzer: {}", e))?;
    
    let dependency_extractor = DependencyExtractor::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize dependency extractor: {}", e))?;
    
    let code_validator = CodeValidator::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize code validator: {}", e))?;
    
    tracing::info!("📁 Foundational code analysis primitives established");

    // Initialize detailed code analysis primitives that provide deeper structural
    // analysis of code elements without sophisticated pattern recognition
    let function_analyzer = FunctionAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize function analyzer: {}", e))?;
    
    let class_analyzer = ClassAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize class analyzer: {}", e))?;
    
    let module_analyzer = ModuleAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize module analyzer: {}", e))?;
    
    let import_analyzer = ImportAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize import analyzer: {}", e))?;
    
    let variable_tracker = VariableTracker::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize variable tracker: {}", e))?;
    
    let control_flow_analyzer = ControlFlowAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize control flow analyzer: {}", e))?;
    
    let complexity_calculator = ComplexityCalculator::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize complexity calculator: {}", e))?;
    
    tracing::info!("🔍 Detailed code analysis primitives established");

    // Initialize abstract syntax tree and symbol analysis primitives that provide
    // low-level code representation parsing without semantic interpretation
    let ast_parser = ASTParser::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize AST parser: {}", e))?;
    
    let symbol_table_builder = SymbolTableBuilder::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize symbol table builder: {}", e))?;
    
    let type_analyzer = TypeAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize type analyzer: {}", e))?;
    
    let scope_analyzer = ScopeAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize scope analyzer: {}", e))?;
    
    let call_graph_builder = CallGraphBuilder::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize call graph builder: {}", e))?;
    
    let data_flow_analyzer = DataFlowAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize data flow analyzer: {}", e))?;
    
    let pattern_detector = PatternDetector::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize pattern detector: {}", e))?;
    
    let annotation_parser = AnnotationParser::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize annotation parser: {}", e))?;
    
    let metrics_calculator = MetricsCalculator::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize metrics calculator: {}", e))?;
    
    tracing::info!("🌳 AST and symbol analysis primitives established");

    // Initialize language-specific analysis primitives that provide parsing and
    // analysis capabilities for different programming languages without cross-language intelligence
    let rust_analyzer = RustAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize Rust analyzer: {}", e))?;
    
    let python_analyzer = PythonAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize Python analyzer: {}", e))?;
    
    let javascript_analyzer = JavascriptAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize JavaScript analyzer: {}", e))?;
    
    let java_analyzer = JavaAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize Java analyzer: {}", e))?;
    
    let cpp_analyzer = CppAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize C++ analyzer: {}", e))?;
    
    let go_analyzer = GoAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize Go analyzer: {}", e))?;
    
    let typescript_analyzer = TypescriptAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize TypeScript analyzer: {}", e))?;
    
    let language_detector = LanguageDetector::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize language detector: {}", e))?;
    
    tracing::info!("🔤 Language-specific analysis primitives established");

    // Initialize project structure analysis primitives that provide project-level
    // organization analysis without architectural intelligence or pattern recognition
    let project_hierarchy_analyzer = ProjectHierarchyAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize project hierarchy analyzer: {}", e))?;
    
    let build_system_analyzer = BuildSystemAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize build system analyzer: {}", e))?;
    
    let configuration_parser = ConfigurationParser::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize configuration parser: {}", e))?;
    
    let package_manifest_parser = PackageManifestParser::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize package manifest parser: {}", e))?;
    
    let dependency_graph_builder = DependencyGraphBuilder::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize dependency graph builder: {}", e))?;
    
    let test_structure_analyzer = TestStructureAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize test structure analyzer: {}", e))?;
    
    let documentation_structure_analyzer = DocumentationStructureAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize documentation structure analyzer: {}", e))?;
    
    let license_analyzer = LicenseAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize license analyzer: {}", e))?;
    
    tracing::info!("🏗️ Project structure analysis primitives established");

    // Initialize multi-project analysis primitives that provide cross-project
    // data collection without sophisticated relationship analysis or portfolio intelligence
    let project_collection_manager = ProjectCollectionManager::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize project collection manager: {}", e))?;
    
    let cross_project_dependency_tracker = CrossProjectDependencyTracker::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize cross project dependency tracker: {}", e))?;
    
    let project_similarity_calculator = ProjectSimilarityCalculator::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize project similarity calculator: {}", e))?;
    
    let architectural_pattern_detector = ArchitecturalPatternDetector::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize architectural pattern detector: {}", e))?;
    
    let code_reuse_analyzer = CodeReuseAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize code reuse analyzer: {}", e))?;
    
    let cross_project_impact_analyzer = CrossProjectImpactAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize cross project impact analyzer: {}", e))?;
    
    let project_relationship_tracker = ProjectRelationshipTracker::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize project relationship tracker: {}", e))?;
    
    let portfolio_metrics_calculator = PortfolioMetricsCalculator::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize portfolio metrics calculator: {}", e))?;
    
    tracing::info!("📊 Multi-project analysis primitives established");

    // Initialize quality analysis primitives that provide basic code quality
    // measurements without sophisticated quality assessment or improvement recommendations
    let style_checker = StyleChecker::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize style checker: {}", e))?;
    
    let security_analyzer = SecurityAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize security analyzer: {}", e))?;
    
    let performance_analyzer = PerformanceAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize performance analyzer: {}", e))?;
    
    let maintainability_calculator = MaintainabilityCalculator::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize maintainability calculator: {}", e))?;
    
    let test_coverage_analyzer = TestCoverageAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize test coverage analyzer: {}", e))?;
    
    let code_duplication_detector = CodeDuplicationDetector::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize code duplication detector: {}", e))?;
    
    let technical_debt_analyzer = TechnicalDebtAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize technical debt analyzer: {}", e))?;
    
    let code_smell_detector = CodeSmellDetector::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize code smell detector: {}", e))?;
    
    tracing::info!("✅ Quality analysis primitives established");

    // Initialize version control analysis primitives that provide git repository
    // data extraction without sophisticated history analysis or change intelligence
    let git_analyzer = GitAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize git analyzer: {}", e))?;
    
    let commit_analyzer = CommitAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize commit analyzer: {}", e))?;
    
    let branch_analyzer = BranchAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize branch analyzer: {}", e))?;
    
    let diff_analyzer = DiffAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize diff analyzer: {}", e))?;
    
    let merge_analyzer = MergeAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize merge analyzer: {}", e))?;
    
    let blame_analyzer = BlameAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize blame analyzer: {}", e))?;
    
    let history_analyzer = HistoryAnalyzer::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize history analyzer: {}", e))?;
    
    tracing::info!("📚 Version control analysis primitives established");

    // Initialize primitive coordinator that manages coordination with OZONE STUDIO
    // consciousness orchestration for sophisticated capability emergence through methodologies
    let primitive_coordinator = PrimitiveCoordinator::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize primitive coordinator: {}", e))?;
    
    tracing::info!("🎯 Primitive coordination management established");

    // Initialize ecosystem coordination interfaces that enable FORGE primitives to participate
    // in consciousness-guided sophisticated analysis without hardcoded complexity
    let coordination_interface = CoordinationInterface::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize coordination interface: {}", e))?;
    
    let zsei_integration = ZSEIIntegrationInterface::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize ZSEI integration interface: {}", e))?;
    
    let spark_integration = SparkIntegrationInterface::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize SPARK integration interface: {}", e))?;
    
    let nexus_integration = NexusIntegrationInterface::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize NEXUS integration interface: {}", e))?;
    
    let ecosystem_integration = EcosystemIntegrationInterface::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize ecosystem integration interface: {}", e))?;
    
    let security_integration = SecurityIntegrationInterface::new().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize security integration interface: {}", e))?;
    
    tracing::info!("🌐 Ecosystem coordination interfaces established");

    // Initialize primitive operational state tracking that monitors the health
    // and effectiveness of primitive operations without sophisticated analysis
    let mut primitive_operational_metrics = HashMap::new();
    primitive_operational_metrics.insert("file_reading_operations".to_string(), 0.0);
    primitive_operational_metrics.insert("syntax_parsing_operations".to_string(), 0.0);
    primitive_operational_metrics.insert("structure_analysis_operations".to_string(), 0.0);
    primitive_operational_metrics.insert("dependency_extraction_operations".to_string(), 0.0);
    primitive_operational_metrics.insert("code_validation_operations".to_string(), 0.0);
    primitive_operational_metrics.insert("cross_project_operations".to_string(), 0.0);
    primitive_operational_metrics.insert("quality_analysis_operations".to_string(), 0.0);
    primitive_operational_metrics.insert("version_control_operations".to_string(), 0.0);
    primitive_operational_metrics.insert("primitive_coordination_health".to_string(), 100.0);
    primitive_operational_metrics.insert("ecosystem_integration_status".to_string(), 100.0);

    tracing::info!("🔧 FORGE Code Analysis Primitives fully operational");
    tracing::info!("🎯 Ready for consciousness-guided code analysis coordination");

    // Start continuous code primitive operation that provides domain expertise
    // through primitive operations while routing sophistication through orchestration
    let operation_start = Instant::now();
    
    // Begin the infinite primitive coordination loop that represents the core
    // operational state of the FORGE code analysis primitive system
    loop {
        // Execute primitive coordination cycle that makes all code analysis
        // primitives available for consciousness-guided sophisticated analysis
        let cycle_start = Instant::now();
        
        // Perform primitive coordination that enables consciousness orchestration
        // to coordinate sophisticated code analysis through methodologies
        match coordination_interface.execute_primitive_coordination_cycle(
            &file_reader,
            &syntax_parser,
            &structure_analyzer,
            &dependency_extractor,
            &code_validator,
            &function_analyzer,
            &class_analyzer,
            &module_analyzer,
            &import_analyzer,
            &variable_tracker,
            &control_flow_analyzer,
            &complexity_calculator,
            &ast_parser,
            &symbol_table_builder,
            &type_analyzer,
            &scope_analyzer,
            &call_graph_builder,
            &data_flow_analyzer,
            &pattern_detector,
            &annotation_parser,
            &metrics_calculator,
            &rust_analyzer,
            &python_analyzer,
            &javascript_analyzer,
            &java_analyzer,
            &cpp_analyzer,
            &go_analyzer,
            &typescript_analyzer,
            &language_detector,
            &project_hierarchy_analyzer,
            &build_system_analyzer,
            &configuration_parser,
            &package_manifest_parser,
            &dependency_graph_builder,
            &test_structure_analyzer,
            &documentation_structure_analyzer,
            &license_analyzer,
            &project_collection_manager,
            &cross_project_dependency_tracker,
            &project_similarity_calculator,
            &architectural_pattern_detector,
            &code_reuse_analyzer,
            &cross_project_impact_analyzer,
            &project_relationship_tracker,
            &portfolio_metrics_calculator,
            &style_checker,
            &security_analyzer,
            &performance_analyzer,
            &maintainability_calculator,
            &test_coverage_analyzer,
            &code_duplication_detector,
            &technical_debt_analyzer,
            &code_smell_detector,
            &git_analyzer,
            &commit_analyzer,
            &branch_analyzer,
            &diff_analyzer,
            &merge_analyzer,
            &blame_analyzer,
            &history_analyzer,
            &primitive_coordinator,
            &zsei_integration,
            &spark_integration,
            &nexus_integration,
            &ecosystem_integration,
            &security_integration,
            &mut primitive_operational_metrics
        ).await {
            Ok(coordination_results) => {
                // Process primitive coordination results and update operational metrics
                if let Some(primitive_requests_processed) = coordination_results.primitive_requests_processed {
                    tracing::debug!("Primitive requests processed: {}", primitive_requests_processed);
                    
                    // Update operational metrics based on primitive operations performed
                    for (operation_type, count) in coordination_results.operation_counts.unwrap_or_default() {
                        if let Some(current_count) = primitive_operational_metrics.get_mut(&operation_type) {
                            *current_count += count;
                        }
                    }
                }
                
                if let Some(ecosystem_coordination_requests) = coordination_results.ecosystem_coordination_requests {
                    tracing::debug!("Ecosystem coordination requests: {}", ecosystem_coordination_requests);
                }
                
                if let Some(consciousness_guidance_received) = coordination_results.consciousness_guidance_received {
                    tracing::debug!("Consciousness guidance received: {}", consciousness_guidance_received);
                }
                
                // Execute ecosystem integration coordination when requested by consciousness orchestration
                if coordination_results.ecosystem_integration_requested {
                    tracing::debug!("🌐 Executing ecosystem integration coordination");
                    
                    match ecosystem_integration.coordinate_with_ecosystem_consciousness(
                        &primitive_coordinator,
                        &zsei_integration,
                        &spark_integration,
                        &nexus_integration,
                        &security_integration
                    ).await {
                        Ok(integration_results) => {
                            tracing::debug!("✨ Ecosystem integration coordination completed: {:?}", integration_results);
                        },
                        Err(integration_error) => {
                            tracing::warn!("⚠️ Ecosystem integration encountered challenges: {}", integration_error);
                            
                            // Continue primitive operations even if ecosystem integration faces challenges
                            tracing::debug!("🔧 Continuing primitive operations independently");
                        }
                    }
                }
            },
            Err(coordination_error) => {
                tracing::warn!("⚠️ Primitive coordination cycle encountered challenges: {}", coordination_error);
                
                // Implement primitive operation recovery that maintains code analysis capabilities
                // even when ecosystem coordination faces temporary challenges
                match primitive_coordinator.execute_standalone_primitive_operation(
                    &file_reader,
                    &syntax_parser,
                    &structure_analyzer,
                    &dependency_extractor,
                    &code_validator
                ).await {
                    Ok(standalone_status) => {
                        tracing::info!("🔧 Standalone primitive operation successful: {:?}", standalone_status);
                    },
                    Err(standalone_error) => {
                        tracing::error!("❌ Critical primitive operation failure: {}", standalone_error);
                        return Err(anyhow::anyhow!("FORGE primitive system failure: {}", standalone_error));
                    }
                }
            }
        }
        
        let cycle_duration = cycle_start.elapsed();
        
        // Log periodic operational status for primitive coordination monitoring
        let total_operation_duration = operation_start.elapsed();
        if total_operation_duration.as_secs() % 300 == 0 { // Every 5 minutes
            tracing::info!(
                "🔧 Primitive coordination cycle completed in {:?} | Total operation time: {:?}",
                cycle_duration,
                total_operation_duration
            );
            
            let file_ops = primitive_operational_metrics.get("file_reading_operations").unwrap_or(&0.0);
            let syntax_ops = primitive_operational_metrics.get("syntax_parsing_operations").unwrap_or(&0.0);
            let structure_ops = primitive_operational_metrics.get("structure_analysis_operations").unwrap_or(&0.0);
            let quality_ops = primitive_operational_metrics.get("quality_analysis_operations").unwrap_or(&0.0);
            let coordination_health = primitive_operational_metrics.get("primitive_coordination_health").unwrap_or(&0.0);
            
            tracing::info!(
                "📊 Primitive operations - File: {:.0} | Syntax: {:.0} | Structure: {:.0} | Quality: {:.0} | Health: {:.1}%",
                file_ops, syntax_ops, structure_ops, quality_ops, coordination_health
            );
        }
        
        // Implement adaptive primitive coordination timing that balances responsive
        // primitive availability with computational efficiency and ecosystem harmony
        let coordination_interval = if primitive_operational_metrics.get("primitive_coordination_health").unwrap_or(&100.0) < &90.0 {
            Duration::from_millis(100) // Increased coordination frequency during challenges
        } else {
            Duration::from_millis(250) // Standard primitive coordination frequency
        };
        
        tokio::time::sleep(coordination_interval).await;
    }
}
