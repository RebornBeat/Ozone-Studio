//! AMT expansion candidates — THE unified store for re-expansion work.
//!
//! Before this module there were TWO producers writing the candidates file
//! with two different JSON shapes (amt.rs's in-build unverified-node
//! recorder, and amt_loop.rs's graph-ripple appender), and the review loop
//! read the file directly. This module is the single contract, K-registry
//! style: one route taxonomy, one append, one unhandled listing.
//!
//! Routes (why this AMT needs more work):
//!   • UnverifiedNode — a freshly persisted project-anchored AMT still has a
//!     node with no source provenance (the original trigger, from
//!     amt.rs::record_amt_reexpansion_candidate at build time).
//!   • GraphRipple — a scoped graph write (proj:/file:) in the project's
//!     living graph; the AMT must re-check its context alignment.
//!   • Continuation — a subsequent prompt expanded the project's AMT; the
//!     prior generation is chained via a `Continues` root relation.
//! (InitialBuild is not a candidate — it is the build itself.)

use serde_json::json;

/// Maximum log length — the same 500-entry convention the in-build
/// recorder used, so the file cannot grow without bound.
const MAX_CANDIDATE_LOG_ENTRIES: usize = 500;

pub fn candidates_path() -> String {
    std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string()) + "/amt_reexpansion_candidates.json"
}

/// Append one candidate if an unhandled candidate for the same AMT
/// container does not already exist. Returns true when appended.
pub fn append(container_id: u64, project_id: Option<u64>, route: &str, detail: Option<&str>) -> bool {
    append_at(&candidates_path(), container_id, project_id, route, detail)
}

/// Path-injected core (tests and the loop pass explicit paths).
pub fn append_at(path: &str, container_id: u64, project_id: Option<u64>, route: &str, detail: Option<&str>) -> bool {
    let mut candidates: Vec<serde_json::Value> = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    let already = candidates.iter().any(|c| {
        c.get("container_id").and_then(|v| v.as_u64()) == Some(container_id)
            && c.get("handled").and_then(|v| v.as_bool()) == Some(false)
    });
    if already {
        return false;
    }

    candidates.push(json!({
        "container_id": container_id,
        "project_id": project_id,
        "route": route,
        "unverified_content": detail.map(|d| d.chars().take(200).collect::<String>()),
        "source": detail.map(|d| d.to_string()),
        "recorded_at": now_secs(),
        "handled": false,
    }));

    if candidates.len() > MAX_CANDIDATE_LOG_ENTRIES {
        let drop = candidates.len() - MAX_CANDIDATE_LOG_ENTRIES;
        candidates.drain(0..drop);
    }

    if let Ok(json_str) = serde_json::to_string_pretty(&candidates) {
        let _ = std::fs::write(path, json_str);
    }
    true
}

/// Load every unhandled candidate (the review pass consumes these).
pub fn unhandled() -> Vec<serde_json::Value> {
    unhandled_at(&candidates_path())
}

pub fn unhandled_at(path: &str) -> Vec<serde_json::Value> {
    let candidates: Vec<serde_json::Value> = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    candidates
        .into_iter()
        .filter(|c| c.get("handled").and_then(|v| v.as_bool()) != Some(true))
        .collect()
}

/// Persist mutations back to the store (the review pass marks handled /
/// attempts on its own working copies, then calls this).
pub fn save_all(candidates: &[serde_json::Value]) {
    save_all_at(&candidates_path(), candidates);
}

pub fn save_all_at(path: &str, candidates: &[serde_json::Value]) {
    if let Ok(json_str) = serde_json::to_string_pretty(candidates) {
        let _ = std::fs::write(path, json_str);
    }
}

/// Load ALL candidates (handled included) — for review passes that mutate
/// specific entries.
pub fn load_all() -> Vec<serde_json::Value> {
    load_all_at(&candidates_path())
}

pub fn load_all_at(path: &str) -> Vec<serde_json::Value> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
