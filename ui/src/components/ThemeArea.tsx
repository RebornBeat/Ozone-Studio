/**
 * ThemeArea — orchestrates tab injection, notifications, and pipeline UI loading.
 *
 * Tab attention rules:
 * - Backend signals attention → tab flashes + notification added
 * - If active tab IS core AND attention tab is also core → switch immediately
 * - Otherwise → flash tab + notification; user decides when to switch
 * - User-injected non-core tabs: wait for user to close (no auto-uninject)
 * - Backend-injected non-core tabs: auto-uninject when task completes
 */

import React, { useEffect, useState, useCallback, useRef } from "react";
import { useOzoneStore } from "../services/store";
import {
  HomeDashboard,
  InjectedTab,
} from "../themes/home_dashboard/HomeDashboard";
import {
  createCoreTabs,
  createDynamicTab,
  hasPipelineUI,
  CORE_TAB_DEFINITIONS,
  clearModuleCache,
} from "../pipeline-ui";
import { onNavigate, NavTarget } from "../navigation";
import UnifiedSearch from "../views/search/UnifiedSearch";

/** J2: which real tab a cross-view NavTarget lands on. `graph-node` and
 * `container` both resolve inside the Graph View tab itself (GraphView.tsx
 * consumes the same target via `takePendingNavigation` to pick the project
 * and node) — this only owns the actual tab switch, not the in-tab detail. */
function tabForNavTarget(target: NavTarget): string | null {
  switch (target.kind) {
    case "tab":
      return target.tabId;
    case "graph-node":
    case "container":
      return "graph-view";
    case "task":
      return "tasks";
    case "code-file":
      return "files-viewer";
    default:
      return null;
  }
}

interface ThemeAreaProps {
  theme: string;
}

// ============================================================================
// Notification Center
// ============================================================================

interface NotificationItem {
  id: string;
  tabId?: string;
  pipelineId?: number;
  message: string;
  timestamp: number;
  read: boolean;
  type: "info" | "task" | "warning";
}

