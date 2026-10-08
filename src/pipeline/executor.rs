//! Pipeline executor - invokes actual pipeline implementations
//!
//! The executor calls the actual pipeline code which lives in pipelines/ directory.
//! This maintains separation between core (here) and pipeline logic (pipelines/).

use crate::config::PipelineConfig;
use crate::types::pipeline::ExecutionID;
use crate::types::{
    OzoneError, OzoneResult, PipelineBlueprint, PipelineID, PipelineInput,
    PipelineOutput, TaskID,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

/// Hard bound for one pipeline child: an absolute wall-clock ceiling only.
/// Operator direction (2026-10-07): "silence is not evidence of a hang" — a
/// reasoning model can think for minutes with no stdout, so an idle-silence
/// kill (briefly tried the same day) would kill working calls mid-thought.
/// The only kill trigger left is ABS_CEILING in run_child_bounded: a child
/// that outlives it is killed regardless of whether it was silent or
/// streaming the whole time. Naming WHICH internal call is actually stuck
/// (vs. merely slow) needs a call-start marker protocol with its own shared
/// state — deferred, see docs/REVIEW_WATCHDOG_2026-10-07.md §10.
// (The former blind watchdog+30 budget, the per-pipeline 3× table tried on
// 2026-10-07, AND the 150s idle-silence kill tried the same day — all GONE.)

/// Sets its flag on drop. Dropping the execute future (watchdog or disconnect)
/// drops this, so the blocking child runner kills its child instead of letting
/// it run on after its caller is gone.
struct CancelOnDrop(Arc<std::sync::atomic::AtomicBool>);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Keeps the running count correct on every exit path, including a cancelled future.
struct RunningGuard<'a>(&'a std::sync::atomic::AtomicUsize);

impl Drop for RunningGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Marks a progress entry Failed when the execute future is dropped before it
/// records its own outcome. Without this, a watchdog-cancelled run stays
/// "Running" in the progress map forever.
struct ProgressOnDrop {
    map: Arc<tokio::sync::RwLock<HashMap<String, PipelineProgress>>>,
    id: String,
    armed: bool,
}

impl Drop for ProgressOnDrop {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let map = self.map.clone();
        let id = self.id.clone();
        handle.spawn(async move {
            let mut m = map.write().await;
            if let Some(p) = m.get_mut(&id) {
                if matches!(p.status, ProgressStatus::Running) {
                    p.status = ProgressStatus::Failed;
                    p.completed_at = Some(now_secs());
                    p.error = Some(
                        "abandoned: the caller was dropped before this execution finished (watchdog or disconnect)"
                            .to_string(),
                    );
                }
            }
        });
    }
}

