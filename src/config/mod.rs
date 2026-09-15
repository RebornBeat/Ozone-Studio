//! Configuration module for Ozone Studio

use crate::OzoneError;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Main configuration for Ozone Studio
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzoneConfig {
    /// General settings
    pub general: GeneralConfig,

    /// ZSEI configuration
    pub zsei: ZSEIConfig,

    /// Pipeline configuration
    pub pipelines: PipelineConfig,

    /// Methodology configuration
    /// methodologies: MethodologyConfig::default(),

    /// Blueprint configuration
    /// blueprints: BlueprintConfig::default(),

    /// Task configuration
    pub tasks: TaskConfig,

    /// Authentication configuration
    pub auth: AuthConfig,

    /// Integrity configuration
    pub integrity: IntegrityConfig,

    /// Network configuration
    pub network: NetworkConfig,

    /// gRPC server configuration
    pub grpc: GrpcConfig,

    /// UI configuration
    pub ui: UIConfig,

    /// Consciousness configuration (enable/disable at runtime via config.toml)
    #[serde(default)]
    pub consciousness: ConsciousnessConfig,

    /// Model configuration (for prompt pipeline)
    pub models: ModelConfig,

    /// Voice configuration (for speech input)
    #[serde(default)]
    pub voice: VoiceConfig,

    /// K-ALGORITHM default presets (src/k_registry.rs). Only convergence and
    /// pairwise are exposed — validation/ordered_loop/search have no live
    /// consumer yet, so a config knob for them would control nothing.
    #[serde(default)]
    pub k_algorithms: KAlgorithmConfig,

    /// Jurisdiction-aware guardrail config. Deliberately a base safety layer,
    /// separate from `consciousness` — the enforcement hook runs regardless
    /// of consciousness_enabled (see stage_jurisdiction_gate). No real legal
    /// content ships here or anywhere in this codebase; instance_region is
    /// an opaque label an operator sets, used only to look up whatever real
    /// JurisdictionRuleSet containers (if any) a human has actually loaded
    /// into ZSEI for that region.
    #[serde(default)]
    pub jurisdiction: JurisdictionConfig,

    /// Real web search (pipeline 56 — see assets/pipelines/general/
    /// web_search). Disabled by default since it requires a real,
    /// user-supplied API key (api_key_env) for an actual external provider —
    /// with no key configured, the pipeline honestly reports "unavailable"
    /// rather than fabricating results.
    #[serde(default)]
    pub web_search: WebSearchConfig,
}

/// See OzoneConfig::web_search's doc comment. `provider` is a label only
/// (currently "brave" is the one real wire format the pipeline implements);
/// `api_key_env` names the environment variable holding the real API key,
/// mirroring ModelConfig::api_key_env's existing convention elsewhere in
/// this file rather than inventing a new one.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_web_search_provider")]
    pub provider: String,
    #[serde(default = "default_web_search_api_key_env")]
    pub api_key_env: String,
    #[serde(default = "default_web_search_endpoint")]
    pub endpoint: String,
}

fn default_web_search_provider() -> String {
    "brave".to_string()
}
fn default_web_search_api_key_env() -> String {
    "BRAVE_SEARCH_API_KEY".to_string()
}
fn default_web_search_endpoint() -> String {
    "https://api.search.brave.com/res/v1/web/search".to_string()
}

impl Default for WebSearchConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: default_web_search_provider(),
            api_key_env: default_web_search_api_key_env(),
            endpoint: default_web_search_endpoint(),
        }
    }
}

impl WebSearchConfig {
    /// Same pattern as ModelConfig/VoiceConfig::to_pipeline_env — applied
    /// once at boot via std::env::set_var (see lib.rs), inherited by every
    /// spawned pipeline subprocess including web_search (56) itself. Does
    /// NOT forward the actual API key value — only which env var name holds
    /// it, so the real key only ever needs to be exported once by whoever
    /// runs the host, same as OPENROUTER_API_KEY today.
    pub fn to_pipeline_env(&self) -> Vec<(String, String)> {
        vec![
            ("OZONE_WEB_SEARCH_ENABLED".to_string(), self.enabled.to_string()),
            ("OZONE_WEB_SEARCH_PROVIDER".to_string(), self.provider.clone()),
            ("OZONE_WEB_SEARCH_API_KEY_ENV".to_string(), self.api_key_env.clone()),
            ("OZONE_WEB_SEARCH_ENDPOINT".to_string(), self.endpoint.clone()),
        ]
    }
}

