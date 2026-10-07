//! OpenRouter free-model quota, read from OpenRouter itself.
//!
//! `GET /api/v1/key` reports `free_model_daily_requests` ({used, limit,
//! remaining}) for the current UTC day. This module reads it on a timer and
//! warns when the account is near or at its cap. The call is metadata only:
//! no model is invoked, so it spends none of the quota it measures.
//!
//! The tier is taken from the reported `limit`, not from `is_free_tier`.
//! OpenRouter's own description of that boolean is ambiguous, so the limit
//! is the value we trust: 50 means the unfunded tier, 1000 means funded. A
//! field the response does not carry is reported as unknown, never estimated.
//!
//! The API key is read from the environment and is never logged.

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock, RwLock};
use std::time::Duration;

use serde::{Deserialize, Serialize};

const KEY_ENDPOINT: &str = "https://openrouter.ai/api/v1/key";
const POLL_INTERVAL: Duration = Duration::from_secs(15 * 60);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const UNFUNDED_LIMIT: u64 = 50;

/// One reading of the free-model counter. `None` means OpenRouter did not
/// report that field, so the value is unknown, not zero.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FreeQuota {
    pub fetched_at_ms: u64,
    pub utc_day: String,
    pub used: Option<u64>,
    pub limit: Option<u64>,
    pub remaining: Option<u64>,
    pub usage_daily_credits: Option<f64>,
    pub limit_remaining_credits: Option<f64>,
}

static LATEST: RwLock<Option<FreeQuota>> = RwLock::new(None);
static WARNED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn warned() -> &'static Mutex<HashSet<String>> {
    WARNED.get_or_init(|| Mutex::new(HashSet::new()))
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Warn once per key. The key carries the UTC day, so each threshold warns
/// once per day and again the next day.
fn warn_once(key: String, message: String) {
    let mut seen = warned().lock().unwrap_or_else(|e| e.into_inner());
    // Every key starts with its UTC day. Drop keys from earlier days so the
    // set stays bounded by one day's keys instead of growing for the process.
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    seen.retain(|k| k.starts_with(&today));
    if seen.insert(key) {
        tracing::warn!("{message}");
    }
}

/// Parse the `GET /api/v1/key` body. Missing fields stay `None`.
pub fn parse_key_response(body: &serde_json::Value, fetched_at_ms: u64) -> FreeQuota {
    let data = body.get("data").unwrap_or(body);
    let free = data.get("free_model_daily_requests");
    // Whole-number counts may arrive as JSON integers or as integral floats;
    // a fractional or non-numeric value stays unknown rather than rounding.
    let count = |field: &str| {
        free.and_then(|f| f.get(field)).and_then(|v| {
            v.as_u64().or_else(|| {
                v.as_f64()
                    .filter(|f| f.is_finite() && *f >= 0.0 && f.fract() == 0.0)
                    .map(|f| f as u64)
            })
        })
    };
    FreeQuota {
        fetched_at_ms,
        utc_day: chrono::Utc::now().format("%Y-%m-%d").to_string(),
        used: count("used"),
        limit: count("limit"),
        remaining: count("remaining"),
        usage_daily_credits: data.get("usage_daily").and_then(|v| v.as_f64()),
        limit_remaining_credits: data.get("limit_remaining").and_then(|v| v.as_f64()),
    }
}

/// Ask OpenRouter for the current key status. Metadata only; no model call.
pub async fn fetch(api_key: &str) -> Result<FreeQuota, String> {
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| format!("client build failed: {e}"))?;
    let resp = client
        .get(KEY_ENDPOINT)
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("response was not JSON: {e}"))?;
    Ok(parse_key_response(&body, now_ms()))
}

