//! WORK-ORDER TREE — the universal checklist on the existing task records (operator 2026-11-10; docs/UNIVERSAL_ORDER_GUIDE.md).
//!
//! WHAT: while a walk runs, its structure is recorded as a tree of `wo_*` tasks (`task/mod.rs`, "WORK-ORDER TREE"):
//!   walk task ── `wo_branch` per AMT node (nested like the AMT) ── `wo_step` per blueprint step (attached to the deepest AMT branch whose
//!   source chunks it covers, else to the walk) ── and, for every capability gap the walk reported, a `wo_capability` task with its own
//!   checklist.
//! THE ONE RULE: a task is complete exactly when all its checklist items are done and all its children are complete — derived by
//!   `TaskManager::settle`, never a separately-set flag. COMPLETED IS FINAL (nothing reopens; a rerun is a NEW task via `rerun_of`; a plan
//!   edit is a graft = new items / new children). The tree GROWS WITH THE AMT: `materialize_work_order` is idempotent on walk-scoped keys, so
//!   calling it again after the AMT expanded adds only the new nodes under their parents, and an expanded branch gates its parent.
//! ITEMS ARE TICKED ONLY BY THE REAL EVENT THAT COMPLETES THEM:
//!   step `run` ← the step executed successfully (`work_order_step_done`);
//!   gap `judge` ← closure recorded its yes/no verdict · `methodology` ← a methodology was drafted for it · `replan` ← the plan was rebuilt
//!   from the richer set (`work_order_after_replan`) · `register` ← the provider appeared in the MCP registry (`satisfied_at`) ·
//!   `verify` ← a walk that carried the closed capability completed (`work_order_walk_done`);
//!   `tool` ← the tool was CREATED (`tool_created`, recorded by `tool_creation` — the closure creates the missing tool on the identified need,
//!   automatically, operator 2026-11-10) · `register` ← it was registered in the MCP registry (`tool_registered`). A provider that appears in the
//!   registry by any OTHER route (`satisfied_at` with no `tool_created` event) ticks both too, with the evidence worded accordingly.
//!   The tool is published as a FINDING (like a created methodology) — there is no "offer" step: gaps are not filled from marketplace offers.
//! A gap judged NOT needed gets only the `judge` item and completes with it; the remaining items are GRAFTED when the verdict is "necessary".

use super::tool_binding::{capability_gaps, link_gap_task, note_gap_event};
use super::{AMTNode, OrchestrationState, PromptOrchestrator};
use crate::task::TaskManager;
use crate::types::TaskID;

/// The items a NECESSARY capability gap grafts after `judge` (key, text).
const GAP_ITEMS: [(&str, &str); 5] = [
    ("methodology", "a methodology that achieves the capability is drafted"),
    ("tool", "the tool / provider is created for the capability (automatic on the identified need; artifact + origin recorded)"),
    ("register", "the provider is registered in the MCP registry"),
    ("replan", "the plan is rebuilt from the richer methodology set"),
    ("verify", "a walk that carried the closed capability completed"),
];

fn row_of(name: &str) -> Option<serde_json::Value> {
    capability_gaps().into_iter().find(|r| r["name"].as_str().map(|n| n.eq_ignore_ascii_case(name.trim())) == Some(true))
}

/// The first history entry with this status that happened AT OR AFTER `since` (0 for a first task; for a rerun, just after the task it re-runs
/// completed), so a rerun never ticks an item from an earlier task's events.
fn resolution<'a>(row: &'a serde_json::Value, status: &str, since: u64) -> Option<&'a serde_json::Value> {
    row["resolutions"].as_array().and_then(|a| a.iter().find(|x| x["status"].as_str() == Some(status) && x["at"].as_u64().unwrap_or(0) >= since))
}

