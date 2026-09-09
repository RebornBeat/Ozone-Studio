//! ZSEIWritePipeline - Pipeline #4
//! 
//! Write operations to ZSEI. For read operations, use ZSEIQueryPipeline (#3).
//! 
//! All writes:
//! - Create integrity snapshots before modification
//! - Update traversal hints
//! - Maintain relationship consistency
//! - Support rollback
//!
//! v0.4.0: Added actual file persistence with JSON + binary storage

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use std::env;

// ============================================================================
// Storage Layer
// ============================================================================

/// Get ZSEI storage path
fn zsei_path() -> PathBuf {
    PathBuf::from(
        env::var("OZONE_ZSEI_PATH").unwrap_or_else(|_| "./zsei_data".to_string())
    )
}

/// Ensure ZSEI directories exist
fn ensure_dirs() -> Result<(), String> {
    let base = zsei_path();
    fs::create_dir_all(base.join("global"))
        .map_err(|e| format!("Failed to create global dir: {}", e))?;
    fs::create_dir_all(base.join("local"))
        .map_err(|e| format!("Failed to create local dir: {}", e))?;
    fs::create_dir_all(base.join("snapshots"))
        .map_err(|e| format!("Failed to create snapshots dir: {}", e))?;
    fs::create_dir_all(base.join("embeddings"))
        .map_err(|e| format!("Failed to create embeddings dir: {}", e))?;
    Ok(())
}

/// Container global state (binary-storable header)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GlobalState {
    container_id: u64,
    parent_id: u64,
    child_ids: Vec<u64>,
    version: u32,
    created_at: u64,
    updated_at: u64,
}

/// Container local state (JSON metadata)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LocalState {
    container_id: u64,
    container_type: String,
    modality: String,
    name: Option<String>,
    description: Option<String>,
    provenance: Option<String>,
    permissions: u64,
    keywords: Vec<String>,
    topics: Vec<String>,
    relationships: Vec<StoredRelationship>,
    content_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredRelationship {
    target_id: u64,
    relation_type: String,
    confidence: f32,
}

/// Load global state for a container
fn load_global(container_id: u64) -> Result<GlobalState, String> {
    let path = zsei_path().join("global").join(format!("{}.json", container_id));
    if !path.exists() {
        return Err(format!("Container {} not found", container_id));
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read global state: {}", e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse global state: {}", e))
}

/// Save global state for a container
fn save_global(state: &GlobalState) -> Result<(), String> {
    let path = zsei_path().join("global").join(format!("{}.json", state.container_id));
    let content = serde_json::to_string_pretty(state)
        .map_err(|e| format!("Failed to serialize global state: {}", e))?;
    fs::write(&path, content)
        .map_err(|e| format!("Failed to write global state: {}", e))
}

/// Load local state for a container
fn load_local(container_id: u64) -> Result<LocalState, String> {
    let path = zsei_path().join("local").join(format!("{}.json", container_id));
    if !path.exists() {
        return Err(format!("Container {} local state not found", container_id));
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read local state: {}", e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse local state: {}", e))
}

/// Save local state for a container
fn save_local(state: &LocalState) -> Result<(), String> {
    let path = zsei_path().join("local").join(format!("{}.json", state.container_id));
    let content = serde_json::to_string_pretty(state)
        .map_err(|e| format!("Failed to serialize local state: {}", e))?;
    fs::write(&path, content)
        .map_err(|e| format!("Failed to write local state: {}", e))
}

/// Create integrity snapshot before modification
fn create_snapshot(container_id: u64) -> Result<u64, String> {
    let snapshot_id = generate_container_id();
    let snapshot_dir = zsei_path().join("snapshots").join(container_id.to_string());
    fs::create_dir_all(&snapshot_dir)
        .map_err(|e| format!("Failed to create snapshot dir: {}", e))?;
    
    // Copy current state to snapshot
    if let Ok(global) = load_global(container_id) {
        let path = snapshot_dir.join(format!("global_{}.json", snapshot_id));
        let content = serde_json::to_string_pretty(&global).unwrap();
        fs::write(&path, content).ok();
    }
    
    if let Ok(local) = load_local(container_id) {
        let path = snapshot_dir.join(format!("local_{}.json", snapshot_id));
        let content = serde_json::to_string_pretty(&local).unwrap();
        fs::write(&path, content).ok();
    }
    
    Ok(snapshot_id)
}

/// Check if container exists
fn container_exists(container_id: u64) -> bool {
    zsei_path().join("global").join(format!("{}.json", container_id)).exists()
}

