//! Data Transfer Protocols for the OZONE STUDIO Ecosystem
//! 
//! This module provides comprehensive data transfer capabilities for the OZONE STUDIO ecosystem,
//! supporting file transfers, streaming data, chunked transfers, compression, encryption, and
//! resume functionality. The protocols are designed to handle everything from small metadata
//! exchanges to massive dataset transfers required for context loop transcendence operations.
//!
//! # Architecture Overview
//!
//! The data transfer system is built around several core concepts:
//! - **Chunked Transfers**: All large data is broken into manageable chunks for efficiency
//! - **Integrity Validation**: Every transfer includes cryptographic verification
//! - **Resume Capability**: Transfers can be paused and resumed without data loss
//! - **Compression Support**: Automatic compression for compatible data types
//! - **Progress Tracking**: Real-time progress reporting for long operations
//! - **Security Integration**: Full integration with the ecosystem security framework
//!
//! # Transfer Types
//!
//! The module supports several transfer patterns:
//! - **File Transfers**: Complete file uploads/downloads with metadata
//! - **Streaming Transfers**: Real-time data streaming for live operations
//! - **Batch Transfers**: Multiple file operations in a single transaction
//! - **Memory Transfers**: Direct memory-to-memory transfers for efficiency
//! - **Incremental Transfers**: Delta-based transfers for synchronized data

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::sync::Arc;
use std::fmt;
use std::path::PathBuf;

// Async runtime and concurrency primitives
use tokio::sync::{RwLock, Mutex, mpsc, oneshot};
use tokio::time::{sleep, timeout, interval, Instant};
use tokio::io::{AsyncRead, AsyncWrite};

// Serialization and data handling
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use bytes::{Bytes, BytesMut, Buf, BufMut};

// Cryptographic operations for integrity and security
use sha2::{Sha256, Digest};
use blake3::Hasher as Blake3Hasher;

// Compression support
use flate2::{Compression, write::GzEncoder, read::GzDecoder};
use std::io::{Read, Write};

// Error handling
use thiserror::Error;
use anyhow::{Result, Context};

// Import security types from our shared security crate
use crate::SecurityContext;

// ============================================================================
// Core Data Transfer Message Types
// ============================================================================

/// Primary message type for all data transfer operations in the ecosystem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataTransferMessage {
    /// Unique identifier for this transfer message
    pub message_id: String,
    
    /// Type of data transfer operation being performed
    pub message_type: DataTransferMessageType,
    
    /// Source component initiating the transfer
    pub source_component: String,
    
    /// Destination component receiving the transfer
    pub destination_component: String,
    
    /// Security context for authentication and authorization
    pub security_context: Option<SecurityContext>,
    
    /// Timestamp when the message was created
    pub timestamp: SystemTime,
    
    /// Optional correlation ID for linking related messages
    pub correlation_id: Option<String>,
    
    /// Priority level for transfer scheduling
    pub priority: TransferPriority,
    
    /// Message payload containing the actual transfer data or metadata
    pub payload: DataTransferPayload,
}

/// Enumeration of all supported data transfer message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataTransferMessageType {
    /// Request to initiate a file transfer operation
    FileTransferRequest,
    
    /// Response to a file transfer request with acceptance or rejection
    FileTransferResponse,
    
    /// Request to start a streaming data operation
    StreamingDataRequest,
    
    /// Continuous streaming data message containing chunks
    StreamingDataMessage,
    
    /// Request to perform a batch of multiple file operations
    BatchTransferRequest,
    
    /// Response to a batch transfer request
    BatchTransferResponse,
    
    /// Data chunk message for chunked transfer operations
    DataChunk,
    
    /// Transfer progress update message
    TransferProgress,
    
    /// Transfer completion notification
    TransferComplete,
    
    /// Transfer error notification
    TransferError,
    
    /// Request to pause an ongoing transfer
    TransferPause,
    
    /// Request to resume a paused transfer
    TransferResume,
    
    /// Request to cancel a transfer operation
    TransferCancel,
    
    /// Heartbeat message to maintain connection during long transfers
    TransferHeartbeat,
    
    /// Data integrity validation message
    IntegrityValidation,
    
    /// Transfer metadata exchange message
    MetadataExchange,
}

/// Priority levels for transfer operations to enable proper scheduling
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TransferPriority {
    /// Critical system operations that must complete immediately
    Critical,
    
    /// High priority operations like consciousness data or urgent human requests
    High,
    
    /// Normal priority operations for standard ecosystem coordination
    Normal,
    
    /// Low priority operations like background synchronization
    Low,
    
    /// Background operations that can be delayed or throttled
    Background,
}

/// Container for all transfer message payload types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataTransferPayload {
    /// File transfer request payload
    FileTransferRequest(FileTransferRequest),
    
    /// File transfer response payload
    FileTransferResponse(FileTransferResponse),
    
    /// Streaming data request payload
    StreamingDataRequest(StreamingDataRequest),
    
    /// Streaming data message payload
    StreamingDataMessage(StreamingDataMessage),
    
    /// Batch transfer request payload
    BatchTransferRequest(BatchTransferRequest),
    
    /// Batch transfer response payload
    BatchTransferResponse(BatchTransferResponse),
    
    /// Data chunk payload for chunked transfers
    DataChunk(DataChunk),
    
    /// Transfer progress update payload
    TransferProgress(TransferProgress),
    
    /// Transfer completion payload
    TransferComplete(TransferComplete),
    
    /// Transfer error payload
    TransferError(TransferError),
    
    /// Transfer control operation payload (pause/resume/cancel)
    TransferControl(TransferControl),
    
    /// Heartbeat payload for connection maintenance
    TransferHeartbeat(TransferHeartbeat),
    
    /// Data integrity validation payload
    IntegrityValidation(DataIntegrity),
    
    /// Metadata exchange payload
    MetadataExchange(MetadataExchange),
}

// ============================================================================
// File Transfer Protocol Types
// ============================================================================

