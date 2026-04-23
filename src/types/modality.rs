//! Modality types and detection
//!
//! Defines all supported modalities and file extension mappings.
//! Mirrors the pipeline registry: pipeline_id = modality pipeline number.

use serde::{Deserialize, Serialize};

/// All supported modality types.
///
/// The discriminant intentionally matches the pipeline ID where applicable
/// (100–126) so that `pipeline_id()` is a simple cast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModalityType {
    // ── Core modalities (pipelines 100–108) ──────────────────────────────────
    Text,
    Code,
    Image,
    Audio,
    Video,
    Math,
    Chemistry,
    DNA,
    EEG,

    // ── Engine / extended modalities (pipelines 109–126) ─────────────────────
    ThreeD,          // 3D scene / engine  (pipeline 109)
    Sound,           // sound reconstruction (pipeline 110)
    Biology,         // multi-scale biology  (pipeline 111)
    Proteomics,      // protein / interaction (pipeline 112)
    Haptic,          // force / texture       (pipeline 113)
    Thermal,         // thermal imaging       (pipeline 114)
    Depth,           // depth / point cloud   (pipeline 115)
    IMU,             // inertial sensors      (pipeline 116)
    Geospatial,      // mapping / geo         (pipeline 117)
    Electromagnetic, // passive EM / RF       (pipeline 118)
    BCI,             // brain-computer iface  (pipeline 119)
    ParametricCAD,   // B-rep / CAD           (pipeline 120)
    Kinematics,      // kinematic chains      (pipeline 121)
    ControlSystems,  // control theory        (pipeline 122)
    NetworkTopology, // network graphs        (pipeline 123)
    ActiveRadar,     // active radar sensing  (pipeline 124)
    ActiveSonar,     // active sonar sensing  (pipeline 125)
    Hyperspectral,   // hyperspectral imaging (pipeline 126)

    Unknown,
}

