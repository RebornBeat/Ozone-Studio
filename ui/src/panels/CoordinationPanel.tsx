import React from "react";
import PanelShell from "../components/PanelShell";
import ConsciousnessReview from "../views/coordination/ConsciousnessReview";
import CoordinationFeed from "../views/coordination/CoordinationFeed";
import ForkDispatchPanel from "../views/coordination/ForkDispatchPanel";
import { AgentActivityPanel } from "../views/coordination/AgentActivity";

export const CoordinationPanel: React.FC = () => (
  <PanelShell
    title="Coordination"
    subtitle="Agents, file claims, coordination events, fork dispatch and consciousness review."
    showProjectPicker={true}
    tabs={[
    { id: "feed", label: "Event feed", render: (projectId) => (<><CoordinationFeed projectId={projectId} /></>) },
    { id: "agents", label: "Agents & claims", render: (projectId) => (<><AgentActivityPanel projectId={projectId} /></>) },
    { id: "forks", label: "Fork dispatch", render: (projectId) => (<><ForkDispatchPanel projectId={projectId} /></>) },
    { id: "review", label: "Consciousness review", render: (projectId) => (<><ConsciousnessReview projectId={projectId} /></>) },
    ]}
  />
);

export default CoordinationPanel;
