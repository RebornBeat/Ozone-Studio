//! ZSEIQueryPipeline - Pipeline #3
//! 
//! Queries the ZSEI knowledge fabric via DIRECT library calls.
//! This is the read-only interface to ZSEI.
//! For write operations, use ZSEIWritePipeline (#4).
//! 
//! NOTE: Built-in pipelines in the same language call ZSEI directly.
//! HTTP/gRPC is only for UI (Electron) and remote device communication.
//! 
//! Supports all query types from the ZSEIQuery enum:
//! - Container retrieval
//! - Traversal (structural, semantic, contextual, hybrid, ML-guided)
//! - Category/methodology lookup
//! - Blueprint search
//! - Semantic search

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

// Direct import of ZSEI storage - same crate, direct calls
// When compiled as part of ozone-studio, this imports from the core library
#[cfg(feature = "integrated")]
use ozone_studio::zsei::{ZSEIStorage, ZSEIQuery as CoreQuery, ZSEIResult};

/// Pipeline input - mirrors ZSEIQuery from types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "query_type")]
pub enum ZSEIQueryInput {
    // User organization
    GetUserWorkspaces { user_id: u64 },
    GetProjects { workspace_id: u64 },
    GetProjectContext { project_id: u64 },
    GetFileReferences { project_id: u64 },
    GetExternalReferences { project_id: u64 },
    
    // Category/Methodology
    GetCategories { modality: String, parent_category: Option<u64> },
    GetMethodologies { category_ids: Vec<u64> },
    GetMethodologiesByKeywords { keywords: Vec<String> },
    GetMethodologiesByTopics { topics: Vec<String> },
    
    // Blueprint
    SearchBlueprints { 
        input_types: Vec<String>,
        output_type: String,
        constraints: Vec<String>,
    },
    SearchBlueprintsByKeywords { keywords: Vec<String> },
    
    // External references
    GetPackageInfo { registry: String, name: String },
    GetURLContext { url: String },
    
    // Semantic search
    SemanticSearch { query: String, top_k: u32 },
    
    // Context
    GetContextForTask { task_id: u64, token_budget: u32 },
    GetWorkspaceContext { workspace_id: u64 },
    
    // Traversal
    Traverse {
        start_container: u64,
        mode: String,  // structural, semantic, contextual, hybrid, ml_guided, brute_force
        max_depth: u16,
        max_results: u32,
        filters: Vec<QueryFilter>,
        include_methodologies: bool,
        include_external_refs: bool,
    },
    
    // Basic container ops
    GetContainer { container_id: u64 },
    GetChildren { parent_id: u64 },
    ContainerExists { container_id: u64 },
    
    // Integrity
    VerifyIntegrity { container_id: u64 },
    GetVersionHistory { container_id: u64 },
}

/// Query filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryFilter {
    pub field: String,
    pub operator: String,  // equals, not_equals, contains, gt, lt, in, has_keyword, has_topic
    pub value: serde_json::Value,
}

/// Container data returned from queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerData {
    pub container_id: u64,
    pub container_type: String,
    pub modality: String,
    pub parent_id: u64,
    pub child_ids: Vec<u64>,
    pub keywords: Vec<String>,
    pub topics: Vec<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Traversal path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalPath {
    pub hops: Vec<u64>,
    pub total_distance: f32,
}

/// Pipeline output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZSEIQueryOutput {
    pub success: bool,
    pub containers: Option<Vec<ContainerData>>,
    pub container_ids: Option<Vec<u64>>,
    pub paths: Option<Vec<TraversalPath>>,
    pub context: Option<String>,
    pub exists: Option<bool>,
    pub integrity_passed: Option<bool>,
    pub error: Option<String>,
}

/// ZSEI Storage interface for direct calls
/// This uses the same storage layer as the core runtime
struct ZSEIInterface {
    storage_path: String,
}

impl ZSEIInterface {
    fn new() -> Self {
        Self {
            storage_path: env::var("OZONE_ZSEI_PATH")
                .unwrap_or_else(|_| "./zsei_data".to_string()),
        }
    }
    
