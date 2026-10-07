//! Per-model outcome ledger: every model call's outcome is recorded against
//! the model that produced it, appended to `model_calls/model_ledger.jsonl`,
//! and replayed on first use so the ranking survives a restart.
//!
//! The ranking is historical. A model that has succeeded more often is tried
//! first. A model with no records has a neutral prior (Laplace: one success
//! and one failure), so a single bad call does not condemn it.
//!
//! `probe_free_models` calls each catalog free model once, to show what each
//! one actually returns. It runs only when OZONE_PROBE_FREE_MODELS=1 is set,
//! because it spends free-model quota.

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

const CHAT_ENDPOINT: &str = "https://openrouter.ai/api/v1/chat/completions";
const PROBE_PROMPT: &str = "Reply with the single word OK.";
const PROBE_MAX_TOKENS: u64 = 64;
const PROBE_TIMEOUT: Duration = Duration::from_secs(60);
const CATALOG_WAIT: Duration = Duration::from_secs(60);

/// What one call to a model produced.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Success,
    /// Pipeline returned Ok with no usable text. `finish_reason` and the
    /// token fields come from the provider's response when present.
    EmptyResponse {
        finish_reason: Option<String>,
        content_null: bool,
        has_reasoning: bool,
        completion_tokens: Option<u64>,
        reasoning_tokens: Option<u64>,
    },
    /// Provider answered with an HTTP status >= 400.
    HttpError { status: u16 },
    Timeout,
    OtherError(String),
}

/// Aggregated history for one model.
#[derive(Debug, Clone)]
pub struct ModelStats {
    pub attempts: u64,
    pub successes: u64,
    pub empties: u64,
    pub http_errors: u64,
    pub timeouts: u64,
    pub other_errors: u64,
    /// Lower median of every recorded latency. `None` only with zero records.
    pub median_latency_ms: Option<u64>,
    pub last_outcome: String,
    pub last_seen_ms: u64,
}

#[derive(Default)]
struct Entry {
    attempts: u64,
    successes: u64,
    empties: u64,
    http_errors: u64,
    timeouts: u64,
    other_errors: u64,
    latencies: Vec<u64>,
    last_outcome: String,
    last_seen_ms: u64,
}

impl Entry {
    fn apply(&mut self, outcome: &Outcome, latency_ms: u64, ts_ms: u64) {
        self.attempts += 1;
        self.latencies.push(latency_ms);
        self.last_outcome = kind_name(outcome).to_string();
        self.last_seen_ms = ts_ms;
        match outcome {
            Outcome::Success => self.successes += 1,
            Outcome::EmptyResponse { .. } => self.empties += 1,
            Outcome::HttpError { .. } => self.http_errors += 1,
            Outcome::Timeout => self.timeouts += 1,
            Outcome::OtherError(_) => self.other_errors += 1,
        }
    }

    fn median(&self) -> Option<u64> {
        // Callers without a timing window record latency 0; those rows still
        // count for the success score but must not drag the median down.
        let real: Vec<u64> = self.latencies.iter().copied().filter(|l| *l > 0).collect();
        if real.is_empty() {
            return None;
        }
        let mut sorted = real;
        sorted.sort_unstable();
        Some(sorted[(sorted.len() - 1) / 2])
    }
}

#[derive(Default)]
struct Table {
    entries: HashMap<String, Entry>,
}

static TABLE: OnceLock<RwLock<Table>> = OnceLock::new();

fn table() -> &'static RwLock<Table> {
    TABLE.get_or_init(|| RwLock::new(load_file()))
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn ledger_path() -> PathBuf {
    let data_dir = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    PathBuf::from(data_dir).join("model_calls").join("model_ledger.jsonl")
}

fn kind_name(outcome: &Outcome) -> &'static str {
    match outcome {
        Outcome::Success => "success",
        Outcome::EmptyResponse { .. } => "empty",
        Outcome::HttpError { .. } => "http_error",
        Outcome::Timeout => "timeout",
        Outcome::OtherError(_) => "other_error",
    }
}

/// One persisted row. Fields that do not apply to a kind are omitted.
#[derive(Serialize, Deserialize)]
struct LedgerLine {
    ts_ms: u64,
    model: String,
    kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    status: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    finish_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    content_null: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    has_reasoning: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    completion_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reasoning_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    /// The attributed cause, stored at write time (see `Cause`). Rows written
    /// before this field existed have none, and are attributed on replay.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cause: Option<String>,
    latency_ms: u64,
    call_site: String,
}

