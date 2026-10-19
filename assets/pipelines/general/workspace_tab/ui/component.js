/**
 * WorkspaceTab UI Component (Pipeline #6)
 *
 * Manages workspaces and projects. This is a CORE TAB - never uninjected.
 *
 * Features:
 * - Workspace creation and management
 * - Project creation within workspaces
 * - File/URL linking via FileLink (#30) and URLLink (#31) pipelines
 * - Project-scoped chat with context aggregation
 *
 * FileLink, URLLink, PackageLink are called FROM this tab's UI,
 * they don't need their own separate tabs.
 */

module.exports = {
  meta: {
    title: "Workspace",
    icon: "📁",
    version: "0.4.0",
  },

  render: function (container, props, React, ReactDOM) {
    const { useState, useEffect, useCallback, useRef } = React;
    const { executePipeline, getSharedState, setSharedState } = props;

    // ========================================================================
    // Main WorkspaceUI Component
    // ========================================================================
    function WorkspaceUI() {
      const [workspaces, setWorkspaces] = useState([]);
      const [selectedWorkspace, setSelectedWorkspace] = useState(null);
      const [projects, setProjects] = useState([]);
      const [selectedProject, setSelectedProject] = useState(null);
      const [files, setFiles] = useState([]);
      const [loading, setLoading] = useState(true);
      const [error, setError] = useState(null);

      // W7.3 — workspace-level aggregation, client-side only (no backend
      // GetWorkspaceContext-style action exists — confirmed against
      // main.rs's real WorkspaceInput enum; building one is a separate,
      // larger backend effort, explicitly out of scope here). `aggFiles`
      // is a MERGED list of real FileRef objects fetched per-project via
      // the existing GetProjectFiles action (parallel fetch, triggered
      // whenever `projects` (re)loads — see loadWorkspaceAggregate below).
      // It intentionally does NOT include URL/package refs: GetProjectFiles
      // only ever returns FileRefs — url_link/package_link (pipelines
      // #31/#32) persist their refs to their own separate per-project
      // storage, never into the files_<id>.json this reads. The aggregate
      // counts rendered alongside instead SUM each project's own
      // file_count/url_count/package_count (from ListProjects, already
      // loaded) since that's the honest source covering all three ref
      // types. Graph/Fabric/Hierarchy aggregation across a workspace's
      // projects is NOT built here — those are fundamentally single-
      // project graph structures; faking a cross-project merge would be
      // fabrication. Deferred, not silently dropped (see
      // docs/WORKSPACE_UI_REVAMP_PLAN.md's W7.3).
      const [aggFiles, setAggFiles] = useState([]);
      const [aggLoading, setAggLoading] = useState(false);

      // Dialog states
      const [showCreateWorkspace, setShowCreateWorkspace] = useState(false);
      const [showCreateProject, setShowCreateProject] = useState(false);
      const [showLinkUrl, setShowLinkUrl] = useState(false);
      const [newName, setNewName] = useState("");
      const [newDescription, setNewDescription] = useState("");
      const [linkUrl, setLinkUrl] = useState("");

      // W7.4 — CRUD completion (Rename/Delete for both workspaces and
      // projects). `editingWorkspace`/`editingProject` branch the EXISTING
      // create-modal JSX (showCreateWorkspace/showCreateProject + the
      // shared newName/newDescription fields) between Create and Rename
      // instead of duplicating a near-identical modal block: non-null means
      // "this modal is renaming that specific item", null means "Create".
      // `deleteTarget` ({kind:'workspace'|'project', item}) drives one
      // shared confirmation modal reused from all four delete entry points
      // (sidebar item, project card, PROJECT VIEW toolbar, PROJECTS VIEW
      // toolbar) — real confirmation always required, no window.confirm(),
      // since DeleteWorkspace cascade-deletes that workspace's projects
      // file (main.rs ~line 280-302) and DeleteProject removes its linked
      // files.
      const [editingWorkspace, setEditingWorkspace] = useState(null);
      const [editingProject, setEditingProject] = useState(null);
      const [deleteTarget, setDeleteTarget] = useState(null);
      const [deleting, setDeleting] = useState(false);

      // Load workspaces on mount
      useEffect(() => {
        loadWorkspaces();
      }, []);

      // Defensive sync, not just an exit-path call (2026-10-18): every exit
      // point that nulls selectedProject (handleBack today, maybe more
      // later) also had to remember to clear the shared-state keys that
      // drive DynamicPipelineUI's <ProjectChatPanel/> sibling, or the chat
      // keeps showing for a project this view no longer has selected — the
      // operator caught this surviving even after handleBack's own clear
      // call was in place, pointing at a path this file doesn't clear
      // (tab switch while viewing a project, or simply a missed case).
      // This makes the invariant self-healing: whenever selectedProject is
      // falsy, the shared bus is forced to match, regardless of which
      // code path got it there.
      useEffect(() => {
        if (!selectedProject) {
          setSharedState({ selectedProjectId: null, selectedProjectName: null });
        }
      }, [selectedProject]);

      const loadWorkspaces = async () => {
        // Bug fix (2026-10-18): the project-chat sibling (DynamicPipelineUI,
        // ui/src/pipeline-ui.tsx) reads this shared-state bus on ITS OWN
        // mount, independent of this module's remount — so leaving the
        // Workspace tab without clicking "Back" first (e.g. switching
        // straight to another core tab) left selectedProjectId/Name sitting
        // in the bus from the LAST project viewed. Reopening the tab then
        // showed the chat panel for that stale project even though this
        // module's own selectedProject state (below) is a fresh `null` on
        // every mount — a ghost chat the operator correctly flagged
        // ("displaying without me selecting a chat"). This module never
        // restores project selection on mount (only selectedWorkspaceId
        // does, right below) — the chat scope must match that and start
        // cleared too, not stay stuck on whatever was last selected.
        setSharedState({ selectedProjectId: null, selectedProjectName: null });
        setLoading(true);
        setError(null);
        try {
          // executePipeline resolves the raw /pipeline/execute envelope
          // ({success, output, error}), not the pipeline's own output
          // fields directly — real data lives under result.output.* Fixed
          // 2026-09-29: this file previously read result.workspaces (always
          // undefined), so the tab rendered empty regardless of real state.
          const result = await executePipeline(6, { action: "ListWorkspaces" });
          const output = result?.output || {};
          setWorkspaces(output.workspaces || []);

          // Restore selection from shared state
          const state = getSharedState();
          if (state.selectedWorkspaceId) {
            const ws = output.workspaces?.find(
              (w) =>
                w.id === state.selectedWorkspaceId ||
                w.workspace_id === state.selectedWorkspaceId,
            );
            if (ws) {
              setSelectedWorkspace(ws);
              loadProjects(ws.id || ws.workspace_id);
            }
          }
        } catch (e) {
          console.error("Failed to load workspaces:", e);
          setError(e.message || "Failed to load workspaces");
        } finally {
          setLoading(false);
        }
      };

      // Fetches GetProjectFiles for every project in parallel and merges
      // the results into one list, each file tagged with its owning
      // project id/name so an aggregate row can both display provenance
      // and navigate back into the right project (handleSelectProject
      // below, via the project grid's own existing click path — no new
      // navigation mechanism). Real per-project action (the same one
      // loadProjectFiles below already uses for PROJECT VIEW) called once
      // per project — no new backend capability, just client-side fan-out
      // + merge, as scoped in W7.3.
      const loadWorkspaceAggregate = async (projectList) => {
        if (!projectList || projectList.length === 0) {
          setAggFiles([]);
          return;
        }
        setAggLoading(true);
        try {
          const perProject = await Promise.all(
            projectList.map(async (proj) => {
              const pid = proj.id || proj.project_id;
              try {
                const result = await executePipeline(6, {
                  action: "GetProjectFiles",
                  project_id: pid,
                });
                const projName =
                  proj.name || proj.project_name || `Project ${pid}`;
                return (result?.output?.files || []).map((f) => ({
                  ...f,
                  __projectId: pid,
                  __projectName: projName,
                }));
              } catch (e) {
                console.warn("Failed to load files for project", pid, e);
                return [];
              }
            }),
          );
          setAggFiles(perProject.flat());
        } finally {
          setAggLoading(false);
        }
      };

      const loadProjects = async (workspaceId) => {
        try {
          const result = await executePipeline(6, {
            action: "ListProjects",
            workspace_id: workspaceId,
          });
          const projList = result?.output?.projects || [];
          setProjects(projList);
          // Refresh the merged aggregate whenever the project list
          // (re)loads — covers initial workspace selection, post-create,
          // and post-link-file/folder/url refreshes that already call
          // loadProjects for count updates elsewhere in this file.
          loadWorkspaceAggregate(projList);
        } catch (e) {
          console.error("Failed to load projects:", e);
          setError(e.message || "Failed to load projects");
        }
      };

      const loadProjectFiles = async (projectId) => {
        try {
          const result = await executePipeline(6, {
            action: "GetProjectFiles",
            project_id: projectId,
          });
          setFiles(result?.output?.files || []);
        } catch (e) {
          console.warn("Failed to load project files:", e);
        }
      };

      const handleSelectWorkspace = (workspace) => {
        const wsId = workspace.id || workspace.workspace_id;
        setSelectedWorkspace(workspace);
        // selectedProjectId/selectedProjectName cleared: no project is
        // selected yet at the workspace list level. Read by
        // DynamicPipelineUI (ui/src/pipeline-ui.tsx) via the same shared-
        // state bus, which mounts/unmounts the real, separate
        // ProjectChatPanel sibling accordingly.
        setSharedState({ selectedWorkspaceId: wsId, selectedProjectId: null, selectedProjectName: null });
        loadProjects(wsId);
        setSelectedProject(null);
        setFiles([]);
        // Clear the previous workspace's merged aggregate immediately so
        // switching workspaces never flashes stale cross-workspace data —
        // loadProjects (above) repopulates it for the new workspace once
        // loadWorkspaceAggregate resolves.
        setAggFiles([]);
      };

      const handleSelectProject = (project) => {
        const projId = project.id || project.project_id;
        setSelectedProject(project);
        // Reported via the real shared-state bus — DynamicPipelineUI
        // mounts a real, separate <ProjectChatPanel/> sibling for this
        // project (its own chat engine instance, calling the same real
        // backend path as MetaPortion, sharing its rendering layer via
        // chatRendering.tsx). MetaPortion itself is untouched and stays
        // its own, always-visible, unlocked global/workspace chat.
        //
        // 2026-10-18 (cc-fork-unify-fe) — selectedProjectId/selectedProjectName
        // above stay the FLAT Workspace-tab id (unrelated local counter),
        // used correctly elsewhere (chat scope, GetProjectFiles/Unlink).
        // selectedProjectZseiId is a SEPARATE key: the real ZSEI graph
        // container id now carried on every Project object returned by
        // ListProjects/CreateProject/etc (backend fix, same session —
        // field confirmed present, backfilled non-null by ListProjects for
        // any pre-existing record; CreateProject's own immediate response
        // can still be null for one tick if the synchronous container
        // create attempt failed, self-heals on the next ListProjects —
        // read directly off whatever `project` object was passed in here,
        // never the flat id, never fabricated). This is what the graph-
        // native viewers (WorkspaceBrowser/FileReferenceViewer/CodeViewer/
        // FabricView/AmtTreeView/ForkLineageView/JurisdictionHierarchy/
        // GraphView, via ProjectWorkspaceViewer) actually need as their
        // `projectId` prop to resolve real containers instead of failing
        // with "Container not found".
        setSharedState({
          selectedProjectId: projId,
          selectedProjectName: project.name || project.project_name || `Project ${projId}`,
          selectedProjectZseiId: project.zsei_container_id ?? null,
        });
        loadProjectFiles(projId);
      };

      const handleBack = () => {
        if (selectedProject) {
          setSelectedProject(null);
          setFiles([]);
          setSharedState({ selectedProjectId: null, selectedProjectName: null });
        } else if (selectedWorkspace) {
          setSelectedWorkspace(null);
          setProjects([]);
          setAggFiles([]);
        }
      };

      // Shared dismiss for both create modals (backdrop click or Cancel) —
      // without this, cancelling one modal left its typed name/description
      // sitting in state, so opening the OTHER create modal later showed
      // that stale text pre-filled (newName/newDescription are shared
      // between both forms; only the success paths below ever cleared them).
      const closeCreateModals = () => {
        setShowCreateWorkspace(false);
        setShowCreateProject(false);
        setNewName("");
        setNewDescription("");
        // W7.4: reset rename-mode targets too, for the same reason the
        // fields above are cleared here — otherwise a cancelled rename
        // would leave editingWorkspace/editingProject set, and the NEXT
        // "+ Create" open would silently reopen in rename mode against a
        // stale item (the exact stale-field-bleed bug class already fixed
        // once for newName/newDescription above).
        setEditingWorkspace(null);
        setEditingProject(null);
      };

      // W7.4 — open the shared workspace modal pre-filled for renaming.
      const openRenameWorkspace = (ws) => {
        setEditingWorkspace(ws);
        setNewName(ws.name || "");
        setNewDescription(ws.description || "");
        setShowCreateWorkspace(true);
      };

      // W7.4 — open the shared project modal pre-filled for renaming.
      const openRenameProject = (proj) => {
        setEditingProject(proj);
        setNewName(proj.name || "");
        setNewDescription(proj.description || "");
        setShowCreateProject(true);
      };

      const openDeleteWorkspace = (ws) => setDeleteTarget({ kind: "workspace", item: ws });
      const openDeleteProject = (proj) => setDeleteTarget({ kind: "project", item: proj });
      const cancelDelete = () => {
        if (!deleting) setDeleteTarget(null);
      };

      // W7.4: handles BOTH Create and Rename for the shared workspace
      // modal — branches on editingWorkspace (set only by openRenameWorkspace
      // above) rather than duplicating this whole function for Update.
      const handleSubmitWorkspace = async () => {
        if (!newName.trim()) return;
        setLoading(true);
        try {
          if (editingWorkspace) {
            const wsId = editingWorkspace.id || editingWorkspace.workspace_id;
            await executePipeline(6, {
              action: "UpdateWorkspace",
              workspace_id: wsId,
              name: newName.trim(),
              description: newDescription.trim() || null,
            });
          } else {
            await executePipeline(6, {
              action: "CreateWorkspace",
              name: newName.trim(),
              description: newDescription.trim() || null,
            });
          }
          closeCreateModals();
          // loadWorkspaces() re-finds the current selectedWorkspaceId (if
          // any) in the freshly-fetched list via the shared-state restore
          // path it already has — so a rename of the currently-open
          // workspace picks up the new name/description here too, with no
          // separate setSelectedWorkspace call needed.
          await loadWorkspaces();
        } catch (e) {
          console.error("Failed to save workspace:", e);
          setError(e.message || "Failed to save workspace");
        } finally {
          setLoading(false);
        }
      };

      // W7.4: handles BOTH Create and Rename for the shared project modal —
      // branches on editingProject (set only by openRenameProject above)
      // rather than duplicating this whole function for Update.
      const handleSubmitProject = async () => {
        if (!newName.trim() || !selectedWorkspace) return;
        const wsId = selectedWorkspace.id || selectedWorkspace.workspace_id;
        setLoading(true);
        try {
          if (editingProject) {
            const projId = editingProject.id || editingProject.project_id;
            const result = await executePipeline(6, {
              action: "UpdateProject",
              project_id: projId,
              name: newName.trim(),
              description: newDescription.trim() || null,
            });
            const updated = result?.output?.project;
            // If the project being renamed is the one currently open
            // (PROJECT VIEW toolbar's Rename), refresh selectedProject +
            // the shared-state bus immediately so the breadcrumb/title and
            // the project-scoped chat sibling's label update without
            // needing a Back-and-reselect round trip.
            if (
              updated &&
              selectedProject &&
              (selectedProject.id || selectedProject.project_id) === projId
            ) {
              setSelectedProject(updated);
              setSharedState({
                selectedProjectId: projId,
                selectedProjectName: updated.name || updated.project_name || `Project ${projId}`,
              });
            }
            closeCreateModals();
            await loadProjects(wsId);
          } else {
            const result = await executePipeline(6, {
              action: "CreateProject",
              workspace_id: wsId,
              name: newName.trim(),
              description: newDescription.trim() || null,
            });
            closeCreateModals();
            await loadProjects(wsId);
            // Open the new project immediately instead of leaving the
            // operator on the project list — they'd otherwise have to find
            // and click the card they just created to reach Link File/
            // Folder/URL or the chat, which read as "where did it go?"/
            // "where's the folder link UI?" since nothing new appeared
            // where they were looking.
            const created = result?.output?.project;
            if (created) {
              handleSelectProject(created);
            }
          }
        } catch (e) {
          console.error("Failed to save project:", e);
          setError(e.message || "Failed to save project");
        } finally {
          setLoading(false);
        }
      };

      // W7.4 — Delete, always behind the shared confirmation modal
      // (renderDeleteModal below) — never a bare instant delete. Handles
      // both kinds since the backend calls + post-delete navigation only
      // differ in a few lines; a real confirmation step happens in the UI
      // before this is ever called (see openDeleteWorkspace/openDeleteProject
      // + renderDeleteModal's Delete button).
      const confirmDelete = async () => {
        if (!deleteTarget) return;
        setDeleting(true);
        try {
          if (deleteTarget.kind === "workspace") {
            const ws = deleteTarget.item;
            const wsId = ws.id || ws.workspace_id;
            await executePipeline(6, { action: "DeleteWorkspace", workspace_id: wsId });
            const wasOpen = (selectedWorkspace?.id || selectedWorkspace?.workspace_id) === wsId;
            if (wasOpen) {
              // Deleted the currently-open workspace — navigate back to
              // the top-level landing (same clears handleBack already does
              // for leaving a workspace, plus the project-chat-sibling keys
              // handleBack also clears when leaving a project, since a
              // workspace delete can't leave a stale open project behind
              // either).
              setSelectedProject(null);
              setSelectedWorkspace(null);
              setProjects([]);
              setFiles([]);
              setAggFiles([]);
              setSharedState({
                selectedWorkspaceId: null,
                selectedProjectId: null,
                selectedProjectName: null,
              });
            }
            setDeleteTarget(null);
            await loadWorkspaces();
          } else {
            const proj = deleteTarget.item;
            const projId = proj.id || proj.project_id;
            const wsId =
              proj.workspace_id ||
              selectedWorkspace?.id ||
              selectedWorkspace?.workspace_id;
            await executePipeline(6, { action: "DeleteProject", project_id: projId });
            const wasOpen = (selectedProject?.id || selectedProject?.project_id) === projId;
            if (wasOpen) {
              // Deleted the currently-open project — fall back to the
              // workspace's project list (selectedWorkspace stays set, so
              // the PROJECTS VIEW branch takes over on next render) rather
              // than leaving the UI pointing at a project that's gone.
              setSelectedProject(null);
              setFiles([]);
              setSharedState({ selectedProjectId: null, selectedProjectName: null });
            }
            setDeleteTarget(null);
            if (wsId) await loadProjects(wsId);
          }
        } catch (e) {
          console.error("Failed to delete:", e);
          setError(e.message || "Failed to delete");
          setDeleteTarget(null);
        } finally {
          setDeleting(false);
        }
      };

      // Link file to project (calls FileLink pipeline #30)
      const handleLinkFile = async () => {
        if (!selectedProject) return;
        const projId = selectedProject.id || selectedProject.project_id;
        const path = await window.ozone?.system?.selectFile?.();
        if (!path) return;
        try {
          // 2026-10-18 (linkparent fix): pass the project's REAL ZSEI
          // container id (when known) so file_link parents the new
          // FileReference container under it instead of the flat project_id
          // above — the flat id was never a real container, so
          // CreateContainer's parent attach silently no-opped and every
          // linked file became unreachable from the graph. Falls back to
          // today's exact behavior (undefined/omitted) when this project
          // hasn't been backfilled with a real container id yet.
          const result = await executePipeline(30, {
            action: "Link",
            project_id: projId,
            file_path: path,
            analyze: true,
            zsei_parent_id: selectedProject.zsei_container_id ?? null,
          });

          if (result?.output?.file_ref?.modality) {
            const fileRef = result.output.file_ref;
            const modality = fileRef.modality.toLowerCase();
            // file_link's Link action runs analysis synchronously (if
            // requested) and returns before this call resolves — there is
            // no pending/async step after this point, so the message must
            // not imply future work. `analyzed` reports the REAL outcome:
            // true only for the modalities file_link's own analyze_file()
            // actually handles (Code/Script/Web, Documentation/Data) —
            // every other modality (image included) currently falls through
            // to a generic metadata-only path that typically can't even
            // read a binary file as text, so `analyzed` is honestly false
            // for those today. Never claim a pipeline will process the file
            // when no such dispatch exists in this code path.
            if (window.__ozoneThemeArea?.addNotification) {
              const message = fileRef.analyzed
                ? `File linked as ${modality} — analysis complete`
                : `File linked as ${modality} — no analysis pipeline wired for this file type yet`;
              window.__ozoneThemeArea.addNotification(message, fileRef.analyzed ? "info" : "warning");
            }
          }

          await loadProjectFiles(projId);
          const wsId = selectedWorkspace?.id || selectedWorkspace?.workspace_id;
          if (wsId) await loadProjects(wsId);
        } catch (e) {
          console.error("Failed to link file:", e);
          setError(e.message || "Failed to link file");
        }
      };

      // Link a whole folder to project (calls FileLink pipeline #30's
      // LinkFolder action — one DirectoryReference container + each direct
      // child file linked the same way handleLinkFile does, non-recursive).
      const handleLinkFolder = async () => {
        if (!selectedProject) return;
        const projId = selectedProject.id || selectedProject.project_id;
        const folderPath = await window.ozone?.system?.selectFolder?.();
        if (!folderPath) return;
        try {
          // 2026-10-18 (linkparent fix): see handleLinkFile's comment on
          // zsei_parent_id — same real-container-id fix, applied here too.
          const result = await executePipeline(30, {
            action: "LinkFolder",
            project_id: projId,
            folder_path: folderPath,
            analyze: true,
            zsei_parent_id: selectedProject.zsei_container_id ?? null,
          });

          const linkedCount = result?.output?.file_refs?.length || 0;
          const skipped = result?.output?.skipped_entries || [];
          if (window.__ozoneThemeArea?.addNotification) {
            const message = skipped.length
              ? `Folder linked — ${linkedCount} files linked, ${skipped.length} subfolder(s) skipped (non-recursive)`
              : `Folder linked — ${linkedCount} files linked`;
            window.__ozoneThemeArea.addNotification(message, "info");
          }

          await loadProjectFiles(projId);
          const wsId = selectedWorkspace?.id || selectedWorkspace?.workspace_id;
          if (wsId) await loadProjects(wsId);
        } catch (e) {
          console.error("Failed to link folder:", e);
          setError(e.message || "Failed to link folder");
        }
      };

      // Link URL to project (calls URLLink pipeline #31)
      const handleLinkURL = async () => {
        if (!selectedProject || !linkUrl.trim()) return;
        const projId = selectedProject.id || selectedProject.project_id;
        try {
          // 2026-10-18 (linkparent fix): see handleLinkFile's comment on
          // zsei_parent_id — same real-container-id fix, applied to
          // url_link (#31) here.
          await executePipeline(31, {
            action: "Link",
            project_id: projId,
            url: linkUrl.trim(),
            analyze: true,
            zsei_parent_id: selectedProject.zsei_container_id ?? null,
          });
          setLinkUrl("");
          setShowLinkUrl(false);
          await loadProjectFiles(projId);
          // Refresh projects to update counts
          const wsId = selectedWorkspace.id || selectedWorkspace.workspace_id;
          await loadProjects(wsId);
        } catch (e) {
          console.error("Failed to link URL:", e);
          setError(e.message || "Failed to link URL");
        }
      };

      const formatDate = (timestamp) => {
        return new Date(timestamp * 1000).toLocaleDateString();
      };

      const getFileIcon = (modality) => {
        switch (modality) {
          case "Code":
            return "📄";
          case "Text":
            return "📝";
          case "Image":
            return "🖼️";
          case "Audio":
            return "🎵";
          case "Video":
            return "🎬";
          default:
            return "📁";
        }
      };

      // W7.4 — shared modal renderers. Each is called from whichever
      // view branch(es) below can trigger it, instead of duplicating the
      // JSX per branch: renderWorkspaceModal() is called from both the
      // WORKSPACES VIEW landing (sidebar "+"/rename) and the PROJECTS VIEW
      // toolbar (rename-this-workspace); renderProjectModal() from both the
      // PROJECTS VIEW (create/rename via card) and the PROJECT VIEW toolbar
      // (rename-this-project); renderDeleteModal() from all three views —
      // a single shared confirmation modal branching on deleteTarget.kind.

      const renderWorkspaceModal = () => {
        if (!showCreateWorkspace) return null;
        const isEdit = !!editingWorkspace;
        return React.createElement(
          "div",
          { className: "modal-overlay", onClick: closeCreateModals },
          React.createElement(
            "div",
            { className: "modal", onClick: (e) => e.stopPropagation() },
            React.createElement("h3", null, isEdit ? "Rename Workspace" : "Create Workspace"),
            React.createElement(
              "p",
              { className: "hint" },
              isEdit
                ? "Update this workspace's name or description."
                : 'Workspaces help organize related projects. Examples: "Development", "Research", "Personal".',
            ),
            React.createElement("input", {
              type: "text",
              placeholder: "Workspace name",
              value: newName,
              onChange: (e) => setNewName(e.target.value),
              autoFocus: true,
            }),
            React.createElement("textarea", {
              placeholder: "Description (optional)",
              value: newDescription,
              onChange: (e) => setNewDescription(e.target.value),
            }),
            React.createElement(
              "div",
              { className: "modal-actions" },
              React.createElement("button", { onClick: closeCreateModals }, "Cancel"),
              React.createElement(
                "button",
                {
                  className: "btn-primary",
                  onClick: handleSubmitWorkspace,
                  disabled: !newName.trim() || loading,
                },
                loading ? (isEdit ? "Saving..." : "Creating...") : isEdit ? "Save" : "Create",
              ),
            ),
          ),
        );
      };

      const renderProjectModal = () => {
        if (!showCreateProject) return null;
        const isEdit = !!editingProject;
        return React.createElement(
          "div",
          { className: "modal-overlay", onClick: closeCreateModals },
          React.createElement(
            "div",
            { className: "modal", onClick: (e) => e.stopPropagation() },
            React.createElement("h3", null, isEdit ? "Rename Project" : "Create New Project"),
            React.createElement(
              "p",
              { className: "hint" },
              isEdit
                ? "Update this project's name or description."
                : "Projects group linked files and URLs together for reference.",
            ),
            React.createElement("input", {
              type: "text",
              placeholder: "Project name (e.g., API Integration)",
              value: newName,
              onChange: (e) => setNewName(e.target.value),
              autoFocus: true,
            }),
            React.createElement("textarea", {
              placeholder: "Description (optional)",
              value: newDescription,
              onChange: (e) => setNewDescription(e.target.value),
            }),
            React.createElement(
              "div",
              { className: "modal-actions" },
              React.createElement("button", { onClick: closeCreateModals }, "Cancel"),
              React.createElement(
                "button",
                {
                  className: "btn-primary",
                  onClick: handleSubmitProject,
                  disabled: !newName.trim() || loading,
                },
                loading ? (isEdit ? "Saving..." : "Creating...") : isEdit ? "Save" : "Create",
              ),
            ),
          ),
        );
      };

      const renderDeleteModal = () => {
        if (!deleteTarget) return null;
        const isWorkspace = deleteTarget.kind === "workspace";
        const item = deleteTarget.item;
        const name = item.name || (isWorkspace ? "this workspace" : "this project");
        const message = isWorkspace
          ? `Delete workspace "${name}"? This also removes its ${item.project_count ?? 0} project(s) and everything linked inside them. This cannot be undone.`
          : `Delete project "${name}"? This removes its linked files, URLs, and packages. This cannot be undone.`;
        return React.createElement(
          "div",
          { className: "modal-overlay", onClick: cancelDelete },
          React.createElement(
            "div",
            { className: "modal", onClick: (e) => e.stopPropagation() },
            React.createElement("h3", null, isWorkspace ? "Delete Workspace" : "Delete Project"),
            React.createElement("p", { className: "hint" }, message),
            React.createElement(
              "div",
              { className: "modal-actions" },
              React.createElement("button", { onClick: cancelDelete, disabled: deleting }, "Cancel"),
              React.createElement(
                "button",
                { className: "btn-danger", onClick: confirmDelete, disabled: deleting },
                deleting ? "Deleting..." : "Delete",
              ),
            ),
          ),
        );
      };

      // Error display
      if (error) {
        return React.createElement(
          "div",
          { className: "workspace-tab" },
          React.createElement(
            "div",
            { className: "panel-error" },
            React.createElement("span", { className: "error-icon" }, "⚠️"),
            React.createElement("p", null, error),
            React.createElement(
              "button",
              {
                onClick: () => {
                  setError(null);
                  loadWorkspaces();
                },
              },
              "Retry",
            ),
          ),
        );
      }

      // Loading state
      if (loading && workspaces.length === 0) {
        return React.createElement(
          "div",
          { className: "workspace-loading" },
          React.createElement("div", { className: "loading-spinner" }),
          React.createElement("p", null, "Loading workspaces..."),
        );
      }

      // PROJECT VIEW - Show project details, linked files, and chat
      if (selectedProject) {
        const projId = selectedProject.id || selectedProject.project_id;
        const wsId = selectedWorkspace?.id || selectedWorkspace?.workspace_id;
        const fileCount =
          selectedProject.file_count ?? selectedProject.files_count ?? 0;
        const urlCount = selectedProject.url_count ?? 0;
        const packageCount = selectedProject.package_count ?? 0;

        return React.createElement(
          "div",
          { className: "workspace-tab" },
          React.createElement(
            "div",
            { className: "panel-toolbar" },
            React.createElement(
              "button",
              { className: "back-btn", onClick: handleBack },
              "← Back to Projects",
            ),
            // W7.5 — breadcrumb: "Workspace / Project" instead of just the
            // project name, so the toolbar shows where this project lives.
            React.createElement(
              "h2",
              null,
              selectedWorkspace ? `${selectedWorkspace.name} / ${selectedProject.name}` : selectedProject.name,
            ),
            React.createElement(
              "div",
              { className: "toolbar-actions" },
              React.createElement(
                "button",
                { className: "toolbar-btn", onClick: handleLinkFile },
                "📎 Link File",
              ),
              React.createElement(
                "button",
                { className: "toolbar-btn", onClick: handleLinkFolder },
                "📁 Link Folder",
              ),
              React.createElement(
                "button",
                {
                  className: "toolbar-btn",
                  onClick: () => setShowLinkUrl(true),
                },
                "🔗 Link URL",
              ),
              // W7.4 — Rename/Delete for the currently-open project.
              React.createElement(
                "button",
                {
                  className: "toolbar-btn",
                  onClick: () => openRenameProject(selectedProject),
                },
                "✏️ Rename",
              ),
              React.createElement(
                "button",
                {
                  className: "toolbar-btn danger",
                  onClick: () => openDeleteProject(selectedProject),
                },
                "🗑 Delete",
              ),
            ),
          ),

          selectedProject.description &&
            React.createElement(
              "div",
              { className: "project-description" },
              React.createElement("p", null, selectedProject.description),
            ),

          React.createElement(
            "div",
            { className: "project-stats" },
            React.createElement(
              "span",
              { className: "stat" },
              "📎 " + fileCount + " files",
            ),
            React.createElement(
              "span",
              { className: "stat" },
              "🔗 " + urlCount + " URLs",
            ),
            React.createElement(
              "span",
              { className: "stat" },
              "📦 " + packageCount + " packages",
            ),
          ),

          // Project-scoped chat is rendered as a real React sibling by the
          // host (ui/src/pipeline-ui.tsx's DynamicPipelineUI), driven by
          // the selectedProjectId/selectedProjectName reported via
          // setSharedState in handleSelectProject above — this pipeline-UI
          // module itself can't mount React components directly (a
          // separately-eval'd bundle), so there is nothing to render here.
          // MetaPortion (the global sidebar) is untouched by any of this.

          // Link URL Dialog
          showLinkUrl &&
            React.createElement(
              "div",
              {
                className: "modal-overlay",
                onClick: () => setShowLinkUrl(false),
              },
              React.createElement(
                "div",
                {
                  className: "modal",
                  onClick: (e) => e.stopPropagation(),
                },
                React.createElement("h3", null, "Link URL"),
                React.createElement(
                  "p",
                  { className: "hint" },
                  "Add a URL to this project's context.",
                ),
                React.createElement("input", {
                  type: "url",
                  placeholder: "https://...",
                  value: linkUrl,
                  onChange: (e) => setLinkUrl(e.target.value),
                  onKeyDown: (e) => e.key === "Enter" && handleLinkURL(),
                  autoFocus: true,
                }),
                React.createElement(
                  "div",
                  { className: "modal-actions" },
                  React.createElement(
                    "button",
                    {
                      onClick: () => setShowLinkUrl(false),
                    },
                    "Cancel",
                  ),
                  React.createElement(
                    "button",
                    {
                      className: "btn-primary",
                      onClick: handleLinkURL,
                      disabled: !linkUrl.trim(),
                    },
                    "Link",
                  ),
                ),
              ),
            ),

          // W7.4 — rename-this-project modal (opened via the toolbar
          // button above) + the shared delete-confirmation modal.
          renderProjectModal(),
          renderDeleteModal(),
        );
      }

      // PROJECTS VIEW - Show projects in selected workspace
      if (selectedWorkspace) {
        const wsId = selectedWorkspace.id || selectedWorkspace.workspace_id;
        // Real summed counts from each project's own already-loaded
        // ListProjects data (file_count/files_count naming varies by
        // record age — same fallback the per-project card below already
        // uses). This covers files+URLs+packages honestly; the merged
        // aggFiles list below only ever covers files (see the aggFiles
        // state comment above for why).
        const totalFiles = projects.reduce(
          (sum, p) => sum + (p.file_count ?? p.files_count ?? 0),
          0,
        );
        const totalUrls = projects.reduce(
          (sum, p) => sum + (p.url_count ?? 0),
          0,
        );
        const totalPackages = projects.reduce(
          (sum, p) => sum + (p.package_count ?? 0),
          0,
        );

        return React.createElement(
          "div",
          { className: "workspace-tab" },
          React.createElement(
            "div",
            { className: "panel-toolbar" },
            React.createElement(
              "button",
              { className: "back-btn", onClick: handleBack },
              "← Back to Workspaces",
            ),
            React.createElement("h2", null, selectedWorkspace.name),
            React.createElement(
              "div",
              { className: "toolbar-actions" },
              // W7.4 — Rename/Delete for the currently-open workspace.
              React.createElement(
                "button",
                {
                  className: "toolbar-btn",
                  onClick: () => openRenameWorkspace(selectedWorkspace),
                },
                "✏️ Rename",
              ),
              React.createElement(
                "button",
                {
                  className: "toolbar-btn danger",
                  onClick: () => openDeleteWorkspace(selectedWorkspace),
                },
                "🗑 Delete",
              ),
              React.createElement(
                "button",
                {
                  className: "toolbar-btn primary",
                  onClick: () => setShowCreateProject(true),
                },
                "+ New Project",
              ),
            ),
          ),

          selectedWorkspace.description &&
            React.createElement(
              "div",
              { className: "workspace-description" },
              React.createElement("p", null, selectedWorkspace.description),
            ),

          // W7.3 aggregate panel — real summed counts + merged, clickable
          // file list across every project in this workspace. Rendered
          // ABOVE the project grid (kept below, unchanged — it's still the
          // real way to open a specific project). Hidden while loading or
          // when there are no projects yet (nothing to aggregate).
          !loading &&
            projects.length > 0 &&
            React.createElement(
              "div",
              {
                className: "workspace-aggregate",
                style: { borderBottom: "1px solid var(--color-border)" },
              },
              React.createElement(
                "div",
                { className: "project-stats" },
                React.createElement(
                  "span",
                  { className: "stat" },
                  "📂 " +
                    projects.length +
                    (projects.length === 1 ? " project" : " projects"),
                ),
                React.createElement(
                  "span",
                  { className: "stat" },
                  "📎 " + totalFiles + " files",
                ),
                React.createElement(
                  "span",
                  { className: "stat" },
                  "🔗 " + totalUrls + " URLs",
                ),
                React.createElement(
                  "span",
                  { className: "stat" },
                  "📦 " + totalPackages + " packages",
                ),
              ),
              aggLoading
                ? React.createElement(
                    "p",
                    { className: "hint", style: { padding: "0 var(--space-lg) var(--space-md)" } },
                    "Loading files across all projects...",
                  )
                : aggFiles.length === 0
                  ? React.createElement(
                      "p",
                      { className: "hint", style: { padding: "0 var(--space-lg) var(--space-md)" } },
                      "No files linked yet in this workspace's projects.",
                    )
                  : React.createElement(
                      "div",
                      {
                        className: "files-list",
                        style: { maxHeight: "240px", overflowY: "auto", flex: "none" },
                      },
                      React.createElement(
                        "h3",
                        null,
                        "All Files Across Projects (" + aggFiles.length + ")",
                      ),
                      aggFiles.map((file) => {
                        // Simplest honest navigation available at render
                        // time: land the operator in the file's owning
                        // project (this file's own handleSelectProject,
                        // the same path the grid card below uses) rather
                        // than deep-linking into the file itself — keeps
                        // everything in this one tab, no new cross-bundle
                        // bridge call needed.
                        const openOwningProject = () => {
                          const proj = projects.find(
                            (p) => (p.id || p.project_id) === file.__projectId,
                          );
                          if (proj) handleSelectProject(proj);
                        };
                        return React.createElement(
                          "div",
                          {
                            key:
                              "agg-" +
                              file.__projectId +
                              "-" +
                              (file.file_ref_id || file.id),
                            className: "file-item",
                            onClick: openOwningProject,
                            style: { cursor: "pointer" },
                            title: "Open " + file.__projectName,
                          },
                          React.createElement(
                            "span",
                            { className: "file-icon" },
                            getFileIcon(file.modality),
                          ),
                          React.createElement(
                            "div",
                            { className: "file-info" },
                            React.createElement(
                              "span",
                              { className: "file-name" },
                              file.name,
                            ),
                            React.createElement(
                              "span",
                              { className: "file-path" },
                              file.path,
                            ),
                          ),
                          React.createElement(
                            "span",
                            { className: "file-size" },
                            "📂 " + file.__projectName,
                          ),
                        );
                      }),
                    ),
            ),

          loading
            ? React.createElement(
                "div",
                { className: "panel-loading" },
                React.createElement("div", { className: "loading-spinner" }),
                React.createElement("p", null, "Loading projects..."),
              )
            : projects.length === 0
              ? React.createElement(
                  "div",
                  { className: "panel-empty-centered" },
                  React.createElement(
                    "span",
                    { className: "empty-icon" },
                    "📂",
                  ),
                  React.createElement(
                    "p",
                    null,
                    "No projects in this workspace",
                  ),
                  React.createElement(
                    "p",
                    { className: "empty-hint" },
                    "Create a project to start working with scoped context",
                  ),
                  React.createElement(
                    "button",
                    {
                      className: "btn-primary",
                      onClick: () => setShowCreateProject(true),
                    },
                    "Create Project",
                  ),
                )
              : React.createElement(
                  "div",
                  { className: "items-grid" },
                  projects.map((proj) => {
                    const fileCount = proj.file_count ?? proj.files_count ?? 0;
                    const urlCount = proj.url_count ?? 0;
                    const packageCount = proj.package_count ?? 0;

                    return React.createElement(
                      "div",
                      {
                        key: proj.id || proj.project_id,
                        className: "item-card",
                        onClick: () => handleSelectProject(proj),
                      },
                      React.createElement(
                        "div",
                        { className: "card-icon" },
                        "📂",
                      ),
                      React.createElement(
                        "div",
                        { className: "card-content" },
                        React.createElement("h3", null, proj.name),
                        proj.description &&
                          React.createElement(
                            "p",
                            { className: "card-desc" },
                            proj.description,
                          ),
                        React.createElement(
                          "div",
                          { className: "card-meta" },
                          React.createElement("span", null, "📎 " + fileCount),
                          React.createElement("span", null, "🔗 " + urlCount),
                          React.createElement(
                            "span",
                            null,
                            "📦 " + packageCount,
                          ),
                        ),
                        // W7.5 — formatDate was real timestamp data, dead
                        // code until now (defined, never called).
                        proj.created_at &&
                          React.createElement(
                            "span",
                            { className: "created-date" },
                            "Created " + formatDate(proj.created_at),
                          ),
                      ),
                      // W7.4 — per-card Rename/Delete. stopPropagation on
                      // each button (same pattern the pre-existing
                      // file-unlink-btn uses above) so clicking them
                      // doesn't also trigger the card's own
                      // onClick={() => handleSelectProject(proj)}.
                      React.createElement(
                        "div",
                        { className: "item-actions" },
                        React.createElement(
                          "button",
                          {
                            className: "icon-action-btn",
                            title: "Rename project",
                            onClick: (e) => {
                              e.stopPropagation();
                              openRenameProject(proj);
                            },
                          },
                          "✏️",
                        ),
                        React.createElement(
                          "button",
                          {
                            className: "icon-action-btn danger",
                            title: "Delete project",
                            onClick: (e) => {
                              e.stopPropagation();
                              openDeleteProject(proj);
                            },
                          },
                          "🗑",
                        ),
                      ),
                      React.createElement(
                        "span",
                        { className: "card-arrow" },
                        "→",
                      ),
                    );
                  }),
                ),

          // W7.4 — create/rename-project modal (shared renderer; branches
          // on editingProject), the rename-workspace modal (reachable from
          // this view's toolbar Rename button above), and the shared
          // delete-confirmation modal (reachable from the toolbar's Delete
          // button, or any card's own delete icon).
          renderProjectModal(),
          renderWorkspaceModal(),
          renderDeleteModal(),
        );
      }

      // No workspaces - show create prompt centered
      if (workspaces.length === 0) {
        return React.createElement(
          "div",
          { className: "workspace-empty" },
          React.createElement(
            "div",
            { className: "empty-state-centered" },
            React.createElement("span", { className: "empty-icon" }, "📁"),
            React.createElement("h3", null, "No workspaces yet"),
            React.createElement(
              "p",
              null,
              "Create one to organize your projects",
            ),
            React.createElement(
              "p",
              { className: "hint" },
              'A workspace groups related projects together - like "Coding", "Research", or "Personal".',
            ),
            React.createElement(
              "button",
              {
                className: "btn-primary",
                onClick: () => setShowCreateWorkspace(true),
              },
              "+ Create Workspace",
            ),
          ),
          // Latent pre-existing bug, found while wiring W7.4: this branch
          // never rendered the create-workspace modal at all, so clicking
          // "+ Create Workspace" here (the very first workspace, before any
          // exist) set showCreateWorkspace but nothing appeared — the
          // branch condition (workspaces.length === 0) doesn't change until
          // a workspace is actually created, so it kept re-rendering this
          // same modal-less branch. Fixed by rendering the same shared
          // modal used everywhere else.
          renderWorkspaceModal(),
        );
      }

      // WORKSPACES VIEW - List all workspaces (with sidebar)
      // Lighter-touch real stat for the top-level landing (W7.3, item 1):
      // there's nothing to meaningfully aggregate across MULTIPLE
      // workspaces (unlike a single workspace's own projects below), so
      // this deliberately stays a cheap, honest count — Workspace's own
      // `project_count` field (main.rs, maintained on create/delete) is
      // already loaded in `workspaces`, summed here with zero extra
      // fetches. No file/URL/package/graph aggregation at this level —
      // that would mean fetching every project of every workspace just to
      // populate a landing screen, which isn't what was asked for here.
      const totalProjectsAcrossWorkspaces = workspaces.reduce(
        (sum, w) => sum + (w.project_count ?? 0),
        0,
      );

      return React.createElement(
        "div",
        { className: "workspace-panel" },
        // Sidebar: Workspace list
        React.createElement(
          "aside",
          { className: "workspace-sidebar" },
          React.createElement(
            "div",
            { className: "sidebar-header" },
            React.createElement("h3", null, "Workspaces"),
            React.createElement(
              "button",
              {
                className: "btn-icon",
                onClick: () => setShowCreateWorkspace(true),
                title: "Create Workspace",
              },
              "+",
            ),
          ),
          React.createElement(
            "div",
            { className: "workspace-list" },
            workspaces.map((ws) => {
              const wsId = ws.id || ws.workspace_id;
              const selectedId =
                selectedWorkspace?.id || selectedWorkspace?.workspace_id;
              return React.createElement(
                "div",
                {
                  key: wsId,
                  className:
                    "workspace-item" + (selectedId === wsId ? " selected" : ""),
                  onClick: () => handleSelectWorkspace(ws),
                },
                React.createElement("span", { className: "ws-icon" }, "📁"),
                React.createElement(
                  "div",
                  { className: "ws-name-col" },
                  React.createElement("span", { className: "ws-name" }, ws.name),
                  // W7.5 — formatDate was real timestamp data, dead code
                  // until now (defined, never called).
                  ws.created_at &&
                    React.createElement(
                      "span",
                      { className: "created-date" },
                      "Created " + formatDate(ws.created_at),
                    ),
                ),
                // W7.4 — per-item Rename/Delete. stopPropagation on each
                // button so clicking them doesn't also trigger this item's
                // own onClick={() => handleSelectWorkspace(ws)}.
                React.createElement(
                  "div",
                  { className: "item-actions" },
                  React.createElement(
                    "button",
                    {
                      className: "icon-action-btn",
                      title: "Rename workspace",
                      onClick: (e) => {
                        e.stopPropagation();
                        openRenameWorkspace(ws);
                      },
                    },
                    "✏️",
                  ),
                  React.createElement(
                    "button",
                    {
                      className: "icon-action-btn danger",
                      title: "Delete workspace",
                      onClick: (e) => {
                        e.stopPropagation();
                        openDeleteWorkspace(ws);
                      },
                    },
                    "🗑",
                  ),
                ),
              );
            }),
          ),
        ),

        // Main content - info banner when no workspace selected
        React.createElement(
          "main",
          { className: "workspace-content" },
          React.createElement(
            "div",
            { className: "info-banner" },
            React.createElement("span", { className: "info-icon" }, "💡"),
            React.createElement(
              "p",
              null,
              "A ",
              React.createElement("strong", null, "Workspace"),
              ' organizes related projects (e.g., "Coding", "Research"). ',
              "Each workspace can contain multiple ",
              React.createElement("strong", null, "Projects"),
              ", each grouping its own linked files/URLs. The sidebar chat can be switched to this workspace (its own conversation, separate from Global).",
            ),
          ),
          React.createElement(
            "div",
            { className: "project-stats" },
            React.createElement(
              "span",
              { className: "stat" },
              "📁 " +
                workspaces.length +
                (workspaces.length === 1 ? " workspace" : " workspaces"),
            ),
            React.createElement(
              "span",
              { className: "stat" },
              "📂 " + totalProjectsAcrossWorkspaces + " projects total",
            ),
          ),
          React.createElement(
            "div",
            { className: "compact-prompt-banner" },
            React.createElement("span", { className: "empty-icon" }, "👈"),
            React.createElement(
              "p",
              null,
              "Select a workspace to view its projects",
            ),
          ),
        ),

        // W7.4 — create/rename-workspace modal (shared renderer; branches
        // on editingWorkspace) + the shared delete-confirmation modal
        // (reachable from each sidebar item's own delete icon above).
        renderWorkspaceModal(),
        renderDeleteModal(),
      );
    }

    const root = ReactDOM.createRoot(container);
    root.render(React.createElement(WorkspaceUI));

    return () => root.unmount();
  },

  onActivate: function () {
    console.log("WorkspaceTab activated");
  },

  onDeactivate: function () {
    console.log("WorkspaceTab deactivated");
  },
};
