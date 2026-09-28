/**
 * D5 — Hierarchy View: jurisdiction hierarchy (scope → EU → global).
 *
 * Real sources (all verified, not assumed):
 *  - src/lib.rs ~651-880 wires the graph: every non-global scope gets a `RelatedTo` edge to "global", and every
 *    EU member state additionally gets one to "eu" (eu itself is layered on "global"). Edges are written with
 *    discovered_via "Manual", confidence 0.9. The scope key is `keywords[0]` of a `JurisdictionRuleSet` container
 *    parented to JURISDICTION_ROOT_ID (7); canonical container per scope = MINIMUM container id (lib.rs comment).
 *  - Live host (read-only, 2026-09-24): root 7 has 298 children — 67 `JurisdictionRuleSet` (42 unique scopes: the
 *    rest are historical duplicates) and 231 keyword-less containers of type "Root" (phantoms). 204 of the 263
 *    relations point at those phantoms (dangling), some point at NON-canonical duplicate copies (e.g. `eu` 1146
 *    while the canonical `eu` is 1099). All relations are `RelatedTo`.
 *
 * So this view never trusts container ids alone: it collapses duplicates by scope key (lowest id wins), resolves each
 * relation target to a scope key — fetching any target it can't resolve from the loaded scopes, read-only — and
 * reports unresolvable targets honestly instead of drawing edges to nothing.
 *
 * Hierarchy = only the real `RelatedTo` relations. Tree placement uses transitive reduction (a member state points at
 * both `eu` and `global`; it is placed under `eu`, and `global` is listed as an implied parent). `JurisdictionScope`
 * (=70) is a declared-but-unwired RelationType (container.rs) and is never drawn as live.
 * Scopes with no resolvable relation at all are listed under "Unlinked scopes", not forced into the tree.
 *
 * NOTE: ZSEI query results arrive as `{success, result, error}` from both the Electron bridge and the HTTP fallback
 * (main.js `zsei:query` does not unwrap) — `unwrap()` below handles both that and an already-unwrapped result.
 */
import React, { useCallback, useEffect, useMemo, useState } from "react";
import { zseiQuery } from "../../ozoneClient";
import { navigateTo } from "../../navigation";
import { loadJurisdictionScopes } from "../../data/jurisdictionData";
import type { JurisdictionScope } from "../../data/jurisdictionData";

export type JurisdictionHierarchyProps = { projectId: number | null };

const C_TEXT = "#dfe7f2";
const C_BODY = "#c7d0dc";
const C_MUTED = "#8b98ab";
const C_BORDER = "#1e2836";
const C_WARN = "#e8c14f";
const C_ERR = "#ff8a8a";
const C_ACCENT = "#5fb3ff";

// ── Model ────────────────────────────────────────────────────────────────

interface ParentLink {
  parentKey: string;
  relationType: string;
  confidence?: number;
  discoveredVia?: string;
  /** How many underlying relations (across duplicate containers) collapsed into this link. */
  relationCount: number;
}
interface ScopeNode {
  key: string;
  containerId: number;
  name: string;
  /** Total containers found for this scope key (1 = no duplicates). */
  copies: number;
  parents: ParentLink[];
  /** Parents dropped by transitive reduction (reachable through a nearer parent). */
  impliedParents: string[];
  /** Relations whose target is not a jurisdiction scope (deleted duplicate / phantom / unreadable). */
  danglingCount: number;
  /** Relations of a type that is declared but not wired (never drawn). */
  ignoredTypeCount: number;
}
export interface Model {
  nodes: Map<string, ScopeNode>;
  children: Map<string, string[]>;
  roots: string[];
  unlinked: string[];
  cyclic: string[];
  stats: { containers: number; scopes: number; duplicatesCollapsed: number; relations: number; resolved: number; dangling: number; ignored: number };
  danglingKinds: Map<string, number>;
}

