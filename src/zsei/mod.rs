//! ZSEI - Zero-Shot Embedded Indexer
//! 
//! The knowledge fabric that everything runs through.
//! Based on Section 6 of the specification.
//!
//! # Architecture
//!
//! ZSEI stores containers with:
//! - GlobalState: mmap-friendly ID lists (parent/child relationships)
//! - LocalState: metadata, context, storage pointers, traversal hints
//!
//! # Key Principles
//!
//! - Structure before intelligence
//! - Compression before learning
//! - Traversal before generation
//! - Context not copies (link files, store semantic meaning)
//! - Zero-shot discovery (no task-specific training)

mod storage;
mod traversal;
mod query;

pub use storage::*;
pub use traversal::*;
pub use query::*;

use crate::config::ZSEIConfig;
use crate::types::{ContainerID, OzoneResult};
use crate::types::container::Container;
use crate::types::zsei::{ZSEIQuery, ZSEIQueryResult, TraversalRequest, TraversalResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main ZSEI instance
pub struct ZSEI {
    /// Configuration
    config: ZSEIConfig,
    
    /// Container storage (mmap-backed) - wrapped in RwLock for interior mutability
    storage: Arc<RwLock<ContainerStorage>>,
    
    /// In-memory cache for hot containers
    cache: Arc<RwLock<HashMap<ContainerID, Container>>>,
    
    /// Traversal engine
    traversal: TraversalEngine,
    
    /// Query processor
    query_processor: Arc<RwLock<QueryProcessor>>,
}

impl ZSEI {
    /// Create a new ZSEI instance
    pub fn new(config: &ZSEIConfig) -> OzoneResult<Self> {
        tracing::info!("Initializing ZSEI");
        
        // Initialize storage
        let storage = ContainerStorage::new(config)?;
        
        // Initialize traversal engine
        let traversal = TraversalEngine::new(config)?;
        
        // Initialize query processor
        let query_processor = QueryProcessor::new();
        
        Ok(Self {
            config: config.clone(),
            storage: Arc::new(RwLock::new(storage)),
            cache: Arc::new(RwLock::new(HashMap::new())),
            traversal,
            query_processor: Arc::new(RwLock::new(query_processor)),
        })
    }
    
    /// Query ZSEI
    pub async fn query(&self, query: ZSEIQuery) -> OzoneResult<ZSEIQueryResult> {
        let mut qp = self.query_processor.write().await;
        let mut storage = self.storage.write().await;
        qp.process(&mut storage, &self.traversal, query).await
    }
    
    /// Get a container by ID
    pub async fn get_container(&self, id: ContainerID) -> OzoneResult<Option<Container>> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(container) = cache.get(&id) {
                return Ok(Some(container.clone()));
            }
        }
        
        // Load from storage
        let storage = self.storage.read().await;
        let container = storage.load(id)?;
        
        // Update cache if found
        if let Some(ref c) = container {
            let mut cache = self.cache.write().await;
            if cache.len() < self.config.max_containers_in_memory {
                cache.insert(id, c.clone());
            }
        }
        
        Ok(container)
    }
    
    /// Store a container
    pub async fn store_container(&self, container: Container) -> OzoneResult<ContainerID> {
        let id = container.global_state.container_id;
        
        // Store to disk
        {
            let mut storage = self.storage.write().await;
            storage.store(&container)?;
        }
        
        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(id, container);
        }
        
        Ok(id)
    }
    
    /// Traverse from a starting container
    pub async fn traverse(&self, request: TraversalRequest) -> OzoneResult<TraversalResult> {
        let storage = self.storage.read().await;
        self.traversal.traverse(&storage, request).await
    }
    
    /// Get root container ID
    pub fn root_id(&self) -> ContainerID {
        0 // Root is always ID 0
    }
}
