import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { executeUnrealCommand } from "../connectors/unreal/index.js";

function toolResult(result: Record<string, unknown>) {
  return {
    content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
    structuredContent: result
  };
}

export function registerUnrealTools(server: McpServer) {
  server.registerTool(
    "unreal_get_world_outliner",
    {
      title: "Unreal Get World Outliner",
      description: "Get the actor hierarchy from the current Unreal Engine level/world.",
      inputSchema: {
        rootPath: z.string().default("").describe("Optional root actor path. Empty for full world."),
        depth: z.number().int().min(1).max(50).default(10).describe("Maximum hierarchy depth."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ rootPath, depth, timeoutMs }) => {
      const result = await executeUnrealCommand("get_world_outliner", { rootPath, depth }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "unreal_get_actor",
    {
      title: "Unreal Get Actor",
      description: "Get properties and components of an actor by path in the Unreal level.",
      inputSchema: {
        path: z.string().min(1).describe("Actor path in the world (e.g. '/Game/Maps/Main.Main:PersistentLevel.PlayerStart')."),
        includeComponents: z.boolean().default(true).describe("Include component details."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ path, includeComponents, timeoutMs }) => {
      const result = await executeUnrealCommand("get_actor", { path, includeComponents }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "unreal_spawn_actor",
    {
      title: "Unreal Spawn Actor",
      description: "Spawn a new actor in the current Unreal level from a class or blueprint path.",
      inputSchema: {
        className: z.string().min(1).describe("Actor class or blueprint path (e.g. 'StaticMeshActor', '/Game/BP/BP_Enemy.BP_Enemy_C')."),
        label: z.string().default("").describe("Optional actor label."),
        location: z.object({ x: z.number(), y: z.number(), z: z.number() }).default({ x: 0, y: 0, z: 0 }).describe("Spawn location."),
        rotation: z.object({ pitch: z.number(), yaw: z.number(), roll: z.number() }).default({ pitch: 0, yaw: 0, roll: 0 }).describe("Spawn rotation."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ className, label, location, rotation, timeoutMs }) => {
      const result = await executeUnrealCommand("spawn_actor", { className, label, location, rotation }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "unreal_destroy_actor",
    {
      title: "Unreal Destroy Actor",
      description: "Destroy an actor in the current Unreal level by path.",
      inputSchema: {
        path: z.string().min(1).describe("Actor path to destroy."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ path, timeoutMs }) => {
      const result = await executeUnrealCommand("destroy_actor", { path }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "unreal_set_actor_transform",
    {
      title: "Unreal Set Actor Transform",
      description: "Set the location, rotation, and/or scale of an actor in the Unreal level.",
      inputSchema: {
        path: z.string().min(1).describe("Actor path."),
        location: z.object({ x: z.number(), y: z.number(), z: z.number() }).optional().describe("World location."),
        rotation: z.object({ pitch: z.number(), yaw: z.number(), roll: z.number() }).optional().describe("Rotation (pitch/yaw/roll)."),
        scale: z.object({ x: z.number(), y: z.number(), z: z.number() }).optional().describe("Scale."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ path, location, rotation, scale, timeoutMs }) => {
      const result = await executeUnrealCommand("set_actor_transform", { path, location, rotation, scale }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "unreal_set_actor_property",
    {
      title: "Unreal Set Actor Property",
      description: "Set a property on an actor or one of its components in Unreal.",
      inputSchema: {
        path: z.string().min(1).describe("Actor path."),
        component: z.string().default("").describe("Component name. Empty for actor-level property."),
        property: z.string().min(1).describe("Property name."),
        value: z.unknown().describe("Property value (JSON serializable)."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ path, component, property, value, timeoutMs }) => {
      const result = await executeUnrealCommand("set_actor_property", { path, component, property, value }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "unreal_get_blueprint",
    {
      title: "Unreal Get Blueprint",
      description: "Inspect a Blueprint asset, listing its components, variables, and functions.",
      inputSchema: {
        assetPath: z.string().min(1).describe("Blueprint asset path (e.g. '/Game/BP/BP_Enemy')."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ assetPath, timeoutMs }) => {
      const result = await executeUnrealCommand("get_blueprint", { assetPath }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "unreal_run_python",
    {
      title: "Unreal Run Python",
      description: "Execute a Python script in the Unreal Editor Python environment.",
      inputSchema: {
        code: z.string().min(1).describe("Python code to execute."),
        timeoutMs: z.number().int().min(0).max(30_000).default(10_000).describe("Command timeout.")
      }
    },
    async ({ code, timeoutMs }) => {
      const result = await executeUnrealCommand("run_python", { code }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "unreal_play_mode",
    {
      title: "Unreal Play Mode",
      description: "Control Unreal Editor PIE (Play In Editor) mode.",
      inputSchema: {
        action: z.enum(["play", "stop", "pause", "resume"]).describe("PIE action."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ action, timeoutMs }) => {
      const result = await executeUnrealCommand("play_mode", { action }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "unreal_get_viewport_screenshot",
    {
      title: "Unreal Get Viewport Screenshot",
      description: "Capture a screenshot of the active Unreal Editor viewport.",
      inputSchema: {
        width: z.number().int().min(64).max(3840).default(1280).describe("Screenshot width."),
        height: z.number().int().min(64).max(2160).default(720).describe("Screenshot height."),
        timeoutMs: z.number().int().min(0).max(30_000).default(10_000).describe("Command timeout.")
      }
    },
    async ({ width, height, timeoutMs }) => {
      const result = await executeUnrealCommand("get_viewport_screenshot", { width, height }, { timeoutMs });
      return toolResult(result);
    }
  );
}
