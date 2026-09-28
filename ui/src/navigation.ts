/**
 * Cross-view navigation bus (J2). Views call navigateTo(); the app shell (J2/J4) subscribes with onNavigate()
 * to switch tabs and pass the target to the destination view. Deep links use real container ids only.
 */
export type NavTarget =
  | { kind: "tab"; tabId: string }
  | { kind: "graph-node"; projectId: number; nodeId: string }
  | { kind: "container"; containerId: number }
  | { kind: "task"; taskId: number }
  | { kind: "code-file"; projectId: number; path: string; line?: number };

type Handler = (t: NavTarget) => void;
const handlers = new Set<Handler>();
let pending: NavTarget | null = null;

export function navigateTo(target: NavTarget): void {
  pending = target;
  handlers.forEach((h) => h(target));
}
export function onNavigate(h: Handler): () => void {
  handlers.add(h);
  return () => handlers.delete(h);
}
/** Destination views call this on mount to pick up a target that arrived before they rendered. */
export function takePendingNavigation(kind: NavTarget["kind"]): NavTarget | null {
  if (pending && pending.kind === kind) {
    const p = pending;
    pending = null;
    return p;
  }
  return null;
}
