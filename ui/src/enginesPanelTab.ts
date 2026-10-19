/** Requested active sub-tab for the Engines panel's `PanelShell` — same
 * `useSyncExternalStore` module pattern as `filesPanelTab.ts`'s
 * `useRequestedFilesTab` (that file's own doc comment explains the pattern;
 * not repeated here). `null` means "no request yet — let PanelShell use its
 * own default." */
import { useSyncExternalStore } from "react";

let requested: string | null = null;
const listeners = new Set<() => void>();

export function requestEnginesTab(id: string): void {
  requested = id;
  listeners.forEach((l) => l());
}

export function useRequestedEnginesTab(): [string | null, (id: string) => void] {
  const v = useSyncExternalStore(
    (cb) => {
      listeners.add(cb);
      return () => listeners.delete(cb);
    },
    () => requested,
  );
  return [v, requestEnginesTab];
}
