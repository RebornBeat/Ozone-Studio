import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { executeBlenderCommand } from "../connectors/blender/index.js";

function toolResult(result: Record<string, unknown>) {
  return {
    content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
    structuredContent: result
  };
}

export function registerBlenderPhysicsTools(server: McpServer) {
  server.registerTool(
    "blender_set_gravity",
    {
      title: "Blender Set Gravity",
      description: "Set the scene gravity vector in Blender. Default is (0, 0, -9.81).",
      inputSchema: {
        x: z.number().default(0).describe("Gravity X."),
        y: z.number().default(0).describe("Gravity Y."),
        z: z.number().default(-9.81).describe("Gravity Z."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ x, y, z: gz, timeoutMs }) => {
      const result = await executeBlenderCommand("set_gravity", { x, y, z: gz }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "blender_setup_rigid_body",
    {
      title: "Blender Setup Rigid Body",
      description: "Add rigid body physics to a Blender object with type, shape, mass, friction, and damping settings. Mirrors poly-mcp/Blender-MCP-Server pattern.",
      inputSchema: {
        objectName: z.string().min(1).describe("Object name in the scene."),
        bodyType: z.enum(["ACTIVE", "PASSIVE"]).default("ACTIVE").describe("ACTIVE (affected by forces) or PASSIVE (static collider)."),
        collisionShape: z.enum(["BOX", "SPHERE", "CAPSULE", "CYLINDER", "CONE", "CONVEX_HULL", "MESH", "COMPOUND"]).default("CONVEX_HULL").describe("Collision shape type."),
        mass: z.number().min(0).default(1).describe("Object mass in kg."),
        friction: z.number().min(0).max(1).default(0.5).describe("Surface friction (0-1)."),
        restitution: z.number().min(0).max(1).default(0).describe("Bounciness (0-1)."),
        linearDamping: z.number().min(0).max(1).default(0.04).describe("Linear damping (0-1)."),
        angularDamping: z.number().min(0).max(1).default(0.1).describe("Angular damping (0-1)."),
        useMargin: z.boolean().default(false).describe("Use collision margin."),
        collisionMargin: z.number().min(0).default(0.04).describe("Collision margin distance."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async (params) => {
      const result = await executeBlenderCommand("setup_rigid_body", params, { timeoutMs: params.timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "blender_add_constraint",
    {
      title: "Blender Add Rigid Body Constraint",
      description: "Add a rigid body constraint between two objects in Blender (Fixed, Point, Hinge, Slider, Piston, Generic, Motor).",
      inputSchema: {
        constraintType: z.enum(["FIXED", "POINT", "HINGE", "SLIDER", "PISTON", "GENERIC", "GENERIC_SPRING", "MOTOR"]).default("FIXED").describe("Constraint type."),
        object1Name: z.string().min(1).describe("First object name."),
        object2Name: z.string().min(1).describe("Second object name."),
        breakable: z.boolean().default(false).describe("Whether the constraint can break."),
        breakThreshold: z.number().default(10).describe("Force threshold to break."),
        disableCollisions: z.boolean().default(true).describe("Disable collisions between constrained objects."),
        properties: z.record(z.unknown()).default({}).describe("Additional constraint-specific properties."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ constraintType, object1Name, object2Name, breakable, breakThreshold, disableCollisions, properties, timeoutMs }) => {
      const result = await executeBlenderCommand("add_rigid_body_constraint", {
        constraintType, object1Name, object2Name, breakable, breakThreshold, disableCollisions, properties
      }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "blender_bake_physics",
    {
      title: "Blender Bake Physics",
      description: "Bake rigid body physics simulation to keyframes for the specified frame range.",
      inputSchema: {
        frameStart: z.number().int().min(0).default(1).describe("Start frame."),
        frameEnd: z.number().int().min(1).default(250).describe("End frame."),
        selectedOnly: z.boolean().default(false).describe("Bake only selected objects."),
        timeoutMs: z.number().int().min(0).max(60_000).default(30_000).describe("Command timeout (baking can be slow).")
      }
    },
    async ({ frameStart, frameEnd, selectedOnly, timeoutMs }) => {
      const result = await executeBlenderCommand("bake_physics", { frameStart, frameEnd, selectedOnly }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "blender_apply_force",
    {
      title: "Blender Apply Force",
      description: "Apply an initial velocity or force field to a rigid body object in Blender by setting keyframe velocities or using force fields.",
      inputSchema: {
        objectName: z.string().min(1).describe("Object name with rigid body."),
        linearVelocity: z.object({ x: z.number(), y: z.number(), z: z.number() }).optional().describe("Initial linear velocity."),
        angularVelocity: z.object({ x: z.number(), y: z.number(), z: z.number() }).optional().describe("Initial angular velocity."),
        forceFieldType: z.enum(["FORCE", "WIND", "VORTEX", "TURBULENCE", "DRAG", "HARMONIC"]).optional().describe("Add a force field instead of direct velocity."),
        forceStrength: z.number().optional().describe("Force field strength."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ objectName, linearVelocity, angularVelocity, forceFieldType, forceStrength, timeoutMs }) => {
      const result = await executeBlenderCommand("apply_force", {
        objectName, linearVelocity, angularVelocity, forceFieldType, forceStrength
      }, { timeoutMs });
      return toolResult(result);
    }
  );
}
