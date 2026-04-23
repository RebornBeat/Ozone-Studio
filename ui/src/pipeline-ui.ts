/**
 * pipeline-ui.ts — Consolidated Pipeline UI System
 *
 * Replaces: DynamicPipelineUI.tsx + PipelineUILoader.tsx + pipeline-ui/index.ts
 *
 * Architecture:
 * - Registry is fetched from /pipeline/registry HTTP endpoint (direct, no binary)
 * - Component JS is fetched from /pipeline/ui-component HTTP endpoint (direct, no binary)
 * - Modules are cached in memory per session
 * - Shared state bus allows cross-tab communication
 */

import React, { useEffect, useRef, useState, useCallback } from 'react';
import { InjectedTab } from './themes/home_dashboard/HomeDashboard';

// ============================================================================
// Module Cache
// ============================================================================

const moduleCache = new Map<number, PipelineUIModule>();
let registryCache: PipelineRegistryEntry[] | null = null;

// ============================================================================
// Shared State Bus (cross-tab communication)
// ============================================================================

let sharedState: Record<string, any> = {};
const stateListeners = new Set<(state: Record<string, any>) => void>();

export function getSharedState(): Record<string, any> {
  return sharedState;
}

export function setSharedState(updates: Record<string, any>): void {
  sharedState = { ...sharedState, ...updates };
  stateListeners.forEach(fn => fn(sharedState));
}

export function subscribeToState(
  listener: (state: Record<string, any>) => void
): () => void {
  stateListeners.add(listener);
  return () => stateListeners.delete(listener);
}

// ============================================================================
// Types
// ============================================================================

export interface PipelineRegistryEntry {
  id: number;
  name: string;
  folder_name: string;
  category: string;
  has_ui: boolean;
  is_tab: boolean;
  description: string;
}

export interface PipelineUIModule {
  render: (
    container: HTMLElement,
    props: PipelineUIProps,
    React: typeof import('react'),
    ReactDOM: typeof import('react-dom/client')
  ) => (() => void) | void;
  meta?: { title?: string; icon?: string; version?: string };
  onActivate?: () => void;
  onDeactivate?: () => void;
}

export interface PipelineUIProps {
  pipelineId: number;
  executePipeline: (pipelineId: number, input: any) => Promise<any>;
  getSharedState: () => Record<string, any>;
  setSharedState: (updates: Record<string, any>) => void;
  subscribeToState: (listener: (state: Record<string, any>) => void) => () => void;
  onClose?: () => void;
  initialData?: any;
}

// ============================================================================
// Registry
// ============================================================================

export async function fetchPipelineRegistry(): Promise<PipelineRegistryEntry[]> {
  if (registryCache) return registryCache;

  try {
    // Prefer dedicated registry endpoint
    if (window.ozone?.pipeline?.getRegistry) {
      const result = await window.ozone.pipeline.getRegistry();
      if (result?.success && result?.registry) {
        registryCache = result.registry;
        return registryCache!;
      }
    }
  } catch (e) {
    console.warn('Failed to fetch pipeline registry:', e);
  }

  // Hardcoded fallback — always works even offline
  return [
    { id: 6, name: 'WorkspaceTab',  folder_name: 'workspace_tab',  category: 'core', has_ui: true, is_tab: true, description: '' },
    { id: 5, name: 'TaskManager',   folder_name: 'task_manager',   category: 'core', has_ui: true, is_tab: true, description: '' },
    { id: 7, name: 'LibraryTab',    folder_name: 'library_tab',    category: 'core', has_ui: true, is_tab: true, description: '' },
    { id: 8, name: 'SettingsTab',   folder_name: 'settings_tab',   category: 'core', has_ui: true, is_tab: true, description: '' },
  ];
}

export function clearRegistryCache(): void {
  registryCache = null;
}

export async function getPipelineInfo(id: number): Promise<PipelineRegistryEntry | null> {
  const reg = await fetchPipelineRegistry();
  return reg.find(p => p.id === id) ?? null;
}

// ============================================================================
// Module Loading
// ============================================================================

export async function loadPipelineUIModule(
  pipelineId: number
): Promise<PipelineUIModule | null> {
  if (moduleCache.has(pipelineId)) return moduleCache.get(pipelineId)!;

  try {
    if (window.ozone?.pipeline?.getUIComponent) {
      const result = await window.ozone.pipeline.getUIComponent(pipelineId);
      if (result?.component_js) {
        const mod = evalPipelineModule(result.component_js, pipelineId);
        if (mod) {
          moduleCache.set(pipelineId, mod);
          return mod;
        }
      }
    }
  } catch (e) {
    console.error(`Failed to load UI module for pipeline ${pipelineId}:`, e);
  }
  return null;
}

function evalPipelineModule(
  jsCode: string,
  pipelineId: number
): PipelineUIModule | null {
  try {
    const wrapper = { exports: {} as any };
    const fn = new Function('module', 'exports', 'pipelineId', jsCode);
    fn(wrapper, wrapper.exports, pipelineId);
    const result = wrapper.exports as any;
    if (typeof result.render === 'function') return result as PipelineUIModule;
    console.warn(`Pipeline ${pipelineId}: UI module missing render function`);
    return null;
  } catch (e) {
    console.error(`Pipeline ${pipelineId}: eval failed:`, e);
    return null;
  }
}

