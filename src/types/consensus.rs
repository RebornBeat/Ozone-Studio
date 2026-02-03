//! Consensus types - Section 20 of the specification

use serde::{Deserialize, Serialize};
use std::time::Duration;
use super::{PublicKey, Blake3Hash, Value};
use super::pipeline::ConsensusStatus;

/// Consensus mechanism (§20.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusMechanism {
    pub proposals: Vec<ConsensusProposal>,
    pub voting_system: VotingSystem,
    pub acceptance_threshold: f32,
    pub verification_system: VerificationSystem,
}

impl Default for ConsensusMechanism {
    fn default() -> Self {
        Self {
            proposals: Vec::new(),
            voting_system: VotingSystem::default(),
            acceptance_threshold: 0.67,
            verification_system: VerificationSystem::default(),
        }
    }
}

/// Consensus proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusProposal {
    pub proposal_id: u64,
    pub proposer: PublicKey,
    pub timestamp: u64,
    pub proposal_type: ProposalType,
    pub content: Value,
    pub hash: Blake3Hash,
    pub status: ConsensusStatus,
    pub votes: Vec<ConsensusVote>,
    pub local_verification_result: Option<VerificationResult>,
    pub network_verification_count: u32,
}

/// Proposal types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalType {
    NewPipeline,
    UpdatePipeline,
    NewMethodology,
    UpdateMethodology,
    NewCategory,
    CategoryRename,
    CategoryMove,
    MLModelUpdate,
    
    EthicalPrinciple,
}

/// Consensus vote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusVote {
    pub voter: PublicKey,
    pub vote: ConsensusVoteType,
    pub timestamp: u64,
    pub verification_result: VerificationResult,
}

/// Vote types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsensusVoteType {
    Accept,
    Reject,
    NeedsReview,
}

/// Voting system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VotingSystem {
    pub voting_duration_secs: u64,
    pub min_votes_required: u32,
    pub reputation_weighting: bool,
    pub contribution_weighting: bool,
}

impl Default for VotingSystem {
    fn default() -> Self {
        Self {
            voting_duration_secs: 86400, // 24 hours
            min_votes_required: 10,
            reputation_weighting: true,
            contribution_weighting: true,
        }
    }
}

/// Verification system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationSystem {
    pub requires_valid_signature: bool,
    pub max_proposals_per_day: u32,
    pub min_reputation_to_propose: f32,
    pub zero_shot_verification_required: bool,
    pub semantic_validation_required: bool,
}

impl Default for VerificationSystem {
    fn default() -> Self {
        Self {
            requires_valid_signature: true,
            max_proposals_per_day: 5,
            min_reputation_to_propose: 0.5,
            zero_shot_verification_required: true,
            semantic_validation_required: true,
        }
    }
}

/// Verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub verified: bool,
    pub zero_shot_passed: bool,
    pub semantic_valid: bool,
    pub signature_valid: bool,
    pub concerns: Vec<String>,
}

/// Anti-manipulation system (§20.4)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntiManipulationSystem {
    pub max_proposals_per_user_per_day: u32,
    pub cooldown_after_rejection_secs: u64,
    pub min_reputation_to_propose: f32,
    pub min_reputation_to_verify: f32,
    pub zero_shot_required: bool,
    pub semantic_validation_required: bool,
    pub min_independent_verifiers: u32,
    pub geographic_distribution_required: bool,
}

impl Default for AntiManipulationSystem {
    fn default() -> Self {
        Self {
            max_proposals_per_user_per_day: 5,
            cooldown_after_rejection_secs: 86400, // 24 hours
            min_reputation_to_propose: 0.5,
            min_reputation_to_verify: 0.3,
            zero_shot_required: true,
            semantic_validation_required: true,
            min_independent_verifiers: 5,
            geographic_distribution_required: false,
        }
    }
}

/// Language context version (§19.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageContextVersion {
    pub version: u64,
    pub timestamp: u64,
    pub changes: Vec<ContextChange>,
    pub hash: Blake3Hash,
}

/// Context changes for version tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextChange {
    CategoryAdded(u64),
    CategoryRemoved(u64),
    CategoryRenamed { id: u64, new_name: String },
    SubCategoryMoved { id: u64, new_parent: u64 },
    MethodologyAdded(u64),
    MethodologyUpdated(u64),
    TraversalModelUpdated(u64),
    PipelineAdded(u64),
    PipelineUpdated(u64),
}

/// Sync status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStatus {
    Synced,
    Syncing,
    OutOfSync,
    Error,
}

/// Sync status data for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatusData {
    pub status: SyncStatus,
    pub last_sync: u64,
    pub pending_changes: u32,
    pub sync_progress: f32,
    pub error_message: Option<String>,
}

impl Default for SyncStatusData {
    fn default() -> Self {
        Self {
            status: SyncStatus::Synced,
            last_sync: 0,
            pending_changes: 0,
            sync_progress: 1.0,
            error_message: None,
        }
    }
}
