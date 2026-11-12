//! Run checkpoints and failure-pause/resume (2026-11-02).
//!
//! FAILURE DOCTRINE: the walk never continues past a failed step. When a step fails the
//! task is PAUSED at that step (status `paused`, error `paused at step N: …`), the
//! failure is surfaced, and the task can be resumed from exactly that step at any time —
//! from the Tasks tab, the project tabs, or the meta chat — with `POST /task/resume_run`.
//! Completed steps are never re-run; their recorded outputs are restored.
//!
//! To make "exactly where it left off" real, the walk writes a checkpoint
//! (`<data>/runs/<task_id>.json`) after task creation and after every completed step. It
//! holds the original request plus everything the step loop reads: the cleaned prompt,
//! keywords/topics, methodology + category ids, graph ids, the blueprint steps, the
//! applicable tools, attached-file contents, and each completed step's output. Resume
//! rebuilds a fresh `OrchestrationState` for that request, overlays the checkpoint, and
//! re-enters the step loop (which skips completed steps), then runs the same result
//! collection / post-execution / delivery stages as a normal run.
//!
//! What is NOT restored (documented, not hidden): the in-memory AMT tree object and the
//! intent/branch capture lists (branches discovered mid-walk and not yet expanded ARE kept: `walk_expansion`) — steps reach the AMT through its container id and the
//! span-out context, and `on_step_complete` alignment reviews degrade to the captured
//! graph state. The checkpoint file is the record; nothing is capped or truncated.

