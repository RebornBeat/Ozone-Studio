/**
 * GraphNativeWorkspace — the project workspace AS the graph (guide §4.7 / §8).
 *
 * One persistent 3D world fills the workspace. The classic views (file
 * browser, fabric, AMT hierarchy, engines…) are not tabs you page between and
 * not a block stacked under a small tile — they are DOCKED INSIDE the world as
 * its detail panel. The bottom lens bar is a camera/highlight preset selector:
 * switching lens re-frames the same world and dims what the lens does not
 * mean; the substrate never remounts.
 *
 *   ┌ header: project · lens line · counts · peer/activity chips ─────────┐
 *   │ ┌ world (3D) ───────────────────────────────┐┌ dock ─────────────┐  │
 *   │ │ HUD: lens · truncation · live pulses       ││ Inspector│Detail │  │
 *   │ │ controls: reset · focus · labels · rotate  ││ selected entity,  │  │
 *   │ │ legend (click = solo a modality)           ││ its cross-links;  │  │
 *   │ │                                            ││ or the classic    │  │
 *   │ │                                            ││ lens view         │  │
 *   │ └────────────────────────────────────────────┘└───────────────────┘  │
 *   └ lens bar ───────────────────────────────────────────────────────────┘
 *
 * Everything shown is real: counts come from the substrate, truncation is
 * reported when a cap trims the drawn set, pulses are real construction
 * events, an empty/loading/error world says why.
 */
import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  Crosshair,
  ExternalLink,
  Maximize2,
  Minimize2,
  Orbit,
  PanelRightClose,
  PanelRightOpen,
  RotateCcw,
  Search,
  Tag,
  X,
} from "lucide-react";
import { useGraphKit, useGraphKitProject } from "./graphProvider";
import type { FocusRequest } from "./GraphWorld3D";
import { CrossLinkStrip } from "./primitives";
import { ModalityBadge } from "./indicators";
import {
  computeLensView,
  computeWorldLayout,
  type WorldLens,
  type WorldNode,
} from "./layout3d";
import { MODALITY_COLOR } from "../graphRenderers/defaults";
import { navigateTo } from "../navigation";
import type { Modality } from "../graphViewTypes";

export interface WorkspaceLensItem {
  id: string;
  label: string;
  icon?: React.ReactNode;
}

export interface GraphNativeWorkspaceProps {
  /** Real ZSEI project container id (null = no project selected). */
  projectId: number | null;
  /** What the 3D world highlights for the active lens. */
  worldLens: WorldLens;
  /** The lens bar. */
  lenses: WorkspaceLensItem[];
  activeLens: string;
  onLens: (id: string) => void;
  /** Sub-views of the active lens (docked detail's segmented control). */
  subs: { id: string; label: string }[] | null;
  activeSub: string | null;
  onSub: (id: string) => void;
  /** The classic view for the active lens/sub, docked into the world. */
  detail: React.ReactNode;
  /** Live-orchestration strip (shown under the header when present). */
  banner?: React.ReactNode;
  /** Peer / activity chips (header, right). */
  statusStrip?: React.ReactNode;
  /** Inspector → switch to an engine lens for this modality. */
  onOpenInEngine?: (modality: string) => void;
  /** Inspector → switch to the Files lens. */
  onOpenFiles?: () => void;
}

// ── Persisted UI prefs (best-effort; the workspace renders without them) ──

interface Prefs {
  dockOpen: boolean;
  dockWidth: number;
  labels: boolean;
  rotate: boolean;
}
const PREFS_KEY = "ozone-gnw-prefs-v1";
const DEFAULT_PREFS: Prefs = { dockOpen: true, dockWidth: 420, labels: true, rotate: false };

