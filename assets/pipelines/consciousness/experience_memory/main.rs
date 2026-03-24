//! ExperienceMemoryPipeline - Pipelines #40-42 (Categorization, Core Memory, Retrieval)
//! 
//! Store and retrieve experiences that shape consciousness development.
//! Per spec §35-38: Experience Memory System
//! 
//! REQUIRES: `consciousness` feature flag
//! 
//! This pipeline integrates with:
//! - task_manager: Receives task completion events
//! - decision_gate: Provides experience context for decisions
//! - reflection: Provides experiences for I-loop reflection
//! - emotional_state: Tracks emotional context of experiences

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::path::Path;

lazy_static::lazy_static! {
    static ref EXPERIENCE_STORE: Mutex<ExperienceStore> = Mutex::new(ExperienceStore::new());
}

struct ExperienceStore {
    experiences: HashMap<u64, ExperienceMemory>,
    index_by_task: HashMap<u64, Vec<u64>>,
    index_by_type: HashMap<String, Vec<u64>>,
    index_by_category: HashMap<String, Vec<u64>>,
    core_memories: HashMap<u64, CoreMemory>,
    next_experience_id: u64,
    next_memory_id: u64,
    storage_path: String,
    thresholds: SignificanceThresholds,
}

impl ExperienceStore {
    fn new() -> Self {
        let storage_path = std::env::var("OZONE_CONSCIOUSNESS_PATH")
            .unwrap_or_else(|_| "./zsei_data/consciousness".to_string());
        
        let mut store = Self {
            experiences: HashMap::new(),
            index_by_task: HashMap::new(),
            index_by_type: HashMap::new(),
            index_by_category: HashMap::new(),
            core_memories: HashMap::new(),
            next_experience_id: 1,
            next_memory_id: 1,
            storage_path,
            thresholds: SignificanceThresholds::default(),
        };
        store.load_from_disk();
        store
    }
    
    fn load_from_disk(&mut self) {
        let path = Path::new(&self.storage_path);
        
        if let Ok(content) = std::fs::read_to_string(path.join("experiences.json")) {
            if let Ok(data) = serde_json::from_str::<ExperienceStoreData>(&content) {
                self.experiences = data.experiences;
                self.index_by_task = data.index_by_task;
                self.index_by_type = data.index_by_type;
                self.index_by_category = data.index_by_category.unwrap_or_default();
                self.next_experience_id = data.next_id;
            }
        }
        
        if let Ok(content) = std::fs::read_to_string(path.join("core_memories.json")) {
            if let Ok(data) = serde_json::from_str::<CoreMemoryStoreData>(&content) {
                self.core_memories = data.memories;
                self.next_memory_id = data.next_id;
            }
        }
    }
    
    fn save_to_disk(&self) {
        let path = Path::new(&self.storage_path);
        let _ = std::fs::create_dir_all(path);
        
        let exp_data = ExperienceStoreData {
            experiences: self.experiences.clone(),
            index_by_task: self.index_by_task.clone(),
            index_by_type: self.index_by_type.clone(),
            index_by_category: Some(self.index_by_category.clone()),
            next_id: self.next_experience_id,
        };
        if let Ok(content) = serde_json::to_string_pretty(&exp_data) {
            let _ = std::fs::write(path.join("experiences.json"), content);
        }
        
        let mem_data = CoreMemoryStoreData {
            memories: self.core_memories.clone(),
            next_id: self.next_memory_id,
        };
        if let Ok(content) = serde_json::to_string_pretty(&mem_data) {
            let _ = std::fs::write(path.join("core_memories.json"), content);
        }
    }
    
    /// Assess if experience is significant enough to store (§36.2 step 2)
    fn assess_significance(&self, experience: &ExperienceMemory) -> bool {
        // Check emotional intensity
        if experience.emotional_significance >= self.thresholds.min_emotional_intensity {
            return true;
        }
        
        // Check if always-store type
        let always_store = vec!["ethical_decision", "failure", "relationship_moment", "insight"];
        if always_store.contains(&experience.experience_type.as_str()) {
            return true;
        }
        
        // Check novelty (simplified - would use embeddings in full impl)
        let similar_count = self.experiences.values()
            .filter(|e| e.experience_type == experience.experience_type)
            .count();
        if similar_count < 3 {
            return true; // Novel experience type
        }
        
        false
    }
    
