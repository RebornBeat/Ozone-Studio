/**
 * TaskDetailPanel — orchestrator/task status: per-step action, status,
 * model_used, tokens, and error; lets you rewind a step and rerun it with
 * a different model, optionally carrying forward earlier steps' original
 * outputs as context. Defaults to the task the chat prompt most recently
 * started (store.lastTaskId), but any task_id can be viewed.
 *
 * Backend: GET-via-POST /task/get (now includes `steps`), /task/cancel,
 * /task/step/rerun (src/grpc/mod.rs).
 */
import React, { useEffect, useState } from "react";
import { useOzoneStore } from "../services/store";

interface TaskStep {
  step_index: number;
  action: string;
  pipeline_id: number;
  status: string;
  tokens_used: number;
  output_summary?: string | null;
  error?: string | null;
  model_used?: string | null;
}

interface ThinkingEntry {
  stage: string;
  raw_response: string;
  tokens_used?: number;
  model_used?: string;
  eval_tokens_per_sec?: number;
  prompt_eval_tokens_per_sec?: number;
  load_time_ms?: number;
}

interface TaskInfo {
  task_id: number;
  status: string;
  progress: number;
  error?: string | null;
  steps: TaskStep[];
  thinking_log?: ThinkingEntry[];
}

async function fetchTask(taskId: number): Promise<TaskInfo | null> {
  const oz = (window as any).ozone;
  if (oz?.task?.status) return oz.task.status(taskId);
  const out = await fetch(
    `${(window as any).OZONE_HOST_URL || "http://127.0.0.1:50051"}/task/get`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ task_id: taskId, session_token: "" }),
    },
  ).then((r) => r.json());
  return out;
}

