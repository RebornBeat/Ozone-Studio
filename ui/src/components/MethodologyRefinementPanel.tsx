/**
 * MethodologyRefinementPanel — READ-ONLY view of methodology refinement (GET /methodology/refinement).
 *
 * Doctrine (operator 2026-11-10): a SOUND methodology is one where ALL of its rules apply to the requests it is gathered for. The rule gate
 * records every rule it judged clearly NOT applicable (with the request terms) and every applicable rule no blueprint step cites; the refinement
 * action then moves a rule into a focused methodology when a unanimous strength-N vote says it belongs to a different concern, and retires it
 * from the original only after the new methodology was created and verified. This panel shows that evidence and those outcomes in plain words:
 * counts of real records, yes/no verdicts, outcomes — never a score or a confidence.
 */
import React, { useEffect, useState } from "react";
import { fetchMethodologyRefinement, MethodologyRefinementResponse, RefinementAction } from "../ozoneClient";

const card: React.CSSProperties = { border: "1px solid rgba(255,255,255,0.08)", borderRadius: 8, padding: "9px 12px", background: "rgba(255,255,255,0.02)" };
const chip: React.CSSProperties = { display: "inline-block", fontSize: 11, padding: "1px 7px", borderRadius: 10, border: "1px solid rgba(255,255,255,0.14)", color: "#9fb3c8", marginRight: 5, marginBottom: 2 };
const okChip: React.CSSProperties = { ...chip, borderColor: "rgba(63,185,80,0.55)", color: "#56d364" };
const warnChip: React.CSSProperties = { ...chip, borderColor: "rgba(210,153,34,0.6)", color: "#e3b341" };
const note: React.CSSProperties = { fontSize: 11.5, color: "#8b98ab" };

