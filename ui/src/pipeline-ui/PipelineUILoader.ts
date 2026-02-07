/**
 * PipelineUILoader - Runtime JS Module Loading System
 *
 * THIS IS THE KEY TO NOT REBUILDING THE FRONTEND!
 *
 * Pipeline info comes from the BACKEND REGISTRY via API.
 * NO HARDCODED PIPELINE IDS HERE - query the registry!
 *
 * How it works:
 * 1. When a pipeline with UI is activated, ThemeLoader calls this
 * 2. This fetches the JS module from the pipeline's ui/component.js
 * 3. DynamicPipelineUI executes and renders it - NO REBUILD REQUIRED
 *
 * Pipeline UI Storage (in backend):
 *   pipelines/core/{pipeline_name}/
 *     main.rs (or .py, .js)
 *     ui/
 *       component.js    <- UI code (fully dynamic!)
 */

import React from 'react';
import { InjectedTab } from '../themes/home_dashboard/HomeDashboard';
import {
  DynamicPipelineUI,
  loadPipelineUIModule,
  hasPipelineUI as checkHasPipelineUI,
  clearModuleCache,
  PipelineUIModule,
} from './DynamicPipelineUI';

// Cache for registry data from backend
let registryCache: PipelineRegistryEntry[] | null = null;

/**
 * Pipeline registry entry from backend
 */
export interface PipelineRegistryEntry {
  id: number;
  name: string;
  folder_name: string;
  category: string;
  has_ui: boolean;
  is_tab: boolean;
  description: string;
}

/**
 * Fetch the full pipeline registry from backend
 * This is the SOURCE OF TRUTH - no hardcoding!
 */
export async function fetchPipelineRegistry(): Promise<PipelineRegistryEntry[]> {
  if (registryCache) {
    return registryCache;
  }

  try {
    // v0.4.0: Try using the dedicated registry endpoint first
    if (window.ozone?.pipeline?.getRegistry) {
      const result = await window.ozone.pipeline.getRegistry();
      if (result?.success && result?.registry) {
        registryCache = result.registry;
        return registryCache;
      }
    }

    // Fallback to pipeline execute
    if (window.ozone?.pipeline?.execute) {
      const result = await window.ozone.pipeline.execute(2, {
        action: 'GetPipelineRegistry',
      });
      if (result?.registry) {
        registryCache = result.registry;
        return registryCache;
      }
    }
  } catch (e) {
    console.warn('Failed to fetch pipeline registry:', e);
  }

  // Fallback to hardcoded core tabs if backend unavailable
  return [
    { id: 6, name: 'WorkspaceTab', folder_name: 'workspace_tab', category: 'core', has_ui: true, is_tab: true, description: 'Workspace management' },
    { id: 5, name: 'TaskManager', folder_name: 'task_manager', category: 'core', has_ui: true, is_tab: true, description: 'Task management' },
    { id: 7, name: 'LibraryTab', folder_name: 'library_tab', category: 'core', has_ui: true, is_tab: true, description: 'Library' },
    { id: 8, name: 'SettingsTab', folder_name: 'settings_tab', category: 'core', has_ui: true, is_tab: true, description: 'Settings' },
  ];
}

/**
 * Clear registry cache (call when pipelines change)
 */
export function clearRegistryCache() {
  registryCache = null;
}

/**
 * Get pipeline info by ID from registry
 */
export async function getPipelineInfo(pipelineId: number): Promise<PipelineRegistryEntry | null> {
  const registry = await fetchPipelineRegistry();
  return registry.find(p => p.id === pipelineId) || null;
}

/**
 * Get pipeline name from ID
 */
export async function getPipelineName(pipelineId: number): Promise<string> {
  const info = await getPipelineInfo(pipelineId);
  return info?.name || `Pipeline${pipelineId}`;
}

/**
 * Get all tab pipeline IDs (pipelines that should be main tabs)
 */
export async function getTabPipelineIds(): Promise<number[]> {
  const registry = await fetchPipelineRegistry();
  return registry.filter(p => p.is_tab).map(p => p.id);
}

