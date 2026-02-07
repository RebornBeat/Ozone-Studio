//! Query processor for ZSEI
//!
//! Handles all query types from the ZSEIQuery enum (§6.7)

use crate::types::{ContainerID, OzoneError, OzoneResult, Blake3Hash};
use crate::types::zsei::{ZSEIQuery, ZSEIQueryResult, TaskSignature, IntegrityCheckResult, IntegrityIssue};
use crate::types::container::{ContainerType, Container, VersionRecord};
use super::storage::ContainerStorage;
use super::traversal::TraversalEngine;
use std::collections::HashMap;

/// Compute Blake3 hash of data
fn compute_blake3_hash(data: &[u8]) -> Blake3Hash {
    let hash = blake3::hash(data);
    *hash.as_bytes()
}

/// Query processor
pub struct QueryProcessor {
    /// Version history cache (container_id -> versions)
    version_history: HashMap<ContainerID, Vec<ContainerVersion>>,
}

#[derive(Debug, Clone)]
pub struct ContainerVersion {
    pub version: u32,
    pub timestamp: u64,
    pub changes: String,
    pub snapshot: Option<Container>,
}

impl QueryProcessor {
    /// Create new query processor
    pub fn new() -> Self {
        Self {
            version_history: HashMap::new(),
        }
    }
    
    /// Process a ZSEI query
    pub async fn process(
        &mut self,
        storage: &mut ContainerStorage,
        traversal: &TraversalEngine,
        query: ZSEIQuery,
    ) -> OzoneResult<ZSEIQueryResult> {
        match query {
            ZSEIQuery::GetUserWorkspaces { user_id } => {
                let ids = self.find_user_workspaces(storage, user_id)?;
                Ok(ZSEIQueryResult::Containers(ids))
            }
            
            ZSEIQuery::GetProjects { workspace_id } => {
                let ids = storage.get_children(workspace_id)?;
                Ok(ZSEIQueryResult::Containers(ids))
            }
            
            ZSEIQuery::GetProjectContext { project_id } => {
                if let Some(container) = storage.load(project_id)? {
                    Ok(ZSEIQueryResult::Container(container))
                } else {
                    Err(crate::types::OzoneError::NotFound(format!("Project {} not found", project_id)))
                }
            }
            
            ZSEIQuery::GetCategories { modality, parent_category } => {
                let ids = self.find_categories(storage, modality, parent_category)?;
                Ok(ZSEIQueryResult::Containers(ids))
            }
            
            ZSEIQuery::GetMethodologies { category_ids } => {
                let ids = self.find_methodologies_by_categories(storage, &category_ids)?;
                Ok(ZSEIQueryResult::Containers(ids))
            }
            
            ZSEIQuery::GetMethodologiesByKeywords { keywords } => {
                let ids = self.find_methodologies_by_keywords(storage, &keywords)?;
                Ok(ZSEIQueryResult::Containers(ids))
            }
            
            ZSEIQuery::SearchBlueprints { task_signature } => {
                let ids = self.search_blueprints(storage, task_signature)?;
                Ok(ZSEIQueryResult::Containers(ids))
            }
            
            ZSEIQuery::Traverse(request) => {
                let result = traversal.traverse(storage, request).await?;
                Ok(ZSEIQueryResult::TraversalResult(result))
            }
            
            ZSEIQuery::CreateContainer { parent_id, container } => {
                let new_id = self.create_container(storage, parent_id, container)?;
                Ok(ZSEIQueryResult::ContainerID(new_id))
            }
            
            ZSEIQuery::UpdateContainer { container_id, updates } => {
                self.update_container(storage, container_id, updates)?;
                Ok(ZSEIQueryResult::Success)
            }
            
            ZSEIQuery::DeleteContainer { container_id } => {
                self.delete_container(storage, container_id)?;
                Ok(ZSEIQueryResult::Success)
            }
            
            ZSEIQuery::VerifyIntegrity { container_id } => {
                let result = self.verify_integrity(storage, container_id)?;
                Ok(ZSEIQueryResult::IntegrityResult(result))
            }
            
            ZSEIQuery::GetVersionHistory { container_id } => {
                let history = self.get_version_history(container_id)?;
                Ok(ZSEIQueryResult::VersionHistory(history))
            }
            
            _ => {
                Err(OzoneError::ZSEIError("Unsupported query type".into()))
            }
        }
    }
    