function unwrap<T>(r: unknown): T | null {
  if (r && typeof r === "object" && "success" in (r as object) && "result" in (r as object)) {
    const env = r as { success: boolean; result: T; error?: string };
    if (env.success === false) throw new Error(env.error || "ZSEI query failed");
    return env.result;
  }
  return (r as T) ?? null;
}

interface TargetInfo {
  regionKey: string | null;
  containerType: string;
}

/** Read-only lookup for relation targets that aren't among the loaded scope containers. */
async function resolveTargets(ids: number[]): Promise<Map<number, TargetInfo>> {
  const out = new Map<number, TargetInfo>();
  await Promise.all(
    ids.map(async (id) => {
      try {
        const res = unwrap<any>(await zseiQuery<unknown>({ GetContainer: { container_id: id } }));
        const c = res?.Container;
        if (!c) {
          out.set(id, { regionKey: null, containerType: "missing" });
          return;
        }
        const type: string = c.local_state?.metadata?.container_type ?? "unknown";
        const kw0: string | undefined = c.local_state?.context?.keywords?.[0];
        out.set(id, { regionKey: type === "JurisdictionRuleSet" && kw0 ? kw0 : null, containerType: type });
      } catch {
        out.set(id, { regionKey: null, containerType: "unreadable" });
      }
    }),
  );
  return out;
}

function fmtConfidence(c: number | undefined): string | null {
  return typeof c === "number" && Number.isFinite(c) ? c.toFixed(2) : null;
}

