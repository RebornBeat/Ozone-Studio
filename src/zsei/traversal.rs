//! Traversal engine for ZSEI
//!
//! Implements traversal modes from §6.7:
//! - Structural: Follow parent/child relationships
//! - Semantic: Follow learned associations and embeddings
//! - Contextual: Based on task context
//! - Hybrid: Combine multiple modes
//! - MLGuided: Use ML model predictions
//! - BruteForce: Exhaustive search (fallback)

use crate::config::ZSEIConfig;
use crate::types::{ContainerID, OzoneResult, Value};
use crate::types::zsei::{
    TraversalRequest, TraversalResult, TraversalMode, Path,
    TraversalStats, Filter, Operator,
};
use crate::types::container::Container;
use super::storage::ContainerStorage;
use std::collections::{HashSet, VecDeque};
use regex::Regex;

/// Traversal engine
pub struct TraversalEngine {
    /// Embedding dimension for semantic search
    embedding_dimension: usize,
    
    /// ML model path (if using ML-guided traversal)
    ml_path: String,
}

impl TraversalEngine {
    /// Create new traversal engine
    pub fn new(config: &ZSEIConfig) -> OzoneResult<Self> {
        Ok(Self {
            embedding_dimension: config.embedding_dimension,
            ml_path: config.ml_path.clone(),
        })
    }
    
    /// Perform traversal based on request
    pub async fn traverse(
        &self,
        storage: &ContainerStorage,
        request: TraversalRequest,
    ) -> OzoneResult<TraversalResult> {
        let start_time = std::time::Instant::now();
        
        let (containers, paths) = match request.mode {
            TraversalMode::Structural => {
                self.structural_traversal(storage, &request).await?
            }
            TraversalMode::Semantic => {
                self.semantic_traversal(storage, &request).await?
            }
            TraversalMode::Contextual => {
                self.contextual_traversal(storage, &request).await?
            }
            TraversalMode::Hybrid => {
                self.hybrid_traversal(storage, &request).await?
            }
            TraversalMode::MLGuided => {
                self.ml_guided_traversal(storage, &request).await?
            }
            TraversalMode::BruteForce => {
                self.brute_force_traversal(storage, &request).await?
            }
        };
        
        let elapsed = start_time.elapsed();
        let containers_count = containers.len() as u32;
        
        Ok(TraversalResult {
            containers,
            distances: vec![1.0; paths.len()], // Default distances
            paths,
            stats: TraversalStats {
                containers_visited: containers_count,
                hops_taken: request.max_depth,
                latency_ms: elapsed.as_millis() as u32,
                cache_hits: 0,
                ml_predictions_used: 0,
                brute_force_fallback: request.mode == TraversalMode::BruteForce,
            },
            methodologies: Vec::new(),
            external_refs: Vec::new(),
            keywords_found: Vec::new(),
            topics_found: Vec::new(),
        })
    }
    
    /// Structural traversal - follow parent/child relationships
    async fn structural_traversal(
        &self,
        storage: &ContainerStorage,
        request: &TraversalRequest,
    ) -> OzoneResult<(Vec<ContainerID>, Vec<Path>)> {
        let mut containers = Vec::new();
        let mut paths = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        
        // Start from the starting container
        queue.push_back((request.start_container, vec![request.start_container]));
        
        let max_depth = request.max_depth;
        let max_containers = request.max_results;
        
        while let Some((current_id, path)) = queue.pop_front() {
            if containers.len() >= max_containers as usize {
                break;
            }
            
            if path.len() > max_depth as usize {
                continue;
            }
            
            if visited.contains(&current_id) {
                continue;
            }
            visited.insert(current_id);
            
            // Check if this container matches filters
            if self.matches_filters(storage, current_id, &request.filters)? {
                containers.push(current_id);
                paths.push(Path {
                    hops: path.clone(),
                    total_distance: path.len() as f32,
                });
            }
            
            // Add children to queue
            let children = storage.get_children(current_id)?;
            for child_id in children {
                if !visited.contains(&child_id) {
                    let mut new_path = path.clone();
                    new_path.push(child_id);
                    queue.push_back((child_id, new_path));
                }
            }
        }
        
        Ok((containers, paths))
    }
    
