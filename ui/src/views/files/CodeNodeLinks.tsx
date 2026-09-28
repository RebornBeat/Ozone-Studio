/**
 * F4 — code viewer jump-to-node mapping.
 *
 * Maps the line ranges of a source file to the REAL code-modality graph nodes
 * that cover them, so F3's CodeViewer can highlight a function/class/import
 * and navigate to it in the Graph View.
 *
 * Real sources this is built against (all confirmed by reading them):
 *  - assets/pipelines/modalities/code/main.rs, `build_code_graph` (~2612-2790):
 *      * Function / Class / Import nodes carry `position: CodePosition
 *        { file_path, start_line, end_line, start_column?, end_column? }`.
 *        Lines are 1-based (`line_num + 1` in the analyzer). Import is a
 *        single line (start_line == end_line == import.line).
 *      * The File node has `position: None`. Its path lives in `name`
 *        (= analysis.file_path) and `properties.line_count` is its real
 *        length. So a File node is a whole-file span 1..line_count, which is
 *        how it is represented here.
 *      * `file_path` is whatever the caller passed to the analyzer as
 *        `analysis.file_path` — `""` when none was given. Real graphs on disk
 *        (zsei_data/graphs/code_*.json) store repo-relative paths such as
 *        `src/auth/a.rs` and `scripts/report.py`; a file linked through
 *        file_link stores `path` exactly as the user supplied it (often
 *        absolute, assets/pipelines/general/file_link/main.rs
 *        `create_file_ref`). Both forms therefore occur and must interoperate.
 *  - ui/src/graphViewData.ts `loadGraphData(projectId, cb, {modality:"code"})`:
 *    real code graph nodes as GraphViewNode; returns a dispose fn and
 *    re-fetches on graph_event frames (invalidate-and-refetch).
 *
 * Matching rules (the false-positive guard matters — two files can share a
 * basename, and the same file can be re-analysed into several graph containers):
 *  1. Paths are normalised (backslashes, `./`, `//`, `.`/`..` segments).
 *  2. An empty path never matches anything.
 *  3. Exact normalised equality wins outright.
 *  4. Otherwise the shorter RELATIVE path must be a suffix of the longer one at
 *     a `/` boundary (`auth/a.rs` matches `src/auth/a.rs`; `a.rs` does NOT
 *     match `data.rs`). Two different absolute paths never suffix-match.
 *  5. If a suffix match is satisfied by more than one distinct stored path the
 *     result is AMBIGUOUS: no links are returned and the paths are reported in
 *     the status, rather than guessing which file was meant.
 *  6. If the same path appears in several graph containers (re-analysis), only
 *     the newest container (highest container id) is used, so stale line
 *     ranges from an older analysis never leak in. Container ids are allocated
 *     increasing — assumption noted, not verified per-container timestamps
 *     (code graph wrappers carry no `created_at`).
 *  7. Whole-file File nodes are dropped when anything narrower matches, and
 *     used only when they are the sole match.
 */
import { useCallback, useMemo, useSyncExternalStore } from "react";
import { loadGraphData } from "../../graphViewData";
import type { GraphViewData, GraphViewNode, GraphViewStatus } from "../../graphViewTypes";

export interface CodeNodeLink {
  startLine: number;
  endLine: number;
  /** GraphViewNode.id of the real function/class node covering these lines. */
  nodeId: string;
  nodeType: string;
  label: string;
}

export interface CodeNodeLinksStatus {
  /** True only while there is no data yet (a background refresh keeps the last data). */
  loading: boolean;
  /** Last fetch error, if any (links from the last good fetch may still be returned). */
  error: string | null;
  /** True when at least one real code ModalityGraph container exists for the project. */
  hasCodeGraph: boolean;
  /** True when `filePath` matched at least one real node. */
  matchedFile: boolean;
  /** Distinct stored paths that suffix-matched `filePath` when the match was ambiguous. */
  ambiguousPaths: string[];
}

// ─────────────────────────────────────────────────────────────────────────
// Pure logic (exported so it can be validated against real graph files)
// ─────────────────────────────────────────────────────────────────────────

export function isAbsolutePath(p: string): boolean {
  return p.startsWith("/") || /^[A-Za-z]:\//.test(p);
}

/** Normalise separators, `./`, duplicate slashes and `.`/`..` segments. Does not lower-case. */
export function normalizePath(p: string): string {
  const s = p.trim().replace(/\\/g, "/");
  if (s === "") return "";
  const abs = isAbsolutePath(s);
  const out: string[] = [];
  for (const seg of s.split("/")) {
    if (seg === "" || seg === ".") continue;
    if (seg === "..") {
      if (out.length > 0 && out[out.length - 1] !== "..") out.pop();
      else if (!abs) out.push("..");
      continue;
    }
    out.push(seg);
  }
  const joined = out.join("/");
  return abs && !/^[A-Za-z]:/.test(joined) ? `/${joined}` : joined;
}