function loadPrefs(): Prefs {
  try {
    const raw = localStorage.getItem(PREFS_KEY);
    if (!raw) return DEFAULT_PREFS;
    const p = JSON.parse(raw);
    return {
      dockOpen: typeof p.dockOpen === "boolean" ? p.dockOpen : DEFAULT_PREFS.dockOpen,
      dockWidth: typeof p.dockWidth === "number" ? Math.min(760, Math.max(300, p.dockWidth)) : DEFAULT_PREFS.dockWidth,
      labels: typeof p.labels === "boolean" ? p.labels : DEFAULT_PREFS.labels,
      rotate: typeof p.rotate === "boolean" ? p.rotate : DEFAULT_PREFS.rotate,
    };
  } catch {
    return DEFAULT_PREFS;
  }
}
function savePrefs(p: Prefs) {
  try {
    localStorage.setItem(PREFS_KEY, JSON.stringify(p));
  } catch {
    /* storage unavailable — prefs just don't persist */
  }
}


// three.js + react-three-fiber are heavy: load the 3D world on demand so the
// rest of the app (and the classic views in the dock) never pay for it until
// a project's workspace is actually shown.
const GraphWorld3D = React.lazy(() =>
  import("./GraphWorld3D").then((m) => ({ default: m.GraphWorld3D })),
);

/** True when this environment can create a WebGL context at all. */
function detectWebGL(): boolean {
  try {
    const c = document.createElement("canvas");
    return !!(c.getContext("webgl2") || c.getContext("webgl"));
  } catch {
    return false;
  }
}

/** A failure inside the 3D renderer must never take the workspace down: the
 * dock (every classic view) keeps working and the world says what happened. */
class WorldBoundary extends React.Component<
  { onError: (message: string) => void; children: React.ReactNode },
  { error: string | null }
> {
  state = { error: null as string | null };
  static getDerivedStateFromError(e: unknown) {
    return { error: e instanceof Error ? e.message : String(e) };
  }
  componentDidCatch(e: unknown) {
    this.props.onError(e instanceof Error ? e.message : String(e));
  }
  render() {
    if (this.state.error) {
      return (
        <div className="gnw-overlay gnw-overlay-error">
          The 3D world could not start: {this.state.error}
          <span className="gnw-muted">Every view is still available in the dock →</span>
        </div>
      );
    }
    return this.props.children;
  }
}

const C_ACCENT = "#4fc3f7";

// ── Inspector ──

