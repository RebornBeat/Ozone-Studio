/** The file/reference currently open in the Files panel (set by browser/reference views, read by code/editor/math/history views). */
import { useSyncExternalStore } from "react";

export interface SelectedFile {
  projectId: number;
  containerId: number;
  kind: "file" | "url" | "package";
  name: string;
  path?: string;
  modality?: string;
}
let selected: SelectedFile | null = null;
const listeners = new Set<() => void>();

export function setSelectedFile(f: SelectedFile | null): void {
  selected = f;
  listeners.forEach((l) => l());
}
export function useSelectedFile(): [SelectedFile | null, (f: SelectedFile | null) => void] {
  const f = useSyncExternalStore(
    (cb) => {
      listeners.add(cb);
      return () => listeners.delete(cb);
    },
    () => selected,
  );
  return [f, setSelectedFile];
}