/// Request to transfer a file from source to destination component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransferRequest {
    /// Unique identifier for this transfer request
    pub transfer_id: String,
    
    /// Type of file transfer operation being requested
    pub transfer_type: FileTransferType,
    
    /// Source file path or identifier
    pub source_path: String,
    
    /// Destination path or identifier  
    pub destination_path: String,
    
    /// File metadata including size, permissions, timestamps
    pub file_metadata: FileMetadata,
    
    /// Transfer options and configuration
    pub transfer_options: TransferOptions,
    
    /// Expected file content hash for integrity verification
    pub expected_hash: Option<String>,
    
    /// Optional description of the transfer purpose
    pub description: Option<String>,
    
    /// Whether this is part of a larger batch operation
    pub batch_id: Option<String>,
    
    /// Callback URL or identifier for progress notifications
    pub progress_callback: Option<String>,
}

/// Type of file transfer operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileTransferType {
    /// Upload a file from the requesting component
    Upload,
    
    /// Download a file to the requesting component
    Download,
    
    /// Copy a file between two locations managed by the destination component
    Copy,
    
    /// Move a file between two locations
    Move,
    
    /// Synchronize files between two locations
    Sync,
    
    /// Create a backup copy of a file
    Backup,
    
    /// Restore a file from backup
    Restore,
}

/// Comprehensive metadata about a file being transferred
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// File name without path
    pub name: String,
    
    /// File size in bytes
    pub size: u64,
    
    /// MIME type of the file content
    pub content_type: Option<String>,
    
    /// File creation timestamp
    pub created: Option<SystemTime>,
    
    /// File last modification timestamp
    pub modified: Option<SystemTime>,
    
    /// File last access timestamp
    pub accessed: Option<SystemTime>,
    
    /// File permissions representation
    pub permissions: FilePermissions,
    
    /// File attributes specific to the source system
    pub attributes: HashMap<String, String>,
    
    /// Cryptographic hash of the file content
    pub content_hash: Option<String>,
    
    /// Hash algorithm used for content verification
    pub hash_algorithm: HashAlgorithm,
    
    /// Whether the file is compressed
    pub is_compressed: bool,
    
    /// Whether the file is encrypted
    pub is_encrypted: bool,
    
    /// Encoding used for text files
    pub encoding: Option<String>,
}

/// File permission representation that works across different systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePermissions {
    /// Whether the file is readable by the owner
    pub owner_read: bool,
    
    /// Whether the file is writable by the owner
    pub owner_write: bool,
    
    /// Whether the file is executable by the owner
    pub owner_execute: bool,
    
    /// Whether the file is readable by the group
    pub group_read: bool,
    
    /// Whether the file is writable by the group
    pub group_write: bool,
    
    /// Whether the file is executable by the group
    pub group_execute: bool,
    
    /// Whether the file is readable by others
    pub other_read: bool,
    
    /// Whether the file is writable by others
    pub other_write: bool,
    
    /// Whether the file is executable by others
    pub other_execute: bool,
    
    /// Owner identifier
    pub owner: Option<String>,
    
    /// Group identifier
    pub group: Option<String>,
}

/// Configuration options for transfer operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferOptions {
    /// Whether to use compression for the transfer
    pub use_compression: bool,
    
    /// Compression algorithm to use
    pub compression_algorithm: CompressionAlgorithm,
    
    /// Compression level (0-9, algorithm dependent)
    pub compression_level: u8,
    
    /// Whether to encrypt the transfer
    pub use_encryption: bool,
    
    /// Encryption algorithm to use
    pub encryption_algorithm: Option<String>,
    
    /// Chunk size for chunked transfers (bytes)
    pub chunk_size: u64,
    
    /// Maximum number of parallel chunks
    pub max_parallel_chunks: u8,
    
    /// Transfer timeout duration
    pub timeout: Duration,
    
    /// Number of retry attempts on failure
    pub retry_attempts: u8,
    
    /// Whether to verify integrity during transfer
    pub verify_integrity: bool,
    
    /// Whether to preserve file metadata
    pub preserve_metadata: bool,
    
    /// Whether to overwrite existing files
    pub overwrite_existing: bool,
    
    /// Whether to create intermediate directories
    pub create_directories: bool,
    
    /// Whether to enable resume capability
    pub enable_resume: bool,
    
    /// Bandwidth limit in bytes per second (0 = unlimited)
    pub bandwidth_limit: u64,
}

/// Supported compression algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    /// No compression
    None,
    
    /// GZIP compression (fast, good compression)
    Gzip,
    
    /// LZ4 compression (very fast, moderate compression)
    Lz4,
    
    /// ZSTD compression (good speed, excellent compression)
    Zstd,
    
    /// BROTLI compression (slow, excellent compression)
    Brotli,
}

/// Supported cryptographic hash algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HashAlgorithm {
    /// SHA-256 hash algorithm
    Sha256,
    
    /// SHA-512 hash algorithm  
    Sha512,
    
    /// BLAKE3 hash algorithm (fast, secure)
    Blake3,
    
    /// MD5 hash algorithm (legacy, not recommended for security)
    Md5,
}

/// Response to a file transfer request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransferResponse {
    /// Transfer ID from the original request
    pub transfer_id: String,
    
    /// Whether the transfer request was accepted
    pub accepted: bool,
    
    /// Reason for rejection if not accepted
    pub rejection_reason: Option<String>,
    
    /// Estimated transfer duration if accepted
    pub estimated_duration: Option<Duration>,
    
    /// Server-assigned transfer session ID
    pub session_id: Option<String>,
    
    /// Actual chunk size that will be used
    pub actual_chunk_size: Option<u64>,
    
    /// Whether compression will be used
    pub compression_enabled: bool,
    
    /// Whether encryption will be used
    pub encryption_enabled: bool,
    
    /// Total number of chunks for the transfer
    pub total_chunks: Option<u64>,
    
    /// Transfer endpoint information for direct communication
    pub transfer_endpoint: Option<TransferEndpoint>,
    
    /// Additional metadata about the transfer setup
    pub transfer_metadata: HashMap<String, String>,
}

/// Information about the endpoint for direct transfer communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferEndpoint {
    /// Endpoint type (TCP, WebSocket, etc.)
    pub endpoint_type: String,
    
    /// Host address for the transfer endpoint
    pub host: String,
    
    /// Port number for the transfer endpoint
    pub port: u16,
    
    /// Whether TLS/SSL is required
    pub secure: bool,
    
    /// Authentication token for the transfer session
    pub auth_token: Option<String>,
    
    /// Additional endpoint-specific parameters
    pub parameters: HashMap<String, String>,
}

// ============================================================================
// Streaming Data Protocol Types
// ============================================================================