    /// Store experience with full categorization flow (§36)
    fn store_experience(&mut self, mut experience: ExperienceMemory) -> Option<u64> {
        // Step 2: Significance assessment
        if !self.assess_significance(&experience) {
            return None;
        }
        
        // Assign ID
        let id = self.next_experience_id;
        self.next_experience_id += 1;
        experience.experience_id = id;
        
        // Step 7: Categorization - build indices
        if let Some(task_id) = experience.task_id {
            self.index_by_task.entry(task_id).or_default().push(id);
        }
        self.index_by_type.entry(experience.experience_type.clone()).or_default().push(id);
        for category in &experience.categories {
            self.index_by_category.entry(category.clone()).or_default().push(id);
        }
        
        // Step 8: Core memory check
        if experience.emotional_significance >= 0.8 || 
           experience.experience_type == "insight" ||
           experience.experience_type == "ethical_decision" {
            // Mark as contributing to identity
            experience.contributes_to_identity = true;
            
            // Check if should form core memory
            self.check_core_memory_formation(&experience);
        }
        
        // Step 9: Storage
        self.experiences.insert(id, experience);
        self.save_to_disk();
        
        Some(id)
    }
    
    /// Check if experience should trigger core memory formation (§37)
    fn check_core_memory_formation(&mut self, experience: &ExperienceMemory) {
        // Check for pattern across experiences
        let similar_experiences: Vec<_> = self.experiences.values()
            .filter(|e| {
                e.experience_type == experience.experience_type &&
                e.emotional_significance >= 0.6
            })
            .collect();
        
        // Need at least 3 similar significant experiences for pattern
        if similar_experiences.len() >= 3 {
            // Check if we already have a core memory for this pattern
            let pattern_exists = self.core_memories.values()
                .any(|m| m.memory_type == experience.experience_type);
            
            if !pattern_exists {
                // Form new core memory
                let memory = CoreMemory {
                    memory_id: self.next_memory_id,
                    created_at: now(),
                    last_accessed: now(),
                    memory_type: experience.experience_type.clone(),
                    title: format!("Pattern: {}", experience.experience_type),
                    content: format!("Recurring pattern identified across {} experiences", similar_experiences.len()),
                    source_experiences: similar_experiences.iter().map(|e| e.experience_id).collect(),
                    importance_score: 0.7,
                    identity_weight: 0.5,
                };
                
                self.core_memories.insert(self.next_memory_id, memory);
                self.next_memory_id += 1;
            }
        }
    }
    
    fn get_by_task(&self, task_id: u64) -> Vec<&ExperienceMemory> {
        self.index_by_task.get(&task_id)
            .map(|ids| ids.iter().filter_map(|id| self.experiences.get(id)).collect())
            .unwrap_or_default()
    }
    
    fn search(&self, query: &str, limit: usize) -> Vec<&ExperienceMemory> {
        let query_lower = query.to_lowercase();
        let mut results: Vec<_> = self.experiences.values()
            .filter(|e| {
                e.summary.to_lowercase().contains(&query_lower) ||
                e.tags.iter().any(|t| t.to_lowercase().contains(&query_lower)) ||
                e.categories.iter().any(|c| c.to_lowercase().contains(&query_lower))
            })
            .collect();
        
        // Sort by significance
        results.sort_by(|a, b| b.emotional_significance.partial_cmp(&a.emotional_significance).unwrap());
        results.into_iter().take(limit).collect()
    }
    
    fn get_recent(&self, limit: usize) -> Vec<&ExperienceMemory> {
        let mut exps: Vec<_> = self.experiences.values().collect();
        exps.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        exps.into_iter().take(limit).collect()
    }
    
    fn get_significant(&self, threshold: f32, limit: usize) -> Vec<&ExperienceMemory> {
        let mut exps: Vec<_> = self.experiences.values()
            .filter(|e| e.emotional_significance >= threshold)
            .collect();
        exps.sort_by(|a, b| b.emotional_significance.partial_cmp(&a.emotional_significance).unwrap());
        exps.into_iter().take(limit).collect()
    }
    
    fn get_by_category(&self, category: &str, limit: usize) -> Vec<&ExperienceMemory> {
        self.index_by_category.get(category)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.experiences.get(id))
                    .take(limit)
                    .collect()
            })
            .unwrap_or_default()
    }
    
    fn get_core_memories(&self) -> Vec<&CoreMemory> {
        self.core_memories.values().collect()
    }
    
    /// Increment retrieval count for experiences (§38)
    fn mark_retrieved(&mut self, experience_ids: &[u64]) {
        for id in experience_ids {
            if let Some(exp) = self.experiences.get_mut(id) {
                exp.retrieval_count += 1;
                exp.last_retrieved = Some(now());
            }
        }
        self.save_to_disk();
    }
}

