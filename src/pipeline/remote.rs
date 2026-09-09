//! Remote pipeline registration + dispatch — the connect-model layer.
//!
//! Pipelines are independent units that spawn on their own, connect to the
//! host, and REGISTER themselves (id, name, execute URL). The host dispatches
//! `execute` to whoever serves that id over the SAME JSON envelope contract
//! the spawn path uses (PipelineInput in, one JSON object out). Spawning from
//! the host remains a configured fallback convention — never per-pipeline
//! host code. Transport (HTTP today) is an implementation detail behind this
//! module; swapping it must not touch the executor or the pipelines.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

/// One live, self-registered pipeline connection.
#[derive(Debug)]
pub struct RemotePipeline {
    pub pipeline_id: u64,
    pub name: String,
    /// Endpoint that accepts POST <PipelineInput JSON> and responds with one
    /// JSON output object (same contract as the spawn path's stdout).
    pub execute_url: String,
    pub registered_at: u64,
    pub call_count: AtomicU64,
    /// What this connection serves: "agent" (tasks), "model" (pipeline-9
    /// model calls), "observer" (monitor-only). Empty = "agent".
    pub roles: Vec<String>,
}

/// Dispatch table of live remote pipelines.
#[derive(Default)]
pub struct RemotePipelines {
    map: RwLock<HashMap<u64, Arc<RemotePipeline>>>,
    client: once_cell_lazy_client::Client,
}

// A tiny holder so RemotePipelines stays Default without a Clone dance.
mod once_cell_lazy_client {
    use std::sync::OnceLock;

    #[derive(Default)]
    pub struct Client;

    impl Client {
        pub fn get(&self) -> &'static reqwest::Client {
            static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
            CLIENT.get_or_init(reqwest::Client::new)
        }
    }
}

impl RemotePipelines {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register (or re-register — latest wins) a live pipeline connection.
    /// Re-registration is the heartbeat: it refreshes `registered_at`, which
    /// the dashboard reads as "last seen".
    pub async fn register(
        &self,
        pipeline_id: u64,
        name: String,
        execute_url: String,
        roles: Vec<String>,
    ) -> Arc<RemotePipeline> {
        let entry = Arc::new(RemotePipeline {
            pipeline_id,
            name,
            execute_url,
            registered_at: now_secs(),
            call_count: AtomicU64::new(0),
            roles,
        });
        self.map.write().await.insert(pipeline_id, entry.clone());
        entry
    }

    pub async fn deregister(&self, pipeline_id: u64) -> bool {
        self.map.write().await.remove(&pipeline_id).is_some()
    }

    pub async fn get(&self, pipeline_id: u64) -> Option<Arc<RemotePipeline>> {
        self.map.read().await.get(&pipeline_id).cloned()
    }

    pub async fn list(&self) -> Vec<RemotePipelineInfo> {
        self.map
            .read()
            .await
            .values()
            .map(|p| RemotePipelineInfo {
                pipeline_id: p.pipeline_id,
                name: p.name.clone(),
                execute_url: p.execute_url.clone(),
                registered_at: p.registered_at,
                call_count: p.call_count.load(Ordering::Relaxed),
                roles: p.roles.clone(),
            })
            .collect()
    }

    pub async fn len(&self) -> usize {
        self.map.read().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.map.read().await.is_empty()
    }

    /// Execute on a registered remote pipeline: POST the same PipelineInput
    /// JSON the spawn path would pass as --input, parse the same one-JSON-
    /// object response. Failure to reach a remote is an Err — the caller
    /// (executor) decides whether to fall back to spawning.
    pub async fn execute(
        &self,
        pipeline_id: u64,
        input: &crate::types::pipeline::PipelineInput,
        execution_id: crate::types::pipeline::ExecutionID,
        task_id: Option<crate::types::TaskID>,
    ) -> Result<crate::types::pipeline::PipelineOutput, String> {
        let remote = self
            .get(pipeline_id)
            .await
            .ok_or_else(|| format!("No remote pipeline registered for id {}", pipeline_id))?;

        let body = serde_json::to_value(input).map_err(|e| e.to_string())?;

        let resp = self
            .client
            .get()
            .post(&remote.execute_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Remote pipeline {} unreachable: {}", pipeline_id, e))?;

        if !resp.status().is_success() {
            return Err(format!(
                "Remote pipeline {} returned HTTP {}",
                pipeline_id,
                resp.status()
            ));
        }

        let output_json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        remote.call_count.fetch_add(1, Ordering::Relaxed);

        // Same output contract as invoke_pipeline: one JSON object → data map.
        let data = match output_json {
            serde_json::Value::Object(m) => m
                .into_iter()
                .collect::<HashMap<String, serde_json::Value>>(),
            other => {
                let mut m = HashMap::new();
                m.insert("raw_output".to_string(), other);
                m
            }
        };

        Ok(crate::types::pipeline::PipelineOutput {
            data,
            execution_id,
            task_id,
            success: true,
            error: None,
        })
    }
}

/// Serializable view for API responses (dashboard / monitoring).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RemotePipelineInfo {
    pub pipeline_id: u64,
    pub name: String,
    pub execute_url: String,
    pub registered_at: u64,
    pub call_count: u64,
    /// "agent" | "model" | "observer" — see RemotePipeline::roles.
    #[serde(default)]
    pub roles: Vec<String>,
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