/// See OzoneConfig::jurisdiction's doc comment — mechanism only, no legal
/// content. `instance_region` is free-form (e.g. "US-CA", "EU-DE") and
/// intentionally not validated against any real list of jurisdictions here;
/// it's just a lookup key into whatever real rulesets a human has loaded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionConfig {
    /// Gates ONLY the National/Local (region-specific) rule layer — never
    /// the Global (U.N.-level) layer, which is a hard invariant enforced
    /// unconditionally by stage_jurisdiction_gate regardless of this flag
    /// (per explicit direction: U.N. scope must always apply whether a
    /// location is configured or not). `enabled=false` means "don't apply
    /// region-specific rules yet", never "skip jurisdiction enforcement".
    #[serde(default = "default_jurisdiction_enabled")]
    pub enabled: bool,
    /// This instance's region, e.g. "US-CA" or a bare ISO country code
    /// like "US". None means no region-specific ruleset applies — only
    /// whatever is registered as Global scope (if anything real has been
    /// loaded).
    ///
    /// An explicit value here (set by whoever deploys the instance, via
    /// config.toml — never exposed as an in-app end-user toggle) always
    /// wins. When left unset, `OzoneConfig::load` auto-fills this from real
    /// hardware/OS signals (system timezone + locale — see
    /// `crate::hardware_region`) rather than leaving it permanently None —
    /// per explicit direction that this must not be a manual, skippable
    /// user setting. It is still never a blind guess: hardware detection
    /// only fills this in when its two independent signals agree with each
    /// other; when they disagree (confirmed to genuinely happen — a real
    /// machine's system timezone and locale pointed at two different
    /// countries during development) this stays None and both raw signals
    /// are logged, since a wrong value here can BLOCK real requests.
    #[serde(default)]
    pub instance_region: Option<String>,
}

fn default_jurisdiction_enabled() -> bool {
    true
}

impl Default for JurisdictionConfig {
    fn default() -> Self {
        Self {
            enabled: default_jurisdiction_enabled(),
            instance_region: None,
        }
    }
}

/// Default preset names for the K-ALGORITHM families that actually have a
/// live consumer (orchestrator/amt.rs). Applied to the global KAlgorithms
/// registry at boot and on /config/set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KAlgorithmConfig {
    /// "fast" (2 passes, default) | "deep" (5 passes)
    #[serde(default = "default_convergence_preset")]
    pub convergence_preset: String,
    /// "default" (window 8 / 50 pairs) | "wide" (16 / 100)
    #[serde(default = "default_pairwise_preset")]
    pub pairwise_preset: String,
}

fn default_convergence_preset() -> String {
    "fast".to_string()
}
fn default_pairwise_preset() -> String {
    "default".to_string()
}

impl Default for KAlgorithmConfig {
    fn default() -> Self {
        Self {
            convergence_preset: default_convergence_preset(),
            pairwise_preset: default_pairwise_preset(),
        }
    }
}

impl Default for OzoneConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            zsei: ZSEIConfig::default(),
            pipelines: PipelineConfig::default(),
            tasks: TaskConfig::default(),
            auth: AuthConfig::default(),
            integrity: IntegrityConfig::default(),
            network: NetworkConfig::default(),
            grpc: GrpcConfig::default(),
            ui: UIConfig::default(),
            consciousness: ConsciousnessConfig::default(),
            models: ModelConfig::default(),
            voice: VoiceConfig::default(),
            k_algorithms: KAlgorithmConfig::default(),
            jurisdiction: JurisdictionConfig::default(),
            web_search: WebSearchConfig::default(),
        }
    }
}

