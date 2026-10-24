/**
 * GraphKit — the public API (docs/GRAPH_NATIVE_UIUX_GUIDE.md).
 *
 * One library for the graph-based engine: the substrate (GraphProvider),
 * the primitives (NodeCard / CrossLinkStrip / ModalityWheel / HonestState),
 * the indicators (real captured values only), and the lens pattern. Views
 * project; nothing fetches privately; nothing renders without its
 * cross-links. See the guide for the doctrine and the migration order.
 */
export { useGraphKit, useGraphKitProject } from "./graphProvider";
export type { GraphKitState } from "./graphProvider";
export {
  HonestState,
  NodeCard,
  CrossLinkStrip,
  ModalityWheel,
} from "./primitives";
export { GraphCanvas } from "./GraphCanvas";
export type { CanvasLayout, GraphCanvasProps } from "./GraphCanvas";
export {
  ModalityBadge,
  TaskStateIcon,
  StepPips,
  GateState,
  PeerState,
  EdgeRealismLegend,
  ActivityPulse,
} from "./indicators";
export {
  defineLens,
  genericLensFor,
  lensExistsFor,
  DEDICATED_ENGINE_MODALITIES,
} from "./lens";
export type { LensDefinition, LensBodyProps, LensLayout } from "./lens";
export type {
  GKCrossLink,
  GKContainerRef,
  GKFileRef,
  GKStatus,
  GKTaskRef,
  LensId,
} from "./types";
