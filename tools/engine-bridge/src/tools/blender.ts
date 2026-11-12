import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { executeBlenderCommand } from "../connectors/blender/index.js";

function toolResult(result: Record<string, unknown>) {
  return {
    content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
    structuredContent: result
  };
}

export function registerBlenderTools(server: McpServer) {
  server.registerTool(
    "blender_get_scene",
    {
      title: "Blender Get Scene",
      description: "Get the object hierarchy and metadata of the active Blender scene.",
      inputSchema: {
        includeTransforms: z.boolean().default(true).describe("Include object transforms."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ includeTransforms, timeoutMs }) => {
      const result = await executeBlenderCommand("get_scene", { includeTransforms }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "blender_get_object",
    {
      title: "Blender Get Object",
      description: "Get detailed properties of a specific Blender object by name.",
      inputSchema: {
        name: z.string().min(1).describe("Object name in the scene."),
        includeModifiers: z.boolean().default(true).describe("Include modifier stack."),
        includeMaterial: z.boolean().default(true).describe("Include material slots."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ name, includeModifiers, includeMaterial, timeoutMs }) => {
      const result = await executeBlenderCommand("get_object", { name, includeModifiers, includeMaterial }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "blender_create_object",
    {
      title: "Blender Create Object",
      description: "Create a new mesh primitive or empty object in the Blender scene.",
      inputSchema: {
        type: z.enum(["cube", "sphere", "cylinder", "plane", "cone", "torus", "empty", "camera", "light"]).describe("Object type to create."),
        name: z.string().default("").describe("Optional object name."),
        location: z.object({ x: z.number(), y: z.number(), z: z.number() }).default({ x: 0, y: 0, z: 0 }).describe("World location."),
        scale: z.object({ x: z.number(), y: z.number(), z: z.number() }).default({ x: 1, y: 1, z: 1 }).describe("Scale."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ type, name, location, scale, timeoutMs }) => {
      const result = await executeBlenderCommand("create_object", { type, name, location, scale }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "blender_delete_object",
    {
      title: "Blender Delete Object",
      description: "Delete an object from the Blender scene by name.",
      inputSchema: {
        name: z.string().min(1).describe("Object name to delete."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ name, timeoutMs }) => {
      const result = await executeBlenderCommand("delete_object", { name }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "blender_set_transform",
    {
      title: "Blender Set Transform",
      description: "Set the location, rotation, and/or scale of a Blender object.",
      inputSchema: {
        name: z.string().min(1).describe("Object name."),
        location: z.object({ x: z.number(), y: z.number(), z: z.number() }).optional().describe("World location."),
        rotation: z.object({ x: z.number(), y: z.number(), z: z.number() }).optional().describe("Euler rotation in radians."),
        scale: z.object({ x: z.number(), y: z.number(), z: z.number() }).optional().describe("Scale."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ name, location, rotation, scale, timeoutMs }) => {
      const result = await executeBlenderCommand("set_transform", { name, location, rotation, scale }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "blender_set_material",
    {
      title: "Blender Set Material",
      description: "Assign or create a material on a Blender object with base color and metallic/roughness.",
      inputSchema: {
        objectName: z.string().min(1).describe("Target object name."),
        materialName: z.string().default("").describe("Material name. Empty to create a new one."),
        baseColor: z.object({ r: z.number().min(0).max(1), g: z.number().min(0).max(1), b: z.number().min(0).max(1) }).optional().describe("Base color RGB (0-1)."),
        metallic: z.number().min(0).max(1).optional().describe("Metallic value (0-1)."),
        roughness: z.number().min(0).max(1).optional().describe("Roughness value (0-1)."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ objectName, materialName, baseColor, metallic, roughness, timeoutMs }) => {
      const result = await executeBlenderCommand("set_material", { objectName, materialName, baseColor, metallic, roughness }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "blender_run_python",
    {
      title: "Blender Run Python",
      description: "Execute arbitrary Python code in Blender's Python environment via bpy.",
      inputSchema: {
        code: z.string().min(1).describe("Python code to execute."),
        timeoutMs: z.number().int().min(0).max(30_000).default(10_000).describe("Command timeout.")
      }
    },
    async ({ code, timeoutMs }) => {
      const result = await executeBlenderCommand("run_python", { code }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "blender_export",
    {
      title: "Blender Export",
      description: "Export the scene or selected objects to a file format (FBX, OBJ, glTF, etc.).",
      inputSchema: {
        format: z.enum(["fbx", "obj", "gltf", "glb", "stl", "ply"]).describe("Export format."),
        outputPath: z.string().min(1).describe("Output file path."),
        selectedOnly: z.boolean().default(false).describe("Export only selected objects."),
        timeoutMs: z.number().int().min(0).max(30_000).default(10_000).describe("Command timeout.")
      }
    },
    async ({ format, outputPath, selectedOnly, timeoutMs }) => {
      const result = await executeBlenderCommand("export", { format, outputPath, selectedOnly }, { timeoutMs });
      return toolResult(result);
    }
  );
}