impl OzoneConfig {
    /// Load configuration from a TOML file. Async since hardware-region
    /// detection now includes one real (short-timeout, non-fatal) IP
    /// geolocation call alongside the OS signals — see
    /// `apply_hardware_region_detection` and `crate::hardware_region`.
    pub async fn load(path: &Path) -> Result<Self, OzoneError> {
        let mut config = if path.exists() {
            let content = std::fs::read_to_string(path)
                .map_err(|e| OzoneError::ConfigError(format!("Failed to read config: {}", e)))?;
            toml::from_str(&content)
                .map_err(|e| OzoneError::ConfigError(format!("Failed to parse config: {}", e)))?
        } else {
            // Create default config
            let config = Self::default();
            config.save(path)?;
            config
        };
        config.apply_hardware_region_detection().await;
        Ok(config)
    }

    /// Auto-fills `jurisdiction.instance_region` from real signals — system
    /// timezone, system locale, and IP geolocation (see
    /// `crate::hardware_region`) — when an operator hasn't explicitly set
    /// one in config.toml. This is a deliberate reversal of this field's
    /// original "never guessed, a human sets this explicitly or it stays
    /// None" design — see `JurisdictionConfig`'s doc comment for why, and
    /// for what still makes this safe: an explicit `instance_region` in
    /// config.toml always wins (this never overwrites an operator's real
    /// choice), and this only ever fills the field when at least two of the
    /// (up to three) available real signals agree with each other — see
    /// `HardwareRegionSignals::agreed_region`'s doc comment for the exact
    /// rule. A wrong silent guess is worse than no guess for a value that
    /// can BLOCK requests. Always logged, either way.
    async fn apply_hardware_region_detection(&mut self) {
        if self.jurisdiction.instance_region.is_some() {
            tracing::info!(
                region = ?self.jurisdiction.instance_region,
                "Jurisdiction region: using explicit config.toml value (hardware detection not consulted)"
            );
            return;
        }
        let signals = crate::hardware_region::detect().await;
        match signals.agreed_region() {
            Some(region) => {
                tracing::info!(
                    region = %region,
                    timezone = ?signals.timezone_name,
                    locale = ?signals.locale_raw,
                    ip_country = ?signals.ip_country,
                    "Jurisdiction region: auto-detected (at least two of timezone/locale/IP agreed)"
                );
                self.jurisdiction.instance_region = Some(region);
            }
            None => {
                tracing::warn!(
                    timezone = ?signals.timezone_name,
                    timezone_country = ?signals.timezone_country,
                    locale = ?signals.locale_raw,
                    locale_country = ?signals.locale_country,
                    ip_country = ?signals.ip_country,
                    "Jurisdiction region: fewer than two available signals agreed — leaving \
                     instance_region unset (only Global/U.N.-scope rules apply). Set \
                     [jurisdiction] instance_region explicitly in config.toml to resolve."
                );
            }
        }
    }

    /// Save configuration to a TOML file
    pub fn save(&self, path: &Path) -> Result<(), OzoneError> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| OzoneError::ConfigError(format!("Failed to serialize config: {}", e)))?;
        std::fs::write(path, content)
            .map_err(|e| OzoneError::ConfigError(format!("Failed to write config: {}", e)))?;
        Ok(())
    }
}

/// General configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub data_dir: String,
    pub log_level: String,
    #[serde(default)]
    pub setup_complete: bool,
    #[serde(default)]
    pub user_setup_complete: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            data_dir: "zsei_data".into(),
            log_level: "info".into(),
            setup_complete: false,
            user_setup_complete: false,
        }
    }
}

/// ZSEI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZSEIConfig {
    pub global_path: String,
    pub local_path: String,
    pub cache_path: String,
    pub ml_path: String,
    pub max_containers_in_memory: usize,
    pub mmap_enabled: bool,
    pub embedding_dimension: usize,
    pub pipeline_index_path: String,
    pub methodology_index_path: String,
    pub blueprint_index_path: String,
}

impl Default for ZSEIConfig {
    fn default() -> Self {
        Self {
            global_path: "zsei_data/global.mmap".into(),
            local_path: "zsei_data/local".into(),
            cache_path: "zsei_data/cache".into(),
            ml_path: "zsei_data/ml".into(),
            max_containers_in_memory: 10000,
            mmap_enabled: true,
            embedding_dimension: 384,
            pipeline_index_path: "zsei_data/pipelines/index.json".into(),
            methodology_index_path: "zsei_data/methodologies/index.json".into(),
            blueprint_index_path: "zsei_data/blueprints/index.json".into(),
        }
    }
}

/// Pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub builtin_path: String,
    pub custom_path: String,
    pub max_concurrent_pipelines: usize,
    pub index_path: String,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            builtin_path: "pipelines".into(),
            custom_path: "pipelines/custom".into(),
            max_concurrent_pipelines: 10,
            index_path: "zsei_data/pipelines/index.json".into(),
        }
    }
}

/// Methodology configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodologyConfig {
    pub builtin_path: String,
    pub custom_path: String,
    pub index_path: String,
}

impl Default for MethodologyConfig {
    fn default() -> Self {
        Self {
            builtin_path: "methodologies".into(),
            custom_path: "methodologies/custom".into(),
            index_path: "zsei_data/methodologies/index.json".into(),
        }
    }
}

/// Blueprint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintConfig {
    pub builtin_path: String,
    pub custom_path: String,
    pub index_path: String,
}

impl Default for BlueprintConfig {
    fn default() -> Self {
        Self {
            builtin_path: "blueprints".into(),
            custom_path: "blueprints/custom".into(),
            index_path: "zsei_data/blueprints/index.json".into(),
        }
    }
}

/// Task configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    pub max_queued_tasks: usize,
    pub task_timeout_secs: u64,
    pub preserve_completed_tasks: bool,
    pub max_task_history: usize,
    /// Interval between methodology meta-loop / AMT re-expansion loop runs.
    /// Config-driven so this never needs a rebuild to change again.
    #[serde(default = "default_refinement_interval_secs")]
    pub refinement_interval_secs: u64,
    #[serde(default = "default_refinement_enabled")]
    pub refinement_enabled: bool,
}

fn default_refinement_interval_secs() -> u64 {
    1800 // 30 minutes
}

fn default_refinement_enabled() -> bool {
    true
}

impl Default for TaskConfig {
    fn default() -> Self {
        Self {
            max_queued_tasks: 100,
            task_timeout_secs: 3600, // 1 hour
            preserve_completed_tasks: true,
            max_task_history: 1000,
            refinement_interval_secs: default_refinement_interval_secs(),
            refinement_enabled: default_refinement_enabled(),
        }
    }
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub keystore_path: String,
    pub session_duration_secs: u64,
    pub challenge_expiry_secs: u64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            keystore_path: "zsei_data/keystore".into(),
            session_duration_secs: 86400, // 24 hours
            challenge_expiry_secs: 300,   // 5 minutes
        }
    }
}

/// Integrity configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityConfig {
    pub enabled: bool,
    pub check_interval_secs: u64,
    pub rollback_path: String,
    pub max_versions: u32,
}

impl Default for IntegrityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval_secs: 3600, // 1 hour
            rollback_path: "zsei_data/integrity/rollback".into(),
            max_versions: 100,
        }
    }
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Enable P2P networking
    pub enable_p2p: bool,
    /// Enable cloud sync (separate from P2P)
    pub enable_cloud_sync: bool,
    /// P2P listen port
    pub p2p_port: u16,
    /// Maximum connected peers
    pub max_peers: u32,
    /// Enable mDNS local discovery
    pub enable_mdns: bool,
    /// Sync interval in seconds for batch sync
    pub batch_sync_interval_secs: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            enable_p2p: true,         // Enabled by default for P2P
            enable_cloud_sync: false, // Disabled - local first
            p2p_port: 9090,
            max_peers: 50,
            enable_mdns: true,
            batch_sync_interval_secs: 60,
        }
    }
}

/// gRPC server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcConfig {
    pub address: String,
    pub port: u16,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            address: "127.0.0.1".into(),
            port: 50051,
        }
    }
}

/// UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIConfig {
    pub theme: String,
    pub default_tabs: Vec<String>,
    pub meta_portion_width_percent: u8,
}

