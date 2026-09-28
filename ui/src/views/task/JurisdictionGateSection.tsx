/**
 * H5 — Jurisdiction gate result display per task.
 *
 * REAL FINDING, verified directly against source (2026-09-27), that corrects
 * this fork's own original brief: `JurisdictionGateResult` (matched rules,
 * `blocked`, real `warnings`, real `confirmations` — src/orchestrator/
 * jurisdiction.rs:86) IS real and IS surfaced on `OrchestrationResponse`
 * (src/orchestrator/mod.rs:227, fixed 2026-09-15 per that field's own doc
 * comment) — but `OrchestrationResponse` is the synchronous body returned
 * directly from `POST /orchestrate` at request time. It is NOT persisted
 * onto the durable `Task` record: `TaskInfo` (src/grpc/mod.rs:129, what
 * `POST /task/get` returns) and the underlying `Task`/`TaskStepData`
 * structs (src/task/mod.rs) have no `jurisdiction`/`jurisdiction_gate`
 * field anywhere — grepped directly, zero hits. So unlike `amt_summary`
 * (which genuinely does survive onto the task record and a restart, per
 * task/mod.rs's own round-trip test), a task's jurisdiction gate result is
 * real at the moment of the request and then gone — there is no route that
 * lets this panel fetch it after the fact.
 *
 * This section is defensive rather than silent about that: it looks for a
 * `jurisdiction_gate`-shaped field on whatever `/task/get` actually returns
 * (in case a future fix threads it through, e.g. onto TaskInfo or into a
 * step's `output_summary` as JSON) and renders it fully if present. If
 * absent — which is every real task today — it says exactly why, instead of
 * presenting a "no jurisdiction rule matched" empty state that would imply
 * something it can't actually know.
 */
import React from "react";

interface JurisdictionRule {
  id?: string;
  name?: string;
  region?: string;
  action?: string;
  reason?: string;
  [k: string]: unknown;
}
interface GateResultLike {
  decision?: string;
  confidence?: number;
  reasoning?: string;
  [k: string]: unknown;
}
interface JurisdictionGateResultLike {
  rules_loaded?: number;
  matched?: Array<[JurisdictionRule, string] | { rule?: JurisdictionRule; action?: string }>;
  blocked?: boolean;
  warnings?: string[];
  confirmations?: Array<[JurisdictionRule, GateResultLike] | { rule?: JurisdictionRule; result?: GateResultLike }>;
}

export type JurisdictionGateSectionProps = { task: any };

const C_TEXT = "#dfe7f2";
const C_BODY = "#c7d0dc";
const C_MUTED = "#8b98ab";
const C_BORDER = "#1e2836";
const C_WARN = "#e8c14f";
const C_BAD = "#ff8a8a";

function findJurisdictionGate(task: any): JurisdictionGateResultLike | null {
  if (!task || typeof task !== "object") return null;
  // Direct field, in case a future backend fix threads it onto TaskInfo.
  if (task.jurisdiction_gate && typeof task.jurisdiction_gate === "object") return task.jurisdiction_gate;
  if (task.jurisdiction_gate_result && typeof task.jurisdiction_gate_result === "object") {
    return task.jurisdiction_gate_result;
  }
  // Fallback: a step's output_summary is a plain string on the real struct
  // (TaskStepData.output_summary: Option<String>), but check defensively in
  // case some caller ever puts a JSON-serialized gate result there.
  if (Array.isArray(task.steps)) {
    for (const step of task.steps) {
      const raw = step?.output_summary;
      if (typeof raw === "string" && raw.includes("jurisdiction")) {
        try {
          const parsed = JSON.parse(raw);
          if (parsed && typeof parsed === "object" && ("matched" in parsed || "rules_loaded" in parsed)) {
            return parsed as JurisdictionGateResultLike;
          }
        } catch {
          // Not JSON — genuinely just a text summary, not a gate result.
        }
      }
    }
  }
  return null;
}

function ruleAndAction(entry: JurisdictionRule | { rule?: JurisdictionRule; action?: string } | [JurisdictionRule, string]): {
  rule: JurisdictionRule | undefined;
  action: string | undefined;
} {
  if (Array.isArray(entry)) return { rule: entry[0], action: entry[1] };
  if ("rule" in entry || "action" in entry) return { rule: (entry as any).rule, action: (entry as any).action };
  return { rule: entry as JurisdictionRule, action: undefined };
}