    /// Create a new container
    fn create_container(
        &mut self,
        storage: &mut ContainerStorage,
        parent_id: ContainerID,
        mut container: Container,
    ) -> OzoneResult<ContainerID> {
        // Allocate new ID
        let new_id = storage.allocate_id();
        container.global_state.container_id = new_id;
        container.global_state.parent_id = parent_id;
        container.global_state.version = 1;
        
        // Store the container
        storage.store(&container)?;
        
        // Update parent's child list
        if let Some(mut parent) = storage.load(parent_id)? {
            parent.global_state.child_ids.push(new_id);
            parent.global_state.child_count = parent.global_state.child_ids.len() as u32;
            storage.store(&parent)?;
        }
        
        // Initialize version history
        self.version_history.insert(new_id, vec![ContainerVersion {
            version: 1,
            timestamp: now(),
            changes: "Initial creation".to_string(),
            snapshot: Some(container),
        }]);
        
        Ok(new_id)
    }
    
    /// Update an existing container
    fn update_container(
        &mut self,
        storage: &mut ContainerStorage,
        container_id: ContainerID,
        updates: crate::types::zsei::ContainerUpdate,
    ) -> OzoneResult<()> {
        let mut container = storage.load(container_id)?
            .ok_or_else(|| crate::types::OzoneError::NotFound(format!("Container {} not found", container_id)))?;
        
        // Apply updates based on what's provided
        if let Some(metadata) = updates.metadata {
            container.local_state.metadata = metadata;
        }
        if let Some(context) = updates.context {
            container.local_state.context = context;
        }
        if let Some(storage_ptrs) = updates.storage {
            container.local_state.storage = storage_ptrs;
        }
        if let Some(hints) = updates.hints {
            container.local_state.hints = hints;
        }
        
        // Increment version and update timestamp
        container.global_state.version += 1;
        container.local_state.metadata.updated_at = now();
        
        // Store updated container
        storage.store(&container)?;
        
        // Record version history
        let history = self.version_history.entry(container_id).or_insert_with(Vec::new);
        history.push(ContainerVersion {
            version: container.global_state.version,
            timestamp: now(),
            changes: "Container updated".to_string(),
            snapshot: Some(container),
        });
        
        Ok(())
    }
    
    /// Delete a container
    fn delete_container(
        &mut self,
        storage: &mut ContainerStorage,
        container_id: ContainerID,
    ) -> OzoneResult<()> {
        // Load container to get parent
        let container = storage.load(container_id)?
            .ok_or_else(|| OzoneError::NotFound(format!("Container {} not found", container_id)))?;
        
        // Remove from parent's child list
        if let Some(mut parent) = storage.load(container.global_state.parent_id)? {
            parent.global_state.child_ids.retain(|&id| id != container_id);
            parent.global_state.child_count = parent.global_state.child_ids.len() as u32;
            storage.store(&parent)?;
        }
        
        // Recursively delete children
        for child_id in container.global_state.child_ids {
            let _ = self.delete_container(storage, child_id);
        }
        
        // Delete the container
        storage.delete(container_id)?;
        
        // Remove version history
        self.version_history.remove(&container_id);
        
        Ok(())
    }
    
