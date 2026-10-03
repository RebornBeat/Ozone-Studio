import React from "react";
import PanelShell from "../components/PanelShell";
import ConfettiViewer from "../views/capture/ConfettiViewer";
import CorrelationView from "../views/capture/CorrelationView";
import DecisionReviewPanel from "../views/capture/DecisionReviewPanel";
import ModelSwitchTimeline from "../views/capture/ModelSwitchTimeline";
import RawThoughtPanel from "../views/capture/RawThoughtPanel";
import ReliabilityDashboard from "../views/capture/ReliabilityDashboard";
import ToolCallPanel from "../views/capture/ToolCallPanel";

export const CapturePanel: React.FC = () => (
  <PanelShell
    title="Raw Thoughts & Capture Stores"
    subtitle="Every real model call and decision review, exactly as captured — failures included."
    showProjectPicker={true}
    tabs={[
    { id: "calls", label: "Model calls", render: (projectId) => (<><RawThoughtPanel projectId={projectId} /></>) },
    { id: "decisions", label: "Decision reviews", render: (projectId) => (<><DecisionReviewPanel projectId={projectId} /></>) },
    { id: "tool-calls", label: "Tool calls", render: (projectId) => (<><ToolCallPanel projectId={projectId} /></>) },
    { id: "correlation", label: "Graph correlation", render: (projectId) => (<><CorrelationView projectId={projectId} /></>) },
    { id: "timeline", label: "Model switches", render: (projectId) => (<><ModelSwitchTimeline projectId={projectId} /></>) },
    { id: "confetti", label: "Confetti bursts", render: (projectId) => (<><ConfettiViewer projectId={projectId} /></>) },
    { id: "reliability", label: "Reliability", render: (projectId) => (<><ReliabilityDashboard projectId={projectId} /></>) },
    ]}
  />
);

export default CapturePanel;