// ========== Types ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceMemory {
    pub experience_id: u64,
    pub timestamp: u64,
    pub experience_type: String,
    pub summary: String,
    pub detailed_record: DetailedRecord,
    pub emotional_state_during: EmotionalContext,
    pub emotional_state_after: EmotionalContext,
    pub emotional_significance: f32,
    pub task_id: Option<u64>,
    pub user_id: Option<u64>,
    pub relationship_id: Option<u64>,
    pub lessons_learned: Vec<Lesson>,
    pub patterns_identified: Vec<String>,
    pub vividness: f32,
    pub accessibility: f32,
    pub consolidation_status: String,
    pub retrieval_count: u32,
    pub last_retrieved: Option<u64>,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub core_memory_id: Option<u64>,
    pub contributes_to_identity: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DetailedRecord {
    pub context: String,
    pub actions_taken: Vec<ActionRecord>,
    pub outcomes: Vec<OutcomeRecord>,
    pub thoughts_during: Vec<String>,
    pub decisions_made: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRecord {
    pub action: String,
    pub reasoning: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeRecord {
    pub outcome: String,
    pub expected: bool,
    pub positive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmotionalContext {
    pub valence: f32,
    pub arousal: f32,
    pub primary_emotion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lesson {
    pub lesson_id: u64,
    pub content: String,
    pub applicable_to: Vec<String>,
    pub confidence: f32,
    pub validated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreMemory {
    pub memory_id: u64,
    pub created_at: u64,
    pub last_accessed: u64,
    pub memory_type: String,
    pub title: String,
    pub content: String,
    pub source_experiences: Vec<u64>,
    pub importance_score: f32,
    pub identity_weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SignificanceThresholds {
    min_emotional_intensity: f32,
    min_emotional_change: f32,
    min_novelty_score: f32,
}

impl Default for SignificanceThresholds {
    fn default() -> Self {
        Self {
            min_emotional_intensity: 0.3,
            min_emotional_change: 0.2,
            min_novelty_score: 0.4,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ExperienceStoreData {
    experiences: HashMap<u64, ExperienceMemory>,
    index_by_task: HashMap<u64, Vec<u64>>,
    index_by_type: HashMap<String, Vec<u64>>,
    index_by_category: Option<HashMap<String, Vec<u64>>>,
    next_id: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct CoreMemoryStoreData {
    memories: HashMap<u64, CoreMemory>,
    next_id: u64,
}

// ========== Pipeline Interface ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ExperienceMemoryInput {
    /// Store a new experience (§35-36)
    Store {
        task_id: Option<u64>,
        experience_type: String,
        summary: String,
        detailed_record: Option<DetailedRecord>,
        emotional_context: Option<EmotionalContext>,
        significance: f32,
        categories: Option<Vec<String>>,
        tags: Option<Vec<String>>,
        user_id: Option<u64>,
    },
    /// Retrieve experiences by search query (§38)
    Retrieve {
        query: String,
        limit: Option<u32>,
        retrieval_mode: Option<String>,
    },
    /// Get experiences for a specific task
    GetByTask { task_id: u64 },
    /// Get recent experiences
    GetRecent { limit: Option<u32> },
    /// Get significant experiences above threshold
    GetSignificant { threshold: f32, limit: Option<u32> },
    /// Get experiences by category
    GetByCategory { category: String, limit: Option<u32> },
    /// Get all core memories (§37)
    GetCoreMemories,
    /// Form a core memory explicitly
    FormCoreMemory {
        memory_type: String,
        title: String,
        content: String,
        source_experience_ids: Vec<u64>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceMemoryOutput {
    pub success: bool,
    pub experience_id: Option<u64>,
    pub experiences: Option<Vec<ExperienceMemory>>,
    pub core_memories: Option<Vec<CoreMemory>>,
    pub memory_id: Option<u64>,
    pub was_significant: Option<bool>,
    pub error: Option<String>,
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub async fn execute(input: ExperienceMemoryInput) -> Result<ExperienceMemoryOutput, String> {
    match input {
        ExperienceMemoryInput::Store {
            task_id,
            experience_type,
            summary,
            detailed_record,
            emotional_context,
            significance,
            categories,
            tags,
            user_id,
        } => {
            let emotional_ctx = emotional_context.unwrap_or_default();
            
            let experience = ExperienceMemory {
                experience_id: 0, // Will be set by store
                timestamp: now(),
                experience_type,
                summary,
                detailed_record: detailed_record.unwrap_or_default(),
                emotional_state_during: emotional_ctx.clone(),
                emotional_state_after: emotional_ctx,
                emotional_significance: significance,
                task_id,
                user_id,
                relationship_id: None,
                lessons_learned: Vec::new(),
                patterns_identified: Vec::new(),
                vividness: 0.7,
                accessibility: 1.0,
                consolidation_status: "recent".to_string(),
                retrieval_count: 0,
                last_retrieved: None,
                categories: categories.unwrap_or_default(),
                tags: tags.unwrap_or_default(),
                core_memory_id: None,
                contributes_to_identity: false,
            };
            
            let mut store = EXPERIENCE_STORE.lock().unwrap();
            let result = store.store_experience(experience);
            
            Ok(ExperienceMemoryOutput {
                success: true,
                experience_id: result,
                experiences: None,
                core_memories: None,
                memory_id: None,
                was_significant: Some(result.is_some()),
                error: None,
            })
        }
        
        ExperienceMemoryInput::Retrieve { query, limit, retrieval_mode: _ } => {
            let mut store = EXPERIENCE_STORE.lock().unwrap();
            let results = store.search(&query, limit.unwrap_or(10) as usize);
            let ids: Vec<_> = results.iter().map(|e| e.experience_id).collect();
            let experiences: Vec<_> = results.into_iter().cloned().collect();
            
            // Mark as retrieved
            store.mark_retrieved(&ids);
            
            Ok(ExperienceMemoryOutput {
                success: true,
                experience_id: None,
                experiences: Some(experiences),
                core_memories: None,
                memory_id: None,
                was_significant: None,
                error: None,
            })
        }
        
        ExperienceMemoryInput::GetByTask { task_id } => {
            let store = EXPERIENCE_STORE.lock().unwrap();
            let experiences: Vec<_> = store.get_by_task(task_id).into_iter().cloned().collect();
            
            Ok(ExperienceMemoryOutput {
                success: true,
                experience_id: None,
                experiences: Some(experiences),
                core_memories: None,
                memory_id: None,
                was_significant: None,
                error: None,
            })
        }
        
        ExperienceMemoryInput::GetRecent { limit } => {
            let store = EXPERIENCE_STORE.lock().unwrap();
            let experiences: Vec<_> = store.get_recent(limit.unwrap_or(10) as usize)
                .into_iter().cloned().collect();
            
            Ok(ExperienceMemoryOutput {
                success: true,
                experience_id: None,
                experiences: Some(experiences),
                core_memories: None,
                memory_id: None,
                was_significant: None,
                error: None,
            })
        }
        
        ExperienceMemoryInput::GetSignificant { threshold, limit } => {
            let store = EXPERIENCE_STORE.lock().unwrap();
            let experiences: Vec<_> = store.get_significant(threshold, limit.unwrap_or(10) as usize)
                .into_iter().cloned().collect();
            
            Ok(ExperienceMemoryOutput {
                success: true,
                experience_id: None,
                experiences: Some(experiences),
                core_memories: None,
                memory_id: None,
                was_significant: None,
                error: None,
            })
        }
        
        ExperienceMemoryInput::GetByCategory { category, limit } => {
            let store = EXPERIENCE_STORE.lock().unwrap();
            let experiences: Vec<_> = store.get_by_category(&category, limit.unwrap_or(10) as usize)
                .into_iter().cloned().collect();
            
            Ok(ExperienceMemoryOutput {
                success: true,
                experience_id: None,
                experiences: Some(experiences),
                core_memories: None,
                memory_id: None,
                was_significant: None,
                error: None,
            })
        }
        
        ExperienceMemoryInput::GetCoreMemories => {
            let store = EXPERIENCE_STORE.lock().unwrap();
            let memories: Vec<_> = store.get_core_memories().into_iter().cloned().collect();
            
            Ok(ExperienceMemoryOutput {
                success: true,
                experience_id: None,
                experiences: None,
                core_memories: Some(memories),
                memory_id: None,
                was_significant: None,
                error: None,
            })
        }
        
        ExperienceMemoryInput::FormCoreMemory { memory_type, title, content, source_experience_ids } => {
            let mut store = EXPERIENCE_STORE.lock().unwrap();
            
            let memory = CoreMemory {
                memory_id: store.next_memory_id,
                created_at: now(),
                last_accessed: now(),
                memory_type,
                title,
                content,
                source_experiences: source_experience_ids,
                importance_score: 0.8,
                identity_weight: 0.6,
            };
            
            let id = memory.memory_id;
            store.core_memories.insert(id, memory);
            store.next_memory_id += 1;
            store.save_to_disk();
            
            Ok(ExperienceMemoryOutput {
                success: true,
                experience_id: None,
                experiences: None,
                core_memories: None,
                memory_id: Some(id),
                was_significant: None,
                error: None,
            })
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() {
        if args[i] == "--input" && i + 1 < args.len() {
            input_json = args[i + 1].clone();
        }
    }
    
    let input: ExperienceMemoryInput = serde_json::from_str(&input_json).unwrap_or_else(|e| {
        eprintln!("Parse error: {}", e);
        std::process::exit(1);
    });
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => {
            println!("{}", serde_json::json!({"success": false, "error": e}));
            std::process::exit(1);
        }
    }
}
