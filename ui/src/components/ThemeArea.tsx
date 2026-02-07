/**
 * ThemeArea Component - §5.3 of the specification
 * 
 * The main content area that is ENTIRELY pipeline-driven.
 * 
 * Key principle: The ThemeArea does NOT contain hardcoded UI logic.
 * Instead, it renders what the ThemeLoaderPipeline provides.
 * 
 * Tab Injection Architecture:
 * - ThemeLoader (Pipeline #2) manages which tabs are displayed
 * - ALL tabs (including core) use dynamic JS loading - NO COMPILED COMPONENTS
 * - Core tabs are injected on startup, never uninjected
 * - Non-core tabs are injected during task execution, uninjected on completion
 * 
 * NO REBUILD NEEDED when adding new pipeline UIs!
 * Each pipeline's UI is a JS file loaded at runtime.
 */

import React, { useEffect, useState, useCallback } from 'react';
import { useOzoneStore } from '../services/store';

// Import theme container
import { HomeDashboard, InjectedTab } from '../themes/home_dashboard/HomeDashboard';

// Import pipeline UI system - ALL DYNAMIC, registry-based
import { 
  createCoreTabs,
  createDynamicTab,
  hasPipelineUI,
  getPipelineName,
  getPipelineIcon,
  CORE_TAB_DEFINITIONS,
} from '../pipeline-ui';

interface ThemeAreaProps {
  theme: string;
}

export function ThemeArea({ theme }: ThemeAreaProps) {
  const { 
    isConnected, 
    activeTasks,
    consciousnessEnabled,
    setConsciousnessEnabled 
  } = useOzoneStore();
  
  // Tab injection state
  const [injectedTabs, setInjectedTabs] = useState<InjectedTab[]>([]);
  const [activeTabId, setActiveTabId] = useState<string>('workspace');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Initialize core tabs on mount
  useEffect(() => {
    initializeCoreTabs();
  }, []);

  // Initialize core tabs - ALL use dynamic loading
  const initializeCoreTabs = useCallback(() => {
    const tabs = createCoreTabs();
    setInjectedTabs(tabs);
    setLoading(false);
  }, []);

  // Inject a new tab for any pipeline - FULLY DYNAMIC
  const injectTab = useCallback(async (
    pipelineId: number,
    options?: {
      id?: string;
      label?: string;
      icon?: string;
      makeActive?: boolean;
      initialData?: any;
    }
  ) => {
    // Check if already injected
    const tabId = options?.id || `pipeline-${pipelineId}`;
    const existingTab = injectedTabs.find(t => t.id === tabId);
    if (existingTab) {
      if (options?.makeActive) {
        setActiveTabId(existingTab.id);
      }
      return;
    }
    
    // Create dynamic tab
    const newTab = await createDynamicTab(pipelineId, options);
    if (newTab) {
      setInjectedTabs(prev => [...prev, newTab]);
      if (options?.makeActive) {
        setActiveTabId(newTab.id);
      }
    } else {
      console.warn(`Pipeline ${pipelineId} has no UI module`);
    }
  }, [injectedTabs]);

  // Uninject a tab (for non-core pipelines when task completes)
  const uninjectTab = useCallback((tabId: string) => {
    setInjectedTabs(prev => {
      const tab = prev.find(t => t.id === tabId);
      // Don't uninject core tabs
      if (tab?.isCore) {
        console.warn('Cannot uninject core tab:', tabId);
        return prev;
      }
      return prev.filter(t => t.id !== tabId);
    });
    
    // If the closed tab was active, switch to first tab
    if (activeTabId === tabId) {
      setActiveTabId(injectedTabs[0]?.id || 'workspace');
    }
  }, [activeTabId, injectedTabs]);

  // Handle tab change
  const handleTabChange = useCallback((tabId: string) => {
    setActiveTabId(tabId);
  }, []);

  // Handle tab close
  const handleTabClose = useCallback((tabId: string) => {
    uninjectTab(tabId);
  }, [uninjectTab]);

  // Update task badge on Tasks tab
  useEffect(() => {
    const runningCount = activeTasks.filter(t => 
      t.status === 'running' || t.status === 'queued'
    ).length;
    
    setInjectedTabs(prev => prev.map(tab => {
      if (tab.id === 'tasks') {
        return { ...tab, badge: runningCount > 0 ? runningCount : undefined };
      }
      return tab;
    }));
  }, [activeTasks]);

  // Expose injection functions globally for ThemeLoader and TaskManager hooks
  useEffect(() => {
    (window as any).__ozoneThemeArea = {
      injectTab,
      uninjectTab,
      setActiveTab: setActiveTabId,
      getInjectedTabs: () => injectedTabs,
      // Called by TaskManager when a step has pipeline UI
      onStepActive: async (pipelineId: number, stepData: any) => {
        const hasUI = await hasPipelineUI(pipelineId);
        if (hasUI) {
          await injectTab(pipelineId, {
            makeActive: true,
            initialData: stepData,
          });
        }
      },
      // Called by TaskManager when a step completes
      onStepComplete: (pipelineId: number) => {
        // Only uninject if not a core tab
        const isCore = CORE_TAB_DEFINITIONS.some(d => d.pipelineId === pipelineId);
        if (!isCore) {
          uninjectTab(`pipeline-${pipelineId}`);
        }
      },
    };
    
    return () => {
      delete (window as any).__ozoneThemeArea;
    };
  }, [injectTab, uninjectTab, injectedTabs]);

  // Loading state
  if (loading) {
    return (
      <main className="theme-area theme-loading">
        <div className="loading-spinner" />
        <p>Loading theme...</p>
      </main>
    );
  }

  // Error state
  if (error) {
    return (
      <main className="theme-area theme-error">
        <h2>Theme Error</h2>
        <p>{error}</p>
        <button onClick={() => window.location.reload()}>
          Reload
        </button>
      </main>
    );
  }

  // Render HomeDashboard with injected tabs
  return (
    <main className="theme-area">
      <HomeDashboard
        injectedTabs={injectedTabs}
        activeTabId={activeTabId}
        onTabChange={handleTabChange}
        onTabClose={handleTabClose}
      />
    </main>
  );
}
