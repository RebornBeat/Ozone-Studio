//! Per-instance jurisdiction POLICY surface.
//!
//! Hardening is not one fixed restriction. Every instance has its own agreements and requirements, so every enforcement point reads
//! THIS policy (durable, versioned, visible in the UI, changeable by the operator, shareable as a staged jurisdiction content file) instead
//! of hard-coded rules. The shipped defaults (`assets/jurisdiction/policy.default.json`, ON, operator opt-out) are content, not code: this
//! module contains no legal claim, only mechanism.
//!
//! Files under the data dir (`OZONE_ZSEI_DATA_DIR`, else `zsei_data`, like every other host-side reader):
//!   * `jurisdiction_policy.json`           — the live policy (latest version)
//!   * `jurisdiction_policy_history.jsonl`  — every saved version, append-only, never rotated
//!   * `jurisdiction_agreements.json`       — per-requirement acceptance (re-asked when a requirement's version/text changes)
//!   * `jurisdiction_outbound_approvals.jsonl` — operator approvals of outbound transfers (append-only)
//!   * `jurisdiction_decisions.jsonl`       — append-only decision log: entry point, hashes, rule conditions, policy/rules version, outcome.
//!                                            NEVER holds request content, file text or personal data — only hashes and rule vocabulary.
//!
//! Doctrine: no confidence scores (the optional personal-data judgment is a yes/no strength-N vote), no fixed caps, never fail open
//! (an outbound call that cannot be cleared is held as an awaiting item and retried by the existing pause/resume machinery), no skip.

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::sync::OnceLock;

pub const POLICY_FILE: &str = "jurisdiction_policy.json";
pub const POLICY_HISTORY: &str = "jurisdiction_policy_history.jsonl";
pub const AGREEMENTS_FILE: &str = "jurisdiction_agreements.json";
pub const APPROVALS_LOG: &str = "jurisdiction_outbound_approvals.jsonl";
pub const DECISION_LOG: &str = "jurisdiction_decisions.jsonl";

pub const KIND_OUTBOUND_TRANSFER: &str = "outbound_transfer";
pub const KIND_AGREEMENT: &str = "jurisdiction_agreement";
/// Prefix of the `Err` a held call returns, so callers (and the pause/resume machinery) recognise a HOLD, not a failure to fix.
pub const AWAITING_PREFIX: &str = "AWAITING_OPERATOR:";

tokio::task_local! {
    /// True inside a call that has ALREADY passed this gate (the adapter scopes its registry call with it) or that is the gate's own
    /// personal-data vote. A task-local, never an input key: an external caller cannot set it by putting a field in its JSON.
    static PASSED: bool;
}

/// Run `fut` marked as already gate-cleared (see `PASSED`).
pub async fn scoped_passed<F: std::future::Future>(fut: F) -> F::Output {
    PASSED.scope(true, fut).await
}

/// Whether the current call chain already passed the outbound gate.
pub fn passed() -> bool {
    PASSED.try_with(|v| *v).unwrap_or(false)
}

/// Convert a pipeline input map to JSON for scanning/classification. `Bytes` are not text and are skipped (never base64-expanded).
pub fn input_to_json(data: &std::collections::HashMap<String, crate::types::Value>) -> serde_json::Value {
    use crate::types::Value as V;
    fn conv(v: &V) -> serde_json::Value {
        match v {
            V::Null => serde_json::Value::Null,
            V::Bool(b) => serde_json::json!(b),
            V::Int(i) => serde_json::json!(i),
            V::Float(f) => serde_json::json!(f),
            V::String(s) => serde_json::Value::String(s.clone()),
            V::Bytes(_) => serde_json::Value::Null,
            V::Array(a) => serde_json::Value::Array(a.iter().map(conv).collect()),
            V::Map(m) => serde_json::Value::Object(m.iter().map(|(k, x)| (k.clone(), conv(x))).collect()),
        }
    }
    serde_json::Value::Object(data.iter().map(|(k, v)| (k.clone(), conv(v))).collect())
}

