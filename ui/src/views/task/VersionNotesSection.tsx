/**
 * H3 — Version notes display (Task Viewer).
 *
 * Real source: `StoredTaskStep.version`/`version_notes: Vec<StepVersionNote>`
 * (src/task/mod.rs:511-514, `StepVersionNote{version,note,timestamp,change_type}`
 * at :634-639) — `#[serde(default)]`, so both fields are ALWAYS present on
 * every real step, never absent. `/task/get` (src/grpc/mod.rs) already
 * returns them inline on every step — confirmed live against real tasks
 * 6/8/9/10/11/12/13/14 (all real, all currently exactly one note per step,
 * every one `{version:1, change_type:"Created", note:"Step N completed: X
 * tokens"}`). `TaskDetailPanel.tsx`'s own `TaskStep` TypeScript interface
 * does NOT declare these fields yet (a real gap in that file, out of this
 * fork's scope to fix) — `task` arrives here as `any` via
 * `TaskInsightSections`, so this file reads the raw JSON field directly
 * rather than through that narrower type.
 *
 * NOT the same mechanism as F7's `views/files/VersionHistory.tsx`, which
 * covers container-level `GetVersionHistory` (blake3 hash records, no
 * content) and text-graph `TextGraphNode.version_notes` (file/container
 * scoped) — this section is task/step scoped only.
 *
 * Every real note seen so far is `change_type:"Created"` (one note written
 * when a step first completes) — multiple versions per step would appear
 * once a step is rewound/rerun (`/task/step/rerun`, see
 * `TaskDetailPanel.tsx`'s `doRerun`), which real data doesn't show yet.
 * Rendered newest-first per step so that real future case displays
 * correctly without changes here.
 */
import React from "react";

interface StepVersionNoteRaw {
  version?: number;
  note?: string;
  timestamp?: number;
  change_type?: string;
}

interface TaskStepRaw {
  step_index?: number;
  action?: string;
  version?: number;
  version_notes?: StepVersionNoteRaw[];
}

const C_TEXT = "#dfe7f2";
const C_BODY = "#c7d0dc";
const C_MUTED = "#8b98ab";
const C_BORDER = "#1e2836";

function formatTimestamp(ts: number | undefined): string {
  if (typeof ts !== "number" || !Number.isFinite(ts)) return "unknown time";
  // Real timestamps observed are Unix seconds (e.g. 1789298467), not ms.
  const ms = ts < 2e10 ? ts * 1000 : ts;
  try {
    return new Date(ms).toLocaleString();
  } catch {
    return String(ts);
  }
}

const SectionTitle: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div style={{ fontSize: 12.5, fontWeight: 700, color: C_TEXT, marginBottom: 8 }}>{children}</div>
);

export const VersionNotesSection: React.FC<{ task: any }> = ({ task }) => {
  const steps: TaskStepRaw[] = Array.isArray(task?.steps) ? task.steps : [];
  const stepsWithNotes = steps.filter(
    (s) => Array.isArray(s.version_notes) && s.version_notes.length > 0,
  );

  return (
    <div className="ocard">
      <SectionTitle>Version notes</SectionTitle>
      {steps.length === 0 && (
        <div style={{ fontSize: 12, color: C_MUTED }}>No steps recorded yet for this task.</div>
      )}
      {steps.length > 0 && stepsWithNotes.length === 0 && (
        <div style={{ fontSize: 12, color: C_MUTED }}>
          No version notes recorded on any step of this task yet. Every real step writes one when it
          first completes (<code>change_type: "Created"</code>) — a rewind/rerun of a step would add a
          further note here.
        </div>
      )}
      {stepsWithNotes.map((step, i) => {
        const notes = [...(step.version_notes ?? [])].sort(
          (a, b) => (b.timestamp ?? 0) - (a.timestamp ?? 0),
        );
        return (
          <div
            key={step.step_index ?? i}
            style={{
              marginBottom: 12,
              paddingBottom: 10,
              borderBottom: i < stepsWithNotes.length - 1 ? `1px solid ${C_BORDER}` : "none",
            }}
          >
            <div style={{ fontSize: 12, color: C_TEXT, marginBottom: 4 }}>
              Step {step.step_index ?? "?"}
              {step.action ? <span style={{ color: C_MUTED }}> — {step.action}</span> : null}
              <span style={{ color: C_MUTED }}> · current version {step.version ?? notes[0]?.version ?? 1}</span>
            </div>
            {notes.map((note, j) => (
              <div
                key={j}
                style={{
                  fontSize: 11.5,
                  color: C_BODY,
                  display: "flex",
                  gap: 8,
                  alignItems: "baseline",
                  marginLeft: 8,
                  marginBottom: 2,
                }}
              >
                <span style={{ color: C_MUTED, minWidth: 34 }}>v{note.version ?? "?"}</span>
                <span style={{ flex: 1 }}>{note.note ?? "(no note text recorded)"}</span>
                <span style={{ color: C_MUTED, fontSize: 11 }}>{note.change_type ?? "unknown"}</span>
                <span style={{ color: C_MUTED, fontSize: 11, whiteSpace: "nowrap" }}>
                  {formatTimestamp(note.timestamp)}
                </span>
              </div>
            ))}
          </div>
        );
      })}
    </div>
  );
};

export default VersionNotesSection;