/// Tick (and graft) one gap task's items from the gap row's REAL history. Idempotent; a completed task is final and is left alone.
async fn sync_one(tm: &TaskManager, task_id: TaskID, row: &serde_json::Value) {
    // Which events belong to THIS task: a first task takes the gap's whole history (the walk's closure ran before the task existed); a RERUN
    // task takes only events after the task it re-runs completed — it never ticks an item from an earlier task's events.
    let since = match tm.work_order(task_id).await.and_then(|n| n["rerun_of"].as_u64()) {
        Some(old) => tm.work_order(old).await.and_then(|n| n["completed_at"].as_u64()).map(|c| c + 1).unwrap_or(0),
        None => 0,
    };
    // GRAFT FIRST: a verdict of "necessary" adds the remaining items BEFORE `judge` is ticked — a task whose only item is done would complete
    // (final) and could no longer take them. (Err = the task already completed: a new need starts a rerun task via `ensure_gap_task`.)
    let necessary = resolution(row, "methodology_drafted", since).is_some() || resolution(row, "wanted", since).is_some();
    if necessary {
        for (key, text) in GAP_ITEMS {
            let _ = tm.graft_item(task_id, key, text).await;
        }
    }
    if let Some(j) = ["not_needed", "methodology_drafted", "wanted"].iter().find_map(|s| resolution(row, s, since)) {
        tm.complete_item(task_id, "judge", j["reason"].as_str().unwrap_or(""), j["by"].as_str().unwrap_or("closure")).await;
    }
    if let Some(m) = resolution(row, "methodology_drafted", since) {
        tm.complete_item(task_id, "methodology", m["reason"].as_str().unwrap_or(""), m["by"].as_str().unwrap_or("closure")).await;
    }
    // the tool this system CREATED for the gap: its own two events carry the exact evidence
    if let Some(t) = resolution(row, "tool_created", since) {
        tm.complete_item(task_id, "tool", t["reason"].as_str().unwrap_or(""), t["by"].as_str().unwrap_or("closure")).await;
    }
    if let Some(t) = resolution(row, "tool_registered", since) {
        tm.complete_item(task_id, "register", t["reason"].as_str().unwrap_or(""), t["by"].as_str().unwrap_or("closure")).await;
    }
    // a provider that appeared in the registry by ANY other route (a peer-installed tool, an operator registration)
    if let Some(at) = row["satisfied_at"].as_u64().filter(|a| *a >= since) {
        let ev = format!("a provider for this capability is registered (observed at {at}); how it arrived is not recorded here");
        tm.complete_item(task_id, "tool", &ev, "registry").await;
        tm.complete_item(task_id, "register", &ev, "registry").await;
    }
    if let Some(r) = resolution(row, "replanned", since) {
        tm.complete_item(task_id, "replan", r["reason"].as_str().unwrap_or(""), r["by"].as_str().unwrap_or("walk")).await;
    }
    if let Some(v) = resolution(row, "verified", since) {
        tm.complete_item(task_id, "verify", v["reason"].as_str().unwrap_or(""), v["by"].as_str().unwrap_or("walk")).await;
    }
}

/// Re-sync EVERY linked gap task from `capability_gaps.json` (the registry-observed `satisfied_at` can arrive outside any walk).
pub async fn sync_gap_tasks(tm: &TaskManager) {
    for row in capability_gaps() {
        if let Some(id) = row["work_order_task"].as_u64() {
            sync_one(tm, id, &row).await;
        }
    }
}

/// The capability-gap task for `name`: created once; a gap that needs work AGAIN after its task completed (final) gets a NEW task
/// (`rerun_of` = the old one, which stays untouched).
async fn ensure_gap_task(tm: &TaskManager, walk: TaskID, name: &str) {
    let Some(row) = row_of(name) else { return };
    let key = name.trim().to_lowercase();
    let existing = tm.find_work_task_any_parent("wo_capability", &key).await;
    let open_again = !matches!(row["status"].as_str(), Some("satisfied") | Some("not_needed"));
    let completed = match existing {
        Some(id) => tm.work_order(id).await.map(|n| n["status"].as_str() == Some("completed")).unwrap_or(false),
        None => false,
    };
    let id = match existing {
        Some(id) if !(completed && open_again) => id,
        Some(old) => match tm
            .create_work_task(Some(walk), "wo_capability", &key, name, &[("judge".to_string(), "judged: does the request really require this capability? (strength-N yes/no)".to_string())], Some(old), Some(walk))
            .await
        {
            Some(id) => id,
            None => return,
        },
        None => match tm
            .create_work_task(Some(walk), "wo_capability", &key, name, &[("judge".to_string(), "judged: does the request really require this capability? (strength-N yes/no)".to_string())], None, Some(walk))
            .await
        {
            Some(id) => id,
            None => return,
        },
    };
    link_gap_task(name, id);
    sync_one(tm, id, &row).await;
}

