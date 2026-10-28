/**
 * GraphNativePanel — the global Files / Fabric / Hierarchy / Engines tabs as
 * mounts of the SAME graph-native workspace the project Workspace tab uses
 * (the canonical-surface law: one component, two mounts). The tab's identity
 * is its starting lens; the project comes from the shared picker; every
 * sub-view the old PanelShell tabs offered is a sub-view of the lens.
 */
import React from "react";
import ProjectPicker from "./ProjectPicker";
import ProjectWorkspaceViewer from "./ProjectWorkspaceViewer";
import { useSelectedProject } from "../projectSelection";

export const GraphNativePanel: React.FC<{
  title: string;
  subtitle?: string;
  section: "files" | "fabric" | "hierarchy" | "graph" | "engines";
  requestedFilesSub?: string | null;
  requestedEngineSub?: string | null;
}> = ({ title, subtitle, section, requestedFilesSub, requestedEngineSub }) => {
  const [projectId] = useSelectedProject();
  return (
    <div className="opanel">
      <div className="opanel-head">
        <span className="opanel-title">{title}</span>
      </div>
      {subtitle && <p className="opanel-sub">{subtitle}</p>}
      <div style={{ display: "flex", gap: 12, alignItems: "center", flexWrap: "wrap", marginBottom: 10 }}>
        <ProjectPicker />
      </div>
      <div className="opanel-scroll gnp-body">
        <ProjectWorkspaceViewer
          projectId={null}
          zseiProjectId={projectId}
          initialSection={section}
          requestedFilesSub={requestedFilesSub}
          requestedEngineSub={requestedEngineSub}
        />
      </div>
    </div>
  );
};

export default GraphNativePanel;
