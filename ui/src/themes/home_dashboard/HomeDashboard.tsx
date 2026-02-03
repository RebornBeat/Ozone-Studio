/**
 * HomeDashboard Theme v0.4.0
 * 
 * Tab-based layout:
 * [Workspace] [Tasks] [Library] [Settings]
 * 
 * - NO prompt input here (only in META portion)
 * - Buttons are functional and communicate with backend
 * - Clean, modern UI/UX
 */

import React, { useEffect, useState, useCallback } from 'react';
import { useOzoneStore } from '../../services/store';

interface HomeDashboardProps {
  themeData: any;
  activeTab?: string;
}

type TabType = 'workspace' | 'tasks' | 'library' | 'settings';

interface Workspace {
  id: string;
  name: string;
  lastAccess: number;
  taskCount: number;
  isGlobal?: boolean;
}

interface TaskInfo {
  id: number;
  pipelineName: string;
  status: 'pending' | 'running' | 'completed' | 'failed';
  progress: number;
  createdAt: number;
}

interface ModelConfig {
  model_type: string;
  api_key?: string;
  local_model_path?: string;
}

export function HomeDashboard({ themeData, activeTab: initialTab }: HomeDashboardProps) {
  const { 
    executePipeline, 
    activeTasks, 
    isConnected, 
    systemStats, 
    consciousnessEnabled,
    setConsciousnessEnabled,
  } = useOzoneStore();
  
  const [currentTab, setCurrentTab] = useState<TabType>(initialTab as TabType || 'workspace');
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [tasks, setTasks] = useState<TaskInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [actionLoading, setActionLoading] = useState<string | null>(null);
  const [newWorkspaceName, setNewWorkspaceName] = useState('');
  const [showNewWorkspaceDialog, setShowNewWorkspaceDialog] = useState(false);
  const [linkUrl, setLinkUrl] = useState('');
  const [showLinkUrlDialog, setShowLinkUrlDialog] = useState(false);
  
  // Settings state
  const [settingsConfig, setSettingsConfig] = useState<any>(null);

  // Load data
  const loadData = useCallback(async () => {
    if (!isConnected || !window.ozone) return;
    
    setLoading(true);
    try {
      // Load workspaces
      try {
        const result = await window.ozone.zsei.query({
          type: 'GetUserWorkspaces',
          limit: 20,
        });
        if (result && Array.isArray((result as any).workspaces)) {
          setWorkspaces((result as any).workspaces);
        }
      } catch (e) {
        // Default workspace
        setWorkspaces([{
          id: 'global',
          name: 'Global Workspace',
          lastAccess: Date.now(),
          taskCount: 0,
          isGlobal: true
        }]);
      }
      
      // Load tasks
      try {
        const result = await window.ozone.task.list();
        if (result && Array.isArray((result as any).tasks)) {
          setTasks((result as any).tasks.map((t: any) => ({
            id: t.task_id,
            pipelineName: t.pipeline_name || `Pipeline #${t.pipeline_id}`,
            status: t.status?.toLowerCase() || 'pending',
            progress: t.progress || 0,
            createdAt: t.created_at || Date.now(),
          })));
        }
      } catch (e) {
        console.warn('Could not load tasks');
      }
      
      // Load config for settings
      try {
        const config = await window.ozone.config.get();
        setSettingsConfig(config);
      } catch (e) {
        console.warn('Could not load config');
      }
    } finally {
      setLoading(false);
    }
  }, [isConnected]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  // === ACTION HANDLERS ===
  
  const handleNewWorkspace = async () => {
    if (!newWorkspaceName.trim() || !isConnected) return;
    
    setActionLoading('new-workspace');
    try {
      await executePipeline(6, { 
        action: 'create',
        name: newWorkspaceName.trim()
      });
      setShowNewWorkspaceDialog(false);
      setNewWorkspaceName('');
      await loadData();
    } catch (err) {
      console.error('Create workspace failed:', err);
    } finally {
      setActionLoading(null);
    }
  };

  const handleLinkFile = async () => {
    if (!isConnected) return;
    
    setActionLoading('link-file');
    try {
      // Request file selection from Electron
      if (window.ozone?.system?.selectFile) {
        const path = await window.ozone.system.selectFile();
        if (path) {
          await executePipeline(30, { type: 'file', path });
          await loadData();
        }
      } else {
        // Fallback: trigger via pipeline
        await executePipeline(30, { type: 'file', action: 'select' });
      }
    } catch (err) {
      console.error('Link file failed:', err);
    } finally {
      setActionLoading(null);
    }
  };

  const handleLinkUrl = async () => {
    if (!linkUrl.trim() || !isConnected) return;
    
    setActionLoading('link-url');
    try {
      await executePipeline(31, { type: 'url', url: linkUrl.trim() });
      setShowLinkUrlDialog(false);
      setLinkUrl('');
      await loadData();
    } catch (err) {
      console.error('Link URL failed:', err);
    } finally {
      setActionLoading(null);
    }
  };

  const handleLinkPackage = async () => {
    if (!isConnected) return;
    
    setActionLoading('link-package');
    try {
      await executePipeline(32, { type: 'package', action: 'select' });
    } catch (err) {
      console.error('Link package failed:', err);
    } finally {
      setActionLoading(null);
    }
  };

  const handleOpenWorkspace = async (workspaceId: string) => {
    if (!isConnected) return;
    
    setActionLoading(`workspace-${workspaceId}`);
    try {
      await executePipeline(6, { 
        action: 'open',
        workspace_id: workspaceId
      });
    } catch (err) {
      console.error('Open workspace failed:', err);
    } finally {
      setActionLoading(null);
    }
  };

  const handleSaveSettings = async (updates: any) => {
    if (!isConnected || !window.ozone?.config?.set) return;
    
    setActionLoading('save-settings');
    try {
      await window.ozone.config.set(updates);
      await loadData();
    } catch (err) {
      console.error('Save settings failed:', err);
    } finally {
      setActionLoading(null);
    }
  };

  const handleToggleConsciousness = async () => {
    const newValue = !consciousnessEnabled;
    setConsciousnessEnabled(newValue);
    await handleSaveSettings({ consciousness: { enabled: newValue } });
  };

  // === TAB RENDERERS ===

  const renderWorkspaceTab = () => (
    <div className="tab-panel workspace-panel">
      {/* Actions Bar */}
      <div className="panel-actions">
        <button 
          className={`action-btn primary ${actionLoading === 'new-workspace' ? 'loading' : ''}`}
          onClick={() => setShowNewWorkspaceDialog(true)}
          disabled={!isConnected || !!actionLoading}
        >
          <span className="btn-icon">+</span>
          <span className="btn-text">New Workspace</span>
        </button>
        <div className="action-divider" />
        <button 
          className={`action-btn ${actionLoading === 'link-file' ? 'loading' : ''}`}
          onClick={handleLinkFile}
          disabled={!isConnected || !!actionLoading}
        >
          <span className="btn-icon">📎</span>
          <span className="btn-text">Link File</span>
        </button>
        <button 
          className={`action-btn ${actionLoading === 'link-url' ? 'loading' : ''}`}
          onClick={() => setShowLinkUrlDialog(true)}
          disabled={!isConnected || !!actionLoading}
        >
          <span className="btn-icon">🔗</span>
          <span className="btn-text">Link URL</span>
        </button>
        <button 
          className={`action-btn ${actionLoading === 'link-package' ? 'loading' : ''}`}
          onClick={handleLinkPackage}
          disabled={!isConnected || !!actionLoading}
        >
          <span className="btn-icon">📦</span>
          <span className="btn-text">Link Package</span>
        </button>
      </div>

      {/* Workspace List */}
      <div className="workspace-grid">
        {!isConnected ? (
          <div className="panel-empty">
            <span className="empty-icon">🔌</span>
            <p>Connect to view workspaces</p>
          </div>
        ) : workspaces.length === 0 ? (
          <div className="panel-empty">
            <span className="empty-icon">📁</span>
            <p>No workspaces yet</p>
            <p className="empty-hint">Create one to organize your projects</p>
          </div>
        ) : (
          workspaces.map((ws) => (
            <div 
              key={ws.id} 
              className={`workspace-card ${ws.isGlobal ? 'global' : ''} ${actionLoading === `workspace-${ws.id}` ? 'loading' : ''}`}
              onClick={() => handleOpenWorkspace(ws.id)}
            >
              <div className="ws-header">
                <span className="ws-icon">{ws.isGlobal ? '🌐' : '📁'}</span>
                <span className="ws-name">{ws.name}</span>
              </div>
              <div className="ws-meta">
                <span>{ws.taskCount} tasks</span>
                <span>•</span>
                <span>{new Date(ws.lastAccess).toLocaleDateString()}</span>
              </div>
            </div>
          ))
        )}
      </div>

      {/* New Workspace Dialog */}
      {showNewWorkspaceDialog && (
        <div className="dialog-overlay" onClick={() => setShowNewWorkspaceDialog(false)}>
          <div className="dialog" onClick={e => e.stopPropagation()}>
            <h3>Create New Workspace</h3>
            <input
              type="text"
              placeholder="Workspace name"
              value={newWorkspaceName}
              onChange={(e) => setNewWorkspaceName(e.target.value)}
              autoFocus
              onKeyDown={(e) => e.key === 'Enter' && handleNewWorkspace()}
            />
            <div className="dialog-buttons">
              <button className="btn-cancel" onClick={() => setShowNewWorkspaceDialog(false)}>Cancel</button>
              <button className="btn-confirm" onClick={handleNewWorkspace} disabled={!newWorkspaceName.trim()}>Create</button>
            </div>
          </div>
        </div>
      )}

      {/* Link URL Dialog */}
      {showLinkUrlDialog && (
        <div className="dialog-overlay" onClick={() => setShowLinkUrlDialog(false)}>
          <div className="dialog" onClick={e => e.stopPropagation()}>
            <h3>Link URL</h3>
            <input
              type="url"
              placeholder="https://example.com"
              value={linkUrl}
              onChange={(e) => setLinkUrl(e.target.value)}
              autoFocus
              onKeyDown={(e) => e.key === 'Enter' && handleLinkUrl()}
            />
            <div className="dialog-buttons">
              <button className="btn-cancel" onClick={() => setShowLinkUrlDialog(false)}>Cancel</button>
              <button className="btn-confirm" onClick={handleLinkUrl} disabled={!linkUrl.trim()}>Link</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );

  const renderTasksTab = () => (
    <div className="tab-panel tasks-panel">
      <div className="panel-header">
        <h2>Tasks</h2>
        <div className="task-stats">
          <span className="stat">{activeTasks.filter((t: any) => t.status === 'running').length} running</span>
          <span className="stat">{tasks.filter(t => t.status === 'completed').length} completed</span>
        </div>
      </div>

      <div className="tasks-list">
        {!isConnected ? (
          <div className="panel-empty">
            <span className="empty-icon">🔌</span>
            <p>Connect to view tasks</p>
          </div>
        ) : tasks.length === 0 && activeTasks.length === 0 ? (
          <div className="panel-empty">
            <span className="empty-icon">📋</span>
            <p>No tasks yet</p>
            <p className="empty-hint">Submit a prompt to create a task</p>
          </div>
        ) : (
          <>
            {/* Active Tasks */}
            {activeTasks.map((task: any) => (
              <div key={task.id} className={`task-item status-${task.status}`}>
                <div className="task-status-dot" />
                <div className="task-info">
                  <span className="task-title">Task #{task.id}</span>
                  <span className="task-pipeline">{task.pipelineName || 'Pipeline'}</span>
                </div>
                {task.status === 'running' && (
                  <div className="task-progress-bar">
                    <div className="progress-fill" style={{ width: `${(task.progress || 0) * 100}%` }} />
                  </div>
                )}
                <span className="task-status-label">{task.status}</span>
              </div>
            ))}
            
            {/* Historical Tasks */}
            {tasks.map((task) => (
              <div key={task.id} className={`task-item status-${task.status}`}>
                <div className="task-status-dot" />
                <div className="task-info">
                  <span className="task-title">Task #{task.id}</span>
                  <span className="task-pipeline">{task.pipelineName}</span>
                </div>
                <span className="task-time">{new Date(task.createdAt).toLocaleTimeString()}</span>
                <span className="task-status-label">{task.status}</span>
              </div>
            ))}
          </>
        )}
      </div>
    </div>
  );

  const renderLibraryTab = () => (
    <div className="tab-panel library-panel">
      <div className="panel-header">
        <h2>Library</h2>
      </div>

      <div className="library-grid">
        <div className="library-card" onClick={() => console.log('View pipelines')}>
          <span className="lib-icon">🔧</span>
          <div className="lib-info">
            <span className="lib-title">Pipelines</span>
            <span className="lib-count">{isConnected ? '54 available' : '—'}</span>
          </div>
          <span className="lib-arrow">→</span>
        </div>

        <div className="library-card" onClick={() => console.log('View methodologies')}>
          <span className="lib-icon">📐</span>
          <div className="lib-info">
            <span className="lib-title">Methodologies</span>
            <span className="lib-count">{isConnected ? `${systemStats.methodologiesShared} shared` : '—'}</span>
          </div>
          <span className="lib-arrow">→</span>
        </div>

        <div className="library-card" onClick={() => console.log('View blueprints')}>
          <span className="lib-icon">📘</span>
          <div className="lib-info">
            <span className="lib-title">Blueprints</span>
            <span className="lib-count">{isConnected ? `${systemStats.blueprintsShared} shared` : '—'}</span>
          </div>
          <span className="lib-arrow">→</span>
        </div>

        <div className="library-card" onClick={() => console.log('View findings')}>
          <span className="lib-icon">🔍</span>
          <div className="lib-info">
            <span className="lib-title">Findings</span>
            <span className="lib-count">{isConnected ? `${systemStats.findingsShared} shared` : '—'}</span>
          </div>
          <span className="lib-arrow">→</span>
        </div>
      </div>
    </div>
  );

  const renderSettingsTab = () => (
    <div className="tab-panel settings-panel">
      <div className="panel-header">
        <h2>Settings</h2>
      </div>

      <div className="settings-sections">
        {/* Model Configuration */}
        <div className="settings-section">
          <div className="section-header">
            <span className="section-icon">🤖</span>
            <h3>Model Configuration</h3>
          </div>
          <div className="section-body">
            <div className="setting-row">
              <label>Model Type</label>
              <span className="setting-value">
                {settingsConfig?.models?.model_type || 'Not configured'}
              </span>
            </div>
            {settingsConfig?.models?.local_model_path && (
              <div className="setting-row">
                <label>Model Path</label>
                <span className="setting-value truncate">
                  {settingsConfig.models.local_model_path}
                </span>
              </div>
            )}
            <button className="setting-btn">Configure Models →</button>
          </div>
        </div>

        {/* Voice */}
        <div className="settings-section">
          <div className="section-header">
            <span className="section-icon">🎤</span>
            <h3>Voice Input</h3>
          </div>
          <div className="section-body">
            <div className="setting-row">
              <label>Enabled</label>
              <span className="setting-value">
                {settingsConfig?.voice?.enabled ? 'Yes' : 'No'}
              </span>
            </div>
            <div className="setting-row">
              <label>Whisper Model</label>
              <span className="setting-value">
                {settingsConfig?.voice?.whisper_model || 'base'}
              </span>
            </div>
          </div>
        </div>

        {/* Consciousness */}
        <div className="settings-section">
          <div className="section-header">
            <span className="section-icon">🧠</span>
            <h3>Consciousness System</h3>
          </div>
          <div className="section-body">
            <div className="setting-row">
              <label>Enable Consciousness</label>
              <label className="toggle-switch">
                <input 
                  type="checkbox"
                  checked={consciousnessEnabled}
                  onChange={handleToggleConsciousness}
                  disabled={!isConnected || actionLoading === 'save-settings'}
                />
                <span className="toggle-slider" />
              </label>
            </div>
            <p className="setting-hint">
              Includes emotional context, experience memory, and self-reflection
            </p>
          </div>
        </div>

        {/* P2P Network */}
        <div className="settings-section">
          <div className="section-header">
            <span className="section-icon">🌐</span>
            <h3>P2P Network</h3>
          </div>
          <div className="section-body">
            <div className="setting-row">
              <label>Connected Peers</label>
              <span className="setting-value">{systemStats.peerCount}</span>
            </div>
            <div className="setting-row">
              <label>Your Contributions</label>
              <span className="setting-value">{systemStats.myContributions}</span>
            </div>
          </div>
        </div>

        {/* Data */}
        <div className="settings-section">
          <div className="section-header">
            <span className="section-icon">📊</span>
            <h3>Data & Privacy</h3>
          </div>
          <div className="section-body">
            <div className="setting-row">
              <label>ZSEI Containers</label>
              <span className="setting-value">{systemStats.zseiContainers}</span>
            </div>
            <button className="setting-btn">Export Data →</button>
            <button className="setting-btn danger">Clear Data →</button>
          </div>
        </div>
      </div>
    </div>
  );

  // Render current tab
  const renderTabContent = () => {
    switch (currentTab) {
      case 'workspace': return renderWorkspaceTab();
      case 'tasks': return renderTasksTab();
      case 'library': return renderLibraryTab();
      case 'settings': return renderSettingsTab();
      default: return renderWorkspaceTab();
    }
  };

  return (
    <div className="home-dashboard">
      {/* Tab Navigation */}
      <nav className="dashboard-nav">
        <button 
          className={`nav-tab ${currentTab === 'workspace' ? 'active' : ''}`}
          onClick={() => setCurrentTab('workspace')}
        >
          <span className="tab-icon">📁</span>
          Workspace
        </button>
        <button 
          className={`nav-tab ${currentTab === 'tasks' ? 'active' : ''}`}
          onClick={() => setCurrentTab('tasks')}
        >
          <span className="tab-icon">📋</span>
          Tasks
          {activeTasks.length > 0 && <span className="tab-badge">{activeTasks.length}</span>}
        </button>
        <button 
          className={`nav-tab ${currentTab === 'library' ? 'active' : ''}`}
          onClick={() => setCurrentTab('library')}
        >
          <span className="tab-icon">📚</span>
          Library
        </button>
        <button 
          className={`nav-tab ${currentTab === 'settings' ? 'active' : ''}`}
          onClick={() => setCurrentTab('settings')}
        >
          <span className="tab-icon">⚙️</span>
          Settings
        </button>
      </nav>

      {/* Tab Content */}
      <main className="dashboard-main">
        {loading ? (
          <div className="panel-loading">
            <div className="loading-spinner" />
            <p>Loading...</p>
          </div>
        ) : (
          renderTabContent()
        )}
      </main>
    </div>
  );
}
