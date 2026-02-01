/**
 * HomeDashboard Theme - Default boot theme
 * 
 * Based on Section 27 of the specification.
 * 
 * This is the starting theme shown after authentication.
 * It provides:
 * - Quick access to recent workspaces/projects
 * - Task recommendations (via TaskRecommendationPipeline)
 * - ZSEI exploration starting point
 * - Quick actions for common operations
 */

import React, { useEffect, useState } from 'react';
import { useOzoneStore } from '../../services/store';

interface HomeDashboardProps {
  themeData: any;
  activeTab: string;
}

interface QuickAction {
  id: string;
  name: string;
  description: string;
  pipelineId: number;
  icon: string;
}

// Quick actions available from home
const QUICK_ACTIONS: QuickAction[] = [
  {
    id: 'new-workspace',
    name: 'New Workspace',
    description: 'Create a new workspace',
    pipelineId: 6, // WorkspaceTabPipeline
    icon: '📁',
  },
  {
    id: 'browse-library',
    name: 'Browse Library',
    description: 'Explore pipelines and methodologies',
    pipelineId: 7, // LibraryTabPipeline
    icon: '📚',
  },
  {
    id: 'link-file',
    name: 'Link File',
    description: 'Link a file to ZSEI',
    pipelineId: 30, // FileLinkPipeline
    icon: '📎',
  },
  {
    id: 'link-url',
    name: 'Link URL',
    description: 'Link a URL to ZSEI',
    pipelineId: 31, // URLLinkPipeline
    icon: '🔗',
  },
  {
    id: 'link-package',
    name: 'Link Package',
    description: 'Link an npm/pip/cargo package',
    pipelineId: 32, // PackageLinkPipeline
    icon: '📦',
  },
  {
    id: 'view-tasks',
    name: 'View Tasks',
    description: 'See all tasks and their status',
    pipelineId: 36, // TaskViewerPipeline
    icon: '📋',
  },
];

export function HomeDashboard({ themeData, activeTab }: HomeDashboardProps) {
  const { executePipeline, activeTasks } = useOzoneStore();
  const [recommendations, setRecommendations] = useState<any[]>([]);
  const [recentWorkspaces, setRecentWorkspaces] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);

  // Load dashboard data on mount
  useEffect(() => {
    async function loadDashboard() {
      setLoading(true);
      try {
        // Get task recommendations (TaskRecommendationPipeline)
        await executePipeline(23, {});
        
        // Query recent workspaces from ZSEI
        const result = await window.ozone.zsei.query({
          type: 'GetUserWorkspaces',
          user_id: 1, // Current user
        });
        
        // Mock data for now
        setRecommendations([
          { id: 1, title: 'Review pending changes', priority: 'high' },
          { id: 2, title: 'Complete documentation', priority: 'medium' },
        ]);
        
        setRecentWorkspaces([
          { id: 1, name: 'Project Alpha', lastAccess: Date.now() - 3600000 },
          { id: 2, name: 'Research Notes', lastAccess: Date.now() - 86400000 },
        ]);
      } catch (err) {
        console.error('Failed to load dashboard:', err);
      } finally {
        setLoading(false);
      }
    }
    
    loadDashboard();
  }, [executePipeline]);

  // Handle quick action
  const handleQuickAction = async (action: QuickAction) => {
    try {
      await executePipeline(action.pipelineId, { action: action.id });
    } catch (err) {
      console.error(`Quick action ${action.id} failed:`, err);
    }
  };

  // Render based on active tab
  const renderTabContent = () => {
    switch (activeTab) {
      case 'workspace':
        return (
          <div className="tab-content workspace-tab">
            <h2>Recent Workspaces</h2>
            {recentWorkspaces.length === 0 ? (
              <p className="empty-state">No recent workspaces. Create one to get started!</p>
            ) : (
              <ul className="workspace-list">
                {recentWorkspaces.map((ws) => (
                  <li key={ws.id} className="workspace-item">
                    <span className="workspace-icon">📁</span>
                    <div className="workspace-info">
                      <span className="workspace-name">{ws.name}</span>
                      <span className="workspace-date">
                        {new Date(ws.lastAccess).toLocaleDateString()}
                      </span>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </div>
        );
      
      case 'library':
        return (
          <div className="tab-content library-tab">
            <h2>Library</h2>
            <div className="library-sections">
              <section className="library-section">
                <h3>Pipelines</h3>
                <p>Browse and manage available pipelines</p>
              </section>
              <section className="library-section">
                <h3>Methodologies</h3>
                <p>Explore methodologies for different domains</p>
              </section>
              <section className="library-section">
                <h3>Blueprints</h3>
                <p>View and create task blueprints</p>
              </section>
            </div>
          </div>
        );
      
      case 'settings':
        return (
          <div className="tab-content settings-tab">
            <h2>Settings</h2>
            <div className="settings-sections">
              <section className="settings-section">
                <h3>Model Configuration</h3>
                <p>Configure LLM models (API, GGUF, ONNX)</p>
              </section>
              <section className="settings-section">
                <h3>Network</h3>
                <p>P2P network and sync settings</p>
              </section>
              <section className="settings-section">
                <h3>UI Preferences</h3>
                <p>Theme and display options</p>
              </section>
            </div>
          </div>
        );
      
      default:
        return null;
    }
  };

  if (loading) {
    return (
      <div className="home-dashboard loading">
        <div className="loading-spinner" />
        <p>Loading dashboard...</p>
      </div>
    );
  }

  return (
    <div className="home-dashboard">
      {/* Welcome Section */}
      <section className="dashboard-welcome">
        <h1>Welcome to Ozone Studio</h1>
        <p>Your zero-shot knowledge orchestration platform</p>
      </section>

      {/* Quick Actions */}
      <section className="dashboard-quick-actions">
        <h2>Quick Actions</h2>
        <div className="quick-actions-grid">
          {QUICK_ACTIONS.map((action) => (
            <button
              key={action.id}
              className="quick-action-card"
              onClick={() => handleQuickAction(action)}
            >
              <span className="action-icon">{action.icon}</span>
              <span className="action-name">{action.name}</span>
              <span className="action-desc">{action.description}</span>
            </button>
          ))}
        </div>
      </section>

      {/* Recommendations */}
      {recommendations.length > 0 && (
        <section className="dashboard-recommendations">
          <h2>Recommended Tasks</h2>
          <ul className="recommendation-list">
            {recommendations.map((rec) => (
              <li key={rec.id} className={`recommendation-item priority-${rec.priority}`}>
                <span className="rec-title">{rec.title}</span>
                <span className="rec-priority">{rec.priority}</span>
              </li>
            ))}
          </ul>
        </section>
      )}

      {/* Active Tasks Summary */}
      {activeTasks.length > 0 && (
        <section className="dashboard-active-tasks">
          <h2>Active Tasks ({activeTasks.length})</h2>
          <ul className="active-tasks-summary">
            {activeTasks.slice(0, 5).map((task) => (
              <li key={task.id} className="task-summary-item">
                Task #{task.id} - {task.status}
              </li>
            ))}
          </ul>
        </section>
      )}

      {/* Tab Content */}
      <section className="dashboard-tab-content">
        {renderTabContent()}
      </section>
    </div>
  );
}