export async function buildModel(scopes: JurisdictionScope[]): Promise<Model> {
  // 1. Collapse duplicate containers by scope key; canonical = lowest container id.
  const byKey = new Map<string, JurisdictionScope[]>();
  const idToKey = new Map<number, string>();
  for (const s of scopes) {
    idToKey.set(s.containerId, s.regionKey);
    const list = byKey.get(s.regionKey) ?? [];
    list.push(s);
    byKey.set(s.regionKey, list);
  }
  const canonical = new Map<string, { scope: JurisdictionScope; copies: JurisdictionScope[] }>();
  for (const [key, list] of byKey) {
    const sorted = [...list].sort((a, b) => a.containerId - b.containerId);
    canonical.set(key, { scope: sorted[0], copies: sorted });
  }

  // 2. Resolve relation targets. Unknown ids get one read-only GetContainer each.
  const unknownTargets = new Set<number>();
  for (const { copies } of canonical.values()) {
    for (const s of copies) {
      for (const r of s.relations ?? []) {
        if (r.relationType === "RelatedTo" && !idToKey.has(r.targetId)) unknownTargets.add(r.targetId);
      }
    }
  }
  const extra = await resolveTargets(Array.from(unknownTargets));

  const nodes = new Map<string, ScopeNode>();
  const danglingKinds = new Map<string, number>();
  let relations = 0;
  let resolved = 0;
  let dangling = 0;
  let ignored = 0;

  for (const [key, { scope, copies }] of canonical) {
    const links = new Map<string, ParentLink>();
    let danglingCount = 0;
    let ignoredTypeCount = 0;
    // Relations from every duplicate copy describe the same scope; collapse them per parent scope.
    for (const copy of copies) {
      for (const r of copy.relations ?? []) {
        relations++;
        if (r.relationType !== "RelatedTo") {
          ignoredTypeCount++;
          ignored++;
          continue;
        }
        const targetKey = idToKey.get(r.targetId) ?? extra.get(r.targetId)?.regionKey ?? null;
        if (!targetKey || targetKey === key) {
          if (!targetKey) {
            danglingCount++;
            dangling++;
            const kind = extra.get(r.targetId)?.containerType ?? "unknown";
            danglingKinds.set(kind, (danglingKinds.get(kind) ?? 0) + 1);
          }
          continue;
        }
        resolved++;
        const existing = links.get(targetKey);
        if (existing) existing.relationCount++;
        else
          links.set(targetKey, {
            parentKey: targetKey,
            relationType: r.relationType,
            confidence: r.confidence,
            discoveredVia: r.discoveredVia,
            relationCount: 1,
          });
      }
    }
    nodes.set(key, {
      key,
      containerId: scope.containerId,
      name: scope.name,
      copies: copies.length,
      parents: Array.from(links.values()),
      impliedParents: [],
      danglingCount,
      ignoredTypeCount,
    });
  }
  // Drop parents that don't exist as scopes at all (resolved key with no loaded container).
  for (const n of nodes.values()) n.parents = n.parents.filter((p) => nodes.has(p.parentKey));

  // 3. Transitive reduction for tree placement (cycle-safe).
  const ancestors = (start: string): Set<string> => {
    const seen = new Set<string>();
    const stack = [start];
    while (stack.length) {
      const cur = stack.pop()!;
      for (const p of nodes.get(cur)?.parents ?? []) {
        if (!seen.has(p.parentKey)) {
          seen.add(p.parentKey);
          stack.push(p.parentKey);
        }
      }
    }
    return seen;
  };
  const anc = new Map<string, Set<string>>();
  for (const k of nodes.keys()) anc.set(k, ancestors(k));
  const reduced = new Map<string, string[]>();
  for (const n of nodes.values()) {
    const keep: string[] = [];
    for (const p of n.parents) {
      const redundant = n.parents.some(
        (q) => q.parentKey !== p.parentKey && anc.get(q.parentKey)!.has(p.parentKey) && !anc.get(p.parentKey)!.has(q.parentKey),
      );
      if (redundant) n.impliedParents.push(p.parentKey);
      else keep.push(p.parentKey);
    }
    reduced.set(n.key, keep);
  }

  const children = new Map<string, string[]>();
  for (const [k, ps] of reduced) for (const p of ps) children.set(p, [...(children.get(p) ?? []), k]);
  const hasAnyChild = new Set<string>();
  for (const n of nodes.values()) for (const p of n.parents) hasAnyChild.add(p.parentKey);

  const rank = (a: string, b: string) => {
    const ca = children.get(a)?.length ?? 0;
    const cb = children.get(b)?.length ?? 0;
    return cb - ca || a.localeCompare(b);
  };
  for (const list of children.values()) list.sort(rank);

  const roots: string[] = [];
  const unlinked: string[] = [];
  for (const n of nodes.values()) {
    const noParents = (reduced.get(n.key) ?? []).length === 0 && n.parents.length === 0;
    if (noParents && hasAnyChild.has(n.key)) roots.push(n.key);
    else if (noParents) unlinked.push(n.key);
  }
  roots.sort(rank);
  unlinked.sort();

  // 4. Anything not reachable from a root and not unlinked is in a relation cycle.
  const reach = new Set<string>();
  const walk = (k: string) => {
    if (reach.has(k)) return;
    reach.add(k);
    for (const c of children.get(k) ?? []) walk(c);
  };
  roots.forEach(walk);
  const cyclic = Array.from(nodes.keys()).filter((k) => !reach.has(k) && !unlinked.includes(k)).sort();

  return {
    nodes,
    children,
    roots,
    unlinked,
    cyclic,
    stats: {
      containers: scopes.length,
      scopes: nodes.size,
      duplicatesCollapsed: scopes.length - nodes.size,
      relations,
      resolved,
      dangling,
      ignored,
    },
    danglingKinds,
  };
}

// ── View ─────────────────────────────────────────────────────────────────

type LoadState = { kind: "loading" } | { kind: "error"; message: string } | { kind: "ready"; model: Model };

const Badge: React.FC<{ children: React.ReactNode; color?: string; title?: string }> = ({ children, color = C_MUTED, title }) => (
  <span
    title={title}
    style={{ fontSize: 10.5, color, border: `1px solid ${C_BORDER}`, borderRadius: 999, padding: "0 6px", whiteSpace: "nowrap" }}
  >
    {children}
  </span>
);

