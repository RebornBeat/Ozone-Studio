/**
 * Graph event WebSocket client — live push from the host's `/ws` endpoint
 * (see src/grpc/mod.rs `websocket_handler` / `handle_websocket` and
 * src/graph_events.rs `GraphEventHub`).
 *
 * Every graph write on the host (container created/updated/deleted, file/
 * URL/package linked) is broadcast to EVERY connected socket, unfiltered.
 * `GraphEvent::visible_to()` on the host is advisory only — graph_events.rs's
 * own doc comment says scope filtering is subscriber-side. This client does
 * that real filtering: `onEvent(listener, scopeKeywords)` only calls the
 * listener for frames visible to that scope, using the same rule as the
 * host's `visible_to()` (mirrored in `graphEventVisibleTo` below).
 *
 * No auth handshake: `/ws` has no middleware on the host today (confirmed —
 * zero `Authorization`/`middleware`/`route_layer` hits in src/grpc/mod.rs's
 * router). This is a plain browser `WebSocket` talking directly to the host,
 * the same browser-direct model ozoneClient.ts already uses for HTTP (CORS
 * is already open there). No Electron IPC bridge: ozoneClient.ts's HTTP
 * bridge exists because Chromium's Private Network Access blocks a
 * packaged app's `file://`-origin renderer from *fetching* `127.0.0.1`
 * directly (see preload.js's own comment) — but the dev workflow this
 * fork's build/type-check runs against loads the renderer from
 * `http://localhost:5173` (electron/main.js `DEV_SERVER`), a normal HTTP
 * origin with no such restriction, so a direct WebSocket works there today.
 * Whether PNA (or an equivalent restriction) also blocks a *packaged*
 * app's raw `ws://` from a `file://` origin is NOT confirmed one way or
 * the other in this codebase (no CSP is set anywhere in electron/main.js,
 * and PNA enforcement has historically differed between fetch/XHR and the
 * WebSocket handshake) — flagged for whichever batch first wires this
 * client into a packaged-app build to verify against a real packaged run,
 * rather than guessing here.
 */

import { OZONE_HOST } from "./ozoneClient";

// ── Wire contract (pinned by t_g4_graph_event_frame_wire_contract in
// src/graph_events.rs, produced by graph_event_frame() in src/grpc/mod.rs) ─

export type GraphEventAction =
  | "created"
  | "updated"
  | "deleted"
  | "linked"
  // Trim markers (docs/CONTEXT_OBJECT_MODEL.md's "every cut is visible on
  // the graph, never silent" contract): emitted by src/context_budget.rs
  // and src/orchestrator/stages.rs. Added here because the closed union
  // was silently failing the type guard on these two real, already-wired
  // backend events — they were arriving over the wire and being dropped.
  | "context_trimmed"
  | "session_context_trimmed";

export interface GraphEventFrame {
  action: "graph_event";
  event: GraphEventAction;
  container_id: number;
  parent_id: number;
  container_type: string;
  source: string;
  scope_keywords: string[];
  timestamp: number;
}

export type GraphEventListener = (frame: GraphEventFrame) => void;

// ── pipeline_progress (src/pipeline/executor.rs PipelineProgress, pushed by
// handle_websocket's 500ms poll loop in src/grpc/mod.rs) — every real
// subprocess pipeline invocation (TextAnalysisPipeline, WorkspaceTab, the
// prompt pipeline, ...). progress_percent is genuinely only ever 0 (Running)
// or 100 (Completed/Failed/Cancelled) — there is no real mid-execution
// granularity, never render it as a smoothly-animating bar implying one. ──

export type PipelineProgressStatus = "Queued" | "Running" | "Completed" | "Failed" | "Cancelled";

export interface PipelineProgressFrame {
  action: "pipeline_progress";
  execution_id: string;
  pipeline_id: number;
  pipeline_name: string;
  status: PipelineProgressStatus;
  progress_percent: number;
  task_id: number | null;
  step_index: number | null;
  tokens_used: number | null;
}

