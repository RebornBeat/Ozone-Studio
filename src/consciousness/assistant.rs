//! Personal Assistant — the consciousness's operator-facing voice
//! (docs/PERSONAL_ASSISTANT_GUIDE.md §4).
//!
//! Identity (operator directive, 2026-09-30): not a flat tool, not an MCP —
//! an internal pipeline, part of consciousness: "monitoring all and
//! responding in a global or per-project format upon review as it comes
//! through the monitor." Every other loop watches something inward (amt_loop
//! watches graph-vs-AMT alignment, review watches capture-store failures,
//! meta_loop watches methodology, actors watch ripple matches) — this loop
//! watches the ORDER and speaks outward: overdue, stalled, paused-too-long,
//! meetings coming due, check-ups firing, slippage cascades.
//!
//! Budget discipline (the guide's stress gate, directive #43): the pass
//! itself is a FREE local filter over the task store — the seven trigger
//! classes are all computable from real fields (kind/due_at/remind_at/
//! progress/steps), zero model cost. Only when something actually fires does
//! the loop make ONE batched pipeline-9 call to compose the prose digest
//! (N findings, one call — never one call per finding). An idle system costs
//! zero LLM calls from this loop; a maximally noisy day stays within one
//! call per pass. On LLM failure the raw findings persist anyway — loud
//! degradation, never fabricated polish.
//!
//! Interval: same clamp idiom as `i_loop.rs` — the configured interval is
//! honored but never below MIN_INTERVAL_SECS (1800s), because the
//! documented-minute-level defaults are budget-incompatible. Event wake:
//! `notify_wake()` is called from the single task-ripple site
//! (`src/task/mod.rs emit_task_ripple`) so a task completing or slipping
//! wakes the pass early — the interval is the fallback, not the gate
//! (amt_loop's exact convention).
//!
//! Output: ONE "assistant-check-up" insight container per firing pass via
//! the SAME `persist_insight` path the review pass uses (content via
//! object_store_path — reachable through GetContainerContent), plus the
//! derived `/assistant/feed` read route (grpc) computed on read by the SAME
//! `compute_findings` fn — one canonical implementation, no second store.

use crate::config::{AvailableModel, ModelFallbackConfig};
use crate::orchestrator::{ModelConfigOverride, PipelineExecutor, StoreAccess};
use crate::task::TaskData;
use std::sync::Arc;

const PROMPT_PIPELINE_ID: u64 = 9;

/// Real safety floor — 1800s (30 min) matches amt_loop/meta_loop/i_loop's
/// proven interval exactly. Never below, whatever config says.
const MIN_INTERVAL_SECS: u64 = 1800;

/// Wake signal — poked by emit_task_ripple so real task events wake the
/// pass instantly; the interval is the fallback (amt_loop's convention).
static ASSISTANT_WAKE: std::sync::OnceLock<tokio::sync::Notify> = std::sync::OnceLock::new();

fn wake() -> &'static tokio::sync::Notify {
    ASSISTANT_WAKE.get_or_init(tokio::sync::Notify::new)
}

/// Wake the assistant check-up loop immediately — called from
/// emit_task_ripple (the one real task-lifecycle ripple site).
pub fn notify_wake() {
    wake().notify_one();
}

const DAY: u64 = 86_400;
const HOUR: u64 = 3_600;
/// A meeting whose remind/due is inside this horizon is speakable now.
const MEETING_WINDOW: u64 = 2 * HOUR;
/// Live work whose LAST real step activity is older than this is stalled.
const STALL_AFTER: u64 = DAY;
/// Paused work older than this is worth a check-up nudge.
const PAUSED_TOO_LONG: u64 = 3 * DAY;
/// Overdue beyond this is urgent (a full day past due, not minutes).
const OVERDUE_URGENT: u64 = DAY;
/// This many overdue items in one project is a cascade, not a coincidence.
const CASCADE_MIN: usize = 3;

/// One speakable finding. Every field traces to a real source — the
/// no-fabricated-anything doctrine applies to prose, not just numbers.
#[derive(Debug, Clone)]
pub struct AssistantFinding {
    /// overdue | due-soon | meeting-soon | check-up-due | stalled |
    /// paused-too-long | slippage-cascade
    pub class: &'static str,
    /// 3 = act now, 2 = today, 1 = worth knowing.
    pub severity: u8,
    pub title: String,
    pub detail: String,
    pub task_id: u64,
    pub project_id: Option<u64>,
    pub workspace_id: Option<u64>,
    pub due_at: Option<u64>,
}

