//! Container storage - mmap-backed persistent storage for ZSEI containers
//!
//! GlobalState uses mmap for efficient random access via binary protocol.
//! LocalState stored as JSON files for flexibility.
//!
//! Binary format for GlobalState (fixed 64-byte header per container):
//! - Bytes 0-7:   container_id (u64 LE)
//! - Bytes 8-11:  child_count (u32 LE)
//! - Bytes 12-15: version (u32 LE)
//! - Bytes 16-23: parent_id (u64 LE)
//! - Bytes 24-31: child_list_offset (u64 LE) - offset to variable-length child list
//! - Bytes 32-63: reserved for future use

use crate::config::ZSEIConfig;
use crate::types::{ContainerID, OzoneError, OzoneResult};
use crate::types::container::{Container, GlobalState, LocalState};
use memmap2::{MmapMut, MmapOptions};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::path::PathBuf;

const HEADER_SIZE: usize = 64;
const INITIAL_FILE_SIZE: u64 = 16 * 1024 * 1024; // 16MB initial
const MAGIC_BYTES: &[u8; 8] = b"OZONEZSE";
const FILE_VERSION: u32 = 1;

/// Container storage with mmap for global state
pub struct ContainerStorage {
    /// Path to global state file (mmap)
    global_path: PathBuf,
    
    /// Path to local state directory (JSON files)
    local_path: PathBuf,
    
    /// Memory-mapped global state file
    global_mmap: Option<MmapMut>,
    
    /// Global state file handle
    global_file: Option<File>,
    
    /// Whether mmap is enabled
    mmap_enabled: bool,
    
    /// Index: container_id -> offset in mmap
    index: HashMap<ContainerID, u64>,
    
    /// In-memory cache for local state (backed by JSON files)
    local_cache: HashMap<ContainerID, LocalState>,
    
    /// Cache for child_ids (stored separately from mmap)
    child_ids_cache: HashMap<ContainerID, Vec<ContainerID>>,
    
    /// Next available container ID
    next_id: ContainerID,
    
    /// Current write offset in mmap
    write_offset: u64,
}

