// OZONE STUDIO - Centralized Error Handling
// This file implements a comprehensive error handling system for OZONE

use std::fmt;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// The primary error type for OZONE STUDIO operations
#[derive(Error, Debug)]
pub enum OzoneError {
    /// Errors related to analysis operations
    #[error("Analysis error: {0}")]
    AnalysisError(String),

    /// Errors related to embedding operations
    #[error("Embedding error: {0}")]
    EmbeddingError(String),

    /// Errors related to storage operations
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Errors related to reasoning operations
    #[error("Reasoning error: {0}")]
    ReasoningError(String),

    /// Errors related to system integration
    #[error("System error: {0}")]
    SystemError(String),

    /// Errors related to knowledge expansion
    #[error("Expansion error: {0}")]
    ExpansionError(String),

    /// Errors related to overlay operations
    #[error("Overlay error: {0}")]
    OverlayError(String),

    /// Errors related to LLM operations
    #[error("LLM error: {0}")]
    LLMError(String),

    /// Errors related to API operations
    #[error("API error: {0}")]
    ApiError(String),

    /// Errors related to marketplace operations
    #[error("Marketplace error: {0}")]
    MarketplaceError(String),

    /// Input/output errors
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),

    /// JSON serialization/deserialization errors
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Permission errors
    #[error("Permission denied: {0}")]
    PermissionError(String),

    /// File not found errors
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    /// Timeout errors
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Connection errors
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// Resource not available errors
    #[error("Resource not available: {0}")]
    ResourceUnavailable(String),

    /// Invalid argument errors
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    /// Authentication errors
    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    /// Authorization errors
    #[error("Authorization error: {0}")]
    AuthorizationError(String),

    /// Extension errors
    #[error("Extension error: {0}")]
    ExtensionError(String),
}

/// Specialized error for analysis operations
#[derive(Error, Debug)]
pub enum AnalysisError {
    /// Errors related to unsupported file formats
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    /// Errors related to content parsing
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Errors related to missing dependencies
    #[error("Missing dependency: {0}")]
    MissingDependency(String),

    /// Errors related to invalid content
    #[error("Invalid content: {0}")]
    InvalidContent(String),

    /// Errors related to encoding issues
    #[error("Encoding error: {0}")]
    EncodingError(String),
}

/// Specialized error for embedding operations
#[derive(Error, Debug)]
pub enum EmbeddingError {
    /// Errors related to dimension mismatches
    #[error("Dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    /// Errors related to invalid vectors
    #[error("Invalid vector: {0}")]
    InvalidVector(String),

    /// Errors related to normalization issues
    #[error("Normalization error: {0}")]
    NormalizationError(String),

    /// Errors related to embedding generation
    #[error("Generation error: {0}")]
    GenerationError(String),
}

/// Specialized error for storage operations
#[derive(Error, Debug)]
pub enum StorageError {
    /// Errors related to index operations
    #[error("Index error: {0}")]
    IndexError(String),

    /// Errors related to vector operations
    #[error("Vector operation error: {0}")]
    VectorOpError(String),

    /// Errors related to persistence
    #[error("Persistence error: {0}")]
    PersistenceError(String),

    /// Errors related to graph operations
    #[error("Graph error: {0}")]
    GraphError(String),

    /// Errors related to data corruption
    #[error("Data corruption: {0}")]
    DataCorruption(String),

    /// Errors related to quota limits
    #[error("Quota exceeded: {0}")]
    QuotaExceeded(String),
}

/// Specialized error for reasoning operations
#[derive(Error, Debug)]
pub enum ReasoningError {
    /// Errors related to inference operations
    #[error("Inference error: {0}")]
    InferenceError(String),

    /// Errors related to path navigation
    #[error("Path navigation error: {0}")]
    PathNavigationError(String),

    /// Errors related to hypothesis generation
    #[error("Hypothesis error: {0}")]
    HypothesisError(String),

    /// Errors related to decision making
    #[error("Decision error: {0}")]
    DecisionError(String),

    /// Errors related to contradictions
    #[error("Contradiction error: {0}")]
    ContradictionError(String),
}

/// Specialized error for system integration operations
#[derive(Error, Debug)]
pub enum SystemError {
    /// Errors related to OS operations
    #[error("OS error: {0}")]
    OsError(String),

