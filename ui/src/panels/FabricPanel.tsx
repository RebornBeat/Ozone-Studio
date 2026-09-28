import React from "react";
import PanelShell from "../components/PanelShell";
import FabricView from "../views/fabric/FabricView";

export const FabricPanel: React.FC = () => (
  <PanelShell
    title="Context Viewer — Fabric View"
    subtitle="Cross-modality spatial map of the real text/code/math graphs."
    showProjectPicker={true}
    tabs={[
    { id: "fabric", label: "Fabric", render: (projectId) => (<><FabricView projectId={projectId} /></>) },
    ]}
  />
);

export default FabricPanel;