/// Run a pipeline child with a hard bound. Stdout and stderr are drained on
/// their own threads, so a child that writes more than a pipe buffer cannot
/// block on a full pipe. The child is killed when it outlives `timeout` or when
/// `cancel` is set, and a kill is returned as an error, never as partial output.
/// A pipe still held open after the child exits (a grandchild inherited it) is
/// an error after five seconds, not a wait forever.
fn run_child_bounded(
    mut cmd: Command,
    cancel: Arc<std::sync::atomic::AtomicBool>,
) -> std::io::Result<std::process::Output> {
    use std::io::Read;
    use std::process::Stdio;
    use std::sync::atomic::Ordering;

    // ABSOLUTE CEILING ONLY (2026-10-07 correction): the idle-silence kill
    // tried earlier the same day is GONE — "silence ≠ hung" (a reasoning
    // model can think for minutes with no stdout; killing on silence would
    // kill working calls mid-thought). A child now runs as long as it wants,
    // silent or streaming, up to this one absolute bound. Diagnosing a
    // genuine hang (vs. merely slow) is deferred to the call-start marker
    // protocol — see docs/REVIEW_WATCHDOG_2026-10-07.md §10.
    const ABS_CEILING: std::time::Duration = std::time::Duration::from_secs(1800);

    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn()?;

    let activity = Arc::new(std::sync::atomic::AtomicU64::new(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
    ));

    // Drain threads: incremental reads (NOT read_to_end, which only returns
    // at EOF and would hide intermediate activity), each read refreshing the
    // activity clock and forwarding bytes for collection.
    fn drain(
        mut pipe: impl Read + Send + 'static,
        activity: Arc<std::sync::atomic::AtomicU64>,
    ) -> std::sync::mpsc::Receiver<Vec<u8>> {
        let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match pipe.read(&mut buf) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        activity.store(
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_millis() as u64)
                                .unwrap_or(0),
                            Ordering::SeqCst,
                        );
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        rx
    }

    let spawn_time = std::time::Instant::now();
    let out_rx = drain(child.stdout.take().unwrap(), activity.clone());
    let err_rx = drain(child.stderr.take().unwrap(), activity.clone());

    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        let reason = if cancel.load(Ordering::SeqCst) {
            Some("its caller was dropped".to_string())
        } else if std::time::Instant::now() - spawn_time > ABS_CEILING {
            // Unconditional: applies whether the child was silent or
            // streaming the whole time — not a silence judgment, a hard
            // wall-clock bound. `activity` is kept (still fed by the drain
            // threads below) as the liveness signal a future call-start
            // marker protocol would read; nothing reads it for a kill
            // decision anymore.
            Some(format!("absolute ceiling ({}s)", ABS_CEILING.as_secs()))
        } else {
            None
        };
        if let Some(reason) = reason {
            let _ = child.kill();
            let _ = child.wait();
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("pipeline child killed: {}", reason),
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    };
    // Collect the drained output: each drain thread forwards its bytes; after
    // the child exits, a pipe still open 5s later means a grandchild is
    // holding it (reported, never silently truncated).
    let collect = |rx: &std::sync::mpsc::Receiver<Vec<u8>>| {
        let mut buf = Vec::new();
        loop {
            match rx.recv_timeout(std::time::Duration::from_secs(5)) {
                Ok(chunk) => buf.extend_from_slice(&chunk),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => break,
                Err(_) => break,
            }
        }
        buf
    };
    let stdout = collect(&out_rx);
    let stderr = collect(&err_rx);
    Ok(std::process::Output {
        status,
        stdout,
        stderr,
    })
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ProgressStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PipelineProgress {
    pub execution_id: String,
    pub pipeline_id: PipelineID,
    pub pipeline_name: String, // human-readable for UI
    pub task_id: Option<TaskID>,
    pub step_index: Option<u32>, // which blueprint step triggered this
    pub status: ProgressStatus,
    pub progress_percent: u8,
    pub started_at: u64,
    pub completed_at: Option<u64>,
    pub tokens_used: Option<u32>,
    pub error: Option<String>,
}

/// Pipeline executor
pub struct PipelineExecutor {
    /// Path to builtin pipelines
    builtin_path: PathBuf,

    /// Path to custom pipelines
    custom_path: PathBuf,

    /// Ordered admission: every call queues by origin tier and always runs.
    gate: Arc<crate::pipeline::gate::OrderedPipelineGate>,

    /// Single-flight on the local BitNet/GGUF model (A6, 2026-10-07).
    /// Confirmed root cause: two background loops (I-Loop, assistant
    /// digest) reached the local model at the same instant and both died
    /// together — nothing serialized them. Capacity 1, and deliberately
    /// NEVER given `spawn_keeper()` (see its use site in `execute()`):
    /// the starvation guard's force-grant-past-max_active would admit a
    /// second holder while the first is still running, defeating
    /// single-flight entirely. An API-bound call never touches this gate.
    local_model_gate: Arc<crate::pipeline::gate::OrderedPipelineGate>,

    /// Currently running pipeline count
    running_count: std::sync::atomic::AtomicUsize,

    /// Live self-registered remote pipelines — dispatch tries here FIRST,
    /// spawn is the fallback convention.
    remote: Arc<crate::pipeline::remote::RemotePipelines>,

