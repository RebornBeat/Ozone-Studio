//! Modality Detector
//!
//! Automatically detects modalities from task requests. This is the first step
//! in OZONE's modality-first architecture - detecting what types of data are
//! involved before any processing begins.
//!
//! # Detection Sources
//!
//! 1. **File Attachments**: Detect from file extensions
//! 2. **Request Text**: Detect from keywords and patterns
//! 3. **Existing Context**: Detect from project's existing modality graphs
//!
//! # Usage
//!
//! ```rust
//! let detector = ModalityDetector::new(enabled_modalities);
//! let detected = detector.detect_modalities(&request);
//! 
//! for dm in detected {
//!     println!("Detected {:?} from {:?} with confidence {}", 
//!         dm.modality, dm.source, dm.confidence);
//! }
//! ```

use std::collections::{HashMap, HashSet};
use std::path::Path;
use serde::{Deserialize, Serialize};
use regex::Regex;

use crate::modalities::ModalityType;

// =============================================================================
// Detection Result
// =============================================================================

/// A detected modality with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedModality {
    /// The modality type detected
    pub modality: ModalityType,
    
    /// Source of the detection
    pub source: ModalitySource,
    
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    
    /// Additional details about the detection
    pub details: Option<String>,
}

/// Source of modality detection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModalitySource {
    /// Detected from file attachment
    Attachment { filename: String },
    
    /// Detected from request text content
    TextContent { matched_pattern: String },
    
    /// Detected from existing project context
    ProjectContext { graph_id: String },
    
    /// Explicitly specified by user
    Explicit,
    
    /// Inferred from task type
    TaskType { task_category: String },
}

// =============================================================================
// Modality Detector
// =============================================================================

/// Detects modalities from task requests
pub struct ModalityDetector {
    /// Enabled modalities to detect
    enabled_modalities: HashSet<ModalityType>,
    
    /// File extension mappings
    extension_map: HashMap<String, ModalityType>,
    
    /// Text patterns for detection
    text_patterns: Vec<(Regex, ModalityType, f32)>,
    
    /// Task type patterns
    task_patterns: Vec<(Regex, Vec<ModalityType>)>,
}

impl ModalityDetector {
    /// Create a new modality detector
    pub fn new(enabled_modalities: Vec<ModalityType>) -> Self {
        let enabled_set: HashSet<_> = enabled_modalities.into_iter().collect();
        
        Self {
            enabled_modalities: enabled_set,
            extension_map: Self::build_extension_map(),
            text_patterns: Self::build_text_patterns(),
            task_patterns: Self::build_task_patterns(),
        }
    }
    
    /// Detect modalities from a task request
    pub fn detect_modalities(&self, request: &TaskRequest) -> Vec<DetectedModality> {
        let mut detections = Vec::new();
        
        // 1. Check file attachments
        for attachment in &request.attachments {
            if let Some(detection) = self.detect_from_file(attachment) {
                if self.enabled_modalities.contains(&detection.modality) {
                    detections.push(detection);
                }
            }
        }
        
        // 2. Check request text content
        let text_detections = self.detect_from_text(&request.content);
        for detection in text_detections {
            if self.enabled_modalities.contains(&detection.modality) {
                detections.push(detection);
            }
        }
        
        // 3. Check explicit modality specifications
        if let Some(explicit) = &request.explicit_modality {
            detections.push(DetectedModality {
                modality: *explicit,
                source: ModalitySource::Explicit,
                confidence: 1.0,
                details: Some("Explicitly specified by user".to_string()),
            });
        }
        
        // 4. Check existing context
        if let Some(context) = &request.context {
            let context_detections = self.detect_from_context(context);
            for detection in context_detections {
                if self.enabled_modalities.contains(&detection.modality) {
                    detections.push(detection);
                }
            }
        }
        
        // 5. Infer from task type
        let task_detections = self.detect_from_task_type(&request.content);
        for detection in task_detections {
            if self.enabled_modalities.contains(&detection.modality) {
                detections.push(detection);
            }
        }
        
        // Deduplicate and merge
        self.deduplicate_modalities(detections)
    }
    
