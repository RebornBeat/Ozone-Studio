//! Modality types and detection
//!
//! Defines all supported modalities and file extension mappings.

use serde::{Deserialize, Serialize};

/// Supported modality types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModalityType {
    Text,
    Code,
    Image,
    Audio,
    Video,
    Math,
    Chemistry,
    DNA,
    EEG,
    Unknown,
}

impl ModalityType {
    /// Get file extensions for this modality
    pub fn file_extensions(&self) -> &[&str] {
        match self {
            ModalityType::Text => &[
                "txt", "md", "doc", "docx", "pdf", "rtf", "csv", "json", "xml", "html", "htm",
                "rst", "adoc", "org", "tex",
            ],
            ModalityType::Code => &[
                "rs",
                "py",
                "js",
                "ts",
                "jsx",
                "tsx",
                "go",
                "java",
                "c",
                "cpp",
                "h",
                "hpp",
                "cs",
                "rb",
                "php",
                "swift",
                "kt",
                "scala",
                "r",
                "jl",
                "lua",
                "sh",
                "bash",
                "zsh",
                "fish",
                "ps1",
                "bat",
                "cmd",
                "sql",
                "graphql",
                "vue",
                "svelte",
                "elm",
                "clj",
                "ex",
                "exs",
                "erl",
                "hs",
                "ml",
                "fs",
                "nim",
                "zig",
                "v",
                "d",
                "ada",
                "pas",
                "pl",
                "pm",
                "tcl",
                "awk",
                "sed",
                "makefile",
                "cmake",
                "dockerfile",
                "yaml",
                "yml",
                "toml",
                "ini",
                "conf",
                "cfg",
            ],
            ModalityType::Image => &[
                "png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "tiff", "tif", "ico", "heic",
                "heif", "raw", "cr2", "nef", "arw", "dng", "psd", "ai", "eps",
            ],
            ModalityType::Audio => &[
                "mp3", "wav", "flac", "ogg", "m4a", "aac", "wma", "aiff", "aif", "opus", "mid",
                "midi", "ra", "ram",
            ],
            ModalityType::Video => &[
                "mp4", "mov", "avi", "mkv", "webm", "wmv", "flv", "m4v", "mpeg", "mpg", "3gp",
                "ogv", "ts", "mts", "m2ts",
            ],
            ModalityType::Math => &[
                "tex", "latex", "nb", "m", "mpl", "maple", "mw", "wxm", "wxmx", "sage", "ipynb",
                "rmd", "qmd",
            ],
            ModalityType::Chemistry => &[
                "mol", "sdf", "pdb", "xyz", "cif", "mol2", "cml", "cdx", "cdxml", "rxn", "smi",
                "smiles", "inchi", "ct", "mae", "maegz",
            ],
            ModalityType::DNA => &[
                "fasta", "fa", "fna", "ffn", "faa", "frn", "fastq", "fq", "gbk", "gb", "genbank",
                "gff", "gff3", "gtf", "vcf", "sam", "bam", "cram", "bed", "wig", "bigwig", "bw",
                "2bit", "nex", "nexus", "phy", "phylip", "aln",
            ],
            ModalityType::EEG => &[
                "edf", "bdf", "gdf", "set", "fif", "vhdr", "vmrk", "eeg", "cnt", "avg", "mff",
                "ncs", "nev", "ns2", "ns3", "ns4", "ns5", "ns6", "plx",
            ],
            ModalityType::Unknown => &[],
        }
    }

    /// Detect modality from file extension
    pub fn from_extension(ext: &str) -> Self {
        let ext_lower = ext.to_lowercase();
        let ext_str = ext_lower.trim_start_matches('.');

        // Check each modality's extensions
        for modality in Self::all() {
            if modality.file_extensions().contains(&ext_str) {
                return *modality;
            }
        }

        ModalityType::Unknown
    }

    /// Detect modality from file path
    pub fn from_path(path: &str) -> Self {
        std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .map(Self::from_extension)
            .unwrap_or(ModalityType::Unknown)
    }

    /// Get all modality types
    pub fn all() -> &'static [ModalityType] {
        &[
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

    /// Get the pipeline ID for this modality
    pub fn pipeline_id(&self) -> Option<u64> {
        match self {
            ModalityType::Text => Some(100),
            ModalityType::Code => Some(101),
            ModalityType::Image => Some(102),
            ModalityType::Audio => Some(103),
            ModalityType::Video => Some(104),
            ModalityType::Math => Some(105),
            ModalityType::Chemistry => Some(106),
            ModalityType::DNA => Some(107),
            ModalityType::EEG => Some(108),
            ModalityType::Unknown => None,
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            ModalityType::Text => "Text",
            ModalityType::Code => "Code",
            ModalityType::Image => "Image",
            ModalityType::Audio => "Audio",
            ModalityType::Video => "Video",
            ModalityType::Math => "Math",
            ModalityType::Chemistry => "Chemistry",
            ModalityType::DNA => "DNA/Genomics",
            ModalityType::EEG => "EEG/Neural",
            ModalityType::Unknown => "Unknown",
        }
    }
}

impl Default for ModalityType {
    fn default() -> Self {
        ModalityType::Unknown
    }
}

impl std::fmt::Display for ModalityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Modality detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalityDetection {
    pub modality: ModalityType,
    pub confidence: f32,
    pub extension: Option<String>,
    pub mime_type: Option<String>,
}

/// Detect modality from multiple sources
pub fn detect_modality(
    path: Option<&str>,
    mime_type: Option<&str>,
    content_sample: Option<&[u8]>,
) -> ModalityDetection {
    // First try extension
    if let Some(p) = path {
        let modality = ModalityType::from_path(p);
        if modality != ModalityType::Unknown {
            return ModalityDetection {
                modality,
                confidence: 0.9,
                extension: std::path::Path::new(p)
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(String::from),
                mime_type: mime_type.map(String::from),
            };
        }
    }

    // Try mime type
    if let Some(mime) = mime_type {
        let modality = match mime.split('/').next() {
            Some("text") => ModalityType::Text,
            Some("image") => ModalityType::Image,
            Some("audio") => ModalityType::Audio,
            Some("video") => ModalityType::Video,
            Some("application") => {
                // Check specific application types
                if mime.contains("json") || mime.contains("xml") {
                    ModalityType::Text
                } else if mime.contains("pdf") {
                    ModalityType::Text
                } else {
                    ModalityType::Unknown
                }
            }
            _ => ModalityType::Unknown,
        };

        if modality != ModalityType::Unknown {
            return ModalityDetection {
                modality,
                confidence: 0.7,
                extension: path.and_then(|p| {
                    std::path::Path::new(p)
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(String::from)
                }),
                mime_type: Some(mime.to_string()),
            };
        }
    }

    // Content-based detection (basic)
    if let Some(sample) = content_sample {
        // Check for text (valid UTF-8)
        if std::str::from_utf8(sample).is_ok() {
            return ModalityDetection {
                modality: ModalityType::Text,
                confidence: 0.5,
                extension: None,
                mime_type: None,
            };
        }
    }

    ModalityDetection {
        modality: ModalityType::Unknown,
        confidence: 0.0,
        extension: None,
        mime_type: None,
    }
}