fn to_line(model: &str, outcome: &Outcome, latency_ms: u64, call_site: &str, ts_ms: u64) -> LedgerLine {
    let mut line = LedgerLine {
        ts_ms,
        model: model.to_string(),
        kind: kind_name(outcome).to_string(),
        status: None,
        finish_reason: None,
        content_null: None,
        has_reasoning: None,
        completion_tokens: None,
        reasoning_tokens: None,
        message: None,
        cause: None,
        latency_ms,
        call_site: call_site.to_string(),
    };
    match outcome {
        Outcome::Success | Outcome::Timeout => {}
        Outcome::EmptyResponse {
            finish_reason,
            content_null,
            has_reasoning,
            completion_tokens,
            reasoning_tokens,
        } => {
            line.finish_reason = finish_reason.clone();
            line.content_null = Some(*content_null);
            line.has_reasoning = Some(*has_reasoning);
            line.completion_tokens = *completion_tokens;
            line.reasoning_tokens = *reasoning_tokens;
        }
        Outcome::HttpError { status } => line.status = Some(*status),
        Outcome::OtherError(m) => line.message = Some(m.clone()),
    }
    line
}

fn from_line(line: &LedgerLine) -> Result<Outcome, String> {
    match line.kind.as_str() {
        "success" => Ok(Outcome::Success),
        "empty" => Ok(Outcome::EmptyResponse {
            finish_reason: line.finish_reason.clone(),
            content_null: line.content_null.unwrap_or(false),
            has_reasoning: line.has_reasoning.unwrap_or(false),
            completion_tokens: line.completion_tokens,
            reasoning_tokens: line.reasoning_tokens,
        }),
        "http_error" => line
            .status
            .map(|status| Outcome::HttpError { status })
            .ok_or_else(|| "http_error row has no status".to_string()),
        "timeout" => Ok(Outcome::Timeout),
        "other_error" => Ok(Outcome::OtherError(line.message.clone().unwrap_or_default())),
        other => Err(format!("unknown outcome kind '{other}'")),
    }
}

/// Load the persisted ledger. A missing file is an empty ledger. Unreadable
/// rows are skipped and reported in one warning with the first bad line.
fn load_file() -> Table {
    let mut table = Table::default();
    let path = ledger_path();
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return table,
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "model ledger not read — rankings start empty");
            return table;
        }
    };
    let mut skipped = 0usize;
    let mut first_bad: Option<(usize, String)> = None;
    for (index, raw) in text.lines().enumerate() {
        if raw.trim().is_empty() {
            continue;
        }
        let parsed: Result<(String, Outcome, u64, u64), String> = serde_json::from_str::<LedgerLine>(raw)
            .map_err(|e| e.to_string())
            .and_then(|line| {
                let outcome = from_line(&line)?;
                Ok((line.model, outcome, line.latency_ms, line.ts_ms))
            });
        match parsed {
            Ok((model, outcome, latency_ms, ts_ms)) => {
                table
                    .entries
                    .entry(model)
                    .or_default()
                    .apply(&outcome, latency_ms, ts_ms);
            }
            Err(reason) => {
                skipped += 1;
                if first_bad.is_none() {
                    first_bad = Some((index + 1, reason));
                }
            }
        }
    }
    if let Some((line_no, reason)) = first_bad {
        tracing::warn!(
            path = %path.display(),
            skipped,
            first_bad_line = line_no,
            reason = %reason,
            "model ledger has unreadable rows — skipped; rankings use the readable rows"
        );
    }
    table
}

fn append_line(line: &LedgerLine) {
    let path = ledger_path();
    if let Some(dir) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(dir) {
            tracing::warn!(path = %path.display(), error = %e, "model ledger directory not created; row not persisted");
            return;
        }
    }
    let text = match serde_json::to_string(line) {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(error = %e, "model ledger row not serialized");
            return;
        }
    };
    let written = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| writeln!(f, "{text}"));
    if let Err(e) = written {
        tracing::warn!(path = %path.display(), error = %e, "model ledger row not persisted");
    }
}

