import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { executeUnrealCommand } from "../connectors/unreal/index.js";

function toolResult(result: Record<string, unknown>) {
  return {
    content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
    structuredContent: result
  };
}

export function registerUnrealPhysicsTools(server: McpServer) {
  server.registerTool(
    "unreal_set_gravity",
    {
      title: "Unreal Set Gravity",
      description: "Set the world gravity override in Unreal Engine. Default is (0, 0, -980).",
      inputSchema: {
        x: z.number().default(0).describe("Gravity X."),
        y: z.number().default(0).describe("Gravity Y."),
        z: z.number().default(-980).describe("Gravity Z."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ x, y, z: gz, timeoutMs }) => {
      const result = await executeUnrealCommand("set_gravity", { x, y, z: gz }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "unreal_set_simulate_physics",
    {
      title: "Unreal Set Simulate Physics",
      description: "Enable or disable physics simulation on an actor's primitive component.",
      inputSchema: {
        path: z.string().min(1).describe("Actor path."),
        component: z.string().default("").describe("Component name. Empty for root component."),
        simulate: z.boolean().describe("True to enable physics simulation, false to disable."),
        mass: z.number().min(0).optional().describe("Override mass in kg."),
        linearDamping: z.number().min(0).optional().describe("Linear damping."),
        angularDamping: z.number().min(0).optional().describe("Angular damping."),
        enableGravity: z.boolean().optional().describe("Enable gravity on this component."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ path, component, simulate, mass, linearDamping, angularDamping, enableGravity, timeoutMs }) => {
      const result = await executeUnrealCommand("set_simulate_physics", {
        path, component, simulate, mass, linearDamping, angularDamping, enableGravity
      }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "unreal_add_physics_constraint",
    {
      title: "Unreal Add Physics Constraint",
      description: "Add a UPhysicsConstraintComponent between two actors for joints, hinges, or locks.",
      inputSchema: {
        ownerPath: z.string().min(1).describe("Actor path to own the constraint."),
        targetPath: z.string().default("").describe("Target actor path. Empty for world constraint."),
        constraintProfile: z.enum(["Free", "Limited", "Locked", "Hinge", "Prismatic", "BallSocket"]).default("Free").describe("Constraint profile preset."),
        breakable: z.boolean().default(false).describe("Whether the constraint can break."),
        breakThreshold: z.number().default(0).describe("Force threshold to break (0 = unbreakable)."),
        properties: z.record(z.unknown()).default({}).describe("Additional constraint properties."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ ownerPath, targetPath, constraintProfile, breakable, breakThreshold, properties, timeoutMs }) => {
      const result = await executeUnrealCommand("add_physics_constraint", {
        ownerPath, targetPath, constraintProfile, breakable, breakThreshold, properties
      }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "unreal_raycast",
    {
      title: "Unreal Raycast",
      description: "Perform a line trace (raycast) in Unreal and return hit results.",
      inputSchema: {
        start: z.object({ x: z.number(), y: z.number(), z: z.number() }).describe("Trace start point."),
        end: z.object({ x: z.number(), y: z.number(), z: z.number() }).describe("Trace end point."),
        traceChannel: z.enum(["Visibility", "Camera", "PhysicsBody"]).default("Visibility").describe("Trace channel."),
        traceComplex: z.boolean().default(false).describe("Trace against complex collision."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ start, end, traceChannel, traceComplex, timeoutMs }) => {
      const result = await executeUnrealCommand("raycast", { start, end, traceChannel, traceComplex }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "unreal_apply_force",
    {
      title: "Unreal Apply Force",
      description: "Apply a force or impulse to an actor's physics-enabled component.",
      inputSchema: {
        path: z.string().min(1).describe("Actor path."),
        component: z.string().default("").describe("Component name. Empty for root."),
        force: z.object({ x: z.number(), y: z.number(), z: z.number() }).optional().describe("Force vector (continuous)."),
        impulse: z.object({ x: z.number(), y: z.number(), z: z.number() }).optional().describe("Impulse vector (instantaneous)."),
        torque: z.object({ x: z.number(), y: z.number(), z: z.number() }).optional().describe("Torque vector."),
        atLocation: z.object({ x: z.number(), y: z.number(), z: z.number() }).optional().describe("World location to apply force at."),
        velocityChange: z.boolean().default(false).describe("Apply as velocity change (ignores mass)."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ path, component, force, impulse, torque, atLocation, velocityChange, timeoutMs }) => {
      const result = await executeUnrealCommand("apply_force", {
        path, component, force, impulse, torque, atLocation, velocityChange
      }, { timeoutMs });
      return toolResult(result);
    }
  );
}
