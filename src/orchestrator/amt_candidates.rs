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

/// Compaction threshold (NOT a cap): above this many live entries, HANDLED history is moved to the archive file. Unhandled entries
/// are never removed.
const MAX_CANDIDATE_LOG_ENTRIES: usize = 500;

pub fn candidates_path() -> String {
    std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string()) + "/amt_reexpansion_candidates.json"
}

/// Routes that carry a distinct FINDING (what happened), as opposed to the
/// structural "this AMT needs another look" routes. Two findings about the
/// same container are different information; two structural re-checks are not.
const FINDING_ROUTES: [&str; 4] = [
    "SecurityFinding",
    "CoordinationReceived",
    "TaskFailure",
    "CodeStaleness",
];

fn is_finding_route(route: &str) -> bool {
    FINDING_ROUTES.contains(&route)
}

/// Short stable digest of a candidate's detail text (blake3, 16 hex chars).
pub fn detail_digest(detail: Option<&str>) -> String {
    let h = blake3::hash(detail.unwrap_or("").as_bytes()).to_hex();
    h.as_str()[..16].to_string()
}

/// Append one candidate unless an equivalent unhandled one is already queued.
/// Returns true when appended.
///
/// DEDUPE KEY (changed 2026-10-23): the key used to be `container_id` alone
/// among unhandled entries. Every global finding resolves to the SAME
/// "Global Findings AMT" container, so while ONE global finding was unhandled
/// every later security / coordination / task-failure finding was dropped
/// without a trace — the host monitored and never acted. Now:
/// - FINDING routes (`SecurityFinding`, `CoordinationReceived`,
///   `TaskFailure`, `CodeStaleness`) dedupe on `(container_id, route,
///   digest(detail))`: the same finding twice collapses, distinct findings
///   all queue.
/// - every other (structural) route keeps the original behaviour: one
///   unhandled structural candidate per container, so a burst of graph
///   ripples still queues a single re-check.
/// Every drop is logged with its key — nothing disappears silently.
pub fn append(container_id: u64, project_id: Option<u64>, route: &str, detail: Option<&str>) -> bool {
    append_at(&candidates_path(), container_id, project_id, route, detail)
}

/// Path-injected core (tests and the loop pass explicit paths).
pub fn append_at(path: &str, container_id: u64, project_id: Option<u64>, route: &str, detail: Option<&str>) -> bool {
    let mut candidates: Vec<serde_json::Value> = crate::durable_write::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    let finding = is_finding_route(route);
    let digest = detail_digest(detail);

    let already = candidates.iter().any(|c| {
        if c.get("handled").and_then(|v| v.as_bool()) != Some(false) {
            return false;
        }
        if c.get("container_id").and_then(|v| v.as_u64()) != Some(container_id) {
            return false;
        }
        let c_route = c.get("route").and_then(|v| v.as_str()).unwrap_or("");
        if finding {
            // Entries written before the digest field existed derive it from
            // their stored `source` text.
            let c_digest = c
                .get("detail_digest")
                .and_then(|v| v.as_str())
                .map(|d| d.to_string())
                .unwrap_or_else(|| detail_digest(c.get("source").and_then(|v| v.as_str())));
            c_route == route && c_digest == digest
        } else {
            !is_finding_route(c_route)
        }
    });
    if already {
        if finding {
            tracing::debug!(
                container_id,
                route,
                digest = %digest,
                "AMT candidate not appended: the identical finding (container+route+detail) is already queued and unhandled"
            );
        } else {
            tracing::debug!(
                container_id,
                route,
                "AMT candidate not appended: an unhandled structural candidate for this container is already queued"
            );
        }
        return false;
    }

    candidates.push(json!({
        "container_id": container_id,
        "project_id": project_id,
        "route": route,
        "unverified_content": detail.map(|d| d.to_string()), // whole (no-cap doctrine, audit #27; was a 200-char cut)
        "source": detail.map(|d| d.to_string()),
        "detail_digest": digest,
        "recorded_at": now_secs(),
        "handled": false,
    }));

    if candidates.len() > MAX_CANDIDATE_LOG_ENTRIES {
        // COMPACT, never drop (no-cap doctrine, 2026-11-06): HANDLED history moves to an append-only archive file
        // (`<log>.archive.jsonl`, oldest first) until the live log is back under the threshold; UNHANDLED candidates are NEVER
        // removed — if they alone exceed the threshold the live log simply grows (the consumer is behind; that is warned).
        let mut need = candidates.len() - MAX_CANDIDATE_LOG_ENTRIES;
        let mut archived: Vec<serde_json::Value> = Vec::new();
        let mut i = 0;
        while need > 0 && i < candidates.len() {
            if candidates[i].get("handled").and_then(|v| v.as_bool()) == Some(true) {
                archived.push(candidates.remove(i));
                need -= 1;
            } else {
                i += 1;
            }
        }
        for row in &archived {
            // Durable append (src/durable_write.rs): an archived row is never lost to a transient I/O error.
            crate::durable_write::append(format!("{path}.archive.jsonl"), row.to_string());
        }
        if need > 0 {
            tracing::warn!(
                over_threshold = need,
                threshold = MAX_CANDIDATE_LOG_ENTRIES,
                "AMT candidate log holds more UNHANDLED entries than the compaction threshold — none were removed; the consumer is not keeping up"
            );
        }
    }

    // Durable replace: a failed write is retried until it succeeds, and reads below see a snapshot still being retried — a queued
    // re-expansion candidate is never silently lost.
    match serde_json::to_string_pretty(&candidates) {
        Ok(json_str) => crate::durable_write::replace(path, json_str.into_bytes()),
        Err(e) => tracing::error!(error = %e, "AMT candidate log not serialized"),
    }
    true
}