    /// Get version history for a container
    fn get_version_history(&self, container_id: ContainerID) -> OzoneResult<Vec<VersionRecord>> {
        let history = self.version_history.get(&container_id)
            .map(|h| h.iter().map(|v| {
                // Compute Blake3 hash of the snapshot if available
                let content_hash = if let Some(ref snapshot) = v.snapshot {
                    // Hash the serialized container data
                    let data = serde_json::to_string(&snapshot.local_state)
                        .unwrap_or_default();
                    compute_blake3_hash(data.as_bytes())
                } else {
                    // Hash the changes string as fallback
                    compute_blake3_hash(v.changes.as_bytes())
                };
                
                VersionRecord {
                    version: v.version as u64,
                    timestamp: v.timestamp,
                    content_hash,
                    change_type: crate::types::container::ChangeType::Update,
                    rollback_available: v.snapshot.is_some(),
                }
            }).collect())
            .unwrap_or_default();
        Ok(history)
    }
    
    /// Find user workspaces
    fn find_user_workspaces(
        &self,
        storage: &ContainerStorage,
        user_id: u64,
    ) -> OzoneResult<Vec<ContainerID>> {
        let mut results = Vec::new();
        
        for id in storage.all_ids() {
            if let Some(container) = storage.load(id)? {
                if container.local_state.metadata.container_type == ContainerType::Workspace
                    && container.local_state.metadata.owner_id == user_id
                {
                    results.push(id);
                }
            }
        }
        
        Ok(results)
    }
    
    /// Find categories
    fn find_categories(
        &self,
        storage: &ContainerStorage,
        modality: crate::types::container::Modality,
        parent_category: Option<ContainerID>,
    ) -> OzoneResult<Vec<ContainerID>> {
        let mut results = Vec::new();
        
        for id in storage.all_ids() {
            if let Some(container) = storage.load(id)? {
                if container.local_state.metadata.container_type == ContainerType::Category
                    && container.local_state.metadata.modality == modality
                {
                    if let Some(parent) = parent_category {
                        if container.global_state.parent_id == parent {
                            results.push(id);
                        }
                    } else {
                        results.push(id);
                    }
                }
            }
        }
        
        Ok(results)
    }
    
    /// Find methodologies by categories
    fn find_methodologies_by_categories(
        &self,
        storage: &ContainerStorage,
        category_ids: &[ContainerID],
    ) -> OzoneResult<Vec<ContainerID>> {
        let mut results = Vec::new();
        
        for id in storage.all_ids() {
            if let Some(container) = storage.load(id)? {
                if container.local_state.metadata.container_type == ContainerType::Methodology {
                    for cat_id in category_ids {
                        if container.local_state.context.categories.contains(cat_id) {
                            results.push(id);
                            break;
                        }
                    }
                }
            }
        }
        
        Ok(results)
    }
    
    /// Find methodologies by keywords
    fn find_methodologies_by_keywords(
        &self,
        storage: &ContainerStorage,
        keywords: &[String],
    ) -> OzoneResult<Vec<ContainerID>> {
        let mut results = Vec::new();
        
        for id in storage.all_ids() {
            if let Some(container) = storage.load(id)? {
                if container.local_state.metadata.container_type == ContainerType::Methodology {
                    for keyword in keywords {
                        if container.local_state.context.keywords.contains(keyword) {
                            results.push(id);
                            break;
                        }
                    }
                }
            }
        }
        
        Ok(results)
    }
    
