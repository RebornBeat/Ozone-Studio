import { commandExists, envFlag } from "../../validation/environment.js";
import { TcpBridge, type TcpCommandResponse } from "../shared/tcp-bridge.js";

export type UnityConnectorStatus = {
  available: boolean;
  reasons: string[];
  detected: {
    command: boolean;
    bridge: boolean;
  };
  bridgeConnected: boolean;
};

const DEFAULT_PORT = 7890;

let bridge: TcpBridge | null = null;

export function getUnityBridge(): TcpBridge | null {
  return bridge;
}

export function startUnityBridge(): void {
  const port = Number(process.env["UNITY_BRIDGE_PORT"]) || DEFAULT_PORT;
  bridge = new TcpBridge({
    host: "127.0.0.1",
    port,
    name: "unity",
    reconnectIntervalMs: 5_000,
    commandTimeoutMs: 10_000
  });
  bridge.start();
}

export function stopUnityBridge(): void {
  if (bridge) {
    bridge.stop();
    bridge = null;
  }
}

export async function detectUnityConnector(): Promise<UnityConnectorStatus> {
  const command = commandExists("unity") || commandExists("Unity");
  const envHint = envFlag("UNITY_BRIDGE_PORT") || envFlag("UNITY_PROJECT_PATH");
  const reasons: string[] = [];

  if (!command) {
    reasons.push("No local Unity executable detected.");
  }

  if (!envHint) {
    reasons.push("No Unity environment hint detected (UNITY_BRIDGE_PORT or UNITY_PROJECT_PATH).");
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

export async function executeUnityCommand(
  method: string,
  params: Record<string, unknown>,
  options: { timeoutMs?: number } = {}
): Promise<Record<string, unknown>> {
  if (!bridge || !bridge.isConnected) {
    return {
      ok: false,
      reason: "Unity bridge not connected. Ensure the Unity Editor C# companion package is running.",
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