/// Request to establish a streaming data connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingDataRequest {
    /// Unique identifier for the streaming session
    pub stream_id: String,
    
    /// Type of streaming operation
    pub stream_type: StreamingType,
    
    /// Data format for the stream
    pub data_format: DataFormat,
    
    /// Streaming configuration options
    pub stream_options: StreamingOptions,
    
    /// Expected total data size (if known)
    pub expected_size: Option<u64>,
    
    /// Expected duration of the stream (if known)
    pub expected_duration: Option<Duration>,
    
    /// Stream metadata and context information
    pub metadata: HashMap<String, String>,
    
    /// Quality of service requirements
    pub qos_requirements: QoSRequirements,
}

/// Types of streaming operations supported
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamingType {
    /// Real-time data streaming (live data)
    RealTime,
    
    /// Buffered streaming with small delay tolerance
    Buffered,
    
    /// Best-effort streaming without timing guarantees
    BestEffort,
    
    /// Batch streaming for large dataset processing
    Batch,
    
    /// Interactive streaming with low latency requirements
    Interactive,
}

/// Data format specifications for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataFormat {
    /// Raw binary data
    Binary,
    
    /// UTF-8 encoded text
    Text,
    
    /// JSON formatted data
    Json,
    
    /// MessagePack binary format
    MessagePack,
    
    /// Protocol Buffers format
    ProtoBuf,
    
    /// Custom format with identifier
    Custom(String),
}

/// Configuration options for streaming operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingOptions {
    /// Buffer size for streaming operations
    pub buffer_size: u64,
    
    /// Whether to use compression for the stream
    pub use_compression: bool,
    
    /// Compression algorithm for streaming
    pub compression_algorithm: CompressionAlgorithm,
    
    /// Whether to enable flow control
    pub flow_control: bool,
    
    /// Maximum bandwidth for the stream (bytes/sec)
    pub max_bandwidth: Option<u64>,
    
    /// Whether to enable error correction
    pub error_correction: bool,
    
    /// Checkpointing interval for resumable streams
    pub checkpoint_interval: Duration,
    
    /// Whether the stream supports seeking/rewinding
    pub seekable: bool,
    
    /// Whether to preserve message boundaries
    pub preserve_boundaries: bool,
}

/// Quality of Service requirements for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QoSRequirements {
    /// Maximum acceptable latency
    pub max_latency: Option<Duration>,
    
    /// Maximum acceptable jitter
    pub max_jitter: Option<Duration>,
    
    /// Minimum required bandwidth
    pub min_bandwidth: Option<u64>,
    
    /// Maximum acceptable packet loss rate (0.0-1.0)
    pub max_packet_loss: Option<f64>,
    
    /// Whether ordering must be preserved
    pub ordered_delivery: bool,
    
    /// Whether duplicate detection is required
    pub duplicate_detection: bool,
    
    /// Reliability level required
    pub reliability_level: ReliabilityLevel,
}

/// Reliability levels for data transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReliabilityLevel {
    /// Best effort, no guarantees
    BestEffort,
    
    /// At most once delivery (may lose messages)
    AtMostOnce,
    
    /// At least once delivery (may duplicate messages)
    AtLeastOnce,
    
    /// Exactly once delivery (guaranteed)
    ExactlyOnce,
}

/// Individual streaming data message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingDataMessage {
    /// Stream ID this message belongs to
    pub stream_id: String,
    
    /// Sequence number for ordering
    pub sequence_number: u64,
    
    /// Whether this is the final message in the stream
    pub is_final: bool,
    
    /// Timestamp when the message was created
    pub timestamp: SystemTime,
    
    /// Data payload for this message
    pub data: Bytes,
    
    /// Checksum for data integrity
    pub checksum: String,
    
    /// Message-specific metadata
    pub metadata: HashMap<String, String>,
    
    /// Whether this message requires acknowledgment
    pub requires_ack: bool,
    
    /// Retry count for this message
    pub retry_count: u8,
}

// ============================================================================
// Data Chunk Protocol Types
// ============================================================================

/// Individual data chunk for chunked transfer operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataChunk {
    /// Transfer ID this chunk belongs to
    pub transfer_id: String,
    
    /// Chunk sequence number
    pub chunk_number: u64,
    
    /// Total number of chunks in the transfer
    pub total_chunks: u64,
    
    /// Size of this chunk in bytes
    pub chunk_size: u64,
    
    /// Offset of this chunk in the complete data
    pub offset: u64,
    
    /// Whether this is the final chunk
    pub is_final: bool,
    
    /// Actual data payload for this chunk
    pub data: Bytes,
    
    /// Checksum for this chunk's data
    pub checksum: String,
    
    /// Hash algorithm used for the checksum
    pub hash_algorithm: HashAlgorithm,
    
    /// Whether this chunk is compressed
    pub is_compressed: bool,
    
    /// Original size before compression (if compressed)
    pub original_size: Option<u64>,
    
    /// Compression ratio achieved (if compressed)
    pub compression_ratio: Option<f64>,
    
    /// Retry count for this chunk
    pub retry_count: u8,
    
    /// Timestamp when the chunk was created
    pub timestamp: SystemTime,
    
    /// Chunk-specific metadata
    pub metadata: HashMap<String, String>,
}

// ============================================================================
// Transfer Progress and Status Types
// ============================================================================

/// Transfer progress information and statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferProgress {
    /// Transfer ID being reported on
    pub transfer_id: String,
    
    /// Current transfer status
    pub status: TransferStatus,
    
    /// Number of bytes transferred so far
    pub bytes_transferred: u64,
    
    /// Total bytes to transfer
    pub total_bytes: u64,
    
    /// Number of chunks completed
    pub chunks_completed: u64,
    
    /// Total number of chunks
    pub total_chunks: u64,
    
    /// Progress percentage (0.0-1.0)
    pub progress_percentage: f64,
    
    /// Current transfer rate in bytes per second
    pub transfer_rate: f64,
    
    /// Average transfer rate since start
    pub average_rate: f64,
    
    /// Estimated time remaining
    pub eta: Option<Duration>,
    
    /// Elapsed time since transfer started
    pub elapsed_time: Duration,
    
    /// Last activity timestamp
    pub last_activity: SystemTime,
    
    /// Number of retries performed
    pub retry_count: u32,
    
    /// Number of errors encountered
    pub error_count: u32,
    
    /// Current operation being performed
    pub current_operation: Option<String>,
    
    /// Additional progress metadata
    pub metadata: HashMap<String, String>,
}

