//! Network module for P2P synchronization
//!
//! Based on Section 15-17 of the specification.
//!
//! Handles:
//! - P2P peer discovery and connection
//! - Hook-based sync triggers for methodologies, blueprints, and findings
//! - Network status monitoring
//!
//! SYNC BEHAVIOR:
//! - Methodologies: ALWAYS sync (no significance check)
//! - Blueprints: ALWAYS sync (no significance check)  
//! - New Container Findings: ALWAYS sync (queue for batch)
//! - Pipeline Results: ON-DEMAND only (explicit share request)
//! - User Data: NEVER sync (local only)

use crate::types::{ContainerID, OzoneResult, OzoneError};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

/// Sync priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncPriority {
    /// Immediate broadcast (methodologies, blueprints)
    High,
    /// Batch sync on next cycle (new findings)
    Normal,
    /// On-demand only (pipeline results)
    Low,
    /// Never sync (user data)
    Never,
}

/// Sync item to be queued
#[derive(Debug, Clone)]
pub struct SyncItem {
    pub container_id: ContainerID,
    pub item_type: SyncItemType,
    pub priority: SyncPriority,
    pub data: Vec<u8>,
    pub timestamp: u64,
}

/// Type of item being synced
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncItemType {
    Methodology,
    Blueprint,
    Container,
    PipelineResult,
}

/// Hook function type
pub type HookFn = Arc<dyn Fn(SyncItem) -> OzoneResult<()> + Send + Sync>;

/// Network manager for P2P operations
pub struct NetworkManager {
    /// P2P node identity
    peer_id: Option<String>,
    
    /// Connected peers
    peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
    
    /// Sync queue for high priority items (immediate)
    high_priority_queue: Arc<RwLock<Vec<SyncItem>>>,
    
    /// Sync queue for normal priority items (batched)
    normal_queue: Arc<RwLock<Vec<SyncItem>>>,
    
    /// Registered hooks
    hooks: Arc<RwLock<HashMap<String, Vec<HookFn>>>>,
    
    /// Channel for sync notifications
    sync_sender: mpsc::Sender<SyncItem>,
    sync_receiver: Arc<RwLock<mpsc::Receiver<SyncItem>>>,
    
    /// Network enabled flag
    enabled: bool,
    
    /// Bootstrap nodes
    bootstrap_nodes: Vec<String>,
}

/// Peer information
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub peer_id: String,
    pub address: String,
    pub last_seen: u64,
    pub latency_ms: u32,
    pub reputation: f32,
    pub contribution_count: u64,
}

impl NetworkManager {
    /// Create a new network manager
    pub fn new(enabled: bool, bootstrap_nodes: Vec<String>) -> Self {
        let (tx, rx) = mpsc::channel(1000);
        
        Self {
            peer_id: None,
            peers: Arc::new(RwLock::new(HashMap::new())),
            high_priority_queue: Arc::new(RwLock::new(Vec::new())),
            normal_queue: Arc::new(RwLock::new(Vec::new())),
            hooks: Arc::new(RwLock::new(HashMap::new())),
            sync_sender: tx,
            sync_receiver: Arc::new(RwLock::new(rx)),
            enabled,
            bootstrap_nodes,
        }
    }
    
    /// Initialize P2P network
    pub async fn initialize(&mut self) -> OzoneResult<()> {
        if !self.enabled {
            tracing::info!("P2P networking disabled");
            return Ok(());
        }
        
        // Generate peer ID
        let peer_id = self.generate_peer_id();
        self.peer_id = Some(peer_id.clone());
        
        tracing::info!("Network initialized with peer ID: {}", peer_id);
        
        // Register default hooks
        self.register_default_hooks().await;
        
        // Connect to bootstrap nodes
        for node in &self.bootstrap_nodes.clone() {
            if let Err(e) = self.connect_to_peer(node).await {
                tracing::warn!("Failed to connect to bootstrap node {}: {}", node, e);
            }
        }
        
        Ok(())
    }
    
