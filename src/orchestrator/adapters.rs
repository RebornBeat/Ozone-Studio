//! Runtime adapters — the standardized abstract calls.
//!
//! These impls are the ONLY place where the orchestrator's abstract
//! contracts (`PipelineExecutor`, `StoreAccess`) meet concrete backends.
//! Swapping the pipeline registry or the ZSEI store for another backend
//! means writing a new adapter here — no orchestrator changes.

use crate::pipeline::PipelineRegistry;
use crate::types::container::Container;
use crate::types::pipeline::{ExecutionContext, PipelineInput};
use crate::types::zsei::{ContainerUpdate, TraversalRequest, ZSEIQuery, ZSEIQueryResult};
use crate::zsei::ZSEI;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// ============================================================================
// PIPELINE EXECUTION ADAPTER
// ============================================================================

/// Adapter: orchestrator's abstract pipeline call → the runtime pipeline
/// registry (which spawns/loads pipeline binaries per the executor).
pub struct RegistryExecutorAdapter {
    pub registry: Arc<RwLock<PipelineRegistry>>,
}

/// Convert untyped JSON into the pipeline input's typed value map.
fn json_to_types_value(v: serde_json::Value) -> crate::types::Value {
    use crate::types::Value as V;
    match v {
        serde_json::Value::Null => V::Null,
        serde_json::Value::Bool(b) => V::Bool(b),
        serde_json::Value::Number(n) if n.is_i64() => V::Int(n.as_i64().unwrap_or(0)),
        serde_json::Value::Number(n) => V::Float(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::String(s) => V::String(s),
        serde_json::Value::Array(a) => V::Array(a.into_iter().map(json_to_types_value).collect()),
        serde_json::Value::Object(m) => {
            V::Map(m.into_iter().map(|(k, v)| (k, json_to_types_value(v))).collect())
        }
    }
}

#[async_trait::async_trait]
impl super::PipelineExecutor for RegistryExecutorAdapter {
    async fn execute(
        &self,
        pipeline_id: u64,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        // LOCK-SCOPE FIX (found live 2026-10-02 via 6-thread all-futex dump):
        // this fn used to hold `self.registry.read()` across the ENTIRE
        // pipeline execution. A queued writer (a boot-time self-registration)
        // then blocked every subsequent read forever — all tokio workers
        // parked on futexes, walks frozen at Stage 4b with nothing in the
        // log. Clone the Arc, drop the guard, let registry.execute() take
        // its own internal lock for the brief blueprint lookup it needs.
        let registry = self.registry.clone();
        // Blueprint must exist before execution (registry owns the map).
        if registry.read().await.get_blueprint(pipeline_id).await.is_none() {
            return Err(format!("Pipeline {} not found", pipeline_id));
        }
        drop(registry);

        // "_execution_context", if present, carries the caller's real
        // ExecutionContext (user/device/workspace/project ids) — e.g. a step
        // rerun (grpc/mod.rs rerun_step) needs the ORIGINAL task's context,
        // not a default one, so the reran step is scoped correctly. Reserved
        // key, stripped before the rest becomes `data`; absent → unchanged
        // default behavior for every existing caller.
        let (context, input) = match input {
            serde_json::Value::Object(mut m) => {
                let context = m
                    .remove("_execution_context")
                    .and_then(|v| serde_json::from_value::<ExecutionContext>(v).ok())
                    .unwrap_or_default();
                (context, serde_json::Value::Object(m))
            }
            other => (ExecutionContext::default(), other),
        };

        let data: HashMap<String, crate::types::Value> = match input {
            serde_json::Value::Object(m) => {
                m.into_iter().map(|(k, v)| (k, json_to_types_value(v))).collect()
            }
            serde_json::Value::Null => HashMap::new(),
            other => {
                let mut m = HashMap::new();
                m.insert("payload".to_string(), json_to_types_value(other));
                m
            }
        };

        // TRACE: stamp the current trace onto the ExecutionContext metadata so
        // the pipeline envelope (and a remote pipeline's request body) carries
        // it even where the environment cannot. Existing keys win — a caller
        // that already set trace metadata keeps its own. The executor layer
        // additionally exports it as OZONE_TRACE_ID / OZONE_PARENT_SPAN to the
        // spawned process. Untraced calls are left exactly as before.
        let mut context = context;
        if let Some(t) = crate::trace::current_trace() {
            for (k, v) in [
                ("trace_id", Some(t.trace_id.clone())),
                ("span_id", Some(t.span_id.clone())),
                ("parent_span", t.parent_span.clone()),
            ] {
                if let Some(v) = v {
                    context.metadata.entry(k.to_string()).or_insert(v);
                }
            }
        }

        let pipeline_input = PipelineInput { data, context };

        // ADAPTER BACKSTOP (rescaled 2026-10-10 after walk B died at exactly
        // 300s in Stage 2): this watchdog predates the subprocess refactor —
        // its original target was an IN-PROCESS hang (15-21 min, no
        // subprocess alive, no other timeout). Every builtin pipeline now
        // runs as a subprocess that self-bounds at its own ABS_CEILING
        // (1800s) with per-call inner bounds, so this seam must BACKSTOP
        // that ceiling, not compete with legitimate multi-call extractions
        // (a 3-section prompt's Stage 2 legitimately exceeds 300s). The
        // registry value is respected when the operator tunes it UPWARD;
        // the child's own 1800s ceiling is the floor here.
        let registry_secs =
            crate::k_registry::KAlgorithms::global().current_watchdog_secs();
        let watchdog_secs = if registry_secs == u64::MAX {
            u64::MAX
        } else {
            registry_secs.max(1800)
        };
        let output = if watchdog_secs == u64::MAX {
            self.registry
                .read()
                .await
                .execute(pipeline_id, pipeline_input, None)
                .await
                .map_err(|e| e.to_string())?
        } else {
            let exec_fut = async {
                let registry = self.registry.read().await;
                registry.execute(pipeline_id, pipeline_input, None).await
            };
            // PROGRESS-AWARE WATCHDOG (operator 2026-11-06: "timeouts should not be a limiter as long as it is doing work"): the budget
            // is a STALL budget, not a wall-clock cap. Past `watchdog_secs` the invocation is kept while a model call made progress in
            // the last 300 s (the prompt binary stamps liveness while streaming); it is abandoned when it stalls, or at a very large
            // hard ceiling (OZONE_WATCHDOG_HARD_SECS, default 14400). A pipeline that does no model calls gets the old behaviour.
            let hard: u64 = std::env::var("OZONE_WATCHDOG_HARD_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(14400);
            let started = std::time::Instant::now();
            tokio::pin!(exec_fut);
            loop {
                tokio::select! {
                    r = &mut exec_fut => break r.map_err(|e| e.to_string())?,
                    _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                        let elapsed = started.elapsed().as_secs();
                        if elapsed >= watchdog_secs {
                            let working = crate::model_ledger::model_activity_within(300);
                            if !working || elapsed >= hard {
                                tracing::error!(
                                    pipeline_id,
                                    secs = elapsed,
                                    "WATCHDOG: pipeline invocation stalled (no model-call progress for 300s) or hit the hard ceiling — failing loudly (hung call abandoned; note the underlying subprocess, if any, may outlive this future)"
                                );
                                return Err(format!(
                                    "watchdog timeout after {elapsed}s — pipeline {pipeline_id} invocation made no progress"
                                ));
                            }
                        }
                    }
                }
            }
        };

        // PipelineOutput wraps the pipeline's own result fields under `data`
        // (alongside execution_id/task_id/success/error, which belong to the
        // dispatch envelope, not the pipeline's contract). Every orchestrator
        // caller (extract_output_text, metered_execute, and every direct
        // `.get("response")` / `.get("tokens_used")` / `.get("model_used")`
        // site) expects the pipeline's own fields directly at the top level —
        // returning the full wrapper here meant every one of those lookups
        // silently missed and fell through to its default (empty response,
        // 0 tokens), even on pipelines that ran successfully. And because
        // `output.success`/`output.error` were never checked here, a pipeline
        // that reported failure (e.g. "binary not built") still came back as
        // Ok(...) to every caller instead of propagating as an Err via `?`.
        if !output.success {
            return Err(output
                .error
                .unwrap_or_else(|| format!("Pipeline {} execution failed", pipeline_id)));
        }

        serde_json::to_value(&output.data).map_err(|e| e.to_string())
    }

    async fn pipeline_exists(&self, pipeline_id: u64) -> bool {
        self.registry
            .read()
            .await
            .get_blueprint(pipeline_id)
            .await
            .is_some()
    }
}

// ============================================================================
// ZSEI STORE ADAPTER (the swappable store contract)
// ============================================================================

/// Adapter: orchestrator's abstract store contract → the ZSEI container
/// store, routed through the `ZSEIQuery` enum — the standardized store call.
/// Any other backend implements the same trait without touching the
/// orchestrator or the pipelines.
pub struct ZseiStoreAdapter {
    pub zsei: Arc<RwLock<ZSEI>>,
}

/// Turn the legacy flat container JSON into the full `Container` shape.
/// Already-full JSON (has `global_state` and `local_state`) is returned as is.
/// Any payload the typed shape has no slot for (an arbitrary `storage` object,
/// extra `local_state` keys such as `voice`) is written to a content file under
/// the ZSEI data dir and referenced via `object_store_path`, so nothing the
/// caller supplied is dropped.
fn normalize_container_json(
    parent_id: u64,
    v: serde_json::Value,
) -> Result<serde_json::Value, String> {
    use serde_json::{json, Value};
    if v.get("global_state").is_some() && v.get("local_state").is_some() {
        return Ok(v);
    }
    let Value::Object(mut flat) = v else {
        return Err("container JSON must be an object".to_string());
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Start from the typed defaults, then overlay what the caller gave.
    let mut local = serde_json::to_value(crate::types::container::LocalState::default())
        .map_err(|e| e.to_string())?;
    let nested_local = flat.remove("local_state").unwrap_or(Value::Null);
    let meta_in = flat
        .remove("metadata")
        .or_else(|| nested_local.get("metadata").cloned())
        .unwrap_or(Value::Null);
    let ctx_in = flat
        .remove("context")
        .or_else(|| nested_local.get("context").cloned())
        .unwrap_or(Value::Null);
    let storage_in = flat
        .remove("storage")
        .or_else(|| nested_local.get("storage").cloned())
        .unwrap_or(Value::Null);

    let type_name = flat
        .remove("container_type")
        .or_else(|| meta_in.get("container_type").cloned())
        .and_then(|t| t.as_str().map(String::from))
        .ok_or_else(|| "container JSON has no container_type".to_string())?;
    let ctype: crate::types::container::ContainerType =
        serde_json::from_value(json!(type_name))
            .map_err(|_| format!("unknown container_type '{type_name}'"))?;
    let modality = flat
        .remove("modality")
        .or_else(|| meta_in.get("modality").cloned())
        .filter(|m| m.is_string())
        .unwrap_or_else(|| json!("Unknown"));

    local["metadata"]["container_type"] = serde_json::to_value(ctype).map_err(|e| e.to_string())?;
    local["metadata"]["modality"] = modality;
    local["metadata"]["created_at"] = json!(now);
    local["metadata"]["updated_at"] = json!(now);
    local["metadata"]["provenance"] = json!(meta_in
        .get("created_by")
        .and_then(|c| c.as_str())
        .unwrap_or("orchestrator"));
    if let Some(name) = meta_in.get("name").and_then(|n| n.as_str()) {
        local["metadata"]["name"] = json!(name);
    }
    for key in ["keywords", "topics", "categories", "methodologies"] {
        if let Some(arr) = ctx_in.get(key).filter(|a| a.is_array()) {
            local["context"][key] = arr.clone();
        }
    }

    // Payload with no typed slot -> content file (never dropped).
    let known_storage = ["db_shard_id", "vector_index_ref", "object_store_path", "compression_type"];
    let mut payload = serde_json::Map::new();
    if let Value::Object(st) = &storage_in {
        if st.keys().any(|k| known_storage.contains(&k.as_str())) {
            for k in known_storage {
                if let Some(x) = st.get(k) {
                    local["storage"][k] = x.clone();
                }
            }
        } else if !st.is_empty() {
            payload.insert("storage".to_string(), storage_in.clone());
        }
    }
    if let Value::Object(nl) = &nested_local {
        for (k, val) in nl {
            if !["metadata", "context", "storage", "hints", "integrity"].contains(&k.as_str()) {
                payload.insert(k.clone(), val.clone());
            }
        }
    }
    if let Some(d) = meta_in.get("description") {
        payload.insert("description".to_string(), d.clone());
    }
    if !payload.is_empty() && local["storage"]["object_store_path"].is_null() {
        let base = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
        let rel = format!(
            "flat_payload/{}-{}.json",
            type_name.to_lowercase(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let abs = std::path::Path::new(&base).join(&rel);
        if let Some(dir) = abs.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        match std::fs::write(&abs, serde_json::to_vec_pretty(&Value::Object(payload)).unwrap_or_default()) {
            Ok(()) => local["storage"]["object_store_path"] = json!(rel),
            Err(e) => tracing::error!(path = %abs.display(), error = %e, "create_container: payload file write failed — payload not stored"),
        }
    }

    tracing::info!(container_type = %type_name, "create_container: normalized legacy flat container JSON");
    Ok(json!({
        "global_state": {
            "container_id": 0,
            "parent_id": parent_id,
            "child_ids": [],
            "child_count": 0,
            "version": 1
        },
        "local_state": local
    }))
}

#[async_trait::async_trait]
impl super::StoreAccess for ZseiStoreAdapter {
    async fn query(&self, query: serde_json::Value) -> Result<serde_json::Value, String> {
        let q: ZSEIQuery = serde_json::from_value(query).map_err(|e| e.to_string())?;
        let zsei = self.zsei.read().await;
        let result = zsei.query(q).await.map_err(|e| e.to_string())?;
        serde_json::to_value(result).map_err(|e| e.to_string())
    }

    async fn traverse(&self, request: serde_json::Value) -> Result<serde_json::Value, String> {
        let req: TraversalRequest = serde_json::from_value(request).map_err(|e| e.to_string())?;
        let zsei = self.zsei.read().await;
        let result = zsei.traverse(req).await.map_err(|e| e.to_string())?;
        serde_json::to_value(result).map_err(|e| e.to_string())
    }

    async fn create_container(
        &self,
        parent_id: u64,
        container: serde_json::Value,
    ) -> Result<u64, String> {
        // FLAT-SHAPE NORMALIZATION (2026-11-02 audit): several orchestrator sites
        // (auto-created categories, synthesized methodologies, voice identity)
        // passed a flat JSON ({container_type, metadata, context, storage}) that
        // does not deserialize into `Container` (needs global_state/local_state).
        // Their callers ignore the Err, so those containers were silently never
        // created. Normalize here — one place — and surface real failures.
        let container = normalize_container_json(parent_id, container)?;
        let c: Container = serde_json::from_value(container).map_err(|e| {
            tracing::error!(error = %e, "create_container: JSON does not form a Container");
            e.to_string()
        })?;
        let zsei = self.zsei.read().await;
        // Route through the query processor: it allocates the container id,
        // links the parent, and seeds version history.
        let result = zsei
            .query(ZSEIQuery::CreateContainer {
                parent_id,
                container: c,
            })
            .await
            .map_err(|e| e.to_string())?;
        match result {
            ZSEIQueryResult::ContainerID(id) => Ok(id),
            _ => Err("CreateContainer returned an unexpected result".to_string()),
        }
    }

    async fn update_container(
        &self,
        container_id: u64,
        updates: serde_json::Value,
    ) -> Result<(), String> {
        let u: ContainerUpdate = serde_json::from_value(updates).map_err(|e| e.to_string())?;
        let zsei = self.zsei.read().await;
        let result = zsei
            .query(ZSEIQuery::UpdateContainer {
                container_id,
                updates: u,
            })
            .await
            .map_err(|e| e.to_string())?;
        match result {
            ZSEIQueryResult::Success => Ok(()),
            _ => Err("UpdateContainer returned an unexpected result".to_string()),
        }
    }

    async fn get_container(
        &self,
        container_id: u64,
    ) -> Result<Option<serde_json::Value>, String> {
        let zsei = self.zsei.read().await;
        let container = zsei
            .get_container(container_id)
            .await
            .map_err(|e| e.to_string())?;
        match container {
            Some(c) => serde_json::to_value(c)
                .map(Some)
                .map_err(|e| e.to_string()),
            None => Ok(None),
        }
    }

    async fn search_by_keywords(
        &self,
        keywords: &[String],
        container_type: Option<&str>,
    ) -> Result<Vec<u64>, String> {
        let q = match container_type {
            Some("Methodology") => {
                ZSEIQuery::GetMethodologiesByKeywords {
                    keywords: keywords.to_vec(),
                }
            }
            Some("Blueprint") => {
                ZSEIQuery::SearchBlueprintsByKeywords {
                    keywords: keywords.to_vec(),
                }
            }
            other => ZSEIQuery::SearchContainersByKeywords {
                keywords: keywords.to_vec(),
                container_type: other.map(String::from),
                // JurisdictionRuleSet lookups are exact-match on a real
                // ISO-3166-1 alpha-2 region code ("eu", "gb", ...) — the
                // default "scan" strategy's length/stopword filter treats
                // any 2-char keyword as noise and silently drops it,
                // confirmed live to make every national-scope jurisdiction
                // lookup return zero results regardless of real content
                // existing. Every other caller keeps today's default
                // ("scan") behavior unchanged.
                strategy: if other == Some("JurisdictionRuleSet") || other == Some("ChatSession") {
                    Some("exact".to_string())
                } else {
                    None
                },
            },
        };
        let zsei = self.zsei.read().await;
        let result = zsei.query(q).await.map_err(|e| e.to_string())?;
        match result {
            ZSEIQueryResult::Containers(ids) => Ok(ids),
            _ => Err("Keyword search returned an unexpected result".to_string()),
        }
    }

    async fn get_categories(&self, modality: &str) -> Result<Vec<u64>, String> {
        use crate::types::container::Modality;
        let m: Modality = serde_json::from_value(serde_json::Value::String(
            modality.to_string(),
        ))
        .map_err(|e| e.to_string())?;
        let zsei = self.zsei.read().await;
        let result = zsei
            .query(ZSEIQuery::GetCategories {
                modality: m,
                parent_category: None,
            })
            .await
            .map_err(|e| e.to_string())?;
        match result {
            ZSEIQueryResult::Containers(ids) => Ok(ids),
            _ => Err("GetCategories returned an unexpected result".to_string()),
        }
    }
}

#[cfg(test)]
mod normalize_tests {
    use super::normalize_container_json;
    use crate::types::container::{Container, ContainerType};

    #[test]
    fn flat_category_becomes_a_real_container() {
        let flat = serde_json::json!({
            "container_type": "Category",
            "modality": "Text",
            "metadata": { "name": "physics", "description": "auto" },
            "context": { "keywords": ["physics"], "topics": [] }
        });
        let v = normalize_container_json(0, flat).unwrap();
        let c: Container = serde_json::from_value(v).unwrap();
        assert_eq!(c.local_state.metadata.container_type, ContainerType::Category);
        assert_eq!(c.local_state.metadata.name.as_deref(), Some("physics"));
        assert_eq!(c.local_state.context.keywords, vec!["physics".to_string()]);
    }

    #[test]
    fn unknown_type_is_a_loud_error_not_a_silent_drop() {
        let flat = serde_json::json!({ "container_type": "NoSuchType" });
        assert!(normalize_container_json(0, flat).unwrap_err().contains("unknown container_type"));
    }

    #[test]
    fn full_shape_passes_through_untouched() {
        let full = serde_json::json!({ "global_state": {}, "local_state": {} });
        assert_eq!(normalize_container_json(5, full.clone()).unwrap(), full);
    }
}
