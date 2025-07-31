// =============================================================================
// spark-linux/src/hardware_optimization/mod.rs
// SPARK Hardware Optimization - Intelligent Performance Adaptation Engine
// =============================================================================

//! # Hardware Optimization Module for SPARK
//! 
//! This module represents the sophisticated performance orchestration system that enables
//! SPARK to achieve optimal AI processing performance across any hardware configuration,
//! from resource-constrained edge devices to high-performance computing clusters.
//! 
//! ## Architecture Philosophy
//! 
//! Think of hardware optimization like a skilled orchestra conductor who must adapt
//! their conducting style based on the musicians available. Sometimes you have a full
//! symphony orchestra (high-end GPU server), sometimes a small chamber ensemble 
//! (modest laptop), and sometimes a solo pianist (edge device). The conductor's job
//! is to extract the best possible performance from whatever musicians are available.
//! 
//! Similarly, this module analyzes the available hardware "instruments" and orchestrates
//! the most efficient AI processing performance possible, adapting strategies dynamically
//! based on real-time performance feedback and changing resource conditions.

use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime};
use std::sync::{Arc, Mutex};
use std::thread;
use std::fmt;

// Async runtime and concurrency primitives for coordination
use tokio::sync::{RwLock, Semaphore, broadcast};
use tokio::time::{sleep, interval};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use anyhow::Result;
use thiserror::Error;

// Hardware detection and system information
use sysinfo::{System, SystemExt, ProcessorExt, ComponentExt};

// Import shared ecosystem types for coordination
use shared_protocols::{
    ComponentType,
    ResourceRequirements,
    ProtocolError,
};

use shared_security::{
    SecurityError,
    SecurityConfig,
};

// Import SPARK's model and inference types for optimization coordination
use crate::local_models::{ModelInfo, ModelCapabilities};
use crate::inference::{InferenceRequest, InferenceMetrics};

// =============================================================================
// SUBMODULE DECLARATIONS
// Each submodule handles a specific aspect of hardware optimization
// =============================================================================

pub mod acceleration_optimizer;    // Main orchestrator for acceleration strategies
pub mod cuda_optimizer;           // GPU acceleration optimization
pub mod cpu_optimizer;            // CPU-specific performance optimization  
pub mod memory_optimizer;         // Memory usage and allocation optimization
pub mod efficiency_monitor;       // Performance monitoring and feedback system

// =============================================================================
// CORE TYPE RE-EXPORTS
// These are the main types that other SPARK modules will interact with
// =============================================================================

// Primary optimization coordinator - this is the "conductor" of the optimization orchestra
pub use acceleration_optimizer::{
    AccelerationOptimizer,           // Main optimization coordinator
    OptimizationStrategy,            // High-level optimization approach selection
    HardwareProfile,                 // Detected hardware capabilities and characteristics
    PerformanceProfile,              // Runtime performance characteristics and measurements
    ResourceAllocation,              // How resources are allocated across different operations
    OptimizationDecision,            // Specific optimization choices made by the system
    AdaptiveOptimization,           // Dynamic optimization that learns from performance
    OptimizationMetrics,            // Measurements of optimization effectiveness
};

// GPU acceleration capabilities - leverages CUDA, Metal, or other GPU compute frameworks
pub use cuda_optimizer::{
    CUDAOptimizer,                  // GPU acceleration coordinator
    GPUConfiguration,               // GPU setup and capability information
    CUDAKernelOptimization,        // Optimizations for CUDA kernel execution
    GPUMemoryManagement,           // GPU memory allocation and management strategies
    TensorOperationOptimization,   // Optimizations specific to tensor operations on GPU
    GPUPerformanceMetrics,         // GPU-specific performance measurements
    CUDAError,                     // GPU-specific error types and handling
};

// CPU optimization capabilities - maximizes performance on CPU-only systems
pub use cpu_optimizer::{
    CPUOptimizer,                  // CPU performance optimization coordinator
    CPUConfiguration,              // CPU capabilities and optimization settings
    SIMDOptimization,              // Single Instruction Multiple Data optimizations
    CacheOptimization,             // CPU cache-aware optimization strategies
    ThreadingStrategy,             // Multi-threading approach for CPU operations
    VectorizedOperations,          // Vector instruction optimization
    CPUPerformanceMetrics,         // CPU-specific performance measurements
    CPUOptimizationError,          // CPU optimization error handling
};

// Memory optimization capabilities - efficient memory usage across all hardware types
pub use memory_optimizer::{
    MemoryOptimizer,               // Memory usage optimization coordinator
    MemoryAllocationStrategy,      // How memory is allocated and managed
    MemoryPoolManagement,          // Memory pool optimization for reduced allocation overhead
    GarbageCollectionOptimization, // Memory cleanup and garbage collection strategies
    MemoryPressureManagement,     // Handling low-memory situations gracefully
    MemoryUsageMetrics,           // Memory usage tracking and analysis
    MemoryOptimizationError,      // Memory-specific error handling
};

// Performance monitoring and feedback system - the "ears" of the optimization system
pub use efficiency_monitor::{
    EfficiencyMonitor,             // Performance monitoring coordinator
    PerformanceTracker,            // Real-time performance measurement
    OptimizationFeedback,          // Feedback system for optimization decisions
    BenchmarkRunner,               // Automated performance benchmarking
    PerformanceRegression,         // Detection of performance degradation
    EfficiencyAnalyzer,            // Analysis of optimization effectiveness
    MonitoringConfiguration,       // Configuration for performance monitoring
    PerformanceAlert,              // Alerts for performance issues
};

// =============================================================================
// CORE CONFIGURATION TYPES
// These types define how the hardware optimization system is configured
// =============================================================================

