/**
 * DiscoveryEffects — real celebration flourishes for genuine backend
 * discoveries during an in-flight `/orchestrate` call. Mounted by the
 * coordinator into `components/MetaPortion.tsx`; this file owns only the
 * effect logic and rendering.
 *
 * Two real, verified event sources (both via `getGraphEventClient()`,
 * `../graphEventClient`):
 *
 * 1. `onOrchestrationStage` — real per-stage completion frames
 *    (`src/orchestration_events.rs`, emitted from the real
 *    `record_stage`/`record_stage_timed` choke point in
 *    `src/orchestrator/mod.rs`). This is the PRIMARY signal here: its
 *    `summary` text is real, stage-produced data, confirmed directly
 *    against the exact `format!(...)` call sites that build it (not
 *    guessed, not live-sampled-then-assumed-stable):
 *      - Stage 3 "Gather Methodologies" (`src/orchestrator/stages.rs`):
 *        `"Methodologies: {n}, Categories: {n} ({n} created)"`
 *      - Stage 5 "Build AMT" (`src/orchestrator/amt.rs:31`):
 *        `"Mode: {AmtBuildMode:?}, Intents: {n}, Branches: {n}, Details: {n}, Cross-refs: {n}, Passes: {n}, Validated: {bool}"`
 *      - Stage 6 "Blueprint Assignment" (`src/orchestrator/stages.rs`, two
 *        real branches): `"Using existing blueprint {id} (match: {pct}%)"`
 *        OR `"Created new blueprint with {n} steps (missing: {n})"`
 *    The chat only ever sends `user_id: 1, device_id: 1` today
 *    (`components/MetaPortion.tsx`, hardcoded) — filtered to those ids as a
 *    known simplification until per-request scoping exists.
 *
 * 2. `onEvent` (graph_event) — LIVE-VERIFIED during a real orchestrate run
 *    (a real "Explain what a binary search tree is" call, watched over
 *    `/ws` for 180s): only `ModalityGraph` and `CoordinationEvent`
 *    container types actually rippled in that window. `ModalityGraph`
 *    creation gets its own pulse here ("knowledge graph created") since
 *    it's a real, user-meaningful moment. `CoordinationEvent` is
 *    deliberately NOT surfaced — it fires for internal agent-coordination
 *    bookkeeping (this project's own multi-agent handoff/claim/release
 *    notes), not something a chat user would recognize as a "discovery".
 *    AMT (`Derived`) / `Methodology` / `Blueprint` container creation was
 *    NOT observed rippling via graph_event within the test window (the run
 *    hadn't reached those stages yet in 180s) — rather than guess whether
 *    they ripple the same way, this file sources those three discoveries
 *    from the orchestration_stage summaries above instead, which ARE fully
 *    verified. If a later fork confirms those container types also ripple
 *    live, graph_event-driven pulses for them can be added alongside the
 *    stage-summary ones without conflicting (dedupe is per discovery kind
 *    + stage, not per event source).
 */
import React, { useEffect, useRef, useState } from "react";
import { getGraphEventClient, GraphEventFrame, OrchestrationStageFrame } from "../graphEventClient";

const CHAT_USER_ID = 1;
const CHAT_DEVICE_ID = 1;

type DiscoveryKind = "methodology" | "amt" | "blueprint" | "graph";

interface Discovery {
  id: string;
  kind: DiscoveryKind;
  title: string;
  detail?: string;
}

const KIND_STYLE: Record<DiscoveryKind, { glyph: string; color: string }> = {
  methodology: { glyph: "✧", color: "#8fe38f" }, // ✧ green, matches text modality hue
  amt: { glyph: "◈", color: "#ffb95f" }, // ◈ amber, matches math/AMT hue
  blueprint: { glyph: "⌘", color: "#5fb3ff" }, // ⌘ blue, matches code hue
  graph: { glyph: "◉", color: "#c792ea" }, // ◉ violet, distinct from the three modality hues
};

// ── Real summary parsers — each returns null on any mismatch; a parse miss
// is silently skipped (console.debug only), never a fabricated fallback. ──

function parseMethodologies(summary: string): Discovery | null {
  const m = summary.match(/Methodologies:\s*(\d+)/);
  if (!m) return null;
  const count = Number(m[1]);
  if (!Number.isFinite(count) || count <= 0) return null; // a real "0 found" is not an achievement
  const cat = summary.match(/Categories:\s*(\d+)\s*\((\d+)\s*created\)/);
  const detail = cat ? `${cat[1]} categor${cat[1] === "1" ? "y" : "ies"}, ${cat[2]} new` : undefined;
  return {
    id: `methodology-${Date.now()}`,
    kind: "methodology",
    title: `${count} methodolog${count === 1 ? "y" : "ies"} found`,
    detail,
  };
}

function parseAmt(summary: string): Discovery | null {
  const intents = summary.match(/Intents:\s*(\d+)/);
  const branches = summary.match(/Branches:\s*(\d+)/);
  const details = summary.match(/Details:\s*(\d+)/);
  if (!intents && !branches && !details) return null;
  const parts: string[] = [];
  if (intents) parts.push(`${intents[1]} intent${intents[1] === "1" ? "" : "s"}`);
  if (branches && Number(branches[1]) > 0) parts.push(`${branches[1]} branch${branches[1] === "1" ? "" : "es"}`);
  if (details && Number(details[1]) > 0) parts.push(`${details[1]} detail${details[1] === "1" ? "" : "s"}`);
  return {
    id: `amt-${Date.now()}`,
    kind: "amt",
    title: "Knowledge structure built",
    detail: parts.length > 0 ? parts.join(", ") : undefined,
  };
}