export type PipelineProgressListener = (frame: PipelineProgressFrame) => void;

function isPipelineProgressFrame(value: unknown): value is PipelineProgressFrame {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    v.action === "pipeline_progress" &&
    typeof v.execution_id === "string" &&
    typeof v.pipeline_id === "number" &&
    typeof v.pipeline_name === "string" &&
    typeof v.status === "string" &&
    typeof v.progress_percent === "number"
  );
}

// ── orchestration_stage (src/orchestration_events.rs, emitted from the real
// record_stage/record_stage_timed choke point every stage already passes
// through) — the ONLY live signal for stages with no subprocess pipeline to
// track via pipeline_progress (Build AMT, Blueprint Assignment, Zero-Shot
// Simulation, ...). Scoped by (user_id, device_id) — the requester's own
// identity, already known client-side since it's who sent the /orchestrate
// call; filter to your own ids, there is no per-request id today (single-
// conversation-at-a-time UI, see the module doc comment on the Rust side). ──

export interface OrchestrationStageFrame {
  action: "orchestration_stage";
  user_id: number;
  device_id: number;
  stage: number;
  stage_name: string;
  success: boolean;
  /** Real dynamic text already produced by the stage — never fabricated. */
  summary: string;
  duration_ms: number;
  timestamp: number;
}

export type OrchestrationStageListener = (frame: OrchestrationStageFrame) => void;

function isOrchestrationStageFrame(value: unknown): value is OrchestrationStageFrame {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    v.action === "orchestration_stage" &&
    typeof v.user_id === "number" &&
    typeof v.device_id === "number" &&
    typeof v.stage === "number" &&
    typeof v.stage_name === "string" &&
    typeof v.success === "boolean" &&
    typeof v.summary === "string"
  );
}

/** Any other real `action` this socket may legitimately carry — recognized
 * by shape but not (yet) surfaced through a typed subscription. Distinct
 * from a genuinely malformed/unparseable frame. */
function hasKnownAction(value: unknown): value is { action: string } {
  return (
    typeof value === "object" &&
    value !== null &&
    typeof (value as Record<string, unknown>).action === "string"
  );
}

/** Real connection state — "open" is only ever reported once the socket's
 * readyState is actually WebSocket.OPEN, never fabricated ahead of it. */
export type GraphSocketState = "connecting" | "open" | "closed" | "error";

export interface GraphSocketStatus {
  state: GraphSocketState;
  /** The real last error/close reason, or null if none has occurred yet. */
  lastError: string | null;
  /** How many reconnect attempts have been made since the last successful open. */
  reconnectAttempt: number;
}

export type GraphSocketStatusListener = (status: GraphSocketStatus) => void;

const GLOBAL_SCOPE = "scope:global";

/**
 * Same rule as GraphEvent::visible_to() (src/graph_events.rs), evaluated
 * client-side: a viewer at global scope sees everything; a viewer scoped to
 * a workspace/project sees global events plus events sharing any of its own
 * scope keywords.
 */
export function graphEventVisibleTo(
  frame: Pick<GraphEventFrame, "scope_keywords">,
  viewerScopeKeywords: string[],
): boolean {
  if (viewerScopeKeywords.some((k) => k === GLOBAL_SCOPE)) return true;
  if (frame.scope_keywords.some((k) => k === GLOBAL_SCOPE)) return true;
  return viewerScopeKeywords.some((k) => frame.scope_keywords.includes(k));
}

/** Derives the `/ws` URL from OZONE_HOST (http→ws, https→wss) rather than a
 * separately-hardcoded base. */
export function graphEventsWsUrl(): string {
  const base = OZONE_HOST.replace(/^http/, "ws").replace(/\/+$/, "");
  return `${base}/ws`;
}

function isGraphEventFrame(value: unknown): value is GraphEventFrame {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    v.action === "graph_event" &&
    typeof v.event === "string" &&
    typeof v.container_id === "number" &&
    typeof v.parent_id === "number" &&
    typeof v.container_type === "string" &&
    typeof v.source === "string" &&
    Array.isArray(v.scope_keywords) &&
    v.scope_keywords.every((k) => typeof k === "string") &&
    typeof v.timestamp === "number"
  );
}