/// Comprehensive configuration for the hardware optimization system
/// 
/// This configuration allows fine-tuning of optimization behavior based on
/// deployment requirements, hardware constraints, and performance objectives.
/// Think of this as the "preferences" you would give to an orchestra conductor
/// about how aggressive or conservative they should be with their interpretation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareOptimizationConfig {
    /// Whether to enable acceleration optimization (auto-detection and selection)
    pub acceleration_optimization: bool,
    
    /// Whether to enable CUDA/GPU optimization when available
    pub cuda_optimization: bool,
    
    /// Whether to enable CPU-specific optimizations
    pub cpu_optimization: bool,
    
    /// Whether to enable memory optimization strategies
    pub memory_optimization: bool,
    
    /// Whether to enable continuous efficiency monitoring
    pub efficiency_monitoring: bool,
    
    /// Whether to enable automatic optimization adaptation based on performance feedback
    pub auto_optimization: bool,
    
    /// Hardware device preference - what to prioritize when multiple options are available
    pub preferred_device: PreferredDevice,
    
    /// Optional memory limit - constrain memory usage to this amount (in bytes)
    pub memory_limit: Option<u64>,
    
    /// Optimization aggressiveness - how aggressive should optimizations be?
    pub optimization_aggressiveness: OptimizationAggressiveness,
    
    /// Performance monitoring frequency - how often to check and adjust performance
    pub monitoring_frequency: Duration,
    
    /// Whether to enable performance regression detection
    pub regression_detection: bool,
    
    /// Maximum number of optimization attempts before giving up
    pub max_optimization_attempts: u32,
}

/// Hardware device preference for optimization targeting
/// 
/// This enum represents the user's preference for which type of hardware
/// acceleration to prioritize when multiple options are available.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PreferredDevice {
    /// Prefer CPU execution - good for compatibility and consistent behavior
    CPU,
    
    /// Prefer CUDA/NVIDIA GPU when available - excellent for large model inference
    CUDA,
    
    /// Prefer Metal (Apple GPU) when available - optimal for Apple Silicon devices
    Metal,
    
    /// Prefer OpenCL when available - cross-platform GPU acceleration
    OpenCL,
    
    /// Automatically select the best available option based on benchmarking
    Auto,
    
    /// Use a hybrid approach that balances multiple acceleration methods
    Hybrid,
}

/// Optimization aggressiveness level
/// 
/// This controls how aggressive the optimization system should be in its
/// optimization attempts. More aggressive optimization can yield better
/// performance but may take longer to initialize and be less stable.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OptimizationAggressiveness {
    /// Conservative - prioritize stability and compatibility over maximum performance
    Conservative,
    
    /// Balanced - good balance between performance and stability (recommended)
    Balanced,
    
    /// Aggressive - prioritize performance, may sacrifice some stability
    Aggressive,
    
    /// Maximum - squeeze every bit of performance possible, stability secondary
    Maximum,
}

// =============================================================================
// HARDWARE DETECTION AND PROFILING TYPES
// These types represent the hardware capabilities that the optimization system discovers
// =============================================================================

/// Comprehensive hardware profile detected by the optimization system
/// 
/// This structure represents a complete "snapshot" of the hardware capabilities
/// available to SPARK. Think of it as a detailed inventory of all the musical
/// instruments available to our orchestra conductor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareProfile {
    /// Unique identifier for this hardware profile
    pub profile_id: String,
    
    /// When this profile was created/last updated
    pub created_at: SystemTime,
    
    /// CPU information and capabilities
    pub cpu_info: CPUInfo,
    
    /// GPU information and capabilities (if available)
    pub gpu_info: Option<GPUInfo>,
    
    /// System memory information
    pub memory_info: MemoryInfo,
    
    /// Storage information (affects model loading performance)
    pub storage_info: StorageInfo,
    
    /// Network information (affects distributed processing)
    pub network_info: NetworkInfo,
    
    /// Detected acceleration frameworks available on this system
    pub acceleration_frameworks: Vec<AccelerationFramework>,
    
    /// Overall system performance class
    pub performance_class: PerformanceClass,
    
    /// Benchmark results for this hardware configuration
    pub benchmark_results: Option<BenchmarkResults>,
}

/// CPU information and capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CPUInfo {
    /// CPU architecture (x86_64, arm64, etc.)
    pub architecture: String,
    
    /// CPU model name
    pub model: String,
    
    /// Number of physical CPU cores
    pub physical_cores: u32,
    
    /// Number of logical CPU cores (including hyperthreading)
    pub logical_cores: u32,
    
    /// Base CPU frequency in MHz
    pub base_frequency: u32,
    
    /// Maximum CPU frequency in MHz (boost/turbo)
    pub max_frequency: Option<u32>,
    
    /// CPU cache information
    pub cache_info: CacheInfo,
    
    /// Supported SIMD instruction sets (SSE, AVX, NEON, etc.)
    pub simd_capabilities: Vec<SIMDCapability>,
    
    /// CPU thermal information
    pub thermal_info: ThermalInfo,
}

/// CPU cache hierarchy information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheInfo {
    /// L1 instruction cache size per core (in KB)
    pub l1_instruction: Option<u32>,
    
    /// L1 data cache size per core (in KB)
    pub l1_data: Option<u32>,
    
    /// L2 cache size per core (in KB)
    pub l2_cache: Option<u32>,
    
    /// L3 cache size shared across cores (in KB)
    pub l3_cache: Option<u32>,
}

/// SIMD (Single Instruction Multiple Data) capabilities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SIMDCapability {
    /// Intel/AMD SSE (Streaming SIMD Extensions)
    SSE,
    SSE2,
    SSE3,
    SSSE3,
    SSE41,
    SSE42,
    
    /// Intel/AMD AVX (Advanced Vector Extensions)
    AVX,
    AVX2,
    AVX512,
    
    /// ARM NEON SIMD
    NEON,
    
    /// ARM SVE (Scalable Vector Extension)
    SVE,
    
    /// Custom or unknown SIMD capability
    Custom(String),
}

/// CPU thermal information for performance management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalInfo {
    /// Current CPU temperature (in Celsius)
    pub current_temperature: Option<f32>,
    
    /// Maximum safe operating temperature
    pub max_temperature: Option<f32>,
    
    /// Current thermal throttling status
    pub thermal_throttling: bool,
}

