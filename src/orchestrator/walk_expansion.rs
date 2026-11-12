//! WALK EXPANSION — an AMT that grows WHILE the task is open (operator 2026-11-11).
//!
//! RULES THIS ENFORCES (operator): a task completes only when ALL its steps complete; re-expansion can happen while the task is open; completed is
//! FINAL (a rerun is a new task instance). So when a finished step's alignment review reveals a new branch (`on_step_complete` in `mod.rs`), the
//! walk does not note it and move on — before its next step it runs `expand_walk`:
//!   1. the discovered branch becomes a real AMT node (`state.amt.children`), recorded as `amt_expanded_in_walk`;
//!   2. the capabilities the review named for it go through the SAME closure as every other gap (`close_capability_gaps`: strength-N necessity
//!      vote, methodology drafted on the spot, tool created on the identified need, privacy-checked before publish) — once, never re-judged;
//!   3. a step is synthesized for it (`Address: <branch>`, pipeline 9 — the same shape the blueprint's own reconciliation uses), carrying the
//!      methodologies drafted for the gaps it raised, depending on the step that discovered it; APPENDED, so no existing step index or position
//!      (completed steps, work-order keys `w<walk>:step:<position>`) moves;
//!   4. the resolved plan is re-checked by the jurisdiction FULL gate (the plan now contains steps the first check never saw); a block removes
//!      the new steps again and PAUSES the task with the branch still queued — never a silent pass;
//!   5. when new methodologies were drafted, the rule gate judges their rules against the plan (`judge_rule_applicability`);
//!   6. the work-order tree grows: `materialize_work_order` (idempotent) adds the branch node, the step under it, and any new gap checklists.
//! The branch queue (`state.walk_expansion`) is persisted in the run checkpoint, so a pause (or a failure inside this function) never loses a
//! discovered branch; each queue item remembers that its gaps were already closed (`closed`) and which methodologies that produced, so a retry
//! after a pause only does the remaining work. Failure = `Err` → the caller PAUSES the task (failure doctrine), it never continues without it.
//!
//! Stage-4 simulation: re-run by the step loop (`stage_6_to_8_execute_steps`) after a round that added steps (the result replaces the earlier one; it informs, it re-opens nothing);
//! The persisted AMT container is rewritten as a new version after each round (step 5b; `persist_amt_version`). No fixed count bounds the number of expansion rounds — every round consumes discovered branches, a branch is captured once (case-insensitive),
//! and the review that discovers branches runs only after real steps.

use super::{AMTNode, AMTNodeType, BlueprintStep, OrchestrationState, PromptOrchestrator};

/// The description of the step synthesized for a branch — the same `Address: …` form the blueprint's branch reconciliation uses.
pub(crate) fn address_description(branch: &str) -> String {
    format!("Address: {}", branch)
}