use super::*;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointStep {
    pub step_index: u32,
    pub pipeline_id: u64,
    pub output: serde_json::Value,
    pub tokens_used: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunCheckpoint {
    pub task_id: u64,
    pub request: OrchestrationRequest,
    pub trace_id: Option<String>,
    pub cleaned_prompt: String,
    pub keywords: Vec<String>,
    pub topics: Vec<String>,
    pub methodologies: Vec<u64>,
    pub categories: Vec<u64>,
    pub amt_container_id: Option<u64>,
    pub blueprint_id: Option<u64>,
    pub blueprint_steps: Vec<BlueprintStep>,
    pub modality_graphs: HashMap<String, u64>,
    pub file_graphs: HashMap<String, u64>,
    pub chunk_graph_ids: Vec<u64>,
    pub applicable_tools: Vec<serde_json::Value>,
    /// per-rule provider bindings + missing capabilities (2026-11-04) — a resumed run keeps the same manifest; older checkpoints default to empty
    #[serde(default)]
    pub tool_manifest: Vec<serde_json::Value>,
    #[serde(default)]
    pub missing_capabilities: Vec<serde_json::Value>,
    /// AMT branches discovered mid-walk and not yet expanded into steps (`walk_expansion.rs`); older checkpoints default to empty
    #[serde(default)]
    pub walk_expansion: Vec<serde_json::Value>,
    /// Graph creations that failed during the walk (retry items under the open task); older checkpoints default to empty
    #[serde(default)]
    pub graph_failures: Vec<serde_json::Value>,
    pub attached_file_contents: HashMap<String, String>,
    pub tokens_used_so_far: u32,
    pub completed_steps: Vec<CheckpointStep>,
    pub paused_at_step: Option<u32>,
    pub pause_error: Option<String>,
    pub saved_at_ms: u64,
}

fn runs_dir(data_dir: &str) -> PathBuf {
    let base = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| data_dir.to_string());
    PathBuf::from(base).join("runs")
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl PromptOrchestrator {
    fn checkpoint_path(&self, task_id: u64) -> PathBuf {
        runs_dir(&self.data_dir).join(format!("{task_id}.json"))
    }

    /// Persist the run's resumable state. Best-effort but LOUD: a failed write is
    /// logged as an error (a run that cannot be checkpointed can still finish, but it
    /// cannot be resumed after a failure).
    pub(crate) fn write_checkpoint(&self, state: &OrchestrationState, paused: Option<(u32, &str)>) {
        let Some(task_id) = state.task_id else { return };
        let cp = RunCheckpoint {
            task_id,
            request: state.request.clone(),
            trace_id: state.trace_id.clone(),
            cleaned_prompt: state.cleaned_prompt.clone(),
            keywords: state.keywords.iter().cloned().collect(),
            topics: state.topics.clone(),
            methodologies: state.methodologies.clone(),
            categories: state.categories.clone(),
            amt_container_id: state.amt_container_id,
            blueprint_id: state.blueprint_id,
            blueprint_steps: state.blueprint_steps.clone(),
            modality_graphs: state.modality_graphs.clone(),
            file_graphs: state.file_graphs.clone(),
            chunk_graph_ids: state.chunk_graph_ids.clone(),
            applicable_tools: state.applicable_tools.clone(),
            tool_manifest: state.tool_manifest.clone(),
            missing_capabilities: state.missing_capabilities.clone(),
            walk_expansion: state.walk_expansion.clone(),
            graph_failures: state.graph_failures.clone(),
            attached_file_contents: state.attached_file_contents.clone(),
            tokens_used_so_far: state.tokens_used_so_far,
            completed_steps: state
                .step_results
                .iter()
                .map(|r| CheckpointStep {
                    step_index: r.step_index,
                    pipeline_id: r.pipeline_id,
                    output: r.output.clone(),
                    tokens_used: r.tokens_used,
                })
                .collect(),
            paused_at_step: paused.map(|(i, _)| i),
            pause_error: paused.map(|(_, e)| e.to_string()),
            saved_at_ms: now_ms(),
        };
        let path = self.checkpoint_path(task_id);
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        match serde_json::to_vec_pretty(&cp) {
            Ok(bytes) => {
                // write-then-rename so a crash never leaves a half-written checkpoint
                let tmp = path.with_extension("json.tmp");
                if let Err(e) = std::fs::write(&tmp, bytes).and_then(|_| std::fs::rename(&tmp, &path)) {
                    tracing::error!(task_id, path = %path.display(), error = %e, "run checkpoint write FAILED — this run cannot be resumed after a failure");
                }
            }
            Err(e) => tracing::error!(task_id, error = %e, "run checkpoint serialize FAILED"),
        }
    }

    pub(crate) fn load_checkpoint(&self, task_id: u64) -> Result<RunCheckpoint, String> {
        let path = self.checkpoint_path(task_id);
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| format!("no run checkpoint for task {task_id} ({}): {e}", path.display()))?;
        serde_json::from_str(&raw).map_err(|e| format!("run checkpoint for task {task_id} is unreadable: {e}"))
    }

    /// A step failed: write the checkpoint at that step, PAUSE the task, and return the
    /// error the walk reports. The walk stops here — it does not continue past a failure.
    pub(crate) async fn pause_on_step_error(
        &self,
        state: &OrchestrationState,
        step_index: u32,
        error: &str,
    ) -> String {
        self.write_checkpoint(state, Some((step_index, error)));
        match state.task_id {
            Some(task_id) => {
                let paused = self
                    .task_manager
                    .read()
                    .await
                    .pause_for_error(task_id, step_index, error)
                    .await;
                tracing::warn!(task_id, step_index, paused, error, "step failed — task PAUSED (resume from the Tasks tab / POST /task/resume_run)");
                format!(
                    "PAUSED: task {task_id} paused at step {step_index}: {error}. It was not continued. Resume it from exactly this step from the Tasks tab, the project tab, or the meta chat (POST /task/resume_run)."
                )
            }
            None => format!("step {step_index} failed before a task record existed: {error}"),
        }
    }

    /// Failures that are DECISIONS (a jurisdiction block, a declined gate, an operator hold, a
    /// clarification request, an awaiting/pause marker already in place) are not "failed runs" and
    /// must not be turned into a resumable pause.
    pub(crate) fn is_pausable_failure(error: &str) -> bool {
        const DECISIONS: [&str; 8] = [
            "blocked by jurisdiction",
            "Consciousness gate declined",
            "held for human review",
            "AWAITING:",
            "PAUSED:",
            "Prompt cannot be empty",
            "declined",
            "clarification",
        ];
        !DECISIONS.iter().any(|d| error.contains(d))
    }

    /// FAILURE DOCTRINE for failures BEFORE task creation (stages 2-8: the model chain exhausted,
    /// a store error, ...). There is no task yet, so one is created now, paused with the error, and
    /// checkpointed with the original request; resuming it re-runs the walk from the start (nothing
    /// is lost: the prompt, scope and model config are the checkpoint). Returns the walk's error text.
    pub(crate) async fn pause_pre_task(&self, state: &mut OrchestrationState, error: &str) -> String {
        let mut inputs = std::collections::HashMap::new();
        inputs.insert("prompt".to_string(), serde_json::json!(state.request.prompt));
        inputs.insert("source".to_string(), serde_json::json!("orchestrator-pre-task-pause"));
        let created = self
            .task_manager
            .read()
            .await
            .enqueue_task(
                None,
                inputs,
                state.request.user_id,
                state.request.device_id,
                state.request.workspace_id,
                state.request.project_id,
                TaskPriority::Normal,
            )
            .await;
        let Ok(task_id) = created else {
            return error.to_string(); // cannot record a task: report the plain failure
        };
        state.task_id = Some(task_id);
        self.write_checkpoint(state, Some((0, error)));
        self.task_manager.read().await.pause_for_error(task_id, 0, error).await;
        tracing::warn!(task_id, error, "walk failed before task creation — recorded as a PAUSED task (resume re-runs the walk from the start)");
        format!(
            "PAUSED: task {task_id} paused at step 0: {error}. The run failed before any step executed; nothing was continued. Resume it (Tasks tab / project tab / meta chat / POST /task/resume_run) to re-run from the start."
        )
    }

    /// Resume a failure-paused task from the step it stopped at. Completed steps are
    /// restored from the checkpoint and not re-run.
    pub async fn resume_run(&self, task_id: u64) -> OrchestrationResponse {
        let ctx = crate::trace::child_or_root("resume_run", "resume-run");
        crate::trace::with_trace(ctx.clone(), self.resume_run_traced(task_id, ctx.trace_id.clone())).await
    }

    async fn resume_run_traced(&self, task_id: u64, trace_id: String) -> OrchestrationResponse {
        let cp = match self.load_checkpoint(task_id) {
            Ok(cp) => cp,
            Err(e) => {
                return OrchestrationResponse {
                    success: false,
                    response: None,
                    task_id: Some(task_id),
                    blueprint_id: None,
                    stages_completed: Vec::new(),
                    consciousness_gate: None,
                    jurisdiction_gate: None,
                    error: Some(e),
                    total_tokens_used: None,
                    execution_time_ms: 0,
                    methodologies_used: Vec::new(),
                    categories_created: 0,
                    blueprints_created: 0,
                    clarification_points: Vec::new(),
                    needs_clarification: false,
                    amt_summary: None,
                    model_used: None,
                    thinking_log: Vec::new(),
                    trace_id: Some(trace_id),
                };
            }
        };

        if cp.blueprint_steps.is_empty() {
            // Paused BEFORE any step existed (pre-task failure): resume = re-run the saved request.
            tracing::info!(task_id, "resuming a pre-task pause: re-running the saved request from the start");
            let _ = self.task_manager.read().await.resume_paused(task_id).await;
            let response = self.orchestrate(cp.request.clone()).await;
            let outputs = serde_json::json!({
                "response": response.response,
                "resumed_as_task": response.task_id,
                "success": response.success,
                "error": response.error,
            });
            let tm = self.task_manager.read().await;
            if response.success {
                let _ = tm.complete_task(task_id, Some(outputs), response.total_tokens_used.unwrap_or(0)).await;
            } else if let Some(e) = response.error.as_deref().filter(|e| !e.contains("PAUSED:")) {
                let _ = tm.fail_task(task_id, format!("resumed run failed: {e}")).await;
            } else {
                let _ = tm.complete_task(task_id, Some(outputs), 0).await;
            }
            return response;
        }
        let mut state = self.init_state(cp.request.clone(), trace_id).await;
        state.task_id = Some(cp.task_id);
        state.cleaned_prompt = cp.cleaned_prompt.clone();
        state.keywords = cp.keywords.iter().cloned().collect();
        state.topics = cp.topics.clone();
        state.methodologies = cp.methodologies.clone();
        state.categories = cp.categories.clone();
        state.amt_container_id = cp.amt_container_id;
        state.blueprint_id = cp.blueprint_id;
        state.blueprint_steps = cp.blueprint_steps.clone();
        state.modality_graphs = cp.modality_graphs.clone();
        state.file_graphs = cp.file_graphs.clone();
        state.chunk_graph_ids = cp.chunk_graph_ids.clone();
        state.applicable_tools = cp.applicable_tools.clone();
        state.tool_manifest = cp.tool_manifest.clone();
        state.missing_capabilities = cp.missing_capabilities.clone();
        state.walk_expansion = cp.walk_expansion.clone();
        state.graph_failures = cp.graph_failures.clone();
        state.attached_file_contents = cp.attached_file_contents.clone();
        state.tokens_used_so_far = cp.tokens_used_so_far;
        for st in &cp.completed_steps {
            let text = self.extract_output_text(&st.output);
            state
                .step_outputs
                .insert(st.step_index, serde_json::json!({ "output": text }));
            state.step_results.push(StepResult {
                step_index: st.step_index,
                pipeline_id: st.pipeline_id,
                output: st.output.clone(),
                tokens_used: st.tokens_used,
                iterations: 0,
                sub_step_results: Vec::new(),
                compliance_check: None,
            });
        }

        // Either we flip paused -> running here (direct callers), or the HTTP route already
        // did (so the UI never sees a stale "paused"); anything else is not resumable.
        let resumed = {
            let tm = self.task_manager.read().await;
            tm.resume_paused(task_id).await
                || tm.get_task(task_id).await.map(|t| t.status == "running").unwrap_or(false)
        };
        if !resumed {
            // Not failure-paused (already running, finished, cancelled…): refuse rather than
            // run a second copy of the same task.
            return self.build_error_response(
                &mut state,
                format!("task {task_id} is not paused for a step failure — nothing to resume"),
            );
        }
        tracing::info!(
            task_id,
            from_step = ?cp.paused_at_step,
            completed = cp.completed_steps.len(),
            "resuming failure-paused task from its checkpoint"
        );

        let result = self.execute_resumed_stages(&mut state).await;
        let response = match result {
            Ok(_) => self.build_success_response(&state),
            Err(e) => self.build_error_response(&mut state, e),
        };
        let _ = self.persist_chat_turn(&state, &response).await;
        response
    }
}