/// GPU information and capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUInfo {
    /// GPU vendor (NVIDIA, AMD, Intel, Apple, etc.)
    pub vendor: String,
    
    /// GPU model name
    pub model: String,
    
    /// GPU memory size in bytes
    pub memory_size: u64,
    
    /// GPU memory bandwidth in GB/s
    pub memory_bandwidth: Option<f32>,
    
    /// Number of compute units/streaming multiprocessors
    pub compute_units: Option<u32>,
    
    /// GPU base frequency in MHz
    pub base_frequency: Option<u32>,
    
    /// GPU boost frequency in MHz
    pub boost_frequency: Option<u32>,
    
    /// Supported compute APIs
    pub compute_apis: Vec<ComputeAPI>,
    
    /// GPU performance tier
    pub performance_tier: GPUPerformanceTier,
    
    /// GPU thermal information
    pub thermal_info: ThermalInfo,
}

/// Supported GPU compute APIs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComputeAPI {
    /// NVIDIA CUDA
    CUDA { version: String },
    
    /// Apple Metal
    Metal { version: String },
    
    /// Cross-platform OpenCL
    OpenCL { version: String },
    
    /// Microsoft DirectML
    DirectML { version: String },
    
    /// AMD ROCm
    ROCm { version: String },
    
    /// Intel oneAPI
    OneAPI { version: String },
    
    /// Vulkan Compute
    Vulkan { version: String },
}

/// GPU performance tier classification
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum GPUPerformanceTier {
    /// Entry-level GPU (integrated graphics, basic discrete)
    Entry,
    
    /// Mid-range GPU (good for moderate AI workloads)
    MidRange,
    
    /// High-end consumer GPU (excellent for AI inference)
    HighEnd,
    
    /// Professional/datacenter GPU (optimal for AI workloads)
    Professional,
    
    /// Specialized AI accelerator (TPU, etc.)
    AIAccelerator,
}

/// System memory information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    /// Total system memory in bytes
    pub total_memory: u64,
    
    /// Currently available memory in bytes
    pub available_memory: u64,
    
    /// Memory type (DDR4, DDR5, LPDDR5, etc.)
    pub memory_type: String,
    
    /// Memory frequency in MHz
    pub frequency: Option<u32>,
    
    /// Number of memory channels
    pub channels: Option<u32>,
    
    /// Memory bandwidth in GB/s
    pub bandwidth: Option<f32>,
    
    /// Whether ECC memory is available
    pub ecc_supported: bool,
}

/// Storage information affecting model loading performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageInfo {
    /// Primary storage type
    pub storage_type: StorageType,
    
    /// Sequential read speed in MB/s
    pub read_speed: Option<f32>,
    
    /// Sequential write speed in MB/s
    pub write_speed: Option<f32>,
    
    /// Random read IOPS
    pub random_read_iops: Option<u32>,
    
    /// Available storage space in bytes
    pub available_space: u64,
}

/// Storage type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StorageType {
    /// Hard Disk Drive (mechanical storage)
    HDD,
    
    /// Solid State Drive (SATA)
    SSD,
    
    /// NVMe SSD (high-performance)
    NVMe,
    
    /// Network-attached storage
    Network,
    
    /// RAM disk (extremely fast, volatile)
    RAMDisk,
    
    /// Unknown or other storage type
    Other(String),
}

/// Network information for distributed processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    /// Network interfaces available
    pub interfaces: Vec<NetworkInterface>,
    
    /// Primary interface bandwidth in Mbps
    pub primary_bandwidth: Option<u32>,
    
    /// Network latency characteristics
    pub latency_profile: LatencyProfile,
}

/// Network interface information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    /// Interface name
    pub name: String,
    
    /// Interface type (Ethernet, WiFi, etc.)
    pub interface_type: String,
    
    /// Maximum bandwidth in Mbps
    pub max_bandwidth: Option<u32>,
    
    /// Whether this interface is currently active
    pub active: bool,
}

/// Network latency characteristics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LatencyProfile {
    /// Very low latency (local/LAN)
    VeryLow,
    
    /// Low latency (fast broadband)
    Low,
    
    /// Medium latency (typical broadband)
    Medium,
    
    /// High latency (slow connection, satellite)
    High,
    
    /// Variable latency (mobile, congested network)
    Variable,
}

/// Available acceleration frameworks detected on the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AccelerationFramework {
    /// NVIDIA CUDA framework
    CUDA { 
        version: String, 
        devices: Vec<String>,
        compute_capability: String,
    },
    
    /// Apple Metal framework
    Metal { 
        version: String, 
        devices: Vec<String>,
    },
    
    /// OpenCL framework
    OpenCL { 
        version: String, 
        devices: Vec<String>,
    },
    
    /// Intel oneDNN/MKL
    IntelMKL { 
        version: String,
    },
    
    /// ARM Compute Library
    ArmComputeLibrary { 
        version: String,
    },
    
    /// CPU-only optimized libraries
    CPUOptimized {
        libraries: Vec<String>,
    },
}

/// Overall system performance classification
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum PerformanceClass {
    /// Low-performance system (edge devices, older hardware)
    Low,
    
    /// Medium-performance system (typical consumer hardware)
    Medium,
    
    /// High-performance system (gaming rigs, workstations)
    High,
    
    /// Very high-performance system (professional workstations)
    VeryHigh,
    
    /// Server/datacenter class system
    Server,
    
    /// Specialized AI hardware
    AIOptimized,
}

// =============================================================================
// PERFORMANCE MEASUREMENT AND BENCHMARKING TYPES
// =============================================================================

/// Comprehensive benchmark results for hardware performance characterization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResults {
    /// When these benchmarks were run
    pub benchmark_timestamp: SystemTime,
    
    /// CPU benchmark results
    pub cpu_benchmarks: CPUBenchmarks,
    
    /// GPU benchmark results (if GPU is available)
    pub gpu_benchmarks: Option<GPUBenchmarks>,
    
    /// Memory benchmark results
    pub memory_benchmarks: MemoryBenchmarks,
    
    /// AI-specific benchmark results
    pub ai_benchmarks: AIBenchmarks,
    
    /// Overall performance score (0.0 to 1.0, normalized)
    pub overall_score: f64,
}

/// CPU-specific benchmark results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CPUBenchmarks {
    /// Single-core performance score
    pub single_core_score: f64,
    
    /// Multi-core performance score  
    pub multi_core_score: f64,
    
    /// SIMD operations performance
    pub simd_performance: f64,
    
    /// Memory access latency (nanoseconds)
    pub memory_latency: f64,
    
    /// Cache performance characteristics
    pub cache_performance: CachePerformance,
}

