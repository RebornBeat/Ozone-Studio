import { commandExists, envFlag } from "../../validation/environment.js";
import { TcpBridge, type TcpCommandResponse } from "../shared/tcp-bridge.js";

export type BlenderConnectorStatus = {
  available: boolean;
  reasons: string[];
  detected: {
    command: boolean;
    bridge: boolean;
  };
  bridgeConnected: boolean;
};

const DEFAULT_PORT = 9876;

let bridge: TcpBridge | null = null;

export function getBlenderBridge(): TcpBridge | null {
  return bridge;
}

export function startBlenderBridge(): void {
  const port = Number(process.env["BLENDER_BRIDGE_PORT"]) || DEFAULT_PORT;
  bridge = new TcpBridge({
    host: "127.0.0.1",
    port,
    name: "blender",
    reconnectIntervalMs: 5_000,
    commandTimeoutMs: 10_000
  });
  bridge.start();
}

export function stopBlenderBridge(): void {
  if (bridge) {
    bridge.stop();
    bridge = null;
  }
}

export async function detectBlenderConnector(): Promise<BlenderConnectorStatus> {
  const command = commandExists("blender");
  const envHint = envFlag("BLENDER_MCP_URL") || envFlag("BLENDER_EXECUTABLE") || envFlag("BLENDER_BRIDGE_PORT");
  const reasons: string[] = [];

  if (!command) {
    reasons.push("No local Blender executable detected.");
  }

  if (!envHint) {
    reasons.push("No Blender bridge hint detected (BLENDER_MCP_URL, BLENDER_EXECUTABLE, or BLENDER_BRIDGE_PORT).");
  }

  return {
    available: command || envHint || (bridge?.isConnected ?? false),
    reasons,
    detected: {
      command,
      bridge: bridge?.isConnected ?? false
    },
    bridgeConnected: bridge?.isConnected ?? false
  };
}

export async function executeBlenderCommand(
  method: string,
  params: Record<string, unknown>,
  options: { timeoutMs?: number } = {}
): Promise<Record<string, unknown>> {
  if (!bridge || !bridge.isConnected) {
    return {
      ok: false,
      reason: "Blender bridge not connected. Ensure the Blender MCP addon is running.",
      bridgeStatus: bridge?.status ?? null
    };
  }

  const resp: TcpCommandResponse = await bridge.dispatch(method, params, options.timeoutMs);

  return {
    ok: resp.status === "ok",
    method,
    ...resp
  };
}