    /// Search blueprints by task signature with proper matching
    fn search_blueprints(
        &self,
        storage: &ContainerStorage,
        task_signature: TaskSignature,
    ) -> OzoneResult<Vec<ContainerID>> {
        let mut results: Vec<(ContainerID, f32)> = Vec::new();
        
        for id in storage.all_ids() {
            if let Some(container) = storage.load(id)? {
                if container.local_state.metadata.container_type == ContainerType::Blueprint {
                    // Calculate match score based on TaskSignature fields
                    let mut score = 0.0f32;
                    let mut factors = 0;
                    
                    // Match output type against keywords (output_type might match a keyword)
                    if container.local_state.context.keywords.iter()
                        .any(|kw| kw.to_lowercase() == task_signature.output_type.to_lowercase()) {
                        score += 1.0;
                        factors += 1;
                    }
                    
                    // Match input types against container keywords/topics
                    let input_match = task_signature.input_types.iter()
                        .filter(|input| {
                            container.local_state.context.keywords.iter()
                                .any(|kw| kw.to_lowercase().contains(&input.to_lowercase()))
                        })
                        .count();
                    if !task_signature.input_types.is_empty() {
                        score += input_match as f32 / task_signature.input_types.len() as f32;
                        factors += 1;
                    }
                    
                    // Match constraints against container metadata
                    let constraint_match = task_signature.constraints.iter()
                        .filter(|c| {
                            container.local_state.context.topics.iter()
                                .any(|t| t.to_lowercase().contains(&c.to_lowercase()))
                        })
                        .count();
                    if !task_signature.constraints.is_empty() {
                        score += constraint_match as f32 / task_signature.constraints.len() as f32;
                        factors += 1;
                    }
                    
                    let final_score = if factors > 0 { score / factors as f32 } else { 0.0 };
                    
                    // Only include blueprints with score > 0.3
                    if final_score > 0.3 {
                        results.push((id, final_score));
                    }
                }
            }
        }
        
        // Sort by score descending
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(results.into_iter().map(|(id, _)| id).collect())
    }
    
    /// Verify container integrity
    fn verify_integrity(
        &self,
        storage: &ContainerStorage,
        container_id: ContainerID,
    ) -> OzoneResult<IntegrityCheckResult> {
        let mut issues: Vec<IntegrityIssue> = Vec::new();
        
        // Load container
        let container = match storage.load(container_id)? {
            Some(c) => c,
            None => {
                return Ok(IntegrityCheckResult {
                    container_id,
                    passed: false,
                    issues: vec![IntegrityIssue {
                        issue_type: crate::types::zsei::IntegrityIssueType::ReconstructionFailure,
                        description: "Container not found".to_string(),
                        severity: crate::types::Severity::Critical,
                        auto_repairable: false,
                    }],
                    checked_at: now(),
                });
            }
        };
        
        // Check parent exists
        if container.global_state.parent_id != 0 { // 0 is root
            if storage.load(container.global_state.parent_id)?.is_none() {
                issues.push(IntegrityIssue {
                    issue_type: crate::types::zsei::IntegrityIssueType::OrphanedReference,
                    description: format!("Parent container {} not found", container.global_state.parent_id),
                    severity: crate::types::Severity::High,
                    auto_repairable: false,
                });
            }
        }
        
        // Check child count matches
        if container.global_state.child_count as usize != container.global_state.child_ids.len() {
            issues.push(IntegrityIssue {
                issue_type: crate::types::zsei::IntegrityIssueType::VersionMismatch,
                description: format!("Child count mismatch: {} vs {}", 
                    container.global_state.child_count, container.global_state.child_ids.len()),
                severity: crate::types::Severity::Medium,
                auto_repairable: true,
            });
        }
        
        // Check all children exist
        for child_id in &container.global_state.child_ids {
            if storage.load(*child_id)?.is_none() {
                issues.push(IntegrityIssue {
                    issue_type: crate::types::zsei::IntegrityIssueType::RelationshipBroken,
                    description: format!("Child container {} not found", child_id),
                    severity: crate::types::Severity::High,
                    auto_repairable: false,
                });
            }
        }
        
        // Check version is valid
        if container.global_state.version == 0 {
            issues.push(IntegrityIssue {
                issue_type: crate::types::zsei::IntegrityIssueType::VersionMismatch,
                description: "Invalid version: 0".to_string(),
                severity: crate::types::Severity::Low,
                auto_repairable: true,
            });
        }
        
        Ok(IntegrityCheckResult {
            container_id,
            passed: issues.is_empty(),
            issues,
            checked_at: now(),
        })
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

impl Default for QueryProcessor {
    fn default() -> Self {
        Self::new()
    }
}