    /// Generate a peer ID
    fn generate_peer_id(&self) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bytes: [u8; 32] = rng.gen();
        format!("12D3KooW{}", hex::encode(&bytes[..16]))
    }
    
    /// Register default hooks for automatic sync
    async fn register_default_hooks(&self) {
        // Methodology hook - ALWAYS sync
        self.register_hook("on_methodology_created", Arc::new(|item: SyncItem| {
            tracing::info!("Methodology created, queuing for immediate sync: {}", item.container_id);
            Ok(())
        })).await;
        
        // Blueprint hook - ALWAYS sync
        self.register_hook("on_blueprint_created", Arc::new(|item: SyncItem| {
            tracing::info!("Blueprint created, queuing for immediate sync: {}", item.container_id);
            Ok(())
        })).await;
        
        // Container finding hook - ALWAYS sync (batched)
        self.register_hook("on_container_created", Arc::new(|item: SyncItem| {
            tracing::debug!("Container created, queuing for batch sync: {}", item.container_id);
            Ok(())
        })).await;
        
        // Pipeline result hook - ON-DEMAND only
        self.register_hook("on_pipeline_completed", Arc::new(|item: SyncItem| {
            // Only sync if explicitly requested
            if item.priority != SyncPriority::Low {
                tracing::debug!("Pipeline result shared: {}", item.container_id);
            }
            Ok(())
        })).await;
    }
    
    /// Register a hook for an event
    pub async fn register_hook(&self, event: &str, hook: HookFn) {
        let mut hooks = self.hooks.write().await;
        hooks.entry(event.to_string())
            .or_insert_with(Vec::new)
            .push(hook);
    }
    
    /// Trigger hooks for an event
    pub async fn trigger_hooks(&self, event: &str, item: SyncItem) -> OzoneResult<()> {
        let hooks = self.hooks.read().await;
        
        if let Some(event_hooks) = hooks.get(event) {
            for hook in event_hooks {
                hook(item.clone())?;
            }
        }
        
        // Also queue for sync based on priority
        match item.priority {
            SyncPriority::High => {
                self.high_priority_queue.write().await.push(item.clone());
                // Immediately trigger broadcast
                self.broadcast_high_priority().await?;
            }
            SyncPriority::Normal => {
                self.normal_queue.write().await.push(item);
            }
            SyncPriority::Low => {
                // Only queue if explicitly requested
            }
            SyncPriority::Never => {
                // Never sync
            }
        }
        
        Ok(())
    }
    
    /// Notify of new methodology (ALWAYS syncs)
    pub async fn on_methodology_created(&self, container_id: ContainerID, data: Vec<u8>) -> OzoneResult<()> {
        let item = SyncItem {
            container_id,
            item_type: SyncItemType::Methodology,
            priority: SyncPriority::High,
            data,
            timestamp: chrono::Utc::now().timestamp() as u64,
        };
        
        self.trigger_hooks("on_methodology_created", item).await
    }
    
    /// Notify of new blueprint (ALWAYS syncs)
    pub async fn on_blueprint_created(&self, container_id: ContainerID, data: Vec<u8>) -> OzoneResult<()> {
        let item = SyncItem {
            container_id,
            item_type: SyncItemType::Blueprint,
            priority: SyncPriority::High,
            data,
            timestamp: chrono::Utc::now().timestamp() as u64,
        };
        
        self.trigger_hooks("on_blueprint_created", item).await
    }
    
    /// Notify of new container finding (ALWAYS syncs, batched)
    pub async fn on_container_created(&self, container_id: ContainerID, data: Vec<u8>) -> OzoneResult<()> {
        let item = SyncItem {
            container_id,
            item_type: SyncItemType::Container,
            priority: SyncPriority::Normal,
            data,
            timestamp: chrono::Utc::now().timestamp() as u64,
        };
        
        self.trigger_hooks("on_container_created", item).await
    }
    
    /// Notify of pipeline completion (ON-DEMAND only)
    pub async fn on_pipeline_completed(
        &self, 
        container_id: ContainerID, 
        data: Vec<u8>,
        share_requested: bool,
    ) -> OzoneResult<()> {
        if !share_requested {
            return Ok(());
        }
        
        let item = SyncItem {
            container_id,
            item_type: SyncItemType::PipelineResult,
            priority: SyncPriority::Low,
            data,
            timestamp: chrono::Utc::now().timestamp() as u64,
        };
        
        self.trigger_hooks("on_pipeline_completed", item).await
    }
    
    /// Broadcast high priority items immediately
    async fn broadcast_high_priority(&self) -> OzoneResult<()> {
        let items = {
            let mut queue = self.high_priority_queue.write().await;
            std::mem::take(&mut *queue)
        };
        
        if items.is_empty() {
            return Ok(());
        }
        
        let peers = self.peers.read().await;
        
        for item in items {
            for (peer_id, peer) in peers.iter() {
                if let Err(e) = self.send_to_peer(peer, &item).await {
                    tracing::warn!("Failed to send to peer {}: {}", peer_id, e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Process batch sync (called periodically)
    pub async fn process_batch_sync(&self) -> OzoneResult<()> {
        let items = {
            let mut queue = self.normal_queue.write().await;
            std::mem::take(&mut *queue)
        };
        
        if items.is_empty() {
            return Ok(());
        }
        
        tracing::info!("Processing batch sync of {} items", items.len());
        
        let peers = self.peers.read().await;
        
        // Group items and send in batches
        for item in items {
            for (peer_id, peer) in peers.iter() {
                if let Err(e) = self.send_to_peer(peer, &item).await {
                    tracing::warn!("Failed to batch send to peer {}: {}", peer_id, e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Connect to a peer
    async fn connect_to_peer(&self, address: &str) -> OzoneResult<()> {
        // TODO: Implement actual P2P connection using libp2p
        tracing::info!("Connecting to peer at {}", address);
        
        let peer_info = PeerInfo {
            peer_id: format!("peer_{}", address),
            address: address.to_string(),
            last_seen: chrono::Utc::now().timestamp() as u64,
            latency_ms: 0,
            reputation: 1.0,
            contribution_count: 0,
        };
        
        self.peers.write().await.insert(peer_info.peer_id.clone(), peer_info);
        
        Ok(())
    }
    
    /// Send item to a peer
    async fn send_to_peer(&self, _peer: &PeerInfo, item: &SyncItem) -> OzoneResult<()> {
        // TODO: Implement actual P2P send using libp2p
        tracing::debug!("Sending {:?} {} to peer", item.item_type, item.container_id);
        Ok(())
    }
    
    /// Get network status
    pub async fn get_status(&self) -> NetworkStatus {
        let peers = self.peers.read().await;
        let high_queue = self.high_priority_queue.read().await;
        let normal_queue = self.normal_queue.read().await;
        
        NetworkStatus {
            enabled: self.enabled,
            peer_id: self.peer_id.clone(),
            connected_peers: peers.len(),
            pending_high_priority: high_queue.len(),
            pending_normal: normal_queue.len(),
        }
    }
}

/// Network status summary
#[derive(Debug, Clone)]
pub struct NetworkStatus {
    pub enabled: bool,
    pub peer_id: Option<String>,
    pub connected_peers: usize,
    pub pending_high_priority: usize,
    pub pending_normal: usize,
}
