import { commandExists, envFlag } from "../../validation/environment.js";
import { TcpBridge, type TcpCommandResponse } from "../shared/tcp-bridge.js";

export type UnrealConnectorStatus = {
  available: boolean;
  reasons: string[];
  detected: {
    command: boolean;
    bridge: boolean;
  };
  bridgeConnected: boolean;
};

const DEFAULT_PORT = 55557;

let bridge: TcpBridge | null = null;

export function getUnrealBridge(): TcpBridge | null {
  return bridge;
}

export function startUnrealBridge(): void {
  const port = Number(process.env["UNREAL_BRIDGE_PORT"]) || DEFAULT_PORT;
  bridge = new TcpBridge({
    host: "127.0.0.1",
    port,
    name: "unreal",
    reconnectIntervalMs: 5_000,
    commandTimeoutMs: 10_000
  });
  bridge.start();
}

export function stopUnrealBridge(): void {
  if (bridge) {
    bridge.stop();
    bridge = null;
  }
}

export async function detectUnrealConnector(): Promise<UnrealConnectorStatus> {
  const command = commandExists("UnrealEditor") || commandExists("UE4Editor") || commandExists("UE5Editor");
  const envHint = envFlag("UNREAL_BRIDGE_PORT") || envFlag("UNREAL_PROJECT_PATH");
  const reasons: string[] = [];

  if (!command) {
    reasons.push("No local Unreal Editor executable detected.");
  }

  if (!envHint) {
    reasons.push("No Unreal environment hint detected (UNREAL_BRIDGE_PORT or UNREAL_PROJECT_PATH).");
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

export async function executeUnrealCommand(
  method: string,
  params: Record<string, unknown>,
  options: { timeoutMs?: number } = {}
): Promise<Record<string, unknown>> {
  if (!bridge || !bridge.isConnected) {
    return {
      ok: false,
      reason: "Unreal bridge not connected. Ensure the Unreal C++ MCP plugin is running.",
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
