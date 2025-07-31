// =============================================================================
// bridge-linux/src/document_handling/mod.rs
// Document Handling Module for BRIDGE Human Interface
// Manages document uploads, processing, format detection, and SCRIBE coordination
// =============================================================================

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use std::sync::Arc;
use std::fmt;
use std::io::Cursor;

// File processing and format detection
use tokio::fs::{File, create_dir_all, remove_file};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{RwLock, Mutex, mpsc, oneshot};
use tokio::time::{sleep, timeout, Instant};

// Document format parsing libraries
use pdf_extract::extract_text as extract_pdf_text;
use docx_rust::document::Document as DocxDocument;
use zip::ZipArchive;
use quick_xml::Reader as XmlReader;
use html2text::from_read as html_to_text;
use encoding_rs::Encoding;

// Serialization, validation, and utilities
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use anyhow::{Result, Context};
use thiserror::Error;
use mime::Mime;
use mime_guess::from_path;
use sha2::{Sha256, Digest};
use regex::Regex;

// Import shared protocol and security types
use shared_protocols::{
    EcosystemIdentity,
    ComponentType,
    AIAppCoordinationRequest,
    AIAppCoordinationResponse,
    TaskOrchestrationRequest,
    ExecutionStatus,
    ProtocolError,
};

use shared_security::{
    SecurityError,
    SecureComponent,
    AuthenticationCredentials,
    SecurityConfig,
    UserCertificate,
    EncryptedMessage,
};

// Import NEXUS file system types (for secure file operations)
use nexus_linux::{
    FileOperationRequest,
    FileOperationResponse,
    FileOperationType,
    NexusError,
    SecurityContext,
    AccessLevel,
};

// Import OZONE STUDIO coordination types
use ozone_core::{
    TaskOrchestrationResult,
    AIAppCoordinationRequest as OzoneCoordinationRequest,
    OzoneStudioError,
};

// Document processing submodules
pub mod document_processor;
pub mod upload_handler;
pub mod format_detector;
pub mod content_extractor;
pub mod scribe_coordinator;
pub mod security_validator;
pub mod metadata_analyzer;
pub mod progress_tracker;
pub mod result_presenter;

// Re-export all document handling types
pub use document_processor::{
    DocumentProcessor,
    ProcessingEngine,
    ProcessingPipeline,
    ProcessingStage,
    ProcessingContext,
    ProcessingResult,
    ProcessingError,
};

pub use upload_handler::{
    UploadHandler,
    UploadSession,
    UploadValidator,
    UploadProgress,
    UploadConfiguration,
    UploadResult,
    UploadError,
    ChunkedUpload,
    UploadSecurity,
};

pub use format_detector::{
    FormatDetector,
    DocumentFormat,
    FormatAnalysis,
    FormatValidation,
    MimeTypeDetection,
    SignatureValidation,
    FormatCompatibility,
    DetectionError,
    FormatCapabilities,
};

pub use content_extractor::{
    ContentExtractor,
    ExtractionEngine,
    TextExtraction,
    MetadataExtraction,
    StructureExtraction,
    ExtractionOptions,
    ExtractionResult,
    ExtractionError,
    ContentAnalysis,
};

pub use scribe_coordinator::{
    ScribeCoordinator,
    ScribeIntegration,
    DocumentAnalysisRequest,
    DocumentAnalysisResponse,
    ScribeProcessingOptions,
    AnalysisCoordination,
    ScribeError,
    ProcessingFeedback,
};

pub use security_validator::{
    SecurityValidator,
    FileSecurityScan,
    ThreatDetection,
    ContentSanitization,
    VirusScanning,
    MalwareDetection,
    SecurityReport,
    SecurityLevel,
    ThreatAssessment,
};

pub use metadata_analyzer::{
    MetadataAnalyzer,
    DocumentMetadata,
    FileProperties,
    AuthorInformation,
    CreationContext,
    DocumentStatistics,
    VersionHistory,
    MetadataExtraction,
    PropertyAnalysis,
};

pub use progress_tracker::{
    ProgressTracker,
    ProcessingProgress,
    StageProgress,
    ProgressNotification,
    ProgressCallback,
    CompletionEstimate,
    ProgressMetrics,
    TrackingError,
};

pub use result_presenter::{
    ResultPresenter,
    ProcessingResults,
    DocumentAnalysis,
    AnalysisSummary,
    RecommendedActions,
    UserFeedback,
    PresentationOptions,
    ResultFormat,
    UserExperience,
};

