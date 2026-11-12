/**
 * HeldCallsPanel — outbound calls the jurisdiction gate is HOLDING right now (GET /jurisdiction/held).
 *
 * A held call is never dropped and never let through: it waits and retries on its own (250 ms doubling to 30 s per call; there is no timeout)
 * until the operator decides. This view makes that visible instead of silent: which hold, why, how long it has waited, how many attempts, when
 * it retries, which requirements are still unaccepted, and the recent held decisions (hashes and rule vocabulary only — never request content).
 * Releasing a hold is the operator's act in the panel below (accept a requirement / approve a transfer); this view does not release anything.
 */
import React, { useCallback, useEffect, useState } from "react";
import { fetchHeldCalls, type HeldCallsResponse } from "../ozoneClient";

const card: React.CSSProperties = { border: "1px solid #2a3a4d", borderRadius: 8, padding: 12, background: "#101923" };
const muted: React.CSSProperties = { color: "#8fa7bf", fontSize: 11.5 };

function age(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  if (s < 90) return `${s}s`;
  if (s < 5400) return `${Math.round(s / 60)} min`;
  return `${(s / 3600).toFixed(1)} h`;
}

export const HeldCallsPanel: React.FC<{ pollMs?: number }> = ({ pollMs = 5000 }) => {
  const [data, setData] = useState<HeldCallsResponse | null>(null);
  const [err, setErr] = useState("");
  const load = useCallback(async () => {
    try {
      const r = await fetchHeldCalls();
      if (r.success) {
        setData(r);
        setErr("");
      } else setErr(r.error ?? "unavailable");
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : String(e));
    }
  }, []);
  useEffect(() => {
    void load();
    const t = window.setInterval(() => void load(), pollMs);
    return () => window.clearInterval(t);
  }, [load, pollMs]);

  if (err && !data) return <div className="oempty" style={{ textAlign: "left" }}>Held calls unavailable: {err}</div>;
  if (!data) return <div className="oempty" style={{ textAlign: "left" }}>Loading held calls…</div>;
  const nothing = data.held.length === 0 && data.unaccepted_requirements.length === 0;
  return (
    <div style={card}>
      <div className="omono" style={{ fontWeight: 600, marginBottom: 6 }}>
        Held outbound calls — {data.held.length === 0 ? "none right now" : `${data.held.length} waiting`}
      </div>
      {nothing && <div style={muted}>No outbound call is waiting on the jurisdiction gate.</div>}
      {data.unaccepted_requirements.length > 0 && (
        <div style={{ color: "#f0c987", marginBottom: 8 }}>
          Requirements not yet accepted (calls to third-country providers wait until you accept them):{" "}
          {data.unaccepted_requirements.map((k) => (
            <span key={k} className="omono" style={{ marginRight: 8 }}>{k}</span>
          ))}
        </div>
      )}
      {data.held.map((h) => (
        <div key={h.await_id} style={{ borderTop: "1px solid #223244", padding: "6px 0" }}>
          <div>
            <strong>{h.kind.replace(/_/g, " ")}</strong>{" "}
            <span style={muted}>waiting {age(h.age_ms)} · {h.attempts} attempt{h.attempts === 1 ? "" : "s"} · next retry {h.next_attempt_at_ms > data.now_ms ? `in ${age(h.next_attempt_at_ms - data.now_ms)}` : "due now"}</span>
          </div>
          <div style={{ fontSize: 12.5, color: "#c9d6e6", overflowWrap: "anywhere" }}>{h.reason}</div>
          <div style={muted}>
            subject {h.subject.slice(0, 16)}{h.subject.length > 16 ? "…" : ""}{h.task_id ? ` · task ${h.task_id}` : ""}
          </div>
        </div>
      ))}
      {data.held_decisions.length > 0 && (
        <details style={{ marginTop: 8 }}>
          <summary style={muted}>Recent held decisions ({data.held_decisions.length}, last 24 h)</summary>
          <pre style={{ fontSize: 11.5, overflowX: "auto", background: "#0c141c", padding: 6, borderRadius: 6 }}>
            {data.held_decisions.map((d) => JSON.stringify(d)).join("\n")}
          </pre>
        </details>
      )}
      <div style={{ ...muted, marginTop: 6 }}>{data.note ?? "Held calls wait and retry on their own; nothing here releases one."}</div>
    </div>
  );
};

export default HeldCallsPanel;

/** Status-bar chip: renders nothing while no call is held and every requirement is accepted. */
export const HeldCallsChip: React.FC<{ pollMs?: number }> = ({ pollMs = 8000 }) => {
  const [data, setData] = useState<HeldCallsResponse | null>(null);
  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      try {
        const r = await fetchHeldCalls();
        if (!cancelled && r.success) setData(r);
      } catch {
        /* a host that cannot answer is shown by the connection indicator, not here */
      }
    };
    void load();
    const t = window.setInterval(() => void load(), pollMs);
    return () => {
      cancelled = true;
      window.clearInterval(t);
    };
  }, [pollMs]);
  if (!data || (data.held.length === 0 && data.unaccepted_requirements.length === 0)) return null;
  const parts = [
    data.held.length > 0 ? `${data.held.length} outbound call${data.held.length === 1 ? "" : "s"} held` : null,
    data.unaccepted_requirements.length > 0 ? `${data.unaccepted_requirements.length} requirement${data.unaccepted_requirements.length === 1 ? "" : "s"} to accept` : null,
  ].filter(Boolean);
  return (
    <span
      title="The jurisdiction gate is holding outbound calls until you decide. Open Marketplace → Jurisdiction to accept requirements or approve a transfer. Held calls wait and retry on their own; nothing is dropped or let through."
      style={{ color: "#d9b45f", fontSize: 11 }}
    >
      ⏸ {parts.join(" · ")}
    </span>
  );
};
