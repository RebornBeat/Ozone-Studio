import React from "react";
import PanelShell from "../components/PanelShell";
import LivingNetworkView from "../views/network/LivingNetworkView";

export const LivingNetworkPanel: React.FC = () => (
  <PanelShell
    title="Living Network"
    subtitle="AMT, graphs, steps and blueprints as one live view."
    showProjectPicker={true}
    tabs={[
    { id: "network", label: "Network", render: (projectId) => (<><LivingNetworkView projectId={projectId} /></>) },
    ]}
  />
);

export default LivingNetworkPanel;
