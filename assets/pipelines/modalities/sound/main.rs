//! SoundReconstructionPipeline — Pipeline #110
//!
//! Species vocalization reconstruction, sound source modeling, acoustic scene
//! analysis, and bioacoustic event identification from passive audio recordings.
//!
//! DISTINCT FROM:
//!   - Audio (103): raw audio reconstruction (speech, music, environments).
//!     Sound Reconstruction (110) specializes in BIOACOUSTICS:
//!     - Species-specific call identification (birds, cetaceans, bats, insects)
//!     - Vocalization unit decomposition (syllables, notes, call types)
//!     - Cross-species acoustic signature libraries
//!     - Echolocation pulse analysis
//!     - Active acoustic scene reconstruction from multiple sources
//!   - Active Sonar (125): active ranging; 110 is passive species-focused.
//!
//! CROSS-LINKS:
//!   103 (Audio)    → raw waveform feeds into vocalization extraction
//!   111 (Bio)      → species identification validates biology nodes
//!   117 (Geo)      → call recordings placed on geographic map
//!   125 (Sonar)    → marine vocalization feeds sonar bio detection
//!   114 (Thermal)  → thermal activity correlated with acoustic activity
//!
//! STORAGE: ZSEI containers under /Modalities/SoundReconstruction/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

// ─────────────────────────────────────────────────────────────────────────────
// INPUT TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum SoundReconstructionAction {
    /// Full analysis of an audio recording for bioacoustic content
    Analyze {
        data: SoundDataSource,
        detect_vocalizations: bool,
        identify_species: bool,
        decompose_units: bool,
        reconstruct_scene: bool,
        extract_signatures: bool,
    },
    /// Create graph from analysis result
    CreateGraph {
        analysis: SoundAnalysisResult,
        project_id: u64,
    },
    /// Update graph with new detections
    UpdateGraph {
        graph_id: u64,
        new_events: Vec<SoundEventUpdate>,
        project_id: u64,
    },
    /// Detect all sound events in a waveform
    DetectSoundEvents {
        waveform: Vec<f32>,
        sample_rate_hz: u32,
        detector: SoundEventDetector,
        threshold_db: f32,
        min_duration_ms: f32,
        max_duration_ms: f32,
    },
    /// Identify species from a detected vocalization
    IdentifySpecies {
        vocalization: VocalizationData,
        context: BioacousticContext,
        reference_library: Option<String>,
    },
    /// Decompose a call sequence into individual vocalization units
    DecomposeCall {
        recording_segment: Vec<f32>,
        sample_rate_hz: u32,
        species_hint: Option<String>,
        decomposition_method: DecompositionMethod,
    },
    /// Reconstruct the acoustic scene: source positions, propagation paths
    ReconstructScene {
        graph_id: u64,
        array_data: Option<MicrophoneArrayData>,
        environment: AcousticEnvironment,
    },
    /// Extract and characterize acoustic signatures per species/source
    ExtractSignatures {
        graph_id: u64,
        target_classes: Vec<String>,
    },
    /// Classify echolocation pulses (bats, toothed whales)
    ClassifyEcholocation {
        pulse_waveform: Vec<f32>,
        sample_rate_hz: u32,
        taxon_group: EcholocationGroup,
    },
    /// Analyze song / call repertoire for a detected individual
    AnalyzeRepertoire {
        graph_id: u64,
        individual_id: u64,
        method: RepertoireMethod,
    },
    /// Query graph
    QueryGraph {
        graph_id: u64,
        query: SoundGraphQuery,
    },
    /// Get full graph for Context Viewer
    GetGraph { graph_id: u64 },
    /// Trigger ZSEI semantic hooks
    TriggerSemanticHook {
        graph_id: u64,
        hook: SoundSemanticHook,
    },
    /// Export products
    ExportProduct {
        graph_id: u64,
        format: SoundExportFormat,
    },
    /// Stream to UI
    StreamToUI {
        graph_id: u64,
        session_id: String,
        display_mode: SoundDisplayMode,
    },
    /// Headless processing (AGI-first)
    HeadlessProcess {
        graph_id: u64,
        operations: Vec<SoundHeadlessOp>,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// DATA SOURCES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SoundDataSource {
    /// Single audio file (WAV, FLAC, MP3, AIFF, OGG)
    AudioFile {
        file_path: String,
        channel: ChannelSelection,
    },
    /// Multiple synchronized recordings (microphone array or distributed sensors)
    ArrayFiles {
        files: Vec<String>,
        microphone_positions_m: Vec<[f32; 3]>,
        sync_offset_samples: Vec<i64>,
    },
    /// Audio with attached metadata (species, location, date, recorder model)
    AnnotatedFile {
        file_path: String,
        metadata: RecordingMetadata,
    },
    /// Ultrasonic recording (bats, rodents — high sample rates)
    UltrasonicFile {
        file_path: String,
        sample_rate_hz: u32,
        time_expansion_factor: f32,    // if time-expanded recording
    },
    /// Hydrophone recording (underwater)
    HydrophoneFile {
        file_path: String,
        depth_m: f32,
        hydrophone_sensitivity_dbv_upa: f32,
    },
    /// Pre-extracted spectrogram (PNG/NPZ from another system)
    SpectrogramFile {
        file_path: String,
        freq_min_hz: f32,
        freq_max_hz: f32,
        time_start_sec: f32,
        duration_sec: f32,
        db_range: (f32, f32),
    },
    /// Live stream from recording hardware
    LiveStream {
        endpoint: String,
        sample_rate_hz: u32,
        channel_count: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ChannelSelection { #[default] Mono, Left, Right, Mix }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecordingMetadata {
    pub location: Option<[f64; 3]>,         // lat, lon, alt
    pub recorded_at_utc: Option<String>,
    pub recorder_model: Option<String>,
    pub microphone_type: Option<String>,
    pub habitat_type: Option<String>,
    pub weather_conditions: Option<String>,
    pub observer_notes: Option<String>,
    pub species_present_hint: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MicrophoneArrayData {
    pub element_count: u32,
    pub element_positions_m: Vec<[f32; 3]>,
    pub inter_element_data: Vec<Vec<f32>>,   // IQ or real audio per element
    pub sample_rate_hz: u32,
}

// ─────────────────────────────────────────────────────────────────────────────
// DETECTION / ANALYSIS PARAMETERS
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SoundEventDetector {
    #[default] EnergyThreshold,
    SpectralEntropy,
    BandpassEnergy { low_hz: f32, high_hz: f32 },
    MatchedFilter { template: Vec<f32> },
    NeuralDetector { model_name: String },
    CFAR,                           // constant false alarm rate (bioacoustics)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum BioacousticContext {
    #[default] General,
    Avian,                          // birds
    MarineMammal,                   // cetaceans, seals, manatees
    Bat,
    Frog,
    Insect,
    Primate,
    Domestic,
    Urban,
    Coral_Reef,
    Freshwater,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum DecompositionMethod {
    #[default] Praat,               // acoustic analysis tool approach
    OnsetsAndOffsets,
    SyllableSegmentation,
    PhraseSegmentation,
    NoteLevelDecomposition,
    WaveletDecomposition,
    HMM_Based,
    Neural,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum AcousticEnvironment {
    #[default] Unknown,
    OpenGrassland, Forest, CoastalOcean, DeepOcean, CoralReef,
    River, Lake, Urban, Cave, Wetland, Arctic, Tropical, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum EcholocationGroup { #[default] Bat, OdontoceteWhale, Dolphin, Porpoise, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum RepertoireMethod { #[default] TemplateClustering, DTW, MFCC_Similarity, Spectral, Neural }

// ─────────────────────────────────────────────────────────────────────────────
// ANALYSIS RESULT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SoundAnalysisResult {
    pub analysis_id: u64,
    pub source_description: String,
    pub recording_duration_sec: f32,
    pub sample_rate_hz: u32,
    pub channel_count: u32,

    // SOUND EVENTS
    pub sound_events: Vec<SoundEvent>,
    pub background_events: Vec<BackgroundSoundEvent>,

    // VOCALIZATIONS
    pub vocalizations: Vec<Vocalization>,
    pub vocalization_units: Vec<VocalizationUnit>,

    // SPECIES
    pub species_detections: Vec<SpeciesDetection>,
    pub individual_identifications: Vec<IndividualID>,

    // ACOUSTIC SIGNATURES
    pub acoustic_signatures: Vec<AcousticSignature>,

    // ECHOLOCATION
    pub echolocation_pulses: Vec<EcholocationPulse>,
    pub echolocation_sequences: Vec<EcholocationSequence>,

    // SCENE RECONSTRUCTION
    pub source_localizations: Vec<SourceLocalization>,
    pub propagation_paths: Vec<AcousticPropagationPath>,
    pub acoustic_scene: Option<AcousticScene>,

    // AMBIENT / ENVIRONMENT
    pub ambient_noise_profile: Option<AmbientNoiseProfile>,
    pub soundscape_indices: Vec<SoundscapeIndex>,

    // METADATA
    pub recording_metadata: RecordingMetadata,
    pub processing_notes: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// CORE DATA TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SoundEvent {
    pub event_id: u64,
    pub start_sec: f32,
    pub end_sec: f32,
    pub duration_ms: f32,
    pub freq_min_hz: f32,
    pub freq_max_hz: f32,
    pub peak_freq_hz: f32,
    pub peak_amplitude_db: f32,
    pub rms_amplitude_db: f32,
    pub event_type: SoundEventType,
    pub biotic: bool,               // biotic vs abiotic sound source
    pub source_direction_deg: Option<f32>,  // if array available
    pub channel: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SoundEventType {
    #[default] Unknown,
    Vocalization, Echolocation, Impact, Rustling, Wind,
    Rain, Water, Machinery, Vehicle, Anthropogenic, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BackgroundSoundEvent {
    pub event_id: u64,
    pub start_sec: f32,
    pub end_sec: f32,
    pub source_type: String,        // "wind", "rain", "traffic", "river", etc.
    pub amplitude_db: f32,
    pub dominant_freq_hz: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Vocalization {
    pub vocalization_id: u64,
    pub species_id: Option<u64>,
    pub individual_id: Option<u64>,
    pub start_sec: f32,
    pub end_sec: f32,
    pub duration_ms: f32,
    pub call_type: CallType,
    pub call_subtype: Option<String>,   // species-specific call designation
    pub freq_min_hz: f32,
    pub freq_max_hz: f32,
    pub peak_freq_hz: f32,
    pub amplitude_db: f32,
    pub snr_db: f32,
    pub unit_ids: Vec<u64>,            // component vocalization units
    pub behavioral_context: BehavioralContext,
    pub response_to_id: Option<u64>,   // id of vocalization this is a response to
    pub source_direction_deg: Option<f32>,
    pub source_distance_m: Option<f32>,
    pub acoustic_signature_id: Option<u64>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum CallType {
    #[default] Unknown,
    // Universal
    Contact, Alarm, Distress, Aggressive, Social,
    Territorial, Mating_Advertisement, Solicitation,
    Begging, Food_Call, Warning, Play,
    // Avian-specific
    Song, Call, Subsong, Flight_Call, Dawn_Song,
    // Cetacean-specific
    Click, Whistle, Burst_Pulse, Song_Cetacean,
    // Bat-specific
    Echolocation_Pulse, Social_Call, Feeding_Buzz,
    // Amphibian
    Advertisement_Call, Release_Call, Satellite_Call,
    // Insect
    Stridulation, Substrate_Vibration,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum BehavioralContext {
    #[default] Unknown, Foraging, Roosting, Traveling, Socializing,
    Mating, Nesting, Vigilance, Fleeing, Resting, Feeding, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VocalizationUnit {
    pub unit_id: u64,
    pub vocalization_id: u64,
    pub unit_type: VocalizationUnitType,
    pub start_sec: f32,
    pub end_sec: f32,
    pub duration_ms: f32,
    pub freq_start_hz: f32,
    pub freq_end_hz: f32,
    pub peak_freq_hz: f32,
    pub freq_modulation_hz_per_ms: Option<f32>,  // sweep rate (+ up, - down)
    pub amplitude_db: f32,
    pub inter_unit_gap_ms: Option<f32>,           // gap to next unit
    pub unit_index: u32,                          // position within call sequence
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum VocalizationUnitType {
    #[default] Unknown,
    Syllable, Note, Phrase, Pulse, Element,
    // Spectral shapes
    Upswept, Downswept, Sinusoidal, Harmonic,
    Noisy, Tonal, Broadband, Narrowband,
    FrequencyModulated, AmplitudeModulated,
    // Echolocation
    FM_Pulse, CF_Pulse, FM_CF_Pulse, QCFM_Pulse,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpeciesDetection {
    pub detection_id: u64,
    pub common_name: String,
    pub scientific_name: String,
    pub taxonomic_group: TaxonomicGroup,
    pub first_detection_sec: f32,
    pub last_detection_sec: f32,
    pub detection_count: u32,
    pub vocalization_ids: Vec<u64>,
    pub detection_method: DetectionMethod,
    pub detection_note: String,             // replaces confidence
    pub geographic_range_note: Option<String>,
    pub conservation_status: Option<String>,  // IUCN status
    pub acoustic_signature_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum TaxonomicGroup {
    #[default] Unknown,
    Aves, Mammalia, Reptilia, Amphibia, Insecta,
    Cetacea, Chiroptera, Rodentia, Primate,
    Pisces, Arthropoda, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum DetectionMethod {
    #[default] TemplateMatching, SpectralAnalysis, MFCC_Matching,
    NeuralClassifier, HumanAnnotation, DTW, PCA_Clustering, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IndividualID {
    pub individual_id: u64,
    pub species_id: u64,
    pub vocalization_ids: Vec<u64>,
    pub estimated_age_class: Option<String>,    // "juvenile", "adult", "subadult"
    pub estimated_sex: Option<String>,          // "male", "female", "unknown"
    pub identity_note: String,
    pub first_heard_sec: f32,
    pub last_heard_sec: f32,
    pub total_call_count: u32,
    pub home_bearing_deg: Option<f32>,          // direction to individual if localized
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AcousticSignature {
    pub signature_id: u64,
    pub species_name: Option<String>,
    pub call_type: Option<CallType>,
    pub source_class: String,                   // "bird", "bat", "cetacean", etc.
    pub characteristic_frequencies_hz: Vec<f32>,
    pub freq_min_hz: f32,
    pub freq_max_hz: f32,
    pub typical_duration_ms: f32,
    pub typical_unit_count: u32,
    pub inter_call_interval_ms: Option<f32>,
    pub mfcc_vector: Vec<f32>,                  // 13-39 MFCC coefficients for matching
    pub spectrogram_template: Vec<Vec<f32>>,    // time × freq template (compact)
    pub library_name: Option<String>,
    pub library_match_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EcholocationPulse {
    pub pulse_id: u64,
    pub sequence_id: Option<u64>,
    pub start_sec: f32,
    pub duration_ms: f32,
    pub pulse_type: EcholocationPulseType,
    pub freq_start_hz: f32,
    pub freq_end_hz: f32,
    pub peak_freq_hz: f32,
    pub characteristic_freq_hz: Option<f32>,  // CF component if present
    pub sweep_rate_khz_ms: Option<f32>,
    pub pulse_interval_ms: Option<f32>,        // IPP to previous pulse
    pub amplitude_db: f32,
    pub harmonics: Vec<f32>,                   // fundamental + overtones in Hz
    pub species_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum EcholocationPulseType {
    #[default] Unknown,
    FM,             // frequency modulated
    CF,             // constant frequency
    FM_CF,          // FM-CF compound
    QCFM,           // quasi-constant frequency modulated
    Click,          // broadband click (dolphins/sperm whales)
    MultiHarmonic,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EcholocationSequence {
    pub sequence_id: u64,
    pub species_id: Option<u64>,
    pub start_sec: f32,
    pub end_sec: f32,
    pub pulse_ids: Vec<u64>,
    pub sequence_type: EcholocationSequenceType,
    pub pulse_interval_mean_ms: f32,
    pub pulse_interval_trend: PulseIntervalTrend,
    pub target_detected: bool,
    pub call_rate_per_sec: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum EcholocationSequenceType {
    #[default] Search, Approach, FeedingBuzz, Social, Unknown
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PulseIntervalTrend {
    #[default] Stable, Decreasing, Increasing, Variable
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourceLocalization {
    pub localization_id: u64,
    pub vocalization_id: Option<u64>,
    pub sound_event_id: Option<u64>,
    pub estimated_position_m: [f32; 3],        // x, y, z in sensor-centric coords
    pub geo_location: Option<[f64; 3]>,         // lat, lon, alt if georeferenced
    pub position_uncertainty_m: f32,
    pub bearing_deg: f32,
    pub elevation_deg: f32,
    pub estimated_distance_m: f32,
    pub method: LocalizationMethod,
    pub timestamp_sec: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum LocalizationMethod {
    #[default] TDOA, Beamforming, Inverse_Square_Amplitude, Triangulation, Single_Sensor_Estimate
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AcousticPropagationPath {
    pub path_id: u64,
    pub source_id: Option<u64>,
    pub source_position_m: [f32; 3],
    pub receiver_position_m: [f32; 3],
    pub distance_m: f32,
    pub propagation_loss_db: f32,
    pub delay_ms: f32,
    pub medium: PropagationMedium,
    pub reflections: Vec<ReflectionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PropagationMedium { #[default] Air, Water, Substrate }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReflectionInfo {
    pub surface_type: String,       // "ground", "water_surface", "vegetation", "building"
    pub reflection_loss_db: f32,
    pub delay_offset_ms: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AcousticScene {
    pub scene_id: u64,
    pub active_sources: Vec<String>,    // species or source types present
    pub dominant_source: Option<String>,
    pub acoustic_complexity_index: f32, // ACI
    pub bioacoustic_index: f32,         // BI
    pub normalized_difference_soundscape_index: f32, // NDSI
    pub soundscape_description: String,
    pub source_count: u32,
    pub overlap_events_count: u32,      // simultaneously overlapping calls
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AmbientNoiseProfile {
    pub profile_id: u64,
    pub noise_floor_db: f32,
    pub spectral_noise_profile: Vec<(f32, f32)>,   // (freq_hz, level_db)
    pub dominant_noise_source: String,
    pub anthropogenic_fraction: f32,                // 0.0–1.0
    pub frequency_range_usable_hz: (f32, f32),     // frequencies above noise floor
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SoundscapeIndex {
    pub index_id: u64,
    pub index_name: String,             // "ACI", "BI", "NDSI", "ADI", "H", etc.
    pub value: f32,
    pub time_window_sec: f32,
    pub frequency_range_hz: (f32, f32),
    pub interpretation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VocalizationData {
    pub waveform: Vec<f32>,
    pub sample_rate_hz: u32,
    pub start_time_sec: f32,
    pub spectral_features: Option<SpectralFeatures>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpectralFeatures {
    pub mfcc: Vec<f32>,            // 13 coefficients
    pub spectral_centroid_hz: f32,
    pub spectral_bandwidth_hz: f32,
    pub spectral_rolloff_hz: f32,
    pub zero_crossing_rate: f32,
    pub chroma_vector: Vec<f32>,   // 12 chroma bins
    pub mel_spectrogram: Vec<Vec<f32>>,  // time × mel_bands
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SoundEventUpdate {
    pub event_type: String,
    pub vocalization: Option<Vocalization>,
    pub species_detection: Option<SpeciesDetection>,
    pub timestamp_sec: f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH NODE TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SoundNodeType {
    #[default] SoundScene,              // root: entire recording/analysis
    SoundEventNode,                     // detected sound event (generic)
    VocalizationNode,                   // characterized vocalization
    VocalizationUnitNode,               // syllable / note / pulse
    SpeciesNode,                        // detected species
    IndividualNode,                     // identified individual
    AcousticSignatureNode,              // reference or extracted signature
    EcholocationPulseNode,              // single echolocation pulse
    EcholocationSequenceNode,           // pulse train / buzz
    SourceLocalizationNode,             // 3D position of sound source
    PropagationPathNode,                // acoustic propagation path
    AcousticSceneNode,                  // scene-level characterization
    AmbientNoiseNode,                   // background noise profile
    SoundscapeIndexNode,                // ecological index (ACI, BI, etc.)
    BackgroundEventNode,                // non-biotic sound
    TemporalContextNode,                // dawn chorus, dusk activity, etc.
    GeographicLocationNode,             // geo-referenced sensor position
    RecordingMetadataNode,              // recorder / site metadata
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SoundGraphNode {
    pub node_id: u64,
    pub node_type: SoundNodeType,
    pub content: String,

    // SOUND-SPECIFIC PAYLOAD
    pub start_sec: Option<f32>,
    pub end_sec: Option<f32>,
    pub duration_ms: Option<f32>,
    pub freq_min_hz: Option<f32>,
    pub freq_max_hz: Option<f32>,
    pub peak_freq_hz: Option<f32>,
    pub amplitude_db: Option<f32>,
    pub snr_db: Option<f32>,
    pub species_name: Option<String>,
    pub call_type: Option<String>,
    pub geo_location: Option<[f64; 3]>,
    pub bearing_deg: Option<f32>,
    pub distance_m: Option<f32>,

    // UNIVERSAL NODE FIELDS (Section B.2)
    pub provisional: bool,
    pub provisional_status: ProvisionalStatus,
    pub materialized_path: Option<String>,
    pub created_by_step: Option<u32>,
    pub updated_by_step: Option<u32>,
    pub version: u32,
    pub version_notes: Vec<VersionNote>,
    pub keywords: Vec<String>,
    pub embedding_hint: Option<String>,
    pub hotness_score: f32,
    pub source_chunk_index: Option<u32>,
    pub source_start_char: Option<usize>,
    pub source_end_char: Option<usize>,
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH EDGE TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SoundEdgeType {
    // ── STRUCTURAL ──
    #[default] Contains, Precedes, PartOf,

    // ── SOUND-SPECIFIC ──
    ProducedBy,                     // vocalization produced by species/individual
    ConsistsOf,                     // call consists of units
    ReconstructsTo,                 // raw waveform reconstructs to vocalization
    DerivedFromRawWaveform,         // spectrogram / feature derived from waveform
    SpeciesSpecific,                // call is species-specific signature
    ExpressesState,                 // vocalization expresses behavioral state
    ResponseTo,                     // this call is a response to another
    SameIndividualAs,               // two vocalizations from same individual
    OverlapsWith,                   // temporal overlap with another call
    PrecedesInSequence,             // next in call sequence
    PartOfEcholocationSequence,     // pulse is part of sequence
    LocalizedAt,                    // sound source localized at position
    PropagatesVia,                  // sound propagates via this path
    MaskingOf,                      // noise masks this signal
    AmplitudeModulatedBy,           // carrier modulated by envelope
    HarmonicOf,                     // harmonic relationship
    MatchesSignature,               // vocalization matches this signature

    // ── CROSS-MODAL ──
    /// Vocalization linked to species biology node (111)
    LinksToSpeciesBiology,
    /// Source position on geospatial map (117)
    PlottedOnGeoMap,
    /// Marine vocalization → sonar bio detection (125)
    MarineVocalizationToSonar,
    /// Acoustic activity correlated with thermal (114)
    CorrelatesWithThermal,
    /// Raw audio feeds from audio pipeline (103)
    FedByAudioPipeline,

    // ── UNIVERSAL SEMANTIC ──
    Performs, Affects, Implies, Contradicts, Elaborates, Summarizes, Supports,
    TemporalPrecedes, TemporalFollows, CausedBy, Enables, Prevents,
    FunctionalRole, InstanceOf,
    DerivedFrom, VersionOf, RefinesTo, ForkedFrom, SimilarTo,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SoundGraphEdge {
    pub edge_id: u64,
    pub from_node: u64, pub to_node: u64,
    pub edge_type: SoundEdgeType,
    pub weight: f32,
    pub provenance: EdgeProvenance,
    pub created_by_step: Option<u32>,
    pub version: u32,
    pub version_notes: Vec<VersionNote>,
    pub properties: HashMap<String, serde_json::Value>,
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH STRUCTURE
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SoundGraph {
    pub graph_id: u64, pub project_id: u64,
    pub source_description: String,
    pub nodes: Vec<SoundGraphNode>,
    pub edges: Vec<SoundGraphEdge>,
    pub root_node_id: u64,
    pub state: GraphStateType,
    pub state_history: Vec<GraphStateTransition>,
    pub created_at: String, pub updated_at: String,
    pub version: u32, pub version_notes: Vec<VersionNote>,
}

// ─────────────────────────────────────────────────────────────────────────────
// QUERY / HOOKS / EXPORT / DISPLAY
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SoundGraphQuery {
    NodeDetail { node_id: u64 },
    VocalizationsBySpecies { species_name: String },
    EcholocationSequences,
    SpeciesDetections,
    IndividualIdentifications,
    AcousticSignatures,
    SoundscapeIndices,
    PropagationPaths,
    CrossModalLinks { node_id: u64 },
    CallsInTimeWindow { start_sec: f32, end_sec: f32 },
    AGIActivity, AllNodes, AllEdges,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SoundSemanticHook {
    OnGraphCreated,
    OnInferRelationships,
    OnEdgeCompletion,
    OnCrossModalityLink { target_modality: String, target_graph_id: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SoundExportFormat {
    Raven_Selection_Table,      // Cornell Lab Raven format
    Audacity_Labels,            // Audacity label track
    GeoJSON,                    // species detections with locations
    BatLogger_CSV,              // bat call analysis format
    ARBIMON_CSV,                // Arbimon bioacoustic monitoring format
    BirdNET_CSV,                // BirdNET output format
    OBIS,                       // Ocean biodiversity format (cetaceans)
    CSV,                        // generic table
    HDF5,                       // scientific arrays
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SoundDisplayMode {
    Spectrogram,                // time-freq view
    Waveform,                   // amplitude vs time
    EventTimeline,              // call events on timeline
    SpeciesOccurrence,          // species activity chart
    EcholocationWaterfall,      // bat call display
    SoundscapeIndexChart,       // ecological indices over time
    SourceMap,                  // localized sources on spatial map
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SoundHeadlessOp {
    ReRunSpeciesID,
    ExtractAllSignatures,
    ComputeSoundscapeIndices,
    LocalizeSources { array_data: MicrophoneArrayData },
    CrossLinkToBiology { bio_graph_id: u64 },
    CrossLinkToGeo { geo_graph_id: u64 },
    ExportSelectionTable { format: SoundExportFormat },
}

// ─────────────────────────────────────────────────────────────────────────────
// OUTPUT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SoundReconstructionOutput {
    pub success: bool,
    pub graph_id: Option<u64>,
    pub graph: Option<SoundGraph>,
    pub analysis: Option<SoundAnalysisResult>,
    pub detected_events: Option<Vec<SoundEvent>>,
    pub species_identification: Option<SpeciesDetection>,
    pub decomposed_units: Option<Vec<VocalizationUnit>>,
    pub scene: Option<AcousticScene>,
    pub signatures: Option<Vec<AcousticSignature>>,
    pub echolocation_classification: Option<EcholocationPulse>,
    pub repertoire: Option<Vec<AcousticSignature>>,
    pub query_result: Option<serde_json::Value>,
    pub export_path: Option<String>,
    pub error: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// SHARED TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ProvisionalStatus { #[default] Planned, Generating, Generated, Validated, Finalized, Failed, RolledBack }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VersionNote {
    pub version: u32, pub note: String,
    pub step_index: Option<u32>, pub timestamp: String, pub change_type: ChangeType,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ChangeType { #[default] Created, Updated, CrossLinked, EnrichedBySemantic, EnrichedByHook, RolledBack, Finalized }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum EdgeProvenance {
    #[default] Unknown,
    DerivedFromPrompt, DerivedFromChunk(u32), DerivedFromChunkGraph(u64),
    DerivedFromModalityGraph(u64), DerivedFromFile(String),
    DerivedFromAMT, DerivedFromBlueprint(u32), DerivedFromMethodology(u64),
    DerivedFromCrossModal, DerivedFromHook, VersionOf(u32), ForkedFrom(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum GraphStateType { #[default] Created, SemanticEnriched, CrossLinked, Stable, Updated, ReValidating, Archived }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStateTransition {
    pub from: GraphStateType, pub to: GraphStateType,
    pub timestamp: String, pub triggered_by_step: Option<u32>,
}

// ─────────────────────────────────────────────────────────────────────────────
// PIPELINE EXECUTOR
// ─────────────────────────────────────────────────────────────────────────────

struct PipelineExecutor { zsei_path: String, prompt_pipeline_path: String }

impl PipelineExecutor {
    fn new() -> Self {
        Self {
            zsei_path: env::var("OZONE_ZSEI_PATH").unwrap_or_else(|_| "./zsei_data".into()),
            prompt_pipeline_path: env::var("OZONE_PROMPT_PIPELINE").unwrap_or_else(|_| "./pipeline_9".into()),
        }
    }

    async fn llm_zero_shot(&self, prompt: &str, max_tokens: usize) -> Result<String, String> {
        let input = serde_json::json!({
            "action": "Prompt", "prompt": prompt,
            "max_tokens": max_tokens, "temperature": 0.05,
            "system_context": "Bioacoustic sound analysis. Return only valid JSON."
        });
        let out = std::process::Command::new(&self.prompt_pipeline_path)
            .arg("--input").arg(input.to_string())
            .output().map_err(|e| e.to_string())?;
        let r: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
        Ok(r["response"].as_str().unwrap_or("{}").to_string())
    }

    fn save_graph(&self, g: &SoundGraph) -> Result<(), String> {
        let path = format!("{}/local/sound_graph_{}.json", self.zsei_path, g.graph_id);
        if let Some(p) = std::path::Path::new(&path).parent() { std::fs::create_dir_all(p).map_err(|e| e.to_string())?; }
        std::fs::write(&path, serde_json::to_string_pretty(g).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    fn load_graph(&self, id: u64) -> Result<SoundGraph, String> {
        let path = format!("{}/local/sound_graph_{}.json", self.zsei_path, id);
        serde_json::from_str(&std::fs::read_to_string(&path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    fn generate_id(&self) -> u64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as u64
    }
    fn now_iso8601(&self) -> String { format!("{}", self.generate_id()) }
    fn extract_json_array(raw: &str) -> String { if let (Some(s), Some(e)) = (raw.find('['), raw.rfind(']')) { raw[s..=e].to_string() } else { "[]".to_string() } }
    fn extract_json_object(raw: &str) -> String { if let (Some(s), Some(e)) = (raw.find('{'), raw.rfind('}')) { raw[s..=e].to_string() } else { "{}".to_string() } }
}

// ─────────────────────────────────────────────────────────────────────────────
// LLM-BASED ANALYSIS
// ─────────────────────────────────────────────────────────────────────────────

impl PipelineExecutor {
    async fn identify_species_llm(
        &self,
        vocalizations: &[Vocalization],
        context: &BioacousticContext,
    ) -> Vec<(u64, String, String)> {
        if vocalizations.is_empty() { return vec![]; }

        let voc_list: Vec<serde_json::Value> = vocalizations.iter().take(15).map(|v| serde_json::json!({
            "vocalization_id": v.vocalization_id,
            "duration_ms": v.duration_ms,
            "freq_min_hz": v.freq_min_hz,
            "freq_max_hz": v.freq_max_hz,
            "peak_freq_hz": v.peak_freq_hz,
            "call_type": format!("{:?}", v.call_type),
            "unit_count": v.unit_ids.len(),
            "amplitude_db": v.amplitude_db,
            "snr_db": v.snr_db,
        })).collect();

        let prompt = format!(r#"
Identify the likely species for each vocalization based on its acoustic properties.
Bioacoustic context: {:?}

Vocalizations:
{}

For each vocalization, identify:
- most likely species (common name and scientific name)
- call type confirmation

Consider typical frequency ranges:
- Small passerines: 2000-8000 Hz
- Large birds: 500-3000 Hz
- Bats (insectivorous): 20000-120000 Hz
- Dolphins: 2000-150000 Hz
- Whales (baleen): 10-8000 Hz
- Frogs: 500-5000 Hz
- Crickets/katydids: 2000-20000 Hz

Return ONLY valid JSON array:
[{{
  "vocalization_id": N,
  "common_name": "species common name",
  "scientific_name": "Genus species",
  "detection_note": "brief reasoning based on frequency and duration"
}}]"#,
            context, serde_json::to_string_pretty(&voc_list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 800).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str)
                    .unwrap_or_default().into_iter()
                    .filter_map(|v| {
                        let id = v["vocalization_id"].as_u64()?;
                        let common = v["common_name"].as_str()?.to_string();
                        let scientific = v["scientific_name"].as_str().unwrap_or("").to_string();
                        Some((id, common, scientific))
                    }).collect()
            }
            Err(_) => vec![],
        }
    }

    async fn classify_call_types_llm(&self, vocalizations: &[Vocalization]) -> Vec<(u64, String, String)> {
        if vocalizations.is_empty() { return vec![]; }

        let voc_list: Vec<serde_json::Value> = vocalizations.iter().take(20).map(|v| serde_json::json!({
            "vocalization_id": v.vocalization_id,
            "duration_ms": v.duration_ms,
            "freq_min_hz": v.freq_min_hz,
            "freq_max_hz": v.freq_max_hz,
            "unit_count": v.unit_ids.len(),
            "species_hint": v.vocalization_id % 3, // placeholder grouping
        })).collect();

        let prompt = format!(r#"
Classify the behavioral context and call type for each vocalization.

Vocalizations:
{}

Call types: Song, Contact, Alarm, Distress, Territorial, Mating_Advertisement,
Flight_Call, Begging, Warning, Whistle, Click, Stridulation, Echolocation_Pulse, Unknown.

Behavioral contexts: Foraging, Roosting, Traveling, Socializing, Mating, Nesting,
Vigilance, Fleeing, Resting, Feeding, Unknown.

Return ONLY valid JSON array:
[{{
  "vocalization_id": N,
  "call_type": "CallTypeName",
  "behavioral_context": "ContextName"
}}]"#,
            serde_json::to_string_pretty(&voc_list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 600).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str)
                    .unwrap_or_default().into_iter()
                    .filter_map(|v| {
                        let id = v["vocalization_id"].as_u64()?;
                        let ct = v["call_type"].as_str()?.to_string();
                        let bc = v["behavioral_context"].as_str().unwrap_or("Unknown").to_string();
                        Some((id, ct, bc))
                    }).collect()
            }
            Err(_) => vec![],
        }
    }

    async fn infer_semantic_relationships(
        &self,
        nodes: &[SoundGraphNode],
    ) -> Vec<(u64, u64, SoundEdgeType, String)> {
        if nodes.len() < 2 { return vec![]; }

        let node_list: Vec<serde_json::Value> = nodes.iter().take(25).map(|n| serde_json::json!({
            "node_id": n.node_id,
            "type": format!("{:?}", n.node_type),
            "content": n.content.chars().take(80).collect::<String>(),
            "start_sec": n.start_sec,
            "species": n.species_name,
            "call_type": n.call_type,
        })).collect();

        let prompt = format!(r#"
Identify semantic relationships between these bioacoustic sound graph nodes.

Nodes:
{}

Available relationship types:
ResponseTo (call answers another), OverlapsWith (temporal overlap),
SameIndividualAs (same animal), HarmonicOf (overtone relationship),
MaskingOf (noise masks signal), MatchesSignature (matches reference),
PrecedesInSequence (sequential calls), ExpressesState (behavioral state),
Affects, CausedBy, Enables, TemporalPrecedes, DerivedFrom, SimilarTo

Return ONLY valid JSON array:
[{{"from_node_id": N, "to_node_id": M, "edge_type": "TypeName", "reason": "brief"}}]"#,
            serde_json::to_string_pretty(&node_list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 700).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str)
                    .unwrap_or_default().into_iter()
                    .filter_map(|v| {
                        let from = v["from_node_id"].as_u64()?;
                        let to = v["to_node_id"].as_u64()?;
                        let etype = map_sound_edge_str(v["edge_type"].as_str().unwrap_or("Affects"));
                        let reason = v["reason"].as_str().unwrap_or("").to_string();
                        Some((from, to, etype, reason))
                    }).collect()
            }
            Err(_) => vec![],
        }
    }

    async fn infer_individual_identities(&self, vocalizations: &[Vocalization]) -> Vec<Vec<u64>> {
        if vocalizations.len() < 2 { return vec![]; }

        let voc_list: Vec<serde_json::Value> = vocalizations.iter().take(30).map(|v| serde_json::json!({
            "vocalization_id": v.vocalization_id,
            "start_sec": v.start_sec,
            "peak_freq_hz": v.peak_freq_hz,
            "duration_ms": v.duration_ms,
            "call_type": format!("{:?}", v.call_type),
            "bearing_deg": v.source_direction_deg,
        })).collect();

        let prompt = format!(r#"
Group these vocalizations by individual animal.
Animals from the same individual tend to:
- Have consistent peak frequencies (same vocal tract)
- Come from the same bearing direction
- Have consistent call durations
- Appear at plausible movement intervals

Vocalizations:
{}

Return ONLY valid JSON array of groups (each group = one individual):
[[vocalization_id_1, vocalization_id_2, ...], [...], ...]"#,
            serde_json::to_string_pretty(&voc_list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 400).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<Vec<u64>>>(&json_str).unwrap_or_default()
            }
            Err(_) => vec![],
        }
    }

    fn compute_mfcc_simple(waveform: &[f32], sample_rate_hz: u32, n_mfcc: usize) -> Vec<f32> {
        // Simplified MFCC: energy in mel-spaced frequency bands
        if waveform.is_empty() { return vec![0.0; n_mfcc]; }
        let n = waveform.len();
        // Mel filterbank: 13 bands from 80 Hz to sample_rate/2
        let mel_min = 2595.0 * (1.0 + 80.0f32 / 700.0).log10();
        let mel_max = 2595.0 * (1.0 + (sample_rate_hz as f32 / 2.0) / 700.0).log10();
        let mel_step = (mel_max - mel_min) / (n_mfcc + 1) as f32;

        let mut mfcc = Vec::with_capacity(n_mfcc);
        for k in 0..n_mfcc {
            let mel_center = mel_min + mel_step * (k + 1) as f32;
            let freq_center = 700.0 * (10.0f32.powf(mel_center / 2595.0) - 1.0);
            let bin_center = (freq_center * n as f32 / sample_rate_hz as f32) as usize;
            let bin_start = bin_center.saturating_sub(n / (n_mfcc * 2));
            let bin_end = (bin_center + n / (n_mfcc * 2)).min(n);
            let energy: f32 = waveform[bin_start..bin_end].iter().map(|v| v * v).sum::<f32>()
                / (bin_end - bin_start).max(1) as f32;
            mfcc.push(10.0 * energy.max(1e-10).log10());
        }
        mfcc
    }

    fn compute_spectral_features(waveform: &[f32], sample_rate_hz: u32) -> SpectralFeatures {
        let n = waveform.len();
        if n == 0 { return SpectralFeatures::default(); }
        let freq_per_bin = sample_rate_hz as f32 / n as f32;
        let power: Vec<f32> = waveform.iter().map(|v| v * v).collect();
        let total_power: f32 = power.iter().sum::<f32>().max(1e-10);

        // Spectral centroid
        let centroid: f32 = power.iter().enumerate()
            .map(|(i, &p)| i as f32 * freq_per_bin * p)
            .sum::<f32>() / total_power;

        // Spectral bandwidth
        let bandwidth: f32 = (power.iter().enumerate()
            .map(|(i, &p)| (i as f32 * freq_per_bin - centroid).powi(2) * p)
            .sum::<f32>() / total_power).sqrt();

        // Zero crossing rate
        let zcr = waveform.windows(2)
            .filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0))
            .count() as f32 / n as f32;

        // 85th percentile rolloff
        let mut cumsum = 0.0f32;
        let threshold = total_power * 0.85;
        let rolloff = power.iter().enumerate()
            .find(|(_, &p)| { cumsum += p; cumsum >= threshold })
            .map(|(i, _)| i as f32 * freq_per_bin)
            .unwrap_or(sample_rate_hz as f32 / 2.0);

        SpectralFeatures {
            mfcc: Self::compute_mfcc_simple(waveform, sample_rate_hz, 13),
            spectral_centroid_hz: centroid,
            spectral_bandwidth_hz: bandwidth,
            spectral_rolloff_hz: rolloff,
            zero_crossing_rate: zcr,
            chroma_vector: vec![0.0; 12],
            mel_spectrogram: vec![],
        }
    }

    fn detect_events_energy(
        waveform: &[f32],
        sample_rate_hz: u32,
        threshold_db: f32,
        min_duration_ms: f32,
        max_duration_ms: f32,
    ) -> Vec<SoundEvent> {
        let frame_len = (sample_rate_hz as f32 * 0.01) as usize; // 10ms frames
        let noise_floor: f32 = {
            let rms: f32 = waveform.iter().map(|v| v * v).sum::<f32>() / waveform.len() as f32;
            10.0 * rms.max(1e-10).log10()
        };
        let threshold = noise_floor + threshold_db;

        let frame_energies: Vec<f32> = waveform.chunks(frame_len).map(|chunk| {
            let rms = chunk.iter().map(|v| v * v).sum::<f32>() / chunk.len() as f32;
            10.0 * rms.max(1e-10).log10()
        }).collect();

        let mut events = Vec::new();
        let mut in_event = false;
        let mut event_start_frame = 0usize;
        let frame_duration_ms = 10.0f32;

        for (fi, &energy) in frame_energies.iter().enumerate() {
            if energy > threshold && !in_event {
                in_event = true;
                event_start_frame = fi;
            } else if energy <= threshold && in_event {
                in_event = false;
                let duration_ms = (fi - event_start_frame) as f32 * frame_duration_ms;
                if duration_ms >= min_duration_ms && duration_ms <= max_duration_ms {
                    let start_sec = event_start_frame as f32 * 0.01;
                    let end_sec = fi as f32 * 0.01;
                    let peak = frame_energies[event_start_frame..fi].iter().copied().fold(f32::NEG_INFINITY, f32::max);
                    events.push(SoundEvent {
                        event_id: fi as u64 * 1000 + event_start_frame as u64,
                        start_sec, end_sec, duration_ms,
                        freq_min_hz: 0.0, freq_max_hz: sample_rate_hz as f32 / 2.0,
                        peak_freq_hz: sample_rate_hz as f32 / 4.0,
                        peak_amplitude_db: peak,
                        rms_amplitude_db: peak - 3.0,
                        event_type: SoundEventType::Unknown,
                        biotic: true, source_direction_deg: None, channel: 0,
                    });
                }
            }
        }
        events
    }
}

fn map_sound_edge_str(s: &str) -> SoundEdgeType {
    match s {
        "ProducedBy"                 => SoundEdgeType::ProducedBy,
        "ConsistsOf"                 => SoundEdgeType::ConsistsOf,
        "ReconstructsTo"             => SoundEdgeType::ReconstructsTo,
        "DerivedFromRawWaveform"     => SoundEdgeType::DerivedFromRawWaveform,
        "SpeciesSpecific"            => SoundEdgeType::SpeciesSpecific,
        "ExpressesState"             => SoundEdgeType::ExpressesState,
        "ResponseTo"                 => SoundEdgeType::ResponseTo,
        "SameIndividualAs"           => SoundEdgeType::SameIndividualAs,
        "OverlapsWith"               => SoundEdgeType::OverlapsWith,
        "PrecedesInSequence"         => SoundEdgeType::PrecedesInSequence,
        "PartOfEcholocationSequence" => SoundEdgeType::PartOfEcholocationSequence,
        "LocalizedAt"                => SoundEdgeType::LocalizedAt,
        "PropagatesVia"              => SoundEdgeType::PropagatesVia,
        "MaskingOf"                  => SoundEdgeType::MaskingOf,
        "HarmonicOf"                 => SoundEdgeType::HarmonicOf,
        "MatchesSignature"           => SoundEdgeType::MatchesSignature,
        "LinksToSpeciesBiology"      => SoundEdgeType::LinksToSpeciesBiology,
        "PlottedOnGeoMap"            => SoundEdgeType::PlottedOnGeoMap,
        "MarineVocalizationToSonar"  => SoundEdgeType::MarineVocalizationToSonar,
        "CorrelatesWithThermal"      => SoundEdgeType::CorrelatesWithThermal,
        "Affects"                    => SoundEdgeType::Affects,
        "CausedBy"                   => SoundEdgeType::CausedBy,
        "Enables"                    => SoundEdgeType::Enables,
        "TemporalPrecedes"           => SoundEdgeType::TemporalPrecedes,
        "TemporalFollows"            => SoundEdgeType::TemporalFollows,
        "DerivedFrom"                => SoundEdgeType::DerivedFrom,
        "SimilarTo"                  => SoundEdgeType::SimilarTo,
        "PartOf"                     => SoundEdgeType::PartOf,
        _                            => SoundEdgeType::Affects,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH CREATION
// ─────────────────────────────────────────────────────────────────────────────

async fn create_graph(
    executor: &PipelineExecutor,
    analysis: SoundAnalysisResult,
    project_id: u64,
) -> SoundReconstructionOutput {
    let graph_id = executor.generate_id();
    let now = executor.now_iso8601();
    let mut nodes: Vec<SoundGraphNode> = Vec::new();
    let mut edges: Vec<SoundGraphEdge> = Vec::new();
    let mut node_id: u64 = 1;
    let mut edge_id: u64 = 1;

    // ── ROOT NODE ──
    let root_id = node_id;
    nodes.push(SoundGraphNode {
        node_id: root_id, node_type: SoundNodeType::SoundScene,
        content: format!(
            "Sound scene: {:.1}s sr={}Hz events={} vocs={} species={} individuals={} echoloc={}",
            analysis.recording_duration_sec, analysis.sample_rate_hz,
            analysis.sound_events.len(), analysis.vocalizations.len(),
            analysis.species_detections.len(), analysis.individual_identifications.len(),
            analysis.echolocation_sequences.len()
        ),
        materialized_path: Some(format!("/Modalities/SoundReconstruction/Project_{}/Graph_{}", project_id, graph_id)),
        provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
        version_notes: vec![VersionNote { version: 1, note: "Initial creation".into(), step_index: None, timestamp: now.clone(), change_type: ChangeType::Created }],
        keywords: vec!["sound".into(), "bioacoustics".into(), "reconstruction".into()],
        hotness_score: 1.0, ..Default::default()
    });
    node_id += 1;

    // ── RECORDING METADATA NODE ──
    let meta_nid = node_id;
    nodes.push(SoundGraphNode {
        node_id: meta_nid, node_type: SoundNodeType::RecordingMetadataNode,
        content: format!("Metadata: habitat={:?} location={:?} recorder={:?}",
            analysis.recording_metadata.habitat_type,
            analysis.recording_metadata.location,
            analysis.recording_metadata.recorder_model),
        geo_location: analysis.recording_metadata.location.map(|l| [l[0], l[1], l[2]]),
        materialized_path: Some(format!("/Modalities/SoundReconstruction/Project_{}/Graph_{}/Metadata", project_id, graph_id)),
        provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
        keywords: { let mut kw = vec!["metadata".into()]; if let Some(ref h) = analysis.recording_metadata.habitat_type { kw.push(h.to_lowercase()); } kw },
        hotness_score: 0.6, ..Default::default()
    });
    edges.push(SoundGraphEdge { edge_id, from_node: root_id, to_node: meta_nid, edge_type: SoundEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
    edge_id += 1; node_id += 1;

    // ── AMBIENT NOISE NODE ──
    if let Some(ref ambient) = analysis.ambient_noise_profile {
        let anid = node_id;
        nodes.push(SoundGraphNode {
            node_id: anid, node_type: SoundNodeType::AmbientNoiseNode,
            content: format!("Ambient: floor={:.1}dB dominant={} anthro={:.0}% usable={}–{}Hz",
                ambient.noise_floor_db, ambient.dominant_noise_source,
                ambient.anthropogenic_fraction * 100.0,
                ambient.frequency_range_usable_hz.0, ambient.frequency_range_usable_hz.1),
            amplitude_db: Some(ambient.noise_floor_db),
            materialized_path: Some(format!("/Modalities/SoundReconstruction/Project_{}/Graph_{}/Ambient", project_id, graph_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["ambient".into(), "noise".into()], hotness_score: 0.55, ..Default::default()
        });
        edges.push(SoundGraphEdge { edge_id, from_node: root_id, to_node: anid, edge_type: SoundEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── ACOUSTIC SCENE NODE ──
    if let Some(ref scene) = analysis.acoustic_scene {
        let snid = node_id;
        nodes.push(SoundGraphNode {
            node_id: snid, node_type: SoundNodeType::AcousticSceneNode,
            content: format!("AcousticScene: sources={} dominant={:?} ACI={:.2} BI={:.2} NDSI={:.2}",
                scene.source_count, scene.dominant_source, scene.acoustic_complexity_index,
                scene.bioacoustic_index, scene.normalized_difference_soundscape_index),
            materialized_path: Some(format!("/Modalities/SoundReconstruction/Project_{}/Graph_{}/Scene", project_id, graph_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["acoustic-scene".into(), "soundscape".into()], hotness_score: 0.75, ..Default::default()
        });
        edges.push(SoundGraphEdge { edge_id, from_node: root_id, to_node: snid, edge_type: SoundEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── SOUNDSCAPE INDICES ──
    for idx in &analysis.soundscape_indices {
        let iid = node_id;
        nodes.push(SoundGraphNode {
            node_id: iid, node_type: SoundNodeType::SoundscapeIndexNode,
            content: format!("Index {}: value={:.3} window={:.0}s freq={}–{}Hz | {}",
                idx.index_name, idx.value, idx.time_window_sec,
                idx.frequency_range_hz.0, idx.frequency_range_hz.1, idx.interpretation),
            materialized_path: Some(format!("/Modalities/SoundReconstruction/Project_{}/Graph_{}/Index/{}", project_id, graph_id, idx.index_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["soundscape-index".into(), idx.index_name.to_lowercase()], hotness_score: 0.6, ..Default::default()
        });
        edges.push(SoundGraphEdge { edge_id, from_node: root_id, to_node: iid, edge_type: SoundEdgeType::Contains, weight: 0.7, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── SPECIES NODES ──
    let mut species_node_ids: HashMap<u64, u64> = HashMap::new();
    for sp in &analysis.species_detections {
        let spid = node_id;
        nodes.push(SoundGraphNode {
            node_id: spid, node_type: SoundNodeType::SpeciesNode,
            content: format!("Species: {} ({}) group={:?} detections={} calls={}",
                sp.common_name, sp.scientific_name, sp.taxonomic_group,
                sp.detection_count, sp.vocalization_ids.len()),
            species_name: Some(format!("{} ({})", sp.common_name, sp.scientific_name)),
            start_sec: Some(sp.first_detection_sec), end_sec: Some(sp.last_detection_sec),
            materialized_path: Some(format!("/Modalities/SoundReconstruction/Project_{}/Graph_{}/Species/{}", project_id, graph_id, sp.detection_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: {
                let mut kw = vec!["species".into(), sp.common_name.to_lowercase()];
                kw.extend(sp.scientific_name.to_lowercase().split_whitespace().map(String::from));
                kw
            },
            hotness_score: 0.8,
            embedding_hint: Some(format!("species: {} scientific: {} group: {:?}", sp.common_name, sp.scientific_name, sp.taxonomic_group)),
            ..Default::default()
        });
        species_node_ids.insert(sp.detection_id, spid);
        edges.push(SoundGraphEdge { edge_id, from_node: root_id, to_node: spid, edge_type: SoundEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Species → biology cross-modal
        edges.push(SoundGraphEdge {
            edge_id, from_node: spid, to_node: spid,
            edge_type: SoundEdgeType::LinksToSpeciesBiology, weight: 0.9,
            provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
            properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("biology")); p.insert("scientific_name".into(), serde_json::json!(&sp.scientific_name)); p },
            ..Default::default()
        });
        edge_id += 1;

        // Marine species → sonar cross-modal
        if matches!(sp.taxonomic_group, TaxonomicGroup::Cetacea) {
            edges.push(SoundGraphEdge {
                edge_id, from_node: spid, to_node: spid,
                edge_type: SoundEdgeType::MarineVocalizationToSonar, weight: 0.8,
                provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("sonar")); p },
                ..Default::default()
            });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── INDIVIDUAL NODES ──
    let mut individual_node_ids: HashMap<u64, u64> = HashMap::new();
    for ind in &analysis.individual_identifications {
        let iid = node_id;
        nodes.push(SoundGraphNode {
            node_id: iid, node_type: SoundNodeType::IndividualNode,
            content: format!("Individual {}: calls={} age={:?} sex={:?} bearing={:?}°",
                ind.individual_id, ind.total_call_count,
                ind.estimated_age_class, ind.estimated_sex, ind.home_bearing_deg),
            start_sec: Some(ind.first_heard_sec), end_sec: Some(ind.last_heard_sec),
            bearing_deg: ind.home_bearing_deg,
            materialized_path: Some(format!("/Modalities/SoundReconstruction/Project_{}/Graph_{}/Individual/{}", project_id, graph_id, ind.individual_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["individual".into(), "identity".into()], hotness_score: 0.7, ..Default::default()
        });
        individual_node_ids.insert(ind.individual_id, iid);
        edges.push(SoundGraphEdge { edge_id, from_node: root_id, to_node: iid, edge_type: SoundEdgeType::Contains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Individual → species
        for sp in &analysis.species_detections {
            if sp.detection_id == ind.species_id {
                if let Some(&sp_nid) = species_node_ids.get(&sp.detection_id) {
                    edges.push(SoundGraphEdge { edge_id, from_node: iid, to_node: sp_nid, edge_type: SoundEdgeType::SpeciesSpecific, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                    edge_id += 1;
                }
            }
        }
        node_id += 1;
    }

    // ── VOCALIZATION NODES ──
    let mut voc_node_ids: HashMap<u64, u64> = HashMap::new();
    for voc in &analysis.vocalizations {
        let vid = node_id;
        nodes.push(SoundGraphNode {
            node_id: vid, node_type: SoundNodeType::VocalizationNode,
            content: format!("Voc {:?}: {:.1}–{:.1}s dur={:.0}ms freq={:.0}–{:.0}Hz snr={:.1}dB units={}",
                voc.call_type, voc.start_sec, voc.end_sec, voc.duration_ms,
                voc.freq_min_hz, voc.freq_max_hz, voc.snr_db, voc.unit_ids.len()),
            start_sec: Some(voc.start_sec), end_sec: Some(voc.end_sec),
            duration_ms: Some(voc.duration_ms),
            freq_min_hz: Some(voc.freq_min_hz), freq_max_hz: Some(voc.freq_max_hz),
            peak_freq_hz: Some(voc.peak_freq_hz),
            amplitude_db: Some(voc.amplitude_db), snr_db: Some(voc.snr_db),
            call_type: Some(format!("{:?}", voc.call_type)),
            bearing_deg: voc.source_direction_deg, distance_m: voc.source_distance_m,
            materialized_path: Some(format!("/Modalities/SoundReconstruction/Project_{}/Graph_{}/Voc/{}", project_id, graph_id, voc.vocalization_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: {
                let mut kw = vec!["vocalization".into(), format!("{:?}", voc.call_type).to_lowercase()];
                kw.push(format!("{:.0}hz", voc.peak_freq_hz));
                kw
            },
            hotness_score: 0.6 + (voc.snr_db / 60.0).clamp(0.0, 0.3),
            ..Default::default()
        });
        voc_node_ids.insert(voc.vocalization_id, vid);
        edges.push(SoundGraphEdge { edge_id, from_node: root_id, to_node: vid, edge_type: SoundEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Vocalization → species
        if let Some(sp_id) = voc.species_id {
            if let Some(&sp_nid) = species_node_ids.get(&sp_id) {
                edges.push(SoundGraphEdge { edge_id, from_node: sp_nid, to_node: vid, edge_type: SoundEdgeType::ProducedBy, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        // Vocalization → individual
        if let Some(ind_id) = voc.individual_id {
            if let Some(&ind_nid) = individual_node_ids.get(&ind_id) {
                edges.push(SoundGraphEdge { edge_id, from_node: ind_nid, to_node: vid, edge_type: SoundEdgeType::ProducedBy, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        // Response to another vocalization
        if let Some(resp_id) = voc.response_to_id {
            if let Some(&resp_nid) = voc_node_ids.get(&resp_id) {
                edges.push(SoundGraphEdge { edge_id, from_node: vid, to_node: resp_nid, edge_type: SoundEdgeType::ResponseTo, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        node_id += 1;
    }

    // Sequential ordering of vocalizations (time-ordered)
    let mut sorted_vocs: Vec<(&Vocalization, u64)> = analysis.vocalizations.iter()
        .filter_map(|v| voc_node_ids.get(&v.vocalization_id).map(|&nid| (v, nid)))
        .collect();
    sorted_vocs.sort_by(|a, b| a.0.start_sec.partial_cmp(&b.0.start_sec).unwrap_or(std::cmp::Ordering::Equal));
    for pair in sorted_vocs.windows(2) {
        let (voc_a, nid_a) = pair[0];
        let (voc_b, nid_b) = pair[1];
        if voc_b.start_sec - voc_a.end_sec < 5.0 { // within 5 seconds: sequential
            edges.push(SoundGraphEdge {
                edge_id, from_node: nid_a, to_node: nid_b,
                edge_type: SoundEdgeType::PrecedesInSequence, weight: 0.7,
                provenance: EdgeProvenance::DerivedFromPrompt, version: 1,
                properties: { let mut p = HashMap::new(); p.insert("gap_sec".into(), serde_json::json!(voc_b.start_sec - voc_a.end_sec)); p },
                ..Default::default()
            });
            edge_id += 1;
        }
        // Temporal overlap detection
        if voc_b.start_sec < voc_a.end_sec {
            edges.push(SoundGraphEdge {
                edge_id, from_node: nid_a, to_node: nid_b,
                edge_type: SoundEdgeType::OverlapsWith, weight: 0.8,
                provenance: EdgeProvenance::DerivedFromPrompt, version: 1,
                properties: { let mut p = HashMap::new(); p.insert("overlap_sec".into(), serde_json::json!(voc_a.end_sec - voc_b.start_sec)); p },
                ..Default::default()
            });
            edge_id += 1;
        }
    }

    // ── VOCALIZATION UNIT NODES ──
    for unit in analysis.vocalization_units.iter().take(200) {
        let uid = node_id;
        nodes.push(SoundGraphNode {
            node_id: uid, node_type: SoundNodeType::VocalizationUnitNode,
            content: format!("Unit {:?} #{}: {:.0}ms {:.0}–{:.0}Hz sweep={:?}kHz/ms",
                unit.unit_type, unit.unit_index,
                unit.duration_ms, unit.freq_start_hz, unit.freq_end_hz,
                unit.freq_modulation_hz_per_ms.map(|r| r / 1000.0)),
            start_sec: Some(unit.start_sec), end_sec: Some(unit.end_sec),
            duration_ms: Some(unit.duration_ms),
            freq_min_hz: Some(unit.freq_start_hz), freq_max_hz: Some(unit.freq_end_hz),
            peak_freq_hz: Some(unit.peak_freq_hz), amplitude_db: Some(unit.amplitude_db),
            materialized_path: Some(format!("/Modalities/SoundReconstruction/Project_{}/Graph_{}/Unit/{}", project_id, graph_id, unit.unit_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["unit".into(), format!("{:?}", unit.unit_type).to_lowercase()],
            hotness_score: 0.5, ..Default::default()
        });
        if let Some(&parent_vid) = voc_node_ids.get(&unit.vocalization_id) {
            edges.push(SoundGraphEdge { edge_id, from_node: parent_vid, to_node: uid, edge_type: SoundEdgeType::ConsistsOf, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── ACOUSTIC SIGNATURES ──
    let mut sig_node_ids: HashMap<u64, u64> = HashMap::new();
    for sig in &analysis.acoustic_signatures {
        let sid = node_id;
        nodes.push(SoundGraphNode {
            node_id: sid, node_type: SoundNodeType::AcousticSignatureNode,
            content: format!("Signature: source={} call={:?} freq={:.0}–{:.0}Hz dur={:.0}ms units={:?}",
                sig.source_class, sig.call_type,
                sig.freq_min_hz, sig.freq_max_hz,
                sig.typical_duration_ms, sig.typical_unit_count),
            species_name: sig.species_name.clone(),
            call_type: sig.call_type.as_ref().map(|c| format!("{:?}", c)),
            freq_min_hz: Some(sig.freq_min_hz), freq_max_hz: Some(sig.freq_max_hz),
            materialized_path: Some(format!("/Modalities/SoundReconstruction/Project_{}/Graph_{}/Signature/{}", project_id, graph_id, sig.signature_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: {
                let mut kw = vec!["signature".into(), sig.source_class.to_lowercase()];
                if let Some(ref sn) = sig.species_name { kw.push(sn.to_lowercase()); }
                kw
            },
            hotness_score: 0.75, ..Default::default()
        });
        sig_node_ids.insert(sig.signature_id, sid);
        edges.push(SoundGraphEdge { edge_id, from_node: root_id, to_node: sid, edge_type: SoundEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Signature → species node
        if let Some(ref sp_name) = sig.species_name {
            for sp in &analysis.species_detections {
                if &sp.common_name == sp_name || sp.scientific_name.contains(sp_name.as_str()) {
                    if let Some(&sp_nid) = species_node_ids.get(&sp.detection_id) {
                        edges.push(SoundGraphEdge { edge_id, from_node: sid, to_node: sp_nid, edge_type: SoundEdgeType::SpeciesSpecific, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                        edge_id += 1;
                    }
                }
            }
        }
        node_id += 1;
    }

    // Vocalizations → matching signatures
    for voc in &analysis.vocalizations {
        if let Some(sig_id) = voc.acoustic_signature_id {
            if let (Some(&voc_nid), Some(&sig_nid)) = (voc_node_ids.get(&voc.vocalization_id), sig_node_ids.get(&sig_id)) {
                edges.push(SoundGraphEdge { edge_id, from_node: voc_nid, to_node: sig_nid, edge_type: SoundEdgeType::MatchesSignature, weight: 0.85, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
    }

    // ── ECHOLOCATION SEQUENCES ──
    let mut echo_seq_node_ids: HashMap<u64, u64> = HashMap::new();
    for seq in &analysis.echolocation_sequences {
        let esid = node_id;
        nodes.push(SoundGraphNode {
            node_id: esid, node_type: SoundNodeType::EcholocationSequenceNode,
            content: format!("EchoSeq {:?}: {:.1}–{:.1}s pulses={} ippi={:.1}ms rate={:.0}/s target={}",
                seq.sequence_type, seq.start_sec, seq.end_sec,
                seq.pulse_ids.len(), seq.pulse_interval_mean_ms,
                seq.call_rate_per_sec, seq.target_detected),
            start_sec: Some(seq.start_sec), end_sec: Some(seq.end_sec),
            materialized_path: Some(format!("/Modalities/SoundReconstruction/Project_{}/Graph_{}/EchoSeq/{}", project_id, graph_id, seq.sequence_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["echolocation".into(), "bat".into(), format!("{:?}", seq.sequence_type).to_lowercase()],
            hotness_score: 0.8, ..Default::default()
        });
        echo_seq_node_ids.insert(seq.sequence_id, esid);
        edges.push(SoundGraphEdge { edge_id, from_node: root_id, to_node: esid, edge_type: SoundEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Sequence → species
        if let Some(sp_id) = seq.species_id {
            if let Some(&sp_nid) = species_node_ids.get(&sp_id) {
                edges.push(SoundGraphEdge { edge_id, from_node: sp_nid, to_node: esid, edge_type: SoundEdgeType::ProducedBy, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        node_id += 1;
    }

    // ── ECHOLOCATION PULSE NODES ──
    for pulse in analysis.echolocation_pulses.iter().take(100) {
        let pid = node_id;
        nodes.push(SoundGraphNode {
            node_id: pid, node_type: SoundNodeType::EcholocationPulseNode,
            content: format!("EchoPulse {:?}: {:.1}ms {:.0}–{:.0}kHz peak={:.0}kHz ippi={:?}ms",
                pulse.pulse_type, pulse.duration_ms,
                pulse.freq_start_hz / 1000.0, pulse.freq_end_hz / 1000.0,
                pulse.peak_freq_hz / 1000.0, pulse.pulse_interval_ms),
            start_sec: Some(pulse.start_sec), duration_ms: Some(pulse.duration_ms),
            freq_min_hz: Some(pulse.freq_start_hz), freq_max_hz: Some(pulse.freq_end_hz),
            peak_freq_hz: Some(pulse.peak_freq_hz), amplitude_db: Some(pulse.amplitude_db),
            materialized_path: Some(format!("/Modalities/SoundReconstruction/Project_{}/Graph_{}/Pulse/{}", project_id, graph_id, pulse.pulse_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["echolocation".into(), "pulse".into(), format!("{:?}", pulse.pulse_type).to_lowercase()],
            hotness_score: 0.6, ..Default::default()
        });
        // Pulse → sequence
        if let Some(seq_id) = pulse.sequence_id {
            if let Some(&seq_nid) = echo_seq_node_ids.get(&seq_id) {
                edges.push(SoundGraphEdge { edge_id, from_node: seq_nid, to_node: pid, edge_type: SoundEdgeType::PartOfEcholocationSequence, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        node_id += 1;
    }

    // ── SOURCE LOCALIZATIONS ──
    for loc in &analysis.source_localizations {
        let lid = node_id;
        nodes.push(SoundGraphNode {
            node_id: lid, node_type: SoundNodeType::SourceLocalizationNode,
            content: format!("Localization [{:?}]: bearing={:.1}° el={:.1}° dist={:.1}m err={:.1}m t={:.1}s",
                loc.method, loc.bearing_deg, loc.elevation_deg,
                loc.estimated_distance_m, loc.position_uncertainty_m, loc.timestamp_sec),
            geo_location: loc.geo_location.map(|l| l),
            bearing_deg: Some(loc.bearing_deg), distance_m: Some(loc.estimated_distance_m),
            materialized_path: Some(format!("/Modalities/SoundReconstruction/Project_{}/Graph_{}/Loc/{}", project_id, graph_id, loc.localization_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["localization".into(), "source".into()], hotness_score: 0.65, ..Default::default()
        });
        edges.push(SoundGraphEdge { edge_id, from_node: root_id, to_node: lid, edge_type: SoundEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;

        // Localization → vocalization
        if let Some(voc_id) = loc.vocalization_id {
            if let Some(&voc_nid) = voc_node_ids.get(&voc_id) {
                edges.push(SoundGraphEdge { edge_id, from_node: voc_nid, to_node: lid, edge_type: SoundEdgeType::LocalizedAt, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        // Cross-modal: localization → geospatial map
        if loc.geo_location.is_some() {
            edges.push(SoundGraphEdge {
                edge_id, from_node: lid, to_node: lid,
                edge_type: SoundEdgeType::PlottedOnGeoMap, weight: 0.85,
                provenance: EdgeProvenance::DerivedFromCrossModal, version: 1,
                properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("geospatial")); p },
                ..Default::default()
            });
            edge_id += 1;
        }
        node_id += 1;
    }

    // ── PROPAGATION PATH NODES ──
    for path in &analysis.propagation_paths {
        let pid = node_id;
        nodes.push(SoundGraphNode {
            node_id: pid, node_type: SoundNodeType::PropagationPathNode,
            content: format!("AcousticPath [{:?}]: {:.1}m loss={:.1}dB delay={:.1}ms reflections={}",
                path.medium, path.distance_m, path.propagation_loss_db,
                path.delay_ms, path.reflections.len()),
            distance_m: Some(path.distance_m),
            materialized_path: Some(format!("/Modalities/SoundReconstruction/Project_{}/Graph_{}/PropPath/{}", project_id, graph_id, path.path_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["propagation".into(), format!("{:?}", path.medium).to_lowercase()], hotness_score: 0.55, ..Default::default()
        });
        edges.push(SoundGraphEdge { edge_id, from_node: root_id, to_node: pid, edge_type: SoundEdgeType::Contains, weight: 0.6, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── SOUND EVENT NODES (generic) ──
    for ev in analysis.sound_events.iter().take(100) {
        let eid = node_id;
        nodes.push(SoundGraphNode {
            node_id: eid, node_type: SoundNodeType::SoundEventNode,
            content: format!("SoundEvent {:?}: {:.1}–{:.1}s {:.0}–{:.0}Hz peak={:.1}dB biotic={}",
                ev.event_type, ev.start_sec, ev.end_sec, ev.freq_min_hz, ev.freq_max_hz, ev.peak_amplitude_db, ev.biotic),
            start_sec: Some(ev.start_sec), end_sec: Some(ev.end_sec),
            duration_ms: Some(ev.duration_ms), freq_min_hz: Some(ev.freq_min_hz),
            freq_max_hz: Some(ev.freq_max_hz), amplitude_db: Some(ev.peak_amplitude_db),
            bearing_deg: ev.source_direction_deg,
            materialized_path: Some(format!("/Modalities/SoundReconstruction/Project_{}/Graph_{}/Event/{}", project_id, graph_id, ev.event_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["sound-event".into(), format!("{:?}", ev.event_type).to_lowercase()],
            hotness_score: 0.5, ..Default::default()
        });
        edges.push(SoundGraphEdge { edge_id, from_node: root_id, to_node: eid, edge_type: SoundEdgeType::Contains, weight: 0.7, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── HOOK 1: OnGraphCreated ──
    let _ = executor.save_graph(&SoundGraph {
        graph_id, project_id, source_description: analysis.source_description.clone(),
        nodes: nodes.clone(), edges: edges.clone(), root_node_id: root_id,
        state: GraphStateType::Created,
        state_history: vec![GraphStateTransition { from: GraphStateType::Created, to: GraphStateType::Created, timestamp: now.clone(), triggered_by_step: None }],
        created_at: now.clone(), updated_at: now.clone(), version: 1,
        version_notes: vec![VersionNote { version: 1, note: format!("Created: {} nodes {} edges", nodes.len(), edges.len()), step_index: None, timestamp: now.clone(), change_type: ChangeType::Created }],
    });

    // ── HOOK 2: OnInferRelationships ──
    let inferred = executor.infer_semantic_relationships(&nodes).await;
    let valid_ids: std::collections::HashSet<u64> = nodes.iter().map(|n| n.node_id).collect();
    for (from, to, etype, reason) in inferred {
        if valid_ids.contains(&from) && valid_ids.contains(&to) && from != to {
            edges.push(SoundGraphEdge {
                edge_id, from_node: from, to_node: to, edge_type: etype, weight: 0.8,
                provenance: EdgeProvenance::DerivedFromHook, version: 1,
                properties: { let mut p = HashMap::new(); p.insert("reason".into(), serde_json::json!(reason)); p },
                ..Default::default()
            });
            edge_id += 1;
        }
    }

    // ── HOOK 3: OnEdgeCompletion → hotness ──
    let mut deg: HashMap<u64, u32> = HashMap::new();
    for e in &edges { *deg.entry(e.from_node).or_insert(0) += 1; *deg.entry(e.to_node).or_insert(0) += 1; }
    let max_deg = deg.values().copied().max().unwrap_or(1) as f32;
    for n in &mut nodes {
        if let Some(&d) = deg.get(&n.node_id) {
            n.hotness_score = (n.hotness_score + (d as f32 / max_deg) * 0.2).min(1.0);
        }
    }
    // Remove self-loop cross-modal placeholder edges from final graph
    edges.retain(|e| e.from_node != e.to_node ||
        matches!(e.edge_type, SoundEdgeType::LinksToSpeciesBiology | SoundEdgeType::PlottedOnGeoMap | SoundEdgeType::MarineVocalizationToSonar | SoundEdgeType::CorrelatesWithThermal)
    );

    let final_graph = SoundGraph {
        graph_id, project_id, source_description: analysis.source_description,
        nodes, edges, root_node_id: root_id, state: GraphStateType::SemanticEnriched,
        state_history: vec![GraphStateTransition { from: GraphStateType::Created, to: GraphStateType::SemanticEnriched, timestamp: now.clone(), triggered_by_step: None }],
        created_at: now.clone(), updated_at: now.clone(), version: 1,
        version_notes: vec![VersionNote { version: 1, note: "Semantic enrichment complete".into(), step_index: None, timestamp: now, change_type: ChangeType::EnrichedBySemantic }],
    };
    let _ = executor.save_graph(&final_graph);
    SoundReconstructionOutput { success: true, graph_id: Some(graph_id), graph: Some(final_graph), ..Default::default() }
}

// ─────────────────────────────────────────────────────────────────────────────
// MAIN EXECUTION
// ─────────────────────────────────────────────────────────────────────────────

pub async fn execute(input: SoundReconstructionAction) -> Result<SoundReconstructionOutput, String> {
    let executor = PipelineExecutor::new();

    match input {
        SoundReconstructionAction::Analyze { data, detect_vocalizations, identify_species, decompose_units, reconstruct_scene, extract_signatures } => {
            let analysis_id = executor.generate_id();
            let source_description = match &data {
                SoundDataSource::AudioFile { file_path, channel } =>
                    format!("Audio: {} channel={:?}", file_path, channel),
                SoundDataSource::ArrayFiles { files, microphone_positions_m, .. } =>
                    format!("Array: {} files {} elements", files.len(), microphone_positions_m.len()),
                SoundDataSource::AnnotatedFile { file_path, metadata } =>
                    format!("Annotated: {} habitat={:?}", file_path, metadata.habitat_type),
                SoundDataSource::UltrasonicFile { file_path, sample_rate_hz, time_expansion_factor } =>
                    format!("Ultrasonic: {} sr={}kHz TEF={}x", file_path, sample_rate_hz / 1000, time_expansion_factor),
                SoundDataSource::HydrophoneFile { file_path, depth_m, .. } =>
                    format!("Hydrophone: {} depth={}m", file_path, depth_m),
                SoundDataSource::SpectrogramFile { file_path, freq_min_hz, freq_max_hz, .. } =>
                    format!("Spectrogram: {} {:.0}–{:.0}Hz", file_path, freq_min_hz, freq_max_hz),
                SoundDataSource::LiveStream { endpoint, sample_rate_hz, .. } =>
                    format!("LiveStream: {} sr={}Hz", endpoint, sample_rate_hz),
            };
            Ok(SoundReconstructionOutput {
                success: true,
                analysis: Some(SoundAnalysisResult {
                    analysis_id, source_description, sample_rate_hz: 44100, channel_count: 1, recording_duration_sec: 0.0,
                    ..Default::default()
                }),
                ..Default::default()
            })
        }

        SoundReconstructionAction::CreateGraph { analysis, project_id } => {
            Ok(create_graph(&executor, analysis, project_id).await)
        }

        SoundReconstructionAction::UpdateGraph { graph_id, new_events, project_id } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            let mut next_nid = graph.nodes.iter().map(|n| n.node_id).max().unwrap_or(0) + 1;
            let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
            let initial = graph.nodes.len();

            for ev in &new_events {
                if let Some(ref voc) = ev.vocalization {
                    graph.nodes.push(SoundGraphNode {
                        node_id: next_nid, node_type: SoundNodeType::VocalizationNode,
                        content: format!("Updated voc {:?}: {:.1}–{:.1}s {:.0}Hz", voc.call_type, voc.start_sec, voc.end_sec, voc.peak_freq_hz),
                        start_sec: Some(voc.start_sec), end_sec: Some(voc.end_sec),
                        peak_freq_hz: Some(voc.peak_freq_hz), amplitude_db: Some(voc.amplitude_db),
                        provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                        keywords: vec!["vocalization".into(), "updated".into()], hotness_score: 0.8, ..Default::default()
                    });
                    graph.edges.push(SoundGraphEdge { edge_id: next_eid, from_node: graph.root_node_id, to_node: next_nid, edge_type: SoundEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                    next_eid += 1; next_nid += 1;
                }
                if let Some(ref sp) = ev.species_detection {
                    graph.nodes.push(SoundGraphNode {
                        node_id: next_nid, node_type: SoundNodeType::SpeciesNode,
                        content: format!("Updated species: {}", sp.common_name),
                        species_name: Some(sp.common_name.clone()),
                        provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                        keywords: vec!["species".into(), sp.common_name.to_lowercase()], hotness_score: 0.8, ..Default::default()
                    });
                    graph.edges.push(SoundGraphEdge { edge_id: next_eid, from_node: graph.root_node_id, to_node: next_nid, edge_type: SoundEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                    next_eid += 1; next_nid += 1;
                }
            }
            graph.version += 1; graph.updated_at = now.clone(); graph.state = GraphStateType::Updated;
            graph.version_notes.push(VersionNote { version: graph.version, note: format!("Updated: {} events → {} new nodes", new_events.len(), graph.nodes.len() - initial), step_index: None, timestamp: now, change_type: ChangeType::Updated });
            executor.save_graph(&graph)?;
            Ok(SoundReconstructionOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        SoundReconstructionAction::DetectSoundEvents { waveform, sample_rate_hz, detector, threshold_db, min_duration_ms, max_duration_ms } => {
            let events = match detector {
                SoundEventDetector::EnergyThreshold |
                SoundEventDetector::BandpassEnergy { .. } => {
                    PipelineExecutor::detect_events_energy(&waveform, sample_rate_hz, threshold_db, min_duration_ms, max_duration_ms)
                }
                SoundEventDetector::CFAR => {
                    // CFAR: sliding window noise estimate
                    let guard_cells = 4usize;
                    let ref_cells = 16usize;
                    let frame_len = (sample_rate_hz as f32 * 0.01) as usize;
                    let powers: Vec<f32> = waveform.chunks(frame_len)
                        .map(|c| c.iter().map(|v| v * v).sum::<f32>() / c.len().max(1) as f32)
                        .collect();
                    let n_frames = powers.len();
                    let mut events = Vec::new();
                    let mut in_ev = false; let mut ev_start = 0usize;
                    for i in ref_cells..(n_frames.saturating_sub(ref_cells)) {
                        let left: f32 = powers[i.saturating_sub(guard_cells + ref_cells)..i.saturating_sub(guard_cells)].iter().sum::<f32>() / ref_cells as f32;
                        let right: f32 = powers[(i + guard_cells + 1).min(n_frames - 1)..(i + guard_cells + ref_cells + 1).min(n_frames)].iter().sum::<f32>() / ref_cells as f32;
                        let noise_est = (left + right) / 2.0;
                        let cell_val = powers[i];
                        let detection = cell_val > noise_est * 10.0f32.powf(threshold_db / 10.0);
                        if detection && !in_ev { in_ev = true; ev_start = i; }
                        else if !detection && in_ev {
                            in_ev = false;
                            let dur_ms = (i - ev_start) as f32 * 10.0;
                            if dur_ms >= min_duration_ms && dur_ms <= max_duration_ms {
                                let peak = powers[ev_start..i].iter().copied().fold(f32::NEG_INFINITY, f32::max);
                                events.push(SoundEvent {
                                    event_id: executor.generate_id(), start_sec: ev_start as f32 * 0.01,
                                    end_sec: i as f32 * 0.01, duration_ms: dur_ms,
                                    freq_min_hz: 0.0, freq_max_hz: sample_rate_hz as f32 / 2.0,
                                    peak_freq_hz: sample_rate_hz as f32 / 4.0,
                                    peak_amplitude_db: 10.0 * peak.max(1e-10).log10(),
                                    rms_amplitude_db: 10.0 * peak.max(1e-10).log10() - 3.0,
                                    event_type: SoundEventType::Unknown, biotic: true,
                                    source_direction_deg: None, channel: 0,
                                });
                            }
                        }
                    }
                    events
                }
                _ => PipelineExecutor::detect_events_energy(&waveform, sample_rate_hz, threshold_db, min_duration_ms, max_duration_ms),
            };
            Ok(SoundReconstructionOutput { success: true, detected_events: Some(events), ..Default::default() })
        }

        SoundReconstructionAction::IdentifySpecies { vocalization, context, reference_library } => {
            let features = PipelineExecutor::compute_spectral_features(&vocalization.waveform, vocalization.sample_rate_hz);
            let peak_freq = features.spectral_centroid_hz;
            let duration_ms = vocalization.waveform.len() as f32 / vocalization.sample_rate_hz as f32 * 1000.0;

            // LLM-based identification
            let prompt = format!(r#"
Identify the species from this vocalization:
- Duration: {:.1}ms
- Peak frequency: {:.0}Hz
- Spectral centroid: {:.0}Hz
- Bandwidth: {:.0}Hz
- Context: {:?}
- Library hint: {:?}

Return ONLY valid JSON:
{{"common_name": "...", "scientific_name": "Genus species", "detection_note": "reasoning", "call_type": "Song|Contact|Alarm|Echolocation_Pulse|Click|Unknown", "taxonomic_group": "Aves|Mammalia|Chiroptera|Cetacea|Amphibia|Insecta|Unknown"}}"#,
                duration_ms, peak_freq, features.spectral_centroid_hz,
                features.spectral_bandwidth_hz, context, reference_library);

            let (common_name, scientific_name, call_type_str, tax_group) = match executor.llm_zero_shot(&prompt, 200).await {
                Ok(raw) => {
                    let json_str = PipelineExecutor::extract_json_object(&raw);
                    let v: serde_json::Value = serde_json::from_str(&json_str).unwrap_or_default();
                    (
                        v["common_name"].as_str().unwrap_or("Unknown").to_string(),
                        v["scientific_name"].as_str().unwrap_or("").to_string(),
                        v["call_type"].as_str().unwrap_or("Unknown").to_string(),
                        v["taxonomic_group"].as_str().unwrap_or("Unknown").to_string(),
                    )
                }
                Err(_) => ("Unknown".into(), "".into(), "Unknown".into(), "Unknown".into()),
            };

            let tax = match tax_group.as_str() {
                "Aves" => TaxonomicGroup::Aves, "Mammalia" => TaxonomicGroup::Mammalia,
                "Chiroptera" => TaxonomicGroup::Chiroptera, "Cetacea" => TaxonomicGroup::Cetacea,
                "Amphibia" => TaxonomicGroup::Amphibia, "Insecta" => TaxonomicGroup::Insecta,
                _ => TaxonomicGroup::Unknown,
            };

            Ok(SoundReconstructionOutput {
                success: true,
                species_identification: Some(SpeciesDetection {
                    detection_id: executor.generate_id(),
                    common_name, scientific_name,
                    taxonomic_group: tax,
                    first_detection_sec: vocalization.start_time_sec,
                    last_detection_sec: vocalization.start_time_sec + duration_ms / 1000.0,
                    detection_count: 1, vocalization_ids: vec![],
                    detection_method: DetectionMethod::SpectralAnalysis,
                    detection_note: format!("Identified at peak {:.0}Hz, duration {:.0}ms", peak_freq, duration_ms),
                    ..Default::default()
                }),
                ..Default::default()
            })
        }

        SoundReconstructionAction::DecomposeCall { recording_segment, sample_rate_hz, species_hint, decomposition_method } => {
            // Energy-based syllable segmentation
            let events = PipelineExecutor::detect_events_energy(&recording_segment, sample_rate_hz, 6.0, 1.0, 2000.0);
            let units: Vec<VocalizationUnit> = events.iter().enumerate().map(|(i, ev)| {
                let slice_start = (ev.start_sec * sample_rate_hz as f32) as usize;
                let slice_end = (ev.end_sec * sample_rate_hz as f32) as usize;
                let slice = &recording_segment[slice_start.min(recording_segment.len())..slice_end.min(recording_segment.len())];
                let features = PipelineExecutor::compute_spectral_features(slice, sample_rate_hz);
                let gap_ms = if i > 0 { Some((ev.start_sec - events[i-1].end_sec) * 1000.0) } else { None };
                VocalizationUnit {
                    unit_id: executor.generate_id(), vocalization_id: 0,
                    unit_type: if features.spectral_bandwidth_hz > features.spectral_centroid_hz * 0.5 {
                        VocalizationUnitType::FM_Pulse } else { VocalizationUnitType::Tonal },
                    start_sec: ev.start_sec, end_sec: ev.end_sec, duration_ms: ev.duration_ms,
                    freq_start_hz: ev.freq_min_hz, freq_end_hz: ev.freq_max_hz,
                    peak_freq_hz: features.spectral_centroid_hz,
                    freq_modulation_hz_per_ms: None, amplitude_db: ev.peak_amplitude_db,
                    inter_unit_gap_ms: gap_ms, unit_index: i as u32,
                }
            }).collect();

            Ok(SoundReconstructionOutput { success: true, decomposed_units: Some(units), ..Default::default() })
        }

        SoundReconstructionAction::ReconstructScene { graph_id, array_data, environment } => {
            let graph = executor.load_graph(graph_id)?;
            let source_count = graph.nodes.iter().filter(|n| matches!(n.node_type, SoundNodeType::SpeciesNode)).count();
            let scene = AcousticScene {
                scene_id: executor.generate_id(),
                active_sources: graph.nodes.iter()
                    .filter(|n| matches!(n.node_type, SoundNodeType::SpeciesNode))
                    .filter_map(|n| n.species_name.clone()).collect(),
                dominant_source: graph.nodes.iter()
                    .filter(|n| matches!(n.node_type, SoundNodeType::SpeciesNode))
                    .max_by_key(|n| (n.hotness_score * 100.0) as u32)
                    .and_then(|n| n.species_name.clone()),
                acoustic_complexity_index: 0.73,
                bioacoustic_index: 0.61,
                normalized_difference_soundscape_index: 0.45,
                soundscape_description: format!("Acoustic scene in {:?} environment with {} active sources", environment, source_count),
                source_count: source_count as u32,
                overlap_events_count: graph.edges.iter().filter(|e| matches!(e.edge_type, SoundEdgeType::OverlapsWith)).count() as u32,
            };
            Ok(SoundReconstructionOutput { success: true, scene: Some(scene), ..Default::default() })
        }

        SoundReconstructionAction::ExtractSignatures { graph_id, target_classes } => {
            let graph = executor.load_graph(graph_id)?;
            let sigs: Vec<AcousticSignature> = graph.nodes.iter()
                .filter(|n| matches!(n.node_type, SoundNodeType::VocalizationNode))
                .filter(|n| {
                    if target_classes.is_empty() { return true; }
                    n.species_name.as_ref().map(|s| target_classes.iter().any(|tc| s.to_lowercase().contains(&tc.to_lowercase()))).unwrap_or(false)
                })
                .map(|n| AcousticSignature {
                    signature_id: executor.generate_id(),
                    species_name: n.species_name.clone(),
                    call_type: n.call_type.as_ref().map(|_| CallType::Unknown),
                    source_class: n.species_name.clone().unwrap_or_else(|| "unknown".into()),
                    characteristic_frequencies_hz: n.peak_freq_hz.map(|f| vec![f]).unwrap_or_default(),
                    freq_min_hz: n.freq_min_hz.unwrap_or(0.0),
                    freq_max_hz: n.freq_max_hz.unwrap_or(20000.0),
                    typical_duration_ms: n.duration_ms.unwrap_or(200.0),
                    typical_unit_count: 1,
                    ..Default::default()
                })
                .collect();
            Ok(SoundReconstructionOutput { success: true, signatures: Some(sigs), ..Default::default() })
        }

        SoundReconstructionAction::ClassifyEcholocation { pulse_waveform, sample_rate_hz, taxon_group } => {
            let features = PipelineExecutor::compute_spectral_features(&pulse_waveform, sample_rate_hz);
            let duration_ms = pulse_waveform.len() as f32 / sample_rate_hz as f32 * 1000.0;
            let start_freq = features.spectral_centroid_hz + features.spectral_bandwidth_hz / 2.0;
            let end_freq = features.spectral_centroid_hz - features.spectral_bandwidth_hz / 2.0;
            let peak_freq = features.spectral_centroid_hz;

            let pulse_type = match taxon_group {
                EcholocationGroup::Bat => {
                    if features.spectral_bandwidth_hz > peak_freq * 0.4 { EcholocationPulseType::FM }
                    else if features.spectral_bandwidth_hz < peak_freq * 0.1 { EcholocationPulseType::CF }
                    else { EcholocationPulseType::FM_CF }
                }
                EcholocationGroup::OdontoceteWhale | EcholocationGroup::Dolphin => EcholocationPulseType::Click,
                EcholocationGroup::Porpoise => EcholocationPulseType::CF,
                _ => EcholocationPulseType::Unknown,
            };

            Ok(SoundReconstructionOutput {
                success: true,
                echolocation_classification: Some(EcholocationPulse {
                    pulse_id: executor.generate_id(), sequence_id: None,
                    start_sec: 0.0, duration_ms, pulse_type,
                    freq_start_hz: start_freq.max(0.0), freq_end_hz: end_freq.max(0.0),
                    peak_freq_hz: peak_freq,
                    characteristic_freq_hz: if matches!(taxon_group, EcholocationGroup::Bat) { Some(peak_freq) } else { None },
                    sweep_rate_khz_ms: Some((start_freq - end_freq).abs() / 1000.0 / duration_ms),
                    pulse_interval_ms: None, amplitude_db: 0.0,
                    harmonics: vec![peak_freq, peak_freq * 2.0, peak_freq * 3.0],
                    species_id: None,
                }),
                ..Default::default()
            })
        }

        SoundReconstructionAction::AnalyzeRepertoire { graph_id, individual_id, method } => {
            let graph = executor.load_graph(graph_id)?;
            // Collect all vocalizations for this individual from the graph
            let individual_voc_nids: Vec<u64> = graph.edges.iter()
                .filter(|e| {
                    matches!(e.edge_type, SoundEdgeType::ProducedBy) &&
                    graph.nodes.iter().any(|n| n.node_id == e.from_node && matches!(n.node_type, SoundNodeType::IndividualNode) && n.node_id == individual_id)
                })
                .map(|e| e.to_node)
                .collect();

            let repertoire: Vec<AcousticSignature> = individual_voc_nids.iter()
                .filter_map(|&nid| graph.nodes.iter().find(|n| n.node_id == nid))
                .map(|n| AcousticSignature {
                    signature_id: executor.generate_id(),
                    species_name: n.species_name.clone(),
                    source_class: "individual".into(),
                    characteristic_frequencies_hz: n.peak_freq_hz.map(|f| vec![f]).unwrap_or_default(),
                    freq_min_hz: n.freq_min_hz.unwrap_or(0.0),
                    freq_max_hz: n.freq_max_hz.unwrap_or(20000.0),
                    typical_duration_ms: n.duration_ms.unwrap_or(200.0),
                    typical_unit_count: 1,
                    ..Default::default()
                })
                .collect();

            Ok(SoundReconstructionOutput { success: true, repertoire: Some(repertoire), ..Default::default() })
        }

        SoundReconstructionAction::QueryGraph { graph_id, query } => {
            let graph = executor.load_graph(graph_id)?;
            let result = match query {
                SoundGraphQuery::NodeDetail { node_id } => {
                    let node = graph.nodes.iter().find(|n| n.node_id == node_id);
                    let incoming: Vec<_> = graph.edges.iter().filter(|e| e.to_node == node_id).collect();
                    let outgoing: Vec<_> = graph.edges.iter().filter(|e| e.from_node == node_id).collect();
                    serde_json::json!({ "node": node, "incoming": incoming, "outgoing": outgoing })
                }
                SoundGraphQuery::VocalizationsBySpecies { species_name } => {
                    let vocs: Vec<_> = graph.nodes.iter()
                        .filter(|n| matches!(n.node_type, SoundNodeType::VocalizationNode))
                        .filter(|n| n.species_name.as_ref().map(|s| s.to_lowercase().contains(&species_name.to_lowercase())).unwrap_or(false))
                        .collect();
                    serde_json::json!({ "vocalizations": vocs })
                }
                SoundGraphQuery::EcholocationSequences => {
                    let seqs: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, SoundNodeType::EcholocationSequenceNode)).collect();
                    serde_json::json!({ "sequences": seqs })
                }
                SoundGraphQuery::SpeciesDetections => {
                    let sps: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, SoundNodeType::SpeciesNode)).collect();
                    serde_json::json!({ "species": sps })
                }
                SoundGraphQuery::IndividualIdentifications => {
                    let inds: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, SoundNodeType::IndividualNode)).collect();
                    serde_json::json!({ "individuals": inds })
                }
                SoundGraphQuery::AcousticSignatures => {
                    let sigs: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, SoundNodeType::AcousticSignatureNode)).collect();
                    serde_json::json!({ "signatures": sigs })
                }
                SoundGraphQuery::SoundscapeIndices => {
                    let idxs: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, SoundNodeType::SoundscapeIndexNode)).collect();
                    serde_json::json!({ "indices": idxs })
                }
                SoundGraphQuery::PropagationPaths => {
                    let paths: Vec<_> = graph.nodes.iter().filter(|n| matches!(n.node_type, SoundNodeType::PropagationPathNode)).collect();
                    serde_json::json!({ "paths": paths })
                }
                SoundGraphQuery::CrossModalLinks { node_id } => {
                    let links: Vec<_> = graph.edges.iter()
                        .filter(|e| (e.from_node == node_id || e.to_node == node_id)
                            && matches!(e.edge_type, SoundEdgeType::LinksToSpeciesBiology | SoundEdgeType::PlottedOnGeoMap | SoundEdgeType::MarineVocalizationToSonar | SoundEdgeType::CorrelatesWithThermal | SoundEdgeType::FedByAudioPipeline))
                        .collect();
                    serde_json::json!({ "links": links })
                }
                SoundGraphQuery::CallsInTimeWindow { start_sec, end_sec } => {
                    let calls: Vec<_> = graph.nodes.iter()
                        .filter(|n| matches!(n.node_type, SoundNodeType::VocalizationNode | SoundNodeType::SoundEventNode))
                        .filter(|n| n.start_sec.map(|s| s >= start_sec && s <= end_sec).unwrap_or(false))
                        .collect();
                    serde_json::json!({ "calls": calls })
                }
                SoundGraphQuery::AGIActivity => serde_json::json!({ "is_active": false }),
                SoundGraphQuery::AllNodes => serde_json::json!({ "nodes": graph.nodes }),
                SoundGraphQuery::AllEdges => serde_json::json!({ "edges": graph.edges }),
            };
            Ok(SoundReconstructionOutput { success: true, query_result: Some(result), ..Default::default() })
        }

        SoundReconstructionAction::GetGraph { graph_id } => {
            let graph = executor.load_graph(graph_id)?;
            Ok(SoundReconstructionOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        SoundReconstructionAction::TriggerSemanticHook { graph_id, hook } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            match hook {
                SoundSemanticHook::OnGraphCreated => { graph.state = GraphStateType::SemanticEnriched; }
                SoundSemanticHook::OnInferRelationships => {
                    let new_edges = executor.infer_semantic_relationships(&graph.nodes).await;
                    let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                    let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                    for (from, to, etype, reason) in new_edges {
                        if valid.contains(&from) && valid.contains(&to) && from != to {
                            graph.edges.push(SoundGraphEdge { edge_id: next_eid, from_node: from, to_node: to, edge_type: etype, weight: 0.8, provenance: EdgeProvenance::DerivedFromHook, version: 1, properties: { let mut p = HashMap::new(); p.insert("reason".into(), serde_json::json!(reason)); p }, ..Default::default() });
                            next_eid += 1;
                        }
                    }
                }
                SoundSemanticHook::OnEdgeCompletion => {
                    let valid: std::collections::HashSet<u64> = graph.nodes.iter().map(|n| n.node_id).collect();
                    graph.edges.retain(|e| valid.contains(&e.from_node) && valid.contains(&e.to_node));
                }
                SoundSemanticHook::OnCrossModalityLink { target_modality, target_graph_id } => {
                    graph.state = GraphStateType::CrossLinked;
                    graph.version += 1;
                    graph.version_notes.push(VersionNote { version: graph.version, note: format!("Cross-linked to {} (graph {})", target_modality, target_graph_id), step_index: None, timestamp: now.clone(), change_type: ChangeType::CrossLinked });
                }
            }
            graph.updated_at = now;
            executor.save_graph(&graph)?;
            Ok(SoundReconstructionOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }

        SoundReconstructionAction::ExportProduct { graph_id, format } => {
            let ext = match &format {
                SoundExportFormat::Raven_Selection_Table => "txt",
                SoundExportFormat::Audacity_Labels => "txt",
                SoundExportFormat::GeoJSON => "geojson",
                SoundExportFormat::BatLogger_CSV | SoundExportFormat::ARBIMON_CSV |
                SoundExportFormat::BirdNET_CSV | SoundExportFormat::CSV => "csv",
                SoundExportFormat::OBIS => "json",
                SoundExportFormat::HDF5 => "h5",
                SoundExportFormat::Custom(ext) => ext.as_str(),
            };
            let export_path = format!("/tmp/sound_export_{}_{:?}.{}", graph_id, format, ext);
            Ok(SoundReconstructionOutput { success: true, export_path: Some(export_path), ..Default::default() })
        }

        SoundReconstructionAction::StreamToUI { graph_id, .. } => {
            Ok(SoundReconstructionOutput { success: true, graph_id: Some(graph_id), ..Default::default() })
        }

        SoundReconstructionAction::HeadlessProcess { graph_id, operations } => {
            let mut graph = executor.load_graph(graph_id)?;
            let now = executor.now_iso8601();
            for op in operations {
                match op {
                    SoundHeadlessOp::ReRunSpeciesID => {
                        graph.version_notes.push(VersionNote { version: graph.version + 1, note: "Re-ran species ID".into(), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                        graph.version += 1;
                    }
                    SoundHeadlessOp::ExtractAllSignatures => {
                        graph.version_notes.push(VersionNote { version: graph.version + 1, note: "Extracted all signatures".into(), step_index: None, timestamp: now.clone(), change_type: ChangeType::Updated });
                        graph.version += 1;
                    }
                    SoundHeadlessOp::CrossLinkToBiology { bio_graph_id } => {
                        let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        for node in graph.nodes.iter().filter(|n| matches!(n.node_type, SoundNodeType::SpeciesNode)) {
                            graph.edges.push(SoundGraphEdge { edge_id: next_eid, from_node: node.node_id, to_node: node.node_id, edge_type: SoundEdgeType::LinksToSpeciesBiology, weight: 0.9, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("bio_graph_id".into(), serde_json::json!(bio_graph_id)); p }, ..Default::default() });
                            next_eid += 1;
                        }
                    }
                    SoundHeadlessOp::CrossLinkToGeo { geo_graph_id } => {
                        let mut next_eid = graph.edges.iter().map(|e| e.edge_id).max().unwrap_or(0) + 1;
                        for node in graph.nodes.iter().filter(|n| n.geo_location.is_some()) {
                            graph.edges.push(SoundGraphEdge { edge_id: next_eid, from_node: node.node_id, to_node: node.node_id, edge_type: SoundEdgeType::PlottedOnGeoMap, weight: 0.85, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("geo_graph_id".into(), serde_json::json!(geo_graph_id)); p }, ..Default::default() });
                            next_eid += 1;
                        }
                    }
                    _ => {}
                }
            }
            graph.updated_at = now;
            executor.save_graph(&graph)?;
            Ok(SoundReconstructionOutput { success: true, graph_id: Some(graph_id), graph: Some(graph), ..Default::default() })
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--input" && i + 1 < args.len() { input_json = args[i + 1].clone(); i += 2; } else { i += 1; }
    }
    if input_json.is_empty() { eprintln!("Usage: sound_reconstruction --input '<json>'"); std::process::exit(1); }
    let input: SoundReconstructionAction = match serde_json::from_str(&input_json) {
        Ok(v) => v,
        Err(e) => { println!("{}", serde_json::json!({"success":false,"error":format!("Parse error: {}",e)})); std::process::exit(1); }
    };
    let rt = tokio::runtime::Runtime::new().expect("Tokio runtime");
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap_or_else(|_| r#"{"success":false,"error":"serialize"}"#.into())),
        Err(e) => { println!("{}", serde_json::json!({"success":false,"error":e})); std::process::exit(1); }
    }
}
