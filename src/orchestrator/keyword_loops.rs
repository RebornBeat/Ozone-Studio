//! The keyword program's own ACTORS (2026-11-08; docs/STATE_REVIEW_2026-11-08_EVENING.md I1).
//!
//! Before this module the rollup, the training and the settlement were the tail of the methodology meta loop's pass. Settlement makes one model
//! call per unsettled container, so the pass ran for HOURS (the last "review pass complete" was 07:23 UTC) and nothing else the meta pass does —
//! sufficiency reviews, quality audit, gap drafting — ran in the meantime. They are now two independent actors, each with its own timing:
//!   * HOUSEKEEPING — cheap and frequent: fold new hit events into the containers, back-fill the typed scope, and TRAIN (continuously — it never
//!     stops and never waits for a promotion; see `keyword_program::train_continuously`).
//!   * SETTLEMENT — long-running: verbatim capture → expansion → unanimous-drop judgment over every unsettled container, as many sweeps as there
//!     is work. No quota policy: when the free-model quota runs out the calls fail and the sweep simply retries later (operator 2026-11-08: "it runs
//!     out as it runs out").
//! Both run as loop actors with a trace and an `Origin::actor`, so everything they write is attributable.

use crate::config::{AvailableModel, ModelFallbackConfig};
use crate::keyword_program::{self, Origin};
use crate::orchestrator::run_spans::PassStats;
use crate::orchestrator::{PipelineExecutor, StoreAccess};
use std::sync::Arc;
use std::time::Duration;

fn env_secs(name: &str, default: u64) -> u64 {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).filter(|n| *n > 0).unwrap_or(default)
}

/// Rollup + typed-scope backfill + continuous training, every `OZONE_KEYWORD_HOUSEKEEPING_SECS` (default 300).
pub async fn run_keyword_housekeeping_loop(executor: Arc<dyn PipelineExecutor>, store: Arc<dyn StoreAccess>) {
    let every = env_secs("OZONE_KEYWORD_HOUSEKEEPING_SECS", 300);
    tracing::info!(every_secs = every, "keyword housekeeping loop: started (rollup, scope backfill, continuous training)");
    // let the host finish booting before the first cycle
    tokio::time::sleep(Duration::from_secs(30)).await;
    loop {
        let ctx = crate::trace::TraceCtx::new_root("loop-pass");
        let (executor, store) = (executor.clone(), store.clone());
        crate::trace::with_trace(
            ctx.clone(),
            keyword_program::with_origin(Origin::actor("keyword-housekeeping"), async move {
                let mut stats = PassStats::default();
                let rolled = keyword_program::rollup_once(&store).await;
                if rolled.events > 0 {
                    stats.candidates_consumed += rolled.containers_updated as u64;
                    stats.notes.push(format!("rollup: {} events into {} containers ({} failed)", rolled.events, rolled.containers_updated, rolled.failures));
                }
                crate::orchestrator::keyword_settle::backfill_scope_once(&store, &mut stats).await;
                crate::orchestrator::keyword_settle::scrub_legacy_hotness_once(&store, &mut stats).await;
                match keyword_program::train_continuously(&executor, &store).await {
                    Ok(report) => {
                        if report.get("success").and_then(|s| s.as_bool()) == Some(true) {
                            stats.notes.push(format!(
                                "training (continuous): {}",
                                report.pointer("/report/message").and_then(|m| m.as_str()).unwrap_or("done")
                            ));
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "keyword housekeeping: training cycle failed — it tries again next cycle");
                        stats.notes.push(format!("training cycle failed: {e}"));
                    }
                }
                if stats.did_work() || !stats.notes.is_empty() {
                    tracing::info!(notes = ?stats.notes, "keyword housekeeping: cycle complete");
                }
            }),
        )
        .await;
        tokio::time::sleep(Duration::from_secs(every)).await;
    }
}

/// Settlement sweeps. A sweep that settled something is followed quickly by another (there may be more); an idle sweep waits
/// `OZONE_KEYWORD_SETTLE_IDLE_SECS` (default 600) before looking again.
pub async fn run_keyword_settlement_loop(
    executor: Arc<dyn PipelineExecutor>,
    store: Arc<dyn StoreAccess>,
    available_models: Vec<AvailableModel>,
    meta_fallback: ModelFallbackConfig,
) {
    let idle = env_secs("OZONE_KEYWORD_SETTLE_IDLE_SECS", 600);
    tracing::info!(idle_secs = idle, "keyword settlement loop: started");
    tokio::time::sleep(Duration::from_secs(60)).await;
    loop {
        let ctx = crate::trace::TraceCtx::new_root("loop-pass");
        let (executor, store, models, fallback) = (executor.clone(), store.clone(), available_models.clone(), meta_fallback.clone());
        let did = crate::trace::with_trace(
            ctx,
            keyword_program::with_origin(Origin::actor("keyword-settlement"), async move {
                let mut stats = PassStats::default();
                crate::orchestrator::keyword_settle::settle_once(&executor, &store, &models, &fallback, &mut stats).await;
                tracing::info!(considered = stats.candidates_consumed, calls = stats.calls_made, failures = stats.failures, "keyword settlement: sweep complete");
                stats.successes > 0
            }),
        )
        .await;
        tokio::time::sleep(Duration::from_secs(if did { 30 } else { idle })).await;
    }
}
