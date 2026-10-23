import React from "react";
import PanelShell, { PanelTab } from "../components/PanelShell";
import CodeCallGraph from "../views/engines/CodeCallGraph";
import CodeEngine from "../views/engines/CodeEngine";
import ProofEngine from "../views/engines/ProofEngine";
import TextEntityBrowser from "../views/engines/TextEntityBrowser";
import TextGrammarViewer from "../views/engines/TextGrammarViewer";
import VariableScopeTree from "../views/engines/VariableScopeTree";
import { ImageEngine } from "../views/engines/ImageEngine";
import { ThreeDEngine } from "../views/engines/ThreeDEngine";
import { AudioEngine, VideoEngine } from "../views/engines/TimelineEngine";
import { GeospatialEngine } from "../views/engines/GeospatialEngine";
import { CADEngine } from "../views/engines/CADEngine";
import { useRequestedEnginesTab } from "../enginesPanelTab";

/**
 * Engine registry (2026-10-18, superseding the prior 3-tab hardcoded panel).
 * `modality` is the real backend pipeline-registration key
 * (src/orchestrator/graphs.rs:777 `modality_name_to_pipeline_id`) — kept on
 * each entry so a future live-switching consumer can resolve "orchestrator
 * is working on modality X" → the right tab id without re-deriving this
 * mapping. All 26 modalities have real backend graphs (confirmed this
 * session); only these three had dedicated engine UI before this change.
 *
 * Image/3D/Audio-Video/Geospatial/CAD dedicated engines (2026-10-18/19, same
 * session, parallel sibling forks) wired in below. The remaining ~18
 * modalities without a dedicated engine yet still reach the graph via
 * GraphView/FabricView directly (not through this panel) — add a tab here
 * the same way when one gets built.
 */
export const ENGINES: PanelTab[] = [
  {
    id: "code",
    label: "Code",
    render: (projectId) => (
      <>
        <CodeEngine projectId={projectId} />
        <CodeCallGraph projectId={projectId} />
      </>
    ),
  },
  {
    id: "math",
    label: "Math",
    render: (projectId) => (
      <>
        <ProofEngine projectId={projectId} />
        <VariableScopeTree projectId={projectId} />
      </>
    ),
  },
  {
    id: "text",
    label: "Text",
    render: (projectId) => (
      <>
        <TextEntityBrowser projectId={projectId} />
        <TextGrammarViewer projectId={projectId} />
      </>
    ),
  },
  {
    id: "image",
    label: "Image",
    render: (projectId) => <ImageEngine projectId={projectId} />,
  },
  {
    id: "3d",
    label: "3D",
    render: (projectId) => <ThreeDEngine projectId={projectId} />,
  },
  {
    id: "audio-video",
    label: "Audio / Video",
    render: (projectId) => (
      <>
        <AudioEngine projectId={projectId} />
        <VideoEngine projectId={projectId} />
      </>
    ),
  },
  {
    id: "geospatial",
    label: "Geospatial",
    render: (projectId) => <GeospatialEngine projectId={projectId} />,
  },
  {
    id: "parametric_cad",
    label: "CAD",
    render: (projectId) => <CADEngine projectId={projectId} />,
  },
];

export const EnginesPanel: React.FC = () => {
  const [requestedTab, setRequestedTab] = useRequestedEnginesTab();
  return (
    <PanelShell
      title="Modality Engines"
      subtitle="Per-modality graph engines built on the real modality graphs — Code/Math/Text/Image/3D/Audio-Video/Geospatial/CAD have dedicated views; the remaining modalities are reachable via Graph/Fabric directly until their own engines land."
      showProjectPicker={true}
      activeTab={requestedTab ?? undefined}
      onActiveTabChange={setRequestedTab}
      tabs={ENGINES}
    />
  );
};

export default EnginesPanel;
