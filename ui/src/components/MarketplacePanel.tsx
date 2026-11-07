/**
 * MarketplacePanel — tools / MCPs / methodologies / blueprints and how they relate.
 *
 * Operator direction (2026-11-04): PROVIDERS (MCP servers) belong PER RULE inside methodologies; blueprint creation turns each into a
 * provider STEP (a step type beside pipeline steps); the marketplace shows what is registered globally (this host + paired P2P peers), what this
 * user has installed/downloaded, the relation per methodology (methodology → rules → tools → blueprints), and the
 * contribute → review → download flow. docs/METHODOLOGY_BLUEPRINT_TOOLS_REVIEW_2026-11-04.md §6.
 *
 * REAL DATA ONLY (AGENTS.md: no fabricated metrics). Everything comes from GET /marketplace/overview. Peer catalogs are
 * not exchanged yet, so the peer sections say exactly that instead of listing invented offers; keyword-INFERRED tools
 * are drawn dashed and labelled "inferred" — only DECLARED `requires[]` bindings are drawn solid.
 */
import React, { useEffect, useMemo, useState } from "react";
import {
  MarketplaceBlueprint,
  MarketplaceMethodology,
  MarketplaceOverview,
  MarketplaceProvider,
  fetchMarketplaceOverview,
} from "../ozoneClient";

type View = "catalog" | "installed" | "relations" | "contribute";

const card: React.CSSProperties = { border: "1px solid rgba(255,255,255,0.08)", borderRadius: 8, padding: "9px 12px", background: "rgba(255,255,255,0.02)" };
const chip: React.CSSProperties = { display: "inline-block", fontSize: 11, padding: "1px 7px", borderRadius: 10, border: "1px solid rgba(255,255,255,0.14)", color: "#9fb3c8", marginRight: 5, marginBottom: 3 };
const declaredChip: React.CSSProperties = { ...chip, borderColor: "#4ade80", color: "#4ade80" };
const inferredChip: React.CSSProperties = { ...chip, borderStyle: "dashed", borderColor: "#d3a24a", color: "#d3a24a" };
const warnChip: React.CSSProperties = { ...chip, borderColor: "#f87171", color: "#f87171" };

const tabBtn = (on: boolean): React.CSSProperties => ({
  background: on ? "rgba(110,195,255,0.16)" : "transparent",
  border: "1px solid " + (on ? "#6ec3ff" : "rgba(255,255,255,0.12)"),
  color: on ? "#cfe9ff" : "#8b98ab",
  borderRadius: 6, padding: "4px 12px", cursor: "pointer", fontSize: 12.5,
});

const providerName = (p: MarketplaceProvider) => p.provider || p.endpoint;

/** provider id -> { declared: methodology names, inferred: methodology names }. A requirement naming a TOOL resolves to the provider that backs it. */
function usageIndex(ms: MarketplaceMethodology[], providers: MarketplaceProvider[]) {
  const toolToProvider = new Map<string, string>();
  for (const p of providers) for (const t of p.tools) toolToProvider.set(t, p.provider);
  const idx = new Map<string, { declared: Set<string>; inferred: Set<string> }>();
  const slot = (t: string) => { if (!idx.has(t)) idx.set(t, { declared: new Set(), inferred: new Set() }); return idx.get(t)!; };
  for (const m of ms) for (const r of m.rules) {
    for (const q of r.requires) slot(q.kind === "provider" ? q.name : toolToProvider.get(q.name) ?? q.name).declared.add(m.name);
    for (const t of r.inferred_providers) slot(t).inferred.add(m.name);
  }
  return idx;
}