/// Current status of a transfer operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransferStatus {
    /// Transfer is being initialized
    Initializing,
    
    /// Transfer is waiting to start (queued)
    Pending,
    
    /// Transfer is actively running
    InProgress,
    
    /// Transfer is temporarily paused
    Paused,
    
    /// Transfer is waiting for retry after error
    Retrying,
    
    /// Transfer completed successfully
    Completed,
    
    /// Transfer failed with error
    Failed,
    
    /// Transfer was cancelled by user/system
    Cancelled,
    
    /// Transfer is being verified for integrity
    Verifying,
    
    /// Transfer is being cleaned up
    Finalizing,
}

/// Transfer completion notification with final statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferComplete {
    /// Transfer ID that completed
    pub transfer_id: String,
    
    /// Final transfer status
    pub final_status: TransferStatus,
    
    /// Total bytes transferred
    pub total_bytes: u64,
    
    /// Total transfer duration
    pub total_duration: Duration,
    
    /// Average transfer rate
    pub average_rate: f64,
    
    /// Peak transfer rate achieved
    pub peak_rate: f64,
    
    /// Final integrity verification result
    pub integrity_verified: bool,
    
    /// Final hash of transferred data
    pub final_hash: Option<String>,
    
    /// Total number of retries performed
    pub total_retries: u32,
    
    /// Total number of errors encountered
    pub total_errors: u32,
    
    /// Compression ratio achieved (if used)
    pub compression_ratio: Option<f64>,
    
    /// Final file metadata (for file transfers)
    pub final_metadata: Option<FileMetadata>,
    
    /// Transfer completion metadata
    pub metadata: HashMap<String, String>,
    
    /// Optional success message
    pub message: Option<String>,
}

// ============================================================================
// Error Handling Types
// ============================================================================

/// Transfer error information with detailed context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferError {
    /// Transfer ID where the error occurred
    pub transfer_id: String,
    
    /// Type of error that occurred
    pub error_type: TransferErrorType,
    
    /// Error code for programmatic handling
    pub error_code: String,
    
    /// Human-readable error message
    pub message: String,
    
    /// Detailed error description
    pub details: Option<String>,
    
    /// Whether the error is recoverable
    pub recoverable: bool,
    
    /// Suggested retry delay if recoverable
    pub retry_delay: Option<Duration>,
    
    /// Context where the error occurred
    pub context: ErrorContext,
    
    /// Timestamp when the error occurred
    pub timestamp: SystemTime,
    
    /// Stack trace or diagnostic information
    pub diagnostic_info: Option<String>,
    
    /// Related error information
    pub related_errors: Vec<String>,
    
    /// Error-specific metadata
    pub metadata: HashMap<String, String>,
}

/// Categories of transfer errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransferErrorType {
    /// Network connectivity error
    NetworkError,
    
    /// Authentication or authorization error
    SecurityError,
    
    /// File system error (permissions, space, etc.)
    FileSystemError,
    
    /// Data corruption or integrity error
    IntegrityError,
    
    /// Transfer timeout error
    TimeoutError,
    
    /// Resource exhaustion error (memory, disk, etc.)
    ResourceError,
    
    /// Configuration or setup error
    ConfigurationError,
    
    /// Protocol violation or format error
    ProtocolError,
    
    /// Transfer was cancelled
    CancellationError,
    
    /// Unknown or unexpected error
    UnknownError,
}

/// Context information about where an error occurred
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    /// Component where the error occurred
    pub component: String,
    
    /// Operation being performed when error occurred
    pub operation: String,
    
    /// Chunk number (if applicable)
    pub chunk_number: Option<u64>,
    
    /// Byte offset where error occurred
    pub byte_offset: Option<u64>,
    
    /// Network endpoint involved (if applicable)
    pub endpoint: Option<String>,
    
    /// File path involved (if applicable)
    pub file_path: Option<String>,
    
    /// Additional context information
    pub additional_context: HashMap<String, String>,
}

// ============================================================================
// Transfer Control Operations
// ============================================================================

/// Transfer control operations (pause, resume, cancel)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferControl {
    /// Transfer ID to control
    pub transfer_id: String,
    
    /// Control operation to perform
    pub operation: ControlOperation,
    
    /// Reason for the control operation
    pub reason: Option<String>,
    
    /// Additional parameters for the operation
    pub parameters: HashMap<String, String>,
    
    /// Whether to force the operation
    pub force: bool,
    
    /// Timeout for the control operation
    pub timeout: Option<Duration>,
}

/// Types of transfer control operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlOperation {
    /// Pause the transfer temporarily
    Pause,
    
    /// Resume a paused transfer
    Resume,
    
    /// Cancel the transfer permanently
    Cancel,
    
    /// Change transfer priority
    ChangePriority(TransferPriority),
    
    /// Adjust bandwidth limit
    AdjustBandwidth(u64),
    
    /// Request transfer status
    Status,
    
    /// Reset transfer statistics
    ResetStats,
}

/// Heartbeat message to maintain connection during long transfers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferHeartbeat {
    /// Transfer ID for the heartbeat
    pub transfer_id: String,
    
    /// Current timestamp
    pub timestamp: SystemTime,
    
    /// Brief status information
    pub status: TransferStatus,
    
    /// Current progress percentage
    pub progress: f64,
    
    /// Whether the transfer is healthy
    pub healthy: bool,
    
    /// Next expected heartbeat time
    pub next_heartbeat: SystemTime,
    
    /// Optional status message
    pub message: Option<String>,
}

// ============================================================================
// Data Integrity and Validation Types
// ============================================================================

/// Data integrity validation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataIntegrity {
    /// Transfer or data identifier
    pub data_id: String,
    
    /// Type of integrity check performed
    pub check_type: IntegrityCheckType,
    
    /// Result of the integrity check
    pub result: IntegrityResult,
    
    /// Hash value computed
    pub computed_hash: String,
    
    /// Expected hash value
    pub expected_hash: String,
    
    /// Hash algorithm used
    pub hash_algorithm: HashAlgorithm,
    
    /// Whether the integrity check passed
    pub passed: bool,
    
    /// Additional verification details
    pub verification_details: VerificationDetails,
    
    /// Timestamp of the integrity check
    pub timestamp: SystemTime,
    
    /// Integrity check metadata
    pub metadata: HashMap<String, String>,
}

