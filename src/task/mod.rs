//! Task management system
//!
//! Based on Sections 11-12 of the specification.
//!
//! All pipeline executions are tracked as tasks. Tasks provide:
//! - Execution state tracking
//! - Progress monitoring
//! - Error handling
//! - History and logging

use crate::config::TaskConfig;
use crate::types::{
    TaskID, PipelineID, UserID, DeviceID, OzoneError, OzoneResult,
    Task, TaskStatus, TaskExecutionState, LogEntry, LogLevel,
    TaskInput, TaskOutput, ResourceUsage,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Task manager - tracks all task executions
pub struct TaskManager {
    /// Configuration
    config: TaskConfig,
    
    /// Active tasks
    active_tasks: Arc<RwLock<HashMap<TaskID, Task>>>,
    
    /// Task execution states
    execution_states: Arc<RwLock<HashMap<TaskID, TaskExecutionState>>>,
    
    /// Completed task history (limited by config)
    completed_history: Arc<RwLock<Vec<Task>>>,
    
    /// Next task ID
    next_id: Arc<RwLock<TaskID>>,
    
    /// Task queue (waiting to execute)
    queue: Arc<RwLock<Vec<TaskID>>>,
}

impl TaskManager {
    /// Create new task manager
    pub fn new(config: &TaskConfig) -> OzoneResult<Self> {
        Ok(Self {
            config: config.clone(),
            active_tasks: Arc::new(RwLock::new(HashMap::new())),
            execution_states: Arc::new(RwLock::new(HashMap::new())),
            completed_history: Arc::new(RwLock::new(Vec::new())),
            next_id: Arc::new(RwLock::new(1)),
            queue: Arc::new(RwLock::new(Vec::new())),
        })
    }
    
    /// Create a new task
    pub async fn create_task(
        &self,
        pipeline_id: PipelineID,
        user_id: UserID,
        device_id: DeviceID,
    ) -> OzoneResult<TaskID> {
        // Check queue limit
        let queue = self.queue.read().await;
        if queue.len() >= self.config.max_queued_tasks {
            return Err(OzoneError::TaskError("Task queue is full".into()));
        }
        drop(queue);
        
        // Allocate task ID
        let mut next_id = self.next_id.write().await;
        let task_id = *next_id;
        *next_id += 1;
        drop(next_id);
        
        // Create task
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let task = Task {
            task_id,
            pipeline_used: pipeline_id,
            user_id,
            device_id,
            workspace_id: None,
            project_id: None,
            status: TaskStatus::Queued,
            created_at: now,
            started_at: None,
            completed_at: None,
            parent_task_id: None,
            child_tasks: Vec::new(),
            pipeline_container: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            task_context_id: None,
            execution_state_id: None,
            logs: Vec::new(),
            error: None,
            progress: 0.0,
            resource_usage: ResourceUsage::default(),
            
            consciousness_observed: false,
            
            consciousness_intervened: false,
            
            intervention_type: None,
        };
        
        // Store task
        self.active_tasks.write().await.insert(task_id, task);
        
        // Create execution state
        let exec_state = TaskExecutionState {
            execution_state_id: task_id, // Use task_id as execution state id for now
            task_id,
            blueprint_id: 0,
            current_step: 0,
            total_steps: 0,
            step_states: Vec::new(),
            step_inputs: std::collections::HashMap::new(),
            step_outputs: std::collections::HashMap::new(),
            intermediate_results: Vec::new(),
            execution_path: Vec::new(),
            skipped_steps: Vec::new(),
            started_at: now,
            last_updated: now,
            status: crate::types::task::ExecutionStatus::Running,
            preserve_for_learning: false,
            drop_on_completion: true,
        };
        self.execution_states.write().await.insert(task_id, exec_state);
        
        // Add to queue
        self.queue.write().await.push(task_id);
        
        tracing::info!("Created task {} for pipeline {}", task_id, pipeline_id);
        
        Ok(task_id)
    }
    
    /// Start a task (move from queued to running)
    pub async fn start_task(&self, task_id: TaskID) -> OzoneResult<()> {
        let mut tasks = self.active_tasks.write().await;
        let task = tasks.get_mut(&task_id)
            .ok_or_else(|| OzoneError::NotFound(format!("Task {} not found", task_id)))?;
        
        if task.status != TaskStatus::Queued {
            return Err(OzoneError::TaskError(
                format!("Task {} is not in queued state", task_id)
            ));
        }
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        task.status = TaskStatus::Running;
        task.started_at = Some(now);
        
        // Remove from queue
        let mut queue = self.queue.write().await;
        queue.retain(|&id| id != task_id);
        
        tracing::info!("Started task {}", task_id);
        
        Ok(())
    }
    
    /// Complete a task successfully
    pub async fn complete_task(&self, task_id: TaskID) -> OzoneResult<()> {
        let mut tasks = self.active_tasks.write().await;
        let task = tasks.remove(&task_id)
            .ok_or_else(|| OzoneError::NotFound(format!("Task {} not found", task_id)))?;
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let mut completed_task = task;
        completed_task.status = TaskStatus::Completed;
        completed_task.completed_at = Some(now);
        
        // Remove execution state
        self.execution_states.write().await.remove(&task_id);
        
        // Add to history if configured
        if self.config.preserve_completed_tasks {
            let mut history = self.completed_history.write().await;
            history.push(completed_task);
            
            // Trim history if needed
            while history.len() > self.config.max_task_history {
                history.remove(0);
            }
        }
        
        tracing::info!("Completed task {}", task_id);
        
        Ok(())
    }
    
    /// Fail a task
    pub async fn fail_task(&self, task_id: TaskID, error: String) -> OzoneResult<()> {
        let mut tasks = self.active_tasks.write().await;
        let task = tasks.get_mut(&task_id)
            .ok_or_else(|| OzoneError::NotFound(format!("Task {} not found", task_id)))?;
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        task.status = TaskStatus::Failed;
        task.completed_at = Some(now);
        task.error = Some(crate::types::task::TaskError {
            error_type: "PipelineError".into(),
            message: error.clone(),
            stack_trace: None,
            recoverable: false,
        });
        
        tracing::error!("Task {} failed: {}", task_id, error);
        
        Ok(())
    }
    
    /// Get task by ID
    pub async fn get_task(&self, task_id: TaskID) -> Option<Task> {
        // Check active tasks first
        if let Some(task) = self.active_tasks.read().await.get(&task_id) {
            return Some(task.clone());
        }
        
        // Check history
        self.completed_history.read().await
            .iter()
            .find(|t| t.task_id == task_id)
            .cloned()
    }
    
    /// Get task execution state
    pub async fn get_execution_state(&self, task_id: TaskID) -> Option<TaskExecutionState> {
        self.execution_states.read().await.get(&task_id).cloned()
    }
    
    /// Update task progress
    pub async fn update_progress(
        &self,
        task_id: TaskID,
        current_step: u32,
        total_steps: u32,
    ) -> OzoneResult<()> {
        let mut states = self.execution_states.write().await;
        let state = states.get_mut(&task_id)
            .ok_or_else(|| OzoneError::NotFound(format!("Task {} not found", task_id)))?;
        
        state.current_step = current_step;
        state.total_steps = total_steps;
        
        Ok(())
    }
    
    /// Add log entry to task
    pub async fn add_log(&self, task_id: TaskID, log: String) -> OzoneResult<()> {
        let mut tasks = self.active_tasks.write().await;
        let task = tasks.get_mut(&task_id)
            .ok_or_else(|| OzoneError::NotFound(format!("Task {} not found", task_id)))?;
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        task.logs.push(LogEntry {
            timestamp: now,
            level: LogLevel::Info,
            message: log,
            metadata: std::collections::HashMap::new(),
        });
        
        Ok(())
    }
    
    /// List active tasks
    pub async fn list_active(&self) -> Vec<Task> {
        self.active_tasks.read().await.values().cloned().collect()
    }
    
    /// List queued tasks
    pub async fn list_queued(&self) -> Vec<TaskID> {
        self.queue.read().await.clone()
    }
    
    /// List task history
    pub async fn list_history(&self, limit: usize) -> Vec<Task> {
        let history = self.completed_history.read().await;
        history.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }
    
    /// Cancel a task
    pub async fn cancel_task(&self, task_id: TaskID) -> OzoneResult<()> {
        let mut tasks = self.active_tasks.write().await;
        let task = tasks.get_mut(&task_id)
            .ok_or_else(|| OzoneError::NotFound(format!("Task {} not found", task_id)))?;
        
        if task.status == TaskStatus::Completed || task.status == TaskStatus::Failed {
            return Err(OzoneError::TaskError(
                format!("Task {} is already finished", task_id)
            ));
        }
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        task.status = TaskStatus::Cancelled;
        task.completed_at = Some(now);
        
        // Remove from queue if queued
        let mut queue = self.queue.write().await;
        queue.retain(|&id| id != task_id);
        
        tracing::info!("Cancelled task {}", task_id);
        
        Ok(())
    }
    
    /// List tasks with optional filtering
    pub async fn list_tasks(
        &self,
        status_filter: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Vec<Task> {
        let active = self.active_tasks.read().await;
        let history = self.completed_history.read().await;
        
        let mut all_tasks: Vec<Task> = active.values().cloned().collect();
        all_tasks.extend(history.iter().cloned());
        
        // Filter by status if specified
        if let Some(status) = status_filter {
            all_tasks.retain(|t| format!("{:?}", t.status).to_lowercase() == status.to_lowercase());
        }
        
        // Sort by created_at descending
        all_tasks.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        // Apply pagination
        all_tasks.into_iter()
            .skip(offset)
            .take(limit)
            .collect()
    }
    
    /// Get count of active tasks
    pub fn active_count(&self) -> usize {
        // Use try_read to avoid blocking
        self.active_tasks.try_read()
            .map(|tasks| tasks.len())
            .unwrap_or(0)
    }
}