/// Record one call's outcome. The in-memory table is updated first; the row
/// is then appended to disk. A failed append is logged and the in-memory
/// record is kept. Returns the attributed cause (`None` for a success).
pub fn record(model: &str, outcome: Outcome, latency_ms: u64, call_site: &str) -> Option<Cause> {
    record_with_cap(model, outcome, latency_ms, call_site, None)
}

/// As `record`, but with the output cap we sent, so the token-cap rule for
/// `Cause::OzoneStudio` can apply at write time.
pub fn record_with_cap(
    model: &str,
    outcome: Outcome,
    latency_ms: u64,
    call_site: &str,
    max_tokens_sent: Option<u64>,
) -> Option<Cause> {
    let ts = now_ms();
    let cause = cause_of_with_cap(&outcome, model, max_tokens_sent);
    let mut line = to_line(model, &outcome, latency_ms, call_site, ts);
    line.cause = cause.map(|c| c.tag().to_string());
    {
        let mut t = table().write().unwrap_or_else(|e| e.into_inner());
        t.entries
            .entry(model.to_string())
            .or_default()
            .apply(&outcome, latency_ms, ts);
    }
    append_line(&line);
    cause
}

/// Who caused a failed attempt. Provider and Model failures belong to
/// OpenRouter or the model; LocalRuntime is the local model process;
/// OzoneStudio is our own code or our own settings; Unknown means the provider
/// did not report the fields needed to decide, and it is never guessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cause {
    Provider,
    Model,
    LocalRuntime,
    OzoneStudio,
    Unknown,
}

impl Cause {
    /// Stable label used in logs, the walk trail, and the ledger.
    pub fn tag(self) -> &'static str {
        match self {
            Cause::Provider => "provider",
            Cause::Model => "model",
            Cause::LocalRuntime => "local-runtime",
            Cause::OzoneStudio => "ozone-studio",
            Cause::Unknown => "unknown",
        }
    }
}

/// True for model ids this host runs itself (BitNet GGUF paths and similar).
/// An OpenRouter slug such as `vendor/model:free` is served remotely. The
/// BitNet ids carry a filesystem path after the colon, so the rule is by
/// prefix rather than by the presence of a slash.
pub fn is_local_model_id(model: &str) -> bool {
    ["bitnet", "gguf", "onnx", "local"].iter().any(|p| model.starts_with(p))
}

/// Attribute a failed attempt. Success has no cause and returns `None`.
///
/// Rules, first match wins:
/// - Empty response, reasoning present and finish "length", or completion
///   tokens >= the cap we sent: OzoneStudio. Our own cap was spent on hidden
///   reasoning; the provider did nothing wrong. (The cap rule needs
///   `max_tokens_sent`; replayed rows without a stored cause do not have it.)
/// - Empty response, content null and finish "stop": Model if remote, LocalRuntime if local.
/// - Other empty response: Unknown.
/// - HTTP 429, 402, 5xx: Provider (rate limit, credits, or the provider is down).
/// - HTTP 401 or 403: OzoneStudio (our key or our account settings were refused).
/// - HTTP 400 or 404: Unknown. The error body is not carried in `Outcome`, so
///   we cannot tell a model-named rejection from a request we built wrongly.
/// - Timeout (a client timeout from the HTTP layer, text "timed out"): Provider
///   if remote, LocalRuntime if local. The provider did not answer in time.
/// - Watchdog expiry (our per-attempt budget, text starting "watchdog"):
///   OzoneStudio, whatever the model was doing. It is our timer.
/// - Other error naming our own pipeline, executor, watchdog, or missing config,
///   or a parse failure that does not name the response body: OzoneStudio.
/// - Other error whose text starts "confetti": Model (several JSON candidates).
/// - Any other error: Unknown.
pub fn cause_of_with_cap(outcome: &Outcome, model: &str, max_tokens_sent: Option<u64>) -> Option<Cause> {
    let local = is_local_model_id(model);
    match outcome {
        Outcome::Success => None,
        Outcome::EmptyResponse {
            finish_reason,
            content_null,
            has_reasoning,
            completion_tokens,
            ..
        } => {
            let finish = finish_reason.as_deref();
            let cap_spent = matches!((completion_tokens, max_tokens_sent), (Some(c), Some(m)) if *c >= m);
            if *has_reasoning && (finish == Some("length") || cap_spent) {
                return Some(Cause::OzoneStudio);
            }
            if *content_null && finish == Some("stop") {
                return Some(if local { Cause::LocalRuntime } else { Cause::Model });
            }
            Some(Cause::Unknown)
        }
        Outcome::HttpError { status } => Some(match *status {
            429 | 402 | 500..=599 => Cause::Provider,
            401 | 403 => Cause::OzoneStudio,
            _ => Cause::Unknown,
        }),
        Outcome::Timeout => Some(if local { Cause::LocalRuntime } else { Cause::Provider }),
        Outcome::OtherError(text) => {
            let t = text.to_lowercase();
            if text.starts_with("watchdog") {
                // Our per-attempt budget expired: our timer, not the provider's.
                Some(Cause::OzoneStudio)
            } else if text.starts_with("confetti") {
                Some(Cause::Model)
            } else if t.contains("pipeline")
                || t.contains("executor")
                || t.contains("watchdog")
                || t.contains("missing")
                || (t.contains("parse") && !t.contains("response"))
            {
                Some(Cause::OzoneStudio)
            } else {
                Some(Cause::Unknown)
            }
        }
    }
}

