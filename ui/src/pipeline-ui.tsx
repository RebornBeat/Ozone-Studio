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

import React, { useEffect, useLayoutEffect, useRef, useState, useCallback } from 'react';
import { InjectedTab } from './themes/home_dashboard/HomeDashboard';
import SettingsPanel from './components/SettingsPanel';
import PipelinesPanel from './components/PipelinesPanel';
import MonitoringPanel from './components/MonitoringPanel';
import OrderPanel from './components/OrderPanel';
import ToolsPanel from './components/ToolsPanel';
import PairingPanel from './components/PairingPanel';
import GraphView from './components/GraphView';
import FabricPanel from './panels/FabricPanel';
import HierarchyPanel from './panels/HierarchyPanel';
import CapturePanel from './panels/CapturePanel';
import FilesPanel from './panels/FilesPanel';
import EnginesPanel from './panels/EnginesPanel';
import CoordinationPanel from './panels/CoordinationPanel';
import LivingNetworkPanel from './panels/LivingNetworkPanel';
import ProjectChatPanel from './components/ProjectChatPanel';
import ProjectWorkspaceViewer from './components/ProjectWorkspaceViewer';

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

  // Never spin forever: surface the backend's reason or time out.
  const withTimeout = <T,>(p: Promise<T>, ms: number, what: string) =>
    Promise.race([
      p,
      new Promise<never>((_, rej) =>
        setTimeout(() => rej(new Error(`${what} timed out after ${ms / 1000}s`)), ms),
      ),
    ]);

  try {
    if (window.ozone?.pipeline?.getUIComponent) {
      console.log(`[PipelineUI ${pipelineId}] fetching component.js via bridge…`);
      const result: any = await withTimeout(
        window.ozone.pipeline.getUIComponent(pipelineId),
        8000,
        `UI component fetch (pipeline ${pipelineId})`,
      );
      console.log(
        `[PipelineUI ${pipelineId}] response: success=${result?.success}, has_js=${!!result?.component_js}, error=${result?.error ?? "none"}`,
      );
      if (result?.component_js) {
        const mod = evalPipelineModule(result.component_js, pipelineId);
        if (mod) {
          moduleCache.set(pipelineId, mod);
          return mod;
        }
      }
      throw new Error(
        result?.error ?? `No UI module found for pipeline ${pipelineId}`
      );
    }
    console.warn(`[PipelineUI ${pipelineId}] getUIComponent bridge unavailable`);
  } catch (e) {
    console.error(`Failed to load UI module for pipeline ${pipelineId}:`, e);
    throw e;
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
  { id: 'workspace', pipelineId: 6,    label: 'Workspace', icon: '📁', order: 0 },
  // Reordered 2026-10-18 (W3.2): Files was order 12 (13th tab) — the real,
  // rich file system (WorkspaceBrowser/FileReferenceViewer/CodeViewer/
  // FileEditor/CodeCallGraph/VersionHistory, all confirmed real and
  // working) was effectively undiscoverable behind Order/Tasks/Library/
  // Settings/Monitor/Tools/Devices/Graph View/Fabric/Hierarchy/Raw
  // Thoughts. Moved to immediately after Workspace, which now also
  // navigates here directly when a linked file is clicked.
  { id: 'files-viewer',   pipelineId: 1007, label: 'Files',        icon: '📂', order: 1 },
  // Universal Order (docs/UNIVERSAL_ORDER_GUIDE.md) — "new chats start
  // inside this order" (§4): placed right after Workspace/Files, ahead
  // of Tasks/Library/Settings. Synthetic id, same convention as Monitor/
  // Tools/Devices below (host-side UI surface, not a dynamically-loaded
  // pipeline UI).
  { id: 'order',     pipelineId: 1011, label: 'Order',     icon: '🗓️', order: 2 },
  { id: 'tasks',     pipelineId: 5,    label: 'Tasks',     icon: '📋', order: 3 },
  { id: 'library',   pipelineId: 7,    label: 'Library',   icon: '📚', order: 4 },
  { id: 'settings',  pipelineId: 8,    label: 'Settings',  icon: '⚙️', order: 5 },
  // Promoted out of the old bottom-drawer toggle mechanism — these are
  // host-side UI surfaces, not dynamically-loaded pipeline UIs, so they get
  // synthetic ids in the custom range (index.json's own next_custom_id
  // starts at 1000) rather than colliding with a real pipeline id.
  { id: 'monitor',   pipelineId: 1000, label: 'Monitor',   icon: '📡', order: 6 },
  { id: 'tools',     pipelineId: 1001, label: 'Tools',     icon: '🔧', order: 7 },
  { id: 'devices',   pipelineId: 1002, label: 'Devices',   icon: '📱', order: 8 },
  // Context Viewer: Graph View (Batch C, fork C1) — real per-project
  // modality-graph node/edge rendering. Same synthetic-id convention as
  // monitor/tools/devices above (host-side UI surface, not a pipeline).
  { id: 'graph-view', pipelineId: 1003, label: 'Graph View', icon: '🕸️', order: 9 },
  // Context Viewer family + capture/engines/coordination surfaces (UI/UX fork plan Batches D-J).
  { id: 'fabric-view',    pipelineId: 1004, label: 'Fabric',       icon: '🧭', order: 10 },
  { id: 'hierarchy-view', pipelineId: 1005, label: 'Hierarchy',    icon: '🌳', order: 11 },
  { id: 'capture-viewer', pipelineId: 1006, label: 'Raw Thoughts', icon: '🧠', order: 12 },
  { id: 'engines',        pipelineId: 1008, label: 'Engines',      icon: '🧩', order: 13 },
  { id: 'coordination',   pipelineId: 1009, label: 'Coordination', icon: '🤝', order: 14 },
  { id: 'living-network', pipelineId: 1010, label: 'Network',      icon: '🌐', order: 15 },
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
  // Settings and Library host OUR config/registry surfaces directly —
  // single source for each, no duplicate panels or stale component.js.
  const nativeTabComponents: Record<string, React.ComponentType<any>> = {
    order: OrderPanel,
    settings: SettingsPanel,
    library: PipelinesPanel,
    monitor: MonitoringPanel,
    tools: ToolsPanel,
    devices: PairingPanel,
    'graph-view': GraphView,
    'fabric-view': FabricPanel,
    'hierarchy-view': HierarchyPanel,
    'capture-viewer': CapturePanel,
    'files-viewer': FilesPanel,
    'engines': EnginesPanel,
    'coordination': CoordinationPanel,
    'living-network': LivingNetworkPanel,
  };

  return CORE_TAB_DEFINITIONS.map(def => ({
    id: def.id,
    pipelineId: def.pipelineId,
    label: def.label,
    icon: def.icon,
    isCore: true,
    component: nativeTabComponents[def.id] ?? createDynamicTabComponent(def.pipelineId),
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
  // Stable node component.js's OWN ReactDOM.createRoot() is created on —
  // never re-created, never directly given a JSX ref, so our own
  // reconciliation above it can never reuse/relabel it. Only ITS PARENT
  // changes (see the layout effect below); component.js's root keeps the
  // exact same container it was created with for its entire lifetime, so
  // its eventual root.unmount() always finds its children right where it
  // left them, no matter where in the document this node currently sits.
  const mountNodeRef = useRef<HTMLDivElement | null>(null);
  if (!mountNodeRef.current) {
    mountNodeRef.current = document.createElement('div');
    mountNodeRef.current.className = 'pipeline-ui-mount';
  }
  const cleanupRef   = useRef<(() => void) | null>(null);
  const [loading, setLoading] = useState(true);
  const [error,   setError]   = useState<string | null>(null);

  // Project-scoped chat (2026-10-18): the Workspace tab (pipelineId 6) is a
  // separately-eval'd pipeline-UI module (workspace_tab/ui/component.js)
  // with no React import access, so it can't mount <ProjectChatPanel/>
  // itself — it just reports the selected project via the existing shared-
  // state bus (setSharedState({selectedProjectId, selectedProjectName,
  // selectedWorkspaceId})), and THIS component, living in the main React
  // bundle, renders the real panel as a sibling alongside the legacy
  // container. Deliberately scoped to pipelineId===6 only — no other
  // dynamic tab gets this treatment.
  const [projectChatScope, setProjectChatScope] = useState<{
    projectId: number;
    workspaceId?: number;
    projectName: string;
    // 2026-10-18 (cc-fork-unify-fe) — the real ZSEI graph container id for
    // this project (separate id space from `projectId` above, which is the
    // Workspace tab's own flat-store id). Read from the shared-state bus's
    // `selectedProjectZseiId` key (set by workspace_tab/ui/component.js's
    // handleSelectProject, straight off the backend's zsei_container_id
    // field). Can legitimately be null (container creation failed and
    // hasn't self-healed yet) — graph-native viewers already render a
    // "nothing selected" state for a null projectId, so this is passed
    // through as-is, never defaulted to the flat id.
    zseiProjectId: number | null;
  } | null>(null);

  useEffect(() => {
    if (pipelineId !== 6) return;
    const apply = (state: Record<string, any>) => {
      const pid = state.selectedProjectId;
      if (pid == null) {
        setProjectChatScope(null);
        return;
      }
      setProjectChatScope({
        projectId: pid,
        workspaceId: state.selectedWorkspaceId ?? undefined,
        projectName: state.selectedProjectName ?? `Project ${pid}`,
        zseiProjectId: state.selectedProjectZseiId ?? null,
      });
    };
    apply(getSharedState());
    return subscribeToState(apply);
  }, [pipelineId]);

  const executePipeline = useCallback(async (pid: number, input: any): Promise<any> => {
    if (window.ozone?.pipeline?.execute) {
      return await window.ozone.pipeline.execute(pid, input);
    }
    throw new Error('Pipeline execution not available');
  }, []);

  // REAL BUG (2026-10-18, found via live DOM inspection, not guessed): when
  // `projectChatScope` flips from null to populated, the JSX this component
  // RETURNS changes shape — single plain div (no children, IS containerRef's
  // target) becomes a 3-child wrapper (NEW className, with containerRef
  // moved onto a child). React sees the same element type ('div') at the
  // same tree position in both cases and REUSES the same physical DOM node
  // for the wrapper, just updating its className and appending the 3 new
  // children — it does not know or care that a FOREIGN React root
  // (component.js's `ReactDOM.createRoot`) had already appended its own
  // content into that node earlier. component.js's real content (toolbar/
  // stats/link list) was permanently stranded as an untracked extra child
  // of the outer wrapper — confirmed directly against the live DOM.
  //
  // FIRST FIX ATTEMPT (reverted): adding this to the mount effect's deps to
  // force a clean unmount+remount on every shape change. Wrong — it
  // destroys component.js's OWN internal React state (selected workspace,
  // selected project, the whole navigation stack) on every single project
  // click, since that's exactly when this flips, making project selection
  // look like it "doesn't load" (it resets back to the top-level view
  // instead of opening the project). component.js's root must NOT be torn
  // down just because the ancestor wrapper's shape changed.
  //
  // REAL FIX: leave the mount effect (and component.js's own React state)
  // completely alone — relocate a DOM node via plain `appendChild` instead
  // of asking React to redo any work. A React root doesn't care where its
  // container sits in the wider document, so this is invisible to
  // component.js's own React tree and preserves every bit of its state.
  //
  // SECOND ATTEMPT (also wrong, caught before shipping): moving whichever
  // child of the old wrapper didn't match React's own expected sibling
  // classes. That did relocate the right content, but component.js's React
  // root was created with `containerRef.current` ITSELF (at mount time) as
  // its container — moving a CHILD of that container elsewhere leaves the
  // root's container correct but strands the relationship the other way on
  // the next `root.unmount()` (pipeline switch/unmount): React's unmount
  // tries to remove its children from the container node it was created
  // with, and if this effect already moved those children out from under
  // it, that remove throws `NotFoundError`.
  //
  // FINAL FIX: add one more layer of indirection. component.js's root is
  // created on `mountNodeRef.current`, a plain node WE create once and
  // never recreate — never the JSX ref target itself, so our own
  // reconciliation can never reuse/relabel it out from under component.js.
  // This effect's only job is to keep that stable node attached as a child
  // of whatever `containerRef.current` currently is. component.js's root
  // keeps the exact same container for its entire lifetime (mountNodeRef's
  // children never move), so `root.unmount()` always finds things right
  // where it left them, regardless of where mountNodeRef itself currently
  // sits in the wider document.
  // `useLayoutEffect` (not `useEffect`) so this runs synchronously right
  // after the DOM commit, before paint — no visible flash in the wrong spot.
  useLayoutEffect(() => {
    if (!containerRef.current || !mountNodeRef.current) return;
    if (mountNodeRef.current.parentNode !== containerRef.current) {
      containerRef.current.appendChild(mountNodeRef.current);
    }
  });

  useEffect(() => {
    let mounted = true;

    async function load() {
      console.log(`[PipelineUI ${pipelineId}] loading module…`);
      try {
        const mod = await loadPipelineUIModule(pipelineId);
        if (!mounted) return;

        if (!mod) {
          console.warn(`[PipelineUI ${pipelineId}] no module returned`);
          setError(`No UI module found for pipeline ${pipelineId}`);
          setLoading(false);
          return;
        }

        const ReactLib = await import("react");
        const ReactDOM = await import("react-dom/client");
        if (!mounted) return;

        // Content mounts only after loading flips false — wait one frame,
        // then render into the now-mounted container.
        setLoading(false);
        await new Promise((r) => requestAnimationFrame(r));
        await new Promise((r) => setTimeout(r, 0));
        if (!mounted) return;
        if (!containerRef.current) {
          console.warn(`[PipelineUI ${pipelineId}] container not mounted after load`);
          return;
        }

        console.log(`[PipelineUI ${pipelineId}] rendering module…`);
        const cleanup = mod.render(
          mountNodeRef.current!,
          {
            pipelineId,
            executePipeline,
            getSharedState,
            setSharedState,
            subscribeToState,
            onClose,
            initialData,
          },
          ReactLib,
          ReactDOM,
        );
        console.log(`[PipelineUI ${pipelineId}] render complete`);
        if (typeof cleanup === "function") cleanupRef.current = cleanup;
        mod.onActivate?.();
      } catch (e: any) {
        if (mounted) {
          console.error(`[PipelineUI ${pipelineId}] load failed:`, e);
          setError(e.message ?? "Failed to load UI module");
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

  if (projectChatScope) {
    // Stacked layout only while a project is selected in the Workspace tab:
    // the legacy container (toolbar/stats/link-management list) shrinks to
    // its natural height on top, the real project-scoped Files/Fabric/
    // Hierarchy viewer takes the dominant middle space, and the
    // independent project chat docks as a bar along the bottom.
    // REVERTED (2026-10-18, operator-reported): briefly swapped this and
    // the viewer region's order — operator said that wasn't what they
    // meant and asked for a literal revert. Back to original order;
    // follow-up question pending on the exact final layout they want.
    return (
      <div className="dynamic-pipeline-ui-with-chat">
        <div ref={containerRef} className="dynamic-pipeline-ui-container dynamic-pipeline-ui-container-top" />
        {/* LAYOUT FIX (2026-10-18, operator-reported): this region is the
            DOMINANT middle area — Files/Fabric/Hierarchy/Graph content must
            fill and scroll HERE, above the chat bar, not flow below it.
            Without an explicit flex sizing the viewer took natural height
            (short content = blank gap, long content = overflow-hidden
            cutoff). */}
        <div className="project-viewer-region">
          <ProjectWorkspaceViewer
            projectId={projectChatScope.projectId}
            zseiProjectId={projectChatScope.zseiProjectId}
          />
        </div>
        <ProjectChatPanel
          projectId={projectChatScope.projectId}
          workspaceId={projectChatScope.workspaceId}
          projectName={projectChatScope.projectName}
        />
      </div>
    );
  }

  return <div ref={containerRef} className="dynamic-pipeline-ui-container" />;
}

export default DynamicPipelineUI;