impl ModalityType {
    /// Get file extensions that map to this modality.
    pub fn file_extensions(&self) -> &[&str] {
        match self {
            // ── Core ───────────────────────────────────────────────────────────
            ModalityType::Text => &[
                "txt", "md", "doc", "docx", "pdf", "rtf", "rst", "adoc", "org", "tex",
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
                "json",
                "jsonl",
                "xml",
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
                "tex", "latex", "nb", "mpl", "maple", "mw", "wxm", "wxmx", "sage", "ipynb", "rmd",
                "qmd",
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

            // ── Extended ───────────────────────────────────────────────────────
            ModalityType::ThreeD => &[
                "blend", "obj", "glb", "gltf", "fbx", "dae", "stl", "ply", "usd", "usda", "usdc",
                "usdz", "3ds", "c4d", "ma", "mb", "urdf", "mjcf", "sdf", "world",
            ],
            ModalityType::Sound => &[
                // raw audio used specifically for vocalization / species reconstruction
                // NOTE: overlaps with Audio intentionally — routing is context-dependent
                "wav", "flac", "ogg", "opus", "aiff",
            ],
            ModalityType::Biology => &[
                // structured biology data formats beyond raw DNA
                "sbml", "owl", "obo", "biopax", "rxnml", "hmdb", "kegg",
            ],
            ModalityType::Proteomics => &[
                "pdb", "cif", "mmcif", "sdf", "mol2", "mae", "maegz", "mrc", "map", "dx", "ccp4",
                "xplor", "mzml", "mzxml", "raw", "wiff",
            ],
            ModalityType::Haptic => &["hdf5", "h5", "haptic", "force", "mat"],
            ModalityType::Thermal => &[
                // thermal-specific TIFF variants and camera formats
                "tif", "tiff", "rjpeg", "fff", "seq", "csq", "ir2",
            ],
            ModalityType::Depth => &["pcd", "las", "laz", "pts", "xyz", "ply", "e57", "zdf"],
            ModalityType::IMU => &[
                "imu", "csv", // .csv matched only when path indicates IMU context
                "bag", "mcap", // ROS bag files
            ],
            ModalityType::Geospatial => &[
                "geojson", "kml", "kmz", "shp", "dbf", "shx", "prj", "gpx", "osm", "pbf", "dem",
                "asc", "grd", "nc", "tif",
            ],
            ModalityType::Electromagnetic => &["iq", "rf", "sigmf", "cf32", "cs16"],
            ModalityType::BCI => &[
                // BCI reuses EEG formats + decoded outputs
                "edf", "bdf", "gdf", "fif", "set", "xdf", "bcilab",
            ],
            ModalityType::ParametricCAD => &[
                "step", "stp", "iges", "igs", "brep", "f3d", "fcstd", "sat", "x_t", "x_b",
            ],
            ModalityType::Kinematics => &["urdf", "mjcf", "sdf", "kin", "dh"],
            ModalityType::ControlSystems => &["slx", "mdl", "m", "simulink", "fmu", "fmi"],
            ModalityType::NetworkTopology => {
                &["graphml", "gexf", "dot", "gml", "net", "pajek", "cyjs"]
            }
            ModalityType::ActiveRadar => {
                &["iq", "bin", "cphd", "sicd", "nitf", "ntf", "rs2", "tiff"]
            }
            ModalityType::ActiveSonar => &["xtf", "jsf", "s7k", "gsf", "all", "wcd", "xse", "xtf"],
            ModalityType::Hyperspectral => &[
                "hdr", "bil", "bip", "bsq", "h4", "hdf4", "hdf5", "h5", "nc", "img",
            ],

            ModalityType::Unknown => &[],
        }
    }

    /// Detect modality from file extension (case-insensitive, no leading dot needed).
    pub fn from_extension(ext: &str) -> Self {
        let ext_lower = ext.to_lowercase();
        let ext_str = ext_lower.trim_start_matches('.');

        // Ambiguous extensions: check most-specific modalities first
        // (.tif can be Thermal or Geospatial — Geospatial wins for generic tif;
        //  thermal files typically come with path hints handled at routing layer)
        for modality in Self::all() {
            if modality.file_extensions().contains(&ext_str) {
                return *modality;
            }
        }

        ModalityType::Unknown
    }

    /// Detect modality from file path (uses extension).
    pub fn from_path(path: &str) -> Self {
        std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .map(Self::from_extension)
            .unwrap_or(ModalityType::Unknown)
    }

    /// All non-Unknown modality types in pipeline order.
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
            ModalityType::ThreeD,
            ModalityType::Sound,
            ModalityType::Biology,
            ModalityType::Proteomics,
            ModalityType::Haptic,
            ModalityType::Thermal,
            ModalityType::Depth,
            ModalityType::IMU,
            ModalityType::Geospatial,
            ModalityType::Electromagnetic,
            ModalityType::BCI,
            ModalityType::ParametricCAD,
            ModalityType::Kinematics,
            ModalityType::ControlSystems,
            ModalityType::NetworkTopology,
            ModalityType::ActiveRadar,
            ModalityType::ActiveSonar,
            ModalityType::Hyperspectral,
        ]
    }

    /// Pipeline ID for this modality, if it has a dedicated pipeline.
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
            ModalityType::ThreeD => Some(109),
            ModalityType::Sound => Some(110),
            ModalityType::Biology => Some(111),
            ModalityType::Proteomics => Some(112),
            ModalityType::Haptic => Some(113),
            ModalityType::Thermal => Some(114),
            ModalityType::Depth => Some(115),
            ModalityType::IMU => Some(116),
            ModalityType::Geospatial => Some(117),
            ModalityType::Electromagnetic => Some(118),
            ModalityType::BCI => Some(119),
            ModalityType::ParametricCAD => Some(120),
            ModalityType::Kinematics => Some(121),
            ModalityType::ControlSystems => Some(122),
            ModalityType::NetworkTopology => Some(123),
            ModalityType::ActiveRadar => Some(124),
            ModalityType::ActiveSonar => Some(125),
            ModalityType::Hyperspectral => Some(126),
            ModalityType::Unknown => None,
        }
    }

    /// Whether this modality has a dedicated AGI engine (headless + optional UI).
    pub fn is_engine(&self) -> bool {
        matches!(
            self,
            ModalityType::ThreeD
                | ModalityType::Math
                | ModalityType::Chemistry
                | ModalityType::DNA
                | ModalityType::EEG
                | ModalityType::Geospatial
                | ModalityType::BCI
                | ModalityType::ParametricCAD
                | ModalityType::ActiveRadar
                | ModalityType::ActiveSonar
                | ModalityType::Hyperspectral
        )
    }

    /// Human-readable display name.
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
            ModalityType::EEG => "EEG/Neural Signal",
            ModalityType::ThreeD => "3D Scene",
            ModalityType::Sound => "Sound Reconstruction",
            ModalityType::Biology => "Multi-Scale Biology",
            ModalityType::Proteomics => "Proteomics",
            ModalityType::Haptic => "Haptic",
            ModalityType::Thermal => "Thermal Imaging",
            ModalityType::Depth => "Depth / Point Cloud",
            ModalityType::IMU => "IMU / Inertial",
            ModalityType::Geospatial => "Geospatial / Mapping",
            ModalityType::Electromagnetic => "Passive EM / RF",
            ModalityType::BCI => "Brain-Computer Interface",
            ModalityType::ParametricCAD => "Parametric CAD",
            ModalityType::Kinematics => "Kinematics",
            ModalityType::ControlSystems => "Control Systems",
            ModalityType::NetworkTopology => "Network Topology",
            ModalityType::ActiveRadar => "Active Radar Sensing",
            ModalityType::ActiveSonar => "Active Sonar Sensing",
            ModalityType::Hyperspectral => "Hyperspectral / Multispectral",
            ModalityType::Unknown => "Unknown",
        }
    }

    /// Short identifier used in ZSEI paths and folder names.
    pub fn path_name(&self) -> &'static str {
        match self {
            ModalityType::Text => "text",
            ModalityType::Code => "code",
            ModalityType::Image => "image",
            ModalityType::Audio => "audio",
            ModalityType::Video => "video",
            ModalityType::Math => "math",
            ModalityType::Chemistry => "chemistry",
            ModalityType::DNA => "dna",
            ModalityType::EEG => "eeg",
            ModalityType::ThreeD => "3d",
            ModalityType::Sound => "sound",
            ModalityType::Biology => "biology",
            ModalityType::Proteomics => "proteomics",
            ModalityType::Haptic => "haptic",
            ModalityType::Thermal => "thermal",
            ModalityType::Depth => "depth",
            ModalityType::IMU => "imu",
            ModalityType::Geospatial => "geospatial",
            ModalityType::Electromagnetic => "electromagnetic",
            ModalityType::BCI => "bci",
            ModalityType::ParametricCAD => "parametric_cad",
            ModalityType::Kinematics => "kinematics",
            ModalityType::ControlSystems => "control_systems",
            ModalityType::NetworkTopology => "network_topology",
            ModalityType::ActiveRadar => "radar",
            ModalityType::ActiveSonar => "sonar",
            ModalityType::Hyperspectral => "hyperspectral",
            ModalityType::Unknown => "unknown",
        }
    }

    /// Construct from a path name string (inverse of `path_name()`).
    pub fn from_path_name(s: &str) -> Self {
        match s {
            "text" => Self::Text,
            "code" => Self::Code,
            "image" => Self::Image,
            "audio" => Self::Audio,
            "video" => Self::Video,
            "math" => Self::Math,
            "chemistry" => Self::Chemistry,
            "dna" => Self::DNA,
            "eeg" => Self::EEG,
            "3d" => Self::ThreeD,
            "sound" => Self::Sound,
            "biology" => Self::Biology,
            "proteomics" => Self::Proteomics,
            "haptic" => Self::Haptic,
            "thermal" => Self::Thermal,
            "depth" => Self::Depth,
            "imu" => Self::IMU,
            "geospatial" => Self::Geospatial,
            "electromagnetic" => Self::Electromagnetic,
            "bci" => Self::BCI,
            "parametric_cad" => Self::ParametricCAD,
            "kinematics" => Self::Kinematics,
            "control_systems" => Self::ControlSystems,
            "network_topology" => Self::NetworkTopology,
            "radar" => Self::ActiveRadar,
            "sonar" => Self::ActiveSonar,
            "hyperspectral" => Self::Hyperspectral,
            _ => Self::Unknown,
        }
    }

    /// Construct from a pipeline ID.
    pub fn from_pipeline_id(id: u64) -> Self {
        match id {
            100 => Self::Text,
            101 => Self::Code,
            102 => Self::Image,
            103 => Self::Audio,
            104 => Self::Video,
            105 => Self::Math,
            106 => Self::Chemistry,
            107 => Self::DNA,
            108 => Self::EEG,
            109 => Self::ThreeD,
            110 => Self::Sound,
            111 => Self::Biology,
            112 => Self::Proteomics,
            113 => Self::Haptic,
            114 => Self::Thermal,
            115 => Self::Depth,
            116 => Self::IMU,
            117 => Self::Geospatial,
            118 => Self::Electromagnetic,
            119 => Self::BCI,
            120 => Self::ParametricCAD,
            121 => Self::Kinematics,
            122 => Self::ControlSystems,
            123 => Self::NetworkTopology,
            124 => Self::ActiveRadar,
            125 => Self::ActiveSonar,
            126 => Self::Hyperspectral,
            _ => Self::Unknown,
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

// ============================================================================
// MODALITY DETECTION
// ============================================================================

/// Result of modality detection from multiple signal sources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalityDetection {
    pub modality: ModalityType,
    pub extension: Option<String>,
    pub mime_type: Option<String>,
    pub detection_source: DetectionSource,
}

/// How the modality was determined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DetectionSource {
    #[default]
    Extension,
    MimeType,
    ContentSample,
    PathHint, // e.g. path contains "imu", "radar", "sonar"
    Unknown,
}