pub fn data_dir() -> String {
    std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string())
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn hash_hex(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

fn t() -> bool {
    true
}

// ============================================================================
// POLICY
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderRule {
    /// Substring of the model endpoint (host) this rule classifies.
    pub endpoint_contains: String,
    /// Region label: `local`, `eea`, `third_country_verified`, `third_country_unverified` — or any label the operator defines.
    pub region: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Requirement {
    pub key: String,
    pub title: String,
    pub text: String,
    #[serde(default)]
    pub source: String,
    /// Bump to re-ask the operator after the requirement changed.
    #[serde(default)]
    pub version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetectionPolicy {
    #[serde(default = "t")]
    pub deterministic: bool,
    #[serde(default)]
    pub kinds: Vec<String>,
    /// Yes/no strength-N vote on the outbound text ("does it contain personal data?"). It sends that text to a model, so it only runs
    /// on a model whose region the policy allows: `vote_model` (an identifier the operator names) in `vote_model_region`.
    #[serde(default = "t")]
    pub vote: bool,
    #[serde(default)]
    pub vote_model: String,
    #[serde(default)]
    pub vote_model_region: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ErasurePolicy {
    #[serde(default = "t")]
    pub enabled: bool,
    /// A before-copy of an erased file would keep the personal data, so it is OFF unless the operator turns it on.
    #[serde(default)]
    pub keep_before_copy: bool,
    /// Operator-set retention horizon for personal data in logs; `None` = never rotated (doctrine).
    #[serde(default)]
    pub retention_days: Option<u64>,
    #[serde(default)]
    pub covers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct P2pPolicy {
    #[serde(default)]
    pub publish_kinds: Vec<String>,
    #[serde(default = "t")]
    pub recall_notice: bool,
    #[serde(default = "t")]
    pub share_policy_as_offer: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JurisdictionPolicy {
    #[serde(default)]
    pub disclaimer: String,
    #[serde(default)]
    pub version: u64,
    #[serde(default)]
    pub updated_at: u64,
    #[serde(default)]
    pub updated_by: String,
    /// Where this version came from: "defaults", "operator", or "adopted:<staged file>".
    #[serde(default)]
    pub origin: String,
    #[serde(default = "t")]
    pub enabled: bool,
    #[serde(default)]
    pub hosting_region: String,
    /// Free text residency note the operator keeps with the policy (provider, datacentre, DPA reference...).
    #[serde(default)]
    pub residency_note: String,
    #[serde(default)]
    pub providers: Vec<ProviderRule>,
    #[serde(default)]
    pub default_region_unlisted_endpoint: String,
    #[serde(default)]
    pub allow_personal_data_regions: Vec<String>,
    /// `await_operator` (default) | `block` | `allow_logged`.
    #[serde(default)]
    pub on_personal_data_outbound: String,
    /// What the gate does when no region is configured or detected: `global_only_flagged` (default) | `await_operator` | `block`.
    #[serde(default)]
    pub unknown_jurisdiction: String,
    /// `strictest` (default): every applicable rule acts and the strictest outcome wins. The instance may set another label; only
    /// `strictest` is implemented, anything else is treated as `strictest` and recorded as such.
    #[serde(default)]
    pub conflict_resolution: String,
    #[serde(default = "t")]
    pub require_agreements_before_outbound: bool,
    #[serde(default)]
    pub requirements: Vec<Requirement>,
    #[serde(default = "default_detection")]
    pub detection: DetectionPolicy,
    #[serde(default = "default_erasure")]
    pub erasure: ErasurePolicy,
    #[serde(default = "default_p2p")]
    pub p2p: P2pPolicy,
    /// Pipeline ids the operator exempts from the outbound content check (e.g. a pipeline known never to leave the host).
    #[serde(default)]
    pub exempt_pipelines: Vec<u64>,
}

fn default_detection() -> DetectionPolicy {
    DetectionPolicy {
        deterministic: true,
        kinds: vec![
            "email".into(),
            "phone".into(),
            "iban".into(),
            "card_number".into(),
        ],
        vote: true,
        vote_model: String::new(),
        vote_model_region: String::new(),
    }
}
fn default_erasure() -> ErasurePolicy {
    ErasurePolicy {
        enabled: true,
        keep_before_copy: false,
        retention_days: None,
        covers: Vec::new(),
    }
}
fn default_p2p() -> P2pPolicy {
    P2pPolicy {
        publish_kinds: Vec::new(),
        recall_notice: true,
        share_policy_as_offer: true,
    }
}

/// The shipped defaults (content file, embedded at compile time so a missing data-dir copy never leaves an instance without a policy).
pub fn shipped_defaults() -> JurisdictionPolicy {
    const RAW: &str = include_str!("../../../assets/jurisdiction/policy.default.json");
    let mut p: JurisdictionPolicy = serde_json::from_str(RAW).unwrap_or_else(|e| {
        // A broken shipped file must be loud, never an empty (open) policy: fall back to the strictest mechanism defaults.
        tracing::error!("jurisdiction: shipped policy.default.json does not parse ({e}) — using the minimal strict defaults");
        JurisdictionPolicy {
            disclaimer: "minimal strict defaults (shipped policy file unreadable)".into(),
            version: 0,
            updated_at: 0,
            updated_by: String::new(),
            origin: "defaults".into(),
            enabled: true,
            hosting_region: "unspecified".into(),
            residency_note: String::new(),
            providers: vec![ProviderRule { endpoint_contains: "localhost".into(), region: "local".into(), note: String::new() }],
            default_region_unlisted_endpoint: "third_country_unverified".into(),
            allow_personal_data_regions: vec!["local".into(), "eea".into()],
            on_personal_data_outbound: "await_operator".into(),
            unknown_jurisdiction: "global_only_flagged".into(),
            conflict_resolution: "strictest".into(),
            require_agreements_before_outbound: true,
            requirements: Vec::new(),
            detection: DetectionPolicy {
                deterministic: true,
                kinds: vec!["email".into(), "phone".into(), "iban".into(), "card_number".into()],
                vote: true,
                vote_model: String::new(),
                vote_model_region: String::new(),
            },
            erasure: default_erasure(),
            p2p: default_p2p(),
            exempt_pipelines: Vec::new(),
        }
    });
    p.origin = "defaults".to_string();
    p
}

fn path(name: &str) -> std::path::PathBuf {
    std::path::Path::new(&data_dir()).join(name)
}

/// The live policy: the saved file when present and valid, else the shipped defaults. An unreadable/corrupt saved file is NOT treated as
/// "no policy": it is logged loudly and the shipped defaults apply (which are ON), never an open gate.
pub fn load_policy() -> JurisdictionPolicy {
    // Cached by file mtime: the gate runs on EVERY pipeline call, so the file is re-read only when it changed on disk.
    static CACHE: OnceLock<std::sync::Mutex<Option<(Option<std::time::SystemTime>, JurisdictionPolicy)>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(None));
    let mtime = std::fs::metadata(path(POLICY_FILE)).and_then(|m| m.modified()).ok();
    {
        let g = cache.lock().unwrap_or_else(|p| p.into_inner());
        if let Some((m, p)) = g.as_ref() {
            if *m == mtime {
                return p.clone();
            }
        }
    }
    let fresh = load_policy_uncached();
    *cache.lock().unwrap_or_else(|p| p.into_inner()) = Some((mtime, fresh.clone()));
    fresh
}

fn load_policy_uncached() -> JurisdictionPolicy {
    match std::fs::read_to_string(path(POLICY_FILE)) {
        Ok(text) => match serde_json::from_str::<JurisdictionPolicy>(&text) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("jurisdiction: {POLICY_FILE} does not parse ({e}) — shipped defaults apply until the operator fixes it");
                shipped_defaults()
            }
        },
        Err(_) => shipped_defaults(),
    }
}

/// Save `next` as the next version: atomic write of the live file, then an append-only history row. Returns the saved policy.
pub fn save_policy(
    mut next: JurisdictionPolicy,
    by: &str,
    origin: &str,
) -> Result<JurisdictionPolicy, String> {
    let current = load_policy();
    next.version = current.version + 1;
    next.updated_at = now_ms();
    next.updated_by = by.to_string();
    next.origin = origin.to_string();
    let dir = data_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("data dir: {e}"))?;
    let body = serde_json::to_string_pretty(&next).map_err(|e| e.to_string())?;
    let live = path(POLICY_FILE);
    let tmp = path(&format!("{POLICY_FILE}.tmp"));
    std::fs::write(&tmp, &body).map_err(|e| format!("write policy: {e}"))?;
    std::fs::rename(&tmp, &live).map_err(|e| format!("replace policy: {e}"))?;
    let line = serde_json::to_string(&next).map_err(|e| e.to_string())?;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path(POLICY_HISTORY))
        .map_err(|e| format!("policy history: {e}"))?;
    writeln!(f, "{line}").map_err(|e| format!("policy history: {e}"))?;
    Ok(next)
}

/// Content hash of the live policy (the value written into decision rows so an outcome is tied to the exact policy that produced it).
pub fn policy_fingerprint(p: &JurisdictionPolicy) -> String {
    let mut canon = p.clone();
    canon.updated_at = 0;
    hash_hex(serde_json::to_string(&canon).unwrap_or_default().as_bytes())[..16].to_string()
}

// ============================================================================
// AGREEMENTS (per requirement; re-asked when its version or text changes)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Agreement {
    pub key: String,
    pub version: u64,
    pub text_hash: String,
    pub accepted: bool,
    pub by: String,
    pub at: u64,
}

