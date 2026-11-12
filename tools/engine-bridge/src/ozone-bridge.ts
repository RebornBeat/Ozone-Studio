// Ozone-Studio bridge shim — Phase 2 of docs/TOOLS_PIPELINES_MCP_GUIDE.md §4.
//
// ONE module instance registers ALL FOUR engines with the Ozone host as
// remote pipelines (connect model, CONTRACTS §2): Roblox 200, Unity 201,
// Unreal 202, Blender 203. Re-registration IS the heartbeat. Execute calls
// arrive at this process's /execute endpoint with the standard
// {data, context} envelope and dispatch to the engine's own
// execute<Engine>Command(method, params, options) — the same function the
// 67 MCP tools call, so bridge operations and tool operations share one
// command surface by construction.
//
// A 5th engine = one more entry in ENGINES below. Zero host changes.
//
// Env: OZONE_HOST (default http://127.0.0.1:50051), OZONE_BRIDGE_BASE_URL
// (URL Ozone should call back, default http://127.0.0.1:3210),
// OZONE_BRIDGE_PORT (default 3210), OZONE_BRIDGE_HEARTBEAT_SECS (default 30).

import { createServer, IncomingMessage, ServerResponse } from "node:http";
import { executeRobloxCommand } from "./connectors/roblox/index.js";
import { executeUnityCommand } from "./connectors/unity/index.js";
import { executeUnrealCommand } from "./connectors/unreal/index.js";
import { executeBlenderCommand, detectBlenderConnector } from "./connectors/blender/index.js";
import { detectRobloxConnector } from "./connectors/roblox/index.js";
import { detectLuauRuntime } from "./connectors/luau/index.js";
import { detectUnityConnector } from "./connectors/unity/index.js";
import { detectUnrealConnector } from "./connectors/unreal/index.js";
import { inspectProjectManifest } from "./project/manifest.js";
import { guardOk, internalHeaders } from "./ozone-auth.js";

const OZONE_HOST = process.env.OZONE_HOST ?? "http://127.0.0.1:50051";
const BASE_URL = process.env.OZONE_BRIDGE_BASE_URL ?? "http://127.0.0.1:3210";
const PORT = Number(process.env.OZONE_BRIDGE_PORT ?? 3210);
const HEARTBEAT_SECS = Number(process.env.OZONE_BRIDGE_HEARTBEAT_SECS ?? 30);

type EngineDispatch = (
  method: string,
  params: Record<string, unknown>,
  options?: { timeoutMs?: number }
) => Promise<Record<string, unknown>>;

const ENGINES: Record<
  number,
  { name: string; prefix: string; dispatch: EngineDispatch }
> = {
  200: { name: "roblox-bridge", prefix: "roblox", dispatch: executeRobloxCommand as EngineDispatch },
  201: { name: "unity-bridge", prefix: "unity", dispatch: executeUnityCommand as EngineDispatch },
  202: { name: "unreal-bridge", prefix: "unreal", dispatch: executeUnrealCommand as EngineDispatch },
  203: { name: "blender-bridge", prefix: "blender", dispatch: executeBlenderCommand as EngineDispatch },
};

async function register(pipelineId: number, name: string): Promise<boolean> {
  try {
    const res = await fetch(`${OZONE_HOST}/pipelines/register`, {
      method: "POST",
      // /pipelines/register is a PROTECTED host route on a strict host: carry the host's internal token.
      headers: internalHeaders(),
      body: JSON.stringify({
        pipeline_id: pipelineId,
        name,
        execute_url: `${BASE_URL}/execute`,
      }),
    });
    return res.ok;
  } catch {
    return false; // host down — heartbeat retries; never crashes the shim
  }
}

function reply(res: ServerResponse, code: number, body: unknown): void {
  res.writeHead(code, { "Content-Type": "application/json" });
  res.end(JSON.stringify(body));
}

/** Engine + project status (the capability the provider console's /api/status and the web dashboard carried): connectors, runtimes, manifest. */
async function bridgeStatus(): Promise<Record<string, unknown>> {
  const [roblox, luau, unity, unreal, blender] = await Promise.all([
    detectRobloxConnector(),
    detectLuauRuntime(),
    detectUnityConnector(),
    detectUnrealConnector(),
    detectBlenderConnector()
  ]);
  return { manifest: inspectProjectManifest(process.cwd()), roblox, luau, unity, unreal, blender };
}

