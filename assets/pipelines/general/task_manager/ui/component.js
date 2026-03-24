/**
 * TaskManager UI Component (Pipeline #5)
 * 
 * Task list and detail view. This is a CORE TAB - never uninjected.
 * 
 * This REPLACES TaskViewer (#36) - all task viewing is now here.
 * Shows running tasks, completed tasks, task steps, and timelines.
 */

module.exports = {
  meta: {
    title: 'Tasks',
    icon: '📋',
    version: '0.4.0',
  },
  
  render: function(container, props, React, ReactDOM) {
    const { useState, useEffect, useCallback } = React;
    const { executePipeline, subscribeToState } = props;
    
    function TasksUI() {
      const [tasks, setTasks] = useState([]);
      const [selectedTask, setSelectedTask] = useState(null);
      const [taskDetails, setTaskDetails] = useState(null);
      const [filter, setFilter] = useState('all'); // all, running, completed, failed
      const [loading, setLoading] = useState(true);
      const [error, setError] = useState(null);
      
      // Load tasks on mount and poll for updates
      useEffect(() => {
        loadTasks();
        const interval = setInterval(loadTasks, 5000); // Poll every 5s
        return () => clearInterval(interval);
      }, []);
      
      const loadTasks = async () => {
        try {
          const result = await executePipeline(5, { action: 'List' });
          setTasks(result?.tasks || []);
          setLoading(false);
          setError(null);
        } catch (e) {
          console.error('Failed to load tasks:', e);
          setError('Failed to load tasks');
          setLoading(false);
        }
      };
      
      const loadTaskDetails = async (taskId) => {
        try {
          const result = await executePipeline(5, { 
            action: 'GetDetails', 
            task_id: taskId 
          });
          setTaskDetails(result);
        } catch (e) {
          console.error('Failed to load task details:', e);
        }
      };
      
      const handleSelectTask = (task) => {
        setSelectedTask(task);
        loadTaskDetails(task.id);
      };
      
      const handleCancelTask = async (taskId) => {
        try {
          await executePipeline(5, { action: 'Cancel', task_id: taskId });
          loadTasks();
        } catch (e) {
          console.error('Failed to cancel task:', e);
        }
      };
      
      const handleRetryTask = async (taskId) => {
        try {
          await executePipeline(5, { action: 'Retry', task_id: taskId });
          loadTasks();
        } catch (e) {
          console.error('Failed to retry task:', e);
        }
      };
      
      // Filter tasks
      const filteredTasks = tasks.filter(task => {
        if (filter === 'all') return true;
        if (filter === 'running') return task.status === 'running' || task.status === 'queued';
        if (filter === 'completed') return task.status === 'completed';
        if (filter === 'failed') return task.status === 'failed' || task.status === 'cancelled';
        return true;
      });
      
      const getStatusIcon = (status) => {
        switch (status) {
          case 'running': return '⏳';
          case 'queued': return '⏸️';
          case 'completed': return '✅';
          case 'failed': return '❌';
          case 'cancelled': return '🚫';
          default: return '❓';
        }
      };
      
      const formatTime = (timestamp) => {
        if (!timestamp) return '-';
        const date = new Date(timestamp * 1000);
        return date.toLocaleString();
      };
      
      const formatDuration = (seconds) => {
        if (!seconds) return '-';
        if (seconds < 60) return `${seconds}s`;
        const mins = Math.floor(seconds / 60);
        const secs = seconds % 60;
        return `${mins}m ${secs}s`;
      };
      
      if (loading) {
        return React.createElement('div', { className: 'tasks-loading' },
          React.createElement('div', { className: 'loading-spinner' }),
          React.createElement('p', null, 'Loading tasks...')
        );
      }
      
      return React.createElement('div', { className: 'tasks-panel' },
        // Toolbar
        React.createElement('div', { className: 'tasks-toolbar' },
          React.createElement('div', { className: 'filter-buttons' },
            ['all', 'running', 'completed', 'failed'].map(f =>
              React.createElement('button', {
                key: f,
                className: `filter-btn ${filter === f ? 'active' : ''}`,
                onClick: () => setFilter(f)
              }, f.charAt(0).toUpperCase() + f.slice(1))
            )
          ),
          React.createElement('button', { 
            className: 'btn-icon refresh-btn',
            onClick: loadTasks,
            title: 'Refresh'
          }, '🔄')
        ),
        
        // Task list and details
        React.createElement('div', { className: 'tasks-content' },
          // Task list
          React.createElement('div', { className: 'tasks-list' },
            filteredTasks.length === 0 ? (
              React.createElement('div', { className: 'empty-state-centered' },
                React.createElement('span', { className: 'empty-icon' }, '📋'),
                React.createElement('p', null, filter === 'all' 
                  ? 'No tasks yet' 
                  : `No ${filter} tasks`
                ),
                React.createElement('p', { className: 'hint' },
                  'Tasks are created when you send prompts.'
                )
              )
            ) : (
              filteredTasks.map(task =>
                React.createElement('div', {
                  key: task.id,
                  className: `task-item ${selectedTask?.id === task.id ? 'selected' : ''} status-${task.status}`,
                  onClick: () => handleSelectTask(task)
                },
                  React.createElement('span', { className: 'task-status' }, 
                    getStatusIcon(task.status)
                  ),
                  React.createElement('div', { className: 'task-info' },
                    React.createElement('span', { className: 'task-name' }, 
                      task.pipeline_name || `Task #${task.id}`
                    ),
                    React.createElement('span', { className: 'task-time' },
                      formatTime(task.created_at)
                    )
                  ),
                  task.status === 'running' && (
                    React.createElement('div', { className: 'task-progress' },
                      React.createElement('div', { 
                        className: 'progress-bar',
                        style: { width: `${(task.progress || 0) * 100}%` }
                      })
                    )
                  )
                )
              )
            )
          ),
          
          // Task details
          React.createElement('div', { className: 'task-details' },
            selectedTask ? (
              React.createElement('div', { className: 'details-content' },
                React.createElement('div', { className: 'details-header' },
                  React.createElement('h3', null, selectedTask.pipeline_name || `Task #${selectedTask.id}`),
                  React.createElement('span', { 
                    className: `status-badge status-${selectedTask.status}` 
                  }, selectedTask.status)
                ),
                
                // Task metadata
                React.createElement('div', { className: 'details-meta' },
                  React.createElement('div', { className: 'meta-row' },
                    React.createElement('span', { className: 'meta-label' }, 'Created:'),
                    React.createElement('span', { className: 'meta-value' }, 
                      formatTime(selectedTask.created_at)
                    )
                  ),
                  selectedTask.started_at && (
                    React.createElement('div', { className: 'meta-row' },
                      React.createElement('span', { className: 'meta-label' }, 'Started:'),
                      React.createElement('span', { className: 'meta-value' }, 
                        formatTime(selectedTask.started_at)
                      )
                    )
                  ),
                  selectedTask.completed_at && (
                    React.createElement('div', { className: 'meta-row' },
                      React.createElement('span', { className: 'meta-label' }, 'Completed:'),
                      React.createElement('span', { className: 'meta-value' }, 
                        formatTime(selectedTask.completed_at)
                      )
                    )
                  ),
                  React.createElement('div', { className: 'meta-row' },
                    React.createElement('span', { className: 'meta-label' }, 'Duration:'),
                    React.createElement('span', { className: 'meta-value' }, 
                      formatDuration(selectedTask.duration_secs)
                    )
                  )
                ),
                
                // Steps (if blueprint has steps)
                taskDetails?.steps && taskDetails.steps.length > 0 && (
                  React.createElement('div', { className: 'task-steps' },
                    React.createElement('h4', null, 'Steps'),
                    React.createElement('div', { className: 'steps-list' },
                      taskDetails.steps.map((step, idx) =>
                        React.createElement('div', { 
                          key: idx,
                          className: `step-item status-${step.status}`
                        },
                          React.createElement('span', { className: 'step-number' }, idx + 1),
                          React.createElement('span', { className: 'step-name' }, step.name),
                          React.createElement('span', { className: 'step-status' }, 
                            getStatusIcon(step.status)
                          )
                        )
                      )
                    )
                  )
                ),
                
                // Error message
                selectedTask.error && (
                  React.createElement('div', { className: 'task-error' },
                    React.createElement('h4', null, 'Error'),
                    React.createElement('p', null, selectedTask.error)
                  )
                ),
                
                // Actions
                React.createElement('div', { className: 'task-actions' },
                  (selectedTask.status === 'running' || selectedTask.status === 'queued') && (
                    React.createElement('button', {
                      className: 'btn-danger',
                      onClick: () => handleCancelTask(selectedTask.id)
                    }, 'Cancel')
                  ),
                  selectedTask.status === 'failed' && (
                    React.createElement('button', {
                      className: 'btn-primary',
                      onClick: () => handleRetryTask(selectedTask.id)
                    }, 'Retry')
                  )
                )
              )
            ) : (
              React.createElement('div', { className: 'empty-state-centered' },
                React.createElement('p', null, 'Select a task to view details')
              )
            )
          )
        )
      );
    }
    
    const root = ReactDOM.createRoot(container);
    root.render(React.createElement(TasksUI));
    
    return () => root.unmount();
  },
  
  onActivate: function() {
    console.log('TaskManager UI activated');
  },
  
  onDeactivate: function() {
    console.log('TaskManager UI deactivated');
  },
};