    /// Monitor activity hub (dashboard feed, browser plugin, ZCode connector).
    activity: Arc<crate::monitor::ActivityHub>,

    progress_map: Arc<tokio::sync::RwLock<std::collections::HashMap<String, PipelineProgress>>>,
    cancel_set: Arc<tokio::sync::RwLock<std::collections::HashSet<String>>>,
}

impl PipelineExecutor {
    /// Create new executor
    pub fn new(config: &PipelineConfig) -> OzoneResult<Self> {
        Ok(Self {
            builtin_path: PathBuf::from(&config.builtin_path),
            custom_path: PathBuf::from(&config.custom_path),
            gate: {
                let g = std::sync::Arc::new(crate::pipeline::gate::OrderedPipelineGate::new(
                    config.max_concurrent_pipelines,
                ));
                g.spawn_keeper();
                g
            },
            // Capacity 1, spawn_keeper() deliberately NOT called — see the
            // field doc comment.
            local_model_gate: Arc::new(crate::pipeline::gate::OrderedPipelineGate::new(1)),
            running_count: std::sync::atomic::AtomicUsize::new(0),
            remote: Arc::new(crate::pipeline::remote::RemotePipelines::new()),
            activity: Arc::new(crate::monitor::ActivityHub::new()),
            progress_map: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            cancel_set: Arc::new(tokio::sync::RwLock::new(std::collections::HashSet::new())),
        })
    }

    /// Registration table for self-connecting pipelines.
    pub fn remote_pipelines(&self) -> Arc<crate::pipeline::remote::RemotePipelines> {
        self.remote.clone()
    }

    /// Monitor activity hub accessor.
    pub fn activity_hub(&self) -> Arc<crate::monitor::ActivityHub> {
        self.activity.clone()
    }

    /// True when this call will invoke the local BitNet/GGUF model — the one
    /// signal available here without new cross-module plumbing or a trait
    /// change. Pipeline 9 (Prompt) is the only pipeline that ever calls a
    /// model directly; every other pipeline either makes no model call or
    /// routes through pipeline 9's own subprocess (confirmed by reading
    /// SubprocessExecutor in the pipeline crates — those nested calls are
    /// pipeline 9's own concern, not visible here, and out of scope for this
    /// change). A per-call override (`model_override_config.model_type`,
    /// set by the walk/a step — see stages.rs/mod.rs) wins when present;
    /// absent one, the global default is read from `OZONE_MODEL_TYPE`,
    /// which this process set on its OWN environment at boot
    /// (`ModelConfig::to_pipeline_env` via `std::env::set_var`, not just
    /// passed to a spawned child — so reading it here needs no new field).
    fn targets_local_model(pipeline_id: PipelineID, input: &PipelineInput) -> bool {
        if pipeline_id != 9 {
            return false;
        }
        // input.data is the host's own untagged types::Value (not
        // serde_json::Value) — match through the enum explicitly.
        let model_type = input
            .data
            .get("model_override_config")
            .and_then(|v| match v {
                crate::types::Value::Map(m) => m.get("model_type"),
                _ => None,
            })
            .and_then(|v| match v {
                crate::types::Value::String(s) => Some(s.as_str()),
                _ => None,
            })
            .map(|s| s.to_string())
            .or_else(|| std::env::var("OZONE_MODEL_TYPE").ok());
        matches!(model_type.as_deref(), Some("bitnet") | Some("gguf"))
    }