const Inspector: React.FC<{
  node: WorldNode;
  neighbours: { node: WorldNode; cls: string }[];
  rawNode: { confidence?: number; contentPreview?: string; props: [string, string][] } | null;
  onSelectContainer: (containerId: number) => void;
  onSelectIndex: (index: number) => void;
  onFocus: () => void;
  onOpenInEngine?: (modality: string) => void;
  onOpenFiles?: () => void;
  projectId: number;
}> = ({ node, neighbours, rawNode, onSelectContainer, onSelectIndex, onFocus, onOpenInEngine, onOpenFiles, projectId }) => {
  const substrate = useGraphKit.getState();
  const filesFed =
    node.kind === "file"
      ? substrate.files.find((f) => f.containerId === node.containerId)?.linkedContainers ?? []
      : [];
  const modality = node.modality !== "container" ? (node.modality as Modality) : null;
  return (
    <div className="gnw-insp">
      <div className="gnw-insp-head">
        <span className="gnw-insp-kind">{node.kind === "hub" ? "graph" : node.kind}</span>
        <div className="gnw-insp-title">{node.label}</div>
        <div className="gnw-insp-sub">
          {modality && <ModalityBadge modality={modality} />}
          <span>{node.nodeType}</span>
          {!node.isReal && <span className="gnw-tag-derived" title="Derived — not directly stored">derived</span>}
          {node.containerId !== null && <span className="gnw-muted">container #{node.containerId}</span>}
        </div>
      </div>

      <div className="gnw-insp-actions">
        <button className="gnw-btn" onClick={onFocus} title="Frame this in the world (F)">
          <Crosshair size={13} /> Focus
        </button>
        {modality && onOpenInEngine && (
          <button className="gnw-btn" onClick={() => onOpenInEngine(modality)} title="Open this modality's engine view">
            <ExternalLink size={13} /> Open {modality} engine
          </button>
        )}
        {node.kind === "file" && onOpenFiles && (
          <button className="gnw-btn" onClick={onOpenFiles} title="Open the Files lens">
            <ExternalLink size={13} /> Open in Files
          </button>
        )}
        {node.containerId !== null && node.kind !== "root" && (
          <button
            className="gnw-btn"
            onClick={() => navigateTo({ kind: "container", containerId: node.containerId! })}
            title="Open this container in its own tab"
          >
            <ExternalLink size={13} /> Container tab
          </button>
        )}
        {node.kind === "node" && (
          <button
            className="gnw-btn"
            onClick={() => navigateTo({ kind: "graph-node", projectId, nodeId: node.id })}
            title="Open this node in the 2D graph view"
          >
            <ExternalLink size={13} /> 2D graph
          </button>
        )}
      </div>

      {rawNode && (
        <div className="gnw-insp-sec">
          {typeof rawNode.confidence === "number" && (
            <div className="gnw-kv"><span>confidence</span><b>{rawNode.confidence.toFixed(2)}</b></div>
          )}
          {rawNode.contentPreview && <div className="gnw-preview">{rawNode.contentPreview}</div>}
          {rawNode.props.length > 0 && (
            <div className="gnw-props">
              {rawNode.props.map(([k, v]) => (
                <div key={k} className="gnw-kv"><span>{k}</span><b title={v}>{v}</b></div>
              ))}
            </div>
          )}
        </div>
      )}

      {node.kind === "root" && <RootSummary />}

      {node.kind === "file" && filesFed.length > 0 && (
        <div className="gnw-insp-sec">
          <div className="gnw-sec-title">Feeds {filesFed.length} graph{filesFed.length === 1 ? "" : "s"}</div>
          <div className="gnw-chips">
            {filesFed.map((c) => (
              <button key={c} className="gnw-chip" onClick={() => onSelectContainer(c)}>
                graph #{c}
              </button>
            ))}
          </div>
        </div>
      )}

      {node.containerId !== null && (
        <div className="gnw-insp-sec">
          <div className="gnw-sec-title">Cross-links</div>
          <CrossLinkStrip containerId={node.containerId} onNavigate={onSelectContainer} max={14} />
          {(substrate.crossLinksByContainer[node.containerId]?.length ?? 0) === 0 && (
            <div className="gnw-muted">No cross-links recorded for this container.</div>
          )}
        </div>
      )}

      {neighbours.length > 0 && (
        <div className="gnw-insp-sec">
          <div className="gnw-sec-title">Connected ({neighbours.length})</div>
          <div className="gnw-chips">
            {neighbours.slice(0, 14).map(({ node: n, cls }) => (
              <button
                key={n.index}
                className="gnw-chip"
                onClick={() => onSelectIndex(n.index)}
                title={`${cls} — ${n.nodeType}`}
              >
                <span className="gnw-dot" style={{ background: n.color }} />
                {n.label}
              </button>
            ))}
            {neighbours.length > 14 && <span className="gnw-muted">+{neighbours.length - 14} more</span>}
          </div>
        </div>
      )}
    </div>
  );
};

const RootSummary: React.FC = () => {
  const graphContainers = useGraphKit((s) => s.graphContainers);
  const nodes = useGraphKit((s) => s.nodes);
  const edges = useGraphKit((s) => s.edges);
  const files = useGraphKit((s) => s.files);
  const containers = useGraphKit((s) => s.containers);
  const crossLinks = useGraphKit((s) => s.crossLinks);
  const byModality = useMemo(() => {
    const c: Record<string, number> = {};
    for (const g of Object.values(graphContainers)) c[g.modality] = (c[g.modality] ?? 0) + 1;
    return Object.entries(c).sort((a, b) => b[1] - a[1]);
  }, [graphContainers]);
  return (
    <div className="gnw-insp-sec">
      <div className="gnw-kv"><span>graphs</span><b>{Object.keys(graphContainers).length}</b></div>
      <div className="gnw-kv"><span>nodes</span><b>{nodes.length}</b></div>
      <div className="gnw-kv"><span>edges</span><b>{edges.length}</b></div>
      <div className="gnw-kv"><span>files</span><b>{files.length}</b></div>
      <div className="gnw-kv"><span>other containers</span><b>{Object.keys(containers).length}</b></div>
      <div className="gnw-kv"><span>container-level links</span><b>{crossLinks.length}</b></div>
      {byModality.length > 0 && (
        <div className="gnw-chips" style={{ marginTop: 6 }}>
          {byModality.map(([m, n]) => (
            <span key={m} className="gnw-chip gnw-chip-static">
              <span className="gnw-dot" style={{ background: MODALITY_COLOR[m as Modality] ?? "#8b949e" }} />
              {m} ×{n}
            </span>
          ))}
        </div>
      )}
    </div>
  );
};

// ── Main ──

export const GraphNativeWorkspace: React.FC<GraphNativeWorkspaceProps> = ({
  projectId, worldLens, lenses, activeLens, onLens, subs, activeSub, onSub, detail, banner, statusStrip,
  onOpenInEngine, onOpenFiles,
}) => {
  useGraphKitProject(projectId);
  const status = useGraphKit((s) => s.status);
  const nodes = useGraphKit((s) => s.nodes);
  const edges = useGraphKit((s) => s.edges);
  const graphContainers = useGraphKit((s) => s.graphContainers);
  const projectName = useGraphKit((s) => s.projectName);
  const files = useGraphKit((s) => s.files);
  const containers = useGraphKit((s) => s.containers);
  const crossLinks = useGraphKit((s) => s.crossLinks);
  const pulses = useGraphKit((s) => s.pulses);
  const unknownModalityGraphs = useGraphKit((s) => s.unknownModalityGraphs);
  const nodeEventCount = useGraphKit((s) => s.nodeEventCount);

  const rootRef = useRef<HTMLDivElement>(null);
  const webglOk = useMemo(detectWebGL, []);
  const [prefs, setPrefs] = useState<Prefs>(loadPrefs);
  const updatePrefs = useCallback((patch: Partial<Prefs>) => {
    setPrefs((p) => {
      const next = { ...p, ...patch };
      savePrefs(next);
      return next;
    });
  }, []);
  const [dockTab, setDockTab] = useState<"inspector" | "detail">("detail");
  const [dockExpanded, setDockExpanded] = useState(false);
  const [narrow, setNarrow] = useState(false);
  const [isFs, setIsFs] = useState(false);
  const [solo, setSolo] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [focus, setFocus] = useState<FocusRequest | null>(null);
  const focusKey = useRef(0);
  const searchRef = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState("");

  // Layout is a pure function of the substrate: same substrate → same world.
  const layout = useMemo(
    () =>
      computeWorldLayout({
        projectId: projectId ?? 0,
        projectName,
        nodes,
        edges,
        graphContainers,
        files,
        containers,
        crossLinks,
      }),
    [projectId, projectName, nodes, edges, graphContainers, files, containers, crossLinks],
  );

  const effectiveLens: WorldLens = solo
    ? { kind: "engines", modalities: [solo], label: solo }
    : worldLens;
  const lensView = useMemo(
    () => computeLensView(layout, effectiveLens, crossLinks, files),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [layout, JSON.stringify(effectiveLens), crossLinks, files],
  );
  const lensViewRef = useRef(lensView);
  lensViewRef.current = lensView;

  const selectedIndex = selectedId !== null ? layout.byId.get(selectedId) ?? null : null;
  const selectedNode = selectedIndex !== null ? layout.nodes[selectedIndex] : null;

  const requestFocus = useCallback((center: [number, number, number], radius: number) => {
    focusKey.current += 1;
    setFocus({ key: focusKey.current, center, radius });
  }, []);

  // Re-frame on lens change (and once when the world first has content).
  const worldReady = layout.nodes.length > 1;
  const lensKey = `${activeLens}|${activeSub ?? ""}|${solo ?? ""}`;
  useEffect(() => {
    if (!worldReady) return;
    const f = lensViewRef.current.focus;
    requestFocus(f.center, f.radius);
  }, [lensKey, worldReady, requestFocus]);

  const select = useCallback(
    (index: number | null) => {
      if (index === null) {
        setSelectedId(null);
        return;
      }
      const n = layout.nodes[index];
      if (!n) return;
      setSelectedId(n.id);
      setDockTab("inspector");
      updatePrefs({ dockOpen: true });
      // A hub frames its whole cluster; anything else frames tight around itself.
      requestFocus(n.pos, n.kind === "hub" ? Math.max(8, n.shell + 2) : Math.max(5, n.radius * 7));
    },
    [layout, requestFocus, updatePrefs],
  );
  const selectContainer = useCallback(
    (containerId: number) => {
      const idx = layout.entityByContainer.get(containerId);
      if (idx !== undefined) select(idx);
    },
    [layout, select],
  );
  const frameSelection = useCallback(() => {
    if (selectedNode) requestFocus(selectedNode.pos, selectedNode.kind === "hub" ? Math.max(8, selectedNode.shell + 2) : Math.max(5, selectedNode.radius * 7));
    else {
      const f = lensViewRef.current.focus;
      requestFocus(f.center, f.radius);
    }
  }, [selectedNode, requestFocus]);
  const resetView = useCallback(() => {
    setSolo(null);
    requestFocus([0, 0, 0], layout.radius);
  }, [layout.radius, requestFocus]);
  const openIndex = useCallback(
    (index: number) => {
      const n = layout.nodes[index];
      if (!n) return;
      select(index);
      // Double-click = traverse INTO it: modality nodes/hubs open their
      // engine; files open the Files lens; the rest stay in the inspector.
      if ((n.kind === "node" || n.kind === "hub") && n.modality !== "container") onOpenInEngine?.(n.modality);
      else if (n.kind === "file") onOpenFiles?.();
    },
    [layout, select, onOpenInEngine, onOpenFiles],
  );

  // Find anything in the world by label / type / modality; hubs and files
  // first, then the rest, capped so the dropdown stays a dropdown.
  const searchResults = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (q.length < 2) return [];
    const rank = (n: WorldNode) => (n.kind === "root" ? 0 : n.kind === "hub" ? 1 : n.kind === "file" ? 2 : n.kind === "container" ? 3 : 4);
    return layout.nodes
      .filter((n) => n.label.toLowerCase().includes(q) || n.nodeType.toLowerCase().includes(q) || n.modality.toLowerCase() === q)
      .sort((a, b) => rank(a) - rank(b) || b.degree - a.degree)
      .slice(0, 8);
  }, [layout, query]);

  // Narrow containers get an overlay dock instead of a side-by-side one.
  useEffect(() => {
    const el = rootRef.current;
    if (!el || typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver((entries) => {
      for (const e of entries) setNarrow(e.contentRect.width < 780);
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  useEffect(() => {
    const onFs = () => setIsFs(document.fullscreenElement === rootRef.current);
    document.addEventListener("fullscreenchange", onFs);
    return () => document.removeEventListener("fullscreenchange", onFs);
  }, []);
  const toggleFullscreen = () => {
    const el = rootRef.current;
    if (!el) return;
    if (document.fullscreenElement) void document.exitFullscreen();
    else void el.requestFullscreen?.();
  };

  // Keyboard (when the workspace has focus): Esc, F, R, L, D.
  const onKeyDown = (e: React.KeyboardEvent) => {
    const t = e.target as HTMLElement;
    if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.tagName === "SELECT" || t.isContentEditable)) return;
    if (e.key === "/") {
      e.preventDefault();
      searchRef.current?.focus();
    } else if (e.key === "Escape") {
      if (dockExpanded) setDockExpanded(false);
      else setSelectedId(null);
    } else if (e.key === "f" || e.key === "F") frameSelection();
    else if (e.key === "r" || e.key === "R") resetView();
    else if (e.key === "l" || e.key === "L") updatePrefs({ labels: !prefs.labels });
    else if (e.key === "d" || e.key === "D") updatePrefs({ dockOpen: !prefs.dockOpen });
  };

  // Dock resize (drag the handle on the dock's left edge).
  const dragging = useRef(false);
  const onResizeDown = (e: React.PointerEvent) => {
    dragging.current = true;
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
  };
  const onResizeMove = (e: React.PointerEvent) => {
    if (!dragging.current || !rootRef.current) return;
    const rect = rootRef.current.getBoundingClientRect();
    const w = Math.round(rect.right - e.clientX);
    setPrefs((p) => ({ ...p, dockWidth: Math.min(Math.max(300, w), Math.min(760, rect.width - 220)) }));
  };
  const onResizeUp = (e: React.PointerEvent) => {
    if (!dragging.current) return;
    dragging.current = false;
    (e.target as HTMLElement).releasePointerCapture(e.pointerId);
    setPrefs((p) => {
      savePrefs(p);
      return p;
    });
  };

  // Selected entity's raw payload + neighbours.
  const nodeById = useMemo(() => {
    const m = new Map<string, (typeof nodes)[number]>();
    for (const n of nodes) m.set(n.id, n);
    return m;
  }, [nodes]);
  const rawNode = useMemo(() => {
    if (!selectedNode || selectedNode.kind !== "node") return null;
    const g = nodeById.get(selectedNode.id);
    if (!g) return null;
    const props = Object.entries((g.raw as { properties?: Record<string, unknown> }).properties ?? {})
      .slice(0, 14)
      .map(([k, v]): [string, string] => {
        const s = typeof v === "string" ? v : JSON.stringify(v);
        return [k, s.length > 90 ? s.slice(0, 89) + "…" : s];
      });
    return { confidence: g.confidence, contentPreview: g.contentPreview, props };
  }, [selectedNode, nodeById]);
  const neighbours = useMemo(() => {
    if (selectedIndex === null) return [];
    const out: { node: WorldNode; cls: string }[] = [];
    for (const e of layout.edges) {
      if (e.a === selectedIndex) out.push({ node: layout.nodes[e.b], cls: e.cls });
      else if (e.b === selectedIndex) out.push({ node: layout.nodes[e.a], cls: e.cls });
    }
    return out.filter((n) => n.node.kind !== "root");
  }, [layout, selectedIndex]);

  // Legend: modalities present (+ files, other containers).
  const legend = useMemo(
    () =>
      Object.entries(layout.modalityCounts)
        .sort((a, b) => b[1] - a[1])
        .map(([m, n]) => ({ modality: m, count: n, color: MODALITY_COLOR[m as Modality] ?? "#8b949e" })),
    [layout],
  );

  const livePulses = useMemo(() => {
    const cutoff = Date.now() - 5000;
    return Object.values(pulses).filter((t) => t >= cutoff).length;
  }, [pulses]);

  if (projectId === null) {
    return <div className="gnw gnw-empty-state">Select a project to explore its graph.</div>;
  }

  const trunc = layout.truncation;
  const truncNotes: string[] = [];
  if (trunc.graphNodes.shown < trunc.graphNodes.total)
    truncNotes.push(`showing the ${trunc.graphNodes.shown} most-connected of ${trunc.graphNodes.total} graph nodes`);
  if (trunc.containers.shown < trunc.containers.total)
    truncNotes.push(`${trunc.containers.shown} of ${trunc.containers.total} other containers`);
  if (trunc.files.shown < trunc.files.total) truncNotes.push(`${trunc.files.shown} of ${trunc.files.total} files`);

  // Without WebGL the dock IS the workspace — never let it be closed away.
  const dockVisible = prefs.dockOpen || !webglOk;
  const dockStyle: React.CSSProperties = dockExpanded
    ? { width: "100%" }
    : narrow
      ? { width: "min(94%, " + prefs.dockWidth + "px)" }
      : { width: prefs.dockWidth };

  return (
    <div
      ref={rootRef}
      className={"gnw" + (narrow ? " gnw-narrow" : "") + (isFs ? " gnw-fs" : "")}
      tabIndex={0}
      onKeyDown={onKeyDown}
    >
      {/* Header */}
      <div className="gnw-head">
        <span className="gnw-head-title" title={projectName ?? undefined}>{projectName ?? `Project #${projectId}`}</span>
        <span className="gnw-head-lens">{lensView.label}</span>
        <span className="gnw-head-spacer" />
        <div className="gnw-search">
          <Search size={13} />
          <input
            ref={searchRef}
            value={query}
            placeholder="Find in graph…  ( / )"
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              e.stopPropagation();
              if (e.key === "Escape") {
                setQuery("");
                (e.target as HTMLInputElement).blur();
              } else if (e.key === "Enter" && searchResults[0]) {
                select(searchResults[0].index);
                setQuery("");
              }
            }}
          />
          {searchResults.length > 0 && (
            <div className="gnw-search-results">
              {searchResults.map((n) => (
                <button
                  key={n.index}
                  onClick={() => {
                    select(n.index);
                    setQuery("");
                  }}
                >
                  <span className="gnw-dot" style={{ background: n.color }} />
                  <span className="gnw-sr-label">{n.label}</span>
                  <span className="gnw-muted">{n.kind === "hub" ? "graph" : n.kind === "node" ? n.nodeType : n.kind}</span>
                </button>
              ))}
            </div>
          )}
        </div>
        {statusStrip}
      </div>
      {banner}

      {/* World + dock */}
      <div className="gnw-main">
        {!dockExpanded && (
          <div className="gnw-world">
            {layout.nodes.length > 0 && webglOk && (
              <WorldBoundary onError={() => updatePrefs({ dockOpen: true })}>
                <React.Suspense
                  fallback={<div className="gnw-overlay"><div className="gnw-spinner" />Preparing the 3D world…</div>}
                >
                  <GraphWorld3D
                    layout={layout}
                    lens={lensView}
                    selectedIndex={selectedIndex}
                    pulses={pulses}
                    showLabels={prefs.labels}
                    autoRotate={prefs.rotate}
                    focus={focus}
                    onSelect={select}
                    onOpen={openIndex}
                  />
                </React.Suspense>
              </WorldBoundary>
            )}
            {!webglOk && (
              <div className="gnw-overlay gnw-overlay-error">
                WebGL is not available in this environment, so the 3D world cannot render.
                <span className="gnw-muted">Every view is still available in the dock →</span>
              </div>
            )}

            {/* HUD — top-left */}
            <div className="gnw-hud gnw-hud-tl">
              <div className="gnw-hud-line">
                <b>{layout.nodes.length - 1}</b> entities · <b>{layout.edges.length}</b> links ·{" "}
                <b>{Object.keys(graphContainers).length}</b> graphs · <b>{files.length}</b> files
              </div>
              {truncNotes.length > 0 && <div className="gnw-hud-warn">{truncNotes.join(" · ")}</div>}
              {unknownModalityGraphs > 0 && (
                <div className="gnw-hud-warn">{unknownModalityGraphs} graph(s) of an unknown modality counted, not drawn</div>
              )}
              {livePulses > 0 ? (
                <div className="gnw-hud-live">⚡ {livePulses} node{livePulses === 1 ? "" : "s"} being constructed now</div>
              ) : nodeEventCount > 0 ? (
                <div className="gnw-hud-dim">{nodeEventCount} construction events this session</div>
              ) : null}
              {solo && (
                <button className="gnw-chip" onClick={() => setSolo(null)} title="Clear solo">
                  solo: {solo} <X size={11} />
                </button>
              )}
            </div>

            {/* Controls — top-right */}
            <div className="gnw-hud gnw-hud-tr">
              <button className="gnw-ibtn" onClick={resetView} title="Reset view (R)"><RotateCcw size={14} /></button>
              <button className="gnw-ibtn" onClick={frameSelection} title="Focus selection / lens (F)"><Crosshair size={14} /></button>
              <button className={"gnw-ibtn" + (prefs.labels ? " on" : "")} onClick={() => updatePrefs({ labels: !prefs.labels })} title="Labels (L)"><Tag size={14} /></button>
              <button className={"gnw-ibtn" + (prefs.rotate ? " on" : "")} onClick={() => updatePrefs({ rotate: !prefs.rotate })} title="Auto-rotate"><Orbit size={14} /></button>
              <button className="gnw-ibtn" onClick={toggleFullscreen} title="Fullscreen">{isFs ? <Minimize2 size={14} /> : <Maximize2 size={14} />}</button>
              <button className="gnw-ibtn" onClick={() => updatePrefs({ dockOpen: !prefs.dockOpen })} title="Dock (D)">
                {prefs.dockOpen ? <PanelRightClose size={14} /> : <PanelRightOpen size={14} />}
              </button>
            </div>

            {/* Legend — bottom-left */}
            {legend.length > 0 && (
              <div className="gnw-hud gnw-hud-bl gnw-legend">
                {legend.map((l) => (
                  <button
                    key={l.modality}
                    className={"gnw-legend-item" + (solo === l.modality ? " on" : "")}
                    onClick={() => setSolo(solo === l.modality ? null : l.modality)}
                    title={`Click to solo ${l.modality}`}
                  >
                    <span className="gnw-dot" style={{ background: l.color }} />
                    {l.modality} <span className="gnw-muted">{l.count}</span>
                  </button>
                ))}
              </div>
            )}

            {/* Hint — bottom-right */}
            <div className="gnw-hud gnw-hud-br gnw-hint">
              drag orbit · scroll zoom · click select · double-click open · F focus · R reset
            </div>

            {/* Honest states */}
            {status.kind === "loading" && layout.nodes.length <= 1 && (
              <div className="gnw-overlay"><div className="gnw-spinner" />Loading the project's graph…</div>
            )}
            {status.kind === "error" && (
              <div className="gnw-overlay gnw-overlay-error">Could not load the graph: {status.message}</div>
            )}
            {status.kind === "empty" && <div className="gnw-overlay">{status.why}</div>}
          </div>
        )}

        {dockVisible && (
          <div className={"gnw-dock" + (narrow && !dockExpanded ? " gnw-dock-overlay" : "")} style={dockStyle}>
            {!dockExpanded && !narrow && (
              <div
                className="gnw-resize"
                onPointerDown={onResizeDown}
                onPointerMove={onResizeMove}
                onPointerUp={onResizeUp}
                title="Drag to resize"
              />
            )}
            <div className="gnw-dock-head">
              <div className="gnw-seg">
                <button className={dockTab === "inspector" ? "on" : ""} onClick={() => setDockTab("inspector")}>
                  Inspector
                </button>
                <button className={dockTab === "detail" ? "on" : ""} onClick={() => setDockTab("detail")}>
                  Detail
                </button>
              </div>
              <span className="gnw-head-spacer" />
              <button
                className="gnw-ibtn"
                onClick={() => setDockExpanded((v) => !v)}
                title={dockExpanded ? "Back to the world (Esc)" : "Expand the detail over the world"}
              >
                {dockExpanded ? <Minimize2 size={14} /> : <Maximize2 size={14} />}
              </button>
              <button className="gnw-ibtn" onClick={() => updatePrefs({ dockOpen: false })} title="Close dock (D)">
                <X size={14} />
              </button>
            </div>

            {dockTab === "inspector" ? (
              <div className="gnw-dock-body">
                {selectedNode ? (
                  <Inspector
                    node={selectedNode}
                    neighbours={neighbours}
                    rawNode={rawNode}
                    onSelectContainer={selectContainer}
                    onSelectIndex={select}
                    onFocus={frameSelection}
                    onOpenInEngine={onOpenInEngine}
                    onOpenFiles={onOpenFiles}
                    projectId={projectId}
                  />
                ) : (
                  <div className="gnw-muted gnw-pad">
                    Click anything in the world to inspect it: its type, its real fields, every container-level link it
                    has, and what it is connected to. Double-click opens it in its engine.
                  </div>
                )}
              </div>
            ) : (
              <div className="gnw-dock-body">
                {subs && subs.length > 0 && (
                  <div className="gnw-subs">
                    {subs.map((s) => (
                      <button key={s.id} className={activeSub === s.id ? "on" : ""} onClick={() => onSub(s.id)}>
                        {s.label}
                      </button>
                    ))}
                  </div>
                )}
                <div className="gnw-detail">{detail}</div>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Lens bar — camera/highlight presets over the one world */}
      <div className="gnw-lensbar">
        {lenses.map((l) => (
          <button
            key={l.id}
            className={activeLens === l.id ? "on" : ""}
            onClick={() => {
              onLens(l.id);
              setDockTab("detail");
              updatePrefs({ dockOpen: true });
              setDockExpanded(false);
            }}
            title={`Switch lens: ${l.label}`}
          >
            {l.icon}
            {l.label}
          </button>
        ))}
      </div>
    </div>
  );
};

export default GraphNativeWorkspace;
