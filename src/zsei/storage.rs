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
            // Structural root containers use fixed low ids (see
            // src/types/container.rs: MODALITY_ROOT_ID=1 through
            // CROSS_MODAL_INDEX_ROOT_ID=77) that this generic allocator has
            // no awareness of — starting at 1 meant the very first
            // dynamically-created container (via CreateContainer, e.g. a
            // modality graph or a methodology/blueprint created through
            // methodology_create/blueprint_create) got id 1, silently
            // overwriting MODALITY_ROOT_ID's real content; the next got 2
            // (METHODOLOGY_ROOT_ID), and so on — confirmed live this
            // session (GetContainer on ids 1 and 2 returned unrelated
            // ModalityGraph content instead of the real structural roots).
            // 1000 clears the entire reserved range with headroom, matching
            // the same floor already used by index.json's next_custom_id
            // convention, and stays below the deterministic offset ranges
            // (pipeline_container_id: 10000+, methodology_container_id:
            // 30000+, blueprint_container_id: 40000+, experience_container_id:
            // 100000+) so dynamic and hash-offset ids never collide either.
            next_id: 1000,
            write_offset: 64, // After file header
        };
        
        // Initialize storage
        if config.mmap_enabled {
            storage.init_mmap()?;
        } else {
            // Plain-file mode: open the same global file with a real handle
            // so store_global/load_global can seek+read/write records. Prior
            // to this, the handle was never opened and the plain branch
            // silently no-opped (real data-loss path, found by tests).
            storage.init_plain_file()?;
        }
        
        // Load existing data
        storage.load_index()?;
        storage.rebuild_child_ids_cache()?;
        storage.load_local_cache()?;

        // Create root container if not exists
        storage.ensure_root()?;
        
        Ok(storage)
    }
    
    /// Initialize the global file without mmap — same file, same header,
    /// same fixed-size records; accessed via seek/read/write instead of a
    /// memory view.
    fn init_plain_file(&mut self) -> OzoneResult<()> {
        let is_new = !self.global_path.exists();

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&self.global_path)
            .map_err(|e| OzoneError::StorageError(format!("Failed to open global file: {}", e)))?;

        let metadata = file
            .metadata()
            .map_err(|e| OzoneError::StorageError(format!("Failed to get file metadata: {}", e)))?;

        if metadata.len() < INITIAL_FILE_SIZE {
            file.set_len(INITIAL_FILE_SIZE)
                .map_err(|e| OzoneError::StorageError(format!("Failed to resize file: {}", e)))?;
        }

        // Write the file header if new — identical layout to the mmap path,
        // so a file written in either mode opens in both.
        if is_new {
            use std::io::{Seek, SeekFrom, Write};
            let mut file = file;
            file.seek(SeekFrom::Start(0)).map_err(|e| {
                OzoneError::StorageError(format!("Failed to seek global file: {}", e))
            })?;
            file.write_all(MAGIC_BYTES).map_err(|e| {
                OzoneError::StorageError(format!("Failed to write magic bytes: {}", e))
            })?;
            file.write_all(&FILE_VERSION.to_le_bytes()).map_err(|e| {
                OzoneError::StorageError(format!("Failed to write file version: {}", e))
            })?;
            file.write_all(&self.next_id.to_le_bytes()).map_err(|e| {
                OzoneError::StorageError(format!("Failed to write next_id: {}", e))
            })?;
            file.write_all(&64u64.to_le_bytes()).map_err(|e| {
                OzoneError::StorageError(format!("Failed to write write_offset: {}", e))
            })?;
            file.flush().map_err(|e| {
                OzoneError::StorageError(format!("Failed to flush header: {}", e))
            })?;
            self.global_file = Some(file);
        } else {
            self.global_file = Some(file);
        }
        Ok(())
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
            mmap[12..20].copy_from_slice(&self.next_id.to_le_bytes()); // next_id (see the reserved-range comment on the field)
            mmap[20..28].copy_from_slice(&64u64.to_le_bytes()); // write_offset
            mmap.flush().map_err(|e| OzoneError::StorageError(format!("Failed to flush: {}", e)))?;
        } else {
            // Verify magic bytes
            if &mmap[0..8] != MAGIC_BYTES {
                return Err(OzoneError::StorageError("Invalid storage file format".into()));
            }
            // Read next_id and write_offset. Clamp against the reserved-range
            // floor: an existing file created before this fix may have
            // already persisted a low next_id (having walked partway through
            // ids 1-77's structural roots) — max() here stops any further
            // corruption on this and every future load without needing to
            // reset the store. Already-corrupted low-id containers from
            // before this fix are a separate, one-time repair, not something
            // this clamp can undo.
            let stored_next_id = u64::from_le_bytes(mmap[12..20].try_into().unwrap());
            self.next_id = stored_next_id.max(self.next_id);
            self.write_offset = u64::from_le_bytes(mmap[20..28].try_into().unwrap());
        }
        
        self.global_file = Some(file);
        self.global_mmap = Some(mmap);
        
        Ok(())
    }
    
    /// Load index by scanning the global file — works in both storage modes
    /// (the mmap branch reads via memory view; the plain-file branch reads
    /// via seek+read). Without this, a plain-file store's index is empty
    /// after restart and no containers are loadable.
    fn load_index(&mut self) -> OzoneResult<()> {
        if let Some(ref mmap) = self.global_mmap {
            let mut offset = 64u64; // Skip file header
            
            while offset + HEADER_SIZE as u64 <= self.write_offset {
                let start = offset as usize;
                let container_id = u64::from_le_bytes(mmap[start..start+8].try_into().unwrap());
                
                if container_id != 0 || offset == 64 {
                    self.index.insert(container_id, offset);
                }
                
                offset += HEADER_SIZE as u64;
            }
        } else if let Some(ref file) = self.global_file {
            // Plain-file scan: read records in HEADER_SIZE strides from
            // offset 64 (after the file header), same layout as the mmap
            // branch. seek+read per record instead of a memory view.
            use std::io::{Seek, SeekFrom, Read};
            let mut scan_file = file.try_clone().map_err(|e| {
                OzoneError::StorageError(format!("Failed to clone for index scan: {}", e))
            })?;
            let file_len = scan_file.metadata().map_err(|e| {
                OzoneError::StorageError(format!("Failed to stat global file: {}", e))
            })?.len();
            let mut offset = 64u64;
            while offset + HEADER_SIZE as u64 <= file_len {
                scan_file.seek(SeekFrom::Start(offset)).map_err(|e| {
                    OzoneError::StorageError(format!("Index scan seek failed at {}: {}", offset, e))
                })?;
                let mut record = [0u8; 24];
                match scan_file.read_exact(&mut record) {
                    Ok(_) => {
                        let container_id = u64::from_le_bytes(record[0..8].try_into().unwrap());
                        if container_id != 0 {
                            self.index.insert(container_id, offset);
                        }
                    }
                    Err(e) => {
                        tracing::debug!(offset, error = %e, "Index scan: short read at offset, stopping");
                        break;
                    }
                }
                offset += HEADER_SIZE as u64;
            }
        }
        Ok(())
    }
    
    /// Reconstruct `child_ids_cache` from persisted `parent_id` links after a
    /// restart. Real bug confirmed live 2026-09-15: `GlobalState.child_ids`
    /// (a variable-length Vec) is never written to the fixed 64-byte mmap
    /// header record at all — `store_global` persists container_id,
    /// child_count, version, and parent_id in-place, but child_ids only ever
    /// lived in `child_ids_cache`, an in-memory HashMap that starts empty on
    /// every process start and is populated solely by this session's own
    /// `store()` calls. Net effect: EVERY container's child_ids read back as
    /// empty immediately after any restart, until re-populated by whatever
    /// this session happens to create fresh. Confirmed via direct on-disk
    /// inspection: the jurisdiction self-heal registration in src/lib.rs
    /// (which asks `get_container(JURISDICTION_ROOT_ID).child_ids` to detect
    /// already-registered scopes) found 0 already-registered on every boot
    /// and re-created all ~40 scopes each time — 299 duplicate
    /// JurisdictionRuleSet containers on disk for what should be ~41 unique
    /// ones. Also silently broke every "Structural" traversal mode's
    /// parent->child walk (`get_children`, reads the same broken field) for
    /// anything not freshly created this session. `parent_id` IS correctly
    /// persisted per-container (confirmed above, same header record), so it
    /// is reconstructible without a storage-format change: one pass over
    /// every indexed container reading its parent_id back and grouping by
    /// it. Cheap (one extra small read per container, boot-time only).
    fn rebuild_child_ids_cache(&mut self) -> OzoneResult<()> {
        let mut rebuilt: HashMap<ContainerID, Vec<ContainerID>> = HashMap::new();
        let ids: Vec<ContainerID> = self.index.keys().copied().collect();
        for id in ids {
            if let Some(state) = self.load_global(id)? {
                if state.parent_id != id {
                    rebuilt.entry(state.parent_id).or_default().push(id);
                }
            }
        }
        self.child_ids_cache = rebuilt;
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
            // Plain-file branch (mmap_enabled: false) — read the same
            // fixed-size record store_global wrote at this offset.
            use std::io::{Read, Seek, SeekFrom};
            let file = self.global_file.as_ref().ok_or_else(|| {
                OzoneError::StorageError("global file not initialized".to_string())
            })?;
            let mut file = file.try_clone().map_err(|e| {
                OzoneError::StorageError(format!("Failed to clone global file handle: {}", e))
            })?;
            let file_len = file.metadata().map_err(|e| {
                OzoneError::StorageError(format!("Failed to stat global file: {}", e))
            })?.len();
            if (offset as u64) + HEADER_SIZE as u64 > file_len {
                return Ok(None);
            }
            file.seek(SeekFrom::Start(offset as u64)).map_err(|e| {
                OzoneError::StorageError(format!("Failed to seek global file: {}", e))
            })?;
            let mut record = [0u8; 24];
            file.read_exact(&mut record).map_err(|e| {
                OzoneError::StorageError(format!("Failed to read global record: {}", e))
            })?;
            
            let container_id = u64::from_le_bytes(record[0..8].try_into().unwrap());
            let child_count = u32::from_le_bytes(record[8..12].try_into().unwrap());
            let version = u32::from_le_bytes(record[12..16].try_into().unwrap());
            let parent_id = u64::from_le_bytes(record[16..24].try_into().unwrap());
            
            // Child IDs live in the child_ids_cache (same as the mmap path)
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
        } else {
            // Plain-file branch (mmap_enabled: false): previously a silent
            // no-op — GlobalState bytes were never written, so persistence
            // silently didn't happen while load_global returned misses.
            // Write the same fixed-size record at the same offset via
            // seek+write so both storage modes produce identical files.
            use std::io::{Seek, SeekFrom, Write};
            let mut file = self.global_file.as_ref().ok_or_else(|| {
                OzoneError::StorageError("global file not initialized".to_string())
            })?.try_clone().map_err(|e| {
                OzoneError::StorageError(format!("Failed to clone global file handle: {}", e))
            })?;
            let need = offset + HEADER_SIZE;
            if need as u64 > file.metadata().map_err(|e| {
                OzoneError::StorageError(format!("Failed to stat global file: {}", e))
            })?.len() {
                file.set_len((need * 2) as u64).map_err(|e| {
                    OzoneError::StorageError(format!("Failed to grow global file: {}", e))
                })?;
            }
            file.seek(SeekFrom::Start(offset as u64)).map_err(|e| {
                OzoneError::StorageError(format!("Failed to seek global file: {}", e))
            })?;
            file.write_all(&state.container_id.to_le_bytes()).map_err(|e| {
                OzoneError::StorageError(format!("Failed to write container id: {}", e))
            })?;
            file.write_all(&state.child_count.to_le_bytes()).map_err(|e| {
                OzoneError::StorageError(format!("Failed to write child count: {}", e))
            })?;
            file.write_all(&state.version.to_le_bytes()).map_err(|e| {
                OzoneError::StorageError(format!("Failed to write version: {}", e))
            })?;
            file.write_all(&state.parent_id.to_le_bytes()).map_err(|e| {
                OzoneError::StorageError(format!("Failed to write parent id: {}", e))
            })?;

            // Keep the FILE header's write_offset current (bytes 20..28) —
            // same field the mmap branch maintains, so files are identical
            // across storage modes.
            file.seek(SeekFrom::Start(20)).map_err(|e| {
                OzoneError::StorageError(format!("Failed to seek header: {}", e))
            })?;
            file.write_all(&self.write_offset.to_le_bytes()).map_err(|e| {
                OzoneError::StorageError(format!("Failed to write header offset: {}", e))
            })?;
            file.flush().map_err(|e| {
                OzoneError::StorageError(format!("Failed to flush global file: {}", e))
            })?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ZSEIConfig;
    use crate::types::container::{Container, ContainerType, LocalState, Metadata, Modality};

    fn test_config(tag: &str, mmap: bool) -> ZSEIConfig {
        let dir = std::env::temp_dir().join(format!(
            "ozone_storage_{}_{}_{}",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        ZSEIConfig {
            global_path: dir.join("global.mmap").to_string_lossy().into(),
            local_path: dir.join("local").to_string_lossy().into(),
            cache_path: dir.join("cache").to_string_lossy().into(),
            ml_path: dir.join("ml").to_string_lossy().into(),
            max_containers_in_memory: 1000,
            mmap_enabled: mmap,
            embedding_dimension: 384,
            pipeline_index_path: dir.join("pi.json").to_string_lossy().into(),
            methodology_index_path: dir.join("mi.json").to_string_lossy().into(),
            blueprint_index_path: dir.join("bi.json").to_string_lossy().into(),
        }
    }

    fn sample(id: ContainerID, parent: ContainerID) -> Container {
        Container {
            global_state: GlobalState {
                container_id: id,
                parent_id: parent,
                child_ids: vec![],
                child_count: 0,
                version: 1,
            },
            local_state: LocalState {
                metadata: Metadata {
                    container_type: ContainerType::Project,
                    modality: Modality::Unknown,
                    created_at: 0,
                    updated_at: 0,
                    provenance: "test".into(),
                    permissions: 0,
                    owner_id: 0,
                    name: Some(format!("test-{id}")),
                    materialized_path: None,
                },
                ..Default::default()
            },
        }
    }

    // T-S7: mmap path — the production path keeps round-tripping.
    #[test]
    fn mmap_store_load_roundtrip() {
        let mut storage = ContainerStorage::new(&test_config("mmap", true)).unwrap();
        storage.store(&sample(1001, 8)).unwrap();
        let loaded = storage.load(1001).unwrap().expect("container found");
        assert_eq!(loaded.global_state.container_id, 1001);
        assert_eq!(loaded.global_state.parent_id, 8);
    }

    // T-S6: the mmap:false data-loss fix — store_global writes real bytes
    // via the plain-file branch and load_global reads them back. Before the
    // fix this test's load returned None (silent no-op).
    #[test]
    fn plain_file_store_load_roundtrip() {
        let mut storage = ContainerStorage::new(&test_config("plain", false)).unwrap();
        storage.store(&sample(2001, 8)).unwrap();
        let loaded = storage.load(2001).unwrap().expect("container found after plain-file write");
        assert_eq!(loaded.global_state.container_id, 2001);
        assert_eq!(loaded.global_state.parent_id, 8);
        // Overwrite in place: same id, changed parent — index reuses offset.
        storage.store(&sample(2001, 9)).unwrap();
        let loaded = storage.load(2001).unwrap().unwrap();
        assert_eq!(loaded.global_state.parent_id, 9);
    }

    // Cross-mode (deferred): a plain-written file read via the mmap branch
    // needs the in-memory index rebuild semantics pinned down first
    // (load_index/next_id interplay across instances) — see the storage
    // notes in docs/LIVING_GRAPH_STATUS.md. Byte layouts are identical by
    // construction (both branches write the same 24-byte record at the same
    // offset); only the index rebuild is untested.
    #[test]
    fn cross_mode_byte_compatibility() {
        let tag = format!(
            "x_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let dir = std::env::temp_dir().join(format!("ozone_cross_{tag}"));
        let mut cfg_plain = test_config("unused", false);
        let gp = dir.join("global.mmap").to_string_lossy().into_owned();
        cfg_plain.global_path = gp.clone();
        {
            let mut storage = ContainerStorage::new(&cfg_plain).unwrap();
            storage.store(&sample(3001, 8)).unwrap();
        }
        let mut cfg_mmap = cfg_plain.clone();
        cfg_mmap.mmap_enabled = true;
        let storage = ContainerStorage::new(&cfg_mmap).unwrap();
        let loaded = storage.load(3001).unwrap().expect("plain-written file readable via mmap");
        assert_eq!(loaded.global_state.parent_id, 8);
        let _ = gp;
    }
}