    /// Detect modality from a file attachment
    pub fn detect_from_file(&self, attachment: &Attachment) -> Option<DetectedModality> {
        let path = Path::new(&attachment.filename);
        let extension = path.extension()?.to_str()?.to_lowercase();
        
        let modality = self.extension_map.get(&extension)?;
        
        Some(DetectedModality {
            modality: *modality,
            source: ModalitySource::Attachment { 
                filename: attachment.filename.clone() 
            },
            confidence: 1.0,
            details: Some(format!("Detected from .{} extension", extension)),
        })
    }
    
    /// Detect modalities from text content
    pub fn detect_from_text(&self, content: &str) -> Vec<DetectedModality> {
        let mut detections = Vec::new();
        let content_lower = content.to_lowercase();
        
        for (pattern, modality, confidence) in &self.text_patterns {
            if pattern.is_match(&content_lower) {
                // Find the matched text for details
                let matched = pattern.find(&content_lower)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
                
                detections.push(DetectedModality {
                    modality: *modality,
                    source: ModalitySource::TextContent { 
                        matched_pattern: matched 
                    },
                    confidence: *confidence,
                    details: Some(format!("Pattern matched: {}", pattern.as_str())),
                });
            }
        }
        
        detections
    }
    
    /// Detect modalities from existing project context
    pub fn detect_from_context(&self, context: &TaskContext) -> Vec<DetectedModality> {
        let mut detections = Vec::new();
        
        for (graph_id, modality) in &context.existing_modality_graphs {
            detections.push(DetectedModality {
                modality: *modality,
                source: ModalitySource::ProjectContext { 
                    graph_id: graph_id.clone() 
                },
                confidence: 0.8,
                details: Some("Existing modality graph in project".to_string()),
            });
        }
        
        detections
    }
    
    /// Detect modalities from task type
    fn detect_from_task_type(&self, content: &str) -> Vec<DetectedModality> {
        let mut detections = Vec::new();
        let content_lower = content.to_lowercase();
        
        for (pattern, modalities) in &self.task_patterns {
            if pattern.is_match(&content_lower) {
                for modality in modalities {
                    detections.push(DetectedModality {
                        modality: *modality,
                        source: ModalitySource::TaskType { 
                            task_category: pattern.as_str().to_string() 
                        },
                        confidence: 0.7,
                        details: Some("Inferred from task type".to_string()),
                    });
                }
            }
        }
        
        detections
    }
    
    /// Deduplicate detections, keeping highest confidence for each modality
    fn deduplicate_modalities(&self, detections: Vec<DetectedModality>) -> Vec<DetectedModality> {
        let mut best_per_modality: HashMap<ModalityType, DetectedModality> = HashMap::new();
        
        for detection in detections {
            let entry = best_per_modality.entry(detection.modality);
            entry.and_modify(|existing| {
                if detection.confidence > existing.confidence {
                    *existing = detection.clone();
                }
            }).or_insert(detection);
        }
        
        let mut result: Vec<_> = best_per_modality.into_values().collect();
        result.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        result
    }
    
    // =========================================================================
    // Pattern Building
    // =========================================================================
    