impl PromptOrchestrator {
    /// Expand every queued discovered branch into AMT node + closed gaps + step + work-order items. Returns the number of steps added.
    /// `Err` = something required failed (the caller pauses the task; the queue is kept so resume finishes the work).
    pub(crate) async fn expand_walk(&self, state: &mut OrchestrationState) -> Result<usize, String> {
        if state.walk_expansion.is_empty() {
            return Ok(0);
        }
        // 1. AMT nodes (idempotent on node id). The queue item is read first so the loop owns no borrow of `state`.
        let queued: Vec<serde_json::Value> = state.walk_expansion.clone();
        for item in &queued {
            let (Some(branch), Some(node_id)) = (item["branch"].as_str(), item["node_id"].as_u64()) else { continue };
            let discovered_at = item["step"].as_u64().unwrap_or(0);
            let mut added_node = false;
            if let Some(amt) = state.amt.as_mut() {
                if !amt.children.iter().any(|c| c.id == node_id) {
                    let mut node = AMTNode::new(node_id, AMTNodeType::Branch, branch.to_string(), 1);
                    node.metadata.insert("discovered_in_walk_step".to_string(), discovered_at.to_string());
                    amt.children.push(node);
                    added_node = true;
                }
            }
            if added_node {
                super::amt::record_intent_event(
                    state,
                    "amt_expanded_in_walk",
                    serde_json::json!({"branch": branch, "node_id": node_id, "step": discovered_at}),
                );
                // participants (src/walk_feed.rs): the agents following this walk see the AMT grow as it happens
                if let Some(tid) = state.task_id {
                    crate::walk_feed::task_event(
                        tid,
                        "amt_expansion",
                        serde_json::json!({"event": "node_added", "branch": branch, "node_id": node_id, "discovered_in_step": discovered_at}),
                    );
                }
            }
        }

        // 2. the capability closure for the gaps the reviews named (judged once: `closure_attempted`; closed once: the `closed` flag)
        if state.walk_expansion.iter().any(|p| !p["closed"].as_bool().unwrap_or(false)) {
            let before = state.methodologies.len();
            let (_created, _needed) = self.close_capability_gaps(state).await;
            let drafted: Vec<u64> = state.methodologies.get(before..).map(|s| s.to_vec()).unwrap_or_default();
            for p in state.walk_expansion.iter_mut() {
                if !p["closed"].as_bool().unwrap_or(false) {
                    p["closed"] = serde_json::json!(true);
                    p["methodology_ids"] = serde_json::json!(drafted);
                }
            }
        }

        // 3. steps — appended after every existing one
        let prev_len = state.blueprint_steps.len();
        let mut next_index = state.blueprint_steps.iter().map(|s| s.step_index).max().map(|m| m + 1).unwrap_or(0);
        let mut added: Vec<u32> = Vec::new();
        let mut drafted_any = false;
        let queue: Vec<serde_json::Value> = state.walk_expansion.clone();
        for p in &queue {
            let Some(branch) = p["branch"].as_str() else { continue };
            let description = address_description(branch);
            // idempotent: a retry after a pause (or a step the blueprint already made for this branch) never doubles the step
            if state.blueprint_steps.iter().any(|s| s.description.trim().eq_ignore_ascii_case(description.trim())) {
                continue;
            }
            let methodology_ids: Vec<u64> = p["methodology_ids"].as_array().map(|a| a.iter().filter_map(|x| x.as_u64()).collect()).unwrap_or_default();
            drafted_any = drafted_any || !methodology_ids.is_empty();
            let keywords: Vec<String> = branch
                .split_whitespace()
                .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
                .filter(|w| w.len() > 3)
                .collect();
            let depends_on: Vec<u32> = p["step"].as_u64().map(|n| vec![n as u32]).unwrap_or_default();
            state.blueprint_steps.push(BlueprintStep {
                step_index: next_index,
                action: "execute_prompt".to_string(),
                description,
                pipeline_id: 9,
                context_requirements: keywords,
                loop_config: None,
                sub_steps: Vec::new(),
                depends_on,
                wait_for_graph_update: false,
                max_retries: 1,
                timeout_ms: None,
                model_override: None,
                source_chunk_indices: Vec::new(),
                methodology_ids,
                rule_refs: Vec::new(),
                step_type: "pipeline".to_string(),
                provider: None,
                tool: None,
                args: serde_json::Value::Null,
                optional: false,
            });
            added.push(next_index);
            next_index += 1;
        }
        if added.is_empty() {
            state.walk_expansion.clear(); // every queued branch already had its step: nothing left to expand
            return Ok(0);
        }

        // 4. the resolved plan now holds steps the first jurisdiction check never saw: check it again. A block takes the new steps OUT again and
        // leaves the queue intact — the task pauses, and resuming re-enters here and re-runs this check (no way around it).
        if let Err(e) = self.stage_jurisdiction_full_gate(state).await {
            state.blueprint_steps.truncate(prev_len);
            return Err(e);
        }

        // 5. rules of any newly drafted methodology are judged against the plan (the same applicability gate the blueprint's own rules pass)
        if drafted_any {
            self.judge_rule_applicability(state).await;
        }

        // 5b. the AMT changed: persist it as a NEW VERSION of its container (old body file + history kept). BEFORE the queue is consumed, so a
        // failed write pauses the task with the queue intact and the resume writes the version — it is never skipped.
        self.persist_amt_version(state, "walk_expansion").await?;

        // 6. the queue is consumed; the work-order tree grows under the open task
        state.walk_expansion.clear();
        super::amt::record_intent_event(
            state,
            "walk_steps_synthesized",
            serde_json::json!({"steps": added, "from_branches": queue.iter().filter_map(|p| p["branch"].as_str()).collect::<Vec<_>>()}),
        );
        self.materialize_work_order(state).await;
        if drafted_any {
            self.work_order_after_replan(state).await;
        }
        if let Some(tid) = state.task_id {
            crate::walk_feed::task_event(
                tid,
                "amt_expansion",
                serde_json::json!({"event": "steps_synthesized", "steps": added, "methodologies_drafted": drafted_any}),
            );
        }
        Ok(added.len())
    }
}