/// `cause_of_with_cap` with no cap known. Success returns `None`.
pub fn cause_of(outcome: &Outcome, model: &str) -> Option<Cause> {
    cause_of_with_cap(outcome, model, None)
}

/// Today's attributed failures (UTC day), read from the ledger on disk so the
/// totals survive a restart. Each failure is counted three ways: `cause=<tag>`
/// (the cause total), `cause=<tag> model=<id>`, and `cause=<tag> kind=<kind>`.
/// Successes are not counted. Rows written before causes were stored are
/// attributed here with the rules above and no cap.
pub fn cause_summary_today() -> Vec<(String, usize)> {
    let day = now_ms() / 86_400_000;
    let Ok(text) = std::fs::read_to_string(ledger_path()) else {
        return Vec::new();
    };
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let Ok(row) = serde_json::from_str::<LedgerLine>(line) else {
            continue;
        };
        if row.ts_ms / 86_400_000 != day {
            continue;
        }
        let tag = match &row.cause {
            Some(stored) => stored.clone(),
            None => match from_line(&row).ok().and_then(|o| cause_of(&o, &row.model)) {
                Some(c) => c.tag().to_string(),
                None => continue,
            },
        };
        *counts.entry(format!("cause={tag}")).or_default() += 1;
        *counts.entry(format!("cause={tag} model={}", row.model)).or_default() += 1;
        *counts.entry(format!("cause={tag} kind={}", row.kind)).or_default() += 1;
    }
    counts.into_iter().collect()
}

/// Historical preference, best first. Score is (successes + 1) / (attempts + 2)
/// (Laplace prior). Ties go to the lower median latency, then to the input
/// order. A model with zero records scores 0.5 and has no median, so it sorts
/// after measured models with the same score.
pub fn rank(models: &[String]) -> Vec<String> {
    let t = table().read().unwrap_or_else(|e| e.into_inner());
    let mut scored: Vec<(f64, u64, usize, String)> = models
        .iter()
        .enumerate()
        .map(|(index, model)| {
            let (successes, attempts, median) = match t.entries.get(model) {
                Some(e) => (e.successes, e.attempts, e.median().unwrap_or(u64::MAX)),
                None => (0, 0, u64::MAX),
            };
            let score = (successes as f64 + 1.0) / (attempts as f64 + 2.0);
            (score, median, index, model.clone())
        })
        .collect();
    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.1.cmp(&b.1))
            .then(a.2.cmp(&b.2))
    });
    scored.into_iter().map(|row| row.3).collect()
}

/// Aggregated history for one model, or `None` when it has no records.
pub fn stats(model: &str) -> Option<ModelStats> {
    let t = table().read().unwrap_or_else(|e| e.into_inner());
    let e = t.entries.get(model)?;
    if e.attempts == 0 {
        return None;
    }
    Some(ModelStats {
        attempts: e.attempts,
        successes: e.successes,
        empties: e.empties,
        http_errors: e.http_errors,
        timeouts: e.timeouts,
        other_errors: e.other_errors,
        median_latency_ms: e.median(),
        last_outcome: e.last_outcome.clone(),
        last_seen_ms: e.last_seen_ms,
    })
}