/**
 * Get all pipelines with UIs
 */
export async function getPipelinesWithUI(): Promise<PipelineRegistryEntry[]> {
  const registry = await fetchPipelineRegistry();
  return registry.filter(p => p.has_ui);
}

/**
 * Default icons for pipelines (client-side aesthetic only)
 */
export function getPipelineIcon(pipelineId: number): string {
  const icons: Record<number, string> = {
    6: '📁',   // Workspace
    7: '📚',   // Library
    8: '⚙️',   // Settings
    5: '📋',   // TaskManager
    18: '🔍',  // CodeAnalysis
    22: '🕸️',  // GraphVisualization
    30: '📎',  // FileLink
    31: '🔗',  // URLLink
    32: '📦',  // PackageLink
    37: '📜',  // LogViewer
  };
  return icons[pipelineId] || '🔧';
}

/**
 * Core tab definitions - fetched from registry, not hardcoded
 */
export const CORE_TAB_DEFINITIONS: Array<{
  id: string;
  pipelineId: number;
  label: string;
  icon: string;
  order: number;
}> = [
  { id: 'workspace', pipelineId: 6, label: 'Workspace', icon: '📁', order: 0 },
  { id: 'tasks', pipelineId: 5, label: 'Tasks', icon: '📋', order: 1 },
  { id: 'library', pipelineId: 7, label: 'Library', icon: '📚', order: 2 },
  { id: 'settings', pipelineId: 8, label: 'Settings', icon: '⚙️', order: 3 },
];

/**
 * Create a React component wrapper for a dynamic pipeline UI
 */
function createDynamicTabComponent(pipelineId: number): React.ComponentType<any> {
  return function DynamicTab(props: any) {
    return (
      <DynamicPipelineUI>
        pipelineId={pipelineId}
        initialData={props.initialData}
        onClose={props.onClose}
      </>
    );
  };
}

/**
 * Create InjectedTab objects for core tabs
 * ALL tabs are dynamic - they load their JS modules at runtime
 */
export function createCoreTabs(): InjectedTab[] {
  return CORE_TAB_DEFINITIONS.map(def => ({
    id: def.id,
    pipelineId: def.pipelineId,
    label: def.label,
    icon: def.icon,
    isCore: true,
    component: createDynamicTabComponent(def.pipelineId),
    closeable: false,
  }));
}

/**
 * Create an InjectedTab for any pipeline
 * This is used for dynamic injection during task execution
 */
export async function createDynamicTab(
  pipelineId: number,
  options?: {
    id?: string;
    label?: string;
    icon?: string;
    makeActive?: boolean;
    initialData?: any;
  }
): Promise<InjectedTab | null> {
  // Check if pipeline has UI
  const hasUI = await checkHasPipelineUI(pipelineId);
  if (!hasUI) {
    return null;
  }

  // Get metadata from registry
  const info = await getPipelineInfo(pipelineId);

  // Get metadata from module if available
  const module = await loadPipelineUIModule(pipelineId);
  const meta = module?.meta;

  // Check if this is a core tab
  const isCore = CORE_TAB_DEFINITIONS.some(d => d.pipelineId === pipelineId);

  return {
    id: options?.id || `pipeline-${pipelineId}`,
    pipelineId,
    label: options?.label || meta?.title || info?.name || `Pipeline ${pipelineId}`,
    icon: options?.icon || meta?.icon || getPipelineIcon(pipelineId),
    isCore,
    component: createDynamicTabComponent(pipelineId),
    props: { initialData: options?.initialData },
    closeable: !isCore,
  };
}

/**
 * Check if a pipeline has a UI
 */
export async function hasPipelineUI(pipelineId: number): Promise<boolean> {
  return checkHasPipelineUI(pipelineId);
}

/**
 * Get list of all pipelines that have UIs
 * Fetches from registry via ThemeLoader
 */
export async function listPipelineUIs(): Promise<PipelineRegistryEntry[]> {
  return getPipelinesWithUI();
}

// Re-export
export { clearModuleCache };
export type { PipelineUIModule };
