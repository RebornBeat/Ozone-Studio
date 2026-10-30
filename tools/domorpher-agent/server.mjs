// domorpher-agent — the DOMorpher conversion's enforcement point (Layer 3,
// docs/DOMORPHER_CONVERSION_GUIDE.md §2.0c). Skeleton per the
// terminal-mcp hardening pattern: loopback-only /call accepting
// {tool, agent, input, context}; LOCKED until configured; per-tool locks;
// optional shared secret; platform/version tags at registration.
//
// BACKENDS (AG7): OZONE_DOMORPHER_BACKEND=farm|local (default farm).
//   farm  — Browser-Farm is the PRIMARY execution home: profile-scoped
//           actions via the registered farm tools, multi-step behaviors
//           via the Command-Center channel. Session/cookie state IS the
//           farm profile (persistent-profile aging native).
//   local — browser-mcp's local CDP for ISOLATED single-session use.
//   Backend is configuration, never a bypass: both execute through
//   /mcp/call with identical metering, jurisdiction gating, and ripple.
//
// The HARVEST (domorpher-core behind these tools) lands incrementally —
// until a capability's library function exists, its tool answers the
// honest shape {success:false, error:"harvest pending (tool declared,
// implementation queued)"} — declared-but-honest, never a mock success.

import { createServer } from "node:http";

const PORT = Number(process.env.OZONE_DOMORPHER_PORT ?? 3283);
const READONLY = process.env.OZONE_DOMORPHER_READONLY === "1";
const BACKEND = (process.env.OZONE_DOMORPHER_BACKEND ?? "farm").toLowerCase();
const TOKEN = process.env.OZONE_DOMORPHER_TOKEN ?? "";
// Navigation allowlist — browser-mcp convention: unset = navigation LOCKED.
const NAV_HOSTS = (process.env.OZONE_DOMORPHER_ALLOW_HOSTS ?? "")
  .split(",").map((s) => s.trim().toLowerCase()).filter(Boolean);
// The eval-equivalent gate (ActionType::ExecuteScript): OFF unless opted in.
const EVAL_ALLOWED = process.env.OZONE_DOMORPHER_ALLOW_EVAL === "1";
// Where domorpher-core mounts once harvest begins (Phase 5 of the guide).
// Set OZONE_DOMORPHER_CORE=1 only when the core crate is wired in this process.
const CORE_WIRED = process.env.OZONE_DOMORPHER_CORE === "1";

// The 16-tool table (conversion guide §2.2): one verb = one tool, from
// domorpher's own ActionType vocabulary. Custom(String) is DELIBERATELY
// absent — an arbitrary-code verb has no honest allowlist.
const TOOLS = {
  domorpher_click: { class: "interaction", destructive: false },
  domorpher_hover: { class: "interaction", destructive: false },
  domorpher_scroll: { class: "interaction", destructive: false },
  domorpher_key: { class: "interaction", destructive: false },
  domorpher_drag: { class: "interaction", destructive: false },
  domorpher_input: { class: "input", destructive: false },
  domorpher_select: { class: "input", destructive: false },
  domorpher_check: { class: "input", destructive: false },
  domorpher_upload: { class: "input", destructive: false },
  domorpher_navigate: { class: "navigation", destructive: false, needsAllowlist: true },
  domorpher_wait: { class: "safe", destructive: false },
  domorpher_switch_frame: { class: "safe", destructive: false },
  domorpher_extract: { class: "read", destructive: false },
  domorpher_dialog: { class: "dialog", destructive: false, confirm: true },
  domorpher_exec_script: { class: "script", destructive: true, evalGate: true },
  domorpher_status: { class: "safe", destructive: false },
};

const started = Date.now();

function status() {
  return {
    tool: "domorpher-agent",
    started_at: new Date(started).toISOString(),
    backend: BACKEND,
    core_wired: CORE_WIRED,
    locked: NAV_HOSTS.length === 0,
    allow_hosts: NAV_HOSTS,
    eval_allowed: EVAL_ALLOWED,
    readonly: READONLY,
    token_required: TOKEN !== "",
    tools: Object.keys(TOOLS),
    note: "harvest pending — declared tools answer honestly until domorpher-core is wired (OZONE_DOMORPHER_CORE=1)",
  };
}

function refused(why) {
  return { success: false, error: why };
}

function navAllowed(raw) {
  try {
    const host = new URL(String(raw)).hostname.toLowerCase();
    return NAV_HOSTS.includes(host);
  } catch {
    return false;
  }
}

async function handle(tool, input) {
  const spec = TOOLS[tool];
  if (!spec) return refused(`unknown tool '${tool}'`);
  if (READONLY && spec.class !== "read" && tool !== "domorpher_status")
    return refused("OZONE_DOMORPHER_READONLY=1: non-read agent tools are disabled");
  if (spec.evalGate && !EVAL_ALLOWED)
    return refused("domorpher_exec_script is LOCKED: OZONE_DOMORPHER_ALLOW_EVAL=1 required (ActionType::ExecuteScript — the eval-equivalent gate)");
  if (spec.needsAllowlist) {
    const url = String(input?.url ?? "");
    if (!navAllowed(url))
      return refused(`domorpher_navigate is LOCKED: '${url || "no url"}' is not in OZONE_DOMORPHER_ALLOW_HOSTS`);
  }
  if (!CORE_WIRED)
    return refused("harvest pending: domorpher-core is not wired in this process (OZONE_DOMORPHER_CORE=1 when it lands) — tool declared, implementation queued (conversion guide §3 step 5)");
  // Harvest dispatch lands here: route to the core's library function with
  // the PageDriver impl selected by BACKEND (farm tools / Command Center
  // channel for farm; browser-mcp CDP for local). Untrusted-labeled output
  // (AG5) is the wrapper's job, applied on the way out.
  return refused("core dispatch not yet implemented in this skeleton");
}

createServer((req, res) => {
  let body = "";
  req.on("data", (c) => (body += c));
  req.on("end", async () => {
    if (TOKEN && req.headers["x-ozone-agent-token"] !== TOKEN) {
      res.writeHead(401, { "content-type": "application/json" });
      return res.end(JSON.stringify(refused("bad or missing x-ozone-agent-token")));
    }
    if (req.url === "/status") {
      res.writeHead(200, { "content-type": "application/json" });
      return res.end(JSON.stringify(status()));
    }
    if (req.url !== "/call") {
      res.writeHead(404, { "content-type": "application/json" });
      return res.end(JSON.stringify(refused("POST /call or GET /status only")));
    }
    let call = {};
    try { call = JSON.parse(body || "{}"); } catch { /* fallthrough */ }
    const out = await handle(String(call.tool ?? ""), call.input ?? {});
    res.writeHead(200, { "content-type": "application/json" });
    res.end(JSON.stringify({ ...out, untrusted_result: out.success ? (out.output ?? null) : undefined }));
  });
}).listen(PORT, "127.0.0.1", () => {
  console.log(`[domorpher-agent] /call on 127.0.0.1:${PORT} | backend: ${BACKEND} | nav: ${NAV_HOSTS.length ? NAV_HOSTS.join(",") : "LOCKED"} | eval: ${EVAL_ALLOWED ? "on" : "off"} | core: ${CORE_WIRED ? "wired" : "not wired (honest refusals)"}`);
});