export type PathMatch = "exact" | "suffix" | "none";

/** How a stored graph path relates to the path open in the viewer. */
export function pathsMatch(stored: string, viewer: string): PathMatch {
  const a = normalizePath(stored);
  const b = normalizePath(viewer);
  if (a === "" || b === "") return "none";
  if (a === b) return "exact";
  const [long, short] = a.length >= b.length ? [a, b] : [b, a];
  if (isAbsolutePath(short)) return "none"; // two different absolute paths never suffix-match
  return long.endsWith(`/${short}`) ? "suffix" : "none";
}

interface NodeSpan {
  node: GraphViewNode;
  storedPath: string;
  start: number;
  end: number;
  wholeFile: boolean;
}

function asPositive(v: unknown): number | null {
  return typeof v === "number" && Number.isFinite(v) && v >= 1 ? v : null;
}

/** Real path + line span of a code node, or null when it has neither (e.g. malformed). */
function spanOf(node: GraphViewNode): NodeSpan | null {
  const raw = node.raw as unknown as {
    name?: unknown;
    position?: { file_path?: unknown; start_line?: unknown; end_line?: unknown } | null;
    properties?: Record<string, unknown>;
  };
  const pos = raw.position;
  if (pos && typeof pos === "object" && typeof pos.file_path === "string") {
    const start = asPositive(pos.start_line);
    const endRaw = asPositive(pos.end_line);
    if (start === null) return null;
    const end = endRaw !== null && endRaw >= start ? endRaw : start;
    return { node, storedPath: pos.file_path, start, end, wholeFile: false };
  }
  if (node.nodeType === "File" && typeof raw.name === "string") {
    // File nodes have no position; their path is `name`, their length `properties.line_count`.
    const lineCount = asPositive(raw.properties?.line_count);
    return { node, storedPath: raw.name, start: 1, end: lineCount ?? Number.MAX_SAFE_INTEGER, wholeFile: true };
  }
  return null;
}

export interface CodeNodeLinkResult {
  links: CodeNodeLink[];
  /** Distinct stored paths that suffix-matched when the match was ambiguous (links is then empty). */
  ambiguousPaths: string[];
}

/** Build the links for `filePath` from real code-graph nodes. Pure. */
export function buildCodeNodeLinks(nodes: GraphViewNode[], filePath: string): CodeNodeLinkResult {
  if (normalizePath(filePath) === "") return { links: [], ambiguousPaths: [] };

  const exact: NodeSpan[] = [];
  const suffix: NodeSpan[] = [];
  for (const n of nodes) {
    if (n.modality !== "code") continue;
    const s = spanOf(n);
    if (!s) continue;
    const m = pathsMatch(s.storedPath, filePath);
    if (m === "exact") exact.push(s);
    else if (m === "suffix") suffix.push(s);
  }

  let chosen: NodeSpan[];
  if (exact.length > 0) {
    chosen = exact;
  } else {
    const distinct = Array.from(new Set(suffix.map((s) => normalizePath(s.storedPath))));
    if (distinct.length > 1) return { links: [], ambiguousPaths: distinct.sort() };
    chosen = suffix;
  }
  if (chosen.length === 0) return { links: [], ambiguousPaths: [] };

  // Same file analysed into several graph containers: keep only the newest one.
  const newest = Math.max(...chosen.map((s) => s.node.sourceContainerId));
  chosen = chosen.filter((s) => s.node.sourceContainerId === newest);

  const narrow = chosen.filter((s) => !s.wholeFile);
  const use = narrow.length > 0 ? narrow : chosen;

  const links = use
    .map((s) => ({
      startLine: s.start,
      endLine: s.end,
      nodeId: s.node.id,
      nodeType: s.node.nodeType,
      label: s.node.label,
    }))
    // Largest span first ⇒ innermost node last, so nested functions win.
    .sort((a, b) => b.endLine - b.startLine - (a.endLine - a.startLine) || a.startLine - b.startLine || a.nodeId.localeCompare(b.nodeId));
  return { links, ambiguousPaths: [] };
}