impl AssistantFinding {
    pub(crate) fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "class": self.class,
            "severity": self.severity,
            "title": self.title,
            "detail": self.detail,
            "task_id": self.task_id,
            "project_id": self.project_id,
            "workspace_id": self.workspace_id,
            "due_at": self.due_at,
        })
    }
}

fn is_active(status: &str) -> bool {
    matches!(status, "running" | "queued" | "interrupted" | "paused")
}

/// The task's last REAL activity timestamp: the newest step start/complete,
/// falling back to started_at, falling back to created_at. No guesses.
fn last_activity(t: &TaskData) -> u64 {
    let step_ts = t
        .steps
        .iter()
        .filter_map(|s| s.started_at.or(s.completed_at))
        .max();
    step_ts.or(t.started_at).unwrap_or(t.created_at)
}

fn item_name(t: &TaskData) -> String {
    let inputs = t.inputs.as_ref();
    inputs
        .and_then(|i| i.get("name"))
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| {
            inputs
                .and_then(|i| i.get("prompt"))
                .and_then(|v| v.as_str())
                // Whole prompt: the cut here silently shortened what the
                // digest model reads about the task. No fixed size cap.
                .map(String::from)
        })
        .unwrap_or_else(|| format!("Task {}", t.task_id))
}

/// The FREE local significance filter — pure function, no I/O, no model.
/// The same fn backs both the derived `/assistant/feed` route and the
/// loop's pass, so the feed can never disagree with what the loop spoke.
pub fn compute_findings(tasks: &[TaskData], now: u64) -> Vec<AssistantFinding> {
    let mut out = Vec::new();

    // Per-item classes.
    for t in tasks {
        if !is_active(&t.status) {
            continue;
        }
        let name = item_name(t);

        // Meetings speak inside a 2h horizon; everything else at its due.
        if t.kind == "meeting" {
            let at = t.remind_at.or(t.due_at);
            if let Some(at) = at {
                if at > now && at <= now + MEETING_WINDOW {
                    out.push(AssistantFinding {
                        class: "meeting-soon",
                        severity: 3,
                        title: format!("Meeting soon: {}", name),
                        detail: format!(
                            "meeting \"{}\" (task {}) starts within {} h",
                            name,
                            t.task_id,
                            (at - now) / HOUR.max(1)
                        ),
                        task_id: t.task_id,
                        project_id: t.project_id,
                        workspace_id: t.workspace_id,
                        due_at: t.due_at,
                    });
                }
            }
        }

        if let Some(due) = t.due_at {
            if due < now {
                let urgent = now - due >= OVERDUE_URGENT;
                out.push(AssistantFinding {
                    class: "overdue",
                    severity: if urgent { 3 } else { 2 },
                    title: format!("Overdue: {}", name),
                    detail: format!(
                        "\"{}\" (task {}) was due {} h ago and is still {}",
                        name,
                        t.task_id,
                        (now - due) / HOUR.max(1),
                        t.status
                    ),
                    task_id: t.task_id,
                    project_id: t.project_id,
                    workspace_id: t.workspace_id,
                    due_at: t.due_at,
                });
            } else if due <= now + DAY && t.kind != "meeting" {
                out.push(AssistantFinding {
                    class: "due-soon",
                    severity: 2,
                    title: format!("Due within 24h: {}", name),
                    detail: format!("\"{}\" (task {}) is due within 24 h", name, t.task_id),
                    task_id: t.task_id,
                    project_id: t.project_id,
                    workspace_id: t.workspace_id,
                    due_at: t.due_at,
                });
            }
        }

        // A fired reminder on unfinished work is a check-up that came due.
        if let Some(rem) = t.remind_at {
            if rem <= now && t.kind != "meeting" {
                out.push(AssistantFinding {
                    class: "check-up-due",
                    severity: 2,
                    title: format!("Check-up due: {}", name),
                    detail: format!(
                        "\"{}\" (task {}) asked to be revisited at {} and is still {}",
                        name, t.task_id, rem, t.status
                    ),
                    task_id: t.task_id,
                    project_id: t.project_id,
                    workspace_id: t.workspace_id,
                    due_at: t.due_at,
                });
            }
        }

        // Stalled: live work whose last real step activity went quiet.
        if matches!(t.status.as_str(), "running" | "queued") {
            let quiet_for = now.saturating_sub(last_activity(t));
            if quiet_for >= STALL_AFTER {
                out.push(AssistantFinding {
                    class: "stalled",
                    severity: 1,
                    title: format!("Stalled: {}", name),
                    detail: format!(
                        "\"{}\" (task {}) is {} but no step activity for {} h (progress {:.0}%)",
                        name,
                        t.task_id,
                        t.status,
                        quiet_for / HOUR.max(1),
                        t.progress * 100.0
                    ),
                    task_id: t.task_id,
                    project_id: t.project_id,
                    workspace_id: t.workspace_id,
                    due_at: t.due_at,
                });
            }
        }

        // Paused too long.
        if t.status == "paused" {
            let quiet_for = now.saturating_sub(last_activity(t));
            if quiet_for >= PAUSED_TOO_LONG {
                out.push(AssistantFinding {
                    class: "paused-too-long",
                    severity: 1,
                    title: format!("Paused {}+ days: {}", PAUSED_TOO_LONG / DAY, name),
                    detail: format!(
                        "\"{}\" (task {}) has been paused for {} days",
                        name,
                        t.task_id,
                        quiet_for / DAY.max(1)
                    ),
                    task_id: t.task_id,
                    project_id: t.project_id,
                    workspace_id: t.workspace_id,
                    due_at: t.due_at,
                });
            }
        }
    }

    // Slippage cascade: >= CASCADE_MIN overdue items anchored to one
    // project — the plan for that project needs a look, said once, not
    // once per item (the per-item overdue findings above still list them).
    // Collect owned data first so the borrow on `out` ends before the
    // cascade findings are pushed onto it.
    let mut by_project: std::collections::HashMap<u64, Vec<(u64, Option<u64>)>> =
        std::collections::HashMap::new();
    for f in out.iter().filter(|f| f.class == "overdue") {
        if let Some(p) = f.project_id {
            by_project.entry(p).or_default().push((f.task_id, f.workspace_id));
        }
    }
    for (project_id, group) in by_project {
        if group.len() >= CASCADE_MIN {
            let ids: Vec<String> = group.iter().map(|(id, _)| format!("{}", id)).collect();
            out.push(AssistantFinding {
                class: "slippage-cascade",
                severity: 3,
                title: format!("Slippage cascade in project {}: {} overdue", project_id, group.len()),
                detail: format!(
                    "project {} has {} overdue items (tasks: {}) — the plan likely needs re-cutting",
                    project_id,
                    group.len(),
                    ids.join(", ")
                ),
                task_id: group[0].0,
                project_id: Some(project_id),
                workspace_id: group[0].1,
                due_at: None,
            });
        }
    }

    // Loudest first, then by due pressure. Stable order for the feed.
    out.sort_by(|a, b| b.severity.cmp(&a.severity).then(a.task_id.cmp(&b.task_id)));
    out
}

