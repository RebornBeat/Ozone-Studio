/** Requested active sub-tab for the Files panel's `PanelShell` (e.g. a same-panel
 * "view history →" link jumping from Code to History). Same `useSyncExternalStore`
 * module pattern as `fileSelection.ts`'s `useSelectedFile`. `null` means "no request
 * yet — let PanelShell use its own default." */
import { useSyncExternalStore } from "react";

let requested: string | null = null;
const listeners = new Set<() => void>();

export function requestFilesTab(id: string): void {
  requested = id;
  listeners.forEach((l) => l());
}

export function useRequestedFilesTab(): [string | null, (id: string) => void] {
  const v = useSyncExternalStore(
    (cb) => {
      listeners.add(cb);
      return () => listeners.delete(cb);
    },
    () => requested,
  );
  return [v, requestFilesTab];
}
