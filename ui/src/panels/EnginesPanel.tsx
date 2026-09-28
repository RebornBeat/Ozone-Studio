import React from "react";
import PanelShell from "../components/PanelShell";
import CodeCallGraph from "../views/engines/CodeCallGraph";
import CodeEngine from "../views/engines/CodeEngine";
import ProofEngine from "../views/engines/ProofEngine";
import TextEntityBrowser from "../views/engines/TextEntityBrowser";
import TextGrammarViewer from "../views/engines/TextGrammarViewer";
import VariableScopeTree from "../views/engines/VariableScopeTree";

export const EnginesPanel: React.FC = () => (
  <PanelShell
    title="Modality Engines"
    subtitle="Code, Math and Text engines built on the real modality graphs."
    showProjectPicker={true}
    tabs={[
    { id: "code", label: "Code", render: (projectId) => (<><CodeEngine projectId={projectId} /><CodeCallGraph projectId={projectId} /></>) },
    { id: "math", label: "Math", render: (projectId) => (<><ProofEngine projectId={projectId} /><VariableScopeTree projectId={projectId} /></>) },
    { id: "text", label: "Text", render: (projectId) => (<><TextEntityBrowser projectId={projectId} /><TextGrammarViewer projectId={projectId} /></>) },
    ]}
  />
);

export default EnginesPanel;