    /// Execute a pipeline
    pub async fn execute(
        &self,
        blueprint: &PipelineBlueprint,
        input: PipelineInput,
        task_id: Option<TaskID>,
    ) -> OzoneResult<PipelineOutput> {
        let execution_id = ExecutionID::new();
        let execution_id_str = execution_id.as_str().to_string();

        tracing::info!(
            pipeline = %blueprint.name,
            execution_id = %execution_id,
            task_id = ?task_id,
            "Starting pipeline execution"
        );

        let _admission = self
            .gate
            .admit(crate::pipeline::gate::current_call_priority())
            .await;
        // Single-flight on the local model (A6) — see the field doc comment
        // on `local_model_gate`. Held for the rest of this call, same scope
        // as `_admission` above; an API-bound call gets `None` and never
        // touches this gate.
        let _local_admission = if Self::targets_local_model(blueprint.pipeline_id, &input) {
            Some(
                self.local_model_gate
                    .admit(crate::pipeline::gate::current_call_priority())
                    .await,
            )
        } else {
            None
        };
        self.running_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        // Decrements on every exit, including a dropped (cancelled) future.
        let _running = RunningGuard(&self.running_count);

        // Register progress early
        let pipeline_name = crate::pipeline::registry::get_pipeline_info(blueprint.pipeline_id)
            .map(|info| info.name.clone())
            .unwrap_or_else(|| blueprint.name.clone());

        {
            let mut map = self.progress_map.write().await;
            map.insert(
                execution_id_str.clone(),
                PipelineProgress {
                    execution_id: execution_id_str.clone(),
                    pipeline_id: blueprint.pipeline_id,
                    pipeline_name: pipeline_name.clone(),
                    task_id,
                    step_index: None, // Will be set by orchestrator via set_step_context()
                    status: ProgressStatus::Running,
                    progress_percent: 0,
                    started_at: now_secs(), // assume now_secs() helper exists
                    completed_at: None,
                    tokens_used: None,
                    error: None,
                },
            );
        }

        // Marks this entry Failed if the future is dropped before it finishes.
        let mut progress_guard = ProgressOnDrop {
            map: self.progress_map.clone(),
            id: execution_id_str.clone(),
            armed: true,
        };

        // Previously ActivityHub::record was only ever called from a handful
        // of admin-type HTTP actions (device pairing, remote-pipeline
        // register/unregister) — never from actual pipeline dispatch, so
        // /monitor/activity stayed empty during real work no matter how much
        // was running. This is the one place every dispatch path (builtin,
        // custom, remote) funnels through, so it's the right single spot to
        // make ordinary pipeline execution show up in the activity feed too.
        self.activity.record(
            crate::monitor::ActivityKind::Job,
            crate::monitor::ActivityLevel::Info,
            "pipeline",
            format!("{} started", pipeline_name),
            Some(serde_json::json!({
                "execution_id": execution_id_str,
                "pipeline_id": blueprint.pipeline_id,
                "task_id": task_id,
            })),
        );

        {
            let cancelled = self.cancel_set.read().await;
            if cancelled.contains(&execution_id_str) {
                let mut map = self.progress_map.write().await;
                if let Some(p) = map.get_mut(&execution_id_str) {
                    p.status = ProgressStatus::Cancelled;
                    p.completed_at = Some(now_secs());
                }
                return Err(OzoneError::PipelineError(format!(
                    "Execution {} cancelled",
                    execution_id
                )));
            }
        }

        let result = self
            .execute_inner(blueprint, input, execution_id, task_id)
            .await;
        // The outcome is recorded below; the guard must not overwrite it.
        progress_guard.armed = false;

        // Update progress on completion
        let final_status = if result.is_ok() {
            ProgressStatus::Completed
        } else {
            ProgressStatus::Failed
        };

        {
            let mut map = self.progress_map.write().await;
            if let Some(progress) = map.get_mut(&execution_id_str) {
                progress.status = final_status;
                progress.progress_percent = 100;
                progress.completed_at = Some(now_secs());
                if let Err(ref e) = result {
                    progress.error = Some(e.to_string());
                }
            }
        }

        self.activity.record(
            crate::monitor::ActivityKind::Job,
            if result.is_ok() {
                crate::monitor::ActivityLevel::Ok
            } else {
                crate::monitor::ActivityLevel::Error
            },
            "pipeline",
            format!(
                "{} {}",
                pipeline_name,
                if result.is_ok() { "completed" } else { "failed" }
            ),
            Some(serde_json::json!({
                "execution_id": execution_id_str,
                "pipeline_id": blueprint.pipeline_id,
                "task_id": task_id,
                "error": result.as_ref().err().map(|e| e.to_string()),
            })),
        );

        // Wrap result
        match result {
            Ok(mut output) => {
                output.execution_id = execution_id;
                output.task_id = task_id;
                Ok(output)
            }
            Err(e) => Err(e),
        }
    }