/// Classify a chat-completions body that arrived with a 2xx status.
pub fn classify_response(body: &serde_json::Value) -> Outcome {
    let choice = &body["choices"][0];
    let message = &choice["message"];
    let content = message["content"].as_str();
    match content {
        Some(text) if !text.trim().is_empty() => Outcome::Success,
        _ => Outcome::EmptyResponse {
            finish_reason: choice["finish_reason"].as_str().map(String::from),
            content_null: message["content"].is_null(),
            has_reasoning: message["reasoning"]
                .as_str()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false),
            completion_tokens: body["usage"]["completion_tokens"].as_u64(),
            reasoning_tokens: body["usage"]["completion_tokens_details"]["reasoning_tokens"].as_u64(),
        },
    }
}

async fn probe_one(client: &reqwest::Client, api_key: &str, model: &str) -> Outcome {
    let body = serde_json::json!({
        "model": model,
        "messages": [{ "role": "user", "content": PROBE_PROMPT }],
        "max_tokens": PROBE_MAX_TOKENS,
    });
    let resp = match client.post(CHAT_ENDPOINT).bearer_auth(api_key).json(&body).send().await {
        Ok(r) => r,
        Err(e) if e.is_timeout() => return Outcome::Timeout,
        Err(e) => return Outcome::OtherError(format!("request failed: {e}")),
    };
    let status = resp.status().as_u16();
    if status >= 400 {
        return Outcome::HttpError { status };
    }
    match resp.json::<serde_json::Value>().await {
        Ok(json) => classify_response(&json),
        Err(e) if e.is_timeout() => Outcome::Timeout,
        Err(e) => Outcome::OtherError(format!("response not JSON: {e}")),
    }
}

fn describe(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Success => "success".to_string(),
        Outcome::EmptyResponse {
            finish_reason,
            content_null,
            has_reasoning,
            completion_tokens,
            reasoning_tokens,
        } => format!(
            "empty (finish_reason={}, content_null={}, has_reasoning={}, completion_tokens={}, reasoning_tokens={})",
            finish_reason.as_deref().unwrap_or("absent"),
            content_null,
            has_reasoning,
            completion_tokens.map(|n| n.to_string()).unwrap_or_else(|| "absent".into()),
            reasoning_tokens.map(|n| n.to_string()).unwrap_or_else(|| "absent".into()),
        ),
        Outcome::HttpError { status } => format!("http_error {status}"),
        Outcome::Timeout => "timeout".to_string(),
        Outcome::OtherError(m) => format!("other_error: {m}"),
    }
}

/// One-shot probe: each catalog free model is called once with a fixed tiny
/// prompt, one at a time, never retried. Every outcome is recorded with
/// call_site "probe" and printed as one line per model, then a summary.
/// Waits up to a minute for the boot-time catalog fetch if it is not loaded.
pub async fn probe_free_models(api_key: String) {
    let mut waited = Duration::ZERO;
    let ids: Vec<String> = loop {
        let list = crate::model_windows::catalog_pool("free", usize::MAX, &HashSet::new(), false);
        if !list.is_empty() || waited >= CATALOG_WAIT {
            break list.into_iter().map(|e| e.id).collect();
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
        waited += Duration::from_secs(2);
    };
    if ids.is_empty() {
        tracing::warn!("OZONE_PROBE_FREE_MODELS: no free models in the catalog after waiting; nothing probed");
        return;
    }
    let client = match reqwest::Client::builder().timeout(PROBE_TIMEOUT).build() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "OZONE_PROBE_FREE_MODELS: HTTP client not built; nothing probed");
            return;
        }
    };
    tracing::warn!(
        models = ids.len(),
        "OZONE_PROBE_FREE_MODELS: probing each catalog free model once (one request each; spends free-model quota)"
    );
    let mut success = 0usize;
    let mut empty = 0usize;
    let mut failed = 0usize;
    for id in &ids {
        let started = Instant::now();
        let outcome = probe_one(&client, &api_key, id).await;
        let latency_ms = started.elapsed().as_millis() as u64;
        let label = describe(&outcome);
        match outcome {
            Outcome::Success => success += 1,
            Outcome::EmptyResponse { .. } => empty += 1,
            _ => failed += 1,
        }
        tracing::info!(model = %id, latency_ms, result = %label, "free-model probe");
        record(id, outcome, latency_ms, "probe");
    }
    tracing::info!(
        total = ids.len(),
        success,
        empty,
        failed,
        "free-model probe finished; results are in model_calls/model_ledger.jsonl"
    );
}