function parseBlueprint(summary: string): Discovery | null {
  const existing = summary.match(/Using existing blueprint\s+(\S+)\s+\(match:\s*([\d.]+)%\)/);
  if (existing) {
    return {
      id: `blueprint-${Date.now()}`,
      kind: "blueprint",
      title: "Blueprint matched",
      detail: `${existing[2]}% match (#${existing[1]})`,
    };
  }
  const created = summary.match(/Created new blueprint with\s+(\d+)\s+steps/);
  if (created) {
    return {
      id: `blueprint-${Date.now()}`,
      kind: "blueprint",
      title: "New blueprint created",
      detail: `${created[1]} step${created[1] === "1" ? "" : "s"}`,
    };
  }
  return null;
}

function fromStageFrame(frame: OrchestrationStageFrame): Discovery | null {
  if (!frame.success) return null;
  switch (frame.stage_name) {
    case "Gather Methodologies":
      return parseMethodologies(frame.summary);
    case "Build AMT":
      return parseAmt(frame.summary);
    case "Blueprint Assignment":
      return parseBlueprint(frame.summary);
    default:
      return null;
  }
}

function fromGraphEvent(frame: GraphEventFrame): Discovery | null {
  if (frame.event !== "created") return null;
  if (frame.container_type !== "ModalityGraph") return null; // see file header — the only live-verified type worth surfacing here
  return {
    id: `graph-${frame.container_id}-${frame.timestamp}`,
    kind: "graph",
    title: "Knowledge graph created",
  };
}

const AUTO_DISMISS_MS = 4200;
const STAGGER_MS = 260;

export const DiscoveryEffects: React.FC<{ isRunning: boolean }> = ({ isRunning }) => {
  const [visible, setVisible] = useState<Discovery[]>([]);
  const queueRef = useRef<Discovery[]>([]);
  const drainingRef = useRef(false);

  useEffect(() => {
    if (!isRunning) {
      queueRef.current = [];
      drainingRef.current = false;
      setVisible([]);
      return;
    }

    const enqueue = (d: Discovery | null) => {
      if (!d) return;
      queueRef.current.push(d);
      drain();
    };

    function drain() {
      if (drainingRef.current) return;
      drainingRef.current = true;
      const step = () => {
        const next = queueRef.current.shift();
        if (!next) {
          drainingRef.current = false;
          return;
        }
        setVisible((prev) => [...prev, next]);
        window.setTimeout(() => {
          setVisible((prev) => prev.filter((d) => d.id !== next.id));
        }, AUTO_DISMISS_MS);
        window.setTimeout(step, STAGGER_MS);
      };
      step();
    }

    const client = getGraphEventClient();
    client.connect();

    const offStage = client.onOrchestrationStage((frame) => {
      if (frame.user_id !== CHAT_USER_ID || frame.device_id !== CHAT_DEVICE_ID) return;
      try {
        enqueue(fromStageFrame(frame));
      } catch (err) {
        // eslint-disable-next-line no-console
        console.debug("[DiscoveryEffects] stage summary parse skipped", frame.stage_name, err);
      }
    });
    const offGraph = client.onEvent((frame) => {
      try {
        enqueue(fromGraphEvent(frame));
      } catch (err) {
        // eslint-disable-next-line no-console
        console.debug("[DiscoveryEffects] graph_event parse skipped", frame.container_type, err);
      }
    });

    return () => {
      offStage();
      offGraph();
    };
  }, [isRunning]);

  if (visible.length === 0) return null;

  return (
    <div
      style={{
        position: "absolute",
        right: 12,
        bottom: 64,
        display: "flex",
        flexDirection: "column",
        gap: 6,
        pointerEvents: "none",
        zIndex: 30,
      }}
    >
      {visible.map((d) => {
        const style = KIND_STYLE[d.kind];
        return (
          <div
            key={d.id}
            className="discovery-effect-toast"
            style={{
              display: "flex",
              alignItems: "center",
              gap: 8,
              background: "#101724",
              border: `1px solid ${style.color}55`,
              borderRadius: 10,
              padding: "7px 12px",
              boxShadow: `0 0 14px ${style.color}33, 0 4px 10px rgba(0,0,0,0.35)`,
              minWidth: 180,
              maxWidth: 260,
            }}
          >
            <span
              style={{
                fontSize: 15,
                color: style.color,
                textShadow: `0 0 8px ${style.color}`,
                flex: "none",
              }}
            >
              {style.glyph}
            </span>
            <div style={{ display: "flex", flexDirection: "column", minWidth: 0 }}>
              <span style={{ fontSize: 12, color: "var(--color-text)", fontWeight: 600, lineHeight: 1.3 }}>{d.title}</span>
              {d.detail && (
                <span style={{ fontSize: 10.5, color: "var(--color-text-muted)", lineHeight: 1.3 }}>{d.detail}</span>
              )}
            </div>
          </div>
        );
      })}
      <style>{`
        .discovery-effect-toast {
          animation: discoveryToastIn 320ms cubic-bezier(0.22, 1, 0.36, 1), discoveryToastOut 420ms ease-in ${AUTO_DISMISS_MS - 420}ms forwards;
        }
        @keyframes discoveryToastIn {
          from { opacity: 0; transform: translateY(10px) scale(0.94); }
          to   { opacity: 1; transform: translateY(0) scale(1); }
        }
        @keyframes discoveryToastOut {
          from { opacity: 1; transform: translateY(0) scale(1); }
          to   { opacity: 0; transform: translateY(-6px) scale(0.96); }
        }
      `}</style>
    </div>
  );
};

export default DiscoveryEffects;
