/**
 * HomeDashboard — thin tab shell.
 * Tab injection/ejection is managed entirely by ThemeArea.
 */

import React from "react";
import {
  Folder, CalendarClock, ListTodo, Library, Settings, Radio, Wrench,
  Smartphone, Share2, Compass, GitBranch, Brain, Files, Puzzle, Handshake,
  Globe,
} from "lucide-react";

// Real Lucide icons for the core tab bar (ZCode's UI/UX review, R3 —
// "trivial, immediate visual-quality gain everywhere"), keyed by tab id.
// Scoped narrowly to THIS render site, not a change to InjectedTab.icon's
// own string type: third-party/dynamic pipelines still supply a plain
// emoji string via getPipelineIcon()/mod.meta.icon (pipeline-ui.tsx) and
// fall through to that unchanged below — swapping that contract itself
// would ripple into every pipeline module's own icon declaration, a much
// bigger, separate, riskier change than this session had scope for. Some
// emoji usage elsewhere (MetaPortion.tsx/ModelSwitchTimeline.tsx's
// interpolated `${icon} switched to...` text) can't take a React element
// the same way a string can — left out of scope for the same reason.
const CORE_TAB_ICONS: Record<string, React.ComponentType<{ size?: number }>> = {
  workspace: Folder,
  order: CalendarClock,
  tasks: ListTodo,
  library: Library,
  settings: Settings,
  monitor: Radio,
  tools: Wrench,
  devices: Smartphone,
  "graph-view": Share2,
  "fabric-view": Compass,
  "hierarchy-view": GitBranch,
  "capture-viewer": Brain,
  "files-viewer": Files,
  engines: Puzzle,
  coordination: Handshake,
  "living-network": Globe,
};

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
                <span className="tab-icon">
                  {CORE_TAB_ICONS[tab.id] ? (
                    React.createElement(CORE_TAB_ICONS[tab.id], { size: 16 })
                  ) : (
                    tab.icon
                  )}
                </span>
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