fn requirement_hash(r: &Requirement) -> String {
    hash_hex(format!("{}\n{}\n{}", r.key, r.version, r.text).as_bytes())[..16].to_string()
}

pub fn load_agreements() -> Vec<Agreement> {
    std::fs::read_to_string(path(AGREEMENTS_FILE))
        .ok()
        .and_then(|t| serde_json::from_str::<Vec<Agreement>>(&t).ok())
        .unwrap_or_default()
}

/// Record acceptance (or withdrawal) of ONE requirement at its CURRENT version/text. The file keeps the latest row per key; every change
/// is also appended to the decision log so acceptance history is auditable.
pub fn record_agreement(key: &str, accepted: bool, by: &str) -> Result<Agreement, String> {
    let policy = load_policy();
    let req = policy
        .requirements
        .iter()
        .find(|r| r.key == key)
        .ok_or_else(|| format!("no requirement '{key}' in policy v{}", policy.version))?
        .clone();
    let row = Agreement {
        key: key.to_string(),
        version: req.version,
        text_hash: requirement_hash(&req),
        accepted,
        by: by.to_string(),
        at: now_ms(),
    };
    let mut all = load_agreements();
    all.retain(|a| a.key != key);
    all.push(row.clone());
    let body = serde_json::to_string_pretty(&all).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(data_dir()).map_err(|e| e.to_string())?;
    let tmp = path(&format!("{AGREEMENTS_FILE}.tmp"));
    std::fs::write(&tmp, body).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path(AGREEMENTS_FILE)).map_err(|e| e.to_string())?;
    record_decision(
        "agreement",
        &row.text_hash,
        None,
        &policy,
        "",
        if accepted { "accepted" } else { "withdrawn" },
        &[key.to_string()],
        &format!("requirement '{key}' v{} by {by}", req.version),
    );
    Ok(row)
}

/// One row per requirement with whether the operator has accepted its CURRENT version and text.
pub fn agreement_status(policy: &JurisdictionPolicy) -> Vec<serde_json::Value> {
    let all = load_agreements();
    policy
        .requirements
        .iter()
        .map(|r| {
            let h = requirement_hash(r);
            let row = all.iter().find(|a| a.key == r.key);
            let current = row.map(|a| a.accepted && a.text_hash == h).unwrap_or(false);
            serde_json::json!({
                "key": r.key, "title": r.title, "text": r.text, "source": r.source, "version": r.version,
                "accepted_current": current,
                "accepted_version": row.map(|a| a.version),
                "stale": row.map(|a| a.accepted && a.text_hash != h).unwrap_or(false),
                "by": row.map(|a| a.by.clone()), "at": row.map(|a| a.at),
            })
        })
        .collect()
}

/// Keys of requirements whose CURRENT version is not accepted.
pub fn unaccepted_requirements(policy: &JurisdictionPolicy) -> Vec<String> {
    agreement_status(policy)
        .into_iter()
        .filter(|v| !v["accepted_current"].as_bool().unwrap_or(false))
        .filter_map(|v| v["key"].as_str().map(str::to_string))
        .collect()
}

// ============================================================================
// DECISION LOG (append-only, never rotated, no personal content)
// ============================================================================

/// Fingerprint of the rule set that was applied (conditions + actions + sources, order-independent).
pub fn rules_fingerprint(rules: &[super::JurisdictionRule]) -> String {
    let mut lines: Vec<String> = rules
        .iter()
        .map(|r| format!("{:?}|{}|{:?}|{}", r.scope, r.condition, r.action, r.source))
        .collect();
    lines.sort();
    hash_hex(lines.join("\n").as_bytes())[..16].to_string()
}

