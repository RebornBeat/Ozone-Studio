/**
 * TaskManager UI Component (Pipeline #5)
 *
 * Task list and detail view. This is a CORE TAB - never uninjected.
 *
 * This REPLACES TaskViewer (#36) - all task viewing is now here.
 * Shows running tasks, completed tasks, task steps, and timelines.
 */

module.exports = {
  meta: {
    title: "Tasks",
    icon: "📋",
    version: "0.4.0",
  },

  render: function (container, props, React, ReactDOM) {
    const { useState, useEffect, useCallback } = React;
    const { executePipeline, subscribeToState } = props;

      // Engine registry — modality → engine UI mapping (K-registry consumers;
      // engines appear when their modality graph exists).
      const ENGINE_REGISTRY = {
        text: { label: "Text Analysis", icon: "📝", pipelineId: 100 },
        code: { label: "Code Intel", icon: "◻", pipelineId: 101 },
        image: { label: "Image", icon: "🖼", pipelineId: 102 },
        audio: { label: "Audio", icon: "🎵", pipelineId: 103 },
        video: { label: "Video", icon: "🎬", pipelineId: 104 },
        math: { label: "Proof", icon: "∑", pipelineId: 105 },
        chemistry: { label: "Molecular", icon: "⚗", pipelineId: 106 },
        dna: { label: "Genome", icon: "🧬", pipelineId: 107 },
        eeg: { label: "Neural Signal", icon: "🧠", pipelineId: 108 },
      };

    function TasksUI(
) {
      const [tasks, setTasks] = useState([]);
      const [selectedTask, setSelectedTask] = useState(null);
      const [contextViewerInjected, setContextViewerInjected] = useState(false);
      const [taskDetails, setTaskDetails] = useState(null);
      const [filter, setFilter] = useState("all"); // all, running, completed, failed
      const [loading, setLoading] = useState(true);
      const [error, setError] = useState(null);
      const [clarificationInput, setClarificationInput] = useState("");

      // Load tasks on mount and poll for updates
      useEffect(() => {
        loadTasks();
        const interval = setInterval(loadTasks, 5000); // Poll every 5s
        return () => clearInterval(interval);
      }, []);

      // 2026-10-16 fix: this used to call executePipeline(5, {action:"List"})
      // — pipeline 5's own TASK_MANAGER process, backed by its own separate
      // tasks.json file store. Confirmed (grep across src/) that the real
      // orchestrate() flow NEVER writes a task into that store (the only
      // call site touching pipeline 5 at all is graphs.rs's RegisterGraphs,
      // which assumes a task already exists there and silently no-ops when
      // it doesn't) — so this tab showed "No tasks yet" regardless of real
      // activity. The REAL task record lives in the host's own in-process
      // TaskManager, reachable here via the dedicated window.ozone.task
      // bridge (preload.js -> ipcMain "task:list"/"task:status"/"task:cancel"
      // -> POST /task/list, /task/get, /task/cancel) — the same real source
      // ui/src/components/TaskDetailPanel.tsx already reads from. Switched
      // to that; field names normalized below since TaskInfo's real key is
      // `task_id`, not `id`.
      const loadTasks = async () => {
        try {
          const result = await window.ozone.task.list();
          const mapped = (result?.tasks || []).map((t) => ({
            id: t.task_id,
            task_id: t.task_id,
            status: t.status,
            progress: t.progress,
            created_at: t.created_at,
            started_at: t.started_at,
            completed_at: t.completed_at,
            duration_secs:
              t.started_at != null && t.completed_at != null
                ? t.completed_at - t.started_at
                : null,
            error: t.error,
            pipeline_name: t.blueprint_name,
          }));
          setTasks(mapped);
          setLoading(false);
          setError(null);
        } catch (e) {
          console.error("Failed to load tasks:", e);
          setError("Failed to load tasks");
          setLoading(false);
        }
      };

      // Real /task/get via the same window.ozone.task bridge (see loadTasks).
      // Returns the real TaskInfo directly (not pipeline 5's fabricated
      // "GetDetails" shape) — real `steps`/`thinking_log`/`amt_summary`,
      // confirmed already populated correctly for real tasks earlier this
      // session (A1). Real TaskInfo has no `pending_clarification` field —
      // the clarification banner below this file's render path will
      // correctly never show for a real task rather than show a fake one;
      // this was never wired to real data under the old pipeline-5 path
      // either, so this is not a regression.
      const loadTaskDetails = async (taskId) => {
        try {
          const result = await window.ozone.task.status(taskId);
          setTaskDetails(result ?? null);
        } catch (e) {
          console.error("Failed to load task details:", e);
        }
      };

      const handleSelectTask = async (task) => {
        setSelectedTask(task);
        await loadTaskDetails(task.id);

        // Context Viewer injection — when the task has graph data, inject the
        // Context tab (pipeline #55); un-inject when it doesn't.
        try {
          const graphResult = await executePipeline(55, {
            action: "GetTaskGraphs",
            task_id: task.task_id || task.id,
          });
          const hasGraphs = (graphResult?.output?.graphs?.length || 0) > 0;

          if (hasGraphs && !contextViewerInjected) {
            if (window.__ozoneThemeArea?.injectTab) {
              await window.__ozoneThemeArea.injectTab(55, {
                id: "context",
                label: "Context",
                icon: "◈",
                makeActive: false,
                fromBackend: false,
                initialData: { taskId: task.task_id || task.id },
              });
              setContextViewerInjected(true);
              window.__ozoneThemeArea.addNotification?.(
                "Graph context available for task — view in Context tab",
                "info",
                "context",
              );
            }
          } else if (!hasGraphs && contextViewerInjected) {
            if (window.__ozoneThemeArea?.uninjectTab) {
              window.__ozoneThemeArea.uninjectTab("context");
              setContextViewerInjected(false);
            }
          }
        } catch (e) {
          // Context Viewer is not critical — ignore failures silently
        }
      };

      // Un-inject the Context tab when this component unmounts.
      useEffect(() => {
        return () => {
          if (contextViewerInjected && window.__ozoneThemeArea?.uninjectTab) {
            window.__ozoneThemeArea.uninjectTab("context");
          }
        };
      }, [contextViewerInjected]);

      // Real /task/cancel via the same bridge as loadTasks/loadTaskDetails.
      const handleCancelTask = async (taskId) => {
        try {
          await window.ozone.task.cancel(taskId);
          loadTasks();
        } catch (e) {
          console.error("Failed to cancel task:", e);
        }
      };

      // No real backend equivalent exists for "retry a whole task" (only
      // /task/step/rerun — a single-step, model-override rerun, a different
      // operation). Left calling the disconnected pipeline 5 would silently
      // no-op (same shape as the bug this whole fix addresses) — surface
      // that honestly instead of pretending it worked.
      const handleRetryTask = async (taskId) => {
        setError(
          "Retry is not available yet — only per-step rerun (via Step Detail) is wired to a real backend operation.",
        );
      };

      const handleClarificationResponse = async (taskId, response) => {
        try {
          await executePipeline(5, {
            action: "SubmitClarification",
            task_id: taskId,
            request_id: response.request_id,
            response_type: response.response_type,
            selected_option: response.selected_option || null,
            free_text: response.free_text || null,
          });
          setClarificationInput("");
          await loadTaskDetails(taskId);
          await loadTasks();
        } catch (e) {
          console.error("Failed to submit clarification:", e);
        }
      };

      // Filter tasks
      const filteredTasks = tasks.filter((task) => {
        if (filter === "all") return true;
        if (filter === "running")
          return task.status === "running" || task.status === "queued";
        if (filter === "completed") return task.status === "completed";
        if (filter === "failed")
          return task.status === "failed" || task.status === "cancelled";
        return true;
      });

      const getStatusIcon = (status) => {
        switch (status) {
          case "running":
            return "⏳";
          case "queued":
            return "⏸️";
          case "completed":
            return "✅";
          case "failed":
            return "❌";
          case "cancelled":
            return "🚫";
          default:
            return "❓";
        }
      };

      const formatTime = (timestamp) => {
        if (!timestamp) return "-";
        const date = new Date(timestamp * 1000);
        return date.toLocaleString();
      };

      const formatDuration = (seconds) => {
        if (!seconds) return "-";
        if (seconds < 60) return `${seconds}s`;
        const mins = Math.floor(seconds / 60);
        const secs = seconds % 60;
        return `${mins}m ${secs}s`;
      };

      if (loading) {
        return React.createElement(
          "div",
          { className: "tasks-loading" },
          React.createElement("div", { className: "loading-spinner" }),
          React.createElement("p", null, "Loading tasks..."),
        );
      }

      return React.createElement(
        "div",
        { className: "tasks-panel" },
        // Toolbar
        React.createElement(
          "div",
          { className: "tasks-toolbar" },
          React.createElement(
            "div",
            { className: "filter-buttons" },
            ["all", "running", "completed", "failed"].map((f) =>
              React.createElement(
                "button",
                {
                  key: f,
                  className: `filter-btn ${filter === f ? "active" : ""}`,
                  onClick: () => setFilter(f),
                },
                f.charAt(0).toUpperCase() + f.slice(1),
              ),
            ),
          ),
          React.createElement(
            "button",
            {
              className: "btn-icon refresh-btn",
              onClick: loadTasks,
              title: "Refresh",
            },
            "🔄",
          ),
        ),

        // Task list and details
        React.createElement(
          "div",
          { className: "tasks-content" },
          // Task list
          React.createElement(
            "div",
            { className: "tasks-list" },
            filteredTasks.length === 0
              ? React.createElement(
                  "div",
                  { className: "empty-state-centered" },
                  React.createElement(
                    "span",
                    { className: "empty-icon" },
                    "📋",
                  ),
                  React.createElement(
                    "p",
                    null,
                    filter === "all" ? "No tasks yet" : `No ${filter} tasks`,
                  ),
                  React.createElement(
                    "p",
                    { className: "hint" },
                    "Tasks are created when you send prompts.",
                  ),
                )
              : filteredTasks.map((task) =>
                  React.createElement(
                    "div",
                    {
                      key: task.id,
                      className: `task-item ${selectedTask?.id === task.id ? "selected" : ""} status-${task.status}`,
                      onClick: () => handleSelectTask(task),
                    },
                    React.createElement(
                      "span",
                      { className: "task-status" },
                      getStatusIcon(task.status),
                    ),
                    React.createElement(
                      "div",
                      { className: "task-info" },
                      React.createElement(
                        "span",
                        { className: "task-name" },
                        task.pipeline_name || `Task #${task.id}`,
                      ),
                      React.createElement(
                        "span",
                        { className: "task-time" },
                        formatTime(task.created_at),
                      ),
                    ),
                    task.status === "running" &&
                      React.createElement(
                        "div",
                        { className: "task-progress" },
                        React.createElement("div", {
                          className: "progress-bar",
                          style: { width: `${(task.progress || 0) * 100}%` },
                        }),
                      ),
                  ),
                ),
          ),

          // Task details
          React.createElement(
            "div",
            { className: "task-details" },
            selectedTask
              ? React.createElement(
                  "div",
                  { className: "details-content" },
                  React.createElement(
                    "div",
                    { className: "details-header" },
                    React.createElement(
                      "h3",
                      null,
                      selectedTask.pipeline_name || `Task #${selectedTask.id}`,
                    ),
                    React.createElement(
                      "span",
                      {
                        className: `status-badge status-${selectedTask.status}`,
                      },
                      selectedTask.status,
                    ),
                  ),

                  selectedTask.status === "clarifying" &&
                    taskDetails?.pending_clarification &&
                    React.createElement(
                      "div",
                      { className: "clarification-banner" },
                      React.createElement(
                        "div",
                        { className: "clarification-icon-wrap" },
                        React.createElement("span", null, "◆"),
                      ),
                      React.createElement(
                        "div",
                        { className: "clarification-body" },
                        React.createElement("h4", null, "Clarification Needed"),
                        React.createElement(
                          "p",
                          null,
                          taskDetails.pending_clarification
                            .clarification_question,
                        ),
                        taskDetails.pending_clarification
                          .multiple_choice_options?.length > 0 &&
                          React.createElement(
                            "div",
                            { className: "clarification-options" },
                            taskDetails.pending_clarification.multiple_choice_options.map(
                              (opt) =>
                                React.createElement(
                                  "button",
                                  {
                                    key: opt.label,
                                    className: "clarification-option-btn",
                                    onClick: () =>
                                      handleClarificationResponse(
                                        selectedTask.id,
                                        {
                                          request_id:
                                            taskDetails.pending_clarification
                                              .request_id || 0,
                                          response_type: "SelectedOption",
                                          selected_option: opt.label,
                                        },
                                      ),
                                  },
                                  React.createElement(
                                    "span",
                                    { className: "opt-label" },
                                    opt.label,
                                  ),
                                  React.createElement(
                                    "span",
                                    { className: "opt-text" },
                                    opt.text,
                                  ),
                                ),
                            ),
                          ),
                        taskDetails.pending_clarification.allows_free_text &&
                          React.createElement(
                            "div",
                            { className: "clarification-free-text" },
                            React.createElement("textarea", {
                              placeholder: "Or type your response...",
                              value: clarificationInput,
                              onChange: (e) =>
                                setClarificationInput(e.target.value),
                              rows: 3,
                            }),
                            React.createElement(
                              "button",
                              {
                                className: "btn-primary",
                                disabled: !clarificationInput.trim(),
                                onClick: () =>
                                  handleClarificationResponse(selectedTask.id, {
                                    request_id:
                                      taskDetails.pending_clarification
                                        .request_id || 0,
                                    response_type: "FreeText",
                                    free_text: clarificationInput,
                                  }),
                              },
                              "Submit",
                            ),
                          ),
                      ),
                    ),

                  // Task metadata
                  React.createElement(
                    "div",
                    { className: "details-meta" },
                    React.createElement(
                      "div",
                      { className: "meta-row" },
                      React.createElement(
                        "span",
                        { className: "meta-label" },
                        "Created:",
                      ),
                      React.createElement(
                        "span",
                        { className: "meta-value" },
                        formatTime(selectedTask.created_at),
                      ),
                    ),
                    selectedTask.started_at &&
                      React.createElement(
                        "div",
                        { className: "meta-row" },
                        React.createElement(
                          "span",
                          { className: "meta-label" },
                          "Started:",
                        ),
                        React.createElement(
                          "span",
                          { className: "meta-value" },
                          formatTime(selectedTask.started_at),
                        ),
                      ),
                    selectedTask.completed_at &&
                      React.createElement(
                        "div",
                        { className: "meta-row" },
                        React.createElement(
                          "span",
                          { className: "meta-label" },
                          "Completed:",
                        ),
                        React.createElement(
                          "span",
                          { className: "meta-value" },
                          formatTime(selectedTask.completed_at),
                        ),
                      ),
                    React.createElement(
                      "div",
                      { className: "meta-row" },
                      React.createElement(
                        "span",
                        { className: "meta-label" },
                        "Duration:",
                      ),
                      React.createElement(
                        "span",
                        { className: "meta-value" },
                        formatDuration(selectedTask.duration_secs),
                      ),
                    ),
                  ),

                  // Steps (if blueprint has steps)
                  taskDetails?.steps?.length > 0 &&
                    React.createElement(
                      "div",
                      { className: "task-steps" },
                      React.createElement("h4", null, "Steps"),
                      taskDetails.steps.map((step, idx) =>
                        React.createElement(
                          "div",
                          {
                            key: idx,
                            className: `step-item status-${step.status}`,
                          },
                          React.createElement(
                            "div",
                            { className: "step-header" },
                            React.createElement(
                              "span",
                              { className: "step-number" },
                              step.step_index != null
                                ? step.step_index + 1
                                : idx + 1,
                            ),
                            React.createElement(
                              "span",
                              { className: "step-name" },
                              step.pipeline_name || step.name,
                            ),
                            React.createElement(
                              "span",
                              { className: "step-status-icon" },
                              getStatusIcon(step.status),
                            ),
                            step.tokens_used > 0 &&
                              React.createElement(
                                "span",
                                {
                                  className: "step-tokens",
                                  style: {
                                    fontSize: 10,
                                    opacity: 0.6,
                                    marginLeft: 4,
                                  },
                                },
                                `${step.tokens_used}tok`,
                              ),
                          ),
                          (step.stages_completed?.length > 0 ||
                            step.current_stage ||
                            step.stages_pending?.length > 0) &&
                            React.createElement(
                              "div",
                              { className: "step-stages" },
                              [
                                ...(step.stages_completed || []).map((s) => ({
                                  name: s,
                                  state: "done",
                                })),
                                ...(step.current_stage
                                  ? [
                                      {
                                        name: step.current_stage,
                                        state: "active",
                                      },
                                    ]
                                  : []),
                                ...(step.stages_pending || []).map((s) => ({
                                  name: s,
                                  state: "pending",
                                })),
                              ].map((stage, si) =>
                                React.createElement(
                                  "div",
                                  {
                                    key: si,
                                    className: `stage-row stage-${stage.state}`,
                                  },
                                  React.createElement(
                                    "span",
                                    { className: "stage-dot" },
                                    stage.state === "done"
                                      ? "✓"
                                      : stage.state === "active"
                                        ? "●"
                                        : "○",
                                  ),
                                  React.createElement(
                                    "span",
                                    { className: "stage-name" },
                                    stage.name,
                                  ),
                                ),
                              ),
                            ),
                          (step.graph_ids_updated?.length > 0 ||
                            step.graph_ids_read?.length > 0) &&
                            React.createElement(
                              "div",
                              { className: "step-graphs" },
                              step.graph_ids_updated?.length > 0 &&
                                React.createElement(
                                  "div",
                                  { className: "graphs-updated" },
                                  React.createElement(
                                    "span",
                                    { className: "impact-label" },
                                    "↑ Modified:",
                                  ),
                                  step.graph_ids_updated.map((id) =>
                                    React.createElement(
                                      "span",
                                      {
                                        key: id,
                                        className: "graph-id-badge",
                                        title: `Graph ${id}`,
                                      },
                                      id,
                                    ),
                                  ),
                                ),
                              step.graph_ids_read?.length > 0 &&
                                React.createElement(
                                  "div",
                                  { className: "graphs-read" },
                                  React.createElement(
                                    "span",
                                    { className: "impact-label" },
                                    "→ Read:",
                                  ),
                                  step.graph_ids_read.map((id) =>
                                    React.createElement(
                                      "span",
                                      {
                                        key: id,
                                        className: "graph-id-badge secondary",
                                        title: `Graph ${id}`,
                                      },
                                      id,
                                    ),
                                  ),
                                ),
                            ),
                          step.version_notes?.length > 0 &&
                            React.createElement(
                              "div",
                              { className: "step-versions" },
                              step.version_notes.map((vn, vi) =>
                                React.createElement(
                                  "div",
                                  { key: vi, className: "step-vn" },
                                  React.createElement(
                                    "span",
                                    { className: "vn-ver" },
                                    `v${vn.version}`,
                                  ),
                                  React.createElement(
                                    "span",
                                    { className: "vn-note" },
                                    vn.note,
                                  ),
                                ),
                              ),
                            ),
                        ),
                      ),
                    ),

                  // Error message
                  selectedTask.error &&
                    React.createElement(
                      "div",
                      { className: "task-error" },
                      React.createElement("h4", null, "Error"),
                      React.createElement("p", null, selectedTask.error),
                    ),

                  // Actions
                  React.createElement(
                    "div",
                    { className: "task-actions" },
                    (selectedTask.status === "running" ||
                      selectedTask.status === "queued") &&
                      React.createElement(
                        "button",
                        {
                          className: "btn-danger",
                          onClick: () => handleCancelTask(selectedTask.id),
                        },
                        "Cancel",
                      ),
                    selectedTask.status === "failed" &&
                      React.createElement(
                        "button",
                        {
                          className: "btn-primary",
                          onClick: () => handleRetryTask(selectedTask.id),
                        },
                        "Retry",
                      ),
                  ),
                )
              : React.createElement(
                  "div",
                  { className: "empty-state-centered" },
                  React.createElement(
                    "p",
                    null,
                    "Select a task to view details",
                  ),
                ),
          ),
        ),
      );
    }

    const root = ReactDOM.createRoot(container);
    root.render(React.createElement(TasksUI));

    return () => root.unmount();
  },

  onActivate: function () {
    console.log("TaskManager UI activated");
  },

  onDeactivate: function () {
    console.log("TaskManager UI deactivated");
  },
};