impl PromptOrchestrator {
    /// Record / extend this walk's work-order tree. Idempotent on walk-scoped keys: call it again after the AMT expanded or a plan was
    /// grafted and only the NEW nodes are added (under their parents). Needs `state.task_id`.
    pub(crate) async fn materialize_work_order(&self, state: &OrchestrationState) {
        let Some(walk) = state.task_id else { return };
        let tm = self.task_manager.read().await;
        // 1. AMT branches, nested like the AMT (the AMT root is the walk task itself)
        let mut anchors: Vec<(TaskID, Vec<u32>, usize)> = Vec::new(); // (branch task, its source chunks, depth)
        if let Some(amt) = &state.amt {
            let mut stack: Vec<(&AMTNode, TaskID, usize)> = amt.children.iter().rev().map(|c| (c, walk, 1usize)).collect();
            while let Some((n, parent, depth)) = stack.pop() {
                let key = format!("w{walk}:amt:{}", n.id);
                let id = match tm.find_work_task_any_parent("wo_branch", &key).await {
                    Some(id) => id,
                    None => match tm.create_work_task(Some(parent), "wo_branch", &key, &n.content, &[], None, Some(walk)).await {
                        Some(id) => id,
                        None => continue,
                    },
                };
                anchors.push((id, n.source_chunk_indices.clone(), depth));
                for c in n.children.iter().rev() {
                    stack.push((c, id, depth + 1));
                }
            }
        }
        // 2. blueprint steps, under the deepest AMT branch whose source chunks they cover (else the walk)
        for (pos, st) in state.blueprint_steps.iter().enumerate() {
            let key = format!("w{walk}:step:{pos}");
            if tm.find_work_task_any_parent("wo_step", &key).await.is_some() {
                continue;
            }
            let parent = anchors
                .iter()
                .filter(|(_, chunks, _)| !chunks.is_empty() && st.source_chunk_indices.iter().any(|c| chunks.contains(c)))
                .max_by_key(|(_, _, depth)| *depth)
                .map(|(id, _, _)| *id)
                .unwrap_or(walk);
            let title: String = format!("step {pos}: {}", st.description.chars().take(160).collect::<String>());
            tm.create_work_task(Some(parent), "wo_step", &key, &title, &[("run".to_string(), "the step ran to completion".to_string())], None, Some(walk)).await;
        }
        // 3. capability gaps this walk reported (one task per capability, a rerun task when it needs work again after completing)
        let mut seen = std::collections::HashSet::new();
        for m in &state.missing_capabilities {
            if let Some(name) = m["name"].as_str() {
                if seen.insert(name.trim().to_lowercase()) {
                    ensure_gap_task(&tm, walk, name).await;
                }
            }
        }
        sync_gap_tasks(&tm).await;
    }

    /// A step ran to completion: tick its `run` item (the completion cascades up through branch and walk by the one rule).
    pub(crate) async fn work_order_step_done(&self, state: &OrchestrationState, pos: usize, evidence: &str) {
        let Some(walk) = state.task_id else { return };
        let tm = self.task_manager.read().await;
        let key = format!("w{walk}:step:{pos}");
        if let Some(id) = tm.find_work_task_any_parent("wo_step", &key).await {
            tm.complete_item(id, "run", evidence, "walk").await;
        }
    }

    /// The plan was rebuilt after a capability was closed: record `replanned` on every gap whose methodology was drafted (the real event),
    /// then tick.
    pub(crate) async fn work_order_after_replan(&self, state: &OrchestrationState) {
        for row in capability_gaps() {
            if resolution(&row, "methodology_drafted", 0).is_some() {
                if let Some(name) = row["name"].as_str() {
                    note_gap_event(name, "replanned", "the blueprint was rebuilt from the richer methodology set", "walk");
                }
            }
        }
        if state.task_id.is_some() {
            let tm = self.task_manager.read().await;
            sync_gap_tasks(&tm).await;
        }
    }

    /// The walk completed: every gap it carried that was closed (methodology drafted or provider registered) is VERIFIED by this walk.
    pub(crate) async fn work_order_walk_done(&self, state: &OrchestrationState) {
        let Some(walk) = state.task_id else { return };
        for m in &state.missing_capabilities {
            let Some(name) = m["name"].as_str() else { continue };
            if let Some(row) = row_of(name) {
                if resolution(&row, "methodology_drafted", 0).is_some() || row["status"].as_str() == Some("satisfied") {
                    note_gap_event(name, "verified", &format!("walk {walk} carried the closed capability and completed"), "walk");
                }
            }
        }
        let tm = self.task_manager.read().await;
        sync_gap_tasks(&tm).await;
    }
}