/// Detect modality from multiple signals, ordered by specificity.
pub fn detect_modality(
    path: Option<&str>,
    mime_type: Option<&str>,
    content_sample: Option<&[u8]>,
) -> ModalityDetection {
    // 1. Path hints (highest specificity for ambiguous extensions)
    if let Some(p) = path {
        let p_lower = p.to_lowercase();
        let hint = if p_lower.contains("imu")
            || p_lower.contains("accelerom")
            || p_lower.contains("gyro")
        {
            Some(ModalityType::IMU)
        } else if p_lower.contains("radar") || p_lower.contains("sar") || p_lower.contains("rdm") {
            Some(ModalityType::ActiveRadar)
        } else if p_lower.contains("sonar")
            || p_lower.contains("xtf")
            || p_lower.contains("bathymet")
        {
            Some(ModalityType::ActiveSonar)
        } else if p_lower.contains("thermal")
            || p_lower.contains("infrared")
            || p_lower.contains("flir")
        {
            Some(ModalityType::Thermal)
        } else if p_lower.contains("hyperspectral")
            || p_lower.contains("multispectral")
            || p_lower.contains("aviris")
        {
            Some(ModalityType::Hyperspectral)
        } else {
            None
        };

        if let Some(modality) = hint {
            return ModalityDetection {
                modality,
                extension: extension_from_path(p),
                mime_type: mime_type.map(String::from),
                detection_source: DetectionSource::PathHint,
            };
        }
    }

    // 2. File extension
    if let Some(p) = path {
        let modality = ModalityType::from_path(p);
        if modality != ModalityType::Unknown {
            return ModalityDetection {
                modality,
                extension: extension_from_path(p),
                mime_type: mime_type.map(String::from),
                detection_source: DetectionSource::Extension,
            };
        }
    }

    // 3. MIME type
    if let Some(mime) = mime_type {
        let modality = match mime.split('/').next() {
            Some("text") => ModalityType::Text,
            Some("image") => ModalityType::Image,
            Some("audio") => ModalityType::Audio,
            Some("video") => ModalityType::Video,
            Some("application") => {
                if mime.contains("json") || mime.contains("xml") || mime.contains("pdf") {
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
                extension: path.and_then(|p| extension_from_path(p)),
                mime_type: Some(mime.to_string()),
                detection_source: DetectionSource::MimeType,
            };
        }
    }

    // 4. Content-based (basic UTF-8 check → text)
    if let Some(sample) = content_sample {
        if std::str::from_utf8(sample).is_ok() {
            return ModalityDetection {
                modality: ModalityType::Text,
                extension: None,
                mime_type: None,
                detection_source: DetectionSource::ContentSample,
            };
        }
    }

    ModalityDetection {
        modality: ModalityType::Unknown,
        extension: None,
        mime_type: None,
        detection_source: DetectionSource::Unknown,
    }
}

fn extension_from_path(path: &str) -> Option<String> {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(String::from)
}