/// Cache performance measurements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePerformance {
    /// L1 cache hit rate (0.0 to 1.0)
    pub l1_hit_rate: f64,
    
    /// L2 cache hit rate (0.0 to 1.0)
    pub l2_hit_rate: f64,
    
    /// L3 cache hit rate (0.0 to 1.0)
    pub l3_hit_rate: f64,
    
    /// Cache miss penalty (cycles)
    pub miss_penalty: f64,
}

/// GPU-specific benchmark results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUBenchmarks {
    /// Compute performance score
    pub compute_score: f64,
    
    /// Memory bandwidth utilization (0.0 to 1.0)
    pub memory_bandwidth_utilization: f64,
    
    /// Tensor operation performance (TOPS - Tera Operations Per Second)
    pub tensor_performance: f64,
    
    /// GPU memory access latency (nanoseconds)
    pub memory_latency: f64,
    
    /// Kernel launch overhead (microseconds)
    pub kernel_overhead: f64,
}

/// Memory system benchmark results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryBenchmarks {
    /// Sequential read bandwidth (GB/s)
    pub sequential_read_bandwidth: f64,
    
    /// Sequential write bandwidth (GB/s)
    pub sequential_write_bandwidth: f64,
    
    /// Random access latency (nanoseconds)
    pub random_access_latency: f64,
    
    /// Memory allocation/deallocation speed (allocations per second)
    pub allocation_speed: f64,
}

/// AI-specific benchmark results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIBenchmarks {
    /// Small model inference performance (inferences per second)
    pub small_model_performance: f64,
    
    /// Medium model inference performance (inferences per second) 
    pub medium_model_performance: f64,
    
    /// Large model inference performance (inferences per second)
    pub large_model_performance: f64,
    
    /// Model loading speed (MB/s)
    pub model_loading_speed: f64,
    
    /// Context switching overhead (microseconds)
    pub context_switch_overhead: f64,
}

// =============================================================================
// OPTIMIZATION STRATEGY AND DECISION TYPES
// =============================================================================

/// High-level optimization strategy selected by the system
/// 
/// This represents the overall approach the optimization system will take
/// based on the detected hardware and performance requirements.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OptimizationStrategy {
    /// CPU-only optimization strategy
    CPUOnly {
        /// Threading strategy for CPU operations
        threading: ThreadingStrategy,
        /// SIMD instruction usage
        simd_optimization: bool,
        /// Memory optimization focus
        memory_strategy: MemoryStrategy,
    },
    
    /// GPU-accelerated strategy
    GPUAccelerated {
        /// Primary GPU compute API to use
        compute_api: ComputeAPI,
        /// CPU fallback strategy
        cpu_fallback: Box<OptimizationStrategy>,
        /// Memory management between CPU and GPU
        memory_management: GPUMemoryStrategy,
    },
    
    /// Hybrid CPU+GPU strategy
    Hybrid {
        /// How to split work between CPU and GPU
        workload_distribution: WorkloadDistribution,
        /// CPU optimization sub-strategy
        cpu_strategy: Box<OptimizationStrategy>,
        /// GPU optimization configuration
        gpu_strategy: GPUOptimizationConfig,
    },
    
    /// Adaptive strategy that changes based on workload
    Adaptive {
        /// Available optimization strategies to choose from
        available_strategies: Vec<OptimizationStrategy>,
        /// Strategy selection criteria
        selection_criteria: AdaptiveSelectionCriteria,
    },
}

/// Threading strategy for CPU operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThreadingStrategy {
    /// Single-threaded execution
    SingleThread,
    
    /// Fixed number of threads
    FixedThreads(u32),
    
    /// Dynamic thread count based on workload
    Dynamic {
        /// Minimum number of threads
        min_threads: u32,
        /// Maximum number of threads
        max_threads: u32,
    },
    
    /// Use all available CPU cores
    AllCores,
    
    /// Leave some cores free for system operations
    ReservedCores {
        /// Number of cores to reserve for system
        reserved: u32,
    },
}

/// Memory optimization strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryStrategy {
    /// Minimize memory usage (good for constrained systems)
    MinimizeUsage,
    
    /// Balance memory usage and performance
    Balanced,
    
    /// Maximize performance, use more memory if needed
    MaximizePerformance,
    
    /// Preallocate memory pools for consistent performance
    PreallocatedPools,
    
    /// Use memory mapping for large models
    MemoryMapped,
}

/// GPU memory management strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GPUMemoryStrategy {
    /// Conservative GPU memory usage
    Conservative,
    
    /// Aggressive GPU memory usage for maximum performance
    Aggressive,
    
    /// Dynamic allocation based on available memory
    Dynamic,
    
    /// Use unified memory (CPU+GPU shared memory space)
    UnifiedMemory,
    
    /// Explicit memory management with pre-allocated pools
    ExplicitManagement,
}

/// Workload distribution between CPU and GPU
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkloadDistribution {
    /// Fixed percentage split
    FixedSplit {
        /// Percentage of work to do on GPU (0.0 to 1.0)
        gpu_percentage: f64,
    },
    
    /// Dynamic split based on performance characteristics
    Dynamic {
        /// Performance threshold for switching strategies
        performance_threshold: f64,
    },
    
    /// Based on operation type
    OperationBased {
        /// Operations that should prefer GPU
        gpu_preferred_ops: Vec<String>,
        /// Operations that should prefer CPU
        cpu_preferred_ops: Vec<String>,
    },
}

/// GPU optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GPUOptimizationConfig {
    /// GPU compute API to use
    pub compute_api: ComputeAPI,
    
    /// Memory management strategy
    pub memory_strategy: GPUMemoryStrategy,
    
    /// Whether to enable tensor operation fusion
    pub enable_fusion: bool,
    
    /// Batch size optimization
    pub batch_optimization: bool,
    
    /// Kernel optimization level
    pub kernel_optimization: KernelOptimizationLevel,
}

/// Kernel optimization aggressiveness
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum KernelOptimizationLevel {
    /// Basic kernel optimization
    Basic,
    
    /// Standard optimization (good balance)
    Standard,
    
    /// Aggressive optimization (may increase compile time)
    Aggressive,
    
    /// Maximum optimization (longest compile time, best performance)
    Maximum,
}