const NodeRow: React.FC<{
  nodeKey: string;
  model: Model;
  depth: number;
  path: Set<string>;
  expanded: Set<string>;
  toggle: (k: string) => void;
  visible: Set<string> | null;
  forceOpen: boolean;
  parentKey: string | null;
}> = ({ nodeKey, model, depth, path, expanded, toggle, visible, forceOpen, parentKey }) => {
  const node = model.nodes.get(nodeKey);
  if (!node) return null;
  if (visible && !visible.has(nodeKey)) return null;
  const kids = (model.children.get(nodeKey) ?? []).filter((c) => !path.has(c) && (!visible || visible.has(c)));
  const isOpen = forceOpen || expanded.has(nodeKey);
  const link = parentKey ? node.parents.find((p) => p.parentKey === parentKey) : undefined;
  const conf = fmtConfidence(link?.confidence);
  const nextPath = new Set(path).add(nodeKey);

  return (
    <div>
      <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "3px 0", marginLeft: depth * 18, minWidth: 0, flexWrap: "wrap" }}>
        <button
          onClick={() => kids.length > 0 && toggle(nodeKey)}
          disabled={kids.length === 0}
          aria-label={isOpen ? "Collapse" : "Expand"}
          style={{
            width: 18,
            background: "none",
            border: "none",
            color: kids.length ? C_ACCENT : "transparent",
            cursor: kids.length ? "pointer" : "default",
            padding: 0,
            fontSize: 12,
          }}
        >
          {isOpen ? "▾" : "▸"}
        </button>
        <span style={{ color: C_TEXT, fontWeight: 600, fontSize: 12.5, fontFamily: "monospace" }}>{node.key}</span>
        <span style={{ color: C_BODY, fontSize: 12, overflowWrap: "anywhere", minWidth: 0 }}>{node.name}</span>
        <Badge title="Canonical container id (lowest id for this scope key)">#{node.containerId}</Badge>
        {kids.length > 0 && <Badge title="Scopes layered directly on this one">{kids.length} child{kids.length === 1 ? "" : "ren"}</Badge>}
        {node.copies > 1 && (
          <Badge color={C_WARN} title="Historical duplicate containers for this scope key exist; the lowest id is shown as canonical">
            {node.copies} containers
          </Badge>
        )}
        {link && (
          <Badge
            color={C_ACCENT}
            title={`Real ${link.relationType} relation to ${link.parentKey}${link.relationCount > 1 ? ` (collapsed from ${link.relationCount} relations across duplicate containers)` : ""}`}
          >
            {link.relationType}
            {conf ? ` · ${conf}` : ""}
            {link.discoveredVia ? ` · ${link.discoveredVia}` : ""}
          </Badge>
        )}
        {node.impliedParents.length > 0 && (
          <Badge title="Also directly related to these scopes, but they are reachable through this node's parent, so it is not placed under them">
            also → {node.impliedParents.join(", ")}
          </Badge>
        )}
        {node.danglingCount > 0 && (
          <Badge color={C_WARN} title="Relations from this scope whose target is not a jurisdiction scope container">
            {node.danglingCount} dangling
          </Badge>
        )}
        <button
          onClick={() => navigateTo({ kind: "container", containerId: node.containerId })}
          title="Open this container"
          style={{ background: "none", border: "none", color: C_MUTED, cursor: "pointer", fontSize: 11, textDecoration: "underline", padding: 0 }}
        >
          open
        </button>
      </div>
      {isOpen &&
        kids.map((c) => (
          <NodeRow
            key={c}
            nodeKey={c}
            model={model}
            depth={depth + 1}
            path={nextPath}
            expanded={expanded}
            toggle={toggle}
            visible={visible}
            forceOpen={forceOpen}
            parentKey={nodeKey}
          />
        ))}
    </div>
  );
};

