import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { startLuauRuntimeBridge, stopLuauRuntimeBridge } from "./connectors/luau/bridge.js";
import { startUnityBridge, stopUnityBridge } from "./connectors/unity/index.js";
import { startUnrealBridge, stopUnrealBridge } from "./connectors/unreal/index.js";
import { startBlenderBridge, stopBlenderBridge } from "./connectors/blender/index.js";
import { createServer } from "./server/create-server.js";
import { startOzoneBridge } from "./ozone-bridge.js";

// Ozone-Studio engine bridge (vendored from gamedev-all-in-one, AGPL-3.0; see LICENSE).
//
// WHAT IS KEPT: the four engine bridges (Roblox/Luau, Unity, Unreal, Blender), the Ozone bridge layer (remote pipelines 200-203 on
// `/execute`, the 67 engine tools on `/call`, `bridge_status`), and the stdio MCP surface for external agents.
// WHAT WAS REMOVED (docs/evidence/bridges-incorporation): the provider console (src/control-plane: vendor APIs + CLI agents behind its own
// keys and its own /api/jobs) and the web dashboard + LLM proxy (src/web: /api/chat). Model calls belong to the HOST's provider layer
// (same adapter seam, jurisdiction gate, decision log and ledger); engine status is `bridge_status` on /call.
//
// OZONE_SUPERVISED=1 (set by the host's mcp_supervisor, stdin is /dev/null): skip the stdio MCP surface, serve /execute + /call only.

async function main() {
  try { startUnityBridge(); } catch (err) { console.error("[unity] bridge start failed:", err); }
  try { startUnrealBridge(); } catch (err) { console.error("[unreal] bridge start failed:", err); }
  try { startBlenderBridge(); } catch (err) { console.error("[blender] bridge start failed:", err); }

  // Register ALL FOUR engines with the Ozone host as remote pipelines (Roblox 200 / Unity 201 / Unreal 202 / Blender 203), heartbeat via
  // re-registration, answer the host's execute / call requests on 127.0.0.1:OZONE_BRIDGE_PORT (default 3210).
  try { startOzoneBridge(); } catch (err) { console.error("[ozone-bridge] start failed:", err); }

  await startLuauRuntimeBridge();

  const supervised = process.env["OZONE_SUPERVISED"] === "1";
  const server = createServer();

  const shutdown = async () => {
    console.error("ozone-engine-bridge shutting down...");
    stopUnityBridge();
    stopUnrealBridge();
    stopBlenderBridge();
    await Promise.allSettled([server.close(), stopLuauRuntimeBridge()]);
    process.exit(0);
  };

  process.on("SIGINT", shutdown);
  process.on("SIGTERM", shutdown);

  if (supervised) {
    console.error("ozone-engine-bridge running (supervised: /execute + /call only, no stdio MCP surface)");
    return;
  }
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error("ozone-engine-bridge running on stdio + /execute + /call");
}

main().catch(async (error) => {
  stopUnityBridge();
  stopUnrealBridge();
  stopBlenderBridge();
  await Promise.allSettled([stopLuauRuntimeBridge()]);
  console.error("ozone-engine-bridge failed:", error);
  process.exit(1);
});
