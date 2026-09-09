/**
 * PipelinesPanel — the host's LOADED pipeline registry, on the shared
 * design system. Authoritative source: POST /pipeline/registry.
 */
import React, { useEffect, useMemo, useState } from "react";
import { PipelineRegistryEntry, fetchPipelineRegistry } from "../ozoneClient";

export const PipelinesPanel: React.FC = () => {
  const [pipelines, setPipelines] = useState<PipelineRegistryEntry[]>([]);
  const [connected, setConnected] = useState(false);
  const [filter, setFilter] = useState("");
  const [category, setCategory] = useState("all");

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      try {
        const out = await fetchPipelineRegistry();
        if (cancelled) return;
        setPipelines(out.registry ?? []);
        setConnected(true);
      } catch {
        if (!cancelled) setConnected(false);
      }
    };
    load();
    return () => {
      cancelled = true;
    };
  }, []);

  const categories = useMemo(() => {
    const m = new Map<string, number>();
    for (const p of pipelines) m.set(p.category, (m.get(p.category) ?? 0) + 1);
    return [...m.entries()].sort();
  }, [pipelines]);

  const shown = pipelines.filter(
    (p) =>
      (category === "all" || p.category === category) &&
      (!filter ||
        p.name.toLowerCase().includes(filter.toLowerCase()) ||
        p.description?.toLowerCase().includes(filter.toLowerCase())),
  );

  return (
    <div className="opanel">
      <div className="opanel-head">
        <span className={`odot ${connected ? "ok" : "err"}`} />
        <span className="opanel-title">Loaded Pipelines</span>
      </div>
      <p className="opanel-sub">
        Everything the host bootstrapped — the executable units of Ozone-Studio.
      </p>

      <div className="ostats">
        <div className="ostat">
          <div className="ostat-num">{pipelines.length}</div>
          <div className="ostat-label">Loaded</div>
        </div>
        <div className="ostat">
          <div className="ostat-num">
            {pipelines.filter((p) => p.has_ui).length}
          </div>
          <div className="ostat-label">With UI</div>
        </div>
        <div className="ostat">
          <div className="ostat-num">{categories.length}</div>
          <div className="ostat-label">Categories</div>
        </div>
      </div>

      <div
        style={{
          display: "flex",
          gap: 10,
          flexWrap: "wrap",
          alignItems: "center",
          marginBottom: 12,
        }}
      >
        <div className="oseg">
          <button
            className={category === "all" ? "on" : ""}
            onClick={() => setCategory("all")}
          >
            all
          </button>
          {categories.map(([cat, n]) => (
            <button
              key={cat}
              className={category === cat ? "on" : ""}
              onClick={() => setCategory(cat)}
            >
              {cat} · {n}
            </button>
          ))}
        </div>
        <input
          className="oinput"
          style={{ maxWidth: 240 }}
          placeholder="Filter pipelines…"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
        />
      </div>

      {!connected ? (
        <div className="oempty">Host registry unreachable.</div>
      ) : (
        <div style={{ maxHeight: 230, overflowY: "auto" }}>
          <table className="otable">
            <thead>
              <tr>
                <th>ID</th>
                <th>Name</th>
                <th>Category</th>
                <th>UI</th>
              </tr>
            </thead>
            <tbody>
              {shown.map((p) => (
                <tr key={p.pipeline_id}>
                  <td className="omono">{p.pipeline_id}</td>
                  <td style={{ color: "#e8eef6" }}>{p.name}</td>
                  <td>
                    <span className="ochip cat">{p.category}</span>
                  </td>
                  <td>{p.has_ui ? <span className="ochip ui">UI</span> : "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
};

export default PipelinesPanel;
