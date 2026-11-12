import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { registerFoundationTools } from "../tools/foundation.js";
import { registerRobloxTools } from "../tools/roblox.js";
import { registerRobloxScriptTools } from "../tools/roblox-script.js";
import { registerRobloxInstanceTools } from "../tools/roblox-instance.js";
import { registerRobloxQueryTools } from "../tools/roblox-query.js";
import { registerUnityTools } from "../tools/unity.js";
import { registerUnrealTools } from "../tools/unreal.js";
import { registerBlenderTools } from "../tools/blender.js";
import { registerRobloxPhysicsTools } from "../tools/roblox-physics.js";
import { registerUnityPhysicsTools } from "../tools/unity-physics.js";
import { registerUnrealPhysicsTools } from "../tools/unreal-physics.js";
import { registerBlenderPhysicsTools } from "../tools/blender-physics.js";
import { NAME, VERSION } from "../version.js";

export function createServer() {
  const server = new McpServer({
    name: NAME,
    version: VERSION
  });

  registerFoundationTools(server);
  registerRobloxTools(server);
  registerRobloxScriptTools(server);
  registerRobloxInstanceTools(server);
  registerRobloxQueryTools(server);
  registerRobloxPhysicsTools(server);
  registerUnityTools(server);
  registerUnityPhysicsTools(server);
  registerUnrealTools(server);
  registerUnrealPhysicsTools(server);
  registerBlenderTools(server);
  registerBlenderPhysicsTools(server);

  return server;
}