/// One assistant pass: free filter → one batched digest call → one insight
/// container. Returns what it persisted (0 or 1 container id).
async fn run_one_check_up(
    executor: &Arc<dyn PipelineExecutor>,
    store: &Arc<dyn StoreAccess>,
    tasks: &Arc<tokio::sync::RwLock<crate::task::TaskManager>>,
    available_models: &[AvailableModel],
    meta_fallback: &ModelFallbackConfig,
    last_fire: &mut Option<u64>,
) -> Option<u64> {
    let now = now_secs();
    // Dedup against our own last firing — one check-up per interval max,
    // even if woken repeatedly by rapid task events.
    if let Some(last) = last_fire {
        if now.saturating_sub(*last) < MIN_INTERVAL_SECS {
            return None;
        }
    }

    let all = tasks.read().await.list_tasks(None, None, 10_000, 0).await;
    let findings = compute_findings(&all, now);
    if findings.is_empty() {
        return None; // free, silent
    }
    *last_fire = Some(now);

    // ── ONE batched digest call (the only LLM cost in the whole loop) ──
    // CONTEXT BUDGET (docs/CONTEXT_OBJECT_MODEL.md — window-aware, every cut
    // recorded): the prompt is three sections in the original layout, so the
    // all-fits output is byte-identical to the old format!; the findings sit
    // between the scaffolds at priority 5, so on overflow THEY trim in place
    // (header/tail are protected scaffold). Budget = half the META model's
    // registered window, floored at today's working size — never a
    // regression on small windows.
    let meta_model = crate::orchestrator::PromptOrchestrator::resolve_meta_model(
        &available_models,
        &meta_fallback,
    );
    let meta_window = meta_model.as_ref().map(|m| m.context_length).unwrap_or(8192) as usize;
    let digest_budget = (meta_window / 2).max(2_000);
    let findings_block = findings
        .iter()
        .map(|f| format!("- [{}] {}", f.class, f.title))
        .collect::<Vec<_>>()
        .join("\n");
    let assembled_findings = {
        let a = crate::context_budget::assemble(
            vec![
                crate::context_budget::ContextSection::new("scaffold-head", 0, format!(
                    "This is the personal assistant's periodic check-up, not a response to a user request.\n\nREAL FINDINGS from the operator's task order (all machine-computed, none guessed):\n"
                )),
                crate::context_budget::ContextSection::new("findings", 5, findings_block.clone()),
                crate::context_budget::ContextSection::new("scaffold-tail", 0, "\n\nIn 3-5 sentences, write the operator a brief, honest check-up note: what needs attention first and why, anything these findings suggest about the day, no filler, no manufactured urgency beyond what the findings themselves carry.\n\nReturn ONLY valid JSON:\n{{\"digest\": \"your 3-5 sentence check-up note\"}}".replace("{{", "{").replace("}}", "}")),
            ],
            digest_budget,
        );
        if !a.trims.is_empty() {
            tracing::warn!(
                trims = a.trims.len(),
                budget_tokens = digest_budget,
                meta_window,
                "assistant digest: findings trimmed to the meta model's window (recorded, markers emitted)"
            );
        }
        a.text
    };
    let prompt = assembled_findings;

    let mut input = serde_json::json!({
        "prompt": prompt,
        "max_tokens": 350,
        "temperature": 0.3,
        "system_prompt": "Personal assistant check-up. Honest, specific, brief. Return only valid JSON."
    });
    // Free/local model for detached meta work — the same meta_fallback
    // convention amt_loop/meta_loop/i_loop use (never the user-facing chain).
    // Chain-order resolution (quality AND speed): openrouter/free first,
    // BitNet as offline backstop — was `find(is_free)` = always BitNet.
    if let Some(default_model) = meta_model.clone() {
        let _ = meta_fallback; // convention parity: same resolution order as i_loop
        let override_cfg = ModelConfigOverride {
            model_type: Some(default_model.model_type.clone()),
            model_identifier: Some(default_model.identifier.clone()),
            max_tokens: None,
            temperature: None,
            context_length: Some(default_model.context_length as u32),
            api_endpoint: default_model.api_endpoint.clone(),
            api_key_env: default_model.api_key_env.clone(),
            api_key: default_model.api_key.clone(),
            wire_protocol: default_model.wire_protocol.clone(),
            bitnet_cli_path: default_model.bitnet_cli_path.clone(),
            local_model_path: default_model.local_model_path.clone(),
        };
        if let Ok(v) = serde_json::to_value(&override_cfg) {
            input["model_override_config"] = v;
        }
    }

    let digest_result = executor.execute(PROMPT_PIPELINE_ID, input).await;
    crate::orchestrator::PromptOrchestrator::capture_loop_model_call("assistant_check_up", &prompt, &digest_result);
    let digest = match digest_result {
        Ok(result) => {
            let response = result.get("response").and_then(|r| r.as_str()).unwrap_or("");
            extract_digest(response).unwrap_or_else(|| {
                format!(
                    "(digest composition returned no parseable JSON — raw findings below)\n{}",
                    findings_block
                )
            })
        }
        Err(e) => {
            // Loud degradation: the findings still speak, honestly labeled.
            tracing::warn!(error = %e, "assistant: digest call failed — persisting raw findings");
            format!(
                "(digest model call failed: {} — raw findings below)\n{}",
                e, findings_block
            )
        }
    };

    let mut content = digest;
    content.push_str("\n\nFindings (machine-computed from the task order):\n");
    content.push_str(&findings_block);
    // CONSCIOUSNESS RESPONSE DIFFERENTIATION (docs/VOICE_BOX_GUIDE.md §1):
    // this is a consciousness-generated response, not an orchestrator
    // answer — the persisted record carries its emotional coloring (real
    // ConsciousnessStore state, never fabricated) and its channel so chat,
    // history, and the future voice pipeline can always tell what kind of
    // response this is and what state produced it.
    let emotion = crate::consciousness::store::get_current_emotional_state();
    content.push_str(&format!(
        "\n\n[consciousness-response: type=consciousness channel=text emotion={} valence={:.2} arousal={:.2} dominance={:.2} findings={}]",
        emotion.primary_emotion, emotion.valence, emotion.arousal, emotion.dominance, findings.len()
    ));
    let citations: Vec<String> = findings
        .iter()
        .map(|f| {
            format!(
                "task {} [{}]{}{}",
                f.task_id,
                f.class,
                f.project_id.map(|p| format!(" proj:{}", p)).unwrap_or_default(),
                f.due_at.map(|d| format!(" due:{}", d)).unwrap_or_default()
            )
        })
        .collect();

    match crate::consciousness::review::persist_insight(
        store,
        "assistant-check-up",
        &content,
        &citations,
    )
    .await
    {
        Some(id) => {
            tracing::info!(container_id = id, findings = findings.len(), "assistant: check-up insight persisted");
            Some(id)
        }
        None => None,
    }
}