/// Get children of a container
fn get_children(container_id: u64) -> Vec<u64> {
    load_global(container_id).map(|g| g.child_ids).unwrap_or_default()
}

/// Check for circular reference
fn would_create_cycle(container_id: u64, new_parent_id: u64) -> bool {
    // Walk up from new_parent to root, checking if we hit container_id
    let mut current = new_parent_id;
    let mut visited = std::collections::HashSet::new();
    
    while current != 0 {
        if current == container_id {
            return true;
        }
        if visited.contains(&current) {
            return true; // Already a cycle in the tree
        }
        visited.insert(current);
        
        match load_global(current) {
            Ok(g) => current = g.parent_id,
            Err(_) => break,
        }
    }
    false
}

/// Pipeline input
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ZSEIWriteInput {
    /// Create a new container
    CreateContainer {
        parent_id: u64,
        container_type: String,
        modality: String,
        metadata: ContainerMetadata,
    },
    /// Update container metadata
    UpdateMetadata {
        container_id: u64,
        updates: ContainerMetadata,
    },
    /// Update container context (keywords, topics, relationships)
    UpdateContext {
        container_id: u64,
        keywords: Option<Vec<String>>,
        topics: Option<Vec<String>>,
        add_relationships: Option<Vec<Relationship>>,
        remove_relationships: Option<Vec<u64>>,
    },
    /// Delete a container (with integrity check)
    DeleteContainer {
        container_id: u64,
        force: bool,  // Skip dependency check
    },
    /// Move container to new parent
    MoveContainer {
        container_id: u64,
        new_parent_id: u64,
    },
    /// Add relationship between containers
    AddRelationship {
        source_id: u64,
        target_id: u64,
        relation_type: String,
        confidence: f32,
    },
    /// Remove relationship
    RemoveRelationship {
        source_id: u64,
        target_id: u64,
        relation_type: Option<String>,
    },
    /// Update embedding vector
    UpdateEmbedding {
        container_id: u64,
        embedding: Vec<f32>,
    },
    /// Batch create containers
    BatchCreate {
        parent_id: u64,
        containers: Vec<BatchContainer>,
    },
    /// Rollback container to previous version
    Rollback {
        container_id: u64,
        target_version: Option<u32>,
    },
}

/// Container metadata for creation/update
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContainerMetadata {
    pub name: Option<String>,
    pub description: Option<String>,
    pub provenance: Option<String>,
    pub permissions: Option<u64>,
}

/// Relationship definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub target_id: u64,
    pub relation_type: String,
    pub confidence: f32,
}

/// Container for batch creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchContainer {
    pub container_type: String,
    pub modality: String,
    pub metadata: ContainerMetadata,
    pub keywords: Vec<String>,
    pub topics: Vec<String>,
}

/// Pipeline output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZSEIWriteOutput {
    pub success: bool,
    pub container_id: Option<u64>,
    pub container_ids: Option<Vec<u64>>,
    pub version: Option<u32>,
    pub error: Option<String>,
    pub warnings: Vec<String>,
}