    fn build_extension_map() -> HashMap<String, ModalityType> {
        let mut map = HashMap::new();
        
        // Text modality
        for ext in ["txt", "md", "doc", "docx", "pdf", "rtf", "csv", "json", "xml", "html", "htm"] {
            map.insert(ext.to_string(), ModalityType::Text);
        }
        
        // Code modality
        for ext in [
            "rs", "py", "js", "ts", "jsx", "tsx", "go", "java", "c", "cpp", "h", "hpp", 
            "cs", "rb", "php", "swift", "kt", "scala", "r", "jl", "lua", "sh", "bash",
            "sql", "vue", "svelte", "elm", "hs", "ml", "fs", "ex", "exs", "clj", "lisp"
        ] {
            map.insert(ext.to_string(), ModalityType::Code);
        }
        
        // Image modality
        for ext in ["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "tiff", "ico", "heic"] {
            map.insert(ext.to_string(), ModalityType::Image);
        }
        
        // Audio modality
        for ext in ["mp3", "wav", "flac", "ogg", "m4a", "aac", "wma", "aiff"] {
            map.insert(ext.to_string(), ModalityType::Audio);
        }
        
        // Video modality
        for ext in ["mp4", "mov", "avi", "mkv", "webm", "wmv", "flv", "m4v"] {
            map.insert(ext.to_string(), ModalityType::Video);
        }
        
        // Math modality
        for ext in ["tex", "latex", "nb", "m", "mpl", "maple", "mw"] {
            map.insert(ext.to_string(), ModalityType::Math);
        }
        
        // Chemistry modality
        for ext in ["mol", "sdf", "pdb", "xyz", "cif", "mol2", "cml", "cdx"] {
            map.insert(ext.to_string(), ModalityType::Chemistry);
        }
        
        // DNA modality
        for ext in ["fasta", "fastq", "gbk", "gb", "gff", "gtf", "vcf", "sam", "bam", "bed"] {
            map.insert(ext.to_string(), ModalityType::DNA);
        }
        
        // EEG modality
        for ext in ["edf", "bdf", "gdf", "set", "fif", "vhdr", "vmrk", "eeg"] {
            map.insert(ext.to_string(), ModalityType::EEG);
        }
        
        map
    }
    
    fn build_text_patterns() -> Vec<(Regex, ModalityType, f32)> {
        vec![
            // Code patterns
            (Regex::new(r"\b(function|class|def|impl|struct|enum|interface|module)\b").unwrap(), 
             ModalityType::Code, 0.9),
            (Regex::new(r"\b(python|javascript|rust|java|typescript|golang|c\+\+)\b").unwrap(), 
             ModalityType::Code, 0.85),
            (Regex::new(r"```[a-z]*\n").unwrap(), ModalityType::Code, 0.9),
            (Regex::new(r"\b(api|sdk|library|package|import|export)\b").unwrap(), 
             ModalityType::Code, 0.7),
            
            // Math patterns
            (Regex::new(r"\b(theorem|lemma|proof|equation|integral|derivative)\b").unwrap(), 
             ModalityType::Math, 0.9),
            (Regex::new(r"\b(matrix|vector|eigenvalue|polynomial|differential)\b").unwrap(), 
             ModalityType::Math, 0.85),
            (Regex::new(r"\$\$.*\$\$|\\\[.*\\\]").unwrap(), ModalityType::Math, 0.95),
            (Regex::new(r"\b(∫|∑|∏|√|∞|∂|∇)\b").unwrap(), ModalityType::Math, 0.95),
            
            // Chemistry patterns
            (Regex::new(r"\b(molecule|compound|reaction|catalyst|bond|atom)\b").unwrap(), 
             ModalityType::Chemistry, 0.85),
            (Regex::new(r"\b(H2O|CO2|NaCl|C6H12O6|CH4|NH3)\b").unwrap(), 
             ModalityType::Chemistry, 0.9),
            (Regex::new(r"\b(organic|inorganic|synthesis|polymer|enzyme)\b").unwrap(), 
             ModalityType::Chemistry, 0.75),
            
            // DNA patterns
            (Regex::new(r"\b(gene|dna|rna|genome|sequence|mutation|protein)\b").unwrap(), 
             ModalityType::DNA, 0.85),
            (Regex::new(r"\b(ATCG){10,}").unwrap(), ModalityType::DNA, 0.95),
            (Regex::new(r"\b(pcr|crispr|sequencing|alignment|blast)\b").unwrap(), 
             ModalityType::DNA, 0.8),
            
            // EEG patterns
            (Regex::new(r"\b(eeg|brainwave|electrode|channel|frequency band)\b").unwrap(), 
             ModalityType::EEG, 0.9),
            (Regex::new(r"\b(alpha|beta|theta|delta|gamma)\s+(wave|rhythm|band)\b").unwrap(), 
             ModalityType::EEG, 0.85),
            (Regex::new(r"\b(erp|evoked potential|coherence|connectivity)\b").unwrap(), 
             ModalityType::EEG, 0.8),
            
            // Image patterns
            (Regex::new(r"\b(image|picture|photo|screenshot|diagram|chart)\b").unwrap(), 
             ModalityType::Image, 0.7),
            (Regex::new(r"\b(pixel|resolution|rgb|grayscale|crop|resize)\b").unwrap(), 
             ModalityType::Image, 0.75),
            
            // Audio patterns
            (Regex::new(r"\b(audio|sound|music|recording|podcast|voice)\b").unwrap(), 
             ModalityType::Audio, 0.7),
            (Regex::new(r"\b(waveform|frequency|amplitude|sample rate|transcribe)\b").unwrap(), 
             ModalityType::Audio, 0.8),
            
            // Video patterns
            (Regex::new(r"\b(video|movie|clip|footage|frame|scene)\b").unwrap(), 
             ModalityType::Video, 0.7),
            (Regex::new(r"\b(fps|framerate|resolution|codec|subtitle)\b").unwrap(), 
             ModalityType::Video, 0.75),
        ]
    }
    