/// Load every unhandled candidate (the review pass consumes these).
pub fn unhandled() -> Vec<serde_json::Value> {
    unhandled_at(&candidates_path())
}

pub fn unhandled_at(path: &str) -> Vec<serde_json::Value> {
    let candidates: Vec<serde_json::Value> = crate::durable_write::read_to_string(path)
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
    match serde_json::to_string_pretty(candidates) {
        Ok(json_str) => crate::durable_write::replace(path, json_str.into_bytes()),
        Err(e) => tracing::error!(error = %e, "AMT candidate log not serialized"),
    }
}

/// Load ALL candidates (handled included) — for review passes that mutate
/// specific entries.
pub fn load_all() -> Vec<serde_json::Value> {
    load_all_at(&candidates_path())
}

pub fn load_all_at(path: &str) -> Vec<serde_json::Value> {
    crate::durable_write::read_to_string(path)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_path(tag: &str) -> String {
        let dir = std::env::temp_dir().join(format!(
            "amt_cand_{}_{}_{}",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("c.json").to_string_lossy().to_string()
    }

    #[test]
    fn distinct_findings_on_one_container_all_queue() {
        let p = tmp_path("distinct");
        assert!(append_at(&p, 9000, None, "SecurityFinding", Some("tool 'firewall_status' failed")));
        // Before the key fix this returned false and the finding was lost.
        assert!(append_at(&p, 9000, None, "SecurityFinding", Some("tool 'net_connections' failed")));
        assert!(append_at(&p, 9000, None, "CoordinationReceived", Some("note: finding about X")));
        assert_eq!(unhandled_at(&p).len(), 3);
    }

    #[test]
    fn identical_finding_collapses() {
        let p = tmp_path("identical");
        assert!(append_at(&p, 9000, None, "SecurityFinding", Some("tool 'x' failed")));
        assert!(!append_at(&p, 9000, None, "SecurityFinding", Some("tool 'x' failed")));
        assert_eq!(unhandled_at(&p).len(), 1);
    }

    #[test]
    fn structural_routes_still_dedupe_per_container() {
        let p = tmp_path("structural");
        assert!(append_at(&p, 7, Some(1), "GraphRipple", Some("event a")));
        assert!(!append_at(&p, 7, Some(1), "GraphRipple", Some("event b")));
        assert!(!append_at(&p, 7, Some(1), "UnverifiedNode", Some("node")));
        assert_eq!(unhandled_at(&p).len(), 1);
    }

    #[test]
    fn a_pending_structural_candidate_does_not_block_a_finding() {
        let p = tmp_path("mixed");
        assert!(append_at(&p, 7, Some(1), "GraphRipple", None));
        assert!(append_at(&p, 7, Some(1), "SecurityFinding", Some("real finding")));
        assert_eq!(unhandled_at(&p).len(), 2);
    }

    #[test]
    fn full_log_archives_handled_history_and_never_drops_unhandled() {
        let p = tmp_path("full");
        let mut seed: Vec<serde_json::Value> = Vec::new();
        for i in 0..MAX_CANDIDATE_LOG_ENTRIES {
            seed.push(json!({"container_id": i as u64, "route": "GraphRipple", "handled": i % 2 == 0}));
        }
        save_all_at(&p, &seed);
        assert!(append_at(&p, 99_999, None, "SecurityFinding", Some("new")));
        let all = load_all_at(&p);
        assert_eq!(all.len(), MAX_CANDIDATE_LOG_ENTRIES);
        // The oldest HANDLED entry (index 0) left the live log for the archive; every unhandled entry (index 1, 3, ...) survives.
        assert!(all.iter().any(|c| c["container_id"] == 1));
        assert!(!all.iter().any(|c| c["container_id"] == 0));
        let archive = std::fs::read_to_string(format!("{p}.archive.jsonl")).unwrap_or_default();
        assert!(archive.contains("\"container_id\":0"), "handled history must be archived, not deleted");
    }

    #[test]
    fn all_unhandled_log_grows_past_the_threshold_without_losing_any() {
        let p = tmp_path("allunhandled");
        let mut seed: Vec<serde_json::Value> = Vec::new();
        for i in 0..MAX_CANDIDATE_LOG_ENTRIES {
            seed.push(json!({"container_id": i as u64, "route": "GraphRipple", "handled": false}));
        }
        save_all_at(&p, &seed);
        assert!(append_at(&p, 99_999, None, "SecurityFinding", Some("new")));
        assert_eq!(load_all_at(&p).len(), MAX_CANDIDATE_LOG_ENTRIES + 1);
    }
}
