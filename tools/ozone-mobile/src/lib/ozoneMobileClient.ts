// Real API client — mirrors ui/src/ozoneClient.ts's exact real shapes
// (OrderItem/GlobalOrder types, /order/global, /task/create), confirmed
// by reading that file directly rather than guessed, so the mobile app's
// day view reads the SAME real backend contract the desktop OrderPanel
// already proves works end-to-end this session.
import { ensureSessionToken, OzoneHostConfig } from "./deviceIdentity";

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

export async function checkHealth(host: OzoneHostConfig): Promise<{ healthy: boolean; version?: string; uptime_secs?: number }> {
  const res = await fetch(`${host.baseUrl}/health`, { method: "GET" });
  return res.json();
}

export async function fetchGlobalOrder(host: OzoneHostConfig, params?: { workspaceId?: number; projectId?: number }): Promise<GlobalOrder> {
  const q = new URLSearchParams();
  if (params?.workspaceId != null) q.set("workspace_id", String(params.workspaceId));
  if (params?.projectId != null) q.set("project_id", String(params.projectId));
  const qs = q.toString();
  const res = await fetch(`${host.baseUrl}/order/global${qs ? `?${qs}` : ""}`, { method: "GET" });
  return res.json();
}

/** Real Universal Order quick-capture — same /task/create path the
 * desktop OrderPanel and the CLI's `ozone capture` both use, confirmed
 * working end-to-end earlier this session. */
export async function createOrderItem(
  host: OzoneHostConfig,
  item: {
    name: string;
    kind?: OrderItemKind;
    dueAt?: number;
    remindAt?: number;
    noteBody?: string;
    meetingUrl?: string;
    workspaceId?: number;
    projectId?: number;
  }
): Promise<{ success: boolean; task_id?: number; error?: string }> {
  const sessionToken = await ensureSessionToken(host);
  const res = await fetch(`${host.baseUrl}/task/create`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      prompt: item.name,
      session_token: sessionToken,
      created_by: "mobile",
      kind: item.kind,
      due_at: item.dueAt,
      remind_at: item.remindAt,
      note_body: item.noteBody,
      meeting_url: item.meetingUrl,
      workspace_id: item.workspaceId,
      project_id: item.projectId,
    }),
  });
  return res.json();
}