    fn build_task_patterns() -> Vec<(Regex, Vec<ModalityType>)> {
        vec![
            // Code-related tasks
            (Regex::new(r"\b(write|create|implement|refactor|debug|review)\s+(code|function|class|api)\b").unwrap(),
             vec![ModalityType::Code]),
            (Regex::new(r"\b(fix|bug|error|crash|exception)\b").unwrap(),
             vec![ModalityType::Code]),
            
            // Documentation tasks
            (Regex::new(r"\b(document|readme|tutorial|guide|explain)\b").unwrap(),
             vec![ModalityType::Text, ModalityType::Code]),
            
            // Analysis tasks
            (Regex::new(r"\b(analyze|analyse|examine|inspect)\s+(data|code|text)\b").unwrap(),
             vec![ModalityType::Text, ModalityType::Code]),
            
            // Math tasks
            (Regex::new(r"\b(prove|derive|solve|calculate|compute)\b").unwrap(),
             vec![ModalityType::Math]),
            
            // Scientific tasks
            (Regex::new(r"\b(experiment|hypothesis|statistical|correlation)\b").unwrap(),
             vec![ModalityType::Math, ModalityType::Text]),
        ]
    }
}

// =============================================================================
// Supporting Types
// =============================================================================

/// A task request to be processed
#[derive(Debug, Clone)]
pub struct TaskRequest {
    /// The main content/query
    pub content: String,
    
    /// File attachments
    pub attachments: Vec<Attachment>,
    
    /// Explicitly specified modality (if any)
    pub explicit_modality: Option<ModalityType>,
    
    /// Existing context (project, previous graphs, etc.)
    pub context: Option<TaskContext>,
}

/// A file attachment
#[derive(Debug, Clone)]
pub struct Attachment {
    /// Filename with extension
    pub filename: String,
    
    /// File content (bytes or path)
    pub content: AttachmentContent,
    
    /// File size in bytes
    pub size: usize,
}

#[derive(Debug, Clone)]
pub enum AttachmentContent {
    Bytes(Vec<u8>),
    Path(std::path::PathBuf),
    Url(String),
}

/// Task context with existing modality information
#[derive(Debug, Clone)]
pub struct TaskContext {
    /// Project ID
    pub project_id: Option<String>,
    
    /// Existing modality graphs in the project
    pub existing_modality_graphs: Vec<(String, ModalityType)>,
    
    /// Previous task modalities (for continuity)
    pub previous_modalities: Vec<ModalityType>,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    fn all_modalities() -> Vec<ModalityType> {
        vec![
            ModalityType::Text,
            ModalityType::Code,
            ModalityType::Image,
            ModalityType::Audio,
            ModalityType::Video,
            ModalityType::Math,
            ModalityType::Chemistry,
            ModalityType::DNA,
            ModalityType::EEG,
        ]
    }
    
