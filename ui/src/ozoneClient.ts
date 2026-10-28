/**
 * Ozone host client — bridge-first, browser-direct fallback.
 *
 * Under Electron, window.ozone.* IPC bridges carry the calls. In a plain
 * browser (Firefox/Chrome running the capture plugin), we talk to the
 * Ozone host REST surface directly — the same endpoints the plugin uses.
 * CORS on the host is allow-origin-any, so cross-origin fetches from the
 * vite dev origin are accepted.
 *
 * Device session: paired devices hold a session token (localStorage) from
 * the QR pairing flow and send it as `Authorization: Bearer <hex>` on every
 * call. The host validates it through the same session store as Ed25519
 * logins.
 */

export const OZONE_HOST: string =
  (typeof window !== "undefined" && (window as any).OZONE_HOST_URL) ||
  "http://127.0.0.1:50051";

export interface AgentInfo {
  pipeline_id: number;
  name: string;
  execute_url: string;
  registered_at: number;
  call_count: number;
  roles?: string[];
}

export interface ActivityEvent {
  id: number;
  timestamp: number;
  kind: string;
  level: string;
  source: string;
  message: string;
  detail?: unknown;
  /** Only present for pipeline dispatch/completion events — real task id, never fabricated. */
  task_id?: number;
}

export interface DeviceInfo {
  device_id: number;
  device_name: string;
  device_type: string;
  registered_at: number;
  last_seen: number;
  online: boolean;
}

export interface McpToolInfo {
  name: string;
  transport: string;
  endpoint: string;
  capabilities: string[];
  server_version?: string | null;
  registered_at: number;
  /** Operator-controlled (2026-10-18, src/mcp.rs): persisted, default false
   * for every tool. Toggling this on ANY tool sharing the same (transport,
   * endpoint) flips the whole group — see `affected` on setMcpAutoStart's
   * result. */
  auto_start: boolean;
  /** Real, computed (not persisted) facts — null unless this specific tool
   * actually needs one. Only android-mcp has a device_requirement
   * ("android"); only gcal-mcp/zoom-mcp have a credential_requirement
   * ("google_oauth"/"zoom_oauth") — confirmed by reading each tool's own
   * source, never guessed. */
  device_requirement: string | null;
  credential_requirement: string | null;
  /** Live: whether the supervisor currently tracks a running child process
   * for this tool's port. Never fabricated — false means genuinely not
   * running, including "registered but nothing ever started it." */
  running: boolean;
}

function deviceToken(): string | null {
  try {
    return localStorage.getItem("ozone_session_token");
  } catch {
    return null;
  }
}

async function bridgeOrHttp<T>(bridgeFn: string, httpPath: string): Promise<T> {
  const oz = (window as any).ozone;
  if (oz?.[bridgeFn]) {
    return (await oz[bridgeFn]()) as T;
  }
  // Desktop fallback: main-process HTTP bridge (renderer on file:// is
  // blocked from fetching localhost directly by Chromium PNA).
  if (oz?.http?.get) {
    return (await oz.http.get(httpPath)) as T;
  }
  const res = await fetch(`${OZONE_HOST}${httpPath}`, {});
  if (!res.ok) {
    throw new Error(`ozone host ${httpPath} → HTTP ${res.status}`);
  }
  return (await res.json()) as T;
}

/** Plain GET against the host through the same bridge-first path every other
 * client call uses (Electron: main-process HTTP bridge, because a file://
 * renderer cannot fetch localhost directly; browser: direct fetch). Exported
 * so components stop hand-rolling relative `fetch("/health")` calls that only
 * work when the page origin happens to be the host. */
export function hostGet<T = unknown>(httpPath: string): Promise<T> {
  return bridgeOrHttp<T>("", httpPath);
}

/** Plain JSON POST through the same bridge-first path (see hostGet). */
export function hostPost<T = unknown>(httpPath: string, body?: unknown): Promise<T> {
  return postHttp<T>(httpPath, body ?? {});
}

async function postHttp<T>(httpPath: string, body: unknown): Promise<T> {
  const oz = (window as any).ozone;
  if (oz?.http?.post) {
    return (await oz.http.post(httpPath, body)) as T;
  }
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
  };
  const token = deviceToken();
  if (token) headers["Authorization"] = `Bearer ${token}`;
  const res = await fetch(`${OZONE_HOST}${httpPath}`, {
    method: "POST",
    headers,
    body: JSON.stringify(body ?? {}),
  });
  if (!res.ok) {
    throw new Error(`ozone host ${httpPath} → HTTP ${res.status}`);
  }
  return (await res.json()) as T;
}

// ── Monitor + registry ────────────────────────────────────────────────────