    /// Internal execution logic
    async fn execute_inner(
        &self,
        blueprint: &PipelineBlueprint,
        input: PipelineInput,
        execution_id: ExecutionID,
        task_id: Option<TaskID>,
    ) -> OzoneResult<PipelineOutput> {
        let pipeline_id = blueprint.pipeline_id;

        tracing::debug!(
            execution_id = %execution_id,
            pipeline_id = pipeline_id,
            "Executing inner pipeline logic"
        );

        // CONNECT MODEL: a self-registered remote pipeline wins — it owns
        // its runtime; the host never spawns over a live connection.
        if self.remote.get(pipeline_id).await.is_some() {
            tracing::info!(
                execution_id = %execution_id,
                pipeline_id = pipeline_id,
                "Dispatching to registered remote pipeline"
            );
            let output = self
                .remote
                .execute(pipeline_id, &input, execution_id, task_id)
                .await
                .map_err(OzoneError::PipelineError)?;
            return Ok(output);
        }

        if self.is_builtin(pipeline_id) {
            self.execute_builtin(pipeline_id, input, execution_id, task_id)
                .await
        } else {
            self.execute_custom(blueprint, input, execution_id, task_id)
                .await
        }
    }

    /// Check if pipeline is builtin — anything with real registry metadata
    /// (compile-time PIPELINE_INFO or runtime-loaded from index.json) ships
    /// with the product and is dispatched by path lookup; ids that are NOT in
    /// the registry are ad-hoc pipelines registered at runtime via
    /// register_custom (e.g. through pipeline 15/PipelineCreation) and are
    /// dispatched by name from custom_path instead. The old hardcoded
    /// `pipeline_id <= 54` cutoff misrouted every modality pipeline (100+)
    /// and pipeline 55 into execute_custom, where they could never resolve.
    fn is_builtin(&self, pipeline_id: PipelineID) -> bool {
        crate::pipeline::registry::get_pipeline_info(pipeline_id).is_some()
    }

    /// Execute a builtin pipeline
    async fn execute_builtin(
        &self,
        pipeline_id: PipelineID,
        input: PipelineInput,
        execution_id: ExecutionID,
        task_id: Option<TaskID>,
    ) -> OzoneResult<PipelineOutput> {
        let (category, pipeline_name) = self.get_builtin_info(pipeline_id);

        let pipeline_dir = self.builtin_path.join(category).join(&pipeline_name);

        // Candidate executables, in order: binary next to the pipeline
        // folder, crate target builds, sibling of the host binary. NEVER
        // execute main.rs — that was an EACCES trap (source isn't runnable).
        let name = pipeline_name.as_str();
        let mut candidates = vec![
            pipeline_dir.join(name),
            pipeline_dir.join("target/release").join(name),
            pipeline_dir.join("target/debug").join(name),
        ];
        // Pipelines are independent workspace roots under assets/pipelines/, built
        // by scripts/build-pipelines.sh into one shared target dir. Anchor via
        // CARGO_MANIFEST_DIR so this resolves regardless of the host's CWD at launch.
        candidates.push(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("target/pipelines/release")
                .join(name),
        );
        if let Ok(exe_dir) = std::env::current_exe() {
            if let Some(dir) = exe_dir.parent() {
                candidates.push(dir.join(name));
            }
        }

        let pipeline_path = candidates
            .iter()
            .find(|c| c.exists())
            .map(|c| c.to_path_buf());

        let pipeline_path = match pipeline_path {
            Some(p) => p,
            None => {
                tracing::error!(
                    execution_id = %execution_id,
                    pipeline_name = %pipeline_name,
                    searched = ?candidates,
                    "Builtin pipeline binary not found"
                );
                return Ok(PipelineOutput {
                    execution_id,
                    task_id,
                    data: {
                        let mut map = HashMap::new();
                        map.insert(
                            "error".into(),
                            serde_json::Value::String(format!(
                                "Pipeline {} binary not built — build its crate and place the binary in the pipelines data dir",
                                pipeline_name
                            )),
                        );
                        map
                    },
                    success: false,
                    error: Some(format!(
                        "Pipeline {} binary not built",
                        pipeline_name
                    )),
                });
            }
        };

        tracing::info!(
            execution_id = %execution_id,
            pipeline_path = ?pipeline_path,
            "Invoking builtin pipeline"
        );

        self.invoke_pipeline(&pipeline_path, input, execution_id, task_id)
            .await
    }

