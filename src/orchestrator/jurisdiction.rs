//! Jurisdiction-aware guardrail mechanism.
//!
//! IMPORTANT — READ BEFORE ADDING CONTENT HERE OR ANYWHERE THIS MODULE READS
//! FROM: this file contains NO real legal content, from any jurisdiction,
//! anywhere. It is the enforcement MECHANISM only — a place for a human to
//! later load real, sourced, authoritative legal rules (via a
//! JurisdictionRuleSet container in ZSEI; see ContainerType::JurisdictionRuleSet
//! in src/types/container.rs). An LLM's best guess at what a statute says is
//! not verified law; shipping fabricated "compliance" rules here would create
//! false confidence in a system that may be used in contexts (e.g. services
//! for minors) where getting this wrong has real consequences. If you are
//! extending this file, do not hardcode any specific real-world legal claim —
//! extend the mechanism, and load real content from ZSEI at runtime instead.
//!
//! This gate runs unconditionally on every orchestration request, regardless
//! of `consciousness_enabled` — it is a base safety layer, not part of the
//! optional consciousness system (see stages.rs's call site, inserted
//! alongside but independent of the Consciousness Gate stage).

use super::*;

/// Where a rule applies. Global = worldwide (e.g. a real U.N.-level rule, if
/// one is ever loaded); National/Local are free-form region labels matched
/// against `JurisdictionConfig.instance_region` (e.g. "US", "US-CA") — not
/// validated against any real list of countries/states here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum JurisdictionScope {
    Global,
    National(String),
    Local(String),
}

/// What happens when a rule's condition matches. Deliberately generic —
/// real severity/consequence judgments belong in the rule content itself
/// (which, per this module's own rule, doesn't exist yet).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RuleAction {
    Block,
    RequireConfirmation,
    Warn,
    Log,
}

/// One jurisdiction rule. `condition` is a plain keyword/phrase matched
/// case-insensitively against the request's cleaned prompt and extracted
/// keywords — intentionally simple (no fabricated NLP-legal-reasoning
/// pipeline) since there is no real rule content to evaluate yet; a richer
/// matcher can be built once real content exists and its actual matching
/// needs are known. `source` is a citation/reference field, always empty
/// until a human fills it in with a real, checkable legal source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionRule {
    pub scope: JurisdictionScope,
    pub condition: String,
    pub action: RuleAction,
    /// Citation/reference to the real legal source this rule implements.
    /// Never left as a fabricated citation — empty until a human sets it.
    #[serde(default)]
    pub source: String,
    /// The actual text a real source returned (a search result title +
    /// snippet, quoted/closely paraphrased — see jurisdiction_search.rs),
    /// never elaborated on or filled in from general knowledge beyond what
    /// was actually retrieved. `None` for hand-curated rules (e.g. the
    /// Global UDHR/CRC set) that cite an article directly instead.
    #[serde(default)]
    pub retrieved_snippet: Option<String>,
    /// Whether `source` looked like an official domain (.gov/.gouv/etc) at
    /// retrieval time, vs. some other site — a real, checkable signal, not
    /// a fabricated confidence score. `None` when not applicable (e.g.
    /// hand-curated rules).
    #[serde(default)]
    pub official_source: Option<bool>,
}

/// Result of evaluating all loaded rules against one request.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JurisdictionGateResult {
    pub rules_loaded: usize,
    pub matched: Vec<(JurisdictionRule, RuleAction)>,
    pub blocked: bool,
}

/// On-disk shape for a jurisdiction content file (referenced by a
/// JurisdictionRuleSet container's object_store_path). `disclaimer` is
/// mandatory content, not decoration — every real content file must state,
/// prominently and in its own words, that this is an engineering starting
/// point drafted from cited sources and NOT verified legal compliance, NOT a
/// substitute for qualified legal counsel. Deserialization fails (rules load
/// as empty, never silently substituting a missing disclaimer) if this field
/// is absent, by design — see load_jurisdiction_rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionContentFile {
    pub disclaimer: String,
    pub rules: Vec<JurisdictionRule>,
}