impl Default for UIConfig {
    fn default() -> Self {
        Self {
            theme: "home_dashboard".into(),
            default_tabs: vec!["workspace".into(), "library".into(), "settings".into()],
            meta_portion_width_percent: 20,
        }
    }
}

/// Consciousness configuration (enable/disable at runtime via config.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessConfig {
    pub enabled: bool,
    pub emotional_system_enabled: bool,
    pub experience_memory_enabled: bool,
    pub identity_system_enabled: bool,
    pub relationship_system_enabled: bool,
    pub ethical_system_enabled: bool,
    pub collective_enabled: bool,
    pub show_emotional_state: bool,
    pub show_decision_reasoning: bool,
    pub i_loop_interval_ms: u64,
    pub playback_enabled: bool,
}

impl Default for ConsciousnessConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Disabled by default, enable in config.toml
            emotional_system_enabled: true,
            experience_memory_enabled: true,
            identity_system_enabled: true,
            relationship_system_enabled: true,
            ethical_system_enabled: true,
            collective_enabled: false,
            show_emotional_state: true,
            show_decision_reasoning: true,
            i_loop_interval_ms: 60000, // 1 minute
            playback_enabled: true,
        }
    }
}

/// Model configuration for prompt pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model type: "api", "gguf", "onnx"
    pub model_type: String,

    /// For API models
    pub api_endpoint: Option<String>,
    /// Name of an env var holding the key — the value is looked up from the
    /// process environment at request time. Set this OR `api_key` below.
    pub api_key_env: Option<String>,
    /// Raw key value persisted directly to config.toml (gitignored — see
    /// Phase 0). When set, `to_pipeline_env()` exports it under the env var
    /// name `api_key_env` designates, so pipeline 9's own `env::var(...)`
    /// lookup keeps working unchanged either way.
    #[serde(default)]
    pub api_key: Option<String>,
    pub api_model: Option<String>,

    /// For local models (GGUF/ONNX)
    pub local_model_type: Option<String>,
    pub local_model_path: Option<String>,
    pub context_length: usize,
    pub gpu_layers: Option<u32>,

    /// Model selection UI setting
    pub allow_user_selection: bool,
    pub available_models: Vec<AvailableModel>,

    /// Named wire protocol for the prompt pipeline (#9): "anthropic" |
    /// "chat_completions". Unset → endpoint-sniffed (backward compatible).
    #[serde(default)]
    pub wire_protocol: Option<String>,

    /// BitNet tier: path to the llama.cpp-fork CLI (BitNet i2_s kernels)
    /// pipeline #9 spawns when model_type is "bitnet".
    #[serde(default)]
    pub bitnet_cli_path: Option<String>,

    /// User-defined multi-provider fallback chain. When a step's own
    /// model_override fails (or none was set), the orchestrator walks this
    /// list of `AvailableModel.identifier` values in order, trying the next
    /// on failure, rather than giving up or blindly retrying the same
    /// backend. Empty means today's single-backend behavior (no fallback).
    #[serde(default)]
    pub fallback: ModelFallbackConfig,

    /// Fallback chain for "meta work" — drafting reusable methodologies and
    /// blueprints (the knowledge-base layer, not answering the user's
    /// immediate request) — kept separate from `fallback` above because meta
    /// work doesn't need to match whatever model the user picked for
    /// conversation; it can always run on the cheapest/most available
    /// option. Defaults to local+free only (never a paid API), per explicit
    /// preference: BitNet (local) first, then OpenRouter's free router.
    #[serde(default = "default_meta_fallback")]
    pub meta_fallback: ModelFallbackConfig,
}

fn default_meta_fallback() -> ModelFallbackConfig {
    ModelFallbackConfig {
        order: vec!["bitnet-i2_s".to_string(), "openrouter/free".to_string()],
        free_only: true,
    }
}

/// Ordered fallback chain across registered models/providers (company then
/// model, per the user's preference — e.g. try Anthropic's models first,
/// then OpenAI's, then OpenRouter's) plus a free/paid gate.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelFallbackConfig {
    /// Ordered `AvailableModel.identifier` values to try in sequence.
    #[serde(default)]
    pub order: Vec<String>,
    /// When true, the effective chain used at request time is `order`
    /// filtered down to entries with `is_free == true` — set this when the
    /// user has selected free-only, or automatically when no paid provider
    /// has a usable key configured (no funds / no key set).
    #[serde(default)]
    pub free_only: bool,
}

