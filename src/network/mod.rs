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

use crate::config::NetworkConfig;
use crate::types::{ContainerID, OzoneError, OzoneResult};
use libp2p::{gossipsub, identify, mdns, ping, PeerId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};

#[allow(unused_imports)]
use tracing;

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
    /// Sync queue for high priority items (immediate)
    high_priority_queue: Arc<RwLock<Vec<SyncItem>>>,

    /// Sync queue for normal priority items (batched)
    normal_queue: Arc<RwLock<Vec<SyncItem>>>,

    /// Registered hooks
    hooks: Arc<RwLock<HashMap<String, Vec<HookFn>>>>,

    /// Channel for sync notifications
    sync_sender: mpsc::Sender<SyncItem>,
    sync_receiver: Arc<RwLock<mpsc::Receiver<SyncItem>>>,

    /// libp2p swarm
    swarm: Arc<Mutex<libp2p::Swarm<OzoneBehaviour>>>,

    /// Network configuration
    config: NetworkConfig,

    /// Known peers by libp2p PeerId (persistent)
    known_peers: Arc<RwLock<HashMap<PeerId, PeerInfo>>>,

    /// Connection attempt tracking
    connection_attempts: HashMap<String, std::time::Instant>,

    /// Network statistics
    stats: Arc<Mutex<NetworkStats>>,

    /// Local libp2p peer ID
    local_peer_id: PeerId,
}

/// Peer information
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub peer_id: String,
    pub address: String,
    pub addresses: Vec<String>,
    pub last_seen: u64,
    pub latency_ms: u32,
    pub reputation: f32,
    pub contribution_count: u64,
    pub capabilities: Vec<String>,
}

/// Network statistics
#[derive(Debug, Clone, Default)]
pub struct NetworkStats {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

/// Get current unix timestamp
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Combined network behaviour
#[derive(libp2p::swarm::NetworkBehaviour)]
struct OzoneBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
    ping: ping::Behaviour,
    identify: identify::Behaviour,
}

impl NetworkManager {
    pub async fn new(config: NetworkConfig) -> OzoneResult<Self> {
        use libp2p::{
            gossipsub, identify, identity, mdns, noise, ping, SwarmBuilder, tcp, yamux,
        };

        // Generate or load keypair
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        log::info!("Local peer ID: {}", local_peer_id);

        // Configure gossipsub
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(std::time::Duration::from_secs(1))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .build()
            .map_err(|e| OzoneError::NetworkError(format!("Gossipsub config error: {}", e)))?;

        let gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(local_key.clone()),
            gossipsub_config,
        )
        .map_err(|e| OzoneError::NetworkError(format!("Gossipsub init error: {}", e)))?;