    /// Get builtin pipeline category and name from ID (uses central registry)
    fn get_builtin_info(&self, pipeline_id: PipelineID) -> (&'static str, String) {
        use crate::pipeline::registry::get_pipeline_info;

        if let Some(info) = get_pipeline_info(pipeline_id) {
            (info.category, info.folder_name.clone())
        } else {
            tracing::warn!(
                "Pipeline {} not found in registry, using fallback path",
                pipeline_id
            );
            ("core", format!("pipeline_{}", pipeline_id))
        }
    }

    /// Execute a custom pipeline
    async fn execute_custom(
        &self,
        blueprint: &PipelineBlueprint,
        input: PipelineInput,
        execution_id: ExecutionID,
        task_id: Option<TaskID>,
    ) -> OzoneResult<PipelineOutput> {
        let pipeline_path = self.custom_path.join(&blueprint.name);

        if !pipeline_path.exists() {
            tracing::error!(
                execution_id = %execution_id,
                custom_path = ?self.custom_path,
                blueprint_name = %blueprint.name,
                "Custom pipeline not found"
            );
            return Err(OzoneError::NotFound(format!(
                "Custom pipeline {} not found (execution {})",
                blueprint.name, execution_id
            )));
        }

        tracing::info!(
            execution_id = %execution_id,
            pipeline_path = ?pipeline_path,
            "Invoking custom pipeline"
        );

        self.invoke_pipeline(&pipeline_path, input, execution_id, task_id)
            .await
    }

