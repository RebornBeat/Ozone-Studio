import React from "react";
import GraphNativePanel from "../components/GraphNativePanel";
import { useRequestedEnginesTab } from "../enginesPanelTab";

// The engine registry lives in enginesRegistry.tsx (kept out of this file so
// ProjectWorkspaceViewer and this panel can both import it without a cycle).
export { ENGINES } from "./enginesRegistry";

export const EnginesPanel: React.FC = () => {
  const [requestedTab] = useRequestedEnginesTab();
  return (
    <GraphNativePanel
      title="Modality Engines"
      subtitle="Per-modality graph engines built on the real modality graphs — Code/Math/Text/Image/3D/Audio-Video/Geospatial/CAD have dedicated views, every other modality has the All-modalities table — inside the project's graph world, Engines lens."
      section="engines"
      requestedEngineSub={requestedTab}
    />
  );
};

export default EnginesPanel;
