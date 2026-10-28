import React from "react";
import GraphNativePanel from "../components/GraphNativePanel";
import { useRequestedFilesTab } from "../filesPanelTab";

export const FilesPanel: React.FC = () => {
  const [requestedTab] = useRequestedFilesTab();
  return (
    <GraphNativePanel
      title="Files & Code"
      subtitle="Graph-native workspace files, code/math viewers, the editor and version history — inside the project's graph world, Files lens."
      section="files"
      requestedFilesSub={requestedTab}
    />
  );
};

export default FilesPanel;