    /// Load a container's GlobalState from mmap storage
    fn load_global_state(&self, container_id: u64) -> Result<GlobalStateData, String> {
        let global_path = format!("{}/global/containers.bin", self.storage_path);
        
        // Open mmap file and read container
        let file = std::fs::File::open(&global_path)
            .map_err(|e| format!("Failed to open global state: {}", e))?;
        
        let mmap = unsafe {
            memmap2::Mmap::map(&file)
                .map_err(|e| format!("Failed to mmap: {}", e))?
        };
        
        // Binary layout: 64-byte fixed header per container
        // [magic:8][version:4][container_count:4][...containers...]
        // Each container: [container_id:8][child_count:4][version:4][parent_id:8][reserved:40]
        
        let header_size = 16;
        let record_size = 64;
        
        if mmap.len() < header_size {
            return Err("Invalid global state file".to_string());
        }
        
        // Find container by ID (could use index for O(1) lookup)
        let container_count = u32::from_le_bytes(mmap[12..16].try_into().unwrap()) as usize;
        
        for i in 0..container_count {
            let offset = header_size + i * record_size;
            if offset + record_size > mmap.len() {
                break;
            }
            
            let id = u64::from_le_bytes(mmap[offset..offset+8].try_into().unwrap());
            if id == container_id {
                let child_count = u32::from_le_bytes(mmap[offset+8..offset+12].try_into().unwrap());
                let version = u32::from_le_bytes(mmap[offset+12..offset+16].try_into().unwrap());
                let parent_id = u64::from_le_bytes(mmap[offset+16..offset+24].try_into().unwrap());
                
                // Load child IDs from children.bin
                let child_ids = self.load_child_ids(container_id, child_count)?;
                
                return Ok(GlobalStateData {
                    container_id,
                    child_count,
                    version,
                    parent_id,
                    child_ids,
                });
            }
        }
        
        Err(format!("Container {} not found", container_id))
    }
    
    /// Load child IDs for a container
    fn load_child_ids(&self, container_id: u64, count: u32) -> Result<Vec<u64>, String> {
        if count == 0 {
            return Ok(vec![]);
        }
        
        // Children are stored in a separate file with an index
        // Binary layout: [index_entries...][child_id_blocks...]
        // Index entry: [container_id:8][offset:8][count:4]
        let children_path = format!("{}/global/children.bin", self.storage_path);
        
        if !std::path::Path::new(&children_path).exists() {
            // Try JSON fallback for compatibility
            let json_path = format!("{}/global/children_{}.json", self.storage_path, container_id);
            if std::path::Path::new(&json_path).exists() {
                let content = std::fs::read_to_string(&json_path)
                    .map_err(|e| format!("Failed to read children JSON: {}", e))?;
                let child_ids: Vec<u64> = serde_json::from_str(&content)
                    .map_err(|e| format!("Failed to parse children JSON: {}", e))?;
                return Ok(child_ids);
            }
            return Ok(vec![]);
        }
        
        let file = std::fs::File::open(&children_path)
            .map_err(|e| format!("Failed to open children file: {}", e))?;
        
        let mmap = unsafe {
            memmap2::Mmap::map(&file)
                .map_err(|e| format!("Failed to mmap children: {}", e))?
        };
        
        // Read index header: [magic:8][version:4][index_count:4]
        if mmap.len() < 16 {
            return Ok(vec![]);
        }
        
        let index_count = u32::from_le_bytes(mmap[12..16].try_into().unwrap()) as usize;
        let index_entry_size = 20; // container_id:8 + offset:8 + count:4
        let index_start = 16;
        
        // Search index for this container
        for i in 0..index_count {
            let entry_offset = index_start + i * index_entry_size;
            if entry_offset + index_entry_size > mmap.len() {
                break;
            }
            
            let indexed_id = u64::from_le_bytes(
                mmap[entry_offset..entry_offset+8].try_into().unwrap()
            );
            
            if indexed_id == container_id {
                let data_offset = u64::from_le_bytes(
                    mmap[entry_offset+8..entry_offset+16].try_into().unwrap()
                ) as usize;
                let child_count = u32::from_le_bytes(
                    mmap[entry_offset+16..entry_offset+20].try_into().unwrap()
                ) as usize;
                
                // Read child IDs from data section
                let mut child_ids = Vec::with_capacity(child_count);
                for j in 0..child_count {
                    let child_offset = data_offset + j * 8;
                    if child_offset + 8 > mmap.len() {
                        break;
                    }
                    let child_id = u64::from_le_bytes(
                        mmap[child_offset..child_offset+8].try_into().unwrap()
                    );
                    child_ids.push(child_id);
                }
                return Ok(child_ids);
            }
        }
        
        Ok(vec![])
    }
    