const BASE_BACKOFF_MS = 1000;
const MAX_BACKOFF_MS = 30_000;

interface EventSubscription {
  fn: GraphEventListener;
  /** null = unfiltered (every frame); otherwise scope-filtered via graphEventVisibleTo. */
  scope: string[] | null;
}

/**
 * Reconnecting WebSocket client for the host's graph-event ripple. Does not
 * connect until `connect()` is called. A dropped connection never kills the
 * feed silently — it reconnects with exponential backoff and reports real
 * status via `onStatusChange`/`getStatus`.
 */
export class GraphEventClient {
  private readonly url: string;
  private socket: WebSocket | null = null;
  private readonly eventListeners = new Set<EventSubscription>();
  private readonly pipelineProgressListeners = new Set<PipelineProgressListener>();
  private readonly orchestrationStageListeners = new Set<OrchestrationStageListener>();
  private readonly statusListeners = new Set<GraphSocketStatusListener>();
  private status: GraphSocketStatus = {
    state: "closed",
    lastError: null,
    reconnectAttempt: 0,
  };
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private closedByCaller = true;

  constructor(url: string = graphEventsWsUrl()) {
    this.url = url;
  }

  /** Opens the socket (idempotent if already connecting/open). */
  connect(): void {
    if (this.socket || this.reconnectTimer) return;
    this.closedByCaller = false;
    this.openSocket();
  }

  /** Closes the socket and stops reconnecting until connect() is called again. */
  close(): void {
    this.closedByCaller = true;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    const socket = this.socket;
    this.socket = null;
    socket?.close();
    this.setStatus({ state: "closed", lastError: null, reconnectAttempt: 0 });
  }

  getStatus(): GraphSocketStatus {
    return this.status;
  }

  /** True only when the underlying socket's readyState is really OPEN. */
  isOpen(): boolean {
    return this.socket !== null && this.socket.readyState === WebSocket.OPEN;
  }

  onStatusChange(listener: GraphSocketStatusListener): () => void {
    this.statusListeners.add(listener);
    listener(this.status);
    return () => {
      this.statusListeners.delete(listener);
    };
  }

  /**
   * Subscribe to graph events. When `scopeKeywords` is given, the listener
   * only fires for frames visible to that scope (graphEventVisibleTo);
   * omit it to receive every frame unfiltered. Returns an unsubscribe fn.
   */
  onEvent(listener: GraphEventListener, scopeKeywords?: string[]): () => void {
    const sub: EventSubscription = { fn: listener, scope: scopeKeywords ?? null };
    this.eventListeners.add(sub);
    return () => {
      this.eventListeners.delete(sub);
    };
  }

  /** Subscribe to real pipeline_progress frames (every subprocess pipeline
   * invocation) — unfiltered; there are usually few enough concurrent
   * executions that a consumer filtering by pipeline_name/execution_id
   * itself is simpler than adding another server-side scope convention. */
  onPipelineProgress(listener: PipelineProgressListener): () => void {
    this.pipelineProgressListeners.add(listener);
    return () => {
      this.pipelineProgressListeners.delete(listener);
    };
  }

  /** Subscribe to real orchestration_stage frames. Filter to your own
   * request by comparing frame.user_id/device_id to the ids you sent on
   * /orchestrate — this client does not filter for you (unlike onEvent's
   * scope keywords, ids are call-site data, not something this client
   * otherwise tracks). */
  onOrchestrationStage(listener: OrchestrationStageListener): () => void {
    this.orchestrationStageListeners.add(listener);
    return () => {
      this.orchestrationStageListeners.delete(listener);
    };
  }

