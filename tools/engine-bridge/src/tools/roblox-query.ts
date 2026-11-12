import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { executeRobloxCommand } from "../connectors/roblox/index.js";

function toolResult(result: Record<string, unknown>) {
  return {
    content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
    structuredContent: result
  };
}

export function registerRobloxQueryTools(server: McpServer) {
  server.registerTool(
    "roblox_get_instance_properties",
    {
      title: "Roblox Get Instance Properties",
      description: "Get all readable properties of a Roblox instance at the given path.",
      inputSchema: {
        path: z.string().min(1).describe("Full instance path (e.g. Workspace.MyPart)."),
        waitForResponseMs: z.number().int().min(0).max(10_000).default(3_000).describe("How long to wait for a runtime response.")
      }
    },
    async ({ path, waitForResponseMs }) => {
      const result = await executeRobloxCommand("get_instance_properties", { path }, { waitForResponseMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "roblox_get_instance_children",
    {
      title: "Roblox Get Instance Children",
      description: "List the direct children of a Roblox instance, returning their names and class names.",
      inputSchema: {
        path: z.string().min(1).describe("Full instance path (e.g. Workspace)."),
        includeDescendants: z.boolean().default(false).describe("If true, recursively list all descendants instead of only direct children."),
        maxDepth: z.number().int().min(1).max(20).default(1).describe("Maximum recursion depth when includeDescendants is true."),
        waitForResponseMs: z.number().int().min(0).max(10_000).default(5_000).describe("How long to wait for a runtime response.")
      }
    },
    async ({ path, includeDescendants, maxDepth, waitForResponseMs }) => {
      const result = await executeRobloxCommand(
        "get_instance_children",
        { path, includeDescendants, maxDepth },
        { waitForResponseMs }
      );
      return toolResult(result);
    }
  );

  server.registerTool(
    "roblox_search_instances",
    {
      title: "Roblox Search Instances",
      description: "Search for instances by name, class, or property value under a root path.",
      inputSchema: {
        query: z.string().min(1).describe("Search query (matched against instance Name)."),
        className: z.string().optional().describe("Filter by ClassName."),
        rootPath: z.string().default("game").describe("Root instance path to search within."),
        maxResults: z.number().int().min(1).max(200).default(50).describe("Maximum number of results to return."),
        waitForResponseMs: z.number().int().min(0).max(15_000).default(5_000).describe("How long to wait for a runtime response.")
      }
    },
    async ({ query, className, rootPath, maxResults, waitForResponseMs }) => {
      const result = await executeRobloxCommand(
        "search_instances",
        { query, className, rootPath, maxResults },
        { waitForResponseMs }
      );
      return toolResult(result);
    }
  );

  server.registerTool(
    "roblox_get_file_tree",
    {
      title: "Roblox Get File Tree",
      description: "Get a hierarchical tree view of the game's instance structure, useful for understanding project layout.",
      inputSchema: {
        rootPath: z.string().default("game").describe("Root instance path for the tree."),
        maxDepth: z.number().int().min(1).max(10).default(3).describe("Maximum tree depth."),
        includeProperties: z.boolean().default(false).describe("Include basic properties (ClassName) in each node."),
        waitForResponseMs: z.number().int().min(0).max(15_000).default(5_000).describe("How long to wait for a runtime response.")
      }
    },
    async ({ rootPath, maxDepth, includeProperties, waitForResponseMs }) => {
      const result = await executeRobloxCommand(
        "get_file_tree",
        { rootPath, maxDepth, includeProperties },
        { waitForResponseMs }
      );
      return toolResult(result);
    }
  );
}