/// Criteria for adaptive strategy selection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdaptiveSelectionCriteria {
    /// Minimum performance threshold
    pub performance_threshold: f64,
    
    /// Memory usage threshold (0.0 to 1.0 of available memory)
    pub memory_threshold: f64,
    
    /// Power consumption considerations
    pub power_aware: bool,
    
    /// Thermal throttling awareness
    pub thermal_aware: bool,
    
    /// Workload characteristics to consider
    pub workload_characteristics: Vec<WorkloadCharacteristic>,
}

/// Workload characteristics that influence optimization decisions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorkloadCharacteristic {
    /// Compute-intensive operations
    ComputeIntensive,
    
    /// Memory bandwidth limited operations
    MemoryBandwidthLimited,
    
    /// Small batch sizes
    SmallBatch,
    
    /// Large batch sizes
    LargeBatch,
    
    /// Latency-sensitive operations
    LatencySensitive,
    
    /// Throughput-focused operations
    ThroughputFocused,
    
    /// Model size considerations
    ModelSize(ModelSizeCategory),
}

/// Model size categories for optimization
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelSizeCategory {
    /// Small models (< 1B parameters)
    Small,
    
    /// Medium models (1B - 10B parameters)
    Medium,
    
    /// Large models (10B - 100B parameters)
    Large,
    
    /// Very large models (> 100B parameters)
    VeryLarge,
}

// =============================================================================
// RESOURCE ALLOCATION AND MANAGEMENT TYPES
// =============================================================================

/// Resource allocation decisions made by the optimization system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    /// Unique identifier for this resource allocation
    pub allocation_id: String,
    
    /// When this allocation was created
    pub created_at: SystemTime,
    
    /// CPU resource allocation
    pub cpu_allocation: CPUAllocation,
    
    /// GPU resource allocation (if GPU is being used)
    pub gpu_allocation: Option<GPUAllocation>,
    
    /// Memory allocation strategy
    pub memory_allocation: MemoryAllocation,
    
    /// Expected performance characteristics with this allocation
    pub performance_expectations: PerformanceExpectations,
    
    /// Resource allocation constraints
    pub constraints: AllocationConstraints,
}

/// CPU resource allocation details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CPUAllocation {
    /// Number of CPU threads allocated
    pub thread_count: u32,
    
    /// CPU cores to use (specific core affinity)
    pub core_affinity: Option<Vec<u32>>,
    
    /// CPU frequency target (if supported)
    pub frequency_target: Option<u32>,
    
    /// CPU utilization target (0.0 to 1.0)
    pub utilization_target: f64,
    
    /// Whether to enable SIMD optimizations
    pub simd_enabled: bool,
}

/// GPU resource allocation details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUAllocation {
    /// GPU device ID to use
    pub device_id: u32,
    
    /// GPU memory allocation in bytes
    pub memory_allocation: u64,
    
    /// GPU utilization target (0.0 to 1.0)
    pub utilization_target: f64,
    
    /// GPU compute capability requirements
    pub compute_capability: String,
    
    /// GPU memory bandwidth target (GB/s)
    pub bandwidth_target: Option<f64>,
}

/// Memory allocation strategy and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAllocation {
    /// Total memory budget in bytes
    pub total_budget: u64,
    
    /// Memory allocation for model weights
    pub model_memory: u64,
    
    /// Memory allocation for intermediate computations
    pub computation_memory: u64,
    
    /// Memory allocation for input/output buffers
    pub io_memory: u64,
    
    /// Memory allocation for caching
    pub cache_memory: u64,
    
    /// Memory management strategy
    pub management_strategy: MemoryManagementStrategy,
}

/// Memory management strategy details
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryManagementStrategy {
    /// Simple malloc/free based allocation
    SimpleAllocation,
    
    /// Pool-based allocation for reduced fragmentation
    PoolAllocation {
        /// Pool sizes in bytes
        pool_sizes: Vec<u64>,
    },
    
    /// Arena-based allocation
    ArenaAllocation {
        /// Arena size in bytes
        arena_size: u64,
    },
    
    /// Memory-mapped file allocation
    MemoryMapped {
        /// Files to memory map
        mapped_files: Vec<String>,
    },
    
    /// Custom allocation strategy
    Custom {
        /// Strategy name
        strategy_name: String,
        /// Strategy parameters
        parameters: HashMap<String, serde_json::Value>,
    },
}

/// Expected performance characteristics with the current resource allocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceExpectations {
    /// Expected inference latency (milliseconds)
    pub expected_latency: f64,
    
    /// Expected throughput (inferences per second)
    pub expected_throughput: f64,
    
    /// Expected memory usage (bytes)
    pub expected_memory_usage: u64,
    
    /// Expected power consumption (watts)
    pub expected_power_consumption: Option<f64>,
    
    /// Confidence level in these expectations (0.0 to 1.0)
    pub confidence_level: f64,
}

/// Resource allocation constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationConstraints {
    /// Maximum memory usage allowed (bytes)
    pub max_memory: Option<u64>,
    
    /// Maximum CPU utilization allowed (0.0 to 1.0)
    pub max_cpu_utilization: Option<f64>,
    
    /// Maximum GPU utilization allowed (0.0 to 1.0)
    pub max_gpu_utilization: Option<f64>,
    
    /// Maximum power consumption allowed (watts)
    pub max_power_consumption: Option<f64>,
    
    /// Thermal constraints
    pub thermal_constraints: ThermalConstraints,
    
    /// Quality of service requirements
    pub qos_requirements: QoSRequirements,
}

/// Thermal constraints for resource allocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalConstraints {
    /// Maximum CPU temperature (Celsius)
    pub max_cpu_temperature: Option<f32>,
    
    /// Maximum GPU temperature (Celsius)
    pub max_gpu_temperature: Option<f32>,
    
    /// Whether to throttle performance if thermal limits are approached
    pub enable_thermal_throttling: bool,
    
    /// How aggressively to throttle when thermal limits are reached
    pub throttling_aggressiveness: ThrottlingAggressiveness,
}