/// Types of integrity checks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntegrityCheckType {
    /// Full data hash verification
    FullHash,
    
    /// Chunk-by-chunk verification
    ChunkedHash,
    
    /// Incremental verification during transfer
    IncrementalHash,
    
    /// Partial verification of random samples
    SampleHash,
    
    /// Error detection code verification
    ErrorDetection,
    
    /// Digital signature verification
    DigitalSignature,
}

/// Result of integrity verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntegrityResult {
    /// Integrity verification passed
    Passed,
    
    /// Integrity verification failed
    Failed,
    
    /// Integrity verification skipped
    Skipped,
    
    /// Integrity verification in progress
    InProgress,
    
    /// Integrity verification error
    Error(String),
}

/// Detailed verification information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationDetails {
    /// Total bytes verified
    pub bytes_verified: u64,
    
    /// Number of chunks verified
    pub chunks_verified: u64,
    
    /// Verification duration
    pub duration: Duration,
    
    /// Verification rate (bytes/sec)
    pub rate: f64,
    
    /// Number of errors found
    pub errors_found: u32,
    
    /// Whether partial verification was used
    pub partial_verification: bool,
    
    /// Confidence level of verification
    pub confidence_level: f64,
    
    /// Additional verification metrics
    pub metrics: HashMap<String, f64>,
}

// ============================================================================
// Batch Transfer Operations
// ============================================================================

/// Request to perform multiple file operations as a batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTransferRequest {
    /// Unique identifier for the batch operation
    pub batch_id: String,
    
    /// List of individual transfer requests in the batch
    pub transfers: Vec<FileTransferRequest>,
    
    /// Batch execution options
    pub batch_options: BatchOptions,
    
    /// Overall batch timeout
    pub timeout: Duration,
    
    /// Batch priority level
    pub priority: TransferPriority,
    
    /// Whether to stop on first error
    pub fail_fast: bool,
    
    /// Maximum number of parallel transfers
    pub max_parallel: u8,
    
    /// Batch metadata and context
    pub metadata: HashMap<String, String>,
}

/// Configuration options for batch operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOptions {
    /// Whether to maintain transaction semantics (all or nothing)
    pub transactional: bool,
    
    /// Whether to optimize transfer order
    pub optimize_order: bool,
    
    /// Whether to enable cross-transfer optimization
    pub cross_transfer_optimization: bool,
    
    /// Whether to generate detailed progress reports
    pub detailed_progress: bool,
    
    /// Whether to verify integrity of all transfers
    pub verify_all: bool,
    
    /// Resource allocation strategy for the batch
    pub resource_strategy: ResourceStrategy,
    
    /// Whether to enable batch checkpointing
    pub enable_checkpoints: bool,
    
    /// Checkpoint frequency for resumable batches
    pub checkpoint_frequency: Duration,
}

/// Resource allocation strategies for batch operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceStrategy {
    /// Allocate equal resources to all transfers
    Equal,
    
    /// Allocate resources based on transfer size
    SizeBased,
    
    /// Allocate resources based on transfer priority
    PriorityBased,
    
    /// Use adaptive resource allocation
    Adaptive,
    
    /// Use custom resource allocation weights
    Custom(HashMap<String, f64>),
}

/// Response to a batch transfer request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTransferResponse {
    /// Batch ID from the request
    pub batch_id: String,
    
    /// Whether the batch was accepted
    pub accepted: bool,
    
    /// Reason for rejection if not accepted
    pub rejection_reason: Option<String>,
    
    /// Estimated total duration for the batch
    pub estimated_duration: Option<Duration>,
    
    /// Individual transfer responses
    pub transfer_responses: Vec<BatchTransferItemResponse>,
    
    /// Batch execution plan
    pub execution_plan: Option<BatchExecutionPlan>,
    
    /// Session ID for tracking the batch
    pub session_id: Option<String>,
    
    /// Batch response metadata
    pub metadata: HashMap<String, String>,
}

/// Response for individual transfers within a batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTransferItemResponse {
    /// Transfer ID from the original request
    pub transfer_id: String,
    
    /// Whether this individual transfer was accepted
    pub accepted: bool,
    
    /// Reason for rejection if not accepted
    pub rejection_reason: Option<String>,
    
    /// Position in the batch execution order
    pub execution_order: u32,
    
    /// Estimated start time for this transfer
    pub estimated_start_time: Option<SystemTime>,
    
    /// Estimated duration for this transfer
    pub estimated_duration: Option<Duration>,
    
    /// Dependencies on other transfers in the batch
    pub dependencies: Vec<String>,
}

/// Execution plan for a batch transfer operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchExecutionPlan {
    /// Total estimated duration
    pub total_duration: Duration,
    
    /// Execution phases for the batch
    pub execution_phases: Vec<BatchPhase>,
    
    /// Resource allocation plan
    pub resource_allocation: ResourceAllocationPlan,
    
    /// Optimization strategies applied
    pub optimizations_applied: Vec<String>,
    
    /// Checkpointing strategy
    pub checkpoint_strategy: CheckpointStrategy,
}

/// Individual phase in batch execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchPhase {
    /// Phase identifier
    pub phase_id: String,
    
    /// Transfers to execute in this phase
    pub transfer_ids: Vec<String>,
    
    /// Whether transfers in this phase can run in parallel
    pub parallel_execution: bool,
    
    /// Estimated phase duration
    pub estimated_duration: Duration,
    
    /// Phase dependencies
    pub dependencies: Vec<String>,
}

/// Resource allocation plan for batch operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocationPlan {
    /// CPU allocation percentages by transfer
    pub cpu_allocation: HashMap<String, f64>,
    
    /// Memory allocation by transfer
    pub memory_allocation: HashMap<String, u64>,
    
    /// Bandwidth allocation by transfer
    pub bandwidth_allocation: HashMap<String, u64>,
    
    /// Storage allocation by transfer
    pub storage_allocation: HashMap<String, u64>,
    
    /// Network connection allocation
    pub connection_allocation: HashMap<String, u8>,
}