/// Append one decision row. `subject_hash` is a hash of the checked text (never the text). `matched` holds rule CONDITIONS or detector KINDS
/// (rule vocabulary), never content. Failure to write is loud and returned to no one — the check itself already acted; the row is retried
/// by the caller-independent append below only once, because the log is the audit trail and must not silently lose rows: on error the row is
/// kept in a process-local queue and flushed before the next append.
#[allow(clippy::too_many_arguments)]
pub fn record_decision(
    entry_point: &str,
    subject_hash: &str,
    region: Option<&str>,
    policy: &JurisdictionPolicy,
    rules_fp: &str,
    outcome: &str,
    matched: &[String],
    detail: &str,
) {
    let row = serde_json::json!({
        "at": now_ms(),
        "entry_point": entry_point,
        "subject_hash": subject_hash,
        "region": region,
        "region_known": region.is_some(),
        "policy_version": policy.version,
        "policy_fp": policy_fingerprint(policy),
        "rules_fp": rules_fp,
        "outcome": outcome,
        "matched": matched,
        "detail": detail,
    });
    pending_rows()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .push(row.to_string());
    flush_pending();
}

fn pending_rows() -> &'static std::sync::Mutex<Vec<String>> {
    static Q: OnceLock<std::sync::Mutex<Vec<String>>> = OnceLock::new();
    Q.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

/// Drain queued rows in order into the append-only log. A row that cannot be written stays queued (and the failure is logged) until a later
/// append succeeds — rows are never dropped.
pub fn flush_pending() {
    let mut q = pending_rows().lock().unwrap_or_else(|p| p.into_inner());
    if q.is_empty() {
        return;
    }
    let _ = std::fs::create_dir_all(data_dir());
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path(DECISION_LOG));
    match file {
        Ok(mut f) => {
            let mut written = 0usize;
            for row in q.iter() {
                if writeln!(f, "{row}").is_err() {
                    break;
                }
                written += 1;
            }
            q.drain(..written);
            if !q.is_empty() {
                tracing::error!("jurisdiction decision log: {} row(s) still queued after a failed write — retried on the next append", q.len());
            }
        }
        Err(e) => tracing::error!("jurisdiction decision log: cannot open {DECISION_LOG} ({e}) — {} row(s) queued, retried on the next append", q.len()),
    }
}

/// Read the decision log (all rows, oldest first). `since_ms` filters by time; no row cap (the caller chunks).
pub fn read_decisions(since_ms: u64) -> Vec<serde_json::Value> {
    std::fs::read_to_string(path(DECISION_LOG))
        .map(|t| {
            t.lines()
                .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
                .filter(|v| v["at"].as_u64().unwrap_or(0) >= since_ms)
                .collect()
        })
        .unwrap_or_default()
}

// ============================================================================
// OUTBOUND CONTENT GATE
// ============================================================================

#[derive(Debug, Clone, Default, Serialize)]
pub struct OutboundVerdict {
    pub allow: bool,
    pub region: String,
    pub kinds: Vec<String>,
    pub reason: String,
    pub await_id: Option<String>,
}

