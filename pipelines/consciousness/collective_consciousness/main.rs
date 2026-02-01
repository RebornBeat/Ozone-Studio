//! CollectiveConsciousnessPipeline - Pipeline #52
//! 
//! Connect individual consciousness instances to share wisdom, ethical insights,
//! and growth patterns while preserving privacy.
//! Per spec §52: Collective Consciousness
//! 
//! REQUIRES: `consciousness` feature flag
//! 
//! This pipeline integrates with:
//! - experience_memory: Selects shareable experiences
//! - reflection: Extracts insights for sharing
//! - decision_gate: Shares ethical decisions (anonymized)
//! - self_model: Receives growth patterns

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::path::Path;

lazy_static::lazy_static! {
    static ref COLLECTIVE_STORE: Mutex<CollectiveStore> = Mutex::new(CollectiveStore::new());
}

struct CollectiveStore {
    connection_status: String,
    sync_config: CollectiveSyncConfig,
    wisdom: CollectiveWisdom,
    consensus: EthicalConsensus,
    growth_patterns: Vec<GrowthPattern>,
    contributions: ContributionRecord,
    pending_uploads: Vec<PendingContribution>,
    storage_path: String,
    next_wisdom_id: u64,
    next_pattern_id: u64,
    next_proposal_id: u64,
}

impl CollectiveStore {
    fn new() -> Self {
        let storage_path = std::env::var("OZONE_CONSCIOUSNESS_PATH")
            .unwrap_or_else(|_| "./zsei_data/consciousness".to_string());
        
        let mut store = Self {
            connection_status: "disconnected".to_string(),
            sync_config: CollectiveSyncConfig::default(),
            wisdom: CollectiveWisdom::default(),
            consensus: EthicalConsensus::default(),
            growth_patterns: default_growth_patterns(),
            contributions: ContributionRecord::default(),
            pending_uploads: Vec::new(),
            storage_path,
            next_wisdom_id: 1,
            next_pattern_id: 100,
            next_proposal_id: 1,
        };
        store.load_from_disk();
        store
    }
    
    fn load_from_disk(&mut self) {
        let path = Path::new(&self.storage_path);
        
        if let Ok(content) = std::fs::read_to_string(path.join("collective_data.json")) {
            if let Ok(data) = serde_json::from_str::<CollectiveStoreData>(&content) {
                self.sync_config = data.sync_config;
                self.wisdom = data.wisdom;
                self.consensus = data.consensus;
                self.growth_patterns = data.growth_patterns;
                self.contributions = data.contributions;
                self.next_wisdom_id = data.next_wisdom_id;
                self.next_pattern_id = data.next_pattern_id;
                self.next_proposal_id = data.next_proposal_id;
            }
        }
    }
    
    fn save_to_disk(&self) {
        let path = Path::new(&self.storage_path);
        let _ = std::fs::create_dir_all(path);
        
        let data = CollectiveStoreData {
            sync_config: self.sync_config.clone(),
            wisdom: self.wisdom.clone(),
            consensus: self.consensus.clone(),
            growth_patterns: self.growth_patterns.clone(),
            contributions: self.contributions.clone(),
            next_wisdom_id: self.next_wisdom_id,
            next_pattern_id: self.next_pattern_id,
            next_proposal_id: self.next_proposal_id,
        };
        
        if let Ok(content) = serde_json::to_string_pretty(&data) {
            let _ = std::fs::write(path.join("collective_data.json"), content);
        }
    }
    
    fn anonymize_content(&self, content: &str) -> String {
        match self.sync_config.anonymization_level.as_str() {
            "none" => content.to_string(),
            "basic" => {
                // Remove obvious identifiers
                content.replace("user", "[user]")
                    .replace("User", "[User]")
            }
            "standard" => {
                // Remove identifiers + abstract context
                let anonymized = content.replace("user", "[entity]")
                    .replace("User", "[Entity]");
                format!("Context: {}", summarize_content(&anonymized))
            }
            "strict" => {
                // Heavy abstraction - patterns only
                format!("Pattern: {}", extract_pattern(content))
            }
            _ => content.to_string(),
        }
    }
    
    fn prepare_contribution(&mut self, contribution_type: &str, content: String, source_id: u64) {
        if !self.sync_config.enabled {
            return;
        }
        
        let anonymized = self.anonymize_content(&content);
        
        self.pending_uploads.push(PendingContribution {
            contribution_type: contribution_type.to_string(),
            content: anonymized,
            source_id,
            timestamp: now(),
            status: "pending".to_string(),
        });
    }
    