export function fetchRemotePipelines(): Promise<{ pipelines: AgentInfo[] }> {
  return bridgeOrHttp("pipelinesRemote", "/pipelines/remote");
}

export function fetchMonitorSummary(): Promise<{
  agents: AgentInfo[];
  activity: ActivityEvent[];
}> {
  return bridgeOrHttp("monitorSummary", "/monitor/summary");
}

// ── Pairing (phone as authenticator) ──────────────────────────────────────

export function startPairing(deviceHint: string): Promise<{
  pairing_id: string;
  code: string;
  approve_url: string;
  qr_payload: string;
  expires_at: number;
}> {
  return postHttp("/pairing/start", { device_hint: deviceHint });
}

export function pairingStatus(pairingId: string): Promise<{
  status: "pending" | "approved" | "expired" | "unknown";
  session_token: string | null;
  device_id: number | null;
  expires_at: number | null;
}> {
  return bridgeOrHttp(
    "pairingStatus",
    `/pairing/status?pairing_id=${encodeURIComponent(pairingId)}`,
  );
}

export function listDevices(): Promise<{ devices: DeviceInfo[] }> {
  return bridgeOrHttp("devices", "/devices");
}

// ── MCP tools ─────────────────────────────────────────────────────────────

export function fetchMcpTools(): Promise<{ tools: McpToolInfo[] }> {
  return bridgeOrHttp("mcpTools", "/mcp/tools");
}

/** Toggle auto_start for the (transport, endpoint) GROUP `name` belongs to
 * (real route, src/grpc/mod.rs's mcp_set_autostart) — only flips the
 * persisted flag; the supervisor's own 30s loop does the actual starting,
 * honoring each tool's real device/credential requirement. `affected`
 * lists every tool name the toggle actually touched (may be more than
 * one — e.g. the 67-tool `bridges` fleet shares one process). */
export function setMcpAutoStart(
  name: string,
  enabled: boolean,
): Promise<{ success: boolean; enabled?: boolean; affected?: string[]; error?: string }> {
  return postHttp(`/mcp/tools/${encodeURIComponent(name)}/autostart`, { enabled });
}

/** Start `name`'s backing process right now, bypassing the auto_start
 * gate — still honors the same real device/credential check server-side,
 * so this can't route around an honestly-unmet requirement. The `error`
 * string on failure is the real reason (device/credential not met, script
 * missing, spawn failed) — never generalized. */
export function startMcpTool(
  name: string,
): Promise<{ success: boolean; started?: boolean; error?: string }> {
  return postHttp(`/mcp/tools/${encodeURIComponent(name)}/start`, {});
}

/** Stop `name`'s backing process. `stopped:false` honestly means nothing
 * was tracked as running for it. */
export function stopMcpTool(
  name: string,
): Promise<{ success: boolean; stopped?: boolean; error?: string }> {
  return postHttp(`/mcp/tools/${encodeURIComponent(name)}/stop`, {});
}

/** Generic dispatch to any registered MCP tool (POST /mcp/call, real route,
 * src/grpc/mod.rs:3437 — meters + jurisdiction-gates + forwards to the
 * tool's registered HTTP endpoint). `success:false` with a "tool
 * unreachable: ..." error is the honest, expected result when a registered
 * tool's backing process isn't currently running — never faked. */
export function callMcpTool<T = unknown>(
  tool: string,
  input: unknown,
): Promise<{ success: boolean; output?: T; error?: string }> {
  return postHttp("/mcp/call", { tool, agent: "ui", input, context: null, session_token: "" });
}

// ── Pipeline registry (the host's loaded pipelines) ──────────────────────

export interface PipelineRegistryEntry {
  pipeline_id: number;
  name: string;
  folder_name: string;
  category: string;
  has_ui: boolean;
  is_tab: boolean;
  description?: string;
}

export function fetchPipelineRegistry(): Promise<{
  success: boolean;
  registry: PipelineRegistryEntry[] | null;
}> {
  return postHttp("/pipeline/registry", {});
}

// ── ZSEI queries (generic graph read surface) ────────────────────────────
//
// Bridge-first: window.ozone.zsei.query (preload.js → IPC "zsei:query" →
// main.js POSTs {query, session_token:""} to /zsei/query). Browser-direct
// fallback posts the identical body shape straight to the host, matching
// main.js's own request exactly so both paths hit the same handler the
// same way.
//
// ENVELOPE UNWRAP (fixed 2026-09-27, found live by four independent forks):
// the host replies to POST /zsei/query with {"success":bool,"result":...,
// "error":...} — and BOTH the Electron bridge (main.js resolves the parsed
// body verbatim) and the browser fallback returned that envelope as-is.
// Every consumer reading `.Container`/`.Containers`/`.Content` at the top
// level silently saw undefined (empty project pickers, "container not
// found"). Unwrapped centrally here: throw on success===false, return
// body.result otherwise. Loaders written defensively (C8/C9/C10 tolerate
// both shapes) keep working; shallow readers start working.

