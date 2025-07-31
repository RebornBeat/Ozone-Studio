// =============================================================================
// spark-linux/src/local_models/mod.rs
// Local Model Discovery, Loading, and Management for SPARK Universal AI Engine
// =============================================================================

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use std::sync::Arc;
use std::fmt;

// File system and async operations
use tokio::fs::{read_dir, metadata, File};
use tokio::io::{AsyncReadExt, BufReader};
use tokio::sync::{RwLock, Mutex, mpsc, oneshot};
use tokio::time::{sleep, timeout, interval, Instant};

// Serialization and configuration
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use anyhow::Result;
use thiserror::Error;

// Model format handling
use hf_hub::api::tokio::Api as HfApi;
use candle_core::{Device, Tensor, DType};
use candle_nn::VarBuilder;
use candle_transformers::models::phi::{Config as PhiConfig, Phi};

// ONNX runtime integration
use ort::{Environment, SessionBuilder, GraphOptimizationLevel, ExecutionProvider};

// Import shared ecosystem types
use shared_protocols::{
    ComponentType,
    EcosystemIdentity,
    ProtocolError,
};

use shared_security::{
    SecurityError,
    SecureComponent,
    SecurityConfig,
};

// Local model discovery and management
pub mod discovery_engine;
pub mod model_loader;
pub mod selection_engine;
pub mod phi4_integration;
pub mod onnx_integration;
pub mod gguf_integration;
pub mod multi_format_support;

// Re-export all local model types for external use
pub use discovery_engine::{
    DiscoveryEngine,
    ModelDiscovery,
    ModelScanResult,
    DiscoveryConfiguration,
    ModelSource,
    DiscoveryMetrics,
    DiscoveryError,
};

pub use model_loader::{
    ModelLoader,
    LoadingOperation,
    LoadingResult,
    LoadingConfiguration,
    LoadingProgress,
    LoadingMetrics,
    LoadingError,
    ModelInstance,
};

pub use selection_engine::{
    SelectionEngine,
    ModelSelection,
    SelectionCriteria,
    SelectionResult,
    ModelRanking,
    SelectionMetrics,
    SelectionError,
};

pub use phi4_integration::{
    Phi4Integration,
    Phi4Model,
    Phi4Configuration,
    Phi4Capabilities,
    Phi4OptimizationProfile,
    Phi4LoadingStrategy,
    Phi4Error,
};

pub use onnx_integration::{
    ONNXIntegration,
    ONNXModel,
    ONNXConfiguration,
    ONNXCapabilities,
    ONNXOptimizationProfile,
    ONNXLoadingStrategy,
    ONNXError,
};

pub use gguf_integration::{
    GGUFIntegration,
    GGUFModel,
    GGUFConfiguration,
    GGUFCapabilities,
    GGUFOptimizationProfile,
    GGUFLoadingStrategy,
    GGUFError,
};

pub use multi_format_support::{
    MultiFormatSupport,
    FormatDetector,
    FormatConverter,
    UniversalModelInterface,
    FormatCapabilities,
    ConversionMetrics,
    FormatError,
};

// Core model registry types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRegistry {
    // Registry state and metadata
    pub registry_id: String,
    pub registry_version: String,
    pub last_updated: SystemTime,
    pub total_models: usize,
    
    // Model storage and organization
    pub models: HashMap<String, ModelInfo>,
    pub model_categories: HashMap<ModelCategory, Vec<String>>,
    pub format_mapping: HashMap<ModelFormat, Vec<String>>,
    pub capability_index: HashMap<ModelCapability, Vec<String>>,
    
    // Performance and usage tracking
    pub usage_statistics: HashMap<String, ModelUsageStats>,
    pub performance_profiles: HashMap<String, ModelPerformanceProfile>,
    pub loading_history: Vec<ModelLoadingRecord>,
    
    // Configuration and optimization
    pub registry_configuration: RegistryConfiguration,
    pub optimization_preferences: OptimizationPreferences,
}