/// Quality of Service requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QoSRequirements {
    /// Maximum acceptable latency (milliseconds)
    pub max_latency: Option<f64>,
    
    /// Minimum acceptable throughput (operations per second)
    pub min_throughput: Option<f64>,
    
    /// Acceptable performance variance (coefficient of variation)
    pub max_variance: Option<f64>,
    
    /// Priority level for resource allocation
    pub priority: ResourcePriority,
}

/// Thermal throttling aggressiveness levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThrottlingAggressiveness {
    /// Gentle throttling, small performance reductions
    Gentle,
    
    /// Moderate throttling, balanced approach
    Moderate,
    
    /// Aggressive throttling, prioritize thermal safety
    Aggressive,
    
    /// Emergency throttling, maximum temperature protection
    Emergency,
}

/// Resource allocation priority levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResourcePriority {
    /// Low priority, can be preempted
    Low,
    
    /// Normal priority, standard allocation
    Normal,
    
    /// High priority, prefer better resources
    High,
    
    /// Critical priority, best available resources
    Critical,
    
    /// Real-time priority, guaranteed resources
    RealTime,
}

// =============================================================================
// PERFORMANCE MONITORING AND METRICS TYPES
// =============================================================================

/// Comprehensive performance metrics collected during operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationMetrics {
    /// Unique identifier for this metrics collection
    pub metrics_id: String,
    
    /// Time period these metrics cover
    pub time_period: TimePeriod,
    
    /// Performance metrics
    pub performance_metrics: PerformanceMetrics,
    
    /// Resource utilization metrics
    pub resource_metrics: ResourceUtilizationMetrics,
    
    /// Optimization effectiveness metrics
    pub optimization_effectiveness: OptimizationEffectiveness,
    
    /// Quality metrics
    pub quality_metrics: QualityMetrics,
    
    /// Error and warning counts
    pub error_metrics: ErrorMetrics,
}

/// Time period for metrics collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimePeriod {
    /// Start time of the measurement period
    pub start_time: SystemTime,
    
    /// End time of the measurement period
    pub end_time: SystemTime,
    
    /// Duration of the measurement period
    pub duration: Duration,
}

/// Core performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Average inference latency (milliseconds)
    pub avg_latency: f64,
    
    /// 95th percentile latency (milliseconds)
    pub p95_latency: f64,
    
    /// 99th percentile latency (milliseconds)
    pub p99_latency: f64,
    
    /// Maximum observed latency (milliseconds)
    pub max_latency: f64,
    
    /// Average throughput (operations per second)
    pub avg_throughput: f64,
    
    /// Peak throughput (operations per second)
    pub peak_throughput: f64,
    
    /// Number of operations completed
    pub operations_completed: u64,
    
    /// Number of operations failed
    pub operations_failed: u64,
    
    /// Success rate (0.0 to 1.0)
    pub success_rate: f64,
}

/// Resource utilization metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUtilizationMetrics {
    /// CPU utilization metrics
    pub cpu_utilization: CPUUtilizationMetrics,
    
    /// GPU utilization metrics (if GPU is being used)
    pub gpu_utilization: Option<GPUUtilizationMetrics>,
    
    /// Memory utilization metrics
    pub memory_utilization: MemoryUtilizationMetrics,
    
    /// Network utilization metrics
    pub network_utilization: NetworkUtilizationMetrics,
    
    /// Storage utilization metrics
    pub storage_utilization: StorageUtilizationMetrics,
}

/// CPU utilization metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CPUUtilizationMetrics {
    /// Average CPU utilization (0.0 to 1.0)
    pub avg_utilization: f64,
    
    /// Peak CPU utilization (0.0 to 1.0)
    pub peak_utilization: f64,
    
    /// Per-core utilization (0.0 to 1.0 for each core)
    pub per_core_utilization: Vec<f64>,
    
    /// CPU frequency utilization
    pub frequency_utilization: f64,
    
    /// Time spent in different CPU states
    pub cpu_states: CPUStateMetrics,
}

/// CPU state time distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CPUStateMetrics {
    /// Time spent in user mode (0.0 to 1.0)
    pub user_time: f64,
    
    /// Time spent in system mode (0.0 to 1.0)
    pub system_time: f64,
    
    /// Time spent idle (0.0 to 1.0)
    pub idle_time: f64,
    
    /// Time spent waiting for I/O (0.0 to 1.0)
    pub iowait_time: f64,
}

/// GPU utilization metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUUtilizationMetrics {
    /// Average GPU compute utilization (0.0 to 1.0)
    pub avg_compute_utilization: f64,
    
    /// Peak GPU compute utilization (0.0 to 1.0)
    pub peak_compute_utilization: f64,
    
    /// Average GPU memory utilization (0.0 to 1.0)
    pub avg_memory_utilization: f64,
    
    /// Peak GPU memory utilization (0.0 to 1.0)
    pub peak_memory_utilization: f64,
    
    /// GPU memory bandwidth utilization (0.0 to 1.0)
    pub memory_bandwidth_utilization: f64,
    
    /// GPU temperature metrics
    pub temperature_metrics: TemperatureMetrics,
}

/// Memory utilization metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUtilizationMetrics {
    /// Average memory utilization (0.0 to 1.0)
    pub avg_utilization: f64,
    
    /// Peak memory utilization (0.0 to 1.0)
    pub peak_utilization: f64,
    
    /// Memory allocation/deallocation counts
    pub allocation_counts: AllocationCounts,
    
    /// Memory fragmentation level (0.0 to 1.0)
    pub fragmentation_level: f64,
    
    /// Cache hit rates
    pub cache_hit_rates: CacheHitRates,
}

/// Memory allocation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationCounts {
    /// Number of allocations performed
    pub allocations: u64,
    
    /// Number of deallocations performed
    pub deallocations: u64,
    
    /// Number of reallocation operations
    pub reallocations: u64,
    
    /// Total bytes allocated
    pub bytes_allocated: u64,
    
    /// Total bytes deallocated
    pub bytes_deallocated: u64,
}

/// Cache hit rate metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheHitRates {
    /// L1 cache hit rate (0.0 to 1.0)
    pub l1_hit_rate: f64,
    
    /// L2 cache hit rate (0.0 to 1.0)
    pub l2_hit_rate: f64,
    
    /// L3 cache hit rate (0.0 to 1.0)
    pub l3_hit_rate: f64,
    
    /// Application-level cache hit rate (0.0 to 1.0)
    pub app_cache_hit_rate: f64,
}

