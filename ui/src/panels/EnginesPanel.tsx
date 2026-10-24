import React, { useEffect, useState } from "react";
import PanelShell, { PanelTab } from "../components/PanelShell";
import { DEDICATED_ENGINE_MODALITIES } from "../graphkit/lens";
import { MODALITY_COLOR } from "../graphRenderers/defaults";
import type { Modality } from "../graphViewTypes";
import type { GraphViewStatus } from "../graphViewTypes";
import { loadGraphData } from "../graphViewData";
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
/**
 * AllModalitiesView (GraphKit wave 6, 2026-10-23): the 18 modalities without
 * dedicated engine views, reachable here — honest typed table per modality
 * via the SAME shared fetch/conversion path (loadGraphData, whose
 * discriminator now covers all 27). Nothing modality-shaped is invisible at
 * this surface anymore.
 */
const AllModalitiesView: React.FC<{ projectId: number | null }> = ({ projectId }) => {
  const dedicated = new Set(DEDICATED_ENGINE_MODALITIES);
  const all = (Object.keys(MODALITY_COLOR) as Modality[]).filter((m) => !dedicated.has(m));
  const [modality, setModality] = useState<Modality>(all[0]);
  const [status, setStatus] = useState<GraphViewStatus>({ kind: "idle" });
  useEffect(() => {
    if (projectId === null) return;
    setStatus({ kind: "loading" });
    return loadGraphData(projectId, setStatus, { modality });
  }, [projectId, modality]);
  const nodes = status.kind === "ready" ? status.data.nodes : [];
  const edges = status.kind === "ready" ? status.data.edges : [];
  return (
    <div>
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8, flexWrap: "wrap" }}>
        <span style={{ fontSize: 12.5, fontWeight: 700, color: "var(--color-text)" }}>Modality</span>
        <select
          value={modality}
          onChange={(e) => setModality(e.target.value as Modality)}
          style={{
            background: "var(--color-bg)",
            color: "var(--color-text)",
            border: "1px solid var(--color-border-faint)",
            borderRadius: 6,
            padding: "3px 8px",
            fontSize: 12,
          }}
        >
          {all.map((m) => (
            <option key={m} value={m}>
              {MODALITY_COLOR[m] !== undefined ? m : m}
            </option>
          ))}
        </select>
        <span style={{ fontSize: 11, color: "var(--color-text-muted)" }}>
          {status.kind === "ready" ? `${nodes.length} nodes · ${edges.length} edges` : status.kind === "empty" ? "no graph for this project yet" : status.kind}
        </span>
      </div>
      {status.kind === "loading" && <div style={{ color: "var(--color-text-muted)", fontSize: 12 }}>Loading…</div>}
      {status.kind === "error" && <div style={{ color: "#ff8a8a", fontSize: 12 }}>Error: {status.message}</div>}
      {status.kind === "empty" && (
        <div style={{ color: "var(--color-text-muted)", fontSize: 12 }}>
          No {modality} graph exists for this project yet — run an orchestration with {modality} data attached.
        </div>
      )}
      {status.kind === "ready" && (
        <div style={{ display: "flex", flexDirection: "column", gap: 1 }}>
          {nodes.slice(0, 200).map((n) => (
            <div
              key={n.id}
              style={{ display: "flex", gap: 8, fontSize: 11.5, color: "var(--color-text-secondary)", padding: "1px 4px" }}
            >
              <span style={{ color: "var(--color-text-muted)", minWidth: 110 }}>{n.nodeType}</span>
              <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{n.label}</span>
            </div>
          ))}
          {nodes.length > 200 && (
            <div style={{ fontSize: 10.5, color: "var(--color-text-muted)", marginTop: 4 }}>
              +{nodes.length - 200} more nodes (showing first 200)
            </div>
          )}
        </div>
      )}
    </div>
  );
};

export const ENGINES: PanelTab[] = [
  {
    id: "all-modalities",
    label: "All modalities",
    render: (projectId) => <AllModalitiesView projectId={projectId} />,
  },
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