impl ModelConfig {
    /// Export the model settings as the OZONE_MODEL_* / OZONE_WIRE_PROTOCOL
    /// environment variables the prompt pipeline (#9) reads. If `api_key` is
    /// set, its raw value is exported here under the env var name
    /// `api_key_env` designates — otherwise only the NAME crosses and the
    /// value must already be present in the launching shell's environment.
    pub fn to_pipeline_env(&self) -> Vec<(String, String)> {
        let api_key_env_name = self
            .api_key_env
            .clone()
            .unwrap_or_else(|| "ANTHROPIC_API_KEY".into());
        let mut env = vec![
            ("OZONE_MODEL_TYPE".to_string(), self.model_type.clone()),
            ("OZONE_API_KEY_ENV".to_string(), api_key_env_name.clone()),
            ("OZONE_CONTEXT_LENGTH".to_string(), self.context_length.to_string()),
        ];
        if let Some(key) = &self.api_key {
            env.push((api_key_env_name, key.clone()));
        }
        if let Some(e) = &self.api_endpoint {
            env.push(("OZONE_API_ENDPOINT".to_string(), e.clone()));
        }
        if let Some(m) = &self.api_model {
            env.push(("OZONE_API_MODEL".to_string(), m.clone()));
        }
        if let Some(p) = &self.local_model_path {
            env.push(("OZONE_LOCAL_MODEL_PATH".to_string(), p.clone()));
        }
        if let Some(w) = &self.wire_protocol {
            env.push(("OZONE_WIRE_PROTOCOL".to_string(), w.clone()));
        }
        if let Some(c) = &self.bitnet_cli_path {
            env.push(("BITNET_CLI_PATH".to_string(), c.clone()));
        }
        env
    }
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            model_type: "api".into(),
            api_endpoint: Some("https://api.anthropic.com/v1/messages".into()),
            api_key_env: Some("ANTHROPIC_API_KEY".into()),
            api_key: None,
            api_model: Some("claude-sonnet-4-20250514".into()),
            local_model_type: None,
            local_model_path: None,
            context_length: 8192, // Default, overridden by per-model setting
            gpu_layers: None,
            allow_user_selection: true,
            available_models: vec![
                AvailableModel {
                    name: "Claude Sonnet (API)".into(),
                    model_type: "api".into(),
                    identifier: "claude-sonnet-4-20250514".into(),
                    context_length: 200000,
                    api_endpoint: None,
                    api_key_env: None,
                    api_key: None,
                    wire_protocol: None,
                    bitnet_cli_path: None,
                    local_model_path: None,
                    gpu_layers: None,
                    provider: "anthropic".into(),
                    is_free: false,
                },
                // Local models are added by user via UI or config
            ],
            wire_protocol: None,
            bitnet_cli_path: None,
            fallback: ModelFallbackConfig::default(),
            meta_fallback: default_meta_fallback(),
        }
    }
}

/// Available model configuration — a named, independently-dispatchable
/// model profile. Connection fields mirror ModelConfig's overridable subset;
/// all optional/`#[serde(default)]` so the one pre-existing default entry
/// (name+type+identifier+context_length only) keeps deserializing fine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableModel {
    pub name: String,
    pub model_type: String,
    pub identifier: String,
    /// Model-specific context length (overrides global setting when this model is active)
    #[serde(default = "default_context_length")]
    pub context_length: usize,
    #[serde(default)]
    pub api_endpoint: Option<String>,
    #[serde(default)]
    pub api_key_env: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub wire_protocol: Option<String>,
    #[serde(default)]
    pub bitnet_cli_path: Option<String>,
    #[serde(default)]
    pub local_model_path: Option<String>,
    #[serde(default)]
    pub gpu_layers: Option<u32>,
    /// Company/provider this model belongs to (e.g. "anthropic", "openai",
    /// "z.ai", "openrouter", "bitnet") — groups entries for the user-defined
    /// fallback order (ModelConfig::fallback) and for company-then-model
    /// selection UI. Free-text, not an enum, since providers are added via
    /// config, not compiled in.
    #[serde(default)]
    pub provider: String,
    /// True for entries known to cost nothing per token (e.g. a local model,
    /// or an OpenRouter entry pinned to a ":free"-suffixed model / the
    /// "openrouter/free" router). Drives ModelConfig::fallback.free_only
    /// filtering — OpenRouter's free lineup rotates over time, so this is a
    /// per-entry flag the user (or a future live /models sync) sets, not a
    /// hardcoded model list.
    #[serde(default)]
    pub is_free: bool,
}