impl ModelRegistry {
    /// Create a new model registry with comprehensive tracking and management capabilities
    /// This serves as the central catalog for all models available to the SPARK ecosystem
    pub fn new(configuration: RegistryConfiguration) -> Self {
        Self {
            registry_id: Uuid::new_v4().to_string(),
            registry_version: "1.0.0".to_string(),
            last_updated: SystemTime::now(),
            total_models: 0,
            models: HashMap::new(),
            model_categories: HashMap::new(),
            format_mapping: HashMap::new(),
            capability_index: HashMap::new(),
            usage_statistics: HashMap::new(),
            performance_profiles: HashMap::new(),
            loading_history: Vec::new(),
            registry_configuration: configuration,
            optimization_preferences: OptimizationPreferences::default(),
        }
    }
    
    /// Register a new model in the registry with comprehensive metadata and capability analysis
    /// This method performs deep analysis of the model to understand its capabilities and optimization potential
    pub async fn register_model(&mut self, model_info: ModelInfo) -> Result<String, ModelRegistryError> {
        // Validate model information completeness and consistency
        self.validate_model_information(&model_info)?;
        
        // Generate unique model identifier for registry tracking
        let model_id = format!("{}_{}", model_info.name, Uuid::new_v4().to_string()[..8].to_uppercase());
        
        // Analyze model capabilities for intelligent selection and optimization
        let capabilities = self.analyze_model_capabilities(&model_info).await?;
        
        // Create enhanced model information with registry metadata
        let mut enhanced_model_info = model_info.clone();
        enhanced_model_info.model_id = model_id.clone();
        enhanced_model_info.capabilities = capabilities;
        enhanced_model_info.registered_at = SystemTime::now();
        enhanced_model_info.registry_version = self.registry_version.clone();
        
        // Store model in primary registry
        self.models.insert(model_id.clone(), enhanced_model_info.clone());
        
        // Update category indexing for efficient retrieval
        self.update_category_index(&model_id, &enhanced_model_info);
        
        // Update format mapping for format-specific queries
        self.update_format_mapping(&model_id, &enhanced_model_info);
        
        // Update capability index for capability-based selection
        self.update_capability_index(&model_id, &enhanced_model_info);
        
        // Initialize usage statistics tracking
        self.initialize_usage_statistics(&model_id);
        
        // Create initial performance profile
        self.create_initial_performance_profile(&model_id, &enhanced_model_info).await?;
        
        // Update registry metadata
        self.total_models += 1;
        self.last_updated = SystemTime::now();
        
        // Log registration event for auditing and monitoring
        self.log_model_registration(&model_id, &enhanced_model_info);
        
        Ok(model_id)
    }
    
    /// Retrieve model information by ID with comprehensive metadata
    pub fn get_model(&self, model_id: &str) -> Result<ModelInfo, ModelRegistryError> {
        self.models.get(model_id)
            .cloned()
            .ok_or_else(|| ModelRegistryError::ModelNotFound { 
                model_id: model_id.to_string() 
            })
    }
    
