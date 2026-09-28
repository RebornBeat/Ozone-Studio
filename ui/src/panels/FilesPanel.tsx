import React from "react";
import PanelShell from "../components/PanelShell";
import CodeViewer from "../views/files/CodeViewer";
import FileEditor from "../views/files/FileEditor";
import FileReferenceViewer from "../views/files/FileReferenceViewer";
import MathViewer from "../views/files/MathViewer";
import VersionHistory from "../views/files/VersionHistory";
import WorkspaceBrowser from "../views/files/WorkspaceBrowser";

export const FilesPanel: React.FC = () => (
  <PanelShell
    title="Files & Code"
    subtitle="Graph-native workspace files, code/math viewers, and the editor."
    showProjectPicker={true}
    tabs={[
    { id: "browser", label: "Browser", render: (projectId) => (<><WorkspaceBrowser projectId={projectId} /></>) },
    { id: "refs", label: "References", render: (projectId) => (<><FileReferenceViewer projectId={projectId} /></>) },
    { id: "code", label: "Code", render: (projectId) => (<><CodeViewer projectId={projectId} /></>) },
    { id: "math", label: "Math", render: (projectId) => (<><MathViewer projectId={projectId} /></>) },
    { id: "editor", label: "Editor", render: (projectId) => (<><FileEditor projectId={projectId} /></>) },
    { id: "history", label: "History", render: (projectId) => (<><VersionHistory projectId={projectId} /></>) },
    ]}
  />
);

export default FilesPanel;
