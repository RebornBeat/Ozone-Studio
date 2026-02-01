/**
 * HomeDashboard Theme - Default boot theme
 * 
 * OZONE STUDIO — Omnidirectional Zero-Shot Neural Engine
 * A Collective AGI Framework with Optional Consciousness
 * 
 * This is the starting theme shown after initialization.
 * It provides:
 * - Quick access to recent workspaces/projects
 * - Task recommendations (via TaskRecommendationPipeline)
 * - ZSEI exploration starting point
 * - Quick actions for common operations
 * 
 * NO MOCK DATA - all content comes from backend or shows empty states
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
    pipelineId: 6,
    icon: '📁',
  },
  {
    id: 'browse-library',
    name: 'Browse Library',
    description: 'Explore pipelines and methodologies',
    pipelineId: 7,
    icon: '📚',
  },
  {
    id: 'link-file',
    name: 'Link File',
    description: 'Link a file to ZSEI',
    pipelineId: 30,
    icon: '📎',
  },
  {
    id: 'link-url',
    name: 'Link URL',
    description: 'Link a URL to ZSEI',
    pipelineId: 31,
    icon: '🔗',
  },
  {
    id: 'link-package',
    name: 'Link Package',
    description: 'Link an npm/pip/cargo package',
    pipelineId: 32,
    icon: '📦',
  },
  {
    id: 'view-tasks',
    name: 'View Tasks',
    description: 'See all tasks and their status',
    pipelineId: 36,
    icon: '📋',
  },
];

export function HomeDashboard({ themeData, activeTab }: HomeDashboardProps) {
  const { executePipeline, activeTasks, isConnected, systemStats, consciousnessEnabled } = useOzoneStore();
  const [recommendations, setRecommendations] = useState<any[]>([]);
  const [recentWorkspaces, setRecentWorkspaces] = useState<any[]>([]);
  const [loading, setLoading] = useState(false);

  // Load dashboard data on mount (only if connected)
  useEffect(() => {
    if (!isConnected || !window.ozone) {
      setLoading(false);
      return;
    }
    
    async function loadDashboard() {
      setLoading(true);
      try {
        // Get task recommendations
        try {
          await executePipeline(23, {});
        } catch (e) {
          console.warn('Task recommendations not available');
        }
        
        // Query recent workspaces from ZSEI
        try {
          const result = await window.ozone!.zsei.query({
            type: 'GetUserWorkspaces',
            limit: 5,
          });
          if (result && Array.isArray((result as any).workspaces)) {
            setRecentWorkspaces((result as any).workspaces);
          }
        } catch (e) {
          console.warn('Could not load workspaces');
        }
        
        // Query recommendations from ZSEI
        try {
          const result = await window.ozone!.zsei.query({
            type: 'GetTaskRecommendations',
            limit: 5,
          });
          if (result && Array.isArray((result as any).recommendations)) {
            setRecommendations((result as any).recommendations);
          }
        } catch (e) {
          console.warn('Could not load recommendations');
        }
      } catch (err) {
        console.error('Failed to load dashboard:', err);
      } finally {
        setLoading(false);
      }
    }
    
    loadDashboard();
  }, [executePipeline, isConnected]);

  // Handle quick action
  const handleQuickAction = async (action: QuickAction) => {
    if (!isConnected) {
      console.warn('Backend not connected - cannot execute action');
      return;
    }
    
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
            {!isConnected ? (
              <p className="empty-state offline">Connect to backend to view workspaces</p>
            ) : recentWorkspaces.length === 0 ? (
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
                <p>Configure LLM models (Local GGUF, API fallback)</p>
              </section>
              <section className="settings-section">
                <h3>P2P Network</h3>
                <p>Peer-to-peer network and sync settings</p>
              </section>
              <section className="settings-section">
                <h3>Consciousness System</h3>
                <p>Enable/disable consciousness features (I-Loop, emotional processing)</p>
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
        <p className="tagline">Omnidirectional Zero-Shot Neural Engine</p>
        <p className="subtitle">A Collective AGI Framework{consciousnessEnabled && ' with Consciousness'}</p>
        
        {/* Connection Status Banner */}
        {!isConnected && (
          <div className="connection-banner offline">
            <span className="banner-icon">⚠️</span>
            <span className="banner-text">Backend not connected. Start the Ozone Studio backend to enable full functionality.</span>
          </div>
        )}
      </section>

      {/* Collective Stats (the focus!) */}
      <section className="dashboard-collective">
        <h2>🌐 Collective Ecosystem</h2>
        <div className="collective-stats">
          <div className="stat-card">
            <span className="stat-value">{systemStats.totalContributions.toLocaleString()}</span>
            <span className="stat-label">Total Contributions</span>
          </div>
          <div className="stat-card">
            <span className="stat-value">{systemStats.peerCount}</span>
            <span className="stat-label">Connected Peers</span>
          </div>
          <div className="stat-card">
            <span className="stat-value">{systemStats.methodologiesShared}</span>
            <span className="stat-label">Methodologies</span>
          </div>
          <div className="stat-card">
            <span className="stat-value">{systemStats.blueprintsShared}</span>
            <span className="stat-label">Blueprints</span>
          </div>
          <div className="stat-card highlight">
            <span className="stat-value">{systemStats.myContributions}</span>
            <span className="stat-label">Your Contributions</span>
          </div>
        </div>
      </section>

      {/* Quick Actions */}
      <section className="dashboard-quick-actions">
        <h2>Quick Actions</h2>
        <div className="quick-actions-grid">
          {QUICK_ACTIONS.map((action) => (
            <button
              key={action.id}
              className={`quick-action-card ${!isConnected ? 'disabled' : ''}`}
              onClick={() => handleQuickAction(action)}
              disabled={!isConnected}
            >
              <span className="action-icon">{action.icon}</span>
              <span className="action-name">{action.name}</span>
              <span className="action-desc">{action.description}</span>
            </button>
          ))}
        </div>
      </section>

      {/* Recommendations (only if connected and have data) */}
      {isConnected && recommendations.length > 0 && (
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