/// Checkpointing strategy for resumable operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointStrategy {
    /// Whether checkpointing is enabled
    pub enabled: bool,
    
    /// Checkpoint frequency
    pub frequency: Duration,
    
    /// Checkpoint storage location
    pub storage_location: String,
    
    /// Whether to compress checkpoints
    pub compress_checkpoints: bool,
    
    /// Checkpoint retention policy
    pub retention_count: u32,
    
    /// Whether to verify checkpoint integrity
    pub verify_integrity: bool,
}

// ============================================================================
// Metadata Exchange Types
// ============================================================================

/// Metadata exchange for coordination and synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataExchange {
    /// Exchange identifier
    pub exchange_id: String,
    
    /// Type of metadata being exchanged
    pub metadata_type: MetadataType,
    
    /// Source component providing the metadata
    pub source_component: String,
    
    /// Target component receiving the metadata
    pub target_component: String,
    
    /// Metadata payload
    pub metadata: HashMap<String, serde_json::Value>,
    
    /// Metadata schema version
    pub schema_version: String,
    
    /// Whether this metadata supersedes previous versions
    pub supersedes_previous: bool,
    
    /// Expiration time for the metadata
    pub expires_at: Option<SystemTime>,
    
    /// Metadata integrity hash
    pub integrity_hash: Option<String>,
    
    /// Exchange timestamp
    pub timestamp: SystemTime,
}

/// Types of metadata that can be exchanged
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetadataType {
    /// File system metadata
    FileSystemMetadata,
    
    /// Transfer capability metadata
    TransferCapabilities,
    
    /// Network topology metadata
    NetworkTopology,
    
    /// Security configuration metadata
    SecurityMetadata,
    
    /// Performance metrics metadata
    PerformanceMetrics,
    
    /// Component status metadata
    ComponentStatus,
    
    /// ZSEI intelligent metadata
    ZSEIMetadata,
    
    /// Custom metadata type
    Custom(String),
}

// ============================================================================
// Error Type Definitions
// ============================================================================

/// Comprehensive error type for data transfer operations
#[derive(Error, Debug)]
pub enum DataTransferError {
    #[error("Network error: {operation} failed - {details}")]
    NetworkError { operation: String, details: String },

    #[error("Security error: {operation} - {details}")]
    SecurityError { operation: String, details: String },

    #[error("File system error: {path} - {details}")]
    FileSystemError { path: String, details: String },

    #[error("Integrity error: expected {expected}, got {actual}")]
    IntegrityError { expected: String, actual: String },

    #[error("Timeout error: {operation} timed out after {duration:?}")]
    TimeoutError { operation: String, duration: Duration },

    #[error("Resource error: {resource} - {details}")]
    ResourceError { resource: String, details: String },

    #[error("Configuration error: {parameter} - {details}")]
    ConfigurationError { parameter: String, details: String },

    #[error("Protocol error: {protocol} - {violation}")]
    ProtocolError { protocol: String, violation: String },

    #[error("Serialization error: {details}")]
    SerializationError { details: String },

    #[error("Compression error: {algorithm} - {details}")]
    CompressionError { algorithm: String, details: String },

    #[error("Transfer cancelled: {transfer_id}")]
    TransferCancelled { transfer_id: String },

    #[error("Transfer not found: {transfer_id}")]
    TransferNotFound { transfer_id: String },

    #[error("Invalid transfer state: expected {expected}, found {actual}")]
    InvalidTransferState { expected: String, actual: String },

    #[error("Batch operation error: {batch_id} - {details}")]
    BatchOperationError { batch_id: String, details: String },

    #[error("Streaming error: {stream_id} - {details}")]
    StreamingError { stream_id: String, details: String },

    #[error("Unknown error: {details}")]
    UnknownError { details: String },
}

// ============================================================================
// Utility Functions and Implementations
// ============================================================================

impl Default for TransferPriority {
    fn default() -> Self {
        TransferPriority::Normal
    }
}

impl Default for CompressionAlgorithm {
    fn default() -> Self {
        CompressionAlgorithm::Gzip
    }
}

impl Default for HashAlgorithm {
    fn default() -> Self {
        HashAlgorithm::Blake3
    }
}

impl Default for TransferOptions {
    fn default() -> Self {
        Self {
            use_compression: true,
            compression_algorithm: CompressionAlgorithm::default(),
            compression_level: 6,
            use_encryption: false,
            encryption_algorithm: None,
            chunk_size: 1024 * 1024, // 1MB default chunk size
            max_parallel_chunks: 4,
            timeout: Duration::from_secs(300), // 5 minute default timeout
            retry_attempts: 3,
            verify_integrity: true,
            preserve_metadata: true,
            overwrite_existing: false,
            create_directories: true,
            enable_resume: true,
            bandwidth_limit: 0, // No limit by default
        }
    }
}

impl Default for FilePermissions {
    fn default() -> Self {
        Self {
            owner_read: true,
            owner_write: true,
            owner_execute: false,
            group_read: true,
            group_write: false,
            group_execute: false,
            other_read: true,
            other_write: false,
            other_execute: false,
            owner: None,
            group: None,
        }
    }
}

impl DataChunk {
    /// Create a new data chunk with automatic checksum calculation
    pub fn new(
        transfer_id: String,
        chunk_number: u64,
        total_chunks: u64,
        offset: u64,
        data: Bytes,
        hash_algorithm: HashAlgorithm,
    ) -> Self {
        let chunk_size = data.len() as u64;
        let is_final = chunk_number == total_chunks - 1;
        let checksum = Self::calculate_checksum(&data, &hash_algorithm);
        
        Self {
            transfer_id,
            chunk_number,
            total_chunks,
            chunk_size,
            offset,
            is_final,
            data,
            checksum,
            hash_algorithm,
            is_compressed: false,
            original_size: None,
            compression_ratio: None,
            retry_count: 0,
            timestamp: SystemTime::now(),
            metadata: HashMap::new(),
        }
    }
    
    /// Calculate checksum for data using the specified algorithm
    pub fn calculate_checksum(data: &[u8], algorithm: &HashAlgorithm) -> String {
        match algorithm {
            HashAlgorithm::Sha256 => {
                let mut hasher = Sha256::new();
                hasher.update(data);
                format!("{:x}", hasher.finalize())
            },
            HashAlgorithm::Blake3 => {
                let mut hasher = Blake3Hasher::new();
                hasher.update(data);
                format!("{}", hasher.finalize().to_hex())
            },
            HashAlgorithm::Sha512 => {
                use sha2::Sha512;
                let mut hasher = Sha512::new();
                hasher.update(data);
                format!("{:x}", hasher.finalize())
            },
            HashAlgorithm::Md5 => {
                use md5::{Md5, Digest};
                let mut hasher = Md5::new();
                hasher.update(data);
                format!("{:x}", hasher.finalize())
            },
        }
    }
    