        // Configure mDNS for local peer discovery
        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)
            .map_err(|e| OzoneError::NetworkError(format!("mDNS init error: {}", e)))?;

        // Build the swarm
        let swarm = SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )
            .map_err(|e| OzoneError::NetworkError(format!("TCP config error: {}", e)))?
            .with_behaviour(|_key| {
                Ok(OzoneBehaviour {
                    gossipsub,
                    mdns,
                    ping: ping::Behaviour::default(),
                    identify: identify::Behaviour::new(identify::Config::new(
                        "/ozone/1.0.0".into(),
                        _key.public(),
                    )),
                })
            })
            .map_err(|e| OzoneError::NetworkError(format!("Behaviour error: {}", e)))?
            .build();

        let (sync_sender, sync_receiver) = mpsc::channel(1000);

        Ok(Self {
            high_priority_queue: Arc::new(RwLock::new(Vec::new())),
            normal_queue: Arc::new(RwLock::new(Vec::new())),
            hooks: Arc::new(RwLock::new(HashMap::new())),
            sync_sender,
            sync_receiver: Arc::new(RwLock::new(sync_receiver)),
            swarm: Arc::new(Mutex::new(swarm)),
            config,
            known_peers: Arc::new(RwLock::new(HashMap::new())),
            connection_attempts: HashMap::new(),
            stats: Arc::new(Mutex::new(NetworkStats::default())),
            local_peer_id,
        })
    }

    /// Initialize P2P network
    pub async fn initialize(&mut self) -> OzoneResult<()> {
        if !self.config.enable_p2p {
            tracing::info!("P2P networking disabled");
            return Ok(());
        }

        tracing::info!("Network initialized with peer ID: {}", self.local_peer_id);

        // Register default hooks
        self.register_default_hooks().await;

        // Reconnect to known peers from persistence
        let peers = self.known_peers.read().await;
        let addrs: Vec<String> = peers.values().map(|p| p.address.clone()).collect();
        drop(peers);

        for addr in addrs {
            if let Err(e) = self.connect_peer(&addr).await {
                tracing::warn!("Failed to reconnect to peer {}: {}", addr, e);
            }
        }

        Ok(())
    }

    /// Register default hooks for automatic sync
    async fn register_default_hooks(&self) {
        // Methodology hook - ALWAYS sync
        self.register_hook(
            "on_methodology_created",
            Arc::new(|item: SyncItem| {
                tracing::info!(
                    "Methodology created, queuing for immediate sync: {}",
                    item.container_id
                );
                Ok(())
            }),
        )
        .await;

        // Blueprint hook - ALWAYS sync
        self.register_hook(
            "on_blueprint_created",
            Arc::new(|item: SyncItem| {
                tracing::info!(
                    "Blueprint created, queuing for immediate sync: {}",
                    item.container_id
                );
                Ok(())
            }),
        )
        .await;

        // Container finding hook - ALWAYS sync (batched)
        self.register_hook(
            "on_container_created",
            Arc::new(|item: SyncItem| {
                tracing::debug!(
                    "Container created, queuing for batch sync: {}",
                    item.container_id
                );
                Ok(())
            }),
        )
        .await;

        // Pipeline result hook - ON-DEMAND only
        self.register_hook(
            "on_pipeline_completed",
            Arc::new(|item: SyncItem| {
                // Only sync if explicitly requested
                if item.priority != SyncPriority::Low {
                    tracing::debug!("Pipeline result shared: {}", item.container_id);
                }
                Ok(())
            }),
        )
        .await;
    }

    /// Register a hook for an event
    pub async fn register_hook(&self, event: &str, hook: HookFn) {
        let mut hooks = self.hooks.write().await;
        hooks
            .entry(event.to_string())
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
    pub async fn on_methodology_created(
        &self,
        container_id: ContainerID,
        data: Vec<u8>,
    ) -> OzoneResult<()> {
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
    pub async fn on_blueprint_created(
        &self,
        container_id: ContainerID,
        data: Vec<u8>,
    ) -> OzoneResult<()> {
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
    pub async fn on_container_created(
        &self,
        container_id: ContainerID,
        data: Vec<u8>,
    ) -> OzoneResult<()> {
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

        let peers = self.known_peers.read().await;

        for item in &items {
            for (peer_id, _peer) in peers.iter() {
                if let Err(e) = self.send_to_peer(peer_id, &item.data).await {
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

        let peers = self.known_peers.read().await;

        // Group items and send in batches
        for item in &items {
            for (peer_id, _peer) in peers.iter() {
                if let Err(e) = self.send_to_peer(peer_id, &item.data).await {
                    tracing::warn!("Failed to batch send to peer {}: {}", peer_id, e);
                }
            }
        }

        Ok(())
    }

    /// Connect to a peer using libp2p
    pub async fn connect_peer(&mut self, peer_addr: &str) -> OzoneResult<()> {
        use libp2p::Multiaddr;
        use std::str::FromStr;

        // Parse the multiaddr
        let multiaddr = Multiaddr::from_str(peer_addr)
            .map_err(|e| OzoneError::NetworkError(format!("Invalid peer address: {}", e)))?;

        // Extract peer ID from the address if present
        let peer_id = multiaddr.iter().find_map(|p| {
            if let libp2p::multiaddr::Protocol::P2p(hash) = p {
                PeerId::from_multihash(hash.into()).ok()
            } else {
                None
            }
        });

        // Get the swarm
        let mut swarm = self.swarm.lock().await;

        // Dial the peer
        match swarm.dial(multiaddr.clone()) {
            Ok(_) => {
                // Track the connection attempt
                self.connection_attempts
                    .insert(multiaddr.to_string(), std::time::Instant::now());

                // If we have peer_id, add to known peers
                if let Some(pid) = peer_id {
                    self.known_peers.write().await.insert(
                        pid,
                        PeerInfo {
                            peer_id: pid.to_string(),
                            address: multiaddr.to_string(),
                            addresses: vec![multiaddr.to_string()],
                            last_seen: now(),
                            latency_ms: 0,
                            reputation: 0.5,
                            contribution_count: 0,
                            capabilities: Vec::new(),
                        },
                    );
                }

                tracing::info!("Initiated connection to peer: {}", peer_addr);
                Ok(())
            }
            Err(e) => {
                tracing::warn!("Failed to dial peer {}: {}", peer_addr, e);
                Err(OzoneError::NetworkError(format!("Failed to connect: {}", e)))
            }
        }
    }

    /// Send data to a specific peer using libp2p gossipsub
    pub async fn send_to_peer(&self, peer_id: &PeerId, data: &[u8]) -> OzoneResult<()> {
        // Get the swarm
        let mut swarm = self.swarm.lock().await;

        // Check if peer is connected
        if !swarm.is_connected(peer_id) {
            return Err(OzoneError::NetworkError(format!(
                "Peer {} is not connected",
                peer_id
            )));
        }

        // Create a direct topic for this peer (for unicast)
        let topic_str = format!("direct/{}", peer_id);
        let topic = libp2p::gossipsub::IdentTopic::new(&topic_str);

        // Publish the message
        match swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic, data.to_vec())
        {
            Ok(message_id) => {
                tracing::debug!("Sent message {} to peer {}", message_id, peer_id);

                // Update stats
                let mut stats = self.stats.lock().await;
                stats.messages_sent += 1;
                stats.bytes_sent += data.len() as u64;

                Ok(())
            }
            Err(e) => {
                tracing::warn!("Failed to send to peer {}: {:?}", peer_id, e);
                Err(OzoneError::NetworkError(format!("Send failed: {:?}", e)))
            }
        }
    }

    /// Broadcast data to all connected peers
    pub async fn broadcast(&self, topic: &str, data: &[u8]) -> OzoneResult<usize> {
        use libp2p::gossipsub::IdentTopic;

        let topic = IdentTopic::new(topic);
        let mut swarm = self.swarm.lock().await;

        match swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic, data.to_vec())
        {
            Ok(_) => {
                let peer_count = swarm.connected_peers().count();
                let mut stats = self.stats.lock().await;
                stats.messages_sent += peer_count as u64;
                stats.bytes_sent += (data.len() * peer_count) as u64;
                Ok(peer_count)
            }
            Err(e) => Err(OzoneError::NetworkError(format!("Broadcast failed: {:?}", e))),
        }
    }

    /// Get network status
    pub async fn get_status(&self) -> NetworkStatus {
        let peers = self.known_peers.read().await;
        let high_queue = self.high_priority_queue.read().await;
        let normal_queue = self.normal_queue.read().await;

        NetworkStatus {
            enabled: self.config.enable_p2p,
            peer_id: Some(self.local_peer_id.to_string()),
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