    /// Semantic traversal - follow embeddings and associations
    async fn semantic_traversal(
        &self,
        storage: &ContainerStorage,
        request: &TraversalRequest,
    ) -> OzoneResult<(Vec<ContainerID>, Vec<Path>)> {
        let mut containers = Vec::new();
        let mut paths = Vec::new();
        let mut visited = HashSet::new();
        
        // Semantic traversal uses keyword/topic similarity instead of structure
        // Get starting container's keywords and topics
        let start_container = match storage.load(request.start_container)? {
            Some(c) => c,
            None => return self.structural_traversal(storage, request).await,
        };
        
        let start_keywords: HashSet<String> = start_container.local_state.context.keywords
            .iter().map(|k| k.to_lowercase()).collect();
        let start_topics: HashSet<String> = start_container.local_state.context.topics
            .iter().map(|t| t.to_lowercase()).collect();
        
        // Search all containers for keyword/topic similarity
        for id in storage.all_ids() {
            if containers.len() >= request.max_results as usize {
                break;
            }
            
            if visited.contains(&id) || id == request.start_container {
                continue;
            }
            visited.insert(id);
            
            if let Some(container) = storage.load(id)? {
                // Calculate semantic similarity based on shared keywords/topics
                let container_keywords: HashSet<String> = container.local_state.context.keywords
                    .iter().map(|k| k.to_lowercase()).collect();
                let container_topics: HashSet<String> = container.local_state.context.topics
                    .iter().map(|t| t.to_lowercase()).collect();
                
                let keyword_overlap = start_keywords.intersection(&container_keywords).count();
                let topic_overlap = start_topics.intersection(&container_topics).count();
                
                // Consider semantically similar if any keyword or topic overlap
                if keyword_overlap > 0 || topic_overlap > 0 {
                    // Check filters
                    if self.matches_filters(storage, id, &request.filters)? {
                        let similarity = (keyword_overlap + topic_overlap) as f32;
                        containers.push(id);
                        paths.push(Path {
                            hops: vec![request.start_container, id],
                            total_distance: 1.0 / (1.0 + similarity), // Lower distance for more similar
                        });
                    }
                }
            }
        }
        
        // Sort by similarity (lower total_distance = more similar)
        let mut combined: Vec<_> = containers.into_iter().zip(paths.into_iter()).collect();
        combined.sort_by(|a, b| a.1.total_distance.partial_cmp(&b.1.total_distance).unwrap_or(std::cmp::Ordering::Equal));
        
        let (containers, paths): (Vec<_>, Vec<_>) = combined.into_iter().unzip();
        
        Ok((containers, paths))
    }
    
    /// Contextual traversal - based on task context
    async fn contextual_traversal(
        &self,
        storage: &ContainerStorage,
        request: &TraversalRequest,
    ) -> OzoneResult<(Vec<ContainerID>, Vec<Path>)> {
        let mut containers = Vec::new();
        let mut paths = Vec::new();
        let mut visited = HashSet::new();
        
        // Context-aware traversal considers the task's requirements
        // Extract context from filters (e.g., container_type, modality requirements)
        let required_types: Vec<String> = request.filters.iter()
            .filter(|f| f.field == "container_type")
            .filter_map(|f| {
                if let crate::types::Value::String(s) = &f.value {
                    Some(s.clone())
                } else { None }
            })
            .collect();
        
        let required_modalities: Vec<String> = request.filters.iter()
            .filter(|f| f.field == "modality")
            .filter_map(|f| {
                if let crate::types::Value::String(s) = &f.value {
                    Some(s.clone())
                } else { None }
            })
            .collect();
        
        // Start from the starting container and expand based on context relevance
        let mut queue = VecDeque::new();
        queue.push_back((request.start_container, vec![request.start_container], 0u32));
        
        while let Some((current_id, path, depth)) = queue.pop_front() {
            if containers.len() >= request.max_results as usize {
                break;
            }
            
            if depth > request.max_depth as u32 {
                continue;
            }
            
            if visited.contains(&current_id) {
                continue;
            }
            visited.insert(current_id);
            
            if let Some(container) = storage.load(current_id)? {
                // Calculate context relevance score
                let mut relevance = 0.0f32;
                
                // Type match
                let type_str = format!("{:?}", container.local_state.metadata.container_type);
                if required_types.is_empty() || required_types.iter().any(|t| type_str.contains(t)) {
                    relevance += 0.5;
                }
                
                // Modality match
                let modality_str = format!("{:?}", container.local_state.metadata.modality);
                if required_modalities.is_empty() || required_modalities.iter().any(|m| modality_str.contains(m)) {
                    relevance += 0.3;
                }
                
                // Keyword/topic presence
                if !container.local_state.context.keywords.is_empty() {
                    relevance += 0.1;
                }
                if !container.local_state.context.topics.is_empty() {
                    relevance += 0.1;
                }
                
                // Include if relevant and matches filters
                if relevance > 0.3 && self.matches_filters(storage, current_id, &request.filters)? {
                    containers.push(current_id);
                    paths.push(Path {
                        hops: path.clone(),
                        total_distance: depth as f32 + (1.0 - relevance),
                    });
                }
                
                // Add children to queue
                for child_id in &container.global_state.child_ids {
                    if !visited.contains(child_id) {
                        let mut new_path = path.clone();
                        new_path.push(*child_id);
                        queue.push_back((*child_id, new_path, depth + 1));
                    }
                }
            }
        }
        
        Ok((containers, paths))
    }
    
