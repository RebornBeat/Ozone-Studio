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
        let registry = self.registry.read().await;
        // Blueprint must exist before execution (registry owns the map).
        if registry.get_blueprint(pipeline_id).await.is_none() {
            return Err(format!("Pipeline {} not found", pipeline_id));
        }

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

        let pipeline_input = PipelineInput {
            data,
            context: ExecutionContext::default(),
        };

        let output = registry
            .execute(pipeline_id, pipeline_input, None)
            .await
            .map_err(|e| e.to_string())?;

        serde_json::to_value(&output).map_err(|e| e.to_string())
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
        let c: Container = serde_json::from_value(container).map_err(|e| e.to_string())?;
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
