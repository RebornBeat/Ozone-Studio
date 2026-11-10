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

// WEB MODE (no Electron bridge, page served over http/https): talk to the
// SAME origin the page came from — the host itself, a reverse proxy, or
// ui-harness.mjs. Headless Chrome refuses cross-origin loopback fetches
// even when CORS permits them ("Failed to fetch", 2026-10-29), and a
// paired browser reaching the host through nginx (EU deploy) is same-origin
// by construction. window.OZONE_HOST_URL stays the explicit override for
// split deployments. Electron (file:// renderer) keeps the absolute default.
const WEB_MODE =
  typeof window !== "undefined" &&
  !("ozone" in window) &&
  /^https?:$/.test(window.location?.protocol ?? "");

export const OZONE_HOST: string =
  (typeof window !== "undefined" && (window as any).OZONE_HOST_URL) ||
  (WEB_MODE ? "" : "http://127.0.0.1:50051");

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
  /** Failure-pause (host never continues past a failed step): error text and flag. */
  error?: string | null;
  paused_for_failure?: boolean;
}

/** An item the system is waiting on (docs/REVIEW_E2E_STATE_2026-11-01.md §10).
 * Shape ASSUMED from the review doc; every field except the task fields is
 * optional so an older/partial host answer still renders honestly.
 * resolver "meta" = consciousness/meta re-judges it itself (no human);
 * resolver "human" = a jurisdiction confirmation only the operator resolves. */
export interface AwaitingOrderItem extends Partial<Omit<OrderItem, "kind">> {
  task_id: number;
  await_id?: string;
  kind?: string; // consciousness_gate | meta_gate | jurisdiction_confirm
  resolver?: "meta" | "human" | string;
  attempts?: number;
  next_try_ts?: number | null; // unix ms or secs, see formatTs
  last_error?: string | null;
  unjudged_chunks?: number[];
  judged?: Record<string, string>;
  review_id?: string | null;
}