    /// Hybrid traversal - combine multiple modes
    async fn hybrid_traversal(
        &self,
        storage: &ContainerStorage,
        request: &TraversalRequest,
    ) -> OzoneResult<(Vec<ContainerID>, Vec<Path>)> {
        // Combine structural and semantic results
        let (mut containers, mut paths) = self.structural_traversal(storage, request).await?;
        let (semantic_containers, semantic_paths) = self.semantic_traversal(storage, request).await?;
        
        // Merge and deduplicate
        let existing: HashSet<_> = containers.iter().copied().collect();
        
        for (container, path) in semantic_containers.into_iter().zip(semantic_paths) {
            if !existing.contains(&container) {
                containers.push(container);
                paths.push(path);
            }
        }
        
        Ok((containers, paths))
    }
    
    /// ML-guided traversal - use trained model for path prediction
    async fn ml_guided_traversal(
        &self,
        storage: &ContainerStorage,
        request: &TraversalRequest,
    ) -> OzoneResult<(Vec<ContainerID>, Vec<Path>)> {
        // ML-guided traversal uses a trained model to predict best paths
        // If ML model not available, fall back to hybrid with intelligent scoring
        
        let ml_model_path = std::path::Path::new(&self.ml_path);
        
        if !ml_model_path.exists() {
            tracing::debug!("ML model not found at {:?}, falling back to hybrid", ml_model_path);
            return self.hybrid_traversal(storage, request).await;
        }
        
        // Load ML predictions (simplified - in production would use actual ML inference)
        // For now, combine structural and semantic with learned weights
        let (structural_containers, structural_paths) = self.structural_traversal(storage, request).await?;
        let (semantic_containers, semantic_paths) = self.semantic_traversal(storage, request).await?;
        
        // Combine with ML-learned weights
        let mut scored_results: Vec<(ContainerID, Path, f32)> = Vec::new();
        
        // Score structural results (weight: 0.6)
        for (container, path) in structural_containers.iter().zip(structural_paths.iter()) {
            let score = 0.6 * (1.0 / (1.0 + path.total_distance));
            scored_results.push((*container, path.clone(), score));
        }
        
        // Score semantic results (weight: 0.4)
        for (container, path) in semantic_containers.iter().zip(semantic_paths.iter()) {
            if let Some(existing) = scored_results.iter_mut().find(|(c, _, _)| c == container) {
                existing.2 += 0.4 * (1.0 / (1.0 + path.total_distance));
            } else {
                let score = 0.4 * (1.0 / (1.0 + path.total_distance));
                scored_results.push((*container, path.clone(), score));
            }
        }
        
        // Sort by combined score
        scored_results.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        
        // Take top results
        let max_results = request.max_results as usize;
        let (containers, paths): (Vec<_>, Vec<_>) = scored_results.into_iter()
            .take(max_results)
            .map(|(c, p, _)| (c, p))
            .unzip();
        
        Ok((containers, paths))
    }
    
    /// Brute force traversal - exhaustive search (fallback)
    async fn brute_force_traversal(
        &self,
        storage: &ContainerStorage,
        request: &TraversalRequest,
    ) -> OzoneResult<(Vec<ContainerID>, Vec<Path>)> {
        let mut containers = Vec::new();
        let mut paths = Vec::new();
        
        // Check all containers
        for id in storage.all_ids() {
            if containers.len() >= request.max_results as usize {
                break;
            }
            
            if self.matches_filters(storage, id, &request.filters)? {
                containers.push(id);
                paths.push(Path {
                    hops: vec![id],
                    total_distance: 1.0,
                });
            }
        }
        
        Ok((containers, paths))
    }
    