impl PromptOrchestrator {
    /// Load real jurisdiction rules from ZSEI for the current instance's
    /// configured region plus Global scope. Returns an empty Vec (not an
    /// error) when nothing has been loaded — that is the expected, honest
    /// starting state for every instance until a human loads real content.
    async fn load_jurisdiction_rules(&self, instance_region: Option<&str>) -> Vec<JurisdictionRule> {
        let mut scopes_to_try: Vec<String> = vec!["global".to_string()];
        if let Some(region) = instance_region {
            scopes_to_try.push(region.to_lowercase());
            // Also try the national prefix of a "US-CA"-style region label.
            if let Some((national, _)) = region.split_once('-') {
                scopes_to_try.push(national.to_lowercase());
            }
        }

        let ids = self
            .store
            .search_by_keywords(&scopes_to_try, Some("JurisdictionRuleSet"))
            .await
            .unwrap_or_default();

        let mut rules = Vec::new();
        for id in ids {
            if let Ok(Some(container)) = self.store.get_container(id).await {
                if let Some(object_store_path) = container
                    .get("local_state")
                    .and_then(|ls| ls.get("storage"))
                    .and_then(|s| s.get("object_store_path"))
                    .and_then(|p| p.as_str())
                {
                    let data_dir = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
                    let full_path = format!("{}/{}", data_dir, object_store_path);
                    if let Ok(content) = std::fs::read_to_string(&full_path) {
                        // Requires the disclaimer field (JurisdictionContentFile,
                        // not a bare array) — a content file missing it fails to
                        // parse and contributes zero rules rather than silently
                        // loading undisclaimed content. This is deliberate: the
                        // disclaimer is mandatory content, not decoration.
                        match serde_json::from_str::<JurisdictionContentFile>(&content) {
                            Ok(parsed) if !parsed.disclaimer.trim().is_empty() => {
                                rules.extend(parsed.rules);
                            }
                            Ok(_) => {
                                tracing::warn!(
                                    path = %full_path,
                                    "Jurisdiction content file has an empty disclaimer — refusing to load its rules"
                                );
                            }
                            Err(e) => {
                                tracing::warn!(path = %full_path, error = %e, "Failed to parse jurisdiction content file");
                            }
                        }
                    }
                }
            }
        }
        rules
    }

    /// STAGE: Jurisdiction Gate — always runs, independent of
    /// consciousness_enabled. With zero rules loaded (the honest default),
    /// this is a real no-op that is still visible in the stage log rather
    /// than silently doing nothing.
    pub(crate) async fn stage_jurisdiction_gate(&self, state: &mut OrchestrationState) -> Result<(), String> {
        // Global (U.N.-level) scope is a hard invariant, not something any
        // config flag can turn off — per explicit direction: "U.N. must
        // always be applied whether location is present or not." Only the
        // National/Local (region-specific) layer is gated by `enabled`, and
        // only ever considered when a region is actually set —
        // `instance_region` itself is auto-filled from real hardware
        // signals at config load when an operator hasn't set one
        // explicitly (see OzoneConfig::apply_hardware_region_detection);
        // this stage just reads whatever ended up there. It stays None
        // (so this layer is skipped) when hardware detection couldn't
        // reach a confident answer — never a fabricated guess. `enabled=false` therefore means
        // "no region-specific enforcement yet", never "no jurisdiction
        // enforcement at all".
        let region = if self.jurisdiction_config.enabled {
            self.jurisdiction_config.instance_region.as_deref()
        } else {
            None
        };
        let rules = self.load_jurisdiction_rules(region).await;

        // If a real region is configured/detected but nothing region-specific
        // has been loaded yet (only Global-scope rules, if any), kick off a
        // real, search-backed attempt to populate it — see
        // jurisdiction_search.rs for the honesty contract (real web search
        // only, never LLM-generated legal text, Log-only action). Spawned
        // in the background so this never adds search latency to the live
        // request that happened to be the first one to see this region.
        if let Some(region) = region {
            let has_regional = rules.iter().any(|r| {
                matches!(&r.scope, JurisdictionScope::National(n) | JurisdictionScope::Local(n) if n.eq_ignore_ascii_case(region))
            });
            if !has_regional {
                let executor = self.executor.clone();
                let store = self.store.clone();
                let region_owned = region.to_string();
                tokio::spawn(async move {
                    crate::orchestrator::jurisdiction_search::populate_jurisdiction_content_for_region(
                        executor, store, region_owned,
                    )
                    .await;
                });
            }
        }

        // Runs at Stage 1, before prompt_normalization — state.cleaned_prompt
        // /keywords aren't populated yet, so this matches the raw request
        // prompt directly (always available immediately, and lets a Block
        // action stop the request before any real processing happens).
        let haystack = state.request.prompt.to_lowercase();

        let mut result = JurisdictionGateResult {
            rules_loaded: rules.len(),
            ..Default::default()
        };

        for rule in &rules {
            if !rule.condition.is_empty() && haystack.contains(&rule.condition.to_lowercase()) {
                if rule.action == RuleAction::Block {
                    result.blocked = true;
                }
                result.matched.push((rule.clone(), rule.action.clone()));
            }
        }

        let blocked = result.blocked;
        state.jurisdiction_gate_result = Some(result.clone());

        self.record_stage(
            state,
            0,
            "Jurisdiction Gate",
            true,
            &format!(
                "region={:?}, rules_loaded={}, matched={}, blocked={}",
                region,
                result.rules_loaded,
                result.matched.len(),
                blocked
            ),
        );

        if blocked {
            return Err(
                "Request blocked by a jurisdiction rule (see jurisdiction_gate_result for which one)"
                    .to_string(),
            );
        }

        Ok(())
    }
}