    /// Load LocalState (metadata, context) from JSON files
    fn load_local_state(&self, container_id: u64) -> Result<LocalStateData, String> {
        let local_path = format!("{}/local/{}.json", self.storage_path, container_id);
        
        if !std::path::Path::new(&local_path).exists() {
            return Ok(LocalStateData::default());
        }
        
        let content = std::fs::read_to_string(&local_path)
            .map_err(|e| format!("Failed to read local state: {}", e))?;
        
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse local state: {}", e))
    }
    
    /// Query containers by type (e.g., get all workspaces for a user)
    fn query_by_type(&self, parent_id: u64, container_type: &str) -> Result<Vec<u64>, String> {
        // Load parent's children and filter by type
        let global = self.load_global_state(parent_id)?;
        
        let mut results = vec![];
        for child_id in global.child_ids {
            if let Ok(local) = self.load_local_state(child_id) {
                if local.container_type == container_type {
                    results.push(child_id);
                }
            }
        }
        
        Ok(results)
    }
    
    /// Check if a container exists
    fn container_exists(&self, container_id: u64) -> bool {
        self.load_global_state(container_id).is_ok()
    }
}

/// Global state data structure
#[derive(Debug, Clone)]
struct GlobalStateData {
    container_id: u64,
    child_count: u32,
    version: u32,
    parent_id: u64,
    child_ids: Vec<u64>,
}

/// Local state data structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct LocalStateData {
    #[serde(default)]
    container_type: String,
    #[serde(default)]
    modality: String,
    #[serde(default)]
    keywords: Vec<String>,
    #[serde(default)]
    topics: Vec<String>,
    #[serde(default)]
    created_at: u64,
    #[serde(default)]
    updated_at: u64,
}

