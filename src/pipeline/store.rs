//! Pipeline Store — registers pipelines as ZSEI containers
//!
//! Uses compile-time PIPELINE_INFO as the source of truth since it has all
//! required fields (has_ui, is_tab, folder_name) that the index.json type lacks.

use crate::bootstrap::{pipeline_container_id, CONSCIOUSNESS_ROOT_ID, PIPELINE_ROOT_ID};
use crate::pipeline::registry::PIPELINE_INFO;
use crate::types::container::{
    Container, ContainerType, Context, GlobalState, IntegrityData, LocalState, Metadata, Modality,
    StoragePointers, TraversalHints,
};
use crate::types::{OzoneError, OzoneResult};
use crate::zsei::ZSEI;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct PipelineStore {
    zsei: Arc<RwLock<ZSEI>>,
}

impl PipelineStore {
    pub fn new(zsei: Arc<RwLock<ZSEI>>) -> Self {
        Self { zsei }
    }

    pub async fn register_all(&self) -> OzoneResult<()> {
        let mut zsei = self.zsei.write().await;
        self.ensure_roots(&mut zsei).await?;

        let mut registered = 0usize;
        for (pipeline_id, info) in PIPELINE_INFO.iter() {
            let container_id = pipeline_container_id(*pipeline_id);

            if zsei.get_container(container_id).await?.is_some() {
                continue;
            }

            let parent_id = if info.category == "consciousness" {
                CONSCIOUSNESS_ROOT_ID
            } else {
                PIPELINE_ROOT_ID
            };

            let mut keywords = vec![
                info.name.to_lowercase(),
                "pipeline".to_string(),
                info.category.to_string(),
            ];
            keywords.extend(info.name.split_whitespace().map(|w| w.to_lowercase()));
            keywords.extend(info.folder_name.split('_').map(|w| w.to_lowercase()));
            keywords.dedup();

            let materialized_path = format!("/Pipelines/{}/{}", info.category, info.folder_name);

            let container = Container {
                global_state: GlobalState {
                    container_id,
                    parent_id,
                    child_ids: vec![],
                    child_count: 0,
                    version: 1,
                },
                local_state: LocalState {
                    metadata: Metadata {
                        container_type: ContainerType::Pipeline,
                        modality: Modality::Unknown,
                        created_at: now(),
                        updated_at: now(),
                        provenance: "bootstrap".to_string(),
                        permissions: 0,
                        owner_id: 0,
                        name: Some(info.name.clone()),
                        materialized_path: Some(materialized_path),
                    },
                    context: Context {
                        categories: vec![],
                        methodologies: vec![],
                        keywords,
                        topics: vec![info.category.to_string()],
                        relationships: vec![],
                        learned_associations: vec![],
                        embedding: None,
                    },
                    storage: StoragePointers {
                        db_shard_id: None,
                        vector_index_ref: None,
                        // Points to the pipeline's folder (not a single file — pipelines are dirs)
                        object_store_path: Some(format!(
                            "pipelines/{}/{}",
                            info.category, info.folder_name
                        )),
                        compression_type: crate::types::container::CompressionType::None,
                    },
                    hints: TraversalHints::default(),
                    integrity: IntegrityData::default(),
                    file_context: None,
                    code_context: None,
                    text_context: None,
                    external_ref: None,
                },
            };

            zsei.store_container(container).await?;
            registered += 1;
        }

        if registered > 0 {
            self.update_pipeline_root_children(&mut zsei).await?;
        }

        tracing::info!(
            "Pipeline store: {} new registrations ({} total in registry)",
            registered,
            PIPELINE_INFO.len()
        );
        Ok(())
    }

    async fn ensure_roots(&self, zsei: &mut ZSEI) -> OzoneResult<()> {
        let roots = [(
            PIPELINE_ROOT_ID,
            "Pipelines",
            "/Pipelines",
            ContainerType::PipelineRoot,
        )];
        for (id, name, path, ct) in roots {
            if zsei.get_container(id).await?.is_some() {
                continue;
            }
            let root = Container {
                global_state: GlobalState {
                    container_id: id,
                    parent_id: 0,
                    child_ids: vec![],
                    child_count: 0,
                    version: 1,
                },
                local_state: LocalState {
                    metadata: Metadata {
                        container_type: ct,
                        modality: Modality::Unknown,
                        created_at: now(),
                        updated_at: now(),
                        provenance: "bootstrap".to_string(),
                        permissions: 0,
                        owner_id: 0,
                        name: Some(name.to_string()),
                        materialized_path: Some(path.to_string()),
                    },
                    context: Context {
                        keywords: vec!["pipeline".to_string()],
                        ..Default::default()
                    },
                    ..Default::default()
                },
            };
            zsei.store_container(root).await?;
        }
        Ok(())
    }

    async fn update_pipeline_root_children(&self, zsei: &mut ZSEI) -> OzoneResult<()> {
        if let Some(mut root) = zsei.get_container(PIPELINE_ROOT_ID).await? {
            let child_ids: Vec<u64> = PIPELINE_INFO
                .iter()
                .filter(|(_, info)| info.category != "consciousness")
                .map(|(id, _)| pipeline_container_id(*id))
                .collect();
            root.global_state.child_ids = child_ids.clone();
            root.global_state.child_count = child_ids.len() as u32;
            zsei.store_container(root).await?;
        }
        Ok(())
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