    #[test]
    fn test_file_extension_detection() {
        let detector = ModalityDetector::new(all_modalities());
        
        // Rust file
        let attachment = Attachment {
            filename: "main.rs".to_string(),
            content: AttachmentContent::Bytes(vec![]),
            size: 100,
        };
        let result = detector.detect_from_file(&attachment);
        assert!(result.is_some());
        assert_eq!(result.unwrap().modality, ModalityType::Code);
        
        // Python file
        let attachment = Attachment {
            filename: "script.py".to_string(),
            content: AttachmentContent::Bytes(vec![]),
            size: 100,
        };
        let result = detector.detect_from_file(&attachment);
        assert_eq!(result.unwrap().modality, ModalityType::Code);
        
        // Image file
        let attachment = Attachment {
            filename: "photo.jpg".to_string(),
            content: AttachmentContent::Bytes(vec![]),
            size: 100,
        };
        let result = detector.detect_from_file(&attachment);
        assert_eq!(result.unwrap().modality, ModalityType::Image);
        
        // DNA file
        let attachment = Attachment {
            filename: "sequence.fasta".to_string(),
            content: AttachmentContent::Bytes(vec![]),
            size: 100,
        };
        let result = detector.detect_from_file(&attachment);
        assert_eq!(result.unwrap().modality, ModalityType::DNA);
    }
    
    #[test]
    fn test_text_pattern_detection() {
        let detector = ModalityDetector::new(all_modalities());
        
        // Code content
        let detections = detector.detect_from_text("Please write a function that calculates fibonacci");
        assert!(detections.iter().any(|d| d.modality == ModalityType::Code));
        
        // Math content
        let detections = detector.detect_from_text("Prove the theorem about eigenvalues");
        assert!(detections.iter().any(|d| d.modality == ModalityType::Math));
        
        // Chemistry content
        let detections = detector.detect_from_text("What is the structure of H2O molecule?");
        assert!(detections.iter().any(|d| d.modality == ModalityType::Chemistry));
        
        // DNA content
        let detections = detector.detect_from_text("Analyze this gene sequence for mutations");
        assert!(detections.iter().any(|d| d.modality == ModalityType::DNA));
    }
    
    #[test]
    fn test_full_request_detection() {
        let detector = ModalityDetector::new(all_modalities());
        
        let request = TaskRequest {
            content: "Review this Python code for bugs".to_string(),
            attachments: vec![
                Attachment {
                    filename: "main.py".to_string(),
                    content: AttachmentContent::Bytes(vec![]),
                    size: 100,
                }
            ],
            explicit_modality: None,
            context: None,
        };
        
        let detections = detector.detect_modalities(&request);
        
        // Should detect code from both text and attachment
        assert!(!detections.is_empty());
        assert!(detections.iter().any(|d| d.modality == ModalityType::Code));
        
        // Highest confidence should be from file attachment
        let code_detection = detections.iter()
            .find(|d| d.modality == ModalityType::Code)
            .unwrap();
        assert_eq!(code_detection.confidence, 1.0);
    }
    
    #[test]
    fn test_deduplication() {
        let detector = ModalityDetector::new(all_modalities());
        
        // Request with multiple code signals
        let request = TaskRequest {
            content: "Write a Python function to parse JSON".to_string(),
            attachments: vec![
                Attachment {
                    filename: "parser.py".to_string(),
                    content: AttachmentContent::Bytes(vec![]),
                    size: 100,
                }
            ],
            explicit_modality: None,
            context: None,
        };
        
        let detections = detector.detect_modalities(&request);
        
        // Should only have one Code entry (deduplicated)
        let code_count = detections.iter()
            .filter(|d| d.modality == ModalityType::Code)
            .count();
        assert_eq!(code_count, 1);
    }
    
    #[test]
    fn test_disabled_modalities() {
        // Only enable Code and Text
        let detector = ModalityDetector::new(vec![
            ModalityType::Code,
            ModalityType::Text,
        ]);
        
        let request = TaskRequest {
            content: "Analyze this gene sequence".to_string(),
            attachments: vec![
                Attachment {
                    filename: "sequence.fasta".to_string(),
                    content: AttachmentContent::Bytes(vec![]),
                    size: 100,
                }
            ],
            explicit_modality: None,
            context: None,
        };
        
        let detections = detector.detect_modalities(&request);
        
        // Should NOT detect DNA since it's not enabled
        assert!(!detections.iter().any(|d| d.modality == ModalityType::DNA));
    }
}