/// Classify a model endpoint into a policy region. `None` endpoint = the instance's configured default (env `OZONE_API_ENDPOINT`);
/// no endpoint anywhere and a `local`/`bitnet`/`llama` model type = `local`. An endpoint no provider rule matches takes the policy's
/// `default_region_unlisted_endpoint` (never silently "allowed").
pub fn classify_destination(policy: &JurisdictionPolicy, input: &serde_json::Value) -> String {
    let over = input.get("model_override_config");
    let endpoint = over
        .and_then(|o| o.get("api_endpoint"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .or_else(|| std::env::var("OZONE_API_ENDPOINT").ok())
        .filter(|s| !s.is_empty());
    let mtype = over
        .and_then(|o| o.get("model_type"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .or_else(|| std::env::var("OZONE_MODEL_TYPE").ok())
        .unwrap_or_default()
        .to_lowercase();
    match endpoint {
        Some(ep) => {
            let lower = ep.to_lowercase();
            policy
                .providers
                .iter()
                .find(|p| {
                    !p.endpoint_contains.is_empty()
                        && lower.contains(&p.endpoint_contains.to_lowercase())
                })
                .map(|p| p.region.clone())
                .unwrap_or_else(|| {
                    if policy.default_region_unlisted_endpoint.is_empty() {
                        "third_country_unverified".to_string()
                    } else {
                        policy.default_region_unlisted_endpoint.clone()
                    }
                })
        }
        None => {
            if matches!(
                mtype.as_str(),
                "local" | "bitnet" | "llama" | "llama_cpp" | "gguf"
            ) {
                "local".to_string()
            } else if mtype.is_empty() {
                // Nothing says where the call goes — never assume local.
                if policy.default_region_unlisted_endpoint.is_empty() {
                    "third_country_unverified".to_string()
                } else {
                    policy.default_region_unlisted_endpoint.clone()
                }
            } else {
                policy.default_region_unlisted_endpoint.clone()
            }
        }
    }
}

fn collect_strings(v: &serde_json::Value, out: &mut String) {
    match v {
        serde_json::Value::String(s) => {
            out.push_str(s);
            out.push('\n');
        }
        serde_json::Value::Array(a) => a.iter().for_each(|x| collect_strings(x, out)),
        serde_json::Value::Object(m) => {
            for (k, x) in m {
                // Configuration, not content: never scanned and never logged.
                if matches!(
                    k.as_str(),
                    "api_key"
                        | "api_key_env"
                        | "model_override_config"
                        | "_execution_context"
                ) {
                    continue;
                }
                collect_strings(x, out);
            }
        }
        _ => {}
    }
}

fn luhn_ok(digits: &str) -> bool {
    let ds: Vec<u32> = digits.chars().filter_map(|c| c.to_digit(10)).collect();
    if ds.len() < 13 || ds.len() > 19 {
        return false;
    }
    let mut sum = 0u32;
    for (i, d) in ds.iter().rev().enumerate() {
        let mut x = *d;
        if i % 2 == 1 {
            x *= 2;
            if x > 9 {
                x -= 9;
            }
        }
        sum += x;
    }
    sum % 10 == 0
}

/// Deterministic personal-data SHAPE detector (email, phone, IBAN, card number with Luhn). Returns the KINDS found, never the values.
pub fn detect_personal_data_shapes(text: &str, kinds: &[String]) -> Vec<String> {
    static EMAIL: OnceLock<regex::Regex> = OnceLock::new();
    static PHONE: OnceLock<regex::Regex> = OnceLock::new();
    static IBAN: OnceLock<regex::Regex> = OnceLock::new();
    static CARD: OnceLock<regex::Regex> = OnceLock::new();
    let on = |k: &str| kinds.iter().any(|x| x == k);
    let mut found = Vec::new();
    if on("email") {
        let re = EMAIL.get_or_init(|| {
            regex::Regex::new(r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}")
                .expect("email regex")
        });
        if re.is_match(text) {
            found.push("email".to_string());
        }
    }
    if on("phone") {
        let re =
            PHONE.get_or_init(|| regex::Regex::new(r"\+\d[\d\s().\-]{7,}\d").expect("phone regex"));
        if re.is_match(text) {
            found.push("phone".to_string());
        }
    }
    if on("iban") {
        let re = IBAN.get_or_init(|| {
            regex::Regex::new(r"\b[A-Z]{2}\d{2}[A-Z0-9]{11,30}\b").expect("iban regex")
        });
        if re.is_match(text) {
            found.push("iban".to_string());
        }
    }
    if on("card_number") {
        let re =
            CARD.get_or_init(|| regex::Regex::new(r"\b(?:\d[ \-]?){13,19}\b").expect("card regex"));
        if re.find_iter(text).any(|m| luhn_ok(m.as_str())) {
            found.push("card_number".to_string());
        }
    }
    found
}

fn approved(policy_region: &str, content_hash: &str) -> bool {
    let Ok(text) = std::fs::read_to_string(path(APPROVALS_LOG)) else {
        return false;
    };
    let mut state = false;
    for line in text.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let scope = v["scope"].as_str().unwrap_or("");
        let key = v["key"].as_str().unwrap_or("");
        let hit = (scope == "content" && key == content_hash)
            || (scope == "region" && key == policy_region);
        if hit {
            state = v["approve"].as_bool().unwrap_or(false);
        }
    }
    state
}

/// Operator decision on an outbound transfer. `scope` = `content` (one content hash) or `region` (every personal-data transfer to a region
/// label — an explicit instance policy decision). Append-only; the latest row for a key wins. Resolves the awaiting item when given.
pub fn record_outbound_approval(
    scope: &str,
    key: &str,
    approve: bool,
    by: &str,
    await_id: Option<&str>,
) -> Result<(), String> {
    if scope != "content" && scope != "region" {
        return Err("scope must be 'content' or 'region'".into());
    }
    let row = serde_json::json!({ "at": now_ms(), "scope": scope, "key": key, "approve": approve, "by": by });
    std::fs::create_dir_all(data_dir()).map_err(|e| e.to_string())?;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path(APPROVALS_LOG))
        .map_err(|e| e.to_string())?;
    writeln!(f, "{row}").map_err(|e| e.to_string())?;
    let policy = load_policy();
    record_decision(
        "outbound_approval",
        key,
        None,
        &policy,
        "",
        if approve { "approved" } else { "denied" },
        &[scope.to_string()],
        &format!("by {by}"),
    );
    if let Some(id) = await_id {
        let dd = data_dir();
        if approve {
            crate::orchestrator::awaiting::mark_resolved(&dd, id, "approved by operator");
        } else {
            crate::orchestrator::awaiting::mark_cancelled(&dd, id, "denied by operator");
        }
    }
    Ok(())
}

/// The outbound content gate. Called by the single pipeline-executor seam (`adapters::RegistryExecutorAdapter::execute`) before ANY pipeline
/// runs, because every model call, graph analysis and loop prompt travels through it.
///
/// Order: policy enabled? → destination region → region allowed for personal data → agreements accepted → deterministic shape detection
/// (+ optional strength-N vote on an allowed vote model) → operator approvals → policy action. A call that cannot be cleared returns
/// `allow=false` with `AWAITING_PREFIX` in the reason and an open awaiting item; it is NEVER let through unjudged.
pub async fn check_outbound(
    pipeline_id: u64,
    input: &serde_json::Value,
    executor: Option<&dyn crate::orchestrator::PipelineExecutor>,
) -> OutboundVerdict {
    let policy = load_policy();
    if !policy.enabled {
        return OutboundVerdict {
            allow: true,
            region: String::new(),
            kinds: Vec::new(),
            reason: "policy disabled by operator".into(),
            await_id: None,
        };
    }
    if passed() || policy.exempt_pipelines.contains(&pipeline_id) {
        return OutboundVerdict {
            allow: true,
            region: String::new(),
            kinds: Vec::new(),
            reason: "exempt".into(),
            await_id: None,
        };
    }
    let region = classify_destination(&policy, input);
    // Destination allowed for personal data: nothing to scan, nothing to ask.
    if policy
        .allow_personal_data_regions
        .iter()
        .any(|r| r == &region)
    {
        return OutboundVerdict {
            allow: true,
            region,
            kinds: Vec::new(),
            reason: "destination region allowed by policy".into(),
            await_id: None,
        };
    }
    let mut text = String::new();
    collect_strings(input, &mut text);
    if text.trim().is_empty() {
        return OutboundVerdict {
            allow: true,
            region,
            kinds: Vec::new(),
            reason: "no text content".into(),
            await_id: None,
        };
    }
    let content_hash = hash_hex(text.as_bytes())[..32].to_string();
    let dd = data_dir();

    // 1. Agreements: a third-country / unverified transfer needs the operator's acceptance of every current requirement.
    if policy.require_agreements_before_outbound {
        let missing = unaccepted_requirements(&policy);
        if !missing.is_empty() {
            let subject = format!("agreements:{}", policy_fingerprint(&policy));
            let item = crate::orchestrator::awaiting::open(
                &dd,
                KIND_AGREEMENT,
                &subject,
                None,
                None,
                &format!(
                    "outbound calls to region '{region}' wait for the operator to accept: {}",
                    missing.join(", ")
                ),
            );
            let id = item.ok().map(|i| i.await_id);
            record_decision(
                "outbound",
                &content_hash,
                None,
                &policy,
                "",
                "held_agreements",
                &missing,
                &format!("pipeline {pipeline_id} region {region}"),
            );
            return OutboundVerdict {
                allow: false,
                region,
                kinds: Vec::new(),
                reason: format!(
                    "{AWAITING_PREFIX} requirements not accepted for the current policy: {}",
                    missing.join(", ")
                ),
                await_id: id,
            };
        }
    }

    // 2. Personal-data detection.
    let mut kinds = if policy.detection.deterministic {
        detect_personal_data_shapes(&text, &policy.detection.kinds)
    } else {
        Vec::new()
    };
    let mut detector = if policy.detection.deterministic {
        "deterministic".to_string()
    } else {
        "none".to_string()
    };
    if kinds.is_empty() && policy.detection.vote {
        let voter_region_ok = !policy.detection.vote_model.is_empty()
            && policy
                .allow_personal_data_regions
                .iter()
                .any(|r| r == &policy.detection.vote_model_region);
        match (voter_region_ok, executor) {
            (true, Some(ex)) => {
                if vote_has_personal_data(ex, &policy.detection.vote_model, &text).await {
                    kinds.push("personal_data_vote".to_string());
                }
                detector.push_str("+vote");
            }
            _ => detector.push_str("+vote_unavailable(no allowed vote model configured)"),
        }
    }
    if kinds.is_empty() {
        record_decision(
            "outbound",
            &content_hash,
            None,
            &policy,
            "",
            "allowed_no_personal_data",
            &[],
            &format!("pipeline {pipeline_id} region {region} detector {detector}"),
        );
        return OutboundVerdict {
            allow: true,
            region,
            kinds,
            reason: format!("no personal data detected ({detector})"),
            await_id: None,
        };
    }

    // 3. Operator approvals (this content, or this region as an explicit instance decision).
    if approved(&region, &content_hash) {
        record_decision(
            "outbound",
            &content_hash,
            None,
            &policy,
            "",
            "allowed_operator_approved",
            &kinds,
            &format!("pipeline {pipeline_id} region {region}"),
        );
        return OutboundVerdict {
            allow: true,
            region,
            kinds,
            reason: "approved by operator".into(),
            await_id: None,
        };
    }

    // 4. Policy action.
    match policy.on_personal_data_outbound.as_str() {
        "allow_logged" => {
            record_decision(
                "outbound",
                &content_hash,
                None,
                &policy,
                "",
                "allowed_logged",
                &kinds,
                &format!("pipeline {pipeline_id} region {region}"),
            );
            OutboundVerdict {
                allow: true,
                region,
                kinds,
                reason: "policy allow_logged".into(),
                await_id: None,
            }
        }
        "block" => {
            record_decision(
                "outbound",
                &content_hash,
                None,
                &policy,
                "",
                "blocked",
                &kinds,
                &format!("pipeline {pipeline_id} region {region}"),
            );
            OutboundVerdict {
                allow: false,
                region: region.clone(),
                reason: format!("personal data ({}) may not be sent to region '{region}' under the instance policy (block)", kinds.join(", ")),
                kinds,
                await_id: None,
            }
        }
        _ => {
            let item = crate::orchestrator::awaiting::open(
                &dd,
                KIND_OUTBOUND_TRANSFER,
                &content_hash,
                None,
                None,
                &format!("personal data ({}) in content bound for region '{region}' — operator approval required (content or region scope)", kinds.join(", ")),
            );
            let id = item.ok().map(|i| i.await_id);
            record_decision(
                "outbound",
                &content_hash,
                None,
                &policy,
                "",
                "held_operator",
                &kinds,
                &format!("pipeline {pipeline_id} region {region}"),
            );
            OutboundVerdict {
                allow: false,
                region: region.clone(),
                reason: format!("{AWAITING_PREFIX} personal data ({}) bound for region '{region}' (content {content_hash})", kinds.join(", ")),
                kinds,
                await_id: id,
            }
        }
    }
}

/// Strength-N yes/no vote: does `text` contain personal data of natural persons? Runs on the operator-named in-region `vote_model` and is
/// scoped as already-cleared (it IS the gate). An unusable answer is NOT "no": it counts as not-yes, and the deterministic result stands.
async fn vote_has_personal_data(
    executor: &dyn crate::orchestrator::PipelineExecutor,
    vote_model: &str,
    text: &str,
) -> bool {
    use crate::shared_contracts::k_validation;
    let strength = k_validation::configured_strength();
    let policy = k_validation::ValidationPolicy { strength };
    let prompt = format!(
        "TEXT:\n{text}\n\nQUESTION: Does the TEXT above contain personal data about identifiable natural persons (names tied to other details, contact details, identifiers, health, financial or location data of a person)? Answer YES only if it clearly does. Answer NO if it does not or you are unsure."
    );
    let model = vote_model.to_string();
    k_validation::confirm_consecutive_yes_with(
        move || {
            let mut input = k_validation::yes_no_input(&prompt);
            input["model_override_config"] =
                serde_json::json!({ "model_identifier": model.clone() });
            async move {
                match scoped_passed(executor.execute(9, input)).await {
                    Ok(r) => r
                        .get("response")
                        .and_then(|x| x.as_str())
                        .and_then(k_validation::parse_yes_no),
                    Err(_) => None,
                }
            }
        },
        &policy,
    )
    .await
}

// ============================================================================
// ERASURE (minimal real part: plan + file-level redaction; the rest is listed in the report)
// ============================================================================

#[derive(Debug, Clone, Serialize, Default)]
pub struct ErasureFile {
    pub path: String,
    pub matches: usize,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ErasurePlan {
    pub needle_hash: String,
    pub files: Vec<ErasureFile>,
    pub total_matches: usize,
    /// Places this pass does NOT reach (store-held copies, hashes, indexes, peers) — named, never silently skipped.
    pub not_covered: Vec<String>,
}

fn is_sensitive_infra(rel: &str) -> bool {
    rel.starts_with(".internal_token")
        || rel == AGREEMENTS_FILE
        || rel == POLICY_FILE
        || rel == POLICY_HISTORY
        || rel == DECISION_LOG
        || rel == APPROVALS_LOG
}

fn walk_files(
    root: &std::path::Path,
    dir: &std::path::Path,
    out: &mut Vec<std::path::PathBuf>,
    seen: &mut std::collections::HashSet<std::path::PathBuf>,
) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        // Symlinks are never followed (no escape from the data dir, no cycles).
        let Ok(meta) = std::fs::symlink_metadata(&p) else {
            continue;
        };
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_dir() {
            if seen.insert(p.clone()) {
                walk_files(root, &p, out, seen);
            }
        } else if meta.is_file() {
            out.push(p);
        }
    }
}

fn file_kind(rel: &str) -> &'static str {
    if rel.ends_with(".jsonl") {
        "jsonl_logs"
    } else if rel.starts_with("runs/") || rel.contains("/runs/") {
        "run_files"
    } else {
        "json_state"
    }
}