export const JurisdictionHierarchy: React.FC<JurisdictionHierarchyProps> = () => {
  const [state, setState] = useState<LoadState>({ kind: "loading" });
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [query, setQuery] = useState("");
  const [reloadTick, setReloadTick] = useState(0);

  useEffect(() => {
    let cancelled = false;
    setState({ kind: "loading" });
    loadJurisdictionScopes()
      .then((scopes) => buildModel(scopes))
      .then((model) => {
        if (cancelled) return;
        setState({ kind: "ready", model });
        setExpanded(new Set(model.roots));
      })
      .catch((e) => !cancelled && setState({ kind: "error", message: e instanceof Error ? e.message : String(e) }));
    return () => {
      cancelled = true;
    };
  }, [reloadTick]);

  const toggle = useCallback((k: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(k)) next.delete(k);
      else next.add(k);
      return next;
    });
  }, []);

  const q = query.trim().toLowerCase();
  const model = state.kind === "ready" ? state.model : null;

  // With a filter active: matching nodes + every ancestor along the tree stay visible.
  const visible = useMemo<Set<string> | null>(() => {
    if (!model || !q) return null;
    const matches = Array.from(model.nodes.values())
      .filter((n) => n.key.toLowerCase().includes(q) || n.name.toLowerCase().includes(q) || String(n.containerId) === q)
      .map((n) => n.key);
    const vis = new Set<string>(matches);
    const up = (k: string) => {
      for (const p of model.nodes.get(k)?.parents ?? []) {
        if (!vis.has(p.parentKey)) {
          vis.add(p.parentKey);
          up(p.parentKey);
        }
      }
    };
    matches.forEach(up);
    return vis;
  }, [model, q]);

  const header = (
    <div style={{ display: "flex", gap: 10, alignItems: "center", flexWrap: "wrap", marginBottom: 10 }}>
      <input
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Filter by scope key, name or container id"
        style={{ background: "#101724", color: C_TEXT, border: `1px solid ${C_BORDER}`, borderRadius: 6, padding: "4px 8px", fontSize: 12.5, minWidth: 240 }}
      />
      {model && (
        <>
          <button onClick={() => setExpanded(new Set(model.nodes.keys()))} style={btn}>
            Expand all
          </button>
          <button onClick={() => setExpanded(new Set())} style={btn}>
            Collapse all
          </button>
        </>
      )}
      <button onClick={() => setReloadTick((t) => t + 1)} style={btn}>
        Reload
      </button>
    </div>
  );

  if (state.kind === "loading") return <div style={{ padding: 16, color: C_MUTED, fontSize: 12.5 }}>Loading jurisdiction scopes…</div>;
  if (state.kind === "error")
    return (
      <div style={{ padding: 16 }}>
        {header}
        <div style={{ color: C_ERR, fontSize: 12.5 }}>Error loading jurisdiction scopes: {state.message}</div>
      </div>
    );

  const m = state.model;
  if (m.nodes.size === 0) {
    return (
      <div style={{ padding: 16 }}>
        {header}
        <div style={{ color: C_MUTED, fontSize: 12.5 }}>
          No jurisdiction scope containers were returned (root container 7 has no JurisdictionRuleSet children) — nothing to draw, nothing fabricated.
        </div>
      </div>
    );
  }

  const danglingSummary = Array.from(m.danglingKinds.entries())
    .map(([kind, n]) => `${n} → container type "${kind}"`)
    .join("; ");
  const showRoots = m.roots.filter((r) => !visible || visible.has(r));
  const showUnlinked = m.unlinked.filter((k) => !visible || visible.has(k));
  const showCyclic = m.cyclic.filter((k) => !visible || visible.has(k));

  return (
    <div style={{ padding: 4, minWidth: 0 }}>
      {header}
      <div style={{ fontSize: 11.5, color: C_MUTED, marginBottom: 10, lineHeight: 1.5 }}>
        {m.stats.scopes} scopes from {m.stats.containers} containers
        {m.stats.duplicatesCollapsed > 0 && ` (${m.stats.duplicatesCollapsed} historical duplicates collapsed, lowest id kept)`} · {m.stats.relations} relations:{" "}
        {m.stats.resolved} resolved to a scope, {m.stats.dangling} dangling{m.stats.ignored > 0 ? `, ${m.stats.ignored} of a not-yet-wired type ignored` : ""}
        {m.stats.dangling > 0 && (
          <div style={{ color: C_WARN }}>
            {m.stats.dangling} relation(s) point at containers that are not jurisdiction scopes ({danglingSummary}) — shown as dangling, never drawn as edges.
          </div>
        )}
      </div>

      {showRoots.length === 0 && !q && (
        <div style={{ color: C_MUTED, fontSize: 12.5, marginBottom: 8 }}>
          No scope is the target of a resolvable RelatedTo relation, so there is no hierarchy to draw yet.
        </div>
      )}
      {showRoots.map((r) => (
        <NodeRow
          key={r}
          nodeKey={r}
          model={m}
          depth={0}
          path={new Set()}
          expanded={expanded}
          toggle={toggle}
          visible={visible}
          forceOpen={!!q}
          parentKey={null}
        />
      ))}

      {showCyclic.length > 0 && (
        <Section title={`In a relation cycle (${showCyclic.length})`} note="These scopes relate to each other in a loop, so no root exists for them.">
          {showCyclic.map((k) => (
            <FlatRow key={k} node={m.nodes.get(k)!} />
          ))}
        </Section>
      )}
      {showUnlinked.length > 0 && (
        <Section
          title={`Unlinked scopes (${showUnlinked.length})`}
          note="No resolvable RelatedTo relation in either direction — listed, not forced into the tree."
        >
          {showUnlinked.map((k) => (
            <FlatRow key={k} node={m.nodes.get(k)!} />
          ))}
        </Section>
      )}
      {visible && showRoots.length + showUnlinked.length + showCyclic.length === 0 && (
        <div style={{ color: C_MUTED, fontSize: 12.5 }}>No scope matches “{query}”.</div>
      )}
    </div>
  );
};

