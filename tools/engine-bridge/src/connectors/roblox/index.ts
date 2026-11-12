import { commandExists, envFlag } from "../../validation/environment.js";
import { detectLuauRuntime } from "../luau/index.js";
import {
  dispatchLuauCreateWorkspacePart,
  dispatchLuauRunCode,
  dispatchLuauCommand,
  type LuauCommandKind
} from "../luau/bridge.js";

export type RobloxConnectorStatus = {
  available: boolean;
  reasons: string[];
  detected: {
    command: boolean;
    token: boolean;
  };
};

export async function detectRobloxConnector(): Promise<RobloxConnectorStatus> {
  const token = envFlag("ROBLOX_API_KEY") || envFlag("ROBLOX_MCP_URL") || envFlag("ROBLOX_PLACE_ID");
  const command = commandExists("roblox") || commandExists("rojo");
  const reasons: string[] = [];

  if (!command) {
    reasons.push("No local Roblox-related command detected (expected roblox or rojo). ");
  }

  if (!token) {
    reasons.push("No Roblox environment hint detected (ROBLOX_API_KEY, ROBLOX_MCP_URL, or ROBLOX_PLACE_ID).");
  }

  return {
    available: command || token,
    reasons,
    detected: {
      command,
      token
    }
  };
}

export async function runRobloxCodeWorkflow(
  code: string,
  options: {
    waitForResponseMs?: number;
    mode?: "edit" | "playtest";
    cwd?: string;
  } = {}
) {
  const luau = await detectLuauRuntime(options.cwd);
  if (!luau.health.alive) {
    return {
      ok: false,
      reason: "Luau companion runtime is not healthy enough to execute Roblox code.",
      luau
    };
  }

  const dispatch = await dispatchLuauRunCode(
    code,
    options.mode ?? "edit",
    options.waitForResponseMs ?? 2_000
  );

  return {
    ok: dispatch.status === "ok" || dispatch.status === "queued",
    luau,
    dispatch
  };
}

export async function createRobloxWorkspacePartWorkflow(
  input: {
    name: string;
    anchored?: boolean;
    position?: { x: number; y: number; z: number };
    size?: { x: number; y: number; z: number };
  },
  options: {
    waitForResponseMs?: number;
    cwd?: string;
  } = {}
) {
  const luau = await detectLuauRuntime(options.cwd);
  if (!luau.health.alive) {
    return {
      ok: false,
      reason: "Luau companion runtime is not healthy enough to create a Roblox part.",
      luau
    };
  }

  const dispatch = await dispatchLuauCreateWorkspacePart(
    {
      name: input.name,
      anchored: input.anchored ?? true,
      position: input.position ?? { x: 0, y: 5, z: 0 },
      size: input.size ?? { x: 4, y: 1, z: 2 }
    },
    options.waitForResponseMs ?? 2_000
  );

  return {
    ok: dispatch.status === "ok" || dispatch.status === "queued",
    luau,
    dispatch
  };
}

export async function executeRobloxCommand(
  kind: LuauCommandKind,
  payload: Record<string, unknown>,
  options: { waitForResponseMs?: number; cwd?: string } = {}
) {
  const luau = await detectLuauRuntime(options.cwd);
  if (!luau.health.alive) {
    return {
      ok: false,
      reason: `Luau companion runtime is not healthy enough to execute ${kind}.`,
      luau
    };
  }

  const dispatch = await dispatchLuauCommand(
    kind,
    payload,
    options.waitForResponseMs ?? 2_000
  );

  return {
    ok: dispatch.status === "ok" || dispatch.status === "queued",
    luau,
    dispatch
  };
}