/// Network utilization metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkUtilizationMetrics {
    /// Bytes sent
    pub bytes_sent: u64,
    
    /// Bytes received
    pub bytes_received: u64,
    
    /// Average bandwidth utilization (0.0 to 1.0)
    pub avg_bandwidth_utilization: f64,
    
    /// Peak bandwidth utilization (0.0 to 1.0)
    pub peak_bandwidth_utilization: f64,
    
    /// Network latency statistics
    pub latency_stats: LatencyStatistics,
}

/// Storage utilization metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageUtilizationMetrics {
    /// Bytes read from storage
    pub bytes_read: u64,
    
    /// Bytes written to storage
    pub bytes_written: u64,
    
    /// Average read bandwidth (MB/s)
    pub avg_read_bandwidth: f64,
    
    /// Average write bandwidth (MB/s)
    pub avg_write_bandwidth: f64,
    
    /// Storage latency statistics
    pub latency_stats: LatencyStatistics,
}

/// Statistical measurements for latency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyStatistics {
    /// Minimum latency observed
    pub min: f64,
    
    /// Maximum latency observed
    pub max: f64,
    
    /// Average latency
    pub avg: f64,
    
    /// Median latency
    pub median: f64,
    
    /// 95th percentile latency
    pub p95: f64,
    
    /// 99th percentile latency
    pub p99: f64,
    
    /// Standard deviation
    pub std_dev: f64,
}

/// Temperature monitoring metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureMetrics {
    /// Current temperature (Celsius)
    pub current: f32,
    
    /// Average temperature over measurement period (Celsius)
    pub average: f32,
    
    /// Maximum temperature observed (Celsius)
    pub maximum: f32,
    
    /// Whether thermal throttling occurred
    pub thermal_throttling_occurred: bool,
    
    /// Time spent in thermal throttling (percentage)
    pub throttling_time_percentage: f64,
}

/// Optimization effectiveness measurements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationEffectiveness {
    /// Performance improvement over baseline (factor, e.g., 1.5 = 50% improvement)
    pub performance_improvement: f64,
    
    /// Resource efficiency improvement (factor)
    pub efficiency_improvement: f64,
    
    /// Memory usage reduction (factor, where < 1.0 means reduction)
    pub memory_usage_factor: f64,
    
    /// Power consumption change (factor, where < 1.0 means reduction)
    pub power_consumption_factor: Option<f64>,
    
    /// Overall optimization score (0.0 to 1.0, higher is better)
    pub overall_score: f64,
    
    /// Time to achieve optimization (milliseconds)
    pub optimization_time: f64,
}

/// Quality metrics for optimization results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    /// Numerical accuracy compared to baseline (0.0 to 1.0)
    pub numerical_accuracy: f64,
    
    /// Result consistency across multiple runs (0.0 to 1.0)
    pub result_consistency: f64,
    
    /// Optimization stability (0.0 to 1.0)
    pub stability_score: f64,
    
    /// Quality of service satisfaction (0.0 to 1.0)
    pub qos_satisfaction: f64,
}

/// Error and warning metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMetrics {
    /// Number of optimization errors
    pub optimization_errors: u32,
    
    /// Number of performance warnings
    pub performance_warnings: u32,
    
    /// Number of resource warnings
    pub resource_warnings: u32,
    
    /// Number of thermal warnings
    pub thermal_warnings: u32,
    
    /// Critical errors that required fallback
    pub critical_errors: u32,
}

// =============================================================================
// ERROR TYPES FOR HARDWARE OPTIMIZATION
// =============================================================================

/// Comprehensive error types for hardware optimization operations
#[derive(Error, Debug)]
pub enum HardwareOptimizationError {
    /// Hardware detection failed
    #[error("Hardware detection failed: {details}")]
    HardwareDetectionFailed { details: String },
    
    /// Optimization strategy selection failed
    #[error("Optimization strategy selection failed: {reason}")]
    StrategySelectionFailed { reason: String },
    
    /// Resource allocation failed
    #[error("Resource allocation failed: {resource} - {details}")]
    ResourceAllocationFailed { resource: String, details: String },
    
    /// Performance benchmarking failed
    #[error("Performance benchmarking failed: {benchmark} - {details}")]
    BenchmarkingFailed { benchmark: String, details: String },
    
    /// Hardware acceleration initialization failed
    #[error("Hardware acceleration initialization failed: {acceleration_type} - {details}")]
    AccelerationInitFailed { acceleration_type: String, details: String },
    
    /// Memory optimization failed
    #[error("Memory optimization failed: {operation} - {details}")]
    MemoryOptimizationFailed { operation: String, details: String },
    
    /// Performance monitoring failed
    #[error("Performance monitoring failed: {component} - {details}")]
    MonitoringFailed { component: String, details: String },
    
    /// Thermal management failed
    #[error("Thermal management failed: {details}")]
    ThermalManagementFailed { details: String },
    
    /// Configuration error
    #[error("Configuration error: {component} - {details}")]
    ConfigurationError { component: String, details: String },
    
    /// Unsupported hardware
    #[error("Unsupported hardware: {hardware_type} - {details}")]
    UnsupportedHardware { hardware_type: String, details: String },
    
    /// Security violation in optimization
    #[error("Security violation in optimization: {details}")]
    SecurityViolation { details: String },
    
    /// Integration error with other SPARK components
    #[error("Integration error: {component} - {details}")]
    IntegrationError { component: String, details: String },
}

// =============================================================================
// CORE TRAITS FOR HARDWARE OPTIMIZATION
// =============================================================================

/// Main trait for hardware optimization coordination
/// 
/// This trait defines the primary interface that other SPARK components use
/// to interact with the hardware optimization system.
pub trait HardwareOptimizer {
    type Config;
    type Error;
    
    /// Initialize the hardware optimization system
    fn initialize(config: Self::Config) -> Result<Self, Self::Error> where Self: Sized;
    
    /// Detect and profile available hardware
    fn detect_hardware(&mut self) -> Result<HardwareProfile, Self::Error>;
    
