import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { executeUnityCommand } from "../connectors/unity/index.js";

function toolResult(result: Record<string, unknown>) {
  return {
    content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
    structuredContent: result
  };
}

export function registerUnityPhysicsTools(server: McpServer) {
  server.registerTool(
    "unity_set_gravity",
    {
      title: "Unity Set Gravity",
      description: "Set the global Physics gravity vector in Unity. Default is (0, -9.81, 0).",
      inputSchema: {
        x: z.number().default(0).describe("Gravity X."),
        y: z.number().default(-9.81).describe("Gravity Y."),
        z: z.number().default(0).describe("Gravity Z."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ x, y, z: gz, timeoutMs }) => {
      const result = await executeUnityCommand("set_gravity", { x, y, z: gz }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "unity_add_rigidbody",
    {
      title: "Unity Add Rigidbody",
      description: "Add a Rigidbody component to a GameObject with mass, drag, constraints, and collision detection settings.",
      inputSchema: {
        path: z.string().min(1).describe("GameObject path."),
        mass: z.number().min(0).default(1).describe("Rigidbody mass."),
        drag: z.number().min(0).default(0).describe("Linear drag."),
        angularDrag: z.number().min(0).default(0.05).describe("Angular drag."),
        useGravity: z.boolean().default(true).describe("Whether gravity affects this body."),
        isKinematic: z.boolean().default(false).describe("Kinematic mode (not affected by forces)."),
        freezePositionX: z.boolean().default(false),
        freezePositionY: z.boolean().default(false),
        freezePositionZ: z.boolean().default(false),
        freezeRotationX: z.boolean().default(false),
        freezeRotationY: z.boolean().default(false),
        freezeRotationZ: z.boolean().default(false),
        collisionDetection: z.enum(["Discrete", "Continuous", "ContinuousDynamic", "ContinuousSpeculative"]).default("Discrete").describe("Collision detection mode."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async (params) => {
      const result = await executeUnityCommand("add_rigidbody", params, { timeoutMs: params.timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "unity_add_joint",
    {
      title: "Unity Add Joint",
      description: "Add a physics joint component (FixedJoint, HingeJoint, SpringJoint, CharacterJoint, ConfigurableJoint) to a GameObject.",
      inputSchema: {
        path: z.string().min(1).describe("GameObject path."),
        jointType: z.enum(["FixedJoint", "HingeJoint", "SpringJoint", "CharacterJoint", "ConfigurableJoint"]).describe("Joint type."),
        connectedBodyPath: z.string().default("").describe("Path to the connected Rigidbody GameObject. Empty for world anchor."),
        breakForce: z.number().default(Infinity).describe("Force threshold to break the joint."),
        breakTorque: z.number().default(Infinity).describe("Torque threshold to break the joint."),
        properties: z.record(z.unknown()).default({}).describe("Additional joint-specific properties."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ path, jointType, connectedBodyPath, breakForce, breakTorque, properties, timeoutMs }) => {
      const result = await executeUnityCommand("add_joint", {
        path, jointType, connectedBodyPath, breakForce, breakTorque, properties
      }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "unity_raycast",
    {
      title: "Unity Raycast",
      description: "Cast a ray in Unity 3D space using Physics.Raycast and return hit info.",
      inputSchema: {
        origin: z.object({ x: z.number(), y: z.number(), z: z.number() }).describe("Ray origin."),
        direction: z.object({ x: z.number(), y: z.number(), z: z.number() }).describe("Ray direction."),
        maxDistance: z.number().min(0).default(1000).describe("Maximum ray distance."),
        layerMask: z.number().int().default(-1).describe("Layer mask (-1 for all layers)."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ origin, direction, maxDistance, layerMask, timeoutMs }) => {
      const result = await executeUnityCommand("raycast", { origin, direction, maxDistance, layerMask }, { timeoutMs });
      return toolResult(result);
    }
  );

  server.registerTool(
    "unity_apply_force",
    {
      title: "Unity Apply Force",
      description: "Apply a force, impulse, or torque to a GameObject's Rigidbody.",
      inputSchema: {
        path: z.string().min(1).describe("GameObject path with Rigidbody."),
        force: z.object({ x: z.number(), y: z.number(), z: z.number() }).optional().describe("Force vector."),
        torque: z.object({ x: z.number(), y: z.number(), z: z.number() }).optional().describe("Torque vector."),
        forceMode: z.enum(["Force", "Impulse", "VelocityChange", "Acceleration"]).default("Force").describe("Force application mode."),
        atPosition: z.object({ x: z.number(), y: z.number(), z: z.number() }).optional().describe("World position to apply force at."),
        timeoutMs: z.number().int().min(0).max(30_000).default(5_000).describe("Command timeout.")
      }
    },
    async ({ path, force, torque, forceMode, atPosition, timeoutMs }) => {
      const result = await executeUnityCommand("apply_force", { path, force, torque, forceMode, atPosition }, { timeoutMs });
      return toolResult(result);
    }
  );
}
