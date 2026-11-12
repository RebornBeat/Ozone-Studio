import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { executeRobloxCommand } from "../connectors/roblox/index.js";

function toolResult(result: Record<string, unknown>) {
  return {
    content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
    structuredContent: result
  };
}

export function registerRobloxInstanceTools(server: McpServer) {
  server.registerTool(
    "roblox_create_instance",
    {
      title: "Roblox Create Instance",
      description: "Create a new Roblox instance of a given class under a parent path.",
      inputSchema: {
        className: z.string().min(1).describe("Roblox class name (e.g. Part, Script, Model, Folder)."),
        parentPath: z.string().min(1).describe("Full instance path of the parent (e.g. Workspace, ServerScriptService)."),
        name: z.string().optional().describe("Name for the new instance."),
        properties: z.record(z.unknown()).default({}).describe("Properties to set on the new instance."),
        waitForResponseMs: z.number().int().min(0).max(10_000).default(3_000).describe("How long to wait for a runtime response.")
      }
    },
    async ({ className, parentPath, name, properties, waitForResponseMs }) => {
      const result = await executeRobloxCommand(
        "create_instance",
        { className, parentPath, name, properties },
        { waitForResponseMs }
      );
      return toolResult(result);
    }
  );

  server.registerTool(
    "roblox_delete_instance",
    {
      title: "Roblox Delete Instance",
      description: "Delete (Destroy) an instance at the given path in Roblox Studio.",
      inputSchema: {
        path: z.string().min(1).describe("Full instance path to delete."),
        waitForResponseMs: z.number().int().min(0).max(10_000).default(3_000).describe("How long to wait for a runtime response.")
      }
    },
    async ({ path, waitForResponseMs }) => {
      const result = await executeRobloxCommand("delete_instance", { path }, { waitForResponseMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "roblox_set_property",
    {
      title: "Roblox Set Property",
      description: "Set one or more properties on a Roblox instance.",
      inputSchema: {
        path: z.string().min(1).describe("Full instance path."),
        properties: z.record(z.unknown()).describe("Key-value pairs of properties to set (e.g. { Name: 'Foo', Anchored: true })."),
        waitForResponseMs: z.number().int().min(0).max(10_000).default(3_000).describe("How long to wait for a runtime response.")
      }
    },
    async ({ path, properties, waitForResponseMs }) => {
      const result = await executeRobloxCommand("set_property", { path, properties }, { waitForResponseMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "roblox_clone_instance",
    {
      title: "Roblox Clone Instance",
      description: "Clone an instance and place the clone under a target parent.",
      inputSchema: {
        sourcePath: z.string().min(1).describe("Full instance path of the source to clone."),
        targetParentPath: z.string().min(1).describe("Full instance path of the clone's parent."),
        newName: z.string().optional().describe("Optional new name for the clone."),
        waitForResponseMs: z.number().int().min(0).max(10_000).default(3_000).describe("How long to wait for a runtime response.")
      }
    },
    async ({ sourcePath, targetParentPath, newName, waitForResponseMs }) => {
      const result = await executeRobloxCommand(
        "clone_instance",
        { sourcePath, targetParentPath, newName },
        { waitForResponseMs }
      );
      return toolResult(result);
    }
  );

  server.registerTool(
    "roblox_reparent_instance",
    {
      title: "Roblox Reparent Instance",
      description: "Move an instance to a new parent in the Roblox hierarchy.",
      inputSchema: {
        path: z.string().min(1).describe("Full instance path to move."),
        newParentPath: z.string().min(1).describe("Full instance path of the new parent."),
        waitForResponseMs: z.number().int().min(0).max(10_000).default(3_000).describe("How long to wait for a runtime response.")
      }
    },
    async ({ path, newParentPath, waitForResponseMs }) => {
      const result = await executeRobloxCommand(
        "reparent_instance",
        { path, newParentPath },
        { waitForResponseMs }
      );
      return toolResult(result);
    }
  );
}
