/**
 * TasksTab UI Component
 * Pipeline #36 - TaskViewerPipeline
 * 
 * View task information and history.
 * Shows active, completed, failed tasks with timeline.
 * 
 * gRPC calls via:
 * - window.ozone.pipeline.execute(36, input) for task details
 * - window.ozone.task.list() for task listing
 * - window.ozone.task.get(id) for specific task
 */

import React, { useEffect, useState, useCallback } from 'react';

interface TaskData {
  task_id: number;
  pipeline_id: number;
  pipeline_name: string;
  status: 'pending' | 'queued' | 'running' | 'completed' | 'failed' | 'cancelled';
  progress: number;
  current_step?: number;
  total_steps?: number;
  created_at: number;
  started_at?: number;
  completed_at?: number;
  error?: string;
  workspace_id?: number;
  project_id?: number;
}

interface TimelineEvent {
  timestamp: number;
  event: string;
  details?: string;
}

interface TaskDetails {
  task_id: number;
  pipeline_name: string;
  status: string;
  started_at?: number;
  completed_at?: number;
  duration_secs?: number;
  input_summary: string;
  output_summary?: string;
}

type FilterType = 'all' | 'running' | 'completed' | 'failed';

export function TasksTab() {
  const [tasks, setTasks] = useState<TaskData[]>([]);
  const [filter, setFilter] = useState<FilterType>('all');
  const [selectedTaskId, setSelectedTaskId] = useState<number | null>(null);
  const [taskDetails, setTaskDetails] = useState<TaskDetails | null>(null);
  const [timeline, setTimeline] = useState<TimelineEvent[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [refreshInterval, setRefreshInterval] = useState<NodeJS.Timeout | null>(null);

  // Load tasks on mount and set up refresh
  useEffect(() => {
    loadTasks();
    
    // Refresh every 3 seconds when there are running tasks
    const interval = setInterval(() => {
      const hasRunning = tasks.some(t => t.status === 'running' || t.status === 'queued');
      if (hasRunning) {
        loadTasks();
      }
    }, 3000);
    
    setRefreshInterval(interval);
    
    return () => {
      if (interval) clearInterval(interval);
    };
  }, []);

  const executePipeline = async (pipelineId: number, input: any) => {
    if (window.ozone?.pipeline?.execute) {
      return await window.ozone.pipeline.execute(pipelineId, input);
    }
    throw new Error('Pipeline execution not available');
  };

  const loadTasks = async () => {
    try {
      // Use task:list from TaskManagerPipeline via gRPC
      if (window.ozone?.task?.list) {
        const result = await window.ozone.task.list();
        if (result?.tasks) {
          setTasks(result.tasks);
        }
      } else {
        // Fallback: call TaskManagerPipeline directly
        const result = await executePipeline(5, { action: 'List', limit: 100 });
        if (result?.tasks) {
          setTasks(result.tasks);
        }
      }
    } catch (e: any) {
      console.warn('Failed to load tasks:', e);
    }
  };

  const loadTaskDetails = async (taskId: number) => {
    setLoading(true);
    try {
      // Call TaskViewerPipeline (#36) for details
      const detailsResult = await executePipeline(36, { 
        action: 'GetDetails', 
        task_id: taskId 
      });
      if (detailsResult?.details) {
        setTaskDetails(detailsResult.details);
      }

      // Get timeline
      const timelineResult = await executePipeline(36, { 
        action: 'GetTimeline', 
        task_id: taskId 
      });
      if (timelineResult?.timeline) {
        setTimeline(timelineResult.timeline);
      }
    } catch (e: any) {
      setError(e.message || 'Failed to load task details');
    }
    setLoading(false);
  };

  const handleSelectTask = (taskId: number) => {
    if (selectedTaskId === taskId) {
      setSelectedTaskId(null);
      setTaskDetails(null);
      setTimeline([]);
    } else {
      setSelectedTaskId(taskId);
      loadTaskDetails(taskId);
    }
  };

  const handleCancelTask = async (taskId: number) => {
    try {
      await executePipeline(5, { action: 'Cancel', task_id: taskId });
      await loadTasks();
    } catch (e: any) {
      setError(e.message || 'Failed to cancel task');
    }
  };

  const handleRetryTask = async (taskId: number) => {
    try {
      await executePipeline(5, { action: 'Retry', task_id: taskId });
      await loadTasks();
    } catch (e: any) {
      setError(e.message || 'Failed to retry task');
    }
  };

  const filteredTasks = tasks.filter(task => {
    if (filter === 'all') return true;
    if (filter === 'running') return task.status === 'running' || task.status === 'queued' || task.status === 'pending';
    if (filter === 'completed') return task.status === 'completed';
    if (filter === 'failed') return task.status === 'failed' || task.status === 'cancelled';
    return true;
  });

  const formatTime = (timestamp?: number) => {
    if (!timestamp) return '—';
    return new Date(timestamp * 1000).toLocaleTimeString();
  };

  const formatDate = (timestamp?: number) => {
    if (!timestamp) return '—';
    return new Date(timestamp * 1000).toLocaleString();
  };

  const formatDuration = (secs?: number) => {
    if (!secs) return '—';
    if (secs < 60) return `${secs}s`;
    if (secs < 3600) return `${Math.floor(secs / 60)}m ${secs % 60}s`;
    return `${Math.floor(secs / 3600)}h ${Math.floor((secs % 3600) / 60)}m`;
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'running': return 'var(--color-warning)';
      case 'completed': return 'var(--color-success)';
      case 'failed': return 'var(--color-error)';
      case 'cancelled': return 'var(--color-text-muted)';
      case 'queued':
      case 'pending': return 'var(--color-accent)';
      default: return 'var(--color-text-secondary)';
    }
  };

  const taskCounts = {
    all: tasks.length,
    running: tasks.filter(t => ['running', 'queued', 'pending'].includes(t.status)).length,
    completed: tasks.filter(t => t.status === 'completed').length,
    failed: tasks.filter(t => ['failed', 'cancelled'].includes(t.status)).length,
  };

  return (
    <div className="tasks-tab">
      <div className="panel-toolbar">
        <h2>Tasks</h2>
        <div className="filter-buttons">
          <button 
            className={`filter-btn ${filter === 'all' ? 'active' : ''}`}
            onClick={() => setFilter('all')}
          >
            All ({taskCounts.all})
          </button>
          <button 
            className={`filter-btn ${filter === 'running' ? 'active' : ''}`}
            onClick={() => setFilter('running')}
          >
            Active ({taskCounts.running})
          </button>
          <button 
            className={`filter-btn ${filter === 'completed' ? 'active' : ''}`}
            onClick={() => setFilter('completed')}
          >
            Completed ({taskCounts.completed})
          </button>
          <button 
            className={`filter-btn ${filter === 'failed' ? 'active' : ''}`}
            onClick={() => setFilter('failed')}
          >
            Failed ({taskCounts.failed})
          </button>
        </div>
      </div>

      {error && (
        <div className="panel-error">
          <span className="error-icon">⚠️</span>
          <p>{error}</p>
          <button onClick={() => setError(null)}>Dismiss</button>
        </div>
      )}

      {filteredTasks.length === 0 ? (
        <div className="panel-empty-centered">
          <span className="empty-icon">📋</span>
          <p>No {filter === 'all' ? '' : filter} tasks</p>
          <p className="empty-hint">Submit a prompt to create a task</p>
        </div>
      ) : (
        <div className="tasks-list">
          {filteredTasks.map((task) => (
            <div 
              key={task.task_id}
              className={`task-row status-${task.status} ${selectedTaskId === task.task_id ? 'expanded' : ''}`}
            >
              <div 
                className="task-main"
                onClick={() => handleSelectTask(task.task_id)}
              >
                <div 
                  className="task-status-indicator"
                  style={{ background: getStatusColor(task.status) }}
                />
                <div className="task-info">
                  <div className="task-header">
                    <span className="task-id">Task #{task.task_id}</span>
                    <span className="task-pipeline">{task.pipeline_name}</span>
                  </div>
                  {task.current_step !== undefined && task.total_steps && (
                    <div className="task-steps">
                      Step {task.current_step}/{task.total_steps}
                    </div>
                  )}
                </div>
                
                {task.status === 'running' && (
                  <div className="task-progress">
                    <div className="progress-bar">
                      <div 
                        className="progress-fill"
                        style={{ width: `${task.progress * 100}%` }}
                      />
                    </div>
                    <span className="progress-text">{Math.round(task.progress * 100)}%</span>
                  </div>
                )}
                
                <div className="task-time">
                  {formatTime(task.started_at || task.created_at)}
                </div>
                
                <div 
                  className="task-status-label"
                  style={{ color: getStatusColor(task.status) }}
                >
                  {task.status}
                </div>
                
                <span className="task-expand-icon">
                  {selectedTaskId === task.task_id ? '▼' : '▶'}
                </span>
              </div>

              {/* Expanded Task Details */}
              {selectedTaskId === task.task_id && (
                <div className="task-details">
                  {loading ? (
                    <div className="details-loading">
                      <div className="loading-spinner small" />
                      <span>Loading details...</span>
                    </div>
                  ) : (
                    <>
                      <div className="details-grid">
                        <div className="detail-item">
                          <span className="detail-label">Pipeline ID</span>
                          <span className="detail-value">{task.pipeline_id}</span>
                        </div>
                        <div className="detail-item">
                          <span className="detail-label">Created</span>
                          <span className="detail-value">{formatDate(task.created_at)}</span>
                        </div>
                        {task.started_at && (
                          <div className="detail-item">
                            <span className="detail-label">Started</span>
                            <span className="detail-value">{formatDate(task.started_at)}</span>
                          </div>
                        )}
                        {task.completed_at && (
                          <div className="detail-item">
                            <span className="detail-label">Completed</span>
                            <span className="detail-value">{formatDate(task.completed_at)}</span>
                          </div>
                        )}
                        {taskDetails?.duration_secs && (
                          <div className="detail-item">
                            <span className="detail-label">Duration</span>
                            <span className="detail-value">{formatDuration(taskDetails.duration_secs)}</span>
                          </div>
                        )}
                        {task.workspace_id && (
                          <div className="detail-item">
                            <span className="detail-label">Workspace</span>
                            <span className="detail-value">#{task.workspace_id}</span>
                          </div>
                        )}
                        {task.project_id && (
                          <div className="detail-item">
                            <span className="detail-label">Project</span>
                            <span className="detail-value">#{task.project_id}</span>
                          </div>
                        )}
                      </div>

                      {taskDetails?.input_summary && (
                        <div className="detail-section">
                          <h4>Input</h4>
                          <p className="detail-text">{taskDetails.input_summary}</p>
                        </div>
                      )}

                      {taskDetails?.output_summary && (
                        <div className="detail-section">
                          <h4>Output</h4>
                          <p className="detail-text">{taskDetails.output_summary}</p>
                        </div>
                      )}

                      {task.error && (
                        <div className="detail-section error">
                          <h4>Error</h4>
                          <p className="detail-text error-text">{task.error}</p>
                        </div>
                      )}

                      {timeline.length > 0 && (
                        <div className="detail-section">
                          <h4>Timeline</h4>
                          <div className="timeline">
                            {timeline.map((event, idx) => (
                              <div key={idx} className="timeline-event">
                                <div className="timeline-dot" />
                                <div className="timeline-content">
                                  <span className="timeline-time">{formatTime(event.timestamp)}</span>
                                  <span className="timeline-event-name">{event.event}</span>
                                  {event.details && (
                                    <span className="timeline-details">{event.details}</span>
                                  )}
                                </div>
                              </div>
                            ))}
                          </div>
                        </div>
                      )}

                      <div className="detail-actions">
                        {task.status === 'running' && (
                          <button 
                            className="action-btn danger"
                            onClick={() => handleCancelTask(task.task_id)}
                          >
                            Cancel
                          </button>
                        )}
                        {(task.status === 'failed' || task.status === 'cancelled') && (
                          <button 
                            className="action-btn"
                            onClick={() => handleRetryTask(task.task_id)}
                          >
                            Retry
                          </button>
                        )}
                      </div>
                    </>
                  )}
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export default TasksTab;