/// Same tolerant-brace-scan idiom as i_loop's extract_reflection: find the
/// first parseable JSON object with a "digest" string anywhere in the
/// response (models decorate; the scan doesn't care).
fn extract_digest(response: &str) -> Option<String> {
    let bytes = response.as_bytes();
    let mut scan = 0usize;
    while scan < bytes.len() {
        if bytes[scan] != b'{' {
            scan += 1;
            continue;
        }
        let mut depth = 0usize;
        let mut in_string = false;
        let mut escaped = false;
        let mut end = None;
        for (i, &b) in bytes.iter().enumerate().skip(scan) {
            if escaped {
                escaped = false;
                continue;
            }
            match b {
                b'\\' if in_string => escaped = true,
                b'"' => in_string = !in_string,
                b'{' if !in_string => depth += 1,
                b'}' if !in_string => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        end = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        if let Some(end) = end {
            let candidate = &response[scan..=end];
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(candidate) {
                if let Some(d) = v.get("digest").and_then(|d| d.as_str()) {
                    if !d.trim().is_empty() {
                        return Some(d.trim().to_string());
                    }
                }
            }
            scan = end + 1;
        } else {
            break;
        }
    }
    None
}

/// Boot-spawned check-up loop — joins the lib.rs boot block, gated on the
/// SAME `consciousness.enabled` switch as the I-Loop (there are not two
/// self-awareness toggles).
pub async fn run_assistant_loop(
    executor: Arc<dyn PipelineExecutor>,
    store: Arc<dyn StoreAccess>,
    tasks: Arc<tokio::sync::RwLock<crate::task::TaskManager>>,
    enabled: bool,
    configured_interval_ms: u64,
    available_models: Vec<AvailableModel>,
    meta_fallback: ModelFallbackConfig,
) {
    if !enabled {
        tracing::info!("Assistant loop disabled (ConsciousnessConfig.enabled = false) — /assistant/feed stays available as a derived read");
        return;
    }
    let configured_secs = configured_interval_ms / 1000;
    let interval_secs = configured_secs.max(MIN_INTERVAL_SECS);
    if configured_secs < MIN_INTERVAL_SECS {
        tracing::warn!(
            configured_secs,
            clamped_to = interval_secs,
            "assistant: configured assistant_interval_ms is below the real budget-safety floor — clamped"
        );
    }
    tracing::info!(interval_secs, "Assistant check-up loop starting");
    let mut ticker = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
    let mut last_fire: Option<u64> = None;
    loop {
        tokio::select! {
            _ = ticker.tick() => {}
            _ = wake().notified() => {}
        }
        // run_one_check_up traces its own outcomes (persisted id, empty
        // pass, digest failure) — nothing to add at this layer.
        let _ = run_one_check_up(
            &executor,
            &store,
            &tasks,
            &available_models,
            &meta_fallback,
            &mut last_fire,
        )
        .await;
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: u64, status: &str, kind: &str, due: Option<u64>, remind: Option<u64>) -> TaskData {
        TaskData {
            task_id: id,
            blueprint_id: None,
            status: status.into(),
            progress: 0.5,
            created_at: now_secs() - DAY,
            started_at: Some(now_secs() - DAY),
            completed_at: None,
            user_id: 0,
            device_id: 0,
            workspace_id: Some(1),
            project_id: Some(3),
            parent_task_id: None,
            child_count: 0,
            error: None,
            inputs: Some(serde_json::json!({"name": format!("item {}", id)})),
            outputs: None,
            steps: vec![],
            total_tokens: 0,
            gate_result: None,
            thinking_log: vec![],
            amt_summary: None,
            kind: kind.into(),
            due_at: due,
            remind_at: remind,
            recurrence: None,
            meeting_url: None,
            external_ref: None,
            checklist: vec![],
            note_body: None,
        }
    }

    #[test]
    fn overdue_fires_and_escalates() {
        let now = now_secs();
        let t = task(1, "running", "todo", Some(now - 2 * HOUR), None);
        let f = compute_findings(&[t], now);
        assert!(f.iter().any(|x| x.class == "overdue" && x.severity == 2));
        let t = task(2, "running", "todo", Some(now - 2 * DAY), None);
        let f = compute_findings(&[t], now);
        assert!(f.iter().any(|x| x.class == "overdue" && x.severity == 3));
    }

    #[test]
    fn done_items_never_fire() {
        let now = now_secs();
        let t = task(3, "completed", "todo", Some(now - 5 * DAY), None);
        assert!(compute_findings(&[t], now).is_empty());
    }

    #[test]
    fn meeting_speaks_only_inside_window() {
        let now = now_secs();
        let soon = task(4, "queued", "meeting", Some(now + HOUR), Some(now + HOUR));
        let f = compute_findings(&[soon], now);
        assert!(f.iter().any(|x| x.class == "meeting-soon" && x.severity == 3));
        let far = task(5, "queued", "meeting", Some(now + 5 * HOUR), Some(now + 5 * HOUR));
        assert!(!compute_findings(&[far], now).iter().any(|x| x.class == "meeting-soon"));
    }

    #[test]
    fn check_up_due_on_fired_reminder() {
        let now = now_secs();
        let t = task(6, "running", "todo", None, Some(now - HOUR));
        let f = compute_findings(&[t], now);
        assert!(f.iter().any(|x| x.class == "check-up-due"));
    }

    #[test]
    fn stalled_needs_quiet_steps() {
        let now = now_secs();
        let mut t = task(7, "running", "code", None, None);
        t.steps = vec![crate::task::TaskStepData {
            step_index: 0,
            action: "do".into(),
            pipeline_id: 9,
            status: "completed".into(),
            started_at: Some(now - 2 * DAY),
            completed_at: Some(now - 2 * DAY),
            tokens_used: 0,
            output_summary: None,
            error: None,
            stages_completed: vec![],
            stages_pending: vec![],
            current_stage: None,
            graph_ids_read: vec![],
            graph_ids_updated: vec![],
            version: 1,
            version_notes: vec![],
            methodology_ids_applied: vec![],
            model_used: None,
            context_assembled: None,
            context_sources: vec![],
        }];
        let f = compute_findings(&[t], now);
        assert!(f.iter().any(|x| x.class == "stalled"));
    }

    #[test]
    fn cascade_needs_three_in_one_project() {
        let now = now_secs();
        let tasks: Vec<TaskData> = (10..13)
            .map(|i| task(i, "queued", "todo", Some(now - 2 * HOUR), None))
            .collect();
        let f = compute_findings(&tasks, now);
        assert!(f.iter().any(|x| x.class == "slippage-cascade" && x.severity == 3));
        // Two in one project is not a cascade.
        let two: Vec<TaskData> = (20..22)
            .map(|i| task(i, "queued", "todo", Some(now - 2 * HOUR), None))
            .collect();
        assert!(!compute_findings(&two, now).iter().any(|x| x.class == "slippage-cascade"));
    }

    #[test]
    fn paused_too_long() {
        let now = now_secs();
        // Threshold is 3 days quiet — the fixture must be older than that.
        let mut t = task(30, "paused", "todo", None, None);
        t.created_at = now - 5 * DAY;
        t.started_at = Some(now - 5 * DAY);
        let f = compute_findings(&[t], now);
        assert!(f.iter().any(|x| x.class == "paused-too-long"));
        // A fresh pause is not "too long".
        let fresh = task(31, "paused", "todo", None, None);
        assert!(!compute_findings(&[fresh], now).iter().any(|x| x.class == "paused-too-long"));
    }

    #[test]
    fn digest_extraction_tolerates_decorated_json() {
        let r = "Sure! {\"digest\": \"three things need you today.\"} hope that helps";
        assert_eq!(extract_digest(r).as_deref(), Some("three things need you today."));
        assert_eq!(extract_digest("no json here"), None);
    }
}


// staleness-actor live-fire probe

// staleness probe 2

// staleness probe 4

// staleness probe 5
