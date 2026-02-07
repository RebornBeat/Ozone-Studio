/**
 * Pipeline UI System - FULLY DYNAMIC
 * 
 * ALL pipelines use the same dynamic loading system!
 * NO COMPILED COMPONENTS - everything loads at runtime.
 * 
 * Pipeline info comes from the BACKEND REGISTRY - NO HARDCODING!
 * 
 * How to add a new pipeline UI (NO REBUILD NEEDED):
 * 1. Create pipelines/core/{pipeline_name}/ui/component.js
 * 2. Export a render function: module.exports = { render: (container, props, React, ReactDOM) => {...} }
 * 3. The frontend loads and executes it at runtime
 * 
 * Tab components (workspace, tasks, library, settings) also load from their
 * respective pipeline ui/ folders. These handle calling other pipelines
 * within their own UI context - they don't create separate tabs for those.
 * 
 * Example: WorkspaceTab has FileLink, URLLink, PackageLink buttons.
 * These call the respective pipelines but the UI stays within WorkspaceTab.
 * The pipelines don't need their own separate tabs.
 */

// Import dynamic UI system
import { 
  DynamicPipelineUI,
  loadPipelineUIModule,
  hasPipelineUI as checkHasPipelineUI,
  clearModuleCache,
  getSharedState,
  setSharedState,
  subscribeToState,
  PipelineUIModule,
  PipelineUIProps,
} from './DynamicPipelineUI';

import { 
  PipelineRegistryEntry,
  fetchPipelineRegistry,
  clearRegistryCache,
  getPipelineInfo,
  getPipelineName,
  getTabPipelineIds,
  getPipelinesWithUI,
  getPipelineIcon,
  CORE_TAB_DEFINITIONS,
  createCoreTabs,
  createDynamicTab,
  hasPipelineUI,
  listPipelineUIs,
  clearModuleCache as clearCache,
} from './PipelineUILoader';

// Tab definition for ThemeLoader injection
export interface TabDefinition {
  id: string;
  pipelineId: number;
  label: string;
  icon: string;
  isCore: boolean;
  order: number;
}

/**
 * Core tabs that are always loaded on startup.
 * ALL use dynamic loading - no compiled components.
 */
export const CORE_TABS: TabDefinition[] = CORE_TAB_DEFINITIONS.map((def, index) => ({
  id: def.id,
  pipelineId: def.pipelineId,
  label: def.label,
  icon: def.icon,
  isCore: true,
  order: def.order,
}));

/**
 * Create a tab definition for injection.
 */
export async function createTabDefinition(
  pipelineId: number,
  options?: {
    id?: string;
    label?: string;
    icon?: string;
    isCore?: boolean;
    order?: number;
  }
): Promise<TabDefinition> {
  const name = await getPipelineName(pipelineId);
  const isCore = CORE_TAB_DEFINITIONS.some(d => d.pipelineId === pipelineId);
  
  return {
    id: options?.id ?? `pipeline-${pipelineId}`,
    pipelineId,
    label: options?.label ?? name,
    icon: options?.icon ?? getPipelineIcon(pipelineId),
    isCore: options?.isCore ?? isCore,
    order: options?.order ?? 100,
  };
}

// Re-export everything
export { 
  // Dynamic UI system
  DynamicPipelineUI,
  loadPipelineUIModule,
  checkHasPipelineUI,
  clearModuleCache,
  getSharedState,
  setSharedState,
  subscribeToState,
  
  // Pipeline registry (queries backend)
  fetchPipelineRegistry,
  clearRegistryCache,
  getPipelineInfo,
  getPipelineName,
  getTabPipelineIds,
  getPipelinesWithUI,
  getPipelineIcon,
  CORE_TAB_DEFINITIONS,
  createCoreTabs,
  createDynamicTab,
  hasPipelineUI,
  listPipelineUIs,
};

// Re-export types
export type { PipelineUIModule, PipelineUIProps, PipelineRegistryEntry };
