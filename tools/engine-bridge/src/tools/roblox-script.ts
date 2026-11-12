import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { executeRobloxCommand } from "../connectors/roblox/index.js";

function toolResult(result: Record<string, unknown>) {
  return {
    content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
    structuredContent: result
  };
}

export function registerRobloxScriptTools(server: McpServer) {
  server.registerTool(
    "roblox_get_script_source",
    {
      title: "Roblox Get Script Source",
      description: "Read the source code of a Script, LocalScript, or ModuleScript by its full instance path in Roblox Studio.",
      inputSchema: {
        path: z.string().min(1).describe("Full instance path of the script (e.g. ServerScriptService.MyScript)."),
        waitForResponseMs: z.number().int().min(0).max(30_000).default(5_000).describe("How long to wait for a runtime response.")
      }
    },
    async ({ path, waitForResponseMs }) => {
      const result = await executeRobloxCommand("get_script_source", { path }, { waitForResponseMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "roblox_set_script_source",
    {
      title: "Roblox Set Script Source",
      description: "Replace the entire source code of a Script, LocalScript, or ModuleScript in Roblox Studio.",
      inputSchema: {
        path: z.string().min(1).describe("Full instance path of the script."),
        source: z.string().describe("New source code for the script."),
        waitForResponseMs: z.number().int().min(0).max(30_000).default(5_000).describe("How long to wait for a runtime response.")
      }
    },
    async ({ path, source, waitForResponseMs }) => {
      const result = await executeRobloxCommand("set_script_source", { path, source }, { waitForResponseMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "roblox_edit_script_lines",
    {
      title: "Roblox Edit Script Lines",
      description: "Replace a substring in a script's source using old_string/new_string matching, similar to search-and-replace.",
      inputSchema: {
        path: z.string().min(1).describe("Full instance path of the script."),
        oldString: z.string().min(1).describe("Exact text to find in the script source."),
        newString: z.string().describe("Replacement text."),
        waitForResponseMs: z.number().int().min(0).max(30_000).default(5_000).describe("How long to wait for a runtime response.")
      }
    },
    async ({ path, oldString, newString, waitForResponseMs }) => {
      const result = await executeRobloxCommand(
        "edit_script_lines",
        { path, oldString, newString },
        { waitForResponseMs }
      );
      return toolResult(result);
    }
  );

  server.registerTool(
    "roblox_grep_scripts",
    {
      title: "Roblox Grep Scripts",
      description: "Search all scripts in the game for a Lua pattern or plain text match. Returns matching script paths and line content.",
      inputSchema: {
        pattern: z.string().min(1).describe("Search pattern (Lua pattern syntax supported)."),
        rootPath: z.string().default("game").describe("Root instance path to search within."),
        caseSensitive: z.boolean().default(true).describe("Whether the search is case-sensitive."),
        waitForResponseMs: z.number().int().min(0).max(30_000).default(10_000).describe("How long to wait for a runtime response.")
      }
    },
    async ({ pattern, rootPath, caseSensitive, waitForResponseMs }) => {
      const result = await executeRobloxCommand(
        "grep_scripts",
        { pattern, rootPath, caseSensitive },
        { waitForResponseMs }
      );
      return toolResult(result);
    }
  );
}