/// Read-only: where does `needle` (case-insensitive) occur in the data dir's text files? Never returns the surrounding text.
pub fn erasure_plan(needle: &str) -> Result<ErasurePlan, String> {
    if needle.trim().chars().count() < 4 {
        return Err(
            "needle must be at least 4 characters (a short needle would match unrelated content)"
                .into(),
        );
    }
    let root = std::path::PathBuf::from(data_dir());
    let mut files = Vec::new();
    let mut seen = std::collections::HashSet::new();
    walk_files(&root, &root, &mut files, &mut seen);
    let lower = needle.to_lowercase();
    let mut plan = ErasurePlan {
        needle_hash: hash_hex(lower.as_bytes())[..16].to_string(),
        ..Default::default()
    };
    for p in files {
        let rel = p
            .strip_prefix(&root)
            .map(|r| r.to_string_lossy().to_string())
            .unwrap_or_default();
        if is_sensitive_infra(&rel) {
            continue;
        }
        // Binary or non-UTF-8 files are not read as text; they are listed as not covered below.
        let Ok(text) = std::fs::read_to_string(&p) else {
            continue;
        };
        let n = text.to_lowercase().matches(&lower).count();
        if n > 0 {
            plan.total_matches += n;
            plan.files.push(ErasureFile {
                path: rel.clone(),
                matches: n,
                kind: file_kind(&rel).to_string(),
            });
        }
    }
    plan.not_covered = vec![
        "container bodies held by the ZSEI store beyond plain files (content hashes and version histories are not rewritten here; use the store's UpdateContainer/DeleteContainer per container)".into(),
        "keyword and relationship indexes derived from the data (rebuilt from containers)".into(),
        "binary or non-UTF-8 files".into(),
        "copies already published to peers (cannot be recalled)".into(),
        "provider-side copies at model providers (governed by their terms; see the transfer requirement)".into(),
        "backups outside the data dir".into(),
    ];
    Ok(plan)
}

