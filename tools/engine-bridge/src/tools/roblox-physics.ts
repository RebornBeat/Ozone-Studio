import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { executeRobloxCommand } from "../connectors/roblox/index.js";

function toolResult(result: Record<string, unknown>) {
  return {
    content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
    structuredContent: result
  };
}

export function registerRobloxPhysicsTools(server: McpServer) {
  server.registerTool(
    "roblox_set_gravity",
    {
      title: "Roblox Set Gravity",
      description: "Set the Workspace gravity vector in Roblox Studio. Default is (0, -196.2, 0).",
      inputSchema: {
        x: z.number().default(0).describe("Gravity X component."),
        y: z.number().default(-196.2).describe("Gravity Y component."),
        z: z.number().default(0).describe("Gravity Z component."),
        waitForResponseMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ x, y, z: gz, waitForResponseMs }) => {
      const result = await executeRobloxCommand("set_gravity", { x, y, z: gz }, { waitForResponseMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "roblox_set_physics",
    {
      title: "Roblox Set Physics",
      description: "Enable or disable physics on a BasePart by setting Anchored and optionally applying velocity.",
      inputSchema: {
        path: z.string().min(1).describe("Full instance path of the BasePart."),
        anchored: z.boolean().describe("True to anchor (disable physics), false to unanchor (enable physics)."),
        velocity: z.object({ x: z.number(), y: z.number(), z: z.number() }).optional().describe("Initial AssemblyLinearVelocity when unanchoring."),
        angularVelocity: z.object({ x: z.number(), y: z.number(), z: z.number() }).optional().describe("Initial AssemblyAngularVelocity."),
        waitForResponseMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ path, anchored, velocity, angularVelocity, waitForResponseMs }) => {
      const result = await executeRobloxCommand("set_physics", { path, anchored, velocity, angularVelocity }, { waitForResponseMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "roblox_add_constraint",
    {
      title: "Roblox Add Constraint",
      description: "Add a physics constraint (SpringConstraint, HingeConstraint, RopeConstraint, WeldConstraint, etc.) between two attachments.",
      inputSchema: {
        constraintType: z.enum([
          "SpringConstraint", "HingeConstraint", "RopeConstraint", "RodConstraint",
          "WeldConstraint", "BallSocketConstraint", "PrismaticConstraint", "CylindricalConstraint",
          "AlignPosition", "AlignOrientation", "VectorForce", "LinearVelocity", "AngularVelocity"
        ]).describe("Type of constraint to create."),
        parentPath: z.string().min(1).describe("Instance path where the constraint will be parented."),
        attachment0Path: z.string().min(1).describe("Path to Attachment0."),
        attachment1Path: z.string().default("").describe("Path to Attachment1 (empty for single-attachment constraints)."),
        properties: z.record(z.unknown()).default({}).describe("Additional properties to set on the constraint (e.g. {Stiffness: 100, Damping: 10})."),
        waitForResponseMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ constraintType, parentPath, attachment0Path, attachment1Path, properties, waitForResponseMs }) => {
      const result = await executeRobloxCommand("add_constraint", {
        constraintType, parentPath, attachment0Path, attachment1Path, properties
      }, { waitForResponseMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "roblox_raycast",
    {
      title: "Roblox Raycast",
      description: "Cast a ray in Roblox 3D space and return hit information.",
      inputSchema: {
        origin: z.object({ x: z.number(), y: z.number(), z: z.number() }).describe("Ray origin point."),
        direction: z.object({ x: z.number(), y: z.number(), z: z.number() }).describe("Ray direction vector."),
        maxDistance: z.number().min(0).max(10000).default(1000).describe("Maximum ray distance."),
        filterType: z.enum(["Exclude", "Include"]).default("Exclude").describe("Raycast filter type."),
        filterPaths: z.array(z.string()).default([]).describe("Instance paths to include/exclude from raycast."),
        waitForResponseMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ origin, direction, maxDistance, filterType, filterPaths, waitForResponseMs }) => {
      const result = await executeRobloxCommand("raycast", {
        origin, direction, maxDistance, filterType, filterPaths
      }, { waitForResponseMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "roblox_simulate_physics",
    {
      title: "Roblox Simulate Physics",
      description: "Apply an impulse or force to a BasePart to simulate physics interaction.",
      inputSchema: {
        path: z.string().min(1).describe("Full instance path of the BasePart."),
        impulse: z.object({ x: z.number(), y: z.number(), z: z.number() }).optional().describe("Impulse vector to apply (instantaneous)."),
        force: z.object({ x: z.number(), y: z.number(), z: z.number() }).optional().describe("Continuous force vector (via VectorForce)."),
        torque: z.object({ x: z.number(), y: z.number(), z: z.number() }).optional().describe("Torque vector to apply."),
        atPosition: z.object({ x: z.number(), y: z.number(), z: z.number() }).optional().describe("World position to apply impulse at."),
        waitForResponseMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ path, impulse, force, torque, atPosition, waitForResponseMs }) => {
      const result = await executeRobloxCommand("simulate_physics", {
        path, impulse, force, torque, atPosition
      }, { waitForResponseMs });
      return toolResult(result);
    }
  );
}