    /// Check if a container matches the given filters
    fn matches_filters(
        &self,
        storage: &ContainerStorage,
        id: ContainerID,
        filters: &[Filter],
    ) -> OzoneResult<bool> {
        if filters.is_empty() {
            return Ok(true);
        }
        
        let container = match storage.load(id)? {
            Some(c) => c,
            None => return Ok(false),
        };
        
        for filter in filters {
            let matches = match &filter.operator {
                Operator::Equals => {
                    self.get_field_value(&container, &filter.field) == Some(filter.value.clone())
                }
                Operator::Contains => {
                    if let Some(val) = self.get_field_value(&container, &filter.field) {
                        val.to_string().contains(&filter.value.to_string())
                    } else {
                        false
                    }
                }
                Operator::GreaterThan => {
                    if let Some(val) = self.get_field_value(&container, &filter.field) {
                        match (&val, &filter.value) {
                            (crate::types::Value::Int(a), crate::types::Value::Int(b)) => *a > *b,
                            (crate::types::Value::Float(a), crate::types::Value::Float(b)) => *a > *b,
                            _ => false
                        }
                    } else {
                        false
                    }
                }
                Operator::LessThan => {
                    if let Some(val) = self.get_field_value(&container, &filter.field) {
                        match (&val, &filter.value) {
                            (crate::types::Value::Int(a), crate::types::Value::Int(b)) => *a < *b,
                            (crate::types::Value::Float(a), crate::types::Value::Float(b)) => *a < *b,
                            _ => false
                        }
                    } else {
                        false
                    }
                }
                Operator::In => {
                    if let (Some(val), crate::types::Value::Array(arr)) = (self.get_field_value(&container, &filter.field), &filter.value) {
                        arr.iter().any(|v| v == &val)
                    } else {
                        false
                    }
                }
                Operator::NotEquals => {
                    self.get_field_value(&container, &filter.field) != Some(filter.value.clone())
                }
                Operator::HasKeyword => {
                    if let crate::types::Value::String(keyword) = &filter.value {
                        container.local_state.context.keywords.contains(keyword)
                    } else {
                        false
                    }
                }
                Operator::HasTopic => {
                    if let crate::types::Value::String(topic) = &filter.value {
                        container.local_state.context.topics.contains(topic)
                    } else {
                        false
                    }
                }
                Operator::Custom(op_name) => {
                    // Custom operators can be registered and executed
                    // For now, support common custom operators
                    match op_name.as_str() {
                        "starts_with" => {
                            if let (Some(crate::types::Value::String(val)), crate::types::Value::String(prefix)) = 
                                (self.get_field_value(&container, &filter.field), &filter.value) {
                                val.starts_with(prefix.as_str())
                            } else {
                                false
                            }
                        }
                        "ends_with" => {
                            if let (Some(crate::types::Value::String(val)), crate::types::Value::String(suffix)) = 
                                (self.get_field_value(&container, &filter.field), &filter.value) {
                                val.ends_with(suffix.as_str())
                            } else {
                                false
                            }
                        }
                        "regex" => {
                            if let (Some(crate::types::Value::String(val)), crate::types::Value::String(pattern)) = 
                                (self.get_field_value(&container, &filter.field), &filter.value) {
                                Regex::new(pattern).map(|r| r.is_match(&val)).unwrap_or(false)
                            } else {
                                false
                            }
                        }
                        _ => {
                            tracing::warn!("Unknown custom operator: {}", op_name);
                            false
                        }
                    }
                }
            };
            
            if !matches {
                return Ok(false);
            }
        }
        
        Ok(true)
    }
    
    /// Get a field value from a container for filtering
    fn get_field_value(
        &self,
        container: &crate::types::container::Container,
        field: &str,
    ) -> Option<crate::types::Value> {
        match field {
            "container_type" => Some(crate::types::Value::String(
                format!("{:?}", container.local_state.metadata.container_type)
            )),
            "modality" => Some(crate::types::Value::String(
                format!("{:?}", container.local_state.metadata.modality)
            )),
            "owner_id" => Some(crate::types::Value::Int(
                container.local_state.metadata.owner_id as i64
            )),
            _ => None,
        }
    }
}