/// Destructive: replace every case-insensitive occurrence of `needle` with `[erased]` in the data dir's text files. Requires the operator's
/// confirmation phrase, a needle of >= 4 characters and `erasure.enabled`. Writes an audit row (needle HASH only). Optional before-copy only
/// when the policy keeps one. Returns the plan that was executed.
pub fn erasure_apply(needle: &str, confirm: &str, by: &str) -> Result<ErasurePlan, String> {
    let policy = load_policy();
    if !policy.erasure.enabled {
        return Err("erasure is disabled by the instance policy".into());
    }
    if confirm != "ERASE" {
        return Err("confirmation phrase must be exactly ERASE".into());
    }
    let plan = erasure_plan(needle)?;
    let root = std::path::PathBuf::from(data_dir());
    let lower = needle.to_lowercase();
    let stamp = now_ms();
    for f in &plan.files {
        let p = root.join(&f.path);
        let Ok(text) = std::fs::read_to_string(&p) else {
            continue;
        };
        if policy.erasure.keep_before_copy {
            let bak = root
                .join("erasure_before")
                .join(stamp.to_string())
                .join(&f.path);
            if let Some(parent) = bak.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("before-copy dir: {e}"))?;
            }
            std::fs::write(&bak, &text).map_err(|e| format!("before-copy: {e}"))?;
        }
        // Case-insensitive replace by walking the lowercase copy's match offsets. `to_lowercase` can change byte lengths for some scripts,
        // so only files whose lowercase form has the same length are rewritten in place; others are reported, not guessed at.
        let low = text.to_lowercase();
        if low.len() != text.len() {
            tracing::warn!("erasure: {} changes byte length when lower-cased — not rewritten (listed for manual handling)", f.path);
            continue;
        }
        let mut out = String::with_capacity(text.len());
        let mut last = 0usize;
        for (idx, _) in low.match_indices(&lower) {
            if idx < last
                || !text.is_char_boundary(idx)
                || !text.is_char_boundary(idx + lower.len())
            {
                continue;
            }
            out.push_str(&text[last..idx]);
            out.push_str("[erased]");
            last = idx + lower.len();
        }
        out.push_str(&text[last..]);
        let tmp = p.with_extension("erasing.tmp");
        std::fs::write(&tmp, &out).map_err(|e| format!("write {}: {e}", f.path))?;
        std::fs::rename(&tmp, &p).map_err(|e| format!("replace {}: {e}", f.path))?;
    }
    record_decision(
        "erasure",
        &plan.needle_hash,
        None,
        &policy,
        "",
        "applied",
        &[],
        &format!(
            "{} file(s), {} match(es) by {by}",
            plan.files.len(),
            plan.total_matches
        ),
    );
    Ok(plan)
}