    /// Errors related to device operations
    #[error("Device error: {0}")]
    DeviceError(String),

    /// Errors related to input capture
    #[error("Input error: {0}")]
    InputError(String),

    /// Errors related to file system operations
    #[error("File system error: {0}")]
    FileSystemError(String),

    /// Errors related to network operations
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Errors related to process operations
    #[error("Process error: {0}")]
    ProcessError(String),
}

/// Specialized error for overlay operations
#[derive(Error, Debug)]
pub enum OverlayError {
    /// Errors related to window operations
    #[error("Window error: {0}")]
    WindowError(String),

    /// Errors related to rendering
    #[error("Rendering error: {0}")]
    RenderingError(String),

    /// Errors related to UI elements
    #[error("UI error: {0}")]
    UiError(String),

    /// Errors related to screen capture
    #[error("Screen capture error: {0}")]
    ScreenCaptureError(String),

    /// Errors related to voice processing
    #[error("Voice error: {0}")]
    VoiceError(String),
}

/// Specialized error for expansion operations
#[derive(Error, Debug)]
pub enum ExpansionError {
    /// Errors related to gap detection
    #[error("Gap detection error: {0}")]
    GapDetectionError(String),

    /// Errors related to branch management
    #[error("Branch error: {0}")]
    BranchError(String),

    /// Errors related to exploration
    #[error("Exploration error: {0}")]
    ExplorationError(String),

    /// Errors related to synthesis
    #[error("Synthesis error: {0}")]
    SynthesisError(String),

    /// Errors related to knowledge integration
    #[error("Integration error: {0}")]
    IntegrationError(String),
}

/// Specialized error for LLM operations
#[derive(Error, Debug)]
pub enum LLMError {
    /// Errors related to model loading
    #[error("Model loading error: {0}")]
    ModelLoadingError(String),

    /// Errors related to inference
    #[error("Inference error: {0}")]
    InferenceError(String),

    /// Errors related to tokenization
    #[error("Tokenization error: {0}")]
    TokenizationError(String),

    /// Errors related to prompt generation
    #[error("Prompt error: {0}")]
    PromptError(String),

    /// Errors related to API calls
    #[error("API error: {0}")]
    ApiError(String),

    /// Errors related to model capacity
    #[error("Model capacity exceeded: {0}")]
    CapacityError(String),
}

/// Result type for OZONE operations
pub type OzoneResult<T> = Result<T, OzoneError>;

/// Convert specialized errors to OzoneError
impl From<AnalysisError> for OzoneError {
    fn from(err: AnalysisError) -> Self {
        OzoneError::AnalysisError(err.to_string())
    }
}

impl From<EmbeddingError> for OzoneError {
    fn from(err: EmbeddingError) -> Self {
        OzoneError::EmbeddingError(err.to_string())
    }
}

impl From<StorageError> for OzoneError {
    fn from(err: StorageError) -> Self {
        OzoneError::StorageError(err.to_string())
    }
}

impl From<ReasoningError> for OzoneError {
    fn from(err: ReasoningError) -> Self {
        OzoneError::ReasoningError(err.to_string())
    }
}

impl From<SystemError> for OzoneError {
    fn from(err: SystemError) -> Self {
        OzoneError::SystemError(err.to_string())
    }
}

impl From<OverlayError> for OzoneError {
    fn from(err: OverlayError) -> Self {
        OzoneError::OverlayError(err.to_string())
    }
}

impl From<ExpansionError> for OzoneError {
    fn from(err: ExpansionError) -> Self {
        OzoneError::ExpansionError(err.to_string())
    }
}

impl From<LLMError> for OzoneError {
    fn from(err: LLMError) -> Self {
        OzoneError::LLMError(err.to_string())
    }
}

/// Helper functions for creating specific errors
impl OzoneError {
    /// Create a new analysis error
    pub fn analysis<S: Into<String>>(msg: S) -> Self {
        OzoneError::AnalysisError(msg.into())
    }

    /// Create a new embedding error
    pub fn embedding<S: Into<String>>(msg: S) -> Self {
        OzoneError::EmbeddingError(msg.into())
    }

    /// Create a new storage error
    pub fn storage<S: Into<String>>(msg: S) -> Self {
        OzoneError::StorageError(msg.into())
    }