const server = createServer((req: IncomingMessage, res: ServerResponse) => {
  // Same loopback guard as every Ozone MCP server: on a strict host only calls routed through the host (x-oz-internal) are accepted.
  if (!guardOk(req, res)) return;
  const url = req.url ?? "";
  if (req.method !== "POST" || !(url.startsWith("/execute") || url.startsWith("/call"))) {
    reply(res, 404, { error: "POST /execute or /call only" });
    return;
  }
  const chunks: Buffer[] = [];
  let size = 0;
  req.on("data", (c: Buffer) => {
    size += c.length;
    if (size > 2 * 1024 * 1024) req.destroy();
    else chunks.push(c);
  });
  req.on("end", () => {
    let pipelineId = 0;
    let method = "";
    let params: Record<string, unknown> = {};
    let timeoutMs: number | undefined;
    let toolName = "";
    try {
      const body = JSON.parse(Buffer.concat(chunks).toString("utf8") || "{}");
      const isCall = url.startsWith("/call");
      // /call (the /mcp/call dispatch surface): {tool, agent, input, context}
      // — tool name prefix picks the engine ("blender_get_scene" → 203 +
      // "get_scene"). /execute keeps the explicit {pipeline_id, method}.
      if (isCall && !body?.data) {
        toolName = String(body?.tool ?? "");
        if (toolName === "bridge_status") {
          bridgeStatus()
            .then((out) => reply(res, 200, { success: true, output: out }))
            .catch((err: unknown) => reply(res, 200, { success: false, error: err instanceof Error ? err.message : String(err) }));
          return;
        }
        const underscore = toolName.indexOf("_");
        const prefix = underscore > 0 ? toolName.slice(0, underscore) : "";
        const entry = Object.entries(ENGINES).find(
          ([, e]) => e.prefix === prefix
        );
        if (!entry) {
          reply(res, 200, {
            success: false,
            error: `tool '${toolName}' is not a bridge engine tool (engine tools: roblox_*, unity_*, unreal_*, blender_*; plus bridge_status; the foundation tools (doctor, list_capabilities, inspect_project, project_init) are on this package's stdio MCP server)`,
          });
          return;
        }
        pipelineId = Number(entry[0]);
        method = toolName.slice(underscore + 1);
        params = (body?.input as Record<string, unknown>) ?? {};
        timeoutMs = undefined;
      } else {
        // Ozone /execute sends the full {data, context} envelope; unwrap `data`.
        const data = body?.data ?? body ?? {};
        pipelineId = Number(data.pipeline_id ?? 0);
        method = String(data.method ?? "");
        params = (data.params as Record<string, unknown>) ?? {};
        timeoutMs = data.timeout_ms as number | undefined;
      }
    } catch {
      reply(res, 400, { success: false, error: "invalid JSON body" });
      return;
    }
    const engine = ENGINES[pipelineId];
    if (!engine || !method) {
      reply(res, 400, {
        success: false,
        error: `unknown bridge pipeline_id ${pipelineId} or missing method`,
      });
      return;
    }
    engine
      .dispatch(method, params, { timeoutMs })
      .then((result) => reply(res, 200, { success: true, output: result }))
      .catch((err: unknown) =>
        reply(res, 200, {
          success: false,
          error: err instanceof Error ? err.message : String(err),
        })
      );
  });
});

export function startOzoneBridge(): void {
  // Graceful degradation, same pattern as the other bridges: if the port
  // is taken (e.g. a second instance spawned by
  // scripts/register-tools-to-ozone.mjs while the long-running process
  // already serves /call), log and continue — registration/listTools must
  // not die over it.
  server.on("error", (err: NodeJS.ErrnoException) => {
    if (err.code === "EADDRINUSE") {
      console.error(
        `[ozone-bridge] :${PORT} already in use — another bridges instance serves /execute+/call; skipping`
      );
      return;
    }
    console.error("[ozone-bridge] server error:", err);
  });
  server.listen(PORT, "127.0.0.1", () => {
    console.error(`[ozone-bridge] execute endpoint on 127.0.0.1:${PORT}`);
    const beat = () => {
      for (const [id, engine] of Object.entries(ENGINES)) {
        void register(Number(id), engine.name);
      }
    };
    void beat();
    setInterval(beat, HEARTBEAT_SECS * 1000).unref();
    console.error(
      `[ozone-bridge] registered ${Object.keys(ENGINES).length} bridges with ${OZONE_HOST} (heartbeat ${HEARTBEAT_SECS}s)`
    );
  });
}