    /// Select optimal optimization strategy based on hardware and requirements
    fn select_strategy(&mut self, requirements: &OptimizationRequirements) -> Result<OptimizationStrategy, Self::Error>;
    
    /// Allocate resources according to the selected strategy
    fn allocate_resources(&mut self, strategy: &OptimizationStrategy) -> Result<ResourceAllocation, Self::Error>;
    
    /// Apply optimizations and start performance monitoring
    fn apply_optimizations(&mut self, allocation: &ResourceAllocation) -> Result<(), Self::Error>;
    
    /// Get current performance metrics
    fn get_metrics(&self) -> Result<OptimizationMetrics, Self::Error>;
    
    /// Adapt optimization strategy based on performance feedback
    fn adapt_strategy(&mut self, metrics: &OptimizationMetrics) -> Result<OptimizationStrategy, Self::Error>;
    
    /// Clean up resources and shut down optimization system
    fn shutdown(&mut self) -> Result<(), Self::Error>;
}

/// Trait for hardware-specific optimizers
pub trait SpecificHardwareOptimizer {
    type HardwareInfo;
    type OptimizationConfig;
    type PerformanceMetrics;
    type Error;
    
    /// Check if this optimizer can handle the detected hardware
    fn can_optimize(&self, hardware: &Self::HardwareInfo) -> bool;
    
    /// Initialize optimization for specific hardware
    fn initialize_optimization(&mut self, hardware: &Self::HardwareInfo, config: Self::OptimizationConfig) -> Result<(), Self::Error>;
    
    /// Apply hardware-specific optimizations
    fn optimize(&mut self) -> Result<(), Self::Error>;
    
    /// Get hardware-specific performance metrics
    fn get_performance_metrics(&self) -> Result<Self::PerformanceMetrics, Self::Error>;
    
    /// Clean up hardware-specific resources
    fn cleanup(&mut self) -> Result<(), Self::Error>;
}

/// Trait for performance monitoring components
pub trait PerformanceMonitor {
    type Metrics;
    type Alert;
    type Error;
    
    /// Start performance monitoring
    fn start_monitoring(&mut self) -> Result<(), Self::Error>;
    
    /// Get current performance metrics
    fn get_current_metrics(&self) -> Result<Self::Metrics, Self::Error>;
    
    /// Check for performance alerts
    fn check_alerts(&self) -> Result<Vec<Self::Alert>, Self::Error>;
    
    /// Stop performance monitoring
    fn stop_monitoring(&mut self) -> Result<(), Self::Error>;
}

// =============================================================================
// OPTIMIZATION REQUIREMENTS TYPE
// =============================================================================

/// Requirements specification for optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRequirements {
    /// Performance requirements
    pub performance: PerformanceRequirements,
    
    /// Resource constraints
    pub constraints: ResourceConstraints,
    
    /// Quality requirements
    pub quality: QualityRequirements,
    
    /// Workload characteristics
    pub workload: WorkloadCharacteristics,
}

/// Performance requirements specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRequirements {
    /// Target latency (milliseconds)
    pub target_latency: Option<f64>,
    
    /// Target throughput (operations per second)
    pub target_throughput: Option<f64>,
    
    /// Performance priority
    pub priority: PerformancePriority,
}

/// Performance optimization priority
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PerformancePriority {
    /// Prioritize low latency
    Latency,
    
    /// Prioritize high throughput
    Throughput,
    
    /// Balance latency and throughput
    Balanced,
    
    /// Prioritize energy efficiency
    Efficiency,
}

/// Resource constraints for optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConstraints {
    /// Maximum memory usage (bytes)
    pub max_memory: Option<u64>,
    
    /// Maximum CPU usage (0.0 to 1.0)
    pub max_cpu_usage: Option<f64>,
    
    /// Maximum GPU usage (0.0 to 1.0)
    pub max_gpu_usage: Option<f64>,
    
    /// Maximum power consumption (watts)
    pub max_power: Option<f64>,
    
    /// Thermal constraints
    pub thermal: Option<ThermalConstraints>,
}

/// Quality requirements for optimization results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityRequirements {
    /// Required numerical accuracy (0.0 to 1.0)
    pub min_accuracy: f64,
    
    /// Required result consistency (0.0 to 1.0)
    pub min_consistency: f64,
    
    /// Acceptable performance variance
    pub max_variance: f64,
}

/// Workload characteristics for optimization tuning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadCharacteristics {
    /// Expected batch sizes
    pub batch_sizes: Vec<u32>,
    
    /// Model size characteristics
    pub model_sizes: Vec<ModelSizeCategory>,
    
    /// Operation types
    pub operation_types: Vec<String>,
    
    /// Access patterns
    pub access_patterns: Vec<String>,
    
    /// Concurrency level
    pub concurrency_level: u32,
}

// =============================================================================
// RESULT TYPE FOR HARDWARE OPTIMIZATION
// =============================================================================

/// Result type for hardware optimization operations
pub type HardwareOptimizationResult<T> = Result<T, HardwareOptimizationError>;

// =============================================================================
// CONSTANTS AND DEFAULTS
// =============================================================================

/// Default hardware optimization configuration
pub const DEFAULT_OPTIMIZATION_CONFIG: HardwareOptimizationConfig = HardwareOptimizationConfig {
    acceleration_optimization: true,
    cuda_optimization: true,
    cpu_optimization: true,
    memory_optimization: true,
    efficiency_monitoring: true,
    auto_optimization: true,
    preferred_device: PreferredDevice::Auto,
    memory_limit: None,
    optimization_aggressiveness: OptimizationAggressiveness::Balanced,
    monitoring_frequency: Duration::from_secs(5),
    regression_detection: true,
    max_optimization_attempts: 3,
};

/// Version of the hardware optimization module
pub const HARDWARE_OPTIMIZATION_VERSION: &str = "1.0.0";

/// Default benchmark timeout
pub const DEFAULT_BENCHMARK_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum number of optimization retries
pub const MAX_OPTIMIZATION_RETRIES: u32 = 5;

/// Default performance measurement window
pub const DEFAULT_MEASUREMENT_WINDOW: Duration = Duration::from_secs(60);
