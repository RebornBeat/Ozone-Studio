import React from "react";
import PanelShell from "../components/PanelShell";
import AmtTreeView from "../views/hierarchy/AmtTreeView";
import ForkLineageView from "../views/hierarchy/ForkLineageView";
import JurisdictionHierarchy from "../views/hierarchy/JurisdictionHierarchy";

export const HierarchyPanel: React.FC = () => (
  <PanelShell
    title="Context Viewer — Hierarchy View"
    subtitle="Real AMT trees, fork lineage, and the jurisdiction hierarchy."
    showProjectPicker={true}
    tabs={[
    { id: "amt", label: "AMT tree", render: (projectId) => (<><AmtTreeView projectId={projectId} /></>) },
    { id: "lineage", label: "Fork lineage", render: (projectId) => (<><ForkLineageView projectId={projectId} /></>) },
    { id: "jurisdiction", label: "Jurisdiction", render: (projectId) => (<><JurisdictionHierarchy projectId={projectId} /></>) },
    ]}
  />
);

export default HierarchyPanel;
