import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { envFlag } from "../../validation/environment.js";
import {
  getLuauRuntimeBridgeConnectionInfo,
  getLuauRuntimeBridgeStatus,
  type LuauRuntimeHandshake
} from "./bridge.js";

export type LuauRuntimeHealth = {
  alive: boolean;
  stale: boolean;
  ageMs: number | null;
  checkedAt: string;
};

export type LuauRuntimeStatus = {
  available: boolean;
  reasons: string[];
  detected: {
    bridge: boolean;
    pluginPath: boolean;
    pluginVersion: boolean;
    scaffold: boolean;
    handshake: boolean;
  };
  handshake: LuauRuntimeHandshake | null;
  health: LuauRuntimeHealth;
  paths: {
    bridgeUrl: string;
    scaffoldPath: string;
  };
};

export async function detectLuauRuntime(cwd = process.cwd()): Promise<LuauRuntimeStatus> {
  const pluginPath = envFlag("ROBLOX_LUAU_PLUGIN_PATH");
  const scaffoldPath = resolve(cwd, "runtime", "roblox-studio-plugin", "src");
  const scaffold = existsSync(scaffoldPath);
  const bridgeInfo = getLuauRuntimeBridgeConnectionInfo();
  const bridgeStatus = getLuauRuntimeBridgeStatus();
  const reasons: string[] = [];

  if (!bridgeStatus.handshake) {
    reasons.push("No live Luau runtime handshake has been received from the Studio-side runtime.");
  }

  if (bridgeStatus.handshake && bridgeStatus.stale) {
    reasons.push("The Luau runtime handshake exists but is stale.");
  }

  if (!pluginPath) {
    reasons.push("No Luau companion plugin path detected (ROBLOX_LUAU_PLUGIN_PATH).");
  }

  if (scaffold && !bridgeStatus.handshake && !pluginPath) {
    reasons.push("A local Luau runtime scaffold exists in this repository, but no live Studio companion runtime is connected.");
  }

  return {
    available: bridgeStatus.connected || (pluginPath && !bridgeStatus.stale),
    reasons,
    detected: {
      bridge: true,
      pluginPath,
      pluginVersion: Boolean(bridgeStatus.handshake?.runtimeVersion),
      scaffold,
      handshake: Boolean(bridgeStatus.handshake)
    },
    handshake: bridgeStatus.handshake,
    health: {
      alive: bridgeStatus.connected,
      stale: bridgeStatus.stale,
      ageMs: bridgeStatus.ageMs,
      checkedAt: bridgeStatus.checkedAt
    },
    paths: {
      bridgeUrl: bridgeInfo.url,
      scaffoldPath
    }
  };
}
