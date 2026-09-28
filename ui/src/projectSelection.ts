/** One shared "current project" for every Context-Viewer-family panel. Persists to localStorage (best effort). */
import { useSyncExternalStore } from "react";
import { zseiQuery } from "./ozoneClient";

const KEY = "ozone_selected_project";
let selected: number | null = (() => {
  try {
    const v = localStorage.getItem(KEY);
    return v ? Number(v) : null;
  } catch {
    return null;
  }
})();
const listeners = new Set<() => void>();

export function setSelectedProject(id: number | null): void {
  selected = id;
  try {
    if (id === null) localStorage.removeItem(KEY);
    else localStorage.setItem(KEY, String(id));
  } catch {
    /* non-fatal */
  }
  listeners.forEach((l) => l());
}
export function useSelectedProject(): [number | null, (id: number | null) => void] {
  const id = useSyncExternalStore(
    (cb) => {
      listeners.add(cb);
      return () => listeners.delete(cb);
    },
    () => selected,
  );
  return [id, setSelectedProject];
}

export interface ProjectOption {
  id: number;
  name: string;
  workspaceId: number;
  workspaceName: string;
}
let optionsPromise: Promise<ProjectOption[]> | null = null;
export function loadProjectOptions(force = false): Promise<ProjectOption[]> {
  if (force || !optionsPromise) {
    optionsPromise = (async () => {
      const userId = (window as any).ozone?.auth?.getCurrentUserId?.() ?? 1;
      const ws = await zseiQuery<any>({ GetUserWorkspaces: { user_id: userId } });
      const out: ProjectOption[] = [];
      for (const wsId of ws?.Containers ?? []) {
        const wc = (await zseiQuery<any>({ GetContainer: { container_id: wsId } }))?.Container;
        if (!wc) continue;
        const wsName: string = wc.local_state?.metadata?.name ?? `Workspace ${wsId}`;
        for (const pid of wc.global_state?.child_ids ?? []) {
          const pc = (await zseiQuery<any>({ GetContainer: { container_id: pid } }))?.Container;
          if (!pc || pc.local_state?.metadata?.container_type !== "Project") continue;
          out.push({ id: pid, name: pc.local_state?.metadata?.name ?? `Project ${pid}`, workspaceId: wsId, workspaceName: wsName });
        }
      }
      return out;
    })();
    optionsPromise.catch(() => {
      optionsPromise = null;
    });
  }
  return optionsPromise;
}