function when(tsMs?: number): string {
  if (typeof tsMs !== "number" || !isFinite(tsMs) || tsMs <= 0) return "";
  return new Date(tsMs).toLocaleString([], { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
}

/** plain wording of what the action did with one rule */
function outcomeOf(a: RefinementAction): { label: string; style: React.CSSProperties } {
  if (a.kind === "declined") return { label: "kept in place — the vote was not unanimous that it belongs elsewhere", style: chip };
  if (a.kind === "split") {
    const r = a.original_rule_retirement?.status;
    if (r === "retired") return { label: "moved into a focused methodology and retired from the original", style: okChip };
    if (r === "refused") return { label: "moved into a focused methodology — retirement from the original was refused (the original still has it)", style: warnChip };
    return { label: "moved into a focused methodology — the original still has it", style: warnChip };
  }
  return { label: String(a.kind), style: chip };
}

const ActionRow: React.FC<{ a: RefinementAction }> = ({ a }) => {
  const o = outcomeOf(a);
  const ret = a.original_rule_retirement;
  return (
    <div style={{ marginTop: 8, paddingBottom: 6, borderBottom: "1px solid rgba(255,255,255,0.06)" }}>
      <span className="omono" style={{ color: "#e8eef6" }}>{a.methodology ?? `methodology ${a.methodology_id}`}</span>{" "}
      <span style={chip}>#{a.methodology_id}</span>
      {a.rule && <span style={chip}>{a.rule}</span>}
      <span style={o.style}>{o.label}</span>
      {a.ts_ms ? <span style={note}> {when(a.ts_ms)}</span> : null}
      {a.rule_text && <div style={{ ...note, marginTop: 2 }}>rule: {a.rule_text}</div>}
      {typeof a.evidence_count === "number" && (
        <div style={note}>flagged on {a.evidence_count} request{a.evidence_count === 1 ? "" : "s"}{typeof a.strength === "number" ? ` · judged in ${a.strength} rounds` : ""}</div>
      )}
      {a.kind === "split" && (
        <div style={note}>
          new methodology: {a.new_name ?? "(unnamed)"}{typeof a.new_methodology_id === "number" ? ` (#${a.new_methodology_id})` : ""}
        </div>
      )}
      {a.reason && <div style={note}>{a.reason}</div>}
      {ret && (
        <div style={note}>
          retirement: <strong>{ret.status}</strong>{ret.reason ? ` — ${ret.reason}` : ""}
          {ret.previous_body_path ? <> · before-body kept at <span className="omono">{ret.previous_body_path}</span> (repoint to undo)</> : null}
        </div>
      )}
      {!ret && a.original_body_path && a.kind === "split" && (
        <div style={note}>original body: <span className="omono">{a.original_body_path}</span></div>
      )}
    </div>
  );
};

export const MethodologyRefinementPanel: React.FC<{ pollMs?: number }> = ({ pollMs = 30000 }) => {
  const [data, setData] = useState<MethodologyRefinementResponse | null>(null);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      try { const r = await fetchMethodologyRefinement(); if (!cancelled) { setData(r); setErr(null); } }
      catch (e: any) { if (!cancelled) setErr(String(e?.message ?? e)); }
    };
    load();
    const t = setInterval(load, pollMs);
    return () => { cancelled = true; clearInterval(t); };
  }, [pollMs]);

  if (err && !data) return <div className="oempty">Methodology refinement could not be read: {err}</div>;
  if (!data) return <div className="oempty">Loading methodology refinement…</div>;

  const flagged = data.report.methodologies_with_inapplicable_rules ?? [];
  const uncovered = Object.entries(data.report.uncovered_rule_refs ?? {});
  const actions = [...(data.actions ?? [])].sort((a, b) => (b.ts_ms ?? 0) - (a.ts_ms ?? 0));
  const nothing = flagged.length === 0 && uncovered.length === 0 && actions.length === 0;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div className="oempty" style={{ textAlign: "left" }}>
        A sound methodology is one where every rule applies whenever it is gathered. The rule gate records a rule it judged clearly NOT applicable to a request,
        and an applicable rule that no plan step cites; a rule moves into a focused methodology only when every judging round agrees it belongs elsewhere.
        Nothing here is scored: these are the records themselves.
      </div>
      {nothing && (
        <div style={card}>
          <div className="oempty" style={{ textAlign: "left" }}>
            Nothing has been recorded yet ({data.candidates} candidate record{data.candidates === 1 ? "" : "s"}). Records appear after walks that gathered methodologies
            have been through the rule gate.
          </div>
        </div>
      )}

      {flagged.length > 0 && (
        <div style={card}>
          <div className="omono" style={{ color: "#e8eef6", fontWeight: 600, marginBottom: 6 }}>Methodologies with rules judged not applicable ({flagged.length})</div>
          {flagged.map((m) => (
            <div key={m.methodology_id} style={{ marginBottom: 8 }}>
              <span className="omono" style={{ color: "#e8eef6" }}>{m.methodology || `methodology ${m.methodology_id}`}</span>{" "}
              <span style={chip}>#{m.methodology_id}</span>
              <span style={warnChip}>flagged in {m.walks_flagged} walk{m.walks_flagged === 1 ? "" : "s"}</span>
              {Object.entries(m.rules_not_applicable ?? {}).map(([rule, n]) => (
                <div key={rule} style={{ ...note, marginLeft: 4 }}>• {rule} — judged not applicable in {n} walk{n === 1 ? "" : "s"}</div>
              ))}
            </div>
          ))}
        </div>
      )}

      {uncovered.length > 0 && (
        <div style={card}>
          <div className="omono" style={{ color: "#e8eef6", fontWeight: 600, marginBottom: 6 }}>Applicable rules no step cited ({uncovered.length})</div>
          <div style={note}>The plan missed the rule, or the rule cannot be acted on as a step.</div>
          <div style={{ marginTop: 4 }}>
            {uncovered.map(([ref, n]) => <span key={ref} style={chip} title={`${n} walk${n === 1 ? "" : "s"}`}>{ref} · {n}×</span>)}
          </div>
        </div>
      )}

      <div style={card}>
        <div className="omono" style={{ color: "#e8eef6", fontWeight: 600, marginBottom: 6 }}>Refinement actions ({actions.length})</div>
        {actions.length === 0
          ? <div className="oempty" style={{ textAlign: "left" }}>No rule has been moved or declined yet.</div>
          : actions.map((a, i) => <ActionRow key={`${a.methodology_id}:${a.rule ?? ""}:${a.ts_ms ?? i}`} a={a} />)}
        {actions.some((a) => a.original_rule_retirement?.status === "refused") && (
          <div style={{ ...note, marginTop: 8, color: "#e3b341" }}>
            A refused retirement usually means the methodology_create pipeline on this host has not been rebuilt with rule retirement; the split itself stands.
          </div>
        )}
      </div>
      {err && <div style={{ ...note, color: "#ff7b72" }}>Last refresh failed: {err}</div>}
    </div>
  );
};

export default MethodologyRefinementPanel;