// Core document handling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentHandlingConfig {
    pub upload_handling: UploadHandlingConfig,
    pub format_detection: FormatDetectionConfig,
    pub content_extraction: ContentExtractionConfig,
    pub scribe_coordination: ScribeCoordinationConfig,
    pub security_validation: SecurityValidationConfig,
    pub processing_options: DocumentProcessingOptions,
    pub user_experience: UserExperienceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadHandlingConfig {
    pub max_file_size: u64,                    // Maximum file size in bytes (default: 100MB)
    pub max_concurrent_uploads: usize,          // Maximum simultaneous uploads (default: 10)
    pub supported_formats: Vec<DocumentFormat>, // List of supported document formats
    pub chunked_upload_enabled: bool,           // Enable chunked uploads for large files
    pub chunk_size: u64,                       // Chunk size for chunked uploads (default: 5MB)
    pub upload_timeout: Duration,               // Timeout for individual uploads
    pub temporary_storage_path: String,         // Path for temporary file storage
    pub virus_scanning_enabled: bool,           // Enable virus scanning on uploads
    pub content_type_validation: bool,          // Validate MIME types strictly
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatDetectionConfig {
    pub signature_validation: bool,             // Validate file signatures/magic numbers
    pub mime_type_detection: bool,              // Detect MIME types from content
    pub extension_validation: bool,             // Validate file extensions
    pub format_compatibility_check: bool,       // Check format compatibility with processors
    pub auto_format_conversion: bool,           // Automatically convert compatible formats
    pub detection_confidence_threshold: f64,    // Minimum confidence for format detection
    pub fallback_to_text_extraction: bool,     // Fall back to text extraction if format unknown
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentExtractionConfig {
    pub text_extraction: bool,                  // Extract plain text from documents
    pub metadata_extraction: bool,              // Extract document metadata
    pub structure_extraction: bool,             // Extract document structure (headings, etc.)
    pub image_extraction: bool,                 // Extract embedded images
    pub table_extraction: bool,                 // Extract table data
    pub max_extraction_time: Duration,          // Maximum time for content extraction
    pub preserve_formatting: bool,              // Preserve original formatting where possible
    pub extract_annotations: bool,              // Extract comments and annotations
    pub language_detection: bool,               // Detect document language
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScribeCoordinationConfig {
    pub automatic_analysis: bool,               // Automatically send documents to SCRIBE
    pub analysis_depth: AnalysisDepth,         // Depth of SCRIBE analysis
    pub custom_analysis_prompts: bool,          // Allow custom analysis instructions
    pub collaborative_editing: bool,           // Enable collaborative document editing
    pub version_tracking: bool,                 // Track document versions and changes
    pub scribe_timeout: Duration,              // Timeout for SCRIBE operations
    pub parallel_processing: bool,             // Process multiple documents in parallel
    pub quality_assurance: bool,               // Quality check SCRIBE results
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalysisDepth {
    Quick,          // Basic content analysis and summary
    Standard,       // Comprehensive analysis with insights
    Deep,           // Detailed analysis with recommendations
    Expert,         // Expert-level analysis with cross-references
    Custom,         // User-defined analysis requirements
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityValidationConfig {
    pub virus_scanning: bool,                   // Scan uploads for viruses
    pub malware_detection: bool,                // Detect potential malware
    pub content_sanitization: bool,            // Sanitize potentially dangerous content
    pub file_type_restrictions: Vec<String>,   // Restricted file types
    pub maximum_archive_depth: u32,            // Maximum depth for archive extraction
    pub quarantine_suspicious_files: bool,     // Quarantine suspicious uploads
    pub security_logging: bool,                // Log all security events
    pub threat_reporting: bool,                // Report threats to security system
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentProcessingOptions {
    pub preserve_original: bool,                // Keep original uploaded file
    pub generate_preview: bool,                 // Generate document preview
    pub create_searchable_index: bool,         // Create searchable text index
    pub extract_keywords: bool,                 // Extract keywords and topics
    pub sentiment_analysis: bool,               // Analyze document sentiment
    pub readability_analysis: bool,             // Analyze document readability
    pub duplicate_detection: bool,              // Detect duplicate documents
    pub version_comparison: bool,               // Compare with previous versions
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserExperienceConfig {
    pub real_time_progress: bool,               // Show real-time processing progress
    pub preview_generation: bool,               // Generate instant previews
    pub drag_drop_interface: bool,             // Enable drag-and-drop uploads
    pub batch_processing: bool,                 // Allow batch document processing
    pub processing_notifications: bool,        // Notify users of processing completion
    pub result_customization: bool,            // Allow customization of result presentation
    pub accessibility_features: bool,          // Enable accessibility features
    pub mobile_optimization: bool,             // Optimize for mobile devices
}

// Core document handling types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentUploadRequest {
    pub upload_id: String,
    pub user_id: String,
    pub session_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub content_type: Option<String>,
    pub processing_options: ProcessingOptions,
    pub metadata: DocumentUploadMetadata,
    pub security_context: SecurityContext,
    pub upload_method: UploadMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UploadMethod {
    SingleFile,                     // Upload entire file at once
    ChunkedUpload {                 // Upload file in chunks
        total_chunks: u32,
        chunk_size: u64,
    },
    StreamingUpload,                // Stream file content
    CloudImport {                   // Import from cloud storage
        provider: String,
        file_path: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentUploadMetadata {
    pub title: Option<String>,                  // User-provided document title
    pub description: Option<String>,            // User-provided description
    pub tags: Vec<String>,                      // User-provided tags
    pub category: Option<DocumentCategory>,     // Document category
    pub language: Option<String>,               // Expected document language
    pub author: Option<String>,                 // Document author
    pub confidentiality: ConfidentialityLevel, // Confidentiality level
    pub processing_priority: ProcessingPriority, // Processing priority
    pub custom_instructions: Option<String>,    // Custom processing instructions
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentCategory {
    Report,             // Business reports, research papers
    Legal,              // Legal documents, contracts
    Technical,          // Technical documentation, manuals
    Creative,           // Creative writing, articles
    Educational,        // Educational materials, textbooks
    Personal,           // Personal documents, notes
    Communication,      // Emails, letters, memos
    Financial,          // Financial documents, invoices
    Medical,            // Medical records, reports
    Other(String),      // Custom category
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfidentialityLevel {
    Public,             // No confidentiality restrictions
    Internal,           // Internal use only
    Confidential,       // Confidential information
    Restricted,         // Restricted access
    TopSecret,          // Highest confidentiality
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingPriority {
    Low,                // Background processing
    Normal,             // Standard processing
    High,               // Priority processing
    Urgent,             // Immediate processing
    Critical,           // Critical priority processing
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingOptions {
    pub extract_text: bool,                     // Extract text content
    pub extract_metadata: bool,                 // Extract document metadata
    pub extract_structure: bool,               // Extract document structure
    pub generate_summary: bool,                 // Generate document summary
    pub analyze_sentiment: bool,                // Analyze document sentiment
    pub extract_keywords: bool,                 // Extract keywords and topics
    pub detect_language: bool,                  // Detect document language
    pub scribe_analysis: Option<ScribeAnalysisOptions>, // SCRIBE analysis options
    pub custom_analysis: Option<CustomAnalysisOptions>, // Custom analysis options
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScribeAnalysisOptions {
    pub analysis_type: ScribeAnalysisType,      // Type of SCRIBE analysis
    pub analysis_depth: AnalysisDepth,         // Depth of analysis
    pub custom_prompts: Vec<String>,            // Custom analysis prompts
    pub cross_reference: bool,                  // Cross-reference with other documents
    pub generate_insights: bool,                // Generate insights and recommendations
    pub compare_versions: bool,                 // Compare with previous versions
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScribeAnalysisType {
    ContentAnalysis,        // Analyze content and structure
    StyleAnalysis,          // Analyze writing style and tone
    QualityAssessment,      // Assess document quality
    ComplianceCheck,        // Check compliance with standards
    SentimentAnalysis,      // Analyze sentiment and emotion
    TopicExtraction,        // Extract main topics and themes
    SummaryGeneration,      // Generate document summary
    RecommendationEngine,   // Generate recommendations
    ComparativeAnalysis,    // Compare with similar documents
    Custom(String),         // Custom analysis type
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomAnalysisOptions {
    pub analysis_instructions: String,         // Custom analysis instructions
    pub expected_output_format: OutputFormat,  // Expected output format
    pub analysis_timeout: Duration,            // Timeout for custom analysis
    pub quality_threshold: f64,                // Quality threshold for results
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    PlainText,              // Plain text output
    StructuredText,         // Structured text with formatting
    JSON,                   // JSON-formatted output
    Markdown,               // Markdown-formatted output
    HTML,                   // HTML-formatted output
    XML,                    // XML-formatted output
    Custom(String),         // Custom format specification
}

// Document processing results and responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentProcessingResponse {
    pub processing_id: String,
    pub upload_id: String,
    pub processing_status: ProcessingStatus,
    pub progress: ProcessingProgress,
    pub results: Option<DocumentProcessingResults>,
    pub errors: Vec<ProcessingError>,
    pub warnings: Vec<ProcessingWarning>,
    pub processing_metadata: ProcessingMetadata,
    pub next_steps: Vec<RecommendedAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingStatus {
    Uploaded,               // File uploaded successfully
    Validating,             // Validating file security and format
    Extracting,             // Extracting content and metadata
    Analyzing,              // Analyzing with SCRIBE or other tools
    Completed,              // Processing completed successfully
    Failed,                 // Processing failed
    PartiallyCompleted,     // Partially completed with some errors
    Cancelled,              // Processing cancelled by user
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentProcessingResults {
    pub document_info: DocumentInformation,
    pub extracted_content: ExtractedContent,
    pub analysis_results: Option<AnalysisResults>,
    pub security_assessment: SecurityAssessment,
    pub quality_metrics: QualityMetrics,
    pub recommendations: Vec<ProcessingRecommendation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentInformation {
    pub file_name: String,
    pub file_size: u64,
    pub detected_format: DocumentFormat,
    pub mime_type: String,
    pub creation_date: Option<SystemTime>,
    pub modification_date: Option<SystemTime>,
    pub author: Option<String>,
    pub title: Option<String>,
    pub subject: Option<String>,
    pub keywords: Vec<String>,
    pub page_count: Option<u32>,
    pub word_count: Option<u32>,
    pub character_count: Option<u32>,
    pub language: Option<String>,
    pub document_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedContent {
    pub text_content: String,
    pub structured_content: Option<StructuredContent>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub images: Vec<ExtractedImage>,
    pub tables: Vec<ExtractedTable>,
    pub links: Vec<ExtractedLink>,
    pub annotations: Vec<ExtractedAnnotation>,
    pub formatting: Option<FormattingInformation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredContent {
    pub headings: Vec<Heading>,
    pub paragraphs: Vec<Paragraph>,
    pub lists: Vec<List>,
    pub sections: Vec<Section>,
    pub outline: DocumentOutline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heading {
    pub level: u32,
    pub text: String,
    pub position: ContentPosition,
    pub style: Option<TextStyle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paragraph {
    pub text: String,
    pub position: ContentPosition,
    pub style: Option<TextStyle>,
    pub formatting: Option<ParagraphFormatting>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct List {
    pub list_type: ListType,
    pub items: Vec<ListItem>,
    pub position: ContentPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ListType {
    Bulleted,
    Numbered,
    Definition,
    Checklist,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListItem {
    pub text: String,
    pub level: u32,
    pub subitems: Vec<ListItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    pub title: String,
    pub content: String,
    pub subsections: Vec<Section>,
    pub position: ContentPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentOutline {
    pub sections: Vec<OutlineSection>,
    pub total_sections: u32,
    pub max_depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineSection {
    pub title: String,
    pub level: u32,
    pub page_number: Option<u32>,
    pub subsections: Vec<OutlineSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPosition {
    pub page_number: Option<u32>,
    pub line_number: Option<u32>,
    pub character_offset: u64,
    pub section_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextStyle {
    pub font_family: Option<String>,
    pub font_size: Option<f32>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParagraphFormatting {
    pub alignment: TextAlignment,
    pub indentation: Option<f32>,
    pub line_spacing: Option<f32>,
    pub spacing_before: Option<f32>,
    pub spacing_after: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
    Justified,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedImage {
    pub image_id: String,
    pub alt_text: Option<String>,
    pub caption: Option<String>,
    pub position: ContentPosition,
    pub size: ImageSize,
    pub format: ImageFormat,
    pub embedded_text: Option<String>,    // OCR text if applicable
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSize {
    pub width: u32,
    pub height: u32,
    pub file_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImageFormat {
    JPEG,
    PNG,
    GIF,
    BMP,
    SVG,
    TIFF,
    WebP,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedTable {
    pub table_id: String,
    pub caption: Option<String>,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub position: ContentPosition,
    pub formatting: Option<TableFormatting>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableFormatting {
    pub border_style: Option<String>,
    pub cell_padding: Option<f32>,
    pub column_widths: Vec<f32>,
    pub header_style: Option<TextStyle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedLink {
    pub link_text: String,
    pub url: String,
    pub link_type: LinkType,
    pub position: ContentPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LinkType {
    External,           // External web link
    Internal,           // Internal document reference
    Email,              // Email address
    File,               // File reference
    Anchor,             // Document anchor
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedAnnotation {
    pub annotation_id: String,
    pub annotation_type: AnnotationType,
    pub content: String,
    pub author: Option<String>,
    pub timestamp: Option<SystemTime>,
    pub position: ContentPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnnotationType {
    Comment,            // Comment annotation
    Highlight,          // Highlighted text
    Note,               // Sticky note
    Revision,           // Revision mark
    Bookmark,           // Bookmark
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattingInformation {
    pub page_layout: PageLayout,
    pub styles: Vec<DocumentStyle>,
    pub fonts: Vec<FontInformation>,
    pub colors: Vec<ColorInformation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageLayout {
    pub page_size: PageSize,
    pub margins: Margins,
    pub orientation: PageOrientation,
    pub columns: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSize {
    pub width: f32,
    pub height: f32,
    pub unit: LengthUnit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LengthUnit {
    Inches,
    Centimeters,
    Points,
    Pixels,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Margins {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
    pub unit: LengthUnit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PageOrientation {
    Portrait,
    Landscape,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentStyle {
    pub style_name: String,
    pub style_type: StyleType,
    pub formatting: StyleFormatting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StyleType {
    Paragraph,
    Character,
    Table,
    List,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleFormatting {
    pub font: Option<FontInformation>,
    pub paragraph: Option<ParagraphFormatting>,
    pub border: Option<BorderFormatting>,
    pub background: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontInformation {
    pub font_name: String,
    pub font_size: f32,
    pub font_weight: FontWeight,
    pub font_style: FontStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FontWeight {
    Normal,
    Bold,
    Light,
    Medium,
    Heavy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorInformation {
    pub color_name: Option<String>,
    pub hex_value: String,
    pub rgb_values: (u8, u8, u8),
    pub usage_context: ColorUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColorUsage {
    Text,
    Background,
    Border,
    Highlight,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BorderFormatting {
    pub border_width: f32,
    pub border_style: BorderStyle,
    pub border_color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BorderStyle {
    Solid,
    Dashed,
    Dotted,
    Double,
    None,
}

// Analysis and assessment results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResults {
    pub scribe_analysis: Option<ScribeAnalysisResults>,
    pub content_analysis: ContentAnalysisResults,
    pub quality_assessment: QualityAssessmentResults,
    pub sentiment_analysis: Option<SentimentAnalysisResults>,
    pub topic_analysis: Option<TopicAnalysisResults>,
    pub readability_analysis: Option<ReadabilityAnalysisResults>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScribeAnalysisResults {
    pub analysis_id: String,
    pub analysis_type: ScribeAnalysisType,
    pub results: HashMap<String, serde_json::Value>,
    pub insights: Vec<String>,
    pub recommendations: Vec<String>,
    pub confidence_score: f64,
    pub processing_time: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentAnalysisResults {
    pub main_topics: Vec<Topic>,
    pub key_concepts: Vec<Concept>,
    pub document_structure_quality: f64,
    pub content_coherence_score: f64,
    pub information_density: f64,
    pub unique_content_percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topic {
    pub topic_name: String,
    pub relevance_score: f64,
    pub keywords: Vec<String>,
    pub occurrences: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    pub concept_name: String,
    pub importance_score: f64,
    pub related_concepts: Vec<String>,
    pub definition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityAssessmentResults {
    pub overall_quality_score: f64,
    pub grammar_score: f64,
    pub clarity_score: f64,
    pub consistency_score: f64,
    pub completeness_score: f64,
    pub professionalism_score: f64,
    pub quality_issues: Vec<QualityIssue>,
    pub improvement_suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityIssue {
    pub issue_type: QualityIssueType,
    pub severity: IssueSeverity,
    pub description: String,
    pub location: Option<ContentPosition>,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QualityIssueType {
    Grammar,
    Spelling,
    Punctuation,
    Style,
    Clarity,
    Consistency,
    Structure,
    Formatting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentAnalysisResults {
    pub overall_sentiment: Sentiment,
    pub sentiment_score: f64,        // -1.0 (very negative) to 1.0 (very positive)
    pub confidence: f64,
    pub emotional_indicators: Vec<EmotionalIndicator>,
    pub sentiment_by_section: Vec<SectionSentiment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Sentiment {
    VeryNegative,
    Negative,
    Neutral,
    Positive,
    VeryPositive,
    Mixed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalIndicator {
    pub emotion: String,
    pub intensity: f64,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionSentiment {
    pub section_title: String,
    pub sentiment: Sentiment,
    pub sentiment_score: f64,
    pub position: ContentPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicAnalysisResults {
    pub primary_topics: Vec<Topic>,
    pub secondary_topics: Vec<Topic>,
    pub topic_distribution: HashMap<String, f64>,
    pub topic_relationships: Vec<TopicRelationship>,
    pub domain_classification: DomainClassification,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicRelationship {
    pub topic1: String,
    pub topic2: String,
    pub relationship_type: RelationshipType,
    pub strength: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipType {
    Related,
    Contradictory,
    Supportive,
    Hierarchical,
    Causal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainClassification {
    pub primary_domain: String,
    pub confidence: f64,
    pub secondary_domains: Vec<String>,
    pub interdisciplinary_indicators: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadabilityAnalysisResults {
    pub flesch_reading_ease: f64,
    pub flesch_kincaid_grade: f64,
    pub automated_readability_index: f64,
    pub coleman_liau_index: f64,
    pub gunning_fog_index: f64,
    pub readability_assessment: ReadabilityLevel,
    pub recommendations: Vec<ReadabilityRecommendation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReadabilityLevel {
    VeryEasy,       // 5th grade
    Easy,           // 6th grade
    FairlyEasy,     // 7th grade
    Standard,       // 8th-9th grade
    FairlyDifficult, // 10th-12th grade
    Difficult,      // College level
    VeryDifficult,  // Graduate level
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadabilityRecommendation {
    pub recommendation_type: ReadabilityImprovementType,
    pub description: String,
    pub impact: ImprovementImpact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReadabilityImprovementType {
    ShortenSentences,
    SimplifyVocabulary,
    ReducePassiveVoice,
    ImproveStructure,
    AddTransitions,
    BreakUpParagraphs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImprovementImpact {
    Low,
    Medium,
    High,
    Critical,
}

// Security assessment results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAssessment {
    pub security_level: SecurityLevel,
    pub threat_assessment: ThreatAssessment,
    pub virus_scan_results: VirusScanResults,
    pub content_security_analysis: ContentSecurityAnalysis,
    pub privacy_assessment: PrivacyAssessment,
    pub compliance_check: ComplianceCheckResults,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatAssessment {
    pub overall_threat_level: ThreatLevel,
    pub identified_threats: Vec<IdentifiedThreat>,
    pub risk_factors: Vec<RiskFactor>,
    pub mitigation_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThreatLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentifiedThreat {
    pub threat_type: ThreatType,
    pub severity: ThreatSeverity,
    pub description: String,
    pub location: Option<String>,
    pub recommended_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThreatType {
    Malware,
    Virus,
    Trojan,
    Phishing,
    DataLeak,
    SuspiciousContent,
    PolicyViolation,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThreatSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub factor_type: String,
    pub risk_level: f64,
    pub description: String,
    pub mitigation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirusScanResults {
    pub scan_completed: bool,
    pub scan_timestamp: SystemTime,
    pub scan_engine: String,
    pub threats_found: u32,
    pub scan_duration: Duration,
    pub detailed_results: Vec<VirusDetection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirusDetection {
    pub threat_name: String,
    pub threat_type: ThreatType,
    pub severity: ThreatSeverity,
    pub location: String,
    pub action_taken: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentSecurityAnalysis {
    pub suspicious_patterns: Vec<SuspiciousPattern>,
    pub data_leakage_risk: DataLeakageRisk,
    pub executable_content_detected: bool,
    pub external_references: Vec<ExternalReference>,
    pub embedded_objects: Vec<EmbeddedObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspiciousPattern {
    pub pattern_type: String,
    pub pattern: String,
    pub occurrences: u32,
    pub risk_level: f64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataLeakageRisk {
    pub risk_level: RiskLevel,
    pub sensitive_data_types: Vec<SensitiveDataType>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Minimal,
    Low,
    Medium,
    High,
    Severe,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SensitiveDataType {
    PersonalIdentifiers,    // SSN, ID numbers
    FinancialData,         // Credit cards, bank accounts
    HealthInformation,     // Medical records, health data
    ContactInformation,    // Addresses, phone numbers
    CorporateSecrets,      // Trade secrets, proprietary info
    Credentials,           // Passwords, API keys
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalReference {
    pub reference_type: ReferenceType,
    pub url: String,
    pub description: Option<String>,
    pub security_assessment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReferenceType {
    Hyperlink,
    ImageReference,
    StylesheetReference,
    ScriptReference,
    DataConnection,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedObject {
    pub object_type: String,
    pub size: u64,
    pub security_status: String,
    pub analysis_result: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyAssessment {
    pub privacy_risk_level: RiskLevel,
    pub personal_data_detected: Vec<PersonalDataDetection>,
    pub privacy_compliance: Vec<PrivacyCompliance>,
    pub recommendations: Vec<PrivacyRecommendation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalDataDetection {
    pub data_type: SensitiveDataType,
    pub confidence: f64,
    pub locations: Vec<ContentPosition>,
    pub recommended_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyCompliance {
    pub regulation: String,      // GDPR, CCPA, HIPAA, etc.
    pub compliance_status: ComplianceStatus,
    pub issues: Vec<ComplianceIssue>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComplianceStatus {
    Compliant,
    NonCompliant,
    PartiallyCompliant,
    Unknown,
    NotApplicable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceIssue {
    pub issue_type: String,
    pub severity: IssueSeverity,
    pub description: String,
    pub location: Option<ContentPosition>,
    pub resolution: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyRecommendation {
    pub recommendation_type: String,
    pub priority: Priority,
    pub description: String,
    pub implementation_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Urgent,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceCheckResults {
    pub regulations_checked: Vec<String>,
    pub overall_compliance_score: f64,
    pub compliance_details: Vec<PrivacyCompliance>,
    pub action_items: Vec<ComplianceActionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceActionItem {
    pub action_type: String,
    pub priority: Priority,
    pub description: String,
    pub deadline: Option<SystemTime>,
    pub responsible_party: Option<String>,
}

// Quality metrics and processing metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub processing_quality_score: f64,     // Overall quality of processing
    pub extraction_completeness: f64,      // Completeness of content extraction
    pub format_detection_confidence: f64,  // Confidence in format detection
    pub analysis_confidence: f64,          // Confidence in analysis results
    pub processing_efficiency: f64,        // Efficiency of processing pipeline
    pub error_rate: f64,                   // Rate of processing errors
    pub performance_metrics: PerformanceMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub total_processing_time: Duration,
    pub upload_time: Duration,
    pub validation_time: Duration,
    pub extraction_time: Duration,
    pub analysis_time: Duration,
    pub memory_usage_peak: u64,
    pub cpu_usage_average: f64,
    pub io_operations: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingMetadata {
    pub processing_timestamp: SystemTime,
    pub processing_version: String,
    pub components_used: Vec<String>,
    pub configuration_snapshot: HashMap<String, serde_json::Value>,
    pub processing_path: Vec<ProcessingStep>,
    pub resource_usage: ResourceUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingStep {
    pub step_name: String,
    pub start_time: SystemTime,
    pub end_time: SystemTime,
    pub status: StepStatus,
    pub details: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepStatus {
    Completed,
    Failed,
    Skipped,
    PartiallyCompleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub memory_used: u64,
    pub cpu_time: Duration,
    pub disk_io_bytes: u64,
    pub network_io_bytes: u64,
    pub temporary_files_created: u32,
}

// Recommendations and actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingRecommendation {
    pub recommendation_type: RecommendationType,
    pub priority: Priority,
    pub title: String,
    pub description: String,
    pub rationale: String,
    pub implementation_steps: Vec<String>,
    pub expected_benefit: String,
    pub effort_required: EffortLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationType {
    QualityImprovement,     // Improve document quality
    SecurityEnhancement,    // Enhance document security
    PrivacyProtection,      // Protect privacy information
    ComplianceCorrection,   // Correct compliance issues
    ProcessingOptimization, // Optimize processing settings
    ContentEnhancement,     // Enhance content structure/clarity
    AnalysisRefinement,     // Refine analysis parameters
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffortLevel {
    Minimal,        // Few minutes
    Low,            // Less than an hour
    Medium,         // Few hours
    High,           // Several hours to a day
    Extensive,      // Multiple days
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendedAction {
    pub action_id: String,
    pub action_type: ActionType,
    pub title: String,
    pub description: String,
    pub urgency: ActionUrgency,
    pub estimated_time: Duration,
    pub prerequisites: Vec<String>,
    pub expected_outcome: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    ReviewDocument,         // Review document for issues
    ModifyContent,          // Modify document content
    ApplySecurity,          // Apply security measures
    ChangeSettings,         // Change processing settings
    RequestAnalysis,        // Request additional analysis
    ContactSupport,         // Contact technical support
    ScheduleReprocessing,   // Schedule document reprocessing
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionUrgency {
    Low,            // Can be done when convenient
    Medium,         // Should be done soon
    High,           // Should be done quickly
    Urgent,         // Needs immediate attention
    Critical,       // Requires immediate action
}

// Error and warning types
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum DocumentHandlingError {
    #[error("Upload error: {operation} - {details}")]
    UploadError { operation: String, details: String },
    
    #[error("Format detection error: {file_name} - {details}")]
    FormatDetectionError { file_name: String, details: String },
    
    #[error("Content extraction error: {format} - {details}")]
    ContentExtractionError { format: String, details: String },
    
    #[error("Security validation error: {validation_type} - {details}")]
    SecurityValidationError { validation_type: String, details: String },
    
    #[error("SCRIBE coordination error: {operation} - {details}")]
    ScribeCoordinationError { operation: String, details: String },
    
    #[error("Processing timeout: {operation} exceeded {timeout:?}")]
    ProcessingTimeout { operation: String, timeout: Duration },
    
    #[error("File size limit exceeded: {size} bytes exceeds limit of {limit} bytes")]
    FileSizeLimitExceeded { size: u64, limit: u64 },
    
    #[error("Unsupported format: {format} is not supported")]
    UnsupportedFormat { format: String },
    
    #[error("Corrupted file: {file_name} appears to be corrupted")]
    CorruptedFile { file_name: String },
    
    #[error("Security threat detected: {threat_type} - {details}")]
    SecurityThreatDetected { threat_type: String, details: String },
    
    #[error("Configuration error: {component} - {details}")]
    ConfigurationError { component: String, details: String },
    
    #[error("Resource exhaustion: {resource} - {details}")]
    ResourceExhaustion { resource: String, details: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingWarning {
    pub warning_type: WarningType,
    pub severity: WarningSeverity,
    pub message: String,
    pub location: Option<ContentPosition>,
    pub recommendation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WarningType {
    QualityIssue,           // Document quality issue
    SecurityConcern,        // Security-related concern
    PrivacyIssue,          // Privacy-related issue
    FormatLimitation,      // Format processing limitation
    PerformanceIssue,      // Performance-related issue
    CompatibilityIssue,    // Compatibility issue
    ConfigurationIssue,    // Configuration issue
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WarningSeverity {
    Info,
    Low,
    Medium,
    High,
}

// Core traits for document handling components
pub trait DocumentHandler {
    type Config;
    type Error;
    
    fn initialize(config: Self::Config) -> Result<Self, Self::Error> where Self: Sized;
    fn process_document(&mut self, request: DocumentUploadRequest) -> Result<DocumentProcessingResponse, Self::Error>;
    fn get_processing_status(&self, processing_id: &str) -> Result<ProcessingStatus, Self::Error>;
    fn cancel_processing(&mut self, processing_id: &str) -> Result<(), Self::Error>;
}

pub trait ContentProcessor {
    type Input;
    type Output;
    type Error;
    
    fn can_process(&self, format: &DocumentFormat) -> bool;
    fn process_content(&mut self, input: Self::Input) -> Result<Self::Output, Self::Error>;
    fn get_supported_formats(&self) -> Vec<DocumentFormat>;
    fn estimate_processing_time(&self, input: &Self::Input) -> Duration;
}

pub trait SecurityValidator {
    type Input;
    type Report;
    type Error;
    
    fn validate_security(&mut self, input: Self::Input) -> Result<Self::Report, Self::Error>;
    fn get_threat_level(&self, report: &Self::Report) -> ThreatLevel;
    fn get_recommendations(&self, report: &Self::Report) -> Vec<String>;
}

// Utility functions and constants
pub const MAX_UPLOAD_SIZE_DEFAULT: u64 = 100 * 1024 * 1024; // 100MB
pub const MAX_PROCESSING_TIME_DEFAULT: Duration = Duration::from_secs(300); // 5 minutes
pub const CHUNK_SIZE_DEFAULT: u64 = 5 * 1024 * 1024; // 5MB
pub const MAX_CONCURRENT_UPLOADS_DEFAULT: usize = 10;

// Helper functions for document format detection
pub fn detect_format_from_signature(data: &[u8]) -> Option<DocumentFormat> {
    // PDF signature
    if data.starts_with(b"%PDF") {
        return Some(DocumentFormat::PDF);
    }
    
    // Microsoft Office Open XML (DOCX, XLSX, PPTX)
    if data.starts_with(b"PK\x03\x04") {
        // This is a ZIP file, could be DOCX
        return Some(DocumentFormat::DOCX); // Further validation needed
    }
    
    // RTF signature
    if data.starts_with(b"{\\rtf") {
        return Some(DocumentFormat::RTF);
    }
    
    // HTML signature
    if data.starts_with(b"<!DOCTYPE html") || data.starts_with(b"<html") {
        return Some(DocumentFormat::HTML);
    }
    
    None
}

pub fn estimate_processing_complexity(file_size: u64, format: &DocumentFormat) -> ProcessingComplexity {
    match format {
        DocumentFormat::TXT | DocumentFormat::MD => {
            if file_size < 1024 * 1024 { ProcessingComplexity::Low }
            else if file_size < 10 * 1024 * 1024 { ProcessingComplexity::Medium }
            else { ProcessingComplexity::High }
        },
        DocumentFormat::PDF | DocumentFormat::DOCX => {
            if file_size < 5 * 1024 * 1024 { ProcessingComplexity::Medium }
            else if file_size < 50 * 1024 * 1024 { ProcessingComplexity::High }
            else { ProcessingComplexity::VeryHigh }
        },
        _ => ProcessingComplexity::Medium,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingComplexity {
    Low,
    Medium,
    High,
    VeryHigh,
}

// Result type for document handling operations
pub type DocumentHandlingResult<T> = Result<T, DocumentHandlingError>;

// Integration with ecosystem components
impl From<NexusError> for DocumentHandlingError {
    fn from(error: NexusError) -> Self {
        DocumentHandlingError::ConfigurationError {
            component: "NEXUS".to_string(),
            details: error.to_string(),
        }
    }
}

impl From<OzoneStudioError> for DocumentHandlingError {
    fn from(error: OzoneStudioError) -> Self {
        DocumentHandlingError::ScribeCoordinationError {
            operation: "OZONE_STUDIO_COORDINATION".to_string(),
            details: error.to_string(),
        }
    }
}

impl From<SecurityError> for DocumentHandlingError {
    fn from(error: SecurityError) -> Self {
        DocumentHandlingError::SecurityValidationError {
            validation_type: "SECURITY_CHECK".to_string(),
            details: error.to_string(),
        }
    }
}

// Metrics and monitoring integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentHandlingMetrics {
    pub total_documents_processed: u64,
    pub successful_processing_rate: f64,
    pub average_processing_time: Duration,
    pub format_distribution: HashMap<DocumentFormat, u64>,
    pub error_distribution: HashMap<String, u64>,
    pub security_issues_detected: u64,
    pub quality_score_average: f64,
    pub user_satisfaction_score: f64,
}

impl Default for DocumentHandlingMetrics {
    fn default() -> Self {
        Self {
            total_documents_processed: 0,
            successful_processing_rate: 0.0,
            average_processing_time: Duration::from_secs(0),
            format_distribution: HashMap::new(),
            error_distribution: HashMap::new(),
            security_issues_detected: 0,
            quality_score_average: 0.0,
            user_satisfaction_score: 0.0,
        }
    }
}

// Configuration validation
impl DocumentHandlingConfig {
    pub fn validate(&self) -> Result<(), DocumentHandlingError> {
        if self.upload_handling.max_file_size == 0 {
            return Err(DocumentHandlingError::ConfigurationError {
                component: "upload_handling".to_string(),
                details: "max_file_size cannot be zero".to_string(),
            });
        }
        
        if self.upload_handling.max_concurrent_uploads == 0 {
            return Err(DocumentHandlingError::ConfigurationError {
                component: "upload_handling".to_string(),
                details: "max_concurrent_uploads cannot be zero".to_string(),
            });
        }
        
        if self.format_detection.detection_confidence_threshold < 0.0 || 
           self.format_detection.detection_confidence_threshold > 1.0 {
            return Err(DocumentHandlingError::ConfigurationError {
                component: "format_detection".to_string(),
                details: "detection_confidence_threshold must be between 0.0 and 1.0".to_string(),
            });
        }
        
        Ok(())
    }
}
