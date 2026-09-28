/**
 * H6 — Simulation prediction display per task.
 *
 * VERIFIED LIVE (2026-09-27, direct source read, not inherited from the plan
 * doc's guess): stage 4's real per-step predictions (`SimulationOutcome` —
 * `overall_feasibility` + `step_predictions[]{step,needs,produces,risks}`,
 * `src/orchestrator/mod.rs:1217-1220`) ARE genuinely computed and stored on
 * `state.simulation_result` (`src/orchestrator/stages.rs:1259`), and ARE
 * genuinely consumed one stage later to build stage 5's consciousness-gate
 * prompt (`stages.rs:1321`). But the chain stops there:
 *
 *   OrchestrationState.simulation_result (pub(crate), request-scoped only)
 *     -> read once by stage_5_consciousness_gate to render prompt text
 *     -> DISCARDED. Never copied onto `OrchestrationResponse`
 *        (`src/orchestrator/mod.rs:215-245` has no simulation field at all —
 *        compare `jurisdiction_gate`, which WAS deliberately surfaced there).
 *     -> Never persisted onto the `Task` record either, so `/task/get`'s
 *        `TaskInfo` (`src/grpc/mod.rs:497-529`) has no simulation field —
 *        confirmed by reading the handler directly, not the TS type alone.
 *
 * So today there is NO reachable API surface for this data, live or
 * historical — not a UI gap, a real backend threading gap. This component
 * does not fake a prediction or poll a nonexistent field; it states the
 * exact gap above so whoever threads it through later has a precise target:
 * add `simulation_result: Option<SimulationOutcome>` to `OrchestrationResponse`
 * (mirroring how `jurisdiction_gate` was added), OR persist it onto the
 * `Task` record the way `amt_summary`/`thinking_log` already are.
 */
import React from "react";

export interface SimulationSectionProps {
  task: any;
}

export const SimulationSection: React.FC<SimulationSectionProps> = () => (
  <div>
    <div style={{ fontSize: 12.5, fontWeight: 700, color: "#dfe7f2", marginBottom: 6 }}>
      Simulation prediction (stage 4)
    </div>
    <div
      style={{
        fontSize: 12,
        color: "#8b98ab",
        lineHeight: 1.6,
        border: "1px solid #1e2836",
        borderRadius: 8,
        padding: "10px 12px",
      }}
    >
      Not available yet — this is a real backend gap, not a missing UI feature.
      <br />
      <br />
      Stage 4 of orchestration genuinely computes a real per-step feasibility
      prediction (<code>SimulationOutcome</code>) and stage 5 genuinely reads
      it to build the consciousness-gate prompt — but the result is discarded
      immediately after: it's never added to the orchestration response and
      never saved onto the task record, so there's no API path to it once the
      request finishes, and no way to show it here.
    </div>
  </div>
);

export default SimulationSection;