export async function hasPipelineUI(pipelineId: number): Promise<boolean> {
  if (moduleCache.has(pipelineId)) return true;
  const info = await getPipelineInfo(pipelineId);
  return info?.has_ui === true;
}

export function clearModuleCache(pipelineId?: number): void {
  if (pipelineId !== undefined) moduleCache.delete(pipelineId);
  else moduleCache.clear();
}

// ============================================================================
// Tab Definitions
// ============================================================================

export const CORE_TAB_DEFINITIONS = [
  { id: 'workspace', pipelineId: 6, label: 'Workspace', icon: '📁', order: 0 },
  { id: 'tasks',     pipelineId: 5, label: 'Tasks',     icon: '📋', order: 1 },
  { id: 'library',   pipelineId: 7, label: 'Library',   icon: '📚', order: 2 },
  { id: 'settings',  pipelineId: 8, label: 'Settings',  icon: '⚙️', order: 3 },
] as const;

export function getPipelineIcon(pipelineId: number): string {
  const icons: Record<number, string> = {
    5: '📋', 6: '📁', 7: '📚', 8: '⚙️',
    18: '🔍', 22: '🕸️', 30: '📎', 31: '🔗', 32: '📦', 37: '📜',
  };
  return icons[pipelineId] ?? '🔧';
}

// ============================================================================
// Tab Factories
// ============================================================================

function createDynamicTabComponent(pipelineId: number): React.ComponentType<any> {
  return function DynamicTab(props: any) {
    return React.createElement(DynamicPipelineUI, {
      pipelineId,
      initialData: props.initialData,
      onClose: props.onClose,
    });
  };
}

/** Create InjectedTab objects for the 4 core tabs. Always works offline. */
export function createCoreTabs(): InjectedTab[] {
  return CORE_TAB_DEFINITIONS.map(def => ({
    id: def.id,
    pipelineId: def.pipelineId,
    label: def.label,
    icon: def.icon,
    isCore: true,
    component: createDynamicTabComponent(def.pipelineId),
    closeable: false,
    needsAttention: false,
  }));
}

/** Create an InjectedTab for any non-core pipeline (async — checks registry). */
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
  const hasUI = await hasPipelineUI(pipelineId);
  if (!hasUI) return null;

  const info = await getPipelineInfo(pipelineId);
  const mod  = await loadPipelineUIModule(pipelineId);
  const isCore = CORE_TAB_DEFINITIONS.some(d => d.pipelineId === pipelineId);

  return {
    id: options?.id ?? `pipeline-${pipelineId}`,
    pipelineId,
    label: options?.label ?? mod?.meta?.title ?? info?.name ?? `Pipeline ${pipelineId}`,
    icon:  options?.icon  ?? mod?.meta?.icon  ?? getPipelineIcon(pipelineId),
    isCore,
    component: createDynamicTabComponent(pipelineId),
    props: { initialData: options?.initialData },
    closeable: !isCore,
    needsAttention: false,
  };
}

// ============================================================================
// DynamicPipelineUI React Component
// ============================================================================

interface DynamicPipelineUIProps {
  pipelineId: number;
  initialData?: any;
  onClose?: () => void;
}

export function DynamicPipelineUI({
  pipelineId,
  initialData,
  onClose,
}: DynamicPipelineUIProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const cleanupRef   = useRef<(() => void) | null>(null);
  const [loading, setLoading] = useState(true);
  const [error,   setError]   = useState<string | null>(null);

  const executePipeline = useCallback(async (pid: number, input: any) => {
    if (window.ozone?.pipeline?.execute) {
      return await window.ozone.pipeline.execute(pid, input);
    }
    throw new Error('Pipeline execution not available');
  }, []);

  useEffect(() => {
    let mounted = true;

    async function load() {
      if (!containerRef.current) return;
      try {
        const mod = await loadPipelineUIModule(pipelineId);
        if (!mounted) return;

        if (!mod) {
          setError(`No UI module found for pipeline ${pipelineId}`);
          setLoading(false);
          return;
        }

        const ReactLib  = await import('react');
        const ReactDOM  = await import('react-dom/client');

        const cleanup = mod.render(
          containerRef.current!,
          { pipelineId, executePipeline, getSharedState, setSharedState, subscribeToState, onClose, initialData },
          ReactLib,
          ReactDOM
        );
        if (typeof cleanup === 'function') cleanupRef.current = cleanup;
        mod.onActivate?.();
        setLoading(false);
      } catch (e: any) {
        if (mounted) {
          setError(e.message ?? 'Failed to load UI module');
          setLoading(false);
        }
      }
    }

    load();
    return () => {
      mounted = false;
      cleanupRef.current?.();
      cleanupRef.current = null;
    };
  }, [pipelineId, executePipeline, onClose, initialData]);

  if (loading) return (
    <div className="dynamic-pipeline-ui loading">
      <div className="loading-spinner" />
      <p>Loading...</p>
    </div>
  );

  if (error) return (
    <div className="dynamic-pipeline-ui error">
      <div className="ui-error"><span>⚠️</span><span>{error}</span></div>
    </div>
  );

  return <div ref={containerRef} className="dynamic-pipeline-ui-container" />;
}

export default DynamicPipelineUI;