export type GlobalOrder = Record<OrderState, OrderItem[]> & {
  counts: Record<string, number>;
  /** Present once the host serves the 'awaiting' group (fork 4). */
  awaiting?: AwaitingOrderItem[];
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


// ── Awaiting items, pending reviews, consciousness state (2026-11-01) ──────
// Backend shapes below are ASSUMED from docs/REVIEW_E2E_STATE_2026-11-01.md
// §10/§17 and are all tolerant: a missing route rejects, callers degrade to
// honest absence. Nothing here is invented client-side.

/** The awaiting group of GET /order/global (empty array when the host does
 * not serve it yet). */
export async function fetchAwaiting(): Promise<AwaitingOrderItem[]> {
  const order = await fetchGlobalOrder();
  return Array.isArray(order.awaiting) ? order.awaiting : [];
}

/** POST /task/resume {task_id} — ask the host to re-enter the walk for an
 * awaiting task (meta drain does this itself; this is the manual door). */
export function resumeTask(taskId: number): Promise<{ success?: boolean; error?: string }> {
  return postHttp("/task/resume", { task_id: taskId, session_token: "" });
}

/** POST /task/resume_run {task_id} — resume a FAILURE-PAUSED task from exactly the step it
 * stopped at (the host never continues past a failed step; completed steps are restored,
 * not re-run). Answers immediately; follow status through the task record. */
export function resumeRun(taskId: number): Promise<{ success?: boolean; resumed?: boolean; from_step?: number; error?: string }> {
  return postHttp("/task/resume_run", { task_id: taskId });
}

/** POST /task/pause_info {task_id} — is the task failure-paused, at which step, with what
 * error, and is it resumable (checkpoint present)? */
export interface PauseInfo {
  task_id: number;
  found: boolean;
  paused_for_failure: boolean;
  paused_at_step: number | null;
  error: string | null;
  status: string | null;
  resumable: boolean;
  checkpoint: boolean;
}
export function fetchPauseInfo(taskId: number): Promise<PauseInfo> {
  return postHttp("/task/pause_info", { task_id: taskId });
}

/** Parse a failure-pause notice ("PAUSED: task N paused at step K: …" from /orchestrate, or
 * the task record's "paused at step K: …" error). null when it is not one. */
export function parsePause(text: unknown, taskIdHint?: number | null): { taskId: number | null; step: number; error: string } | null {
  const t = String(text ?? "");
  const m = t.match(/(?:PAUSED:\s*task\s*(\d+)\s*)?paused at step\s*(\d+)\s*:\s*([\s\S]*)/i);
  if (!m) return null;
  const taskId = m[1] ? Number(m[1]) : (taskIdHint ?? null);
  return { taskId, step: Number(m[2]), error: m[3].replace(/\s*It was not continued[\s\S]*$/, "").trim() };
}

export interface PendingReviewRow {
  review_id?: string;
  id?: string;
  ts?: string;
  decision?: string;
  review_kind?: string;
  resolver?: string;
  model_used?: string;
  task_summary_preview?: string;
  reasoning_preview?: string;
  raw_response_preview?: string;
}

/** GET /consciousness/reviews/pending. */
export function fetchPendingReviews(): Promise<{ count?: number; rows?: PendingReviewRow[] }> {
  return hostGet("/consciousness/reviews/pending");
}

/** POST /consciousness/reviews/resolve — the operator's disposition of a
 * human-actionable review (jurisdiction confirmations, assistant proposals). */
export function resolveReview(
  reviewId: string,
  decision: "Proceed" | "Decline",
  note?: string,
): Promise<{ success?: boolean; error?: string; [k: string]: unknown }> {
  return postHttp("/consciousness/reviews/resolve", {
    review_id: reviewId,
    decision,
    note,
    session_token: "",
  });
}

export interface ConsciousnessStateInfo {
  enabled?: boolean;
  emotion?: { primary_emotion?: string; valence?: number; arousal?: number; dominance?: number } | null;
  i_loop?: { running?: boolean; last_reflection_ts?: number | null; interval_secs?: number | null } | null;
  [k: string]: unknown;
}

/** GET /consciousness/state — real store-backed values only. */
export function fetchConsciousnessState(): Promise<ConsciousnessStateInfo> {
  return hostGet("/consciousness/state");
}

/** GET /network/paired-peers → count (0 when the route is gated/absent). */
export async function fetchPairedPeerCount(): Promise<number> {
  const r: any = await hostGet("/network/paired-peers");
  return Array.isArray(r?.paired_peers) ? r.paired_peers.length : 0;
}

// ── Marketplace (GET /marketplace/overview) ──────────────────────────────────────────────

export interface MarketplaceToolRequirement { kind: string; name: string; purpose?: string | null; optional?: boolean }
export interface MarketplaceRule {
  index: number;
  condition: string;
  action: string;
  priority: number;
  requires: MarketplaceToolRequirement[];
  /** "declared" = the methodology itself binds tools to this rule; "none" = nothing declared (inferred_tools are keyword guesses). */
  binding: "declared" | "none";
  /** keyword-overlap guesses (PROVIDER ids) for a rule that declares none — never presented as a binding */
  inferred_providers: string[];
}
export interface MarketplaceMethodology {
  id: number | null;
  name: string;
  file: string;
  source: "runtime" | "created";
  categories: string[];
  rule_count: number;
  rules_with_declared_tools: number;
  rules: MarketplaceRule[];
}
export interface MarketplaceBlueprint {
  id: number | null;
  name: string;
  step_count: number;
  pipelines: number[];
  methodology_ids: number[];
  declares_tool_calls: boolean;
}
export interface MarketplaceProvider {
  /** the MCP server id (supervisor port table dir), e.g. `threed-mcp` — the unit installed / trusted / started / shared */
  provider: string;
  endpoints: string[];
  endpoint: string;
  tools: string[];
  tool_count: number;
  running: boolean;
  auto_start: boolean;
  device_requirement: string | null;
  credential_requirement: string | null;
  capabilities: string[];
  source: "local";
  installed_from: string | null;
}
export interface MarketplaceWanted {
  kind: string;
  name: string;
  status: "wanted" | "satisfied";
  count: number;
  first_seen: number;
  last_seen: number;
  details: { source: string; detail: string; at: number }[];
}
export interface MarketplaceProviderHealth { calls: number; failures: number; degraded: boolean; last_error: string | null }
export interface MarketplaceBindingProposal { methodology_id: number; provider?: string; tool: string; successes: number; status: string; example_step: string; last_seen: number }

export interface MarketplacePeerOffer {
  offer_key: string;
  from_peer: string;
  item_type: string;
  name: string;
  body_hash: string;
  keywords: string[];
  announced_at?: number;
  received_at?: number;
  /** have = same name + same content already here · conflict = same name, different content (kept as a separate version, never overwritten) · same_name = a TOOL with this name is already registered (name only, no hash) · new */
  local_status: "have" | "conflict" | "same_name" | "new" | "not_checked";
  decision: "pending" | "download" | "ignore" | "not_needed";
  note: string;
  /** outcome of an approved download: imported / have / staged / approval_only / failed / rejected (null = not started or in flight) */
  import: null | { status: string; detail?: string; local_name?: string; local_id?: number; path?: string; at?: number };
  /** the gate's ADVISORY recommendation (zero-shot, judged from the manifest alone) — the operator's decision is what acts; null = not reviewed yet */
  review: null | { recommendation: "download" | "ignore" | "review"; reason?: string; by?: string; rounds?: string[]; strength?: number; at?: number };
  /** true while an approved download has been requested and its chunks have not all arrived */
  downloading: boolean;
}

export interface MarketplaceOverview {
  success: boolean;
  generated_at: number;
  tools: { count: number; providers: MarketplaceProvider[] };
  /** required-but-unavailable capabilities (methodology binding / blueprint / simulation) — to be built or downloaded after review */
  wanted: MarketplaceWanted[];
  provider_health: Record<string, MarketplaceProviderHealth>;
  binding_proposals: MarketplaceBindingProposal[];
  peers: { paired: string[]; catalog_exchange: string; note: string };
  /** offers published by connected peers (manifests only) — gated here; nothing is downloaded until the operator approves */
  peer_offers?: MarketplacePeerOffer[];
  /** live state of the marketplace exchange — real values only */
  exchange?: { publishing_enabled: boolean; p2p_enabled: boolean; connected_peers: number; refused_inbound: number };
  peer_offers_pending?: number;
  /** what THIS instance publishes to the marketplace (manifests only; blueprints are never published) */
  published?: { item_type: string; name: string; body_hash?: string; keywords?: string[]; announced_at?: number }[];
  methodologies: MarketplaceMethodology[];
  blueprints: MarketplaceBlueprint[];
  policy: { auto_broadcast_sync: boolean; downloads_require_review: boolean; executables_require_human_approval: boolean; installed_from_peer: unknown[] };
}

export function fetchMarketplaceOverview(): Promise<MarketplaceOverview> {
  return hostGet<MarketplaceOverview>("/marketplace/overview");
}

/** Record the operator's decision on a peer offer. `download` approves it (methodologies are fetched + imported, jurisdiction files are staged — never
 *  activated — and tools / pipelines stay approval-only); `ignore` dismisses it. */
export function decidePeerOffer(offerKey: string, decision: "download" | "ignore" | "pending"): Promise<{ success: boolean; note?: string; error?: string }> {
  return hostPost("/marketplace/offers/decide", { offer_key: offerKey, decision });
}