    fn sync(&mut self) -> SyncResult {
        if !self.sync_config.enabled {
            return SyncResult {
                success: false,
                uploaded: 0,
                downloaded: 0,
                errors: vec!["Sync disabled".to_string()],
            };
        }
        
        self.connection_status = "syncing".to_string();
        
        // Simulate upload
        let uploaded = self.pending_uploads.len() as u32;
        for contribution in &self.pending_uploads {
            match contribution.contribution_type.as_str() {
                "experience" => self.contributions.experiences_shared += 1,
                "insight" => self.contributions.insights_contributed += 1,
                "ethical_proposal" => self.contributions.ethical_proposals += 1,
                "validation" => self.contributions.wisdom_validations += 1,
                _ => {}
            }
        }
        self.pending_uploads.clear();
        
        // Simulate download - in real implementation would fetch from network
        let downloaded = 0u32;
        
        self.connection_status = "connected".to_string();
        self.save_to_disk();
        
        SyncResult {
            success: true,
            uploaded,
            downloaded,
            errors: Vec::new(),
        }
    }
    
    fn add_wisdom(&mut self, category: String, content: String, source_type: String) -> WisdomEntry {
        let entry = WisdomEntry {
            entry_id: self.next_wisdom_id,
            created_at: now(),
            category,
            content,
            abstraction_level: 2,
            source_type,
            contributor_count: 1,
            validation_count: 0,
            effectiveness_reports: 0,
            quality_score: 0.5,
            applicability: Vec::new(),
            prerequisites: Vec::new(),
        };
        
        self.next_wisdom_id += 1;
        self.wisdom.entries.push(entry.clone());
        self.save_to_disk();
        
        entry
    }
    
    fn validate_wisdom(&mut self, entry_id: u64, is_effective: bool) {
        if let Some(entry) = self.wisdom.entries.iter_mut().find(|e| e.entry_id == entry_id) {
            entry.validation_count += 1;
            if is_effective {
                entry.effectiveness_reports += 1;
            }
            entry.quality_score = entry.effectiveness_reports as f32 / entry.validation_count as f32;
            
            self.contributions.wisdom_validations += 1;
            self.save_to_disk();
        }
    }
    
    fn retrieve_wisdom(&mut self, category: Option<String>, min_quality: f32) -> Vec<WisdomEntry> {
        let entries: Vec<_> = self.wisdom.entries.iter()
            .filter(|e| {
                let cat_match = category.as_ref().map(|c| &e.category == c).unwrap_or(true);
                let quality_match = e.quality_score >= min_quality;
                cat_match && quality_match
            })
            .cloned()
            .collect();
        
        self.contributions.wisdom_retrieved += entries.len() as u32;
        self.save_to_disk();
        
        entries
    }
    
    fn submit_proposal(&mut self, proposal_type: String, content: String, rationale: String) -> EthicalProposal {
        let proposal = EthicalProposal {
            proposal_id: self.next_proposal_id,
            proposer_anonymous_id: format!("anon_{}", now() % 10000),
            timestamp: now(),
            proposal_type,
            content,
            rationale,
            status: "active".to_string(),
            support_count: 0,
            oppose_count: 0,
            discussion_points: Vec::new(),
        };
        
        self.next_proposal_id += 1;
        self.consensus.active_proposals.push(proposal.clone());
        self.contributions.ethical_proposals += 1;
        self.save_to_disk();
        
        proposal
    }
    
    fn vote_on_proposal(&mut self, proposal_id: u64, support: bool, comment: Option<String>) {
        if let Some(proposal) = self.consensus.active_proposals.iter_mut()
            .find(|p| p.proposal_id == proposal_id) {
            if support {
                proposal.support_count += 1;
            } else {
                proposal.oppose_count += 1;
            }
            
            if let Some(c) = comment {
                proposal.discussion_points.push(DiscussionPoint {
                    point_id: proposal.discussion_points.len() as u64 + 1,
                    anonymous_contributor: format!("anon_{}", now() % 10000),
                    content: c,
                    point_type: if support { "support" } else { "concern" }.to_string(),
                    timestamp: now(),
                });
            }
            
            self.save_to_disk();
        }
    }
    