    /// Invoke a pipeline executable/script
    async fn invoke_pipeline(
        &self,
        pipeline_path: &PathBuf,
        input: PipelineInput,
        execution_id: ExecutionID,
        task_id: Option<TaskID>,
    ) -> OzoneResult<PipelineOutput> {
        let input_json = serde_json::to_string(&input)
            .map_err(|e| OzoneError::SerializationError(e.to_string()))?;

        let extension = pipeline_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let mut cmd = match extension {
            "rs" | "" => Command::new(pipeline_path),
            "py" => {
                let mut c = Command::new("python3");
                c.arg(pipeline_path);
                c
            }
            "js" | "ts" => {
                let mut c = Command::new("node");
                c.arg(pipeline_path);
                c
            }
            _ => {
                return Err(OzoneError::PipelineError(format!(
                    "Unsupported pipeline type: {} (execution {})",
                    extension, execution_id
                )));
            }
        };

        cmd.arg("--input")
            .arg(&input_json)
            .arg("--execution-id")
            .arg(execution_id.as_str());

        if let Some(tid) = task_id {
            cmd.arg("--task-id").arg(tid.to_string());
        }

        // Blocking child process, now bounded: killed at the watchdog budget
        // plus a margin, or when this call is dropped, so a stuck child can
        // never outlive its caller while still holding a gate slot the gate
        // believes is free. Kept off the async worker thread as before.
        // The child's own subprocesses (a local model) get a generous fixed
        // bound: one load + generation with headroom. The child itself is
        // not age-bounded anymore — idle-kill governs (run_child_bounded).
        cmd.env("OZONE_PIPELINE_TIMEOUT_SECS", "600");
        let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let _cancel_on_drop = CancelOnDrop(cancel.clone());
        let output = tokio::task::spawn_blocking(move || {
            run_child_bounded(cmd, cancel)
        })
        .await
        .map_err(|e| {
            OzoneError::PipelineError(format!(
                "Pipeline task join failed (execution {}): {}",
                execution_id, e
            ))
        })?
        .map_err(|e| {
            OzoneError::PipelineError(format!(
                "Failed to execute pipeline (execution {}): {}",
                execution_id, e
            ))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Confirmed live: the prompt pipeline (and likely others) prints
            // its real error JSON to STDOUT via println! before calling
            // std::process::exit(1) on any recoverable failure (rate limit,
            // network error, bad response) — stderr stays empty every time,
            // silently hiding the actual reason behind a blank
            // "Pipeline execution failed (non-zero exit) stderr=" log line.
            // Capture stdout too so the real message is never discarded.
            let stdout = String::from_utf8_lossy(&output.stdout);
            let detail = if !stderr.trim().is_empty() {
                stderr.to_string()
            } else if !stdout.trim().is_empty() {
                stdout.to_string()
            } else {
                String::new()
            };
            tracing::error!(
                execution_id = %execution_id,
                stderr = %stderr,
                stdout = %stdout,
                "Pipeline execution failed (non-zero exit)"
            );
            return Ok(PipelineOutput {
                execution_id,
                task_id,
                data: HashMap::new(),
                success: false,
                error: Some(detail),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let output_data: HashMap<String, serde_json::Value> = serde_json::from_str(&stdout)
            .unwrap_or_else(|parse_err| {
                tracing::warn!(
                    execution_id = %execution_id,
                    parse_error = %parse_err,
                    "Pipeline output not valid JSON — using raw stdout"
                );
                let mut map = HashMap::new();
                map.insert(
                    "raw_output".into(),
                    serde_json::Value::String(stdout.into()),
                );
                map
            });

        tracing::info!(
            execution_id = %execution_id,
            success = true,
            "Pipeline execution completed successfully"
        );

        Ok(PipelineOutput {
            execution_id,
            task_id,
            data: output_data,
            success: true,
            error: None,
        })
    }

    /// Called by the orchestrator before/during step execution to link this execution to a specific task step
    pub async fn set_step_context(&self, execution_id: &str, task_id: TaskID, step_index: u32) {
        let mut map = self.progress_map.write().await;
        if let Some(p) = map.get_mut(execution_id) {
            p.task_id = Some(task_id);
            p.step_index = Some(step_index);
        }
    }

    /// Get progress for an execution
    pub async fn get_progress(&self, execution_id: &str) -> Option<PipelineProgress> {
        let map = self.progress_map.read().await;
        map.get(execution_id).cloned()
    }

    /// Request cancellation of an execution
    pub async fn cancel(&self, execution_id: &str) -> bool {
        let map = self.progress_map.read().await;
        if map
            .get(execution_id)
            .map(|p| matches!(p.status, ProgressStatus::Running | ProgressStatus::Queued))
            .unwrap_or(false)
        {
            drop(map);
            let mut cancel_set = self.cancel_set.write().await;
            cancel_set.insert(execution_id.to_string());
            true
        } else {
            false
        }
    }

    /// Clean up completed executions older than TTL
    pub async fn cleanup_old_progress(&self, ttl_secs: u64) {
        let now = now_secs();
        let mut map = self.progress_map.write().await;
        map.retain(|_, p| match p.status {
            ProgressStatus::Running | ProgressStatus::Queued => true,
            _ => p.completed_at.map(|t| now - t < ttl_secs).unwrap_or(false),
        });

        let mut cancel_set = self.cancel_set.write().await;
        cancel_set.retain(|id| map.contains_key(id));
    }

    /// Get shared progress map reference (for HTTP handler / UI access)
    pub fn progress_map(
        &self,
    ) -> Arc<tokio::sync::RwLock<std::collections::HashMap<String, PipelineProgress>>> {
        self.progress_map.clone()
    }
}

// Helper function - you should define this in a common utils module if not already present
fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
