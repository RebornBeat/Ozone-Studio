//! Methodology Store — registers methodologies as ZSEI containers
//!
//! On startup, every entry in methodologies/index.json becomes a first-class
//! ZSEI Container under /Methodologies/ (METHODOLOGY_ROOT_ID).
//! This enables full structural + semantic + contextual traversal.

use crate::types::container::{
    methodology_container_id, Container, ContainerType, Context, GlobalState, IntegrityData,
    LocalState, Metadata, Modality, StoragePointers, TraversalHints, METHODOLOGY_ROOT_ID,
};
use crate::types::index::MethodologyIndex;
use crate::types::{ContainerID, OzoneError, OzoneResult};
use crate::zsei::ZSEI;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct MethodologyStore {
    zsei: Arc<RwLock<ZSEI>>,
    index_path: PathBuf,
}

impl MethodologyStore {
    pub fn new(zsei: Arc<RwLock<ZSEI>>, index_path: PathBuf) -> Self {
        Self { zsei, index_path }
    }

    /// Register all methodologies from index.json as ZSEI containers.
    /// Safe to call on every startup (checks existence before creating).
    pub async fn register_all(&self) -> OzoneResult<()> {
        let content = std::fs::read_to_string(&self.index_path).map_err(|e| {
            OzoneError::StorageError(format!(
                "Failed to read methodology index at {}: {}",
                self.index_path.display(),
                e
            ))
        })?;

        let index: MethodologyIndex = serde_json::from_str(&content).map_err(|e| {
            OzoneError::StorageError(format!("Failed to parse methodology index: {}", e))
        })?;

        let mut zsei = self.zsei.write().await;

        // Ensure the methodology root container exists
        self.ensure_root(&mut zsei).await?;

        let mut registered = 0usize;
        for entry in &index.methodologies {
            let container_id = methodology_container_id(entry.methodology_id);

            // Idempotent: skip if already registered
            if zsei.get_container(container_id).await?.is_some() {
                continue;
            }

            let mut keywords = entry
                .categories
                .iter()
                .map(|c| c.to_lowercase())
                .collect::<Vec<_>>();
            keywords.push(entry.name.to_lowercase());
            // Also split name by spaces for better keyword search
            keywords.extend(entry.name.split_whitespace().map(|w| w.to_lowercase()));
            keywords.dedup();

            let container = Container {
                global_state: GlobalState {
                    container_id,
                    parent_id: METHODOLOGY_ROOT_ID,
                    child_ids: vec![],
                    child_count: 0,
                    version: 1,
                },
                local_state: LocalState {
                    metadata: Metadata {
                        container_type: ContainerType::Methodology,
                        modality: Modality::Unknown,
                        created_at: now(),
                        updated_at: now(),
                        provenance: "bootstrap".to_string(),
                        permissions: 0,
                        owner_id: 0,
                        name: Some(entry.name.clone()),
                        materialized_path: Some(format!("/Methodologies/{}", entry.name)),
                    },
                    context: Context {
                        categories: vec![],
                        methodologies: vec![],
                        keywords,
                        topics: entry.categories.clone(),
                        relationships: vec![],
                        learned_associations: vec![],
                        embedding: None,
                    },
                    storage: StoragePointers {
                        db_shard_id: None,
                        vector_index_ref: None,
                        object_store_path: Some(format!("methodologies/{}", entry.file)),
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

        // Update root's child list after all are registered
        if registered > 0 {
            self.update_root_children(&mut zsei, &index).await?;
        }

        tracing::info!(
            "Methodology store: {} new registrations ({} total in index)",
            registered,
            index.methodologies.len()
        );
        Ok(())
    }

    async fn ensure_root(&self, zsei: &mut ZSEI) -> OzoneResult<()> {
        if zsei.get_container(METHODOLOGY_ROOT_ID).await?.is_some() {
            return Ok(());
        }
        let root = Container {
            global_state: GlobalState {
                container_id: METHODOLOGY_ROOT_ID,
                parent_id: 0,
                child_ids: vec![],
                child_count: 0,
                version: 1,
            },
            local_state: LocalState {
                metadata: Metadata {
                    container_type: ContainerType::MethodologyRoot,
                    modality: Modality::Unknown,
                    created_at: now(),
                    updated_at: now(),
                    provenance: "bootstrap".to_string(),
                    permissions: 0,
                    owner_id: 0,
                    name: Some("Methodologies".to_string()),
                    materialized_path: Some("/Methodologies".to_string()),
                },
                context: Context {
                    keywords: vec!["methodology".to_string(), "methodologies".to_string()],
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
        index: &MethodologyIndex,
    ) -> OzoneResult<()> {
        if let Some(mut root) = zsei.get_container(METHODOLOGY_ROOT_ID).await? {
            let child_ids: Vec<u64> = index
                .methodologies
                .iter()
                .map(|e| methodology_container_id(e.methodology_id))
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