    /// Create a new reasoning error
    pub fn reasoning<S: Into<String>>(msg: S) -> Self {
        OzoneError::ReasoningError(msg.into())
    }

    /// Create a new system error
    pub fn system<S: Into<String>>(msg: S) -> Self {
        OzoneError::SystemError(msg.into())
    }

    /// Create a new expansion error
    pub fn expansion<S: Into<String>>(msg: S) -> Self {
        OzoneError::ExpansionError(msg.into())
    }

    /// Create a new overlay error
    pub fn overlay<S: Into<String>>(msg: S) -> Self {
        OzoneError::OverlayError(msg.into())
    }

    /// Create a new LLM error
    pub fn llm<S: Into<String>>(msg: S) -> Self {
        OzoneError::LLMError(msg.into())
    }

    /// Create a new configuration error
    pub fn config<S: Into<String>>(msg: S) -> Self {
        OzoneError::ConfigError(msg.into())
    }

    /// Create a new file not found error
    pub fn file_not_found<P: Into<PathBuf>>(path: P) -> Self {
        OzoneError::FileNotFound(path.into())
    }

    /// Create a new timeout error
    pub fn timeout<S: Into<String>>(msg: S) -> Self {
        OzoneError::Timeout(msg.into())
    }

    /// Create a new permission error
    pub fn permission<S: Into<String>>(msg: S) -> Self {
        OzoneError::PermissionError(msg.into())
    }

    /// Check if this error is a timeout
    pub fn is_timeout(&self) -> bool {
        matches!(self, OzoneError::Timeout(_))
    }

    /// Check if this error is a permission error
    pub fn is_permission_error(&self) -> bool {
        matches!(self, OzoneError::PermissionError(_))
    }

    /// Check if this error is a file not found error
    pub fn is_file_not_found(&self) -> bool {
        matches!(self, OzoneError::FileNotFound(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversions() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let ozone_err: OzoneError = io_err.into();
        assert!(matches!(ozone_err, OzoneError::IoError(_)));

        let json_err =
            serde_json::Error::syntax(serde_json::error::ErrorCode::ExpectedSomeValue, 0, 0);
        let ozone_err: OzoneError = json_err.into();
        assert!(matches!(ozone_err, OzoneError::JsonError(_)));
    }

    #[test]
    fn test_helper_functions() {
        let analysis_err = OzoneError::analysis("test error");
        assert!(matches!(analysis_err, OzoneError::AnalysisError(_)));

        let embedding_err = OzoneError::embedding("test error");
        assert!(matches!(embedding_err, OzoneError::EmbeddingError(_)));

        let storage_err = OzoneError::storage("test error");
        assert!(matches!(storage_err, OzoneError::StorageError(_)));

        let reasoning_err = OzoneError::reasoning("test error");
        assert!(matches!(reasoning_err, OzoneError::ReasoningError(_)));

        let system_err = OzoneError::system("test error");
        assert!(matches!(system_err, OzoneError::SystemError(_)));

        let expansion_err = OzoneError::expansion("test error");
        assert!(matches!(expansion_err, OzoneError::ExpansionError(_)));

        let overlay_err = OzoneError::overlay("test error");
        assert!(matches!(overlay_err, OzoneError::OverlayError(_)));

        let llm_err = OzoneError::llm("test error");
        assert!(matches!(llm_err, OzoneError::LLMError(_)));

        let config_err = OzoneError::config("test error");
        assert!(matches!(config_err, OzoneError::ConfigError(_)));

        let file_not_found_err = OzoneError::file_not_found(PathBuf::from("/test/file"));
        assert!(matches!(file_not_found_err, OzoneError::FileNotFound(_)));

        let timeout_err = OzoneError::timeout("test error");
        assert!(matches!(timeout_err, OzoneError::Timeout(_)));

        let permission_err = OzoneError::permission("test error");
        assert!(matches!(permission_err, OzoneError::PermissionError(_)));
    }

    #[test]
    fn test_error_checking() {
        let timeout_err = OzoneError::timeout("test error");
        assert!(timeout_err.is_timeout());

        let permission_err = OzoneError::permission("test error");
        assert!(permission_err.is_permission_error());

        let file_not_found_err = OzoneError::file_not_found(PathBuf::from("/test/file"));
        assert!(file_not_found_err.is_file_not_found());
    }
}
