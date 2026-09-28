/**
 * F1 — Workspace browser: real Workspace → Project → file-reference
 * hierarchy over `data/workspaceData.ts` (B12/B13). Selecting a project here
 * updates the shared `projectSelection` store (the same one `PanelShell`'s
 * `ProjectPicker` reads), so this tree and the picker stay in sync either
 * direction. Selecting a file/url/package updates `fileSelection` for the
 * Code/Editor/Math/History views to pick up.
 *
 * Per A5: this shows only the real, graph-native `FileReference`/
 * `URLReference`/`PackageReference` containers — `workspace_tab`'s own file
 * list is a separate, disconnected flat-JSON store and is NOT merged in here
 * (labelled below rather than silently combined).
 */
import React, { useEffect, useState } from "react";
import {
  FileRef,
  loadProjectFileRefs,
  loadProjects,
  loadWorkspaces,
  ProjectSummary,
  WorkspaceSummary,
} from "../../data/workspaceData";
import { setSelectedProject } from "../../projectSelection";
import { setSelectedFile, useSelectedFile } from "../../fileSelection";

export type WorkspaceBrowserProps = { projectId: number | null };

const C_TEXT = "#dfe7f2";
const C_MUTED = "#8b98ab";
const C_BORDER = "#1e2836";

function currentUserId(): number {
  return (window as any).ozone?.auth?.getCurrentUserId?.() ?? 1;
}

const KIND_LABEL: Record<FileRef["kind"], string> = { file: "Files", url: "URLs", package: "Packages" };
const KIND_ICON: Record<FileRef["kind"], string> = { file: "📄", url: "🔗", package: "📦" };