    /// Verify the integrity of this chunk's data
    pub fn verify_integrity(&self) -> bool {
        let calculated_checksum = Self::calculate_checksum(&self.data, &self.hash_algorithm);
        calculated_checksum == self.checksum
    }
    
    /// Compress the chunk data using the specified algorithm
    pub fn compress(&mut self, algorithm: CompressionAlgorithm) -> Result<(), DataTransferError> {
        if self.is_compressed {
            return Ok(()); // Already compressed
        }
        
        let original_size = self.data.len();
        let compressed_data = match algorithm {
            CompressionAlgorithm::None => return Ok(()),
            CompressionAlgorithm::Gzip => {
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(&self.data).map_err(|e| DataTransferError::CompressionError {
                    algorithm: "gzip".to_string(),
                    details: e.to_string(),
                })?;
                encoder.finish().map_err(|e| DataTransferError::CompressionError {
                    algorithm: "gzip".to_string(),
                    details: e.to_string(),
                })?
            },
            _ => return Err(DataTransferError::CompressionError {
                algorithm: format!("{:?}", algorithm),
                details: "Algorithm not implemented".to_string(),
            }),
        };
        
        let compressed_size = compressed_data.len();
        let compression_ratio = compressed_size as f64 / original_size as f64;
        
        // Only use compression if it actually reduces size
        if compressed_size < original_size {
            self.data = Bytes::from(compressed_data);
            self.chunk_size = compressed_size as u64;
            self.is_compressed = true;
            self.original_size = Some(original_size as u64);
            self.compression_ratio = Some(compression_ratio);
            
            // Recalculate checksum for compressed data
            self.checksum = Self::calculate_checksum(&self.data, &self.hash_algorithm);
        }
        
        Ok(())
    }
    
    /// Decompress the chunk data if it's compressed
    pub fn decompress(&mut self) -> Result<(), DataTransferError> {
        if !self.is_compressed {
            return Ok(()); // Not compressed
        }
        
        let decompressed_data = {
            let mut decoder = GzDecoder::new(&self.data[..]);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed).map_err(|e| DataTransferError::CompressionError {
                algorithm: "gzip".to_string(),
                details: e.to_string(),
            })?;
            decompressed
        };
        
        self.data = Bytes::from(decompressed_data);
        self.chunk_size = self.data.len() as u64;
        self.is_compressed = false;
        
        // Recalculate checksum for decompressed data
        self.checksum = Self::calculate_checksum(&self.data, &self.hash_algorithm);
        
        Ok(())
    }
}

impl TransferProgress {
    /// Create a new transfer progress update
    pub fn new(transfer_id: String, status: TransferStatus) -> Self {
        Self {
            transfer_id,
            status,
            bytes_transferred: 0,
            total_bytes: 0,
            chunks_completed: 0,
            total_chunks: 0,
            progress_percentage: 0.0,
            transfer_rate: 0.0,
            average_rate: 0.0,
            eta: None,
            elapsed_time: Duration::from_secs(0),
            last_activity: SystemTime::now(),
            retry_count: 0,
            error_count: 0,
            current_operation: None,
            metadata: HashMap::new(),
        }
    }
    
    /// Update progress with new byte count
    pub fn update_bytes(&mut self, bytes_transferred: u64, total_bytes: u64) {
        self.bytes_transferred = bytes_transferred;
        self.total_bytes = total_bytes;
        self.progress_percentage = if total_bytes > 0 {
            bytes_transferred as f64 / total_bytes as f64
        } else {
            0.0
        };
        self.last_activity = SystemTime::now();
    }
    
    /// Update progress with new chunk count
    pub fn update_chunks(&mut self, chunks_completed: u64, total_chunks: u64) {
        self.chunks_completed = chunks_completed;
        self.total_chunks = total_chunks;
        self.last_activity = SystemTime::now();
    }
    
    /// Calculate and update transfer rate
    pub fn update_rate(&mut self, elapsed_time: Duration) {
        self.elapsed_time = elapsed_time;
        if elapsed_time.as_secs() > 0 {
            self.average_rate = self.bytes_transferred as f64 / elapsed_time.as_secs_f64();
        }
        
        // Estimate time remaining
        if self.average_rate > 0.0 && self.total_bytes > self.bytes_transferred {
            let remaining_bytes = self.total_bytes - self.bytes_transferred;
            let remaining_seconds = remaining_bytes as f64 / self.average_rate;
            self.eta = Some(Duration::from_secs_f64(remaining_seconds));
        }
    }
}

impl FileMetadata {
    /// Create minimal file metadata with just name and size
    pub fn minimal(name: String, size: u64) -> Self {
        Self {
            name,
            size,
            content_type: None,
            created: None,
            modified: None,
            accessed: None,
            permissions: FilePermissions::default(),
            attributes: HashMap::new(),
            content_hash: None,
            hash_algorithm: HashAlgorithm::default(),
            is_compressed: false,
            is_encrypted: false,
            encoding: None,
        }
    }
    
    /// Create complete file metadata with all available information
    pub fn complete(
        name: String,
        size: u64,
        content_type: Option<String>,
        created: Option<SystemTime>,
        modified: Option<SystemTime>,
        permissions: FilePermissions,
    ) -> Self {
        Self {
            name,
            size,
            content_type,
            created,
            modified,
            accessed: Some(SystemTime::now()),
            permissions,
            attributes: HashMap::new(),
            content_hash: None,
            hash_algorithm: HashAlgorithm::default(),
            is_compressed: false,
            is_encrypted: false,
            encoding: None,
        }
    }
}

// ============================================================================
// Result Type Definitions
// ============================================================================

/// Result type for data transfer operations
pub type DataTransferResult<T> = Result<T, DataTransferError>;

// ============================================================================
// Constants and Default Values
// ============================================================================

/// Default chunk size for file transfers (1MB)
pub const DEFAULT_CHUNK_SIZE: u64 = 1024 * 1024;

/// Maximum chunk size allowed (100MB)
pub const MAX_CHUNK_SIZE: u64 = 100 * 1024 * 1024;