// ============================================================================
// ADOPT / EXPORT (policy sharing is manifest-only, staged, operator-gated)
// ============================================================================

/// Adopt the `policy` block of a STAGED jurisdiction content file (`peer_staging/jurisdiction/<file>.json`) as the next policy version.
/// Operator-gated (the HTTP route requires an authenticated operator call); nothing is applied automatically when a peer offer is staged.
pub fn adopt_staged_policy(rel_path: &str, by: &str) -> Result<JurisdictionPolicy, String> {
    if rel_path.contains("..") || !rel_path.starts_with("peer_staging/jurisdiction/") {
        return Err("path must be under peer_staging/jurisdiction/".into());
    }
    let text =
        std::fs::read_to_string(path(rel_path)).map_err(|e| format!("read staged file: {e}"))?;
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("staged file is not JSON: {e}"))?;
    let block = v
        .get("policy")
        .cloned()
        .ok_or("staged file carries no `policy` block")?;
    let candidate: JurisdictionPolicy =
        serde_json::from_value(block).map_err(|e| format!("policy block invalid: {e}"))?;
    if candidate.providers.is_empty() || candidate.allow_personal_data_regions.is_empty() {
        return Err(
            "a policy must name providers and allowed regions (an empty policy is not adopted)"
                .into(),
        );
    }
    save_policy(candidate, by, &format!("adopted:{rel_path}"))
}

/// The live policy as a jurisdiction content file (disclaimer + one marker rule + `policy`) that the existing marketplace staging accepts.
pub fn export_policy_as_content() -> serde_json::Value {
    let p = load_policy();
    serde_json::json!({
        "disclaimer": "Instance jurisdiction POLICY exported for sharing as a manifest. Engineering configuration, NOT legal advice; the receiving operator reviews it and adopts it deliberately (never automatic). Contains no personal data and no content.",
        "rules": [{
            "scope": "Global",
            "condition": "jurisdiction policy",
            "action": "Log",
            "source": "instance policy export (marker rule so the staging validator accepts the file)",
            "retrieved_snippet": null,
            "official_source": null,
            "aliases": []
        }],
        "policy": p,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_policy_parses_and_is_never_open() {
        let p = shipped_defaults();
        assert!(p.enabled && p.require_agreements_before_outbound);
        assert!(!p.providers.is_empty() && !p.allow_personal_data_regions.is_empty());
        assert!(!p.requirements.is_empty(), "the shipped content lists the requirements the operator must accept");
        assert_eq!(p.on_personal_data_outbound, "await_operator");
        assert_ne!(p.version.to_string(), "", "version is a number");
    }

    #[test]
    fn detector_finds_shapes_and_returns_kinds_not_values() {
        let kinds: Vec<String> = ["email", "phone", "iban", "card_number"].iter().map(|s| s.to_string()).collect();
        let found = detect_personal_data_shapes("mail me at a.b@example.org or +44 20 7946 0958; card 4111 1111 1111 1111", &kinds);
        assert!(found.contains(&"email".to_string()));
        assert!(found.contains(&"phone".to_string()));
        assert!(found.contains(&"card_number".to_string()));
        assert!(detect_personal_data_shapes("list the first five prime numbers", &kinds).is_empty());
        // a 16-digit number that fails Luhn is not a card number
        assert!(!detect_personal_data_shapes("order 1234 5678 9012 3456", &kinds).contains(&"card_number".to_string()));
    }

    #[test]
    fn unlisted_or_unknown_destination_is_never_allowed_by_default() {
        let p = shipped_defaults();
        let over = serde_json::json!({ "model_override_config": { "api_endpoint": "https://api.example-llm.com/v1" } });
        let region = classify_destination(&p, &over);
        assert!(!p.allow_personal_data_regions.contains(&region), "an endpoint no provider rule matches must not be treated as allowed");
        let local = serde_json::json!({ "model_override_config": { "api_endpoint": "http://localhost:8080/v1" } });
        assert_eq!(classify_destination(&p, &local), "local");
    }

    #[test]
    fn luhn_accepts_a_valid_number_and_rejects_a_bad_one() {
        assert!(luhn_ok("4111111111111111"));
        assert!(!luhn_ok("4111111111111112"));
    }
}