/** Innermost link covering `line` (1-based), or null. On equal spans the later link wins. */
export function nodeAtLine(links: CodeNodeLink[], line: number): CodeNodeLink | null {
  let best: CodeNodeLink | null = null;
  for (const l of links) {
    if (line < l.startLine || line > l.endLine) continue;
    if (best === null || l.endLine - l.startLine <= best.endLine - best.startLine) best = l;
  }
  return best;
}

/** Every link covering `line`, outermost → innermost. */
export function linksAtLine(links: CodeNodeLink[], line: number): CodeNodeLink[] {
  return links.filter((l) => line >= l.startLine && line <= l.endLine);
}

// ─────────────────────────────────────────────────────────────────────────
// Shared per-project subscription (one loadGraphData per project, ref-counted,
// so the links hook and the status hook don't each open their own fetch).
// ─────────────────────────────────────────────────────────────────────────

interface Snapshot {
  data: GraphViewData | null;
  loading: boolean;
  error: string | null;
  hasCodeGraph: boolean;
}

const INITIAL: Snapshot = { data: null, loading: true, error: null, hasCodeGraph: false };
const IDLE: Snapshot = { data: null, loading: false, error: null, hasCodeGraph: false };

interface Entry {
  snapshot: Snapshot;
  refs: number;
  listeners: Set<() => void>;
  dispose: (() => void) | null;
}
const entries = new Map<number, Entry>();

function applyStatus(prev: Snapshot, status: GraphViewStatus): Snapshot {
  switch (status.kind) {
    case "loading":
      // A refetch after a graph event keeps the last good data instead of flickering empty.
      return prev.data ? { ...prev, loading: false, error: null } : { ...INITIAL };
    case "error":
      return { data: prev.data, loading: false, error: status.message, hasCodeGraph: prev.hasCodeGraph };
    case "empty":
      return { data: null, loading: false, error: null, hasCodeGraph: false };
    case "ready":
      return { data: status.data, loading: false, error: null, hasCodeGraph: status.data.sourceContainers.length > 0 };
  }
}

function subscribeProject(projectId: number, cb: () => void): () => void {
  let entry = entries.get(projectId);
  if (!entry) {
    const created: Entry = { snapshot: INITIAL, refs: 0, listeners: new Set(), dispose: null };
    entries.set(projectId, created);
    created.dispose = loadGraphData(
      projectId,
      (status) => {
        created.snapshot = applyStatus(created.snapshot, status);
        created.listeners.forEach((l) => l());
      },
      { modality: "code" },
    );
    entry = created;
  }
  entry.refs += 1;
  entry.listeners.add(cb);
  const owned = entry;
  return () => {
    owned.listeners.delete(cb);
    owned.refs -= 1;
    if (owned.refs <= 0) {
      owned.dispose?.();
      if (entries.get(projectId) === owned) entries.delete(projectId);
    }
  };
}

function useCodeGraphSnapshot(projectId: number | null): Snapshot {
  const subscribe = useCallback(
    (cb: () => void) => (projectId === null ? () => undefined : subscribeProject(projectId, cb)),
    [projectId],
  );
  const getSnapshot = useCallback(
    () => (projectId === null ? IDLE : entries.get(projectId)?.snapshot ?? INITIAL),
    [projectId],
  );
  return useSyncExternalStore(subscribe, getSnapshot);
}

const NO_LINKS: CodeNodeLink[] = [];

/**
 * Real graph nodes covering `filePath`, innermost-last. Empty while loading, when the project has no
 * code graph, when the file isn't in it, or when the path is ambiguous — use `useCodeNodeLinksStatus`
 * to tell those apart.
 */
export function useCodeNodeLinks(projectId: number | null, filePath: string | undefined): CodeNodeLink[] {
  const snap = useCodeGraphSnapshot(projectId);
  return useMemo(() => {
    if (!filePath || !snap.data) return NO_LINKS;
    const { links } = buildCodeNodeLinks(snap.data.nodes, filePath);
    return links.length > 0 ? links : NO_LINKS;
  }, [snap.data, filePath]);
}

/** Why `useCodeNodeLinks` is (or isn't) empty. */
export function useCodeNodeLinksStatus(projectId: number | null, filePath?: string): CodeNodeLinksStatus {
  const snap = useCodeGraphSnapshot(projectId);
  return useMemo(() => {
    const result =
      filePath && snap.data ? buildCodeNodeLinks(snap.data.nodes, filePath) : { links: NO_LINKS, ambiguousPaths: [] as string[] };
    return {
      loading: snap.loading,
      error: snap.error,
      hasCodeGraph: snap.hasCodeGraph,
      matchedFile: result.links.length > 0,
      ambiguousPaths: result.ambiguousPaths,
    };
  }, [snap, filePath]);
}