/// Warn when the reading is near or at its cap, and say which tier the
/// reported limit implies. Nothing is warned about a field that is absent.
pub fn check_and_warn(q: &FreeQuota) {
    let day = &q.utc_day;
    let (used, limit) = match (q.used, q.limit) {
        (Some(_), Some(0)) => {
            warn_once(
                format!("{day}:zero-limit"),
                "OpenRouter reported a free-model limit of 0 — the free quota is effectively unavailable today".to_string(),
            );
            return;
        }
        (Some(u), Some(l)) => (u, l),
        _ => {
            warn_once(
                format!("{day}:unknown"),
                "OpenRouter /api/v1/key did not report free_model_daily_requests — free-model quota is UNKNOWN today, not zero".to_string(),
            );
            return;
        }
    };
    let tier = if limit == UNFUNDED_LIMIT {
        "unfunded tier (50/day; a $10 credit purchase raises it to 1000/day)".to_string()
    } else if limit >= 1000 {
        format!("funded tier ({limit}/day)")
    } else {
        format!("tier with a {limit}/day cap")
    };
    let pct = used.saturating_mul(100) / limit;
    if pct >= 100 {
        warn_once(
            format!("{day}:exhausted"),
            format!("OpenRouter free-model quota EXHAUSTED: {used}/{limit} used today, {tier}. Free-model calls fail until the UTC day resets."),
        );
    } else if pct >= 95 {
        warn_once(
            format!("{day}:95"),
            format!("OpenRouter free-model quota at {pct}%: {used}/{limit} used today, {tier}."),
        );
    } else if pct >= 80 {
        warn_once(
            format!("{day}:80"),
            format!("OpenRouter free-model quota at {pct}%: {used}/{limit} used today, {tier}."),
        );
    }
}

/// Warn once per day when a paid model serves a call that came through a
/// router in the chain. Paid routing is billed against credits, so it should
/// be visible rather than silent.
pub fn note_paid_served(via: &str, served: &str) {
    let day = chrono::Utc::now().format("%Y-%m-%d").to_string();
    warn_once(
        format!("{day}:paid:{served}"),
        format!("paid model '{served}' served a call routed through '{via}' — this is billed against credits, not the free quota"),
    );
}

/// Say once per day that `free_only` removed a named fallback entry. The
/// entry is not paid-routed; it is excluded because the chain is free-only.
pub fn note_free_only_exclusion(model: &str) {
    let day = chrono::Utc::now().format("%Y-%m-%d").to_string();
    warn_once(
        format!("{day}:excluded:{model}"),
        format!("fallback entry '{model}' excluded: the chain is free-only and this entry is not marked free, so it is never requested"),
    );
}

/// Most recent successful reading, if any.
pub fn latest() -> Option<FreeQuota> {
    LATEST.read().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Where the latest reading is written, next to the other capture files.
fn snapshot_path() -> std::path::PathBuf {
    let data_dir = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    std::path::PathBuf::from(data_dir).join("model_calls").join("openrouter_quota.json")
}

async fn persist(q: &FreeQuota) {
    let path = snapshot_path();
    if let Some(dir) = path.parent() {
        let _ = tokio::fs::create_dir_all(dir).await;
    }
    match serde_json::to_string_pretty(q) {
        Ok(text) => {
            if let Err(e) = tokio::fs::write(&path, text).await {
                tracing::warn!(path = %path.display(), error = %e, "OpenRouter quota snapshot not written");
            }
        }
        Err(e) => tracing::warn!(error = %e, "OpenRouter quota snapshot not serialized"),
    }
}

/// Poll the key status at boot and every fifteen minutes. A failed poll is
/// logged every time, not deduplicated, so a stale reading is never silent.
pub fn spawn_monitor(api_key: String) {
    tokio::spawn(async move {
        loop {
            match fetch(&api_key).await {
                Ok(q) => {
                    check_and_warn(&q);
                    persist(&q).await;
                    *LATEST.write().unwrap_or_else(|e| e.into_inner()) = Some(q);
                }
                Err(e) => tracing::warn!(
                    error = %e,
                    "OpenRouter quota check failed — free-model usage is unknown until the next check"
                ),
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    });
}