function unwrapZseiEnvelope<T>(body: unknown): T {
  if (
    body !== null &&
    typeof body === "object" &&
    "success" in (body as Record<string, unknown>) &&
    "result" in (body as Record<string, unknown>)
  ) {
    const env = body as { success: unknown; result: unknown; error: unknown };
    if (env.success === false) {
      throw new Error(
        typeof env.error === "string" && env.error
          ? env.error
          : "zsei query failed (host returned success:false)",
      );
    }
    return env.result as T;
  }
  return body as T;
}

export async function zseiQuery<T = unknown>(query: Record<string, unknown>): Promise<T> {
  const oz = (window as any).ozone;
  if (oz?.zsei?.query) {
    return unwrapZseiEnvelope<T>(await oz.zsei.query(query));
  }
  const body = await postHttp<unknown>("/zsei/query", { query, session_token: "" });
  return unwrapZseiEnvelope<T>(body);
}

// ── Universal Order (docs/UNIVERSAL_ORDER_GUIDE.md) ───────────────────────

export type OrderItemKind =
  | "todo" | "meeting" | "followup" | "note" | "code" | "milestone" | "external";
export type OrderState = "live" | "paused" | "queued" | "interrupted" | "done" | "other";

export interface OrderItem {
  task_id: number;
  name: string;
  status: string;
  kind: OrderItemKind;
  due_at: number | null;
  remind_at: number | null;
  meeting_url: string | null;
  note_body: string | null;
  progress: number;
  steps_done: number;
  steps_total: number;
  assignee: string | null;
  source: string | null;
  workspace_id: number | null;
  project_id: number | null;
  created_at: number;
}

export type GlobalOrder = Record<OrderState, OrderItem[]> & {
  counts: Record<string, number>;
};

export function fetchGlobalOrder(params?: {
  workspaceId?: number;
  projectId?: number;
}): Promise<GlobalOrder> {
  const q = new URLSearchParams();
  if (params?.workspaceId != null) q.set("workspace_id", String(params.workspaceId));
  if (params?.projectId != null) q.set("project_id", String(params.projectId));
  const qs = q.toString();
  return bridgeOrHttp("globalOrder", `/order/global${qs ? `?${qs}` : ""}`);
}

/** Universal Order Stage 4 quick-capture — creates a personal item (no
 * assignee) through the same real /task/create path agent-coordination
 * tasks use (CoordinationTaskRequest widened additively for this). */
export function createOrderItem(item: {
  name: string;
  kind?: OrderItemKind;
  dueAt?: number;
  remindAt?: number;
  noteBody?: string;
  meetingUrl?: string;
  workspaceId?: number;
  projectId?: number;
}): Promise<{ success: boolean; task_id?: number; error?: string }> {
  return postHttp("/task/create", {
    prompt: item.name,
    session_token: "",
    created_by: "ui",
    kind: item.kind,
    due_at: item.dueAt,
    remind_at: item.remindAt,
    note_body: item.noteBody,
    meeting_url: item.meetingUrl,
    workspace_id: item.workspaceId,
    project_id: item.projectId,
  });
}

// ── Personal Assistant feed (docs/PERSONAL_ASSISTANT_GUIDE.md §4.2) ────────

export interface AssistantFinding {
  /** overdue | due-soon | meeting-soon | check-up-due | stalled |
   * paused-too-long | slippage-cascade */
  class: string;
  /** 3 = act now, 2 = today, 1 = worth knowing. */
  severity: 1 | 2 | 3;
  title: string;
  detail: string;
  task_id: number;
  project_id: number | null;
  workspace_id: number | null;
  due_at: number | null;
}

export interface AssistantFeed {
  scope: string;
  generated_at: number;
  counts: Record<string, number>;
  findings: AssistantFinding[];
}

/** Derived read — findings are computed from the task order at request time
 * by the same free local filter the consciousness check-up loop uses (one
 * canonical implementation server-side). No store, no LLM cost on this
 * route. Scope: "global" (default) | "ws:<id>" | "proj:<id>". */
export function fetchAssistantFeed(scope?: string): Promise<AssistantFeed> {
  const qs = scope ? `?scope=${encodeURIComponent(scope)}` : "";
  return bridgeOrHttp("assistantFeed", `/assistant/feed${qs}`);
}