export const MarketplacePanel: React.FC<{ pollMs?: number }> = ({ pollMs = 15000 }) => {
  const [data, setData] = useState<MarketplaceOverview | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [view, setView] = useState<View>("catalog");
  const [q, setQ] = useState("");
  const [kind, setKind] = useState<"all" | "tools" | "methodologies" | "blueprints">("all");
  const [selMeth, setSelMeth] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      try { const o = await fetchMarketplaceOverview(); if (!cancelled) { setData(o); setErr(null); } }
      catch (e: any) { if (!cancelled) setErr(String(e?.message ?? e)); }
    };
    load();
    const t = setInterval(load, pollMs);
    return () => { cancelled = true; clearInterval(t); };
  }, [pollMs]);

  const usage = useMemo(() => usageIndex(data?.methodologies ?? [], data?.tools.providers ?? []), [data]);
  const ql = q.trim().toLowerCase();
  const providers = (data?.tools.providers ?? []).filter((p) => !ql || (p.provider + " " + p.tools.join(" ") + " " + p.capabilities.join(" ")).toLowerCase().includes(ql));
  const methodologies = (data?.methodologies ?? []).filter((m) => !ql || (m.name + " " + m.categories.join(" ")).toLowerCase().includes(ql));
  const blueprints = (data?.blueprints ?? []).filter((b) => !ql || b.name.toLowerCase().includes(ql));
  const registered = useMemo(() => new Map((data?.tools.providers ?? []).map((p) => [p.provider, p] as const)), [data]);

  if (err && !data) return <div className="opanel"><div className="oempty">Marketplace unavailable: {err}</div></div>;
  if (!data) return <div className="opanel"><div className="oempty">Loading marketplace…</div></div>;

  const declaredRules = data.methodologies.reduce((n, m) => n + m.rules_with_declared_tools, 0);
  const totalRules = data.methodologies.reduce((n, m) => n + m.rule_count, 0);
  const peersLive = data.peers.catalog_exchange === "active";

  const methForRelations = data.methodologies.find((m) => m.file === selMeth) ?? data.methodologies[0];
  const bpsFor = (m?: MarketplaceMethodology): MarketplaceBlueprint[] => m && m.id != null ? data.blueprints.filter((b) => b.methodology_ids.includes(m.id as number)) : [];

  return (
    <div className="opanel">
      <div className="opanel-head">
        <span className={`odot ${err ? "err" : "ok"}`} />
        <span className="opanel-title">Marketplace</span>
      </div>
      <p className="opanel-sub">
        Tools and MCPs belong <strong>per rule</strong> inside methodologies; blueprint creation aggregates them into steps. This is
        what is registered on this host, what paired peers could offer, and how each methodology relates to the tools it needs.
      </p>

      <div className="ostats">
        <div className="ostat"><div className="ostat-num">{data.tools.count}</div><div className="ostat-label">Tools (this host)</div></div>
        <div className="ostat"><div className="ostat-num">{data.tools.providers.length}</div><div className="ostat-label">Providers</div></div>
        <div className="ostat"><div className="ostat-num">{data.peers.paired.length}</div><div className="ostat-label">Paired peers</div></div>
        <div className="ostat"><div className="ostat-num">{data.methodologies.length}</div><div className="ostat-label">Methodologies</div></div>
        <div className="ostat" title="rules whose methodology DECLARES the tools it needs / all rules"><div className="ostat-num">{declaredRules}/{totalRules}</div><div className="ostat-label">Rules with declared tools</div></div>
        <div className="ostat"><div className="ostat-num">{data.blueprints.length}</div><div className="ostat-label">Blueprints</div></div>
      </div>

      <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center", margin: "10px 0" }}>
        {(["catalog", "installed", "relations", "contribute"] as View[]).map((v) => (
          <button key={v} style={tabBtn(view === v)} onClick={() => setView(v)}>
            {{ catalog: "Catalog", installed: "Installed", relations: "Relations", contribute: "Contribute & Review" }[v]}
          </button>
        ))}
        <input value={q} onChange={(e) => setQ(e.target.value)} placeholder="search tools, capabilities, methodologies…"
          style={{ marginLeft: "auto", minWidth: 220, background: "rgba(255,255,255,0.04)", border: "1px solid rgba(255,255,255,0.12)", borderRadius: 6, color: "#dbe6f3", padding: "4px 10px", fontSize: 12.5 }} />
      </div>

      {view === "catalog" && (
        <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          <div>
            {(["all", "tools", "methodologies", "blueprints"] as const).map((k) => (
              <button key={k} style={{ ...tabBtn(kind === k), marginRight: 6, padding: "2px 10px", fontSize: 12 }} onClick={() => setKind(k)}>{k}</button>
            ))}
          </div>

          <div style={card}>
            <div className="omono" style={{ color: "#e8eef6", fontWeight: 600, marginBottom: 6 }}>Wanted capabilities ({data.wanted.length})</div>
            {data.wanted.length === 0
              ? <div className="oempty" style={{ textAlign: "left" }}>Nothing is wanted: every provider a methodology rule or blueprint required is registered. A required-but-missing provider appears here the moment a walk needs it — captured, with a methodology drafted around it — to be built or downloaded after review.</div>
              : data.wanted.map((w) => (
                <div key={w.kind + w.name} style={{ marginBottom: 6 }}>
                  <span style={warnChip}>{w.kind}</span><span className="omono" style={{ color: "#e8eef6" }}>{w.name}</span>
                  <span style={chip}>needed {w.count}×</span>
                  <div style={{ fontSize: 11.5, color: "#8b98ab", marginLeft: 4 }}>{w.details[w.details.length - 1]?.detail} ({w.details[w.details.length - 1]?.source})</div>
                </div>
              ))}
          </div>

          <div style={card}>
            <div className="omono" style={{ color: "#e8eef6", fontWeight: 600, marginBottom: 6 }}>From paired peers</div>
            {peersLive ? <div className="oempty">Peer catalogs would be listed here.</div> : (
              <div className="oempty" style={{ textAlign: "left" }}>
                <div>{data.peers.paired.length} paired peer(s){data.peers.paired.length ? ": " : "."}{data.peers.paired.map((p) => <span key={p} style={chip} title={p}>{p.slice(0, 10)}…</span>)}</div>
                <div style={{ marginTop: 6 }}>{data.peers.note}</div>
                <div style={{ marginTop: 6 }}>Next build step: signed manifest-only <code>Catalog</code> exchange between paired peers → offers appear here with review state.</div>
              </div>
            )}
          </div>

          {(kind === "all" || kind === "tools") && (
            <div>
              <div className="omono" style={{ color: "#e8eef6", fontWeight: 600, marginBottom: 6 }}>Providers (MCP servers) — this host ({providers.length}; {data.tools.count} tools)</div>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill,minmax(300px,1fr))", gap: 10 }}>
                {providers.map((p) => {
                  const used = usage.get(p.provider)?.declared ?? new Set<string>();
                  const inferred = usage.get(p.provider)?.inferred ?? new Set<string>();
                  const health = data.provider_health[p.provider];
                  return (
                    <div key={p.provider} style={card}>
                      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                        <span className={`odot ${p.running ? "ok" : "err"}`} title={p.running ? "running" : "not running"} />
                        <span className="omono" style={{ color: "#e8eef6", fontWeight: 500 }}>{providerName(p)}</span>
                        <span style={{ ...chip, marginLeft: "auto" }}>local provider</span>
                      </div>
                      <div className="omono" style={{ fontSize: 11, color: "#6f7f93", margin: "2px 0 6px" }}>{p.endpoints.join(", ")}</div>
                      <div>{p.tools.slice(0, 8).map((t) => <span key={t} style={chip}>{t}</span>)}{p.tools.length > 8 && <span style={chip}>+{p.tools.length - 8}</span>}</div>
                      <div style={{ marginTop: 4 }}>
                        {p.device_requirement && <span style={warnChip}>needs {p.device_requirement} device</span>}
                        {p.credential_requirement && <span style={warnChip}>needs {p.credential_requirement} credentials</span>}
                        {health && <span style={health.degraded ? warnChip : chip} title={health.last_error ?? ""}>{health.degraded ? "degraded" : "healthy"} · {health.failures}/{health.calls} recent failures</span>}
                        <span style={used.size ? declaredChip : chip}>declared by {used.size} methodolog{used.size === 1 ? "y" : "ies"}</span>
                        <span style={inferredChip} title="keyword overlap with rule text — not a binding">inferred {inferred.size}</span>
                      </div>
                    </div>
                  );
                })}
                {providers.length === 0 && <div className="oempty">No tools match.</div>}
              </div>
            </div>
          )}

          {(kind === "all" || kind === "methodologies") && (
            <div>
              <div className="omono" style={{ color: "#e8eef6", fontWeight: 600, marginBottom: 6 }}>Methodologies ({methodologies.length})</div>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill,minmax(260px,1fr))", gap: 8 }}>
                {methodologies.map((m) => (
                  <div key={m.file} style={{ ...card, cursor: "pointer" }} onClick={() => { setSelMeth(m.file); setView("relations"); }}>
                    <div className="omono" style={{ color: "#e8eef6" }}>{m.name}</div>
                    <div style={{ marginTop: 4 }}>
                      <span style={chip}>{m.source}</span><span style={chip}>{m.rule_count} rules</span>
                      <span style={m.rules_with_declared_tools ? declaredChip : chip}>{m.rules_with_declared_tools} with declared tools</span>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {(kind === "all" || kind === "blueprints") && (
            <div>
              <div className="omono" style={{ color: "#e8eef6", fontWeight: 600, marginBottom: 6 }}>Blueprints ({blueprints.length})</div>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill,minmax(260px,1fr))", gap: 8 }}>
                {blueprints.map((b) => (
                  <div key={String(b.id) + b.name} style={card}>
                    <div className="omono" style={{ color: "#e8eef6" }}>{b.name}</div>
                    <div style={{ marginTop: 4 }}>
                      <span style={chip}>{b.step_count} steps</span><span style={chip}>pipelines {b.pipelines.join(", ") || "—"}</span>
                      <span style={b.declares_tool_calls ? declaredChip : chip}>{b.declares_tool_calls ? "declares tool calls" : "no declared tool calls"}</span>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      {view === "installed" && (
        <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
          <div style={card}>
            <div className="omono" style={{ color: "#e8eef6", fontWeight: 600 }}>Downloaded from peers</div>
            <div className="oempty" style={{ textAlign: "left" }}>
              {data.policy.installed_from_peer.length === 0
                ? "Nothing downloaded from a peer yet — install-from-peer (fetch → quarantine → review → approve) is not built. Everything below is registered locally."
                : `${data.policy.installed_from_peer.length} item(s)`}
            </div>
          </div>
          <div className="omono" style={{ color: "#e8eef6", fontWeight: 600 }}>Registered on this host ({providers.length} providers)</div>
          <table className="otable">
            <thead><tr><th>Provider</th><th>Tools</th><th>State</th><th>Health</th><th>Auto-start</th><th>Requires</th><th>Source</th><th>Used by (declared / inferred)</th></tr></thead>
            <tbody>
              {providers.map((p) => {
                const d = usage.get(p.provider)?.declared ?? new Set<string>(), i = usage.get(p.provider)?.inferred ?? new Set<string>();
                const h = data.provider_health[p.provider];
                return (
                  <tr key={p.provider}>
                    <td className="omono">{providerName(p)}<div style={{ fontSize: 10.5, color: "#6f7f93" }}>{p.endpoints.join(", ")}</div></td>
                    <td title={p.tools.join(", ")}>{p.tool_count}</td>
                    <td><span className={`odot ${p.running ? "ok" : "err"}`} /> {p.running ? "running" : "stopped"}</td>
                    <td>{h ? (h.degraded ? `degraded (${h.failures}/${h.calls})` : `ok (${h.failures}/${h.calls} failed)`) : "no calls yet"}</td>
                    <td>{p.auto_start ? "on" : "off"}</td>
                    <td>{[p.device_requirement && `${p.device_requirement} device`, p.credential_requirement && `${p.credential_requirement} creds`].filter(Boolean).join(", ") || "—"}</td>
                    <td>local</td>
                    <td>{d.size} / {i.size}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>
          <p className="opanel-sub">Start, stop and auto-start controls live in the Tools tab.</p>
        </div>
      )}

      {view === "relations" && (
        <div style={{ display: "grid", gridTemplateColumns: "minmax(220px,280px) 1fr", gap: 14 }}>
          <div style={{ display: "flex", flexDirection: "column", gap: 4, maxHeight: 560, overflowY: "auto" }}>
            {methodologies.map((m) => (
              <div key={m.file} onClick={() => setSelMeth(m.file)}
                style={{ ...card, cursor: "pointer", borderColor: methForRelations?.file === m.file ? "#6ec3ff" : "rgba(255,255,255,0.08)" }}>
                <div className="omono" style={{ color: "#e8eef6", fontSize: 12.5 }}>{m.name}</div>
                <div style={{ fontSize: 11, color: "#6f7f93" }}>{m.rule_count} rules · {m.rules_with_declared_tools} declared</div>
              </div>
            ))}
          </div>
          {methForRelations ? (
            <div>
              <div className="omono" style={{ color: "#e8eef6", fontSize: 15, fontWeight: 600 }}>{methForRelations.name}</div>
              <div style={{ margin: "4px 0 10px" }}>
                <span style={declaredChip}>solid = declared by the rule</span>
                <span style={inferredChip}>dashed = inferred from keywords (not a binding)</span>
                <span style={warnChip}>red = not registered on this host (captured as wanted) or degraded</span>
              </div>
              {methForRelations.rules.map((r) => {
                const declared = r.requires;
                return (
                  <div key={r.index} style={{ ...card, marginBottom: 8 }}>
                    <div style={{ display: "flex", gap: 8 }}>
                      <span style={{ ...chip, flexShrink: 0 }}>rule {r.index + 1} · p{r.priority}</span>
                      <div style={{ fontSize: 12.5, color: "#c9d6e6" }}>
                        <div><span style={{ color: "#7fa8cc" }}>if</span> {r.condition}</div>
                        <div><span style={{ color: "#7fa8cc" }}>then</span> {r.action}</div>
                      </div>
                    </div>
                    <div style={{ marginTop: 6, paddingLeft: 4 }}>
                      {declared.map((d) => {
                        const provId = d.kind === "provider" ? d.name : (data.tools.providers.find((p) => p.tools.includes(d.name))?.provider ?? d.name);
                        const reg = registered.get(provId);
                        const health = data.provider_health[provId];
                        const missing = (d.kind === "provider" || d.kind === "mcp" || d.kind === "tool") && !reg;
                        return <span key={d.kind + d.name} style={missing ? warnChip : health?.degraded ? warnChip : declaredChip} title={d.purpose ?? ""}>
                          {d.kind}: {d.name}{d.optional ? " (optional)" : ""}{reg ? (health?.degraded ? " ⚠ degraded" : reg.running ? " ● running" : " ○ stopped") : missing ? " — not registered (wanted)" : ""}
                        </span>;
                      })}
                      {declared.length === 0 && r.inferred_providers.map((t) => (
                        <span key={t} style={inferredChip} title="keyword overlap with this rule's text — not a declared binding">{t} (inferred)</span>
                      ))}
                      {declared.length === 0 && r.inferred_providers.length === 0 && <span style={chip}>no provider declared or inferred</span>}
                    </div>
                  </div>
                );
              })}
              <div className="omono" style={{ color: "#e8eef6", fontWeight: 600, margin: "10px 0 4px" }}>Blueprints citing this methodology</div>
              {bpsFor(methForRelations).length === 0
                ? <div className="oempty" style={{ textAlign: "left" }}>No stored blueprint cites this methodology by id yet. New blueprints carry `methodology_ids`, `rule_refs` and PROVIDER STEPS (step type `provider`) created from a rule's declared providers.</div>
                : bpsFor(methForRelations).map((b) => <span key={String(b.id)} style={chip}>{b.name} · {b.step_count} steps</span>)}
            </div>
          ) : <div className="oempty">No methodologies found.</div>}
        </div>
      )}

      {view === "contribute" && (
        <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          <div style={card}>
            <div className="omono" style={{ color: "#e8eef6", fontWeight: 600 }}>Contribute a methodology</div>
            <p className="opanel-sub" style={{ marginBottom: 8 }}>
              Pick a methodology to see <strong>exactly what would leave this host</strong>: its name, categories, rules text and the tools each rule
              requires — never data, keys or chat content. Receivers review it before anything is installed, and a methodology offer carries its
              <code> requires</code> (providers) so installing it also offers the providers it needs.
            </p>
            <select value={methForRelations?.file ?? ""} onChange={(e) => setSelMeth(e.target.value)}
              style={{ background: "#0f1722", color: "#dbe6f3", border: "1px solid rgba(255,255,255,0.14)", borderRadius: 6, padding: "4px 8px", marginBottom: 8 }}>
              {data.methodologies.map((m) => <option key={m.file} value={m.file}>{m.name}</option>)}
            </select>
            {methForRelations && (
              <pre className="omono" style={{ fontSize: 11.5, background: "rgba(0,0,0,0.25)", padding: 10, borderRadius: 6, maxHeight: 260, overflow: "auto", margin: 0 }}>
{JSON.stringify({
  name: methForRelations.name, categories: methForRelations.categories,
  rules: methForRelations.rules.map((r) => ({ if: r.condition, then: r.action, priority: r.priority, requires: r.requires })),
}, null, 2)}
              </pre>
            )}
            <button disabled style={{ ...tabBtn(false), marginTop: 8, opacity: 0.5, cursor: "not-allowed" }}
              title="Publishing needs the peer catalog exchange and the Propose flow (currently a stub returning 'not implemented')">
              Publish to paired peers — not available yet
            </button>
            <span className="opanel-sub" style={{ marginLeft: 10 }}>Needs: peer catalog exchange + signed offers + Propose implementation.</span>
          </div>

          <div style={card}>
            <div className="omono" style={{ color: "#e8eef6", fontWeight: 600 }}>Review &amp; download policy (what will apply to every offer)</div>
            <ul style={{ margin: "6px 0 0 18px", color: "#9fb3c8", fontSize: 12.5, lineHeight: 1.7 }}>
              <li>Only <strong>paired</strong> peers are sources ({data.peers.paired.length} paired now). Pairing is default-deny and bidirectional.</li>
              <li>Offers arrive as <strong>manifests</strong>; artifacts are fetched into quarantine only on your request.</li>
              <li>Automatic review first: hash/signature, loopback-only endpoints, jurisdiction gate, security posture scan.</li>
              <li>Any <strong>executable</strong> tool/MCP needs your explicit approval ({String(data.policy.executables_require_human_approval)}); installs register with auto-start OFF.</li>
              <li>Sharing of your own methodologies/blueprints: <code>auto_broadcast_sync</code> = <strong>{data.policy.auto_broadcast_sync ? "ON" : "OFF"}</strong>.</li>
            </ul>
          </div>

          <div style={card}>
            <div className="omono" style={{ color: "#e8eef6", fontWeight: 600 }}>Proposed bindings ({data.binding_proposals.length})</div>
            <p className="opanel-sub" style={{ marginBottom: 6 }}>Providers chosen by run-time search that really answered a step. Promote one to a declared per-rule <code>requires</code> to make it authoritative (never applied automatically).</p>
            {data.binding_proposals.length === 0 ? <div className="oempty" style={{ textAlign: "left" }}>None yet.</div> : data.binding_proposals.map((b, i) => (
              <div key={i} style={{ marginBottom: 4, fontSize: 12.5 }}>
                <span style={inferredChip}>methodology #{b.methodology_id}</span><span className="omono" style={{ color: "#e8eef6" }}>{b.provider ?? b.tool}</span>
                <span style={chip}>{b.successes} success(es)</span><span style={{ color: "#8b98ab" }}> — {b.example_step.slice(0, 120)}</span>
              </div>
            ))}
          </div>

          <div style={card}>
            <div className="omono" style={{ color: "#e8eef6", fontWeight: 600 }}>Review queue</div>
            <div className="oempty" style={{ textAlign: "left" }}>No offers received — peer catalog exchange is not built, so there is nothing real to review yet.</div>
          </div>
        </div>
      )}
    </div>
  );
};

export default MarketplacePanel;