const btn: React.CSSProperties = {
  background: "transparent",
  color: C_MUTED,
  border: `1px solid ${C_BORDER}`,
  borderRadius: 6,
  padding: "3px 10px",
  fontSize: 12,
  cursor: "pointer",
};

const Section: React.FC<{ title: string; note: string; children: React.ReactNode }> = ({ title, note, children }) => (
  <div style={{ marginTop: 14, paddingTop: 10, borderTop: `1px solid ${C_BORDER}` }}>
    <div style={{ fontSize: 12.5, fontWeight: 700, color: C_TEXT }}>{title}</div>
    <div style={{ fontSize: 11, color: C_MUTED, margin: "2px 0 6px" }}>{note}</div>
    {children}
  </div>
);

const FlatRow: React.FC<{ node: ScopeNode }> = ({ node }) => (
  <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "2px 0", flexWrap: "wrap" }}>
    <span style={{ color: C_TEXT, fontWeight: 600, fontSize: 12.5, fontFamily: "monospace" }}>{node.key}</span>
    <span style={{ color: C_BODY, fontSize: 12, overflowWrap: "anywhere", minWidth: 0 }}>{node.name}</span>
    <Badge>#{node.containerId}</Badge>
    {node.copies > 1 && <Badge color={C_WARN}>{node.copies} containers</Badge>}
    {node.danglingCount > 0 && (
      <Badge color={C_WARN} title="Relations from this scope whose target is not a jurisdiction scope container">
        {node.danglingCount} dangling relation{node.danglingCount === 1 ? "" : "s"}
      </Badge>
    )}
    <button
      onClick={() => navigateTo({ kind: "container", containerId: node.containerId })}
      style={{ background: "none", border: "none", color: C_MUTED, cursor: "pointer", fontSize: 11, textDecoration: "underline", padding: 0 }}
    >
      open
    </button>
  </div>
);

export default JurisdictionHierarchy;