function NotificationCenter({
  notifications,
  onDismiss,
  onDismissAll,
  onTabFocus,
}: {
  notifications: NotificationItem[];
  onDismiss: (id: string) => void;
  onDismissAll: () => void;
  onTabFocus: (tabId: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const unread = notifications.filter((n) => !n.read).length;

  if (notifications.length === 0 && !open) {
    return (
      <div className="notification-bell quiet">
        <span>🔔</span>
      </div>
    );
  }

  return (
    <div className="notification-center">
      <button
        className={`notification-bell ${unread > 0 ? "has-unread" : ""}`}
        onClick={() => setOpen((o) => !o)}
        title={`${unread} unread notifications`}
      >
        🔔
        {unread > 0 && <span className="bell-badge">{unread}</span>}
      </button>

      {open && (
        <div className="notification-dropdown">
          <div className="notification-header">
            <span>Notifications</span>
            {notifications.length > 0 && (
              <button className="dismiss-all-btn" onClick={onDismissAll}>
                Clear all
              </button>
            )}
          </div>

          {notifications.length === 0 ? (
            <div className="notification-empty">No notifications</div>
          ) : (
            notifications.map((n) => (
              <div
                key={n.id}
                className={`notification-item ${n.read ? "read" : "unread"} type-${n.type}`}
              >
                <div className="notification-body">
                  <span className="notification-icon">
                    {n.type === "task"
                      ? "⚡"
                      : n.type === "warning"
                        ? "⚠️"
                        : "ℹ️"}
                  </span>
                  <span className="notification-message">{n.message}</span>
                </div>
                <div className="notification-actions">
                  {n.tabId && (
                    <button
                      className="notification-goto"
                      onClick={() => {
                        onTabFocus(n.tabId!);
                        setOpen(false);
                      }}
                    >
                      View →
                    </button>
                  )}
                  <button
                    className="notification-dismiss"
                    onClick={() => onDismiss(n.id)}
                  >
                    ×
                  </button>
                </div>
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
}

// ============================================================================
// ThemeArea
// ============================================================================

export function ThemeArea({ theme }: ThemeAreaProps) {
  const { activeTasks } = useOzoneStore();

  const [injectedTabs, setInjectedTabs] = useState<InjectedTab[]>([]);
  const [activeTabId, setActiveTabId] = useState<string>("workspace");
  const [notifications, setNotifications] = useState<NotificationItem[]>([]);
  const [loading, setLoading] = useState(true);

  // Track which tabs were backend-injected (auto-uninject eligible)
  const backendInjected = useRef<Set<string>>(new Set());

  // Initialize core tabs once on mount
  useEffect(() => {
    const tabs = createCoreTabs();
    setInjectedTabs(tabs);
    setLoading(false);
  }, []);

  // ── Notifications ────────────────────────────────────────────────────────

  const addNotification = useCallback(
    (
      message: string,
      type: NotificationItem["type"] = "info",
      tabId?: string,
      pipelineId?: number,
    ) => {
      const item: NotificationItem = {
        id: `notif-${Date.now()}-${Math.random().toString(36).slice(2)}`,
        message,
        type,
        tabId,
        pipelineId,
        timestamp: Date.now(),
        read: false,
      };
      setNotifications((prev) => [item, ...prev].slice(0, 50)); // cap at 50
    },
    [],
  );

  const dismissNotification = useCallback((id: string) => {
    setNotifications((prev) => prev.filter((n) => n.id !== id));
  }, []);

  const dismissAllNotifications = useCallback(() => {
    setNotifications([]);
  }, []);

  const markTabNotificationsRead = useCallback((tabId: string) => {
    setNotifications((prev) =>
      prev.map((n) => (n.tabId === tabId ? { ...n, read: true } : n)),
    );
  }, []);

  // ── Tab Management ───────────────────────────────────────────────────────

  const injectTab = useCallback(
    async (
      pipelineId: number,
      options?: {
        id?: string;
        label?: string;
        icon?: string;
        makeActive?: boolean;
        initialData?: any;
        fromBackend?: boolean; // true = auto-uninject when task completes
      },
    ) => {
      const tabId = options?.id ?? `pipeline-${pipelineId}`;
      setInjectedTabs((prev) => {
        const existing = prev.find((t) => t.id === tabId);
        if (existing) {
          if (options?.makeActive) setActiveTabId(tabId);
          return prev;
        }
        return prev; // will create below
      });

      // Check if already injected
      const alreadyExists = injectedTabs.some((t) => t.id === tabId);
      if (alreadyExists) {
        if (options?.makeActive) setActiveTabId(tabId);
        return;
      }

      const newTab = await createDynamicTab(pipelineId, options);
      if (!newTab) {
        console.warn(`Pipeline ${pipelineId} has no UI module`);
        return;
      }

      if (options?.fromBackend) backendInjected.current.add(tabId);

      setInjectedTabs((prev) => [...prev, newTab]);
      if (options?.makeActive) setActiveTabId(newTab.id);
    },
    [injectedTabs],
  );

  const uninjectTab = useCallback((tabId: string) => {
    setInjectedTabs((prev) => {
      const tab = prev.find((t) => t.id === tabId);
      if (tab?.isCore) {
        console.warn("Cannot uninject core tab:", tabId);
        return prev;
      }
      return prev.filter((t) => t.id !== tabId);
    });
    backendInjected.current.delete(tabId);
    setActiveTabId((prev) => (prev === tabId ? "workspace" : prev));
  }, []);

  /**
   * Signal that a background tab needs user attention.
   * Flashes the tab and adds a notification.
   * Only auto-switches if both current and target are core tabs.
   */
  const notifyTabNeedsAttention = useCallback(
    (
      tabId: string,
      message: string,
      type: NotificationItem["type"] = "task",
    ) => {
      const isCoreActive = CORE_TAB_DEFINITIONS.some(
        (d) => d.id === activeTabId,
      );
      const isTargetCore = CORE_TAB_DEFINITIONS.some((d) => d.id === tabId);

      if (isCoreActive && isTargetCore) {
        // Both core: safe to auto-switch
        setActiveTabId(tabId);
      } else {
        // Flash tab + notification
        setInjectedTabs((prev) =>
          prev.map((t) =>
            t.id === tabId ? { ...t, needsAttention: true } : t,
          ),
        );
        addNotification(message, type, tabId);
      }
    },
    [activeTabId, addNotification],
  );

  // ── Tab Change ───────────────────────────────────────────────────────────

  const handleTabChange = useCallback(
    (tabId: string) => {
      setActiveTabId(tabId);
      // Clear attention flag when user manually switches to a tab
      setInjectedTabs((prev) =>
        prev.map((t) => (t.id === tabId ? { ...t, needsAttention: false } : t)),
      );
      markTabNotificationsRead(tabId);
    },
    [markTabNotificationsRead],
  );

  const handleTabClose = useCallback(
    (tabId: string) => {
      uninjectTab(tabId);
    },
    [uninjectTab],
  );

  // ── Task Badge on Tasks tab ──────────────────────────────────────────────

  useEffect(() => {
    const running = activeTasks.filter(
      (t) => t.status === "running" || t.status === "queued",
    ).length;

    setInjectedTabs((prev) =>
      prev.map((tab) =>
        tab.id === "tasks"
          ? { ...tab, badge: running > 0 ? running : undefined }
          : tab,
      ),
    );
  }, [activeTasks]);

  // ── Cross-view navigation (J2) ─────────────────────────────────────────────
  // The only piece missing before this batch: `navigateTo()` calls from every
  // "jump to X" fork (F3/F4/G1/G2/G5/E3/D5/etc.) had no listener actually
  // switching tabs. `task` targets also set the real store field
  // TaskDetailPanel already reads (`lastTaskId`) so the right task loads.

  useEffect(() => {
    return onNavigate((target) => {
      const tabId = tabForNavTarget(target);
      if (tabId) handleTabChange(tabId);
      if (target.kind === "task") {
        useOzoneStore.getState().setLastTaskId(target.taskId);
      }
    });
  }, [handleTabChange]);

  // ── Global API for backend/pipeline hooks ─────────────────────────────────

  useEffect(() => {
    (window as any).__ozoneThemeArea = {
      injectTab,
      uninjectTab,
      setActiveTab: (tabId: string) => handleTabChange(tabId),
      getInjectedTabs: () => injectedTabs,
      notifyTabNeedsAttention,
      addNotification,

      // Called by TaskManager/Orchestrator when a step has pipeline UI
      onStepActive: async (pipelineId: number, stepData: any) => {
        const hasUI = await hasPipelineUI(pipelineId);
        if (hasUI) {
          await injectTab(pipelineId, {
            makeActive: false, // don't auto-switch; notify instead
            initialData: stepData,
            fromBackend: true,
          });
          notifyTabNeedsAttention(
            `pipeline-${pipelineId}`,
            `Pipeline ${pipelineId} needs your input`,
            "task",
          );
        }
      },

      // Called when a backend-injected step completes
      onStepComplete: (pipelineId: number) => {
        const tabId = `pipeline-${pipelineId}`;
        const isCore = CORE_TAB_DEFINITIONS.some(
          (d) => d.pipelineId === pipelineId,
        );
        if (!isCore && backendInjected.current.has(tabId)) {
          uninjectTab(tabId);
        }
      },
    };

    return () => {
      delete (window as any).__ozoneThemeArea;
    };
  }, [
    injectTab,
    uninjectTab,
    injectedTabs,
    notifyTabNeedsAttention,
    addNotification,
    handleTabChange,
  ]);

  // ── Render ────────────────────────────────────────────────────────────────

  if (loading)
    return (
      <main className="theme-area theme-loading">
        <div className="loading-spinner" />
        <p>Loading...</p>
      </main>
    );

  return (
    <main className="theme-area">
      {/* Notification center floats over the tab bar */}
      <div className="theme-notification-anchor">
        <NotificationCenter
          notifications={notifications}
          onDismiss={dismissNotification}
          onDismissAll={dismissAllNotifications}
          onTabFocus={handleTabChange}
        />
      </div>

      {/* J3: unified search, same floating-toolbar convention as the
          notification center above — collapses to just an input when idle.
          `absolute` (not `fixed`) relative to .theme-area, matching
          .theme-notification-anchor exactly: .theme-area already sits below
          the 48px header bar, so `fixed`+viewport-relative `top` landed this
          inside the header band and overlapped the feature badges
          (Consciousness/P2P/ConnectedAgents) — found live, 2026-09-29. */}
      <div className="theme-search-anchor" style={{ position: "absolute", top: 0, right: 56, zIndex: 40 }}>
        <UnifiedSearch />
      </div>

      <HomeDashboard
        injectedTabs={injectedTabs}
        activeTabId={activeTabId}
        onTabChange={handleTabChange}
        onTabClose={handleTabClose}
      />
    </main>
  );
}
