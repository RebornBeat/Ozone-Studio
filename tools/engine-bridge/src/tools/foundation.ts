import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { detectBlenderConnector } from "../connectors/blender/index.js";
import { detectLuauRuntime } from "../connectors/luau/index.js";
import { detectRobloxConnector } from "../connectors/roblox/index.js";
import { detectUnityConnector } from "../connectors/unity/index.js";
import { detectUnrealConnector } from "../connectors/unreal/index.js";
import {
  createProjectManifest,
  inspectProjectManifest,
  type ManifestInspection
} from "../project/manifest.js";

type CapabilityReport = {
  manifest: ManifestInspection;
  roblox: Awaited<ReturnType<typeof detectRobloxConnector>>;
  luau: Awaited<ReturnType<typeof detectLuauRuntime>>;
  unity: Awaited<ReturnType<typeof detectUnityConnector>>;
  unreal: Awaited<ReturnType<typeof detectUnrealConnector>>;
  blender: Awaited<ReturnType<typeof detectBlenderConnector>>;
  workflows: {
    toolsOnlyCompatible: boolean;
    defaultTransport: "stdio";
  };
};

async function buildCapabilityReport(cwd = process.cwd()): Promise<CapabilityReport> {
  const [roblox, luau, unity, unreal, blender] = await Promise.all([
    detectRobloxConnector(),
    detectLuauRuntime(),
    detectUnityConnector(),
    detectUnrealConnector(),
    detectBlenderConnector()
  ]);

  return {
    manifest: inspectProjectManifest(cwd),
    roblox,
    luau,
    unity,
    unreal,
    blender,
    workflows: {
      toolsOnlyCompatible: true,
      defaultTransport: "stdio"
    }
  };
}

export function registerFoundationTools(server: McpServer) {
  server.registerTool(
    "project_init",
    {
      title: "Project Init",
      description: "Create the local Roblox all-in-one MCP project manifest in the current workspace.",
      inputSchema: {
        projectName: z.string().optional().describe("Optional display name for the project manifest."),
        enableBlender: z.boolean().default(true).describe("Enable Blender integration in the created manifest."),
        enableLuau: z.boolean().default(true).describe("Enable the Luau companion runtime in the created manifest."),
        force: z.boolean().default(false).describe("Overwrite an existing valid manifest.")
      }
    },
    async ({ projectName, enableBlender, enableLuau, force }) => {
      const result = createProjectManifest(process.cwd(), {
        projectName,
        enableBlender,
        enableLuau,
        force
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

  server.registerTool(
    "inspect_project",
    {
      title: "Inspect Project",
      description: "Inspect the current project manifest and connector/runtime capability state.",
      inputSchema: {
        includeCapabilities: z.boolean().default(true).describe("Include live connector capability detection in the response.")
      }
    },
    async ({ includeCapabilities }) => {
      const manifest = inspectProjectManifest(process.cwd());
      const report = includeCapabilities ? await buildCapabilityReport(process.cwd()) : null;
      const result = {
        manifest,
        capabilities: report
      };

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
    "list_capabilities",
    {
      title: "List Capabilities",
      description: "Return the current local capabilities for Roblox, Luau runtime, Blender, manifest state, and MCP workflow compatibility.",
      inputSchema: {}
    },
    async () => {
      const report = await buildCapabilityReport();

      return {
        content: [
          {
            type: "text",
            text: JSON.stringify(report, null, 2)
          }
        ],
        structuredContent: report
      };
    }
  );

  server.registerTool(
    "doctor",
    {
      title: "Doctor",
      description: "Run local prerequisite checks for the Roblox all-in-one MCP environment.",
      inputSchema: {
        includeManifest: z.boolean().default(true).describe("Include manifest inspection in the report.")
      }
    },
    async ({ includeManifest }) => {
      const report = await buildCapabilityReport();
      const summary = {
        ok: Boolean(report.roblox.available || report.luau.available || report.unity.available || report.unreal.available || report.blender.available),
        manifestChecked: includeManifest,
        robloxAvailable: report.roblox.available,
        luauAvailable: report.luau.available,
        unityAvailable: report.unity.available,
        unrealAvailable: report.unreal.available,
        blenderAvailable: report.blender.available,
        missing: [
          ...(report.roblox.available ? [] : report.roblox.reasons),
          ...(report.luau.available ? [] : report.luau.reasons),
          ...(report.unity.available ? [] : report.unity.reasons),
          ...(report.unreal.available ? [] : report.unreal.reasons),
          ...(report.blender.available ? [] : report.blender.reasons)
        ]
      };
      const result = {
        summary,
        report: includeManifest
          ? report
          : {
              ...report,
              manifest: {
                path: report.manifest.path,
                exists: report.manifest.exists,
                valid: report.manifest.valid,
                parseError: report.manifest.parseError,
                manifest: null
              }
            }
      };

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