export const TaskDetailPanel: React.FC = () => {
  const lastTaskId = useOzoneStore((s) => s.lastTaskId);
  const availableModels = useOzoneStore((s) => s.availableModels);
  const [taskIdInput, setTaskIdInput] = useState<string>("");
  const [task, setTask] = useState<TaskInfo | null>(null);
  const [error, setError] = useState("");
  const [rerunStep, setRerunStep] = useState<number | null>(null);
  const [rerunModel, setRerunModel] = useState("");
  const [carryForward, setCarryForward] = useState(true);
  const [rerunning, setRerunning] = useState(false);
  const [expandedThinking, setExpandedThinking] = useState<Set<number>>(new Set());

  const activeTaskId = taskIdInput ? Number(taskIdInput) : lastTaskId;

  useEffect(() => {
    if (lastTaskId && !taskIdInput) setTaskIdInput(String(lastTaskId));
  }, [lastTaskId]);

  useEffect(() => {
    if (!activeTaskId) return;
    let cancelled = false;
    const poll = async () => {
      try {
        const t = await fetchTask(activeTaskId);
        if (!cancelled) {
          setTask(t);
          setError("");
        }
      } catch (e: any) {
        if (!cancelled) setError(e?.message ?? "Failed to load task");
      }
    };
    poll();
    const interval = setInterval(poll, 2000);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, [activeTaskId]);

  const cancelTask = async () => {
    if (!activeTaskId) return;
    try {
      await (window as any).ozone?.task?.cancel?.(activeTaskId);
    } catch (e) {
      console.warn("Cancel failed:", e);
    }
  };

  const doRerun = async (stepIndex: number) => {
    if (!activeTaskId) return;
    setRerunning(true);
    try {
      const modelOverride = rerunModel
        ? { model_identifier: rerunModel }
        : null;
      const out = await (window as any).ozone?.task?.rerunStep?.(
        activeTaskId,
        stepIndex,
        modelOverride,
        carryForward,
      );
      if (out?.success === false) {
        setError(out?.error ?? "Rerun failed");
      } else {
        setError("");
      }
      const t = await fetchTask(activeTaskId);
      setTask(t);
    } catch (e: any) {
      setError(e?.message ?? "Rerun failed");
    } finally {
      setRerunning(false);
      setRerunStep(null);
    }
  };

  const isRunning = task && ["running", "queued", "inprogress"].some((s) =>
    (task.status ?? "").toLowerCase().includes(s),
  );

  return (
    <div className="ocard" style={{ marginTop: 14 }}>
      <h4>🧭 Task / Orchestrator Status</h4>
      <div className="ofield" style={{ display: "flex", gap: 10, alignItems: "flex-end" }}>
        <div style={{ flex: 1 }}>
          <label>Task ID</label>
          <input
            className="oinput"
            placeholder="most recent chat task by default"
            value={taskIdInput}
            onChange={(e) => setTaskIdInput(e.target.value)}
          />
        </div>
        {isRunning && (
          <button className="obtn" onClick={cancelTask} style={{ color: "#f87171" }}>
            Stop
          </button>
        )}
      </div>

      {error && <p style={{ color: "#f87171", fontSize: 12.5 }}>{error}</p>}

      {!activeTaskId ? (
        <div className="oempty">No task selected yet — send a chat message, or enter a task id above.</div>
      ) : !task ? (
        <div className="oempty">Loading task {activeTaskId}…</div>
      ) : (
        <>
          <div className="ostats">
            <div className="ostat">
              <div className="ostat-num" style={{ fontSize: 13 }}>{task.status}</div>
              <div className="ostat-label">Status</div>
            </div>
            <div className="ostat">
              <div className="ostat-num">{Math.round((task.progress ?? 0) * 100)}%</div>
              <div className="ostat-label">Progress</div>
            </div>
            <div className="ostat">
              <div className="ostat-num">{task.steps?.length ?? 0}</div>
              <div className="ostat-label">Steps recorded</div>
            </div>
          </div>

          {(task.steps ?? []).length === 0 ? (
            <div className="oempty">No steps recorded yet.</div>
          ) : (
            <table className="otable">
              <thead>
                <tr>
                  <th>#</th>
                  <th>Action</th>
                  <th>Pipeline</th>
                  <th>Status</th>
                  <th>Model</th>
                  <th>Tokens</th>
                  <th>Error</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {(task.steps ?? []).map((s) => {
                  const prevModel = task.steps.find((p) => p.step_index === s.step_index - 1)?.model_used;
                  const switched = s.model_used && prevModel && s.model_used !== prevModel;
                  return (
                    <React.Fragment key={s.step_index}>
                      {switched && (
                        <tr>
                          <td colSpan={8} style={{ color: "#c084fc", fontSize: 11.5, textAlign: "center" }}>
                            ⇄ switched to {s.model_used}
                          </td>
                        </tr>
                      )}
                      <tr>
                        <td className="omono">{s.step_index}</td>
                        <td style={{ color: "#e8eef6" }}>{s.action}</td>
                        <td className="omono">{s.pipeline_id}</td>
                        <td>
                          <span className={`ochip ${s.status === "completed" ? "agent" : s.status === "failed" ? "observer" : "cat"}`}>
                            {s.status}
                          </span>
                        </td>
                        <td className="omono" style={{ fontSize: 11 }}>{s.model_used ?? "—"}</td>
                        <td className="omono">{s.tokens_used}</td>
                        <td style={{ color: "#f87171", fontSize: 11 }}>{s.error ?? ""}</td>
                        <td>
                          <button
                            className="obtn subtle"
                            style={{ fontSize: 11, padding: "4px 8px" }}
                            onClick={() => {
                              setRerunStep(rerunStep === s.step_index ? null : s.step_index);
                              setRerunModel(s.model_used ?? "");
                            }}
                          >
                            Rerun ⇄
                          </button>
                        </td>
                      </tr>
                      {rerunStep === s.step_index && (
                        <tr>
                          <td colSpan={8}>
                            <div
                              style={{
                                display: "flex",
                                gap: 10,
                                alignItems: "center",
                                flexWrap: "wrap",
                                padding: "8px 4px",
                                background: "#0e1524",
                                borderRadius: 8,
                              }}
                            >
                              {availableModels.length > 0 ? (
                                <select
                                  className="oinput"
                                  style={{ maxWidth: 220 }}
                                  value={rerunModel}
                                  onChange={(e) => setRerunModel(e.target.value)}
                                >
                                  <option value="">(default model)</option>
                                  {availableModels.map((m) => (
                                    <option key={m.identifier} value={m.identifier}>
                                      {m.name}
                                    </option>
                                  ))}
                                </select>
                              ) : (
                                <input
                                  className="oinput"
                                  style={{ maxWidth: 220 }}
                                  placeholder="model identifier (blank = default)"
                                  value={rerunModel}
                                  onChange={(e) => setRerunModel(e.target.value)}
                                />
                              )}
                              <label className="otoggle" style={{ margin: 0 }}>
                                <input
                                  type="checkbox"
                                  checked={carryForward}
                                  onChange={(e) => setCarryForward(e.target.checked)}
                                />
                                <span>Carry forward earlier steps' original outputs as context</span>
                              </label>
                              <button
                                className="obtn primary"
                                disabled={rerunning}
                                onClick={() => doRerun(s.step_index)}
                              >
                                {rerunning ? "Running…" : "Run"}
                              </button>
                            </div>
                          </td>
                        </tr>
                      )}
                    </React.Fragment>
                  );
                })}
              </tbody>
            </table>
          )}

          {(task.thinking_log ?? []).length > 0 && (
            <div style={{ marginTop: 16 }}>
              <div className="ostat-label" style={{ marginBottom: 8 }}>
                Thinking cycle ({task.thinking_log!.length} calls)
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                {task.thinking_log!.map((t, i) => {
                  const expanded = expandedThinking.has(i);
                  return (
                    <div
                      key={i}
                      style={{
                        border: "1px solid #223046",
                        borderRadius: 8,
                        padding: "8px 10px",
                        fontSize: 12,
                      }}
                    >
                      <div
                        onClick={() => {
                          setExpandedThinking((prev) => {
                            const next = new Set(prev);
                            if (next.has(i)) next.delete(i);
                            else next.add(i);
                            return next;
                          });
                        }}
                        style={{
                          display: "flex",
                          justifyContent: "space-between",
                          gap: 8,
                          cursor: "pointer",
                          opacity: 0.8,
                        }}
                      >
                        <span>{expanded ? "▾" : "▸"} {t.stage}</span>
                        <span style={{ opacity: 0.6 }}>
                          {t.model_used ? `${t.model_used} · ` : ""}
                          {t.tokens_used != null ? `${t.tokens_used} tok` : ""}
                          {t.eval_tokens_per_sec != null
                            ? ` · ${t.eval_tokens_per_sec.toFixed(1)} tok/s`
                            : ""}
                        </span>
                      </div>
                      {expanded && (
                        <div style={{ whiteSpace: "pre-wrap", marginTop: 6, opacity: 0.85 }}>
                          {t.raw_response}
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
};

export default TaskDetailPanel;