    /// Find models matching specific criteria with advanced filtering and ranking
    pub async fn find_models(&self, criteria: ModelSearchCriteria) -> Result<Vec<ModelSearchResult>, ModelRegistryError> {
        let mut matching_models = Vec::new();
        
        // Apply category filtering if specified
        let candidate_models = if let Some(category) = &criteria.category {
            self.get_models_by_category(category)?
        } else {
            self.models.keys().cloned().collect()
        };
        
        // Apply format filtering if specified
        let format_filtered = if let Some(format) = &criteria.format {
            self.filter_by_format(&candidate_models, format)?
        } else {
            candidate_models
        };
        
        // Apply capability filtering if specified
        let capability_filtered = if !criteria.required_capabilities.is_empty() {
            self.filter_by_capabilities(&format_filtered, &criteria.required_capabilities)?
        } else {
            format_filtered
        };
        
        // Apply performance requirements if specified
        let performance_filtered = if let Some(ref perf_req) = criteria.performance_requirements {
            self.filter_by_performance(&capability_filtered, perf_req).await?
        } else {
            capability_filtered
        };
        
        // Create search results with ranking and metadata
        for model_id in performance_filtered {
            if let Some(model_info) = self.models.get(&model_id) {
                let relevance_score = self.calculate_relevance_score(model_info, &criteria);
                let performance_profile = self.performance_profiles.get(&model_id);
                let usage_stats = self.usage_statistics.get(&model_id);
                
                matching_models.push(ModelSearchResult {
                    model_id: model_id.clone(),
                    model_info: model_info.clone(),
                    relevance_score,
                    performance_profile: performance_profile.cloned(),
                    usage_statistics: usage_stats.cloned(),
                    recommendation_reason: self.generate_recommendation_reason(model_info, &criteria),
                });
            }
        }
        
        // Sort by relevance score and performance characteristics
        matching_models.sort_by(|a, b| {
            b.relevance_score.partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // Apply result limiting if specified
        if let Some(limit) = criteria.max_results {
            matching_models.truncate(limit);
        }
        
        Ok(matching_models)
    }
    
    /// Update model usage statistics for intelligent selection and optimization
    pub fn update_usage_statistics(&mut self, model_id: &str, usage_event: ModelUsageEvent) -> Result<(), ModelRegistryError> {
        let stats = self.usage_statistics.get_mut(model_id)
            .ok_or_else(|| ModelRegistryError::ModelNotFound { 
                model_id: model_id.to_string() 
            })?;
        
        // Update usage counters
        stats.total_uses += 1;
        stats.last_used = SystemTime::now();
        
        // Update performance metrics
        if let Some(duration) = usage_event.processing_duration {
            stats.total_processing_time += duration;
            stats.average_processing_time = stats.total_processing_time / stats.total_uses as u32;
        }
        
        // Update quality metrics
        if let Some(quality_score) = usage_event.quality_score {
            stats.quality_scores.push(quality_score);
            stats.average_quality_score = stats.quality_scores.iter().sum::<f64>() / stats.quality_scores.len() as f64;
        }
        
        // Update error tracking
        if usage_event.had_errors {
            stats.error_count += 1;
            stats.error_rate = stats.error_count as f64 / stats.total_uses as f64;
        }
        
        // Update success tracking
        if usage_event.was_successful {
            stats.success_count += 1;
            stats.success_rate = stats.success_count as f64 / stats.total_uses as f64;
        }
        
        // Update context-specific usage
        if let Some(component) = usage_event.requesting_component {
            *stats.usage_by_component.entry(component).or_insert(0) += 1;
        }
        
        Ok(())
    }
    
    /// Get the best model for a specific use case with intelligent selection
    pub async fn get_best_model_for_use_case(&self, use_case: ModelUseCase) -> Result<ModelRecommendation, ModelRegistryError> {
        // Convert use case to search criteria
        let search_criteria = self.use_case_to_search_criteria(&use_case);
        
        // Find matching models
        let candidates = self.find_models(search_criteria).await?;
        
        if candidates.is_empty() {
            return Err(ModelRegistryError::NoSuitableModel { 
                use_case: format!("{:?}", use_case) 
            });
        }
        
        // Select best candidate based on comprehensive evaluation
        let best_candidate = &candidates[0];
        
        // Create detailed recommendation with reasoning
        let recommendation = ModelRecommendation {
            model_id: best_candidate.model_id.clone(),
            model_info: best_candidate.model_info.clone(),
            confidence_score: best_candidate.relevance_score,
            recommendation_reasons: vec![
                best_candidate.recommendation_reason.clone(),
                self.generate_performance_reasoning(&best_candidate.model_id),
                self.generate_usage_reasoning(&best_candidate.model_id),
            ],
            alternative_models: candidates.iter().skip(1).take(3).map(|c| c.model_id.clone()).collect(),
            optimization_suggestions: self.generate_optimization_suggestions(&best_candidate.model_id, &use_case),
            estimated_performance: self.estimate_performance_for_use_case(&best_candidate.model_id, &use_case).await?,
        };
        
        Ok(recommendation)
    }
    
    // Private helper methods for registry management
    
    /// Validate model information for completeness and consistency
    fn validate_model_information(&self, model_info: &ModelInfo) -> Result<(), ModelRegistryError> {
        if model_info.name.is_empty() {
            return Err(ModelRegistryError::InvalidModelInfo { 
                reason: "Model name cannot be empty".to_string() 
            });
        }
        
        if model_info.path.as_os_str().is_empty() {
            return Err(ModelRegistryError::InvalidModelInfo { 
                reason: "Model path cannot be empty".to_string() 
            });
        }
        
        if !model_info.path.exists() {
            return Err(ModelRegistryError::InvalidModelInfo { 
                reason: format!("Model path does not exist: {:?}", model_info.path) 
            });
        }
        
        Ok(())
    }
    
    /// Analyze model capabilities through format-specific inspection
    async fn analyze_model_capabilities(&self, model_info: &ModelInfo) -> Result<Vec<ModelCapability>, ModelRegistryError> {
        let mut capabilities = Vec::new();
        
        // Determine capabilities based on model format and metadata
        match &model_info.format {
            ModelFormat::Phi4 => {
                capabilities.extend(vec![
                    ModelCapability::TextGeneration,
                    ModelCapability::ConversationalAI,
                    ModelCapability::CodeGeneration,
                    ModelCapability::ReasoningSupport,
                ]);
            },
            ModelFormat::ONNX => {
                // Analyze ONNX model graph to determine capabilities
                capabilities.extend(self.analyze_onnx_capabilities(model_info).await?);
            },
            ModelFormat::GGUF => {
                // Analyze GGUF metadata to determine capabilities
                capabilities.extend(self.analyze_gguf_capabilities(model_info).await?);
            },
            _ => {
                // Generic capability analysis for other formats
                capabilities.push(ModelCapability::GeneralProcessing);
            }
        }
        
        // Add inferred capabilities based on model size and architecture
        if model_info.parameter_count > 1_000_000_000 {
            capabilities.push(ModelCapability::LargeScaleProcessing);
        }
        
        if model_info.supports_streaming {
            capabilities.push(ModelCapability::StreamingGeneration);
        }
        
        Ok(capabilities)
    }
    
    /// Update category indexing for efficient model organization
    fn update_category_index(&mut self, model_id: &str, model_info: &ModelInfo) {
        self.model_categories
            .entry(model_info.category.clone())
            .or_insert_with(Vec::new)
            .push(model_id.to_string());
    }
    
    /// Update format mapping for format-specific queries
    fn update_format_mapping(&mut self, model_id: &str, model_info: &ModelInfo) {
        self.format_mapping
            .entry(model_info.format.clone())
            .or_insert_with(Vec::new)
            .push(model_id.to_string());
    }
    
    /// Update capability index for capability-based model selection
    fn update_capability_index(&mut self, model_id: &str, model_info: &ModelInfo) {
        for capability in &model_info.capabilities {
            self.capability_index
                .entry(capability.clone())
                .or_insert_with(Vec::new)
                .push(model_id.to_string());
        }
    }
    
    /// Initialize usage statistics tracking for new model
    fn initialize_usage_statistics(&mut self, model_id: &str) {
        let stats = ModelUsageStats {
            model_id: model_id.to_string(),
            total_uses: 0,
            success_count: 0,
            error_count: 0,
            success_rate: 0.0,
            error_rate: 0.0,
            first_used: None,
            last_used: None,
            total_processing_time: Duration::from_secs(0),
            average_processing_time: Duration::from_secs(0),
            quality_scores: Vec::new(),
            average_quality_score: 0.0,
            usage_by_component: HashMap::new(),
            usage_patterns: Vec::new(),
        };
        
        self.usage_statistics.insert(model_id.to_string(), stats);
    }
    
    /// Create initial performance profile for model
    async fn create_initial_performance_profile(&mut self, model_id: &str, model_info: &ModelInfo) -> Result<(), ModelRegistryError> {
        let profile = ModelPerformanceProfile {
            model_id: model_id.to_string(),
            loading_time: Duration::from_secs(0), // Will be updated on first load
            memory_usage: self.estimate_memory_usage(model_info),
            inference_speed: 0.0, // Will be updated during usage
            throughput: 0.0, // Will be updated during usage
            quality_metrics: QualityMetrics::default(),
            hardware_compatibility: self.analyze_hardware_compatibility(model_info),
            optimization_level: OptimizationLevel::None,
            benchmark_results: Vec::new(),
            performance_trends: Vec::new(),
        };
        
        self.performance_profiles.insert(model_id.to_string(), profile);
        Ok(())
    }
    
    /// Log model registration event for auditing
    fn log_model_registration(&mut self, model_id: &str, model_info: &ModelInfo) {
        let record = ModelLoadingRecord {
            model_id: model_id.to_string(),
            operation: LoadingOperation::Registration,
            timestamp: SystemTime::now(),
            success: true,
            duration: Duration::from_secs(0),
            error_message: None,
            performance_metrics: None,
        };
        
        self.loading_history.push(record);
        
        // Keep history manageable
        if self.loading_history.len() > 1000 {
            self.loading_history.remove(0);
        }
    }
    
    /// Calculate relevance score for model selection
    fn calculate_relevance_score(&self, model_info: &ModelInfo, criteria: &ModelSearchCriteria) -> f64 {
        let mut score = 0.0;
        
        // Category match scoring
        if let Some(ref target_category) = criteria.category {
            if &model_info.category == target_category {
                score += 0.3;
            }
        }
        
        // Capability match scoring
        let capability_match_ratio = if !criteria.required_capabilities.is_empty() {
            let matches = criteria.required_capabilities.iter()
                .filter(|cap| model_info.capabilities.contains(cap))
                .count();
            matches as f64 / criteria.required_capabilities.len() as f64
        } else {
            1.0
        };
        score += capability_match_ratio * 0.4;
        
        // Performance preference scoring
        if let Some(ref perf_req) = criteria.performance_requirements {
            if let Some(usage_stats) = self.usage_statistics.get(&model_info.model_id) {
                if usage_stats.success_rate >= perf_req.min_success_rate {
                    score += 0.2;
                }
                if usage_stats.average_quality_score >= perf_req.min_quality_score {
                    score += 0.1;
                }
            }
        }
        
        score
    }
    
    // Additional helper methods would continue here...
    // For brevity, I'm showing the pattern of comprehensive implementation
}

// Core model information types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    // Basic identification and metadata
    pub model_id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub license: Option<String>,
    
    // File system and format information
    pub path: PathBuf,
    pub format: ModelFormat,
    pub file_size: u64,
    pub checksum: String,
    pub last_modified: SystemTime,
    
    // Model architecture and capabilities
    pub architecture: ModelArchitecture,
    pub parameter_count: u64,
    pub context_length: usize,
    pub vocabulary_size: usize,
    pub supports_streaming: bool,
    pub supports_batching: bool,
    
    // Categorization and capabilities
    pub category: ModelCategory,
    pub capabilities: Vec<ModelCapability>,
    pub supported_tasks: Vec<String>,
    pub languages: Vec<String>,
    
    // Performance and optimization characteristics
    pub memory_requirements: MemoryRequirements,
    pub compute_requirements: ComputeRequirements,
    pub optimization_profiles: Vec<OptimizationProfile>,
    pub hardware_compatibility: HardwareCompatibility,
    
    // Registry and usage metadata
    pub registered_at: SystemTime,
    pub registry_version: String,
    pub usage_priority: UsagePriority,
    pub quality_rating: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ModelFormat {
    Phi4,
    ONNX,
    GGUF,
    SafeTensors,
    PyTorch,
    TensorFlow,
    Huggingface,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelArchitecture {
    Transformer,
    GAN,
    VAE,
    ConvNet,
    RNN,
    LSTM,
    GPT,
    BERT,
    T5,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ModelCategory {
    LanguageModel,
    ConversationalAI,
    CodeGeneration,
    TextClassification,
    SentimentAnalysis,
    Translation,
    Summarization,
    QuestionAnswering,
    ImageGeneration,
    AudioProcessing,
    MultiModal,
    Specialized(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ModelCapability {
    TextGeneration,
    ConversationalAI,
    CodeGeneration,
    ReasoningSupport,
    MathematicalReasoning,
    CreativeWriting,
    TechnicalWriting,
    LanguageTranslation,
    Summarization,
    QuestionAnswering,
    SentimentAnalysis,
    StreamingGeneration,
    BatchProcessing,
    LargeScaleProcessing,
    GeneralProcessing,
    SpecializedDomain(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRequirements {
    pub minimum_ram: u64,
    pub recommended_ram: u64,
    pub vram_required: Option<u64>,
    pub storage_space: u64,
    pub loading_memory_peak: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeRequirements {
    pub minimum_cpu_cores: u32,
    pub recommended_cpu_cores: u32,
    pub gpu_required: bool,
    pub preferred_gpu_memory: Option<u64>,
    pub compute_intensity: ComputeIntensity,
    pub parallel_processing_support: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComputeIntensity {
    Low,
    Medium,
    High,
    VeryHigh,
    Extreme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationProfile {
    pub profile_name: String,
    pub optimization_type: OptimizationType,
    pub target_hardware: HardwareTarget,
    pub performance_gain: f64,
    pub memory_reduction: f64,
    pub quality_impact: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationType {
    Quantization,
    Pruning,
    Distillation,
    GraphOptimization,
    HardwareSpecific,
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HardwareTarget {
    CPU,
    GPU,
    Mobile,
    Edge,
    Server,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareCompatibility {
    pub cpu_support: bool,
    pub gpu_support: bool,
    pub cuda_support: bool,
    pub metal_support: bool,
    pub opencl_support: bool,
    pub vulkan_support: bool,
    pub minimum_compute_capability: Option<String>,
    pub supported_precisions: Vec<DataPrecision>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataPrecision {
    FP32,
    FP16,
    BF16,
    INT8,
    INT4,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UsagePriority {
    Critical,     // Essential for ecosystem operation
    High,         // Preferred for performance
    Medium,       // Standard usage
    Low,          // Backup or specialized use
    Experimental, // Testing and development
}

// Model usage and performance tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsageStats {
    pub model_id: String,
    pub total_uses: u64,
    pub success_count: u64,
    pub error_count: u64,
    pub success_rate: f64,
    pub error_rate: f64,
    pub first_used: Option<SystemTime>,
    pub last_used: Option<SystemTime>,
    pub total_processing_time: Duration,
    pub average_processing_time: Duration,
    pub quality_scores: Vec<f64>,
    pub average_quality_score: f64,
    pub usage_by_component: HashMap<ComponentType, u64>,
    pub usage_patterns: Vec<UsagePattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsagePattern {
    pub pattern_type: UsagePatternType,
    pub frequency: f64,
    pub typical_duration: Duration,
    pub success_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UsagePatternType {
    BurstProcessing,
    ContinuousProcessing,
    InteractiveSession,
    BatchProcessing,
    BackgroundProcessing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPerformanceProfile {
    pub model_id: String,
    pub loading_time: Duration,
    pub memory_usage: u64,
    pub inference_speed: f64, // tokens per second
    pub throughput: f64,      // requests per second
    pub quality_metrics: QualityMetrics,
    pub hardware_compatibility: HardwareCompatibility,
    pub optimization_level: OptimizationLevel,
    pub benchmark_results: Vec<BenchmarkResult>,
    pub performance_trends: Vec<PerformanceTrend>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub accuracy_score: Option<f64>,
    pub perplexity: Option<f64>,
    pub bleu_score: Option<f64>,
    pub rouge_score: Option<f64>,
    pub human_evaluation_score: Option<f64>,
    pub consistency_score: Option<f64>,
}

impl Default for QualityMetrics {
    fn default() -> Self {
        Self {
            accuracy_score: None,
            perplexity: None,
            bleu_score: None,
            rouge_score: None,
            human_evaluation_score: None,
            consistency_score: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationLevel {
    None,
    Basic,
    Standard,
    Advanced,
    Maximum,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub benchmark_name: String,
    pub score: f64,
    pub percentile: f64,
    pub tested_at: SystemTime,
    pub hardware_config: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTrend {
    pub metric_name: String,
    pub values: Vec<f64>,
    pub timestamps: Vec<SystemTime>,
    pub trend_direction: TrendDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    Improving,
    Stable,
    Degrading,
    Volatile,
}

// Model search and selection types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSearchCriteria {
    pub category: Option<ModelCategory>,
    pub format: Option<ModelFormat>,
    pub required_capabilities: Vec<ModelCapability>,
    pub performance_requirements: Option<PerformanceRequirements>,
    pub hardware_constraints: Option<HardwareConstraints>,
    pub quality_threshold: Option<f64>,
    pub max_results: Option<usize>,
    pub sort_by: SortCriteria,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRequirements {
    pub min_success_rate: f64,
    pub max_loading_time: Duration,
    pub max_memory_usage: u64,
    pub min_throughput: f64,
    pub min_quality_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareConstraints {
    pub available_memory: u64,
    pub available_vram: Option<u64>,
    pub cpu_cores: u32,
    pub gpu_available: bool,
    pub compute_budget: ComputeBudget,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComputeBudget {
    Minimal,
    Conservative,
    Moderate,
    Aggressive,
    Unlimited,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortCriteria {
    Relevance,
    Performance,
    Quality,
    UsageFrequency,
    LoadingTime,
    MemoryUsage,
    RecentlyUsed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSearchResult {
    pub model_id: String,
    pub model_info: ModelInfo,
    pub relevance_score: f64,
    pub performance_profile: Option<ModelPerformanceProfile>,
    pub usage_statistics: Option<ModelUsageStats>,
    pub recommendation_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRecommendation {
    pub model_id: String,
    pub model_info: ModelInfo,
    pub confidence_score: f64,
    pub recommendation_reasons: Vec<String>,
    pub alternative_models: Vec<String>,
    pub optimization_suggestions: Vec<String>,
    pub estimated_performance: PerformanceEstimate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceEstimate {
    pub estimated_loading_time: Duration,
    pub estimated_memory_usage: u64,
    pub estimated_throughput: f64,
    pub confidence_interval: (f64, f64),
}

// Model usage and loading events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsageEvent {
    pub event_id: String,
    pub model_id: String,
    pub requesting_component: Option<ComponentType>,
    pub usage_type: UsageType,
    pub processing_duration: Option<Duration>,
    pub quality_score: Option<f64>,
    pub was_successful: bool,
    pub had_errors: bool,
    pub error_details: Option<String>,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UsageType {
    Inference,
    Batch Processing,
    Streaming,
    Fine Tuning,
    Validation,
    Testing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelLoadingRecord {
    pub model_id: String,
    pub operation: LoadingOperation,
    pub timestamp: SystemTime,
    pub success: bool,
    pub duration: Duration,
    pub error_message: Option<String>,
    pub performance_metrics: Option<LoadingPerformanceMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadingOperation {
    Registration,
    Loading,
    Unloading,
    Optimization,
    Validation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadingPerformanceMetrics {
    pub memory_allocated: u64,
    pub initialization_time: Duration,
    pub validation_time: Duration,
    pub optimization_time: Duration,
}

// Use case definitions for intelligent model selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelUseCase {
    ConsciousnessProcessing {
        complexity: ConsciousnessComplexity,
        real_time_required: bool,
    },
    CodeGeneration {
        language: Option<String>,
        complexity: CodeComplexity,
        quality_priority: QualityPriority,
    },
    TextProcessing {
        task_type: TextProcessingTask,
        length: TextLength,
        quality_priority: QualityPriority,
    },
    ConversationalAI {
        conversation_type: ConversationType,
        context_length: ContextLength,
        personality_required: bool,
    },
    GeneralProcessing {
        domain: String,
        performance_priority: PerformancePriority,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsciousnessComplexity {
    BasicAwareness,
    Reflection,
    MetaCognition,
    EthicalReasoning,
    IdentityDevelopment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CodeComplexity {
    Simple,
    Moderate,
    Complex,
    Enterprise,
    Architectural,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QualityPriority {
    Speed,
    Balanced,
    Quality,
    Perfection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TextProcessingTask {
    Generation,
    Summarization,
    Translation,
    Analysis,
    Enhancement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TextLength {
    Short,
    Medium,
    Long,
    Document,
    Book,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConversationType {
    Casual,
    Professional,
    Technical,
    Creative,
    Therapeutic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextLength {
    Short,
    Medium,
    Long,
    Extended,
    Unlimited,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerformancePriority {
    Latency,
    Throughput,
    Quality,
    ResourceEfficiency,
}

// Configuration types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfiguration {
    pub auto_discovery: bool,
    pub performance_monitoring: bool,
    pub usage_tracking: bool,
    pub optimization_enabled: bool,
    pub model_validation: bool,
    pub cache_size_limit: Option<u64>,
    pub cleanup_interval: Duration,
    pub backup_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationPreferences {
    pub prefer_speed: bool,
    pub prefer_quality: bool,
    pub prefer_memory_efficiency: bool,
    pub auto_optimization: bool,
    pub optimization_aggressiveness: OptimizationAggressiveness,
}

impl Default for OptimizationPreferences {
    fn default() -> Self {
        Self {
            prefer_speed: false,
            prefer_quality: true,
            prefer_memory_efficiency: true,
            auto_optimization: true,
            optimization_aggressiveness: OptimizationAggressiveness::Moderate,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationAggressiveness {
    Conservative,
    Moderate,
    Aggressive,
    Maximum,
}

// Error types for model registry operations
#[derive(Error, Debug)]
pub enum ModelRegistryError {
    #[error("Model not found: {model_id}")]
    ModelNotFound { model_id: String },
    
    #[error("Invalid model information: {reason}")]
    InvalidModelInfo { reason: String },
    
    #[error("Model loading failed: {model_id} - {details}")]
    ModelLoadingFailed { model_id: String, details: String },
    
    #[error("No suitable model found for use case: {use_case}")]
    NoSuitableModel { use_case: String },
    
    #[error("Registry operation failed: {operation} - {details}")]
    RegistryOperationFailed { operation: String, details: String },
    
    #[error("Performance analysis failed: {model_id} - {details}")]
    PerformanceAnalysisFailed { model_id: String, details: String },
    
    #[error("Capability analysis failed: {model_id} - {details}")]
    CapabilityAnalysisFailed { model_id: String, details: String },
}

// Result type for registry operations
pub type ModelRegistryResult<T> = Result<T, ModelRegistryError>;

// Constants for model registry configuration
pub const MAX_MODELS_IN_REGISTRY: usize = 1000;
pub const DEFAULT_CACHE_SIZE: u64 = 10 * 1024 * 1024 * 1024; // 10GB
pub const DEFAULT_CLEANUP_INTERVAL: Duration = Duration::from_secs(3600); // 1 hour
pub const MIN_QUALITY_SCORE: f64 = 0.0;
pub const MAX_QUALITY_SCORE: f64 = 1.0;
pub const DEFAULT_SUCCESS_RATE_THRESHOLD: f64 = 0.95;