impl ContainerStorage {
    /// Create new container storage
    pub fn new(config: &ZSEIConfig) -> OzoneResult<Self> {
        let global_path = PathBuf::from(&config.global_path);
        let local_path = PathBuf::from(&config.local_path);
        
        // Ensure directories exist
        if let Some(parent) = global_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| OzoneError::StorageError(format!("Failed to create global dir: {}", e)))?;
        }
        fs::create_dir_all(&local_path)
            .map_err(|e| OzoneError::StorageError(format!("Failed to create local dir: {}", e)))?;
        
        let mut storage = Self {
            global_path,
            local_path,
            global_mmap: None,
            global_file: None,
            mmap_enabled: config.mmap_enabled,
            index: HashMap::new(),
            local_cache: HashMap::new(),
            child_ids_cache: HashMap::new(),
            next_id: 1, // 0 is reserved for root
            write_offset: 64, // After file header
        };
        
        // Initialize storage
        if config.mmap_enabled {
            storage.init_mmap()?;
        }
        
        // Load existing data
        storage.load_index()?;
        storage.load_local_cache()?;
        
        // Create root container if not exists
        storage.ensure_root()?;
        
        Ok(storage)
    }
    
    /// Initialize memory-mapped file with proper header
    fn init_mmap(&mut self) -> OzoneResult<()> {
        let is_new = !self.global_path.exists();
        
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&self.global_path)
            .map_err(|e| OzoneError::StorageError(format!("Failed to open global file: {}", e)))?;
        
        // Ensure minimum size
        let metadata = file.metadata()
            .map_err(|e| OzoneError::StorageError(format!("Failed to get file metadata: {}", e)))?;
        
        if metadata.len() < INITIAL_FILE_SIZE {
            file.set_len(INITIAL_FILE_SIZE)
                .map_err(|e| OzoneError::StorageError(format!("Failed to resize file: {}", e)))?;
        }
        
        let mut mmap = unsafe {
            MmapOptions::new()
                .map_mut(&file)
                .map_err(|e| OzoneError::StorageError(format!("Failed to mmap: {}", e)))?
        };
        
        // Write file header if new
        if is_new {
            mmap[0..8].copy_from_slice(MAGIC_BYTES);
            mmap[8..12].copy_from_slice(&FILE_VERSION.to_le_bytes());
            mmap[12..20].copy_from_slice(&1u64.to_le_bytes()); // next_id
            mmap[20..28].copy_from_slice(&64u64.to_le_bytes()); // write_offset
            mmap.flush().map_err(|e| OzoneError::StorageError(format!("Failed to flush: {}", e)))?;
        } else {
            // Verify magic bytes
            if &mmap[0..8] != MAGIC_BYTES {
                return Err(OzoneError::StorageError("Invalid storage file format".into()));
            }
            // Read next_id and write_offset
            self.next_id = u64::from_le_bytes(mmap[12..20].try_into().unwrap());
            self.write_offset = u64::from_le_bytes(mmap[20..28].try_into().unwrap());
        }
        
        self.global_file = Some(file);
        self.global_mmap = Some(mmap);
        
        Ok(())
    }
    
    /// Load index from mmap file
    fn load_index(&mut self) -> OzoneResult<()> {
        if let Some(ref mmap) = self.global_mmap {
            let mut offset = 64u64; // Skip file header
            
            while offset + HEADER_SIZE as u64 <= self.write_offset {
                let start = offset as usize;
                let container_id = u64::from_le_bytes(mmap[start..start+8].try_into().unwrap());
                
                if container_id != 0 || offset == 64 { // Valid container or root at offset 64
                    self.index.insert(container_id, offset);
                }
                
                offset += HEADER_SIZE as u64;
            }
        }
        Ok(())
    }
    
    /// Load local state cache from JSON files
    fn load_local_cache(&mut self) -> OzoneResult<()> {
        if let Ok(entries) = fs::read_dir(&self.local_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    if let Some(stem) = path.file_stem() {
                        if let Ok(id) = stem.to_string_lossy().parse::<u64>() {
                            if let Ok(contents) = fs::read_to_string(&path) {
                                if let Ok(state) = serde_json::from_str::<LocalState>(&contents) {
                                    self.local_cache.insert(id, state);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
    
    /// Ensure root container exists
    fn ensure_root(&mut self) -> OzoneResult<()> {
        if !self.index.contains_key(&0) {
            let root = Container {
                global_state: GlobalState {
                    container_id: 0,
                    child_count: 0,
                    version: 1,
                    parent_id: 0,
                    child_ids: Vec::new(),
                },
                local_state: LocalState::default(),
            };
            self.store(&root)?;
        }
        Ok(())
    }
    
    /// Load a container by ID
    pub fn load(&self, id: ContainerID) -> OzoneResult<Option<Container>> {
        let global_state = self.load_global(id)?;
        
        if global_state.is_none() {
            return Ok(None);
        }
        
        let local_state = self.load_local(id)?.unwrap_or_default();
        
        Ok(Some(Container {
            global_state: global_state.unwrap(),
            local_state,
        }))
    }
    
    /// Store a container
    pub fn store(&mut self, container: &Container) -> OzoneResult<()> {
        let id = container.global_state.container_id;
        
        if id >= self.next_id {
            self.next_id = id + 1;
        }
        
        // Store child_ids in the cache
        self.child_ids_cache.insert(id, container.global_state.child_ids.clone());
        
        self.store_global(&container.global_state)?;
        self.store_local(id, &container.local_state)?;
        
        Ok(())
    }
    
    /// Allocate new container ID
    pub fn allocate_id(&mut self) -> ContainerID {
        let id = self.next_id;
        self.next_id += 1;
        
        // Update header
        if let Some(ref mut mmap) = self.global_mmap {
            mmap[12..20].copy_from_slice(&self.next_id.to_le_bytes());
            let _ = mmap.flush();
        }
        
        id
    }
    
    /// Load global state from mmap
    fn load_global(&self, id: ContainerID) -> OzoneResult<Option<GlobalState>> {
        let offset = match self.index.get(&id) {
            Some(o) => *o as usize,
            None => return Ok(None),
        };
        
        if let Some(ref mmap) = self.global_mmap {
            if offset + HEADER_SIZE > mmap.len() {
                return Ok(None);
            }
            
            let container_id = u64::from_le_bytes(mmap[offset..offset+8].try_into().unwrap());
            let child_count = u32::from_le_bytes(mmap[offset+8..offset+12].try_into().unwrap());
            let version = u32::from_le_bytes(mmap[offset+12..offset+16].try_into().unwrap());
            let parent_id = u64::from_le_bytes(mmap[offset+16..offset+24].try_into().unwrap());
            
            // Child IDs are stored in the child_ids_cache
            let child_ids = self.child_ids_cache.get(&id)
                .cloned()
                .unwrap_or_default();
            
            Ok(Some(GlobalState {
                container_id,
                child_count,
                version,
                parent_id,
                child_ids,
            }))
        } else {
            Ok(None)
        }
    }
    
    /// Store global state to mmap
    fn store_global(&mut self, state: &GlobalState) -> OzoneResult<()> {
        let offset = if let Some(existing) = self.index.get(&state.container_id) {
            *existing as usize
        } else {
            let new_offset = self.write_offset as usize;
            self.write_offset += HEADER_SIZE as u64;
            self.index.insert(state.container_id, new_offset as u64);
            new_offset
        };
        
        if let Some(ref mut mmap) = self.global_mmap {
            // Ensure file is large enough
            if offset + HEADER_SIZE > mmap.len() {
                // Need to grow the file
                if let Some(ref file) = self.global_file {
                    let new_size = (mmap.len() as u64) * 2;
                    file.set_len(new_size).map_err(|e| 
                        OzoneError::StorageError(format!("Failed to grow file: {}", e)))?;
                    
                    // Remap
                    *mmap = unsafe {
                        MmapOptions::new()
                            .map_mut(file)
                            .map_err(|e| OzoneError::StorageError(format!("Failed to remap: {}", e)))?
                    };
                }
            }
            
            mmap[offset..offset+8].copy_from_slice(&state.container_id.to_le_bytes());
            mmap[offset+8..offset+12].copy_from_slice(&state.child_count.to_le_bytes());
            mmap[offset+12..offset+16].copy_from_slice(&state.version.to_le_bytes());
            mmap[offset+16..offset+24].copy_from_slice(&state.parent_id.to_le_bytes());
            
            // Update file header
            mmap[20..28].copy_from_slice(&self.write_offset.to_le_bytes());
            
            mmap.flush().map_err(|e| OzoneError::StorageError(format!("Failed to flush: {}", e)))?;
        }
        
        Ok(())
    }
    
    /// Load local state from JSON file
    fn load_local(&self, id: ContainerID) -> OzoneResult<Option<LocalState>> {
        if let Some(cached) = self.local_cache.get(&id) {
            return Ok(Some(cached.clone()));
        }
        
        let path = self.local_path.join(format!("{}.json", id));
        if path.exists() {
            let contents = fs::read_to_string(&path)
                .map_err(|e| OzoneError::StorageError(format!("Failed to read local state: {}", e)))?;
            let state: LocalState = serde_json::from_str(&contents)
                .map_err(|e| OzoneError::StorageError(format!("Failed to parse local state: {}", e)))?;
            Ok(Some(state))
        } else {
            Ok(None)
        }
    }
    
    /// Store local state to JSON file
    fn store_local(&mut self, id: ContainerID, state: &LocalState) -> OzoneResult<()> {
        self.local_cache.insert(id, state.clone());
        
        let path = self.local_path.join(format!("{}.json", id));
        let contents = serde_json::to_string_pretty(state)
            .map_err(|e| OzoneError::StorageError(format!("Failed to serialize local state: {}", e)))?;
        
        fs::write(&path, contents)
            .map_err(|e| OzoneError::StorageError(format!("Failed to write local state: {}", e)))?;
        
        Ok(())
    }
    
    /// Get all container IDs
    pub fn all_ids(&self) -> Vec<ContainerID> {
        let mut ids: Vec<_> = self.index.keys().copied().collect();
        ids.sort();
        ids
    }
    
    /// Get children of a container
    pub fn get_children(&self, parent_id: ContainerID) -> OzoneResult<Vec<ContainerID>> {
        if let Some(container) = self.load(parent_id)? {
            Ok(container.global_state.child_ids)
        } else {
            Ok(Vec::new())
        }
    }
    
    /// Delete a container
    pub fn delete(&mut self, id: ContainerID) -> OzoneResult<()> {
        if id == 0 {
            return Err(OzoneError::StorageError("Cannot delete root container".into()));
        }
        
        self.index.remove(&id);
        self.local_cache.remove(&id);
        
        let path = self.local_path.join(format!("{}.json", id));
        if path.exists() {
            fs::remove_file(&path).ok();
        }
        
        Ok(())
    }
    
    /// Sync all data to disk
    pub fn sync(&mut self) -> OzoneResult<()> {
        if let Some(ref mut mmap) = self.global_mmap {
            mmap.flush().map_err(|e| OzoneError::StorageError(format!("Failed to sync: {}", e)))?;
        }
        Ok(())
    }
}