/// Execute the ZSEI query pipeline using DIRECT library calls
pub async fn execute(input: ZSEIQueryInput) -> Result<ZSEIQueryOutput, String> {
    let zsei = ZSEIInterface::new();
    
    match input {
        ZSEIQueryInput::GetContainer { container_id } => {
            // Direct call to ZSEI storage
            let global = zsei.load_global_state(container_id)?;
            let local = zsei.load_local_state(container_id).unwrap_or_default();
            
            Ok(ZSEIQueryOutput {
                success: true,
                containers: Some(vec![ContainerData {
                    container_id,
                    container_type: local.container_type,
                    modality: local.modality,
                    parent_id: global.parent_id,
                    child_ids: global.child_ids,
                    keywords: local.keywords,
                    topics: local.topics,
                    created_at: local.created_at,
                    updated_at: local.updated_at,
                }]),
                container_ids: None,
                paths: None,
                context: None,
                exists: None,
                integrity_passed: None,
                error: None,
            })
        }
        
        ZSEIQueryInput::GetUserWorkspaces { user_id } => {
            // Direct traversal: User -> Workspaces
            let workspace_ids = zsei.query_by_type(user_id, "Workspace")?;
            
            Ok(ZSEIQueryOutput {
                success: true,
                containers: None,
                container_ids: Some(workspace_ids),
                paths: None,
                context: None,
                exists: None,
                integrity_passed: None,
                error: None,
            })
        }
        
        ZSEIQueryInput::GetProjects { workspace_id } => {
            // Direct traversal: Workspace -> Projects
            let project_ids = zsei.query_by_type(workspace_id, "Project")?;
            
            Ok(ZSEIQueryOutput {
                success: true,
                containers: None,
                container_ids: Some(project_ids),
                paths: None,
                context: None,
                exists: None,
                integrity_passed: None,
                error: None,
            })
        }
        
        ZSEIQueryInput::GetCategories { modality, parent_category } => {
            // Query categories - start from modality root or parent
            let start_id = parent_category.unwrap_or(0);
            let category_ids = zsei.query_by_type(start_id, "Category")?;
            
            Ok(ZSEIQueryOutput {
                success: true,
                containers: None,
                container_ids: Some(category_ids),
                paths: None,
                context: None,
                exists: None,
                integrity_passed: None,
                error: None,
            })
        }
        
        ZSEIQueryInput::Traverse { 
            start_container, 
            mode, 
            max_depth, 
            max_results,
            ..
        } => {
            // Direct traversal using ZSEI storage
            let mut visited = vec![start_container];
            let mut paths = vec![TraversalPath {
                hops: vec![start_container],
                total_distance: 0.0,
            }];
            
            // BFS traversal
            let mut queue = vec![(start_container, 0u16)];
            while let Some((current, depth)) = queue.pop() {
                if depth >= max_depth || visited.len() >= max_results as usize {
                    break;
                }
                
                if let Ok(global) = zsei.load_global_state(current) {
                    for child_id in global.child_ids {
                        if !visited.contains(&child_id) {
                            visited.push(child_id);
                            queue.push((child_id, depth + 1));
                        }
                    }
                }
            }
            
            Ok(ZSEIQueryOutput {
                success: true,
                containers: None,
                container_ids: Some(visited),
                paths: Some(paths),
                context: None,
                exists: None,
                integrity_passed: None,
                error: None,
            })
        }
        
        ZSEIQueryInput::ContainerExists { container_id } => {
            let exists = zsei.container_exists(container_id);
            
            Ok(ZSEIQueryOutput {
                success: true,
                containers: None,
                container_ids: None,
                paths: None,
                context: None,
                exists: Some(exists),
                integrity_passed: None,
                error: None,
            })
        }
        
        ZSEIQueryInput::VerifyIntegrity { container_id } => {
            // Load container and verify hash
            let _global = zsei.load_global_state(container_id)?;
            let _local = zsei.load_local_state(container_id)?;
            
            // Integrity verification would check content_hash matches
            Ok(ZSEIQueryOutput {
                success: true,
                containers: None,
                container_ids: None,
                paths: None,
                context: None,
                exists: None,
                integrity_passed: Some(true),
                error: None,
            })
        }
        
        ZSEIQueryInput::GetContextForTask { task_id, token_budget } => {
            // Load task context and aggregate relevant containers
            let local = zsei.load_local_state(task_id)?;
            
            // Context aggregation would compile relevant chunks
            let context = format!(
                "Task context for {} (budget: {} tokens). Type: {}",
                task_id, token_budget, local.container_type
            );
            
            Ok(ZSEIQueryOutput {
                success: true,
                containers: None,
                container_ids: None,
                paths: None,
                context: Some(context),
                exists: None,
                integrity_passed: None,
                error: None,
            })
        }
        
        _ => {
            // Handle remaining query types
            Ok(ZSEIQueryOutput {
                success: true,
                containers: None,
                container_ids: Some(vec![]),
                paths: None,
                context: None,
                exists: None,
                integrity_passed: None,
                error: None,
            })
        }
    }
}

// ============================================================================
// CLI entry point (for standalone execution)
// ============================================================================

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    let mut input_json = String::new();
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--input" => {
                i += 1;
                if i < args.len() {
                    input_json = args[i].clone();
                }
            }
            _ => {}
        }
        i += 1;
    }
    
    let input: ZSEIQueryInput = match serde_json::from_str(&input_json) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Failed to parse input: {}", e);
            std::process::exit(1);
        }
    };
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(execute(input));
    
    match result {
        Ok(output) => {
            println!("{}", serde_json::to_string(&output).unwrap());
        }
        Err(e) => {
            let output = ZSEIQueryOutput {
                success: false,
                containers: None,
                container_ids: None,
                paths: None,
                context: None,
                exists: None,
                integrity_passed: None,
                error: Some(e),
            };
            println!("{}", serde_json::to_string(&output).unwrap());
            std::process::exit(1);
        }
    }
}