fn default_context_length() -> usize {
    8192
}

/// Voice configuration for speech input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    pub enabled: bool,

    /// Voice backend: "whisper_rs" (integrated), "whisper_cpp" (standalone), "api"
    pub backend: String,

    /// Path to whisper model file (for whisper_rs or whisper_cpp)
    pub whisper_model_path: Option<String>,

    /// Path to whisper-cli binary (for whisper_cpp backend)
    pub whisper_cpp_path: Option<String>,

    /// API endpoint for voice transcription (if using API backend)
    pub api_endpoint: Option<String>,

    /// API key environment variable (if using API backend)
    pub api_key_env: Option<String>,

    /// Raw key value persisted directly to config.toml (gitignored). Same
    /// convention as ModelConfig.api_key — preferred over api_key_env when set.
    #[serde(default)]
    pub api_key: Option<String>,

    /// Transcription language hint (e.g. "en"). The shipped base.en model is
    /// English-only — set this to avoid auto-detect misfires. Forwarded to
    /// the voice pipeline as OZONE_VOICE_LANGUAGE.
    #[serde(default)]
    pub language: Option<String>,

    /// ffmpeg binary for transcoding compressed captures (webm/opus from
    /// MediaRecorder) into the 16 kHz mono WAV whisper requires. Forwarded
    /// as OZONE_VOICE_FFMPEG.
    #[serde(default = "default_ffmpeg_path")]
    pub ffmpeg_path: String,
}

fn default_ffmpeg_path() -> String {
    "ffmpeg".into()
}

impl VoiceConfig {
    /// Export the voice settings as the OZONE_VOICE_* environment variables
    /// the voice pipeline (#10) reads. Called at bootstrap and whenever the
    /// gRPC voice-settings update lands — spawned pipeline children inherit
    /// the process environment, so no per-pipeline host code is needed.
    pub fn to_pipeline_env(&self) -> Vec<(String, String)> {
        let mut env = vec![
            (
                "OZONE_VOICE_BACKEND".to_string(),
                self.backend.clone(),
            ),
            (
                "OZONE_VOICE_FFMPEG".to_string(),
                self.ffmpeg_path.clone(),
            ),
        ];
        if let Some(p) = &self.whisper_model_path {
            env.push(("OZONE_VOICE_MODEL_PATH".to_string(), p.clone()));
        }
        if let Some(p) = &self.whisper_cpp_path {
            env.push(("OZONE_VOICE_CPP_PATH".to_string(), p.clone()));
        }
        if let Some(e) = &self.api_endpoint {
            env.push(("OZONE_VOICE_API_ENDPOINT".to_string(), e.clone()));
        }
        if let Some(key) = &self.api_key {
            env.push(("OZONE_VOICE_API_KEY".to_string(), key.clone()));
        } else if let Some(k) = &self.api_key_env {
            if let Ok(key) = std::env::var(k) {
                env.push(("OZONE_VOICE_API_KEY".to_string(), key));
            }
        }
        if let Some(l) = &self.language {
            env.push(("OZONE_VOICE_LANGUAGE".to_string(), l.clone()));
        }
        env
    }
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            backend: "whisper_rs".into(), // Integrated by default
            whisper_model_path: None,     // User sets via UI
            whisper_cpp_path: Some("/usr/local/bin/whisper-cli".into()),
            api_endpoint: None,
            api_key_env: None,
            api_key: None,
            language: None,
            ffmpeg_path: default_ffmpeg_path(),
        }
    }
}
