import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { executeUnityCommand } from "../connectors/unity/index.js";

function toolResult(result: Record<string, unknown>) {
  return {
    content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
    structuredContent: result
  };
}

export function registerUnityTools(server: McpServer) {
  server.registerTool(
    "unity_get_hierarchy",
    {
      title: "Unity Get Hierarchy",
      description: "Get the full scene hierarchy tree from the active Unity scene.",
      inputSchema: {
        rootPath: z.string().default("").describe("Optional root GameObject path to start from. Empty for full scene."),
        depth: z.number().int().min(1).max(50).default(10).describe("Maximum hierarchy depth."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ rootPath, depth, timeoutMs }) => {
      const result = await executeUnityCommand("get_hierarchy", { rootPath, depth }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "unity_get_gameobject",
    {
      title: "Unity Get GameObject",
      description: "Get properties of a specific GameObject by path in the Unity scene.",
      inputSchema: {
        path: z.string().min(1).describe("Full path of the GameObject (e.g. 'Canvas/Panel/Button')."),
        includeComponents: z.boolean().default(true).describe("Include component list."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ path, includeComponents, timeoutMs }) => {
      const result = await executeUnityCommand("get_gameobject", { path, includeComponents }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "unity_create_gameobject",
    {
      title: "Unity Create GameObject",
      description: "Create a new GameObject in the Unity scene under a parent path.",
      inputSchema: {
        name: z.string().min(1).describe("Name for the new GameObject."),
        parentPath: z.string().default("").describe("Parent GameObject path. Empty for scene root."),
        primitiveType: z.enum(["Empty", "Cube", "Sphere", "Capsule", "Cylinder", "Plane", "Quad"]).default("Empty").describe("Primitive type to create."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ name, parentPath, primitiveType, timeoutMs }) => {
      const result = await executeUnityCommand("create_gameobject", { name, parentPath, primitiveType }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "unity_delete_gameobject",
    {
      title: "Unity Delete GameObject",
      description: "Delete a GameObject from the Unity scene by path.",
      inputSchema: {
        path: z.string().min(1).describe("Full path of the GameObject to delete."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ path, timeoutMs }) => {
      const result = await executeUnityCommand("delete_gameobject", { path }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "unity_set_component_property",
    {
      title: "Unity Set Component Property",
      description: "Set a property on a component attached to a GameObject in Unity.",
      inputSchema: {
        path: z.string().min(1).describe("Full path of the GameObject."),
        component: z.string().min(1).describe("Component type name (e.g. 'Transform', 'MeshRenderer')."),
        property: z.string().min(1).describe("Property name."),
        value: z.unknown().describe("Property value (JSON serializable)."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ path, component, property, value, timeoutMs }) => {
      const result = await executeUnityCommand("set_component_property", { path, component, property, value }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "unity_add_component",
    {
      title: "Unity Add Component",
      description: "Add a component to a GameObject in the Unity scene.",
      inputSchema: {
        path: z.string().min(1).describe("Full path of the GameObject."),
        component: z.string().min(1).describe("Component type to add (e.g. 'Rigidbody', 'BoxCollider')."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ path, component, timeoutMs }) => {
      const result = await executeUnityCommand("add_component", { path, component }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "unity_set_transform",
    {
      title: "Unity Set Transform",
      description: "Set the position, rotation, and/or scale of a GameObject's Transform.",
      inputSchema: {
        path: z.string().min(1).describe("Full path of the GameObject."),
        position: z.object({ x: z.number(), y: z.number(), z: z.number() }).optional().describe("World position."),
        rotation: z.object({ x: z.number(), y: z.number(), z: z.number() }).optional().describe("Euler rotation in degrees."),
        scale: z.object({ x: z.number(), y: z.number(), z: z.number() }).optional().describe("Local scale."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ path, position, rotation, scale, timeoutMs }) => {
      const result = await executeUnityCommand("set_transform", { path, position, rotation, scale }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "unity_get_script_source",
    {
      title: "Unity Get Script Source",
      description: "Read the source code of a C# script file from the Unity project's Assets folder.",
      inputSchema: {
        scriptPath: z.string().min(1).describe("Relative path from Assets/ (e.g. 'Scripts/PlayerController.cs')."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ scriptPath, timeoutMs }) => {
      const result = await executeUnityCommand("get_script_source", { scriptPath }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "unity_play_mode",
    {
      title: "Unity Play Mode",
      description: "Control Unity Editor play mode — enter, exit, or pause.",
      inputSchema: {
        action: z.enum(["play", "stop", "pause", "unpause"]).describe("Play mode action."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ action, timeoutMs }) => {
      const result = await executeUnityCommand("play_mode", { action }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "unity_execute_menu_item",
    {
      title: "Unity Execute Menu Item",
      description: "Execute a Unity Editor menu item by its path (e.g. 'File/Save', 'Assets/Refresh').",
      inputSchema: {
        menuPath: z.string().min(1).describe("Menu item path (e.g. 'File/Save')."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ menuPath, timeoutMs }) => {
      const result = await executeUnityCommand("execute_menu_item", { menuPath }, { timeoutMs });
      return toolResult(result);
    }
  );
}
