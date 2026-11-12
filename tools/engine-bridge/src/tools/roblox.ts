import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { createRobloxWorkspacePartWorkflow, runRobloxCodeWorkflow } from "../connectors/roblox/index.js";

export function registerRobloxTools(server: McpServer) {
  server.registerTool(
    "roblox_create_workspace_part",
    {
      title: "Roblox Create Workspace Part",
      description: "Create a Part in Workspace through the Studio-side Luau runtime boundary.",
      inputSchema: {
        name: z.string().min(1).describe("Name of the new Workspace Part."),
        anchored: z.boolean().default(true).describe("Whether the new part should be anchored."),
        position: z.object({
          x: z.number(),
          y: z.number(),
          z: z.number()
        }).default({ x: 0, y: 5, z: 0 }).describe("Workspace position for the new part."),
        size: z.object({
          x: z.number(),
          y: z.number(),
          z: z.number()
        }).default({ x: 4, y: 1, z: 2 }).describe("Size for the new part."),
        waitForResponseMs: z.number().int().min(0).max(10_000).default(2_000).describe("How long to wait for a runtime response before returning queued status.")
      }
    },
    async ({ name, anchored, position, size, waitForResponseMs }) => {
      const result = await createRobloxWorkspacePartWorkflow(
        {
          name,
          anchored,
          position,
          size
        },
        {
          waitForResponseMs
        }
      );

      return {
        content: [
          {
            type: "text",
            text: JSON.stringify(result, null, 2)
          }
        ],
        structuredContent: result
      };
    }
  );

  server.registerTool(
    "roblox_run_code",
    {
      title: "Roblox Run Code",
      description: "Send a Luau code execution request through the Studio-side companion runtime boundary.",
      inputSchema: {
        code: z.string().min(1).describe("Luau code to execute through the Roblox runtime bridge."),
        mode: z.enum(["edit", "playtest"]).default("edit").describe("Execution mode for the runtime request."),
        waitForResponseMs: z.number().int().min(0).max(10_000).default(2_000).describe("How long to wait for a runtime response before returning queued status.")
      }
    },
    async ({ code, mode, waitForResponseMs }) => {
      const result = await runRobloxCodeWorkflow(code, {
        mode,
        waitForResponseMs
      });

      return {
        content: [
          {
            type: "text",
            text: JSON.stringify(result, null, 2)
          }
        ],
        structuredContent: result
      };
    }
  );
}
