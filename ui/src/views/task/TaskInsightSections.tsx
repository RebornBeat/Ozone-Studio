import React from "react";
import StepDetailSection from "./StepDetailSection";
import StepExecutionList from "./StepExecutionList";
import VersionNotesSection from "./VersionNotesSection";
import AmtActivitySection from "./AmtActivitySection";
import JurisdictionGateSection from "./JurisdictionGateSection";
import SimulationSection from "./SimulationSection";
import ConsciousnessGateSection from "./ConsciousnessGateSection";

/** Mounted by TaskDetailPanel after its own step list. Scaffold-owned; each section is one fork's file. */
export const TaskInsightSections: React.FC<{ task: any }> = ({ task }) => (
  <div style={{ marginTop: 16, display: "flex", flexDirection: "column", gap: 14 }}>
    <StepDetailSection task={task} />
    <StepExecutionList task={task} />
    <VersionNotesSection task={task} />
    <AmtActivitySection task={task} />
    <JurisdictionGateSection task={task} />
    <SimulationSection task={task} />
    <ConsciousnessGateSection task={task} />
  </div>
);
export default TaskInsightSections;