function ruleAndResult(
  entry: [JurisdictionRule, GateResultLike] | { rule?: JurisdictionRule; result?: GateResultLike },
): { rule: JurisdictionRule | undefined; result: GateResultLike | undefined } {
  if (Array.isArray(entry)) return { rule: entry[0], result: entry[1] };
  return { rule: entry.rule, result: entry.result };
}

const SectionTitle: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div style={{ fontSize: 12.5, fontWeight: 700, color: C_TEXT, marginBottom: 6 }}>{children}</div>
);

export const JurisdictionGateSection: React.FC<JurisdictionGateSectionProps> = ({ task }) => {
  const gate = findJurisdictionGate(task);

  return (
    <div style={{ border: `1px solid ${C_BORDER}`, borderRadius: 10, padding: 12 }}>
      <SectionTitle>Jurisdiction gate</SectionTitle>

      {!gate && (
        <div style={{ fontSize: 12, color: C_MUTED, lineHeight: 1.6 }}>
          Not available for this task. The jurisdiction gate genuinely runs on every request and its real result
          (matched rules, warnings, confirmations) is returned in <code>OrchestrationResponse</code> at request time
          — but it is not persisted onto the task record (<code>/task/get</code>'s <code>TaskInfo</code> and its
          steps have no jurisdiction field), so it cannot be retrieved here after the fact. This is a real backend
          gap, not an empty result — the gate may well have matched something for this task and there is no way to
          know from here.
        </div>
      )}

      {gate && (
        <div style={{ fontSize: 12, color: C_BODY, lineHeight: 1.6 }}>
          <div style={{ marginBottom: 6 }}>
            <span style={{ color: C_MUTED }}>Rules loaded: </span>
            {gate.rules_loaded ?? "unknown"}
            {gate.blocked && (
              <span style={{ marginLeft: 10, color: C_BAD, fontWeight: 700 }}>BLOCKED</span>
            )}
          </div>

          {(!gate.matched || gate.matched.length === 0) &&
            (!gate.warnings || gate.warnings.length === 0) &&
            (!gate.confirmations || gate.confirmations.length === 0) && (
              <div style={{ color: C_MUTED, fontStyle: "italic" }}>No jurisdiction rule matched this request.</div>
            )}

          {gate.matched && gate.matched.length > 0 && (
            <div style={{ marginBottom: 8 }}>
              <div style={{ color: C_MUTED, marginBottom: 3 }}>Matched rules:</div>
              {gate.matched.map((entry, i) => {
                const { rule, action } = ruleAndAction(entry);
                return (
                  <div key={i} style={{ marginLeft: 8, marginBottom: 3 }}>
                    <b>{rule?.name ?? rule?.id ?? `rule #${i}`}</b>
                    {rule?.region && <span style={{ color: C_MUTED }}> · {rule.region}</span>}
                    {action && <span style={{ color: C_MUTED }}> — {action}</span>}
                    {rule?.reason && <div style={{ color: C_MUTED, marginLeft: 4 }}>{rule.reason}</div>}
                  </div>
                );
              })}
            </div>
          )}

          {gate.warnings && gate.warnings.length > 0 && (
            <div style={{ marginBottom: 8 }}>
              <div style={{ color: C_WARN, marginBottom: 3 }}>Warnings:</div>
              {gate.warnings.map((w, i) => (
                <div key={i} style={{ marginLeft: 8, color: C_WARN }}>
                  {w}
                </div>
              ))}
            </div>
          )}

          {gate.confirmations && gate.confirmations.length > 0 && (
            <div>
              <div style={{ color: C_MUTED, marginBottom: 3 }}>Confirmations required:</div>
              {gate.confirmations.map((entry, i) => {
                const { rule, result } = ruleAndResult(entry);
                return (
                  <div key={i} style={{ marginLeft: 8, marginBottom: 4 }}>
                    <b>{rule?.name ?? rule?.id ?? `rule #${i}`}</b>
                    {result?.decision && (
                      <span
                        style={{ marginLeft: 6, color: result.decision.toLowerCase().includes("proceed") ? "#8fe38f" : C_BAD }}
                      >
                        {result.decision}
                      </span>
                    )}
                    {typeof result?.confidence === "number" && (
                      <span style={{ color: C_MUTED }}> ({result.confidence.toFixed(2)})</span>
                    )}
                    {result?.reasoning && <div style={{ color: C_MUTED, marginLeft: 4 }}>{result.reasoning}</div>}
                  </div>
                );
              })}
            </div>
          )}
        </div>
      )}
    </div>
  );
};

export default JurisdictionGateSection;