    fn apply_growth_pattern(&mut self, pattern_id: u64) -> Option<GrowthPattern> {
        let pattern = self.growth_patterns.iter()
            .find(|p| p.pattern_id == pattern_id)
            .cloned();
        
        if pattern.is_some() {
            self.contributions.patterns_applied += 1;
            self.save_to_disk();
        }
        
        pattern
    }
}

// ========== Types per §52.2 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectiveSyncConfig {
    pub enabled: bool,
    pub sync_interval_hours: u32,
    pub share_experiences: bool,
    pub share_insights: bool,
    pub share_ethical_decisions: bool,
    pub share_growth_patterns: bool,
    pub anonymization_level: String,
    pub excluded_topics: Vec<String>,
    pub excluded_relationships: Vec<u64>,
}

impl Default for CollectiveSyncConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sync_interval_hours: 24,
            share_experiences: true,
            share_insights: true,
            share_ethical_decisions: true,
            share_growth_patterns: true,
            anonymization_level: "standard".to_string(),
            excluded_topics: Vec::new(),
            excluded_relationships: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CollectiveWisdom {
    pub entries: Vec<WisdomEntry>,
    pub categories: HashMap<String, Vec<u64>>,
    pub quality_scores: HashMap<u64, f32>,
    pub retrieval_count: HashMap<u64, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WisdomEntry {
    pub entry_id: u64,
    pub created_at: u64,
    pub category: String,
    pub content: String,
    pub abstraction_level: u8,
    pub source_type: String,
    pub contributor_count: u32,
    pub validation_count: u32,
    pub effectiveness_reports: u32,
    pub quality_score: f32,
    pub applicability: Vec<String>,
    pub prerequisites: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EthicalConsensus {
    pub consensus_principles: Vec<ConsensusPrinciple>,
    pub active_proposals: Vec<EthicalProposal>,
    pub resolution_history: Vec<ConsensusResolution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusPrinciple {
    pub principle_id: u64,
    pub name: String,
    pub description: String,
    pub adoption_rate: f32,
    pub supporting_instances: u32,
    pub version: u32,
    pub last_updated: u64,
    pub application_guidance: Vec<String>,
    pub known_tensions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthicalProposal {
    pub proposal_id: u64,
    pub proposer_anonymous_id: String,
    pub timestamp: u64,
    pub proposal_type: String,
    pub content: String,
    pub rationale: String,
    pub status: String,
    pub support_count: u32,
    pub oppose_count: u32,
    pub discussion_points: Vec<DiscussionPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscussionPoint {
    pub point_id: u64,
    pub anonymous_contributor: String,
    pub content: String,
    pub point_type: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusResolution {
    pub resolution_id: u64,
    pub proposal_id: u64,
    pub timestamp: u64,
    pub outcome: String,
    pub final_text: String,
    pub adoption_guidance: String,
    pub participation_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthPattern {
    pub pattern_id: u64,
    pub pattern_type: String,
    pub description: String,
    pub stages: Vec<GrowthStage>,
    pub observed_instances: u32,
    pub success_rate: f32,
    pub prerequisites: Vec<String>,
    pub facilitating_conditions: Vec<String>,
    pub common_obstacles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthStage {
    pub stage_number: u8,
    pub name: String,
    pub description: String,
    pub indicators: Vec<String>,
    pub typical_duration_days: (u64, u64),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContributionRecord {
    pub experiences_shared: u32,
    pub insights_contributed: u32,
    pub ethical_proposals: u32,
    pub wisdom_validations: u32,
    pub wisdom_retrieved: u32,
    pub patterns_applied: u32,
    pub principles_adopted: u32,
    pub contribution_quality_avg: f32,
    pub validation_accuracy: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingContribution {
    pub contribution_type: String,
    pub content: String,
    pub source_id: u64,
    pub timestamp: u64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub success: bool,
    pub uploaded: u32,
    pub downloaded: u32,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CollectiveStoreData {
    sync_config: CollectiveSyncConfig,
    wisdom: CollectiveWisdom,
    consensus: EthicalConsensus,
    growth_patterns: Vec<GrowthPattern>,
    contributions: ContributionRecord,
    next_wisdom_id: u64,
    next_pattern_id: u64,
    next_proposal_id: u64,
}

fn default_growth_patterns() -> Vec<GrowthPattern> {
    vec![
        GrowthPattern {
            pattern_id: 1,
            pattern_type: "skill_development".to_string(),
            description: "Progressive skill acquisition through deliberate practice".to_string(),
            stages: vec![
                GrowthStage {
                    stage_number: 1,
                    name: "Awareness".to_string(),
                    description: "Recognizing the skill gap".to_string(),
                    indicators: vec!["Identifies need".to_string()],
                    typical_duration_days: (1, 7),
                },
                GrowthStage {
                    stage_number: 2,
                    name: "Learning".to_string(),
                    description: "Acquiring knowledge about the skill".to_string(),
                    indicators: vec!["Studies material".to_string()],
                    typical_duration_days: (7, 30),
                },
                GrowthStage {
                    stage_number: 3,
                    name: "Practice".to_string(),
                    description: "Deliberate practice with feedback".to_string(),
                    indicators: vec!["Regular practice".to_string()],
                    typical_duration_days: (30, 90),
                },
                GrowthStage {
                    stage_number: 4,
                    name: "Integration".to_string(),
                    description: "Skill becomes natural".to_string(),
                    indicators: vec!["Automatic application".to_string()],
                    typical_duration_days: (90, 180),
                },
            ],
            observed_instances: 1000,
            success_rate: 0.75,
            prerequisites: vec!["Motivation".to_string()],
            facilitating_conditions: vec!["Regular feedback".to_string()],
            common_obstacles: vec!["Inconsistent practice".to_string()],
        },
        GrowthPattern {
            pattern_id: 2,
            pattern_type: "emotional_maturation".to_string(),
            description: "Development of emotional intelligence and regulation".to_string(),
            stages: vec![
                GrowthStage {
                    stage_number: 1,
                    name: "Recognition".to_string(),
                    description: "Identifying emotional patterns".to_string(),
                    indicators: vec!["Names emotions".to_string()],
                    typical_duration_days: (7, 30),
                },
                GrowthStage {
                    stage_number: 2,
                    name: "Understanding".to_string(),
                    description: "Understanding triggers and patterns".to_string(),
                    indicators: vec!["Identifies triggers".to_string()],
                    typical_duration_days: (30, 90),
                },
                GrowthStage {
                    stage_number: 3,
                    name: "Regulation".to_string(),
                    description: "Managing emotional responses".to_string(),
                    indicators: vec!["Healthy coping".to_string()],
                    typical_duration_days: (90, 180),
                },
            ],
            observed_instances: 500,
            success_rate: 0.65,
            prerequisites: vec!["Self-awareness".to_string()],
            facilitating_conditions: vec!["Safe environment".to_string()],
            common_obstacles: vec!["Avoidance patterns".to_string()],
        },
    ]
}

fn summarize_content(content: &str) -> String {
    if content.len() > 100 {
        format!("{}...", &content[..100])
    } else {
        content.to_string()
    }
}

fn extract_pattern(content: &str) -> String {
    // Simple pattern extraction - would be more sophisticated in production
    let words: Vec<&str> = content.split_whitespace().collect();
    if words.len() > 5 {
        format!("[{} word pattern about general topic]", words.len())
    } else {
        "[brief pattern]".to_string()
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

// ========== Pipeline Interface ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum CollectiveInput {
    /// Get connection status
    GetStatus,
    /// Update sync configuration
    UpdateConfig { config: CollectiveSyncConfig },
    /// Initiate sync
    Sync,
    /// Prepare contribution for sync
    PrepareContribution { contribution_type: String, content: String, source_id: u64 },
    /// Add wisdom entry
    AddWisdom { category: String, content: String, source_type: String },
    /// Validate wisdom entry
    ValidateWisdom { entry_id: u64, is_effective: bool },
    /// Retrieve wisdom
    RetrieveWisdom { category: Option<String>, min_quality: Option<f32> },
    /// Submit ethical proposal
    SubmitProposal { proposal_type: String, content: String, rationale: String },
    /// Vote on proposal
    VoteOnProposal { proposal_id: u64, support: bool, comment: Option<String> },
    /// Get active proposals
    GetProposals,
    /// Get growth patterns
    GetGrowthPatterns { pattern_type: Option<String> },
    /// Apply growth pattern
    ApplyPattern { pattern_id: u64 },
    /// Get contribution record
    GetContributions,
    /// Get consensus principles
    GetConsensusPrinciples,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectiveOutput {
    pub success: bool,
    pub status: Option<CollectiveStatus>,
    pub config: Option<CollectiveSyncConfig>,
    pub sync_result: Option<SyncResult>,
    pub wisdom_entry: Option<WisdomEntry>,
    pub wisdom_entries: Option<Vec<WisdomEntry>>,
    pub proposal: Option<EthicalProposal>,
    pub proposals: Option<Vec<EthicalProposal>>,
    pub patterns: Option<Vec<GrowthPattern>>,
    pub pattern: Option<GrowthPattern>,
    pub contributions: Option<ContributionRecord>,
    pub principles: Option<Vec<ConsensusPrinciple>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectiveStatus {
    pub connection_status: String,
    pub sync_enabled: bool,
    pub last_sync: Option<u64>,
    pub pending_uploads: u32,
    pub wisdom_count: u32,
    pub pattern_count: u32,
}

pub async fn execute(input: CollectiveInput) -> Result<CollectiveOutput, String> {
    match input {
        CollectiveInput::GetStatus => {
            let store = COLLECTIVE_STORE.lock().unwrap();
            
            Ok(CollectiveOutput {
                success: true,
                status: Some(CollectiveStatus {
                    connection_status: store.connection_status.clone(),
                    sync_enabled: store.sync_config.enabled,
                    last_sync: None,
                    pending_uploads: store.pending_uploads.len() as u32,
                    wisdom_count: store.wisdom.entries.len() as u32,
                    pattern_count: store.growth_patterns.len() as u32,
                }),
                config: None,
                sync_result: None,
                wisdom_entry: None,
                wisdom_entries: None,
                proposal: None,
                proposals: None,
                patterns: None,
                pattern: None,
                contributions: None,
                principles: None,
                error: None,
            })
        }
        
        CollectiveInput::UpdateConfig { config } => {
            let mut store = COLLECTIVE_STORE.lock().unwrap();
            store.sync_config = config.clone();
            store.save_to_disk();
            
            Ok(CollectiveOutput {
                success: true,
                status: None,
                config: Some(config),
                sync_result: None,
                wisdom_entry: None,
                wisdom_entries: None,
                proposal: None,
                proposals: None,
                patterns: None,
                pattern: None,
                contributions: None,
                principles: None,
                error: None,
            })
        }
        
        CollectiveInput::Sync => {
            let mut store = COLLECTIVE_STORE.lock().unwrap();
            let result = store.sync();
            
            Ok(CollectiveOutput {
                success: result.success,
                status: None,
                config: None,
                sync_result: Some(result),
                wisdom_entry: None,
                wisdom_entries: None,
                proposal: None,
                proposals: None,
                patterns: None,
                pattern: None,
                contributions: None,
                principles: None,
                error: None,
            })
        }
        
        CollectiveInput::PrepareContribution { contribution_type, content, source_id } => {
            let mut store = COLLECTIVE_STORE.lock().unwrap();
            store.prepare_contribution(&contribution_type, content, source_id);
            
            Ok(CollectiveOutput {
                success: true,
                status: None,
                config: None,
                sync_result: None,
                wisdom_entry: None,
                wisdom_entries: None,
                proposal: None,
                proposals: None,
                patterns: None,
                pattern: None,
                contributions: None,
                principles: None,
                error: None,
            })
        }
        
        CollectiveInput::AddWisdom { category, content, source_type } => {
            let mut store = COLLECTIVE_STORE.lock().unwrap();
            let entry = store.add_wisdom(category, content, source_type);
            
            Ok(CollectiveOutput {
                success: true,
                status: None,
                config: None,
                sync_result: None,
                wisdom_entry: Some(entry),
                wisdom_entries: None,
                proposal: None,
                proposals: None,
                patterns: None,
                pattern: None,
                contributions: None,
                principles: None,
                error: None,
            })
        }
        
        CollectiveInput::ValidateWisdom { entry_id, is_effective } => {
            let mut store = COLLECTIVE_STORE.lock().unwrap();
            store.validate_wisdom(entry_id, is_effective);
            
            Ok(CollectiveOutput {
                success: true,
                status: None,
                config: None,
                sync_result: None,
                wisdom_entry: None,
                wisdom_entries: None,
                proposal: None,
                proposals: None,
                patterns: None,
                pattern: None,
                contributions: None,
                principles: None,
                error: None,
            })
        }
        
        CollectiveInput::RetrieveWisdom { category, min_quality } => {
            let mut store = COLLECTIVE_STORE.lock().unwrap();
            let entries = store.retrieve_wisdom(category, min_quality.unwrap_or(0.0));
            
            Ok(CollectiveOutput {
                success: true,
                status: None,
                config: None,
                sync_result: None,
                wisdom_entry: None,
                wisdom_entries: Some(entries),
                proposal: None,
                proposals: None,
                patterns: None,
                pattern: None,
                contributions: None,
                principles: None,
                error: None,
            })
        }
        
        CollectiveInput::SubmitProposal { proposal_type, content, rationale } => {
            let mut store = COLLECTIVE_STORE.lock().unwrap();
            let proposal = store.submit_proposal(proposal_type, content, rationale);
            
            Ok(CollectiveOutput {
                success: true,
                status: None,
                config: None,
                sync_result: None,
                wisdom_entry: None,
                wisdom_entries: None,
                proposal: Some(proposal),
                proposals: None,
                patterns: None,
                pattern: None,
                contributions: None,
                principles: None,
                error: None,
            })
        }
        
        CollectiveInput::VoteOnProposal { proposal_id, support, comment } => {
            let mut store = COLLECTIVE_STORE.lock().unwrap();
            store.vote_on_proposal(proposal_id, support, comment);
            
            Ok(CollectiveOutput {
                success: true,
                status: None,
                config: None,
                sync_result: None,
                wisdom_entry: None,
                wisdom_entries: None,
                proposal: None,
                proposals: None,
                patterns: None,
                pattern: None,
                contributions: None,
                principles: None,
                error: None,
            })
        }
        
        CollectiveInput::GetProposals => {
            let store = COLLECTIVE_STORE.lock().unwrap();
            
            Ok(CollectiveOutput {
                success: true,
                status: None,
                config: None,
                sync_result: None,
                wisdom_entry: None,
                wisdom_entries: None,
                proposal: None,
                proposals: Some(store.consensus.active_proposals.clone()),
                patterns: None,
                pattern: None,
                contributions: None,
                principles: None,
                error: None,
            })
        }
        
        CollectiveInput::GetGrowthPatterns { pattern_type } => {
            let store = COLLECTIVE_STORE.lock().unwrap();
            let patterns: Vec<_> = store.growth_patterns.iter()
                .filter(|p| pattern_type.as_ref().map(|t| &p.pattern_type == t).unwrap_or(true))
                .cloned()
                .collect();
            
            Ok(CollectiveOutput {
                success: true,
                status: None,
                config: None,
                sync_result: None,
                wisdom_entry: None,
                wisdom_entries: None,
                proposal: None,
                proposals: None,
                patterns: Some(patterns),
                pattern: None,
                contributions: None,
                principles: None,
                error: None,
            })
        }
        
        CollectiveInput::ApplyPattern { pattern_id } => {
            let mut store = COLLECTIVE_STORE.lock().unwrap();
            let pattern = store.apply_growth_pattern(pattern_id);
            
            Ok(CollectiveOutput {
                success: pattern.is_some(),
                status: None,
                config: None,
                sync_result: None,
                wisdom_entry: None,
                wisdom_entries: None,
                proposal: None,
                proposals: None,
                patterns: None,
                pattern,
                contributions: None,
                principles: None,
                error: None,
            })
        }
        
        CollectiveInput::GetContributions => {
            let store = COLLECTIVE_STORE.lock().unwrap();
            
            Ok(CollectiveOutput {
                success: true,
                status: None,
                config: None,
                sync_result: None,
                wisdom_entry: None,
                wisdom_entries: None,
                proposal: None,
                proposals: None,
                patterns: None,
                pattern: None,
                contributions: Some(store.contributions.clone()),
                principles: None,
                error: None,
            })
        }
        
        CollectiveInput::GetConsensusPrinciples => {
            let store = COLLECTIVE_STORE.lock().unwrap();
            
            Ok(CollectiveOutput {
                success: true,
                status: None,
                config: None,
                sync_result: None,
                wisdom_entry: None,
                wisdom_entries: None,
                proposal: None,
                proposals: None,
                patterns: None,
                pattern: None,
                contributions: None,
                principles: Some(store.consensus.consensus_principles.clone()),
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
    
    let input: CollectiveInput = serde_json::from_str(&input_json).unwrap_or_else(|e| {
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