export const WorkspaceBrowser: React.FC<WorkspaceBrowserProps> = ({ projectId }) => {
  const [workspaces, setWorkspaces] = useState<WorkspaceSummary[] | null>(null);
  const [projectsByWs, setProjectsByWs] = useState<Record<number, ProjectSummary[]>>({});
  const [treeError, setTreeError] = useState<string | null>(null);

  const [fileRefs, setFileRefs] = useState<FileRef[] | null>(null);
  const [fileRefsError, setFileRefsError] = useState<string | null>(null);
  const [selectedFile] = useSelectedFile();

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const ws = await loadWorkspaces(currentUserId());
        if (cancelled) return;
        setWorkspaces(ws);
        const entries = await Promise.all(
          ws.map(async (w) => [w.containerId, await loadProjects(w.containerId)] as const),
        );
        if (cancelled) return;
        setProjectsByWs(Object.fromEntries(entries));
      } catch (err) {
        if (!cancelled) setTreeError(err instanceof Error ? err.message : String(err));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (projectId === null) {
      setFileRefs(null);
      return;
    }
    let cancelled = false;
    setFileRefs(null);
    setFileRefsError(null);
    loadProjectFileRefs(projectId)
      .then((refs) => !cancelled && setFileRefs(refs))
      .catch((err) => !cancelled && setFileRefsError(err instanceof Error ? err.message : String(err)));
    return () => {
      cancelled = true;
    };
  }, [projectId]);

  function openRef(ref: FileRef) {
    if (projectId === null) return;
    setSelectedFile({
      projectId,
      containerId: ref.containerId,
      kind: ref.kind,
      name: ref.name,
      path: ref.path,
      modality: ref.modality,
    });
  }

  const grouped: Record<FileRef["kind"], FileRef[]> = { file: [], url: [], package: [] };
  for (const r of fileRefs ?? []) grouped[r.kind].push(r);

  return (
    <div style={{ display: "flex", gap: 16, fontSize: 12.5, color: C_TEXT }}>
      <div style={{ width: 260, flexShrink: 0, borderRight: `1px solid ${C_BORDER}`, paddingRight: 12 }}>
        <div style={{ fontWeight: 700, marginBottom: 6 }}>Workspaces</div>
        {treeError && <div style={{ color: "#ff8a8a" }}>Error: {treeError}</div>}
        {!treeError && workspaces === null && <div style={{ color: C_MUTED }}>Loading…</div>}
        {!treeError && workspaces !== null && workspaces.length === 0 && (
          <div style={{ color: C_MUTED }}>No workspaces found yet.</div>
        )}
        {workspaces?.map((w) => (
          <div key={w.containerId} style={{ marginBottom: 10 }}>
            <div style={{ color: C_MUTED, fontSize: 11.5 }}>📁 {w.name}</div>
            <div style={{ marginLeft: 14, marginTop: 2 }}>
              {(projectsByWs[w.containerId] ?? []).length === 0 ? (
                <div style={{ color: C_MUTED, fontSize: 11 }}>no projects</div>
              ) : (
                projectsByWs[w.containerId].map((p) => {
                  const active = p.containerId === projectId;
                  return (
                    <div
                      key={p.containerId}
                      onClick={() => setSelectedProject(p.containerId)}
                      style={{
                        cursor: "pointer",
                        padding: "2px 6px",
                        borderRadius: 4,
                        background: active ? C_BORDER : "transparent",
                        color: active ? C_TEXT : C_MUTED,
                        fontWeight: active ? 600 : 400,
                      }}
                    >
                      {p.name}
                    </div>
                  );
                })
              )}
            </div>
          </div>
        ))}
      </div>

      <div style={{ flex: 1, minWidth: 0 }}>
        {projectId === null ? (
          <div style={{ color: C_MUTED }}>Select a project to see its linked files, URLs and packages.</div>
        ) : (
          <>
            <div style={{ fontWeight: 700, marginBottom: 4 }}>Linked references</div>
            <div style={{ color: C_MUTED, fontSize: 11, marginBottom: 10 }}>
              Real graph-native `FileReference`/`URLReference`/`PackageReference` containers only. Files added
              through the legacy Workspace tab are stored separately and are not shown here.
            </div>
            {fileRefsError && <div style={{ color: "#ff8a8a" }}>Error: {fileRefsError}</div>}
            {!fileRefsError && fileRefs === null && <div style={{ color: C_MUTED }}>Loading…</div>}
            {!fileRefsError && fileRefs !== null && fileRefs.length === 0 && (
              <div style={{ color: C_MUTED }}>No files, URLs or packages linked to this project yet.</div>
            )}
            {(["file", "url", "package"] as const).map((kind) =>
              grouped[kind].length === 0 ? null : (
                <div key={kind} style={{ marginBottom: 12 }}>
                  <div style={{ color: C_MUTED, fontSize: 11, textTransform: "uppercase", marginBottom: 4 }}>
                    {KIND_ICON[kind]} {KIND_LABEL[kind]} ({grouped[kind].length})
                  </div>
                  {grouped[kind].map((ref) => {
                    const active =
                      selectedFile?.containerId === ref.containerId && selectedFile?.kind === ref.kind;
                    return (
                      <div
                        key={ref.containerId}
                        onClick={() => openRef(ref)}
                        style={{
                          cursor: "pointer",
                          padding: "4px 8px",
                          borderRadius: 6,
                          border: `1px solid ${active ? C_TEXT : C_BORDER}`,
                          marginBottom: 4,
                          display: "flex",
                          alignItems: "center",
                          gap: 8,
                        }}
                      >
                        <span style={{ overflowWrap: "anywhere" }}>{ref.name}</span>
                        {ref.modality && (
                          <span
                            style={{
                              fontSize: 10.5,
                              color: C_MUTED,
                              border: `1px solid ${C_BORDER}`,
                              borderRadius: 999,
                              padding: "0 6px",
                            }}
                          >
                            {ref.modality}
                          </span>
                        )}
                      </div>
                    );
                  })}
                </div>
              ),
            )}
          </>
        )}
      </div>
    </div>
  );
};

export default WorkspaceBrowser;
