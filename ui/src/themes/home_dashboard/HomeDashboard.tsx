/**
 * HomeDashboard — thin tab shell.
 * Tab injection/ejection is managed entirely by ThemeArea.
 */

import React from "react";

export interface InjectedTab {
  id: string;
  pipelineId: number;
  label: string;
  icon: string;
  isCore: boolean;
  component: React.ComponentType<any>;
  props?: any;
  badge?: number;
  closeable?: boolean;
  needsAttention?: boolean; // flashes tab if true and not active
}

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
  onTabClose,
}: HomeDashboardProps) {
  const activeTab = injectedTabs.find((t) => t.id === activeTabId);
  const coreTabs = injectedTabs.filter((t) => t.isCore);
  const extraTabs = injectedTabs.filter((t) => !t.isCore);
  const orderedTabs = [...coreTabs, ...extraTabs];

  return (
    <div className="home-dashboard">
      <nav className="dashboard-tabs">
        <div className="tabs-scroll">
          {orderedTabs.map((tab) => {
            const isActive = activeTabId === tab.id;
            const needsFlash = tab.needsAttention && !isActive;
            return (
              <button
                key={tab.id}
                className={[
                  "tab-btn",
                  isActive ? "active" : "",
                  !tab.isCore ? "pipeline-tab" : "",
                  needsFlash ? "needs-attention" : "",
                ]
                  .filter(Boolean)
                  .join(" ")}
                onClick={() => onTabChange(tab.id)}
              >
                <span className="tab-icon">{tab.icon}</span>
                <span className="tab-label">{tab.label}</span>
                {tab.badge !== undefined && tab.badge > 0 && (
                  <span className="tab-badge">{tab.badge}</span>
                )}
                {needsFlash && <span className="attention-dot" />}
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
            );
          })}
        </div>
      </nav>

      <main className="dashboard-content">
        {activeTab ? (
          <activeTab.component {...(activeTab.props ?? {})} />
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

export default HomeDashboard;
