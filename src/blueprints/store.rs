//! Blueprint Store — registers blueprints as ZSEI containers

use crate::bootstrap::{blueprint_container_id, BLUEPRINT_ROOT_ID};
use crate::types::container::{
    Container, ContainerType, Context, GlobalState, IntegrityData, LocalState, Metadata, Modality,
    StoragePointers, TraversalHints,
};
use crate::types::index::BlueprintIndex;
use crate::types::{OzoneError, OzoneResult};
use crate::zsei::ZSEI;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct BlueprintStore {
    zsei: Arc<RwLock<ZSEI>>,
    index_path: PathBuf,
}

impl BlueprintStore {
    pub fn new(zsei: Arc<RwLock<ZSEI>>, index_path: PathBuf) -> Self {
        Self { zsei, index_path }
    }

    pub async fn register_all(&self) -> OzoneResult<()> {
        let content = std::fs::read_to_string(&self.index_path).map_err(|e| {
            OzoneError::StorageError(format!(
                "Failed to read blueprint index at {}: {}",
                self.index_path.display(),
                e
            ))
        })?;

        let index: BlueprintIndex = serde_json::from_str(&content).map_err(|e| {
            OzoneError::StorageError(format!("Failed to parse blueprint index: {}", e))
        })?;

        let mut zsei = self.zsei.write().await;
        self.ensure_root(&mut zsei).await?;

        let mut registered = 0usize;
        for entry in &index.blueprints {
            let container_id = blueprint_container_id(entry.blueprint_id);

            if zsei.get_container(container_id).await?.is_some() {
                continue;
            }

            let mut keywords = entry.keywords.clone();
            keywords.push(entry.name.to_lowercase());
            keywords.extend(entry.name.split_whitespace().map(|w| w.to_lowercase()));
            // Include input/output types as keywords for matching
            keywords.extend(entry.input_types.iter().map(|t| t.to_lowercase()));
            keywords.push(entry.output_types.join(" ").to_lowercase());
            keywords.dedup();

            let container = Container {
                global_state: GlobalState {
                    container_id,
                    parent_id: BLUEPRINT_ROOT_ID,
                    child_ids: vec![],
                    child_count: 0,
                    version: 1,
                },
                local_state: LocalState {
                    metadata: Metadata {
                        container_type: ContainerType::Blueprint,
                        modality: Modality::Unknown,
                        created_at: now(),
                        updated_at: now(),
                        provenance: "bootstrap".to_string(),
                        permissions: 0,
                        owner_id: 0,
                        name: Some(entry.name.clone()),
                        materialized_path: Some(format!("/Blueprints/{}", entry.name)),
                    },
                    context: Context {
                        categories: vec![],
                        methodologies: vec![],
                        keywords,
                        topics: entry.input_types.clone(),
                        relationships: vec![],
                        learned_associations: vec![],
                        embedding: None,
                    },
                    storage: StoragePointers {
                        db_shard_id: None,
                        vector_index_ref: None,
                        object_store_path: Some(format!("blueprints/{}", entry.file)),
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
            self.update_root_children(&mut zsei, &index).await?;
        }

        tracing::info!(
            "Blueprint store: {} new registrations ({} total in index)",
            registered,
            index.blueprints.len()
        );
        Ok(())
    }

    async fn ensure_root(&self, zsei: &mut ZSEI) -> OzoneResult<()> {
        if zsei.get_container(BLUEPRINT_ROOT_ID).await?.is_some() {
            return Ok(());
        }
        let root = Container {
            global_state: GlobalState {
                container_id: BLUEPRINT_ROOT_ID,
                parent_id: 0,
                child_ids: vec![],
                child_count: 0,
                version: 1,
            },
            local_state: LocalState {
                metadata: Metadata {
                    container_type: ContainerType::BlueprintRoot,
                    modality: Modality::Unknown,
                    created_at: now(),
                    updated_at: now(),
                    provenance: "bootstrap".to_string(),
                    permissions: 0,
                    owner_id: 0,
                    name: Some("Blueprints".to_string()),
                    materialized_path: Some("/Blueprints".to_string()),
                },
                context: Context {
                    keywords: vec!["blueprint".to_string(), "blueprints".to_string()],
                    ..Default::default()
                },
                ..Default::default()
            },
        };
        zsei.store_container(root).await?;
        Ok(())
    }

    async fn update_root_children(
        &self,
        zsei: &mut ZSEI,
        index: &BlueprintIndex,
    ) -> OzoneResult<()> {
        if let Some(mut root) = zsei.get_container(BLUEPRINT_ROOT_ID).await? {
            let child_ids: Vec<u64> = index
                .blueprints
                .iter()
                .map(|e| blueprint_container_id(e.blueprint_id))
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
