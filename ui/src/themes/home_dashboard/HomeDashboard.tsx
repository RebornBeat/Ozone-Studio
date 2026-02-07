/**
 * HomeDashboard Theme v0.4.0
 * 
 * This is ONLY the shell/container for tabs.
 * ALL tabs (including core) are INJECTED by ThemeLoader.
 * 
 * Architecture:
 * - HomeDashboard provides: tab bar + content area
 * - ThemeLoader injects: tab definitions + UI components
 * - Each pipeline has ui/ folder with its component
 * 
 * Core tabs (injected on startup, never uninjected):
 * - Workspace (Pipeline #6)
 * - Tasks (Pipeline #5 - TaskManager)
 * - Library (Pipeline #7)
 * - Settings (Pipeline #8)
 * 
 * Non-core tabs (injected during tasks, uninjected on completion):
 * - Appear after Settings
 * - Can be set active on injection
 * - Automatically uninjected when task/step completes
 */

import React, { useEffect, useState, useCallback } from 'react';
import { useOzoneStore } from '../../services/store';

// Tab definition from ThemeLoader
export interface InjectedTab {
  id: string;
  pipelineId: number;
  label: string;
  icon: string;
  isCore: boolean;           // Core tabs never uninjected
  component: React.ComponentType<any>;  // The actual UI component
  props?: any;               // Props to pass to component
  badge?: number;            // Badge count (e.g., task count)
  closeable?: boolean;       // Can user close this tab?
}

// Props received from App.tsx
interface HomeDashboardProps {
  injectedTabs: InjectedTab[];
  activeTabId: string;
  onTabChange: (tabId: string) => void;
  onTabClose?: (tabId: string) => void;
}

export function HomeDashboard({ 
  injectedTabs, 
  activeTabId, 
  onTabChange,
  onTabClose 
}: HomeDashboardProps) {
  // Find the active tab
  const activeTab = injectedTabs.find(t => t.id === activeTabId);
  
  // Separate core and non-core tabs for ordering
  const coreTabs = injectedTabs.filter(t => t.isCore);
  const pipelineTabs = injectedTabs.filter(t => !t.isCore);
  const orderedTabs = [...coreTabs, ...pipelineTabs];

  return (
    <div className="home-dashboard">
      {/* Tab Navigation - Full width, edge to edge */}
      <nav className="dashboard-tabs">
        <div className="tabs-scroll">
          {orderedTabs.map((tab) => (
            <button 
              key={tab.id}
              className={`tab-btn ${activeTabId === tab.id ? 'active' : ''} ${!tab.isCore ? 'pipeline-tab' : ''}`}
              onClick={() => onTabChange(tab.id)}
            >
              <span className="tab-icon">{tab.icon}</span>
              <span className="tab-label">{tab.label}</span>
              {tab.badge !== undefined && tab.badge > 0 && (
                <span className="tab-badge">{tab.badge}</span>
              )}
              {tab.closeable && !tab.isCore && (
                <span 
                  className="tab-close"
                  onClick={(e) => {
                    e.stopPropagation();
                    onTabClose?.(tab.id);
                  }}
                >
                  ×
                </span>
              )}
            </button>
          ))}
        </div>
      </nav>

      {/* Tab Content - Full height, edge to edge */}
      <main className="dashboard-content">
        {activeTab ? (
          <activeTab.component {...(activeTab.props || {})} />
        ) : (
          <div className="panel-empty-centered">
            <span className="empty-icon">📭</span>
            <p>No tab selected</p>
          </div>
        )}
      </main>
    </div>
  );
}

/**
 * Default export for theme loading
 */
export default HomeDashboard;