  private openSocket(): void {
    this.setStatus({ ...this.status, state: "connecting" });

    let socket: WebSocket;
    try {
      socket = new WebSocket(this.url);
    } catch (err) {
      this.setStatus({
        state: "error",
        lastError: err instanceof Error ? err.message : String(err),
        reconnectAttempt: this.status.reconnectAttempt,
      });
      this.scheduleReconnect();
      return;
    }
    this.socket = socket;

    socket.onopen = () => {
      if (this.socket !== socket) return;
      this.setStatus({ state: "open", lastError: null, reconnectAttempt: 0 });
    };

    socket.onmessage = (evt) => {
      if (this.socket !== socket) return;
      this.handleMessage(evt.data);
    };

    socket.onerror = () => {
      if (this.socket !== socket) return;
      // The browser's WebSocket error event carries no real diagnostic
      // detail (by spec) — report that a real error occurred rather than
      // fabricating a reason; onclose (which fires next) carries the code.
      this.setStatus({
        state: "error",
        lastError: "websocket error",
        reconnectAttempt: this.status.reconnectAttempt,
      });
    };

    socket.onclose = (evt) => {
      if (this.socket !== socket) return; // stale handler from a prior socket
      this.socket = null;
      if (this.closedByCaller) {
        this.setStatus({ state: "closed", lastError: null, reconnectAttempt: 0 });
        return;
      }
      this.setStatus({
        state: "error",
        lastError: this.status.lastError ?? `connection closed (code ${evt.code})`,
        reconnectAttempt: this.status.reconnectAttempt,
      });
      this.scheduleReconnect();
    };
  }

  private handleMessage(data: unknown): void {
    if (typeof data !== "string") {
      // Frames are always JSON text per the wire contract — a real
      // unexpected payload is a reportable error, not a silent drop.
      this.setStatus({
        ...this.status,
        state: "error",
        lastError: "received non-text frame",
      });
      return;
    }
    let parsed: unknown;
    try {
      parsed = JSON.parse(data);
    } catch (err) {
      this.setStatus({
        ...this.status,
        state: "error",
        lastError: `malformed frame: ${err instanceof Error ? err.message : String(err)}`,
      });
      return;
    }
    if (isGraphEventFrame(parsed)) {
      // eslint-disable-next-line no-console
      console.debug("[graphEventClient] graph_event received", parsed);
      for (const { fn, scope } of this.eventListeners) {
        if (scope === null || graphEventVisibleTo(parsed, scope)) {
          fn(parsed);
        }
      }
      return;
    }
    if (isPipelineProgressFrame(parsed)) {
      for (const fn of this.pipelineProgressListeners) fn(parsed);
      return;
    }
    if (isOrchestrationStageFrame(parsed)) {
      for (const fn of this.orchestrationStageListeners) fn(parsed);
      return;
    }
    if (hasKnownAction(parsed)) {
      // A real frame this socket legitimately carries (e.g. "pong",
      // "subscribed", "cancel_requested" — server-side ack replies, see
      // handle_ws_message in src/grpc/mod.rs) that just has no typed
      // subscription here yet. Not a malformed-frame error.
      return;
    }
    this.setStatus({
      ...this.status,
      state: "error",
      lastError: `unrecognized frame shape: ${data.slice(0, 200)}`,
    });
  }

  private scheduleReconnect(): void {
    if (this.closedByCaller || this.reconnectTimer) return;
    const attempt = this.status.reconnectAttempt + 1;
    const delay = Math.min(BASE_BACKOFF_MS * 2 ** (attempt - 1), MAX_BACKOFF_MS);
    this.setStatus({ ...this.status, reconnectAttempt: attempt });
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      if (!this.closedByCaller) this.openSocket();
    }, delay);
  }

  private setStatus(status: GraphSocketStatus): void {
    this.status = status;
    for (const listener of this.statusListeners) listener(status);
  }
}

let singleton: GraphEventClient | null = null;

/** Lazily-created, process-wide client for the default host `/ws` endpoint.
 * Does not auto-connect — callers still call `.connect()`. */
export function getGraphEventClient(): GraphEventClient {
  if (!singleton) singleton = new GraphEventClient();
  return singleton;
}