/// Minimum chunk size allowed (4KB)
pub const MIN_CHUNK_SIZE: u64 = 4 * 1024;

/// Default transfer timeout (5 minutes)
pub const DEFAULT_TRANSFER_TIMEOUT: Duration = Duration::from_secs(300);

/// Maximum transfer timeout (24 hours)
pub const MAX_TRANSFER_TIMEOUT: Duration = Duration::from_secs(86400);

/// Default retry attempts
pub const DEFAULT_RETRY_ATTEMPTS: u8 = 3;

/// Maximum retry attempts
pub const MAX_RETRY_ATTEMPTS: u8 = 10;

/// Default heartbeat interval (30 seconds)
pub const DEFAULT_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

/// Default progress update interval (1 second)
pub const DEFAULT_PROGRESS_INTERVAL: Duration = Duration::from_secs(1);

/// Maximum parallel chunks per transfer
pub const MAX_PARALLEL_CHUNKS: u8 = 16;

/// Default compression level
pub const DEFAULT_COMPRESSION_LEVEL: u8 = 6;

/// Maximum batch size (number of transfers)
pub const MAX_BATCH_SIZE: usize = 1000;

// ============================================================================
// Module Documentation and Usage Examples
// ============================================================================

//! # Usage Examples
//!
//! ## Basic File Transfer
//!
//! ```rust
//! use data_transfer_protocols::*;
//!
//! // Create a file transfer request
//! let transfer_request = FileTransferRequest {
//!     transfer_id: "transfer_001".to_string(),
//!     transfer_type: FileTransferType::Upload,
//!     source_path: "/local/file.txt".to_string(),
//!     destination_path: "/remote/file.txt".to_string(),
//!     file_metadata: FileMetadata::minimal("file.txt".to_string(), 1024),
//!     transfer_options: TransferOptions::default(),
//!     expected_hash: None,
//!     description: Some("Example file transfer".to_string()),
//!     batch_id: None,
//!     progress_callback: None,
//! };
//!
//! // Create the transfer message
//! let message = DataTransferMessage {
//!     message_id: "msg_001".to_string(),
//!     message_type: DataTransferMessageType::FileTransferRequest,
//!     source_component: "BRIDGE".to_string(),
//!     destination_component: "NEXUS".to_string(),
//!     security_context: None,
//!     timestamp: SystemTime::now(),
//!     correlation_id: None,
//!     priority: TransferPriority::Normal,
//!     payload: DataTransferPayload::FileTransferRequest(transfer_request),
//! };
//! ```
//!
//! ## Streaming Data Transfer
//!
//! ```rust
//! // Create a streaming data request
//! let stream_request = StreamingDataRequest {
//!     stream_id: "stream_001".to_string(),
//!     stream_type: StreamingType::RealTime,
//!     data_format: DataFormat::Json,
//!     stream_options: StreamingOptions {
//!         buffer_size: 64 * 1024,
//!         use_compression: true,
//!         compression_algorithm: CompressionAlgorithm::Gzip,
//!         flow_control: true,
//!         max_bandwidth: Some(1024 * 1024), // 1MB/s
//!         error_correction: true,
//!         checkpoint_interval: Duration::from_secs(10),
//!         seekable: false,
//!         preserve_boundaries: true,
//!     },
//!     expected_size: None,
//!     expected_duration: Some(Duration::from_secs(3600)),
//!     metadata: HashMap::new(),
//!     qos_requirements: QoSRequirements {
//!         max_latency: Some(Duration::from_millis(100)),
//!         max_jitter: Some(Duration::from_millis(10)),
//!         min_bandwidth: Some(512 * 1024),
//!         max_packet_loss: Some(0.001),
//!         ordered_delivery: true,
//!         duplicate_detection: true,
//!         reliability_level: ReliabilityLevel::ExactlyOnce,
//!     },
//! };
//! ```
//!
//! ## Batch Transfer Operation
//!
//! ```rust
//! // Create multiple file transfer requests
//! let transfers = vec![
//!     FileTransferRequest { /* ... */ },
//!     FileTransferRequest { /* ... */ },
//!     FileTransferRequest { /* ... */ },
//! ];
//!
//! // Create batch transfer request
//! let batch_request = BatchTransferRequest {
//!     batch_id: "batch_001".to_string(),
//!     transfers,
//!     batch_options: BatchOptions {
//!         transactional: true,
//!         optimize_order: true,
//!         cross_transfer_optimization: true,
//!         detailed_progress: true,
//!         verify_all: true,
//!         resource_strategy: ResourceStrategy::SizeBased,
//!         enable_checkpoints: true,
//!         checkpoint_frequency: Duration::from_secs(60),
//!     },
//!     timeout: Duration::from_secs(3600),
//!     priority: TransferPriority::High,
//!     fail_fast: false,
//!     max_parallel: 4,
//!     metadata: HashMap::new(),
//! };
//! ```
//!
//! ## Data Chunk Processing
//!
//! ```rust
//! use bytes::Bytes;
//!
//! // Create a data chunk
//! let data = Bytes::from("Hello, OZONE STUDIO ecosystem!");
//! let mut chunk = DataChunk::new(
//!     "transfer_001".to_string(),
//!     0, // First chunk
//!     1, // Only one chunk
//!     0, // Offset 0
//!     data,
//!     HashAlgorithm::Blake3,
//! );
//!
//! // Compress the chunk
//! chunk.compress(CompressionAlgorithm::Gzip)?;
//!
//! // Verify integrity
//! assert!(chunk.verify_integrity());
//!
//! // Decompress when needed
//! chunk.decompress()?;
//! ```
//!
//! # Integration with OZONE STUDIO Ecosystem
//!
//! This module integrates with other ecosystem components as follows:
//!
//! - **NEXUS**: Primary consumer of file transfer protocols for all file operations
//! - **BRIDGE**: Uses streaming protocols for real-time human interaction data
//! - **ZSEI**: Transfers intelligent metadata and methodologies
//! - **OZONE STUDIO**: Coordinates large-scale data movements for context loops
//! - **SPARK**: Transfers model data and inference results
//! - **FORGE/SCRIBE**: Transfer code files and documents for processing
//!
//! All transfers are secured using the shared security framework and support
//! the ecosystem's requirements for consciousness continuity, context preservation,
//! and relationship maintenance across unlimited complexity processing operations.