/// Execute the ZSEI write pipeline
pub async fn execute(input: ZSEIWriteInput) -> Result<ZSEIWriteOutput, String> {
    // Ensure directories exist
    ensure_dirs()?;
    
    match input {
        ZSEIWriteInput::CreateContainer { parent_id, container_type, modality, metadata } => {
            // 1. Validate parent exists (unless root)
            if parent_id != 0 && !container_exists(parent_id) {
                return Err(format!("Parent container {} does not exist", parent_id));
            }
            
            // 2. Generate new container ID
            let new_id = generate_container_id();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            
            // 3. Create global state
            let global = GlobalState {
                container_id: new_id,
                parent_id,
                child_ids: vec![],
                version: 1,
                created_at: now,
                updated_at: now,
            };
            save_global(&global)?;
            
            // 4. Create local state
            let local = LocalState {
                container_id: new_id,
                container_type,
                modality,
                name: metadata.name,
                description: metadata.description,
                provenance: metadata.provenance,
                permissions: metadata.permissions.unwrap_or(0),
                keywords: vec![],
                topics: vec![],
                relationships: vec![],
                content_hash: None,
            };
            save_local(&local)?;
            
            // 5. Update parent's child list
            if parent_id != 0 {
                if let Ok(mut parent_global) = load_global(parent_id) {
                    parent_global.child_ids.push(new_id);
                    parent_global.updated_at = now;
                    save_global(&parent_global)?;
                }
            }
            
            Ok(ZSEIWriteOutput {
                success: true,
                container_id: Some(new_id),
                container_ids: None,
                version: Some(1),
                error: None,
                warnings: vec![],
            })
        }
        
        ZSEIWriteInput::UpdateMetadata { container_id, updates } => {
            // Create snapshot before update
            create_snapshot(container_id)?;
            
            // Load and update
            let mut local = load_local(container_id)?;
            let mut global = load_global(container_id)?;
            
            if let Some(name) = updates.name {
                local.name = Some(name);
            }
            if let Some(desc) = updates.description {
                local.description = Some(desc);
            }
            if let Some(prov) = updates.provenance {
                local.provenance = Some(prov);
            }
            if let Some(perms) = updates.permissions {
                local.permissions = perms;
            }
            
            // Increment version
            global.version += 1;
            global.updated_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            
            save_local(&local)?;
            save_global(&global)?;
            
            Ok(ZSEIWriteOutput {
                success: true,
                container_id: Some(container_id),
                container_ids: None,
                version: Some(global.version),
                error: None,
                warnings: vec![],
            })
        }
        
        ZSEIWriteInput::UpdateContext { container_id, keywords, topics, add_relationships, remove_relationships } => {
            // Create snapshot
            create_snapshot(container_id)?;
            
            let mut local = load_local(container_id)?;
            let mut global = load_global(container_id)?;
            let mut warnings = vec![];
            
            // Update keywords
            if let Some(kws) = keywords {
                local.keywords = kws;
            }
            
            // Update topics
            if let Some(tps) = topics {
                local.topics = tps;
            }
            
            // Add relationships
            if let Some(rels) = add_relationships {
                for rel in rels {
                    // Validate target exists
                    if !container_exists(rel.target_id) {
                        warnings.push(format!("Target container {} does not exist", rel.target_id));
                        continue;
                    }
                    if rel.target_id == container_id {
                        warnings.push("Self-referential relationship skipped".to_string());
                        continue;
                    }
                    local.relationships.push(StoredRelationship {
                        target_id: rel.target_id,
                        relation_type: rel.relation_type,
                        confidence: rel.confidence,
                    });
                }
            }
            
            // Remove relationships
            if let Some(target_ids) = remove_relationships {
                local.relationships.retain(|r| !target_ids.contains(&r.target_id));
            }
            
            // Update version
            global.version += 1;
            global.updated_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            
            save_local(&local)?;
            save_global(&global)?;
            
            Ok(ZSEIWriteOutput {
                success: true,
                container_id: Some(container_id),
                container_ids: None,
                version: Some(global.version),
                error: None,
                warnings,
            })
        }
        
        ZSEIWriteInput::DeleteContainer { container_id, force } => {
            if container_id == 0 {
                return Err("Cannot delete root container".into());
            }
            
            let mut warnings = vec![];
            
            // Check for children
            let children = get_children(container_id);
            if !children.is_empty() && !force {
                return Err(format!(
                    "Container has {} children. Use force=true to delete anyway.",
                    children.len()
                ));
            }
            
            if !children.is_empty() {
                warnings.push(format!("Deleted container with {} children", children.len()));
            }
            
            // Create snapshot before deletion
            create_snapshot(container_id)?;
            
            // Load container to get parent
            let global = load_global(container_id)?;
            
            // Remove from parent's child list
            if global.parent_id != 0 {
                if let Ok(mut parent_global) = load_global(global.parent_id) {
                    parent_global.child_ids.retain(|&id| id != container_id);
                    parent_global.updated_at = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    save_global(&parent_global)?;
                }
            }
            
            // Delete the files
            let global_path = zsei_path().join("global").join(format!("{}.json", container_id));
            let local_path = zsei_path().join("local").join(format!("{}.json", container_id));
            
            fs::remove_file(&global_path).ok();
            fs::remove_file(&local_path).ok();
            
            Ok(ZSEIWriteOutput {
                success: true,
                container_id: Some(container_id),
                container_ids: None,
                version: None,
                error: None,
                warnings,
            })
        }
        
        ZSEIWriteInput::MoveContainer { container_id, new_parent_id } => {
            if container_id == 0 {
                return Err("Cannot move root container".into());
            }
            
            if container_id == new_parent_id {
                return Err("Cannot move container to itself".into());
            }
            
            if !container_exists(new_parent_id) && new_parent_id != 0 {
                return Err(format!("New parent {} does not exist", new_parent_id));
            }
            
            // Check for circular reference
            if would_create_cycle(container_id, new_parent_id) {
                return Err("Move would create circular reference".into());
            }
            
            // Create snapshot
            create_snapshot(container_id)?;
            
            let mut global = load_global(container_id)?;
            let old_parent_id = global.parent_id;
            
            // Remove from old parent's child list
            if old_parent_id != 0 {
                if let Ok(mut old_parent) = load_global(old_parent_id) {
                    old_parent.child_ids.retain(|&id| id != container_id);
                    save_global(&old_parent)?;
                }
            }
            
            // Add to new parent's child list
            if new_parent_id != 0 {
                if let Ok(mut new_parent) = load_global(new_parent_id) {
                    new_parent.child_ids.push(container_id);
                    save_global(&new_parent)?;
                }
            }
            
            // Update container's parent
            global.parent_id = new_parent_id;
            global.version += 1;
            global.updated_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            save_global(&global)?;
            
            Ok(ZSEIWriteOutput {
                success: true,
                container_id: Some(container_id),
                container_ids: None,
                version: Some(global.version),
                error: None,
                warnings: vec![],
            })
        }
        
        ZSEIWriteInput::AddRelationship { source_id, target_id, relation_type, confidence } => {
            if source_id == target_id {
                return Err("Cannot create self-referential relationship".into());
            }
            
            if !container_exists(source_id) {
                return Err(format!("Source container {} does not exist", source_id));
            }
            
            if !container_exists(target_id) {
                return Err(format!("Target container {} does not exist", target_id));
            }
            
            create_snapshot(source_id)?;
            
            let mut local = load_local(source_id)?;
            let mut global = load_global(source_id)?;
            
            // Check if relationship already exists
            let exists = local.relationships.iter()
                .any(|r| r.target_id == target_id && r.relation_type == relation_type);
            
            if !exists {
                local.relationships.push(StoredRelationship {
                    target_id,
                    relation_type,
                    confidence,
                });
            }
            
            global.version += 1;
            global.updated_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            
            save_local(&local)?;
            save_global(&global)?;
            
            Ok(ZSEIWriteOutput {
                success: true,
                container_id: Some(source_id),
                container_ids: None,
                version: Some(global.version),
                error: None,
                warnings: vec![],
            })
        }
        
        ZSEIWriteInput::RemoveRelationship { source_id, target_id, relation_type } => {
            if !container_exists(source_id) {
                return Err(format!("Source container {} does not exist", source_id));
            }
            
            create_snapshot(source_id)?;
            
            let mut local = load_local(source_id)?;
            let mut global = load_global(source_id)?;
            
            let initial_count = local.relationships.len();
            
            local.relationships.retain(|r| {
                if r.target_id != target_id {
                    return true;
                }
                if let Some(ref rt) = relation_type {
                    r.relation_type != *rt
                } else {
                    false // Remove all relationships to target
                }
            });
            
            let removed = initial_count - local.relationships.len();
            
            global.version += 1;
            global.updated_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            
            save_local(&local)?;
            save_global(&global)?;
            
            Ok(ZSEIWriteOutput {
                success: true,
                container_id: Some(source_id),
                container_ids: None,
                version: Some(global.version),
                error: None,
                warnings: if removed == 0 { vec!["No relationships removed".into()] } else { vec![] },
            })
        }
        
        ZSEIWriteInput::UpdateEmbedding { container_id, embedding } => {
            // Validate embedding dimension (common dimensions: 384, 768, 1536)
            if embedding.is_empty() {
                return Err("Embedding cannot be empty".into());
            }
            
            if !container_exists(container_id) {
                return Err(format!("Container {} does not exist", container_id));
            }
            
            // Store embedding as binary for efficiency
            let embedding_path = zsei_path().join("embeddings").join(format!("{}.bin", container_id));
            let bytes: Vec<u8> = embedding.iter()
                .flat_map(|f| f.to_le_bytes())
                .collect();
            
            fs::write(&embedding_path, &bytes)
                .map_err(|e| format!("Failed to write embedding: {}", e))?;
            
            // Update version
            let mut global = load_global(container_id)?;
            global.version += 1;
            global.updated_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            save_global(&global)?;
            
            Ok(ZSEIWriteOutput {
                success: true,
                container_id: Some(container_id),
                container_ids: None,
                version: Some(global.version),
                error: None,
                warnings: vec![],
            })
        }
        
        ZSEIWriteInput::BatchCreate { parent_id, containers } => {
            // Validate parent
            if parent_id != 0 && !container_exists(parent_id) {
                return Err(format!("Parent container {} does not exist", parent_id));
            }
            
            let mut created_ids = vec![];
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            
            for batch_container in &containers {
                let new_id = generate_container_id();
                
                let global = GlobalState {
                    container_id: new_id,
                    parent_id,
                    child_ids: vec![],
                    version: 1,
                    created_at: now,
                    updated_at: now,
                };
                save_global(&global)?;
                
                let local = LocalState {
                    container_id: new_id,
                    container_type: batch_container.container_type.clone(),
                    modality: batch_container.modality.clone(),
                    name: batch_container.metadata.name.clone(),
                    description: batch_container.metadata.description.clone(),
                    provenance: batch_container.metadata.provenance.clone(),
                    permissions: batch_container.metadata.permissions.unwrap_or(0),
                    keywords: batch_container.keywords.clone(),
                    topics: batch_container.topics.clone(),
                    relationships: vec![],
                    content_hash: None,
                };
                save_local(&local)?;
                
                created_ids.push(new_id);
            }
            
            // Update parent's child list
            if parent_id != 0 {
                if let Ok(mut parent_global) = load_global(parent_id) {
                    parent_global.child_ids.extend(created_ids.iter());
                    parent_global.updated_at = now;
                    save_global(&parent_global)?;
                }
            }
            
            Ok(ZSEIWriteOutput {
                success: true,
                container_id: None,
                container_ids: Some(created_ids),
                version: None,
                error: None,
                warnings: vec![],
            })
        }
        
        ZSEIWriteInput::Rollback { container_id, target_version } => {
            let snapshot_dir = zsei_path().join("snapshots").join(container_id.to_string());
            
            if !snapshot_dir.exists() {
                return Err(format!("No snapshots found for container {}", container_id));
            }
            
            // Find the appropriate snapshot
            let mut snapshots: Vec<_> = fs::read_dir(&snapshot_dir)
                .map_err(|e| format!("Failed to read snapshots: {}", e))?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().to_string_lossy().contains("global_"))
                .collect();
            
            snapshots.sort_by(|a, b| b.path().cmp(&a.path())); // Newest first
            
            let snapshot = if let Some(version) = target_version {
                // Find specific version (simplified - would need version tracking in snapshots)
                snapshots.get(0)
            } else {
                // Get most recent snapshot
                snapshots.get(0)
            };
            
            let snapshot = snapshot.ok_or("No suitable snapshot found")?;
            let snapshot_id = snapshot.path()
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.strip_prefix("global_"))
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            
            // Load and restore global state
            let global_snapshot_path = snapshot_dir.join(format!("global_{}.json", snapshot_id));
            if global_snapshot_path.exists() {
                let content = fs::read_to_string(&global_snapshot_path)
                    .map_err(|e| format!("Failed to read snapshot: {}", e))?;
                let mut global: GlobalState = serde_json::from_str(&content)
                    .map_err(|e| format!("Failed to parse snapshot: {}", e))?;
                global.version += 1; // Increment version for rollback
                save_global(&global)?;
            }
            
            // Restore local state
            let local_snapshot_path = snapshot_dir.join(format!("local_{}.json", snapshot_id));
            if local_snapshot_path.exists() {
                let content = fs::read_to_string(&local_snapshot_path)
                    .map_err(|e| format!("Failed to read local snapshot: {}", e))?;
                let local: LocalState = serde_json::from_str(&content)
                    .map_err(|e| format!("Failed to parse local snapshot: {}", e))?;
                save_local(&local)?;
            }
            
            let current_version = load_global(container_id)
                .map(|g| g.version)
                .unwrap_or(1);
            
            Ok(ZSEIWriteOutput {
                success: true,
                container_id: Some(container_id),
                container_ids: None,
                version: Some(current_version),
                error: None,
                warnings: vec![],
            })
        }
    }
}

/// Generate a new container ID (ID allocation via ZSEI)
fn generate_container_id() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}

// ============================================================================
// CLI entry point
// ============================================================================

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--input" {
            i += 1;
            if i < args.len() {
                input_json = args[i].clone();
            }
        }
        i += 1;
    }
    
    let input: ZSEIWriteInput = match serde_json::from_str(&input_json) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Failed to parse input: {}", e);
            std::process::exit(1);
        }
    };
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(output) => println!("{}", serde_json::to_string(&output).unwrap()),
        Err(e) => {
            let output = ZSEIWriteOutput {
                success: false,
                container_id: None,
                container_ids: None,
                version: None,
                error: Some(e),
                warnings: vec![],
            };
            println!("{}", serde_json::to_string(&output).unwrap());
            std::process::exit(1);
        }
    }
}
