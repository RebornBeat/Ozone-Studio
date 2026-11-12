import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, dirname, isAbsolute, join, resolve } from "node:path";
import { getLuauRuntimeBridgeConnectionInfo } from "../connectors/luau/bridge.js";

export type RuntimeTarget = "roblox-studio-mcp";

export type ProjectManifest = {
  version: 1;
  projectName: string;
  projectRoot: string;
  runtimeTarget: RuntimeTarget;
  connectors: {
    roblox: { enabled: true };
    luau: { enabled: boolean };
    blender: { enabled: boolean };
  };
  runtime: {
    luauPluginScaffoldPath: string;
    luauBridgeUrl: string;
  };
  assetMappings: Record<string, never>;
  syncState: {
    initializedAt: string;
  };
  templates: Record<string, never>;
  createdAt: string;
  updatedAt: string;
};

export type ManifestInspection = {
  path: string;
  exists: boolean;
  valid: boolean;
  parseError: string | null;
  manifest: ProjectManifest | null;
};

export type CreateProjectManifestOptions = {
  projectName?: string;
  enableBlender?: boolean;
  enableLuau?: boolean;
  runtimeTarget?: RuntimeTarget;
  force?: boolean;
};

export function getProjectManifestPath(cwd = process.cwd()) {
  return join(cwd, ".roblox-mcp", "project.json");
}

export function getLuauPluginScaffoldPath(cwd = process.cwd()) {
  return join(cwd, "runtime", "roblox-studio-plugin", "src");
}

export function validateProjectManifest(manifest: unknown): manifest is ProjectManifest {
  if (!manifest || typeof manifest !== "object") {
    return false;
  }

  const candidate = manifest as Partial<ProjectManifest>;
  return candidate.version === 1
    && typeof candidate.projectName === "string"
    && typeof candidate.projectRoot === "string"
    && isAbsolute(candidate.projectRoot)
    && candidate.runtimeTarget === "roblox-studio-mcp";
}

export function loadProjectManifest(cwd = process.cwd()): ProjectManifest | null {
  const path = getProjectManifestPath(cwd);
  if (!existsSync(path)) {
    return null;
  }

  const content = readFileSync(path, "utf8");
  const parsed = JSON.parse(content);

  if (!validateProjectManifest(parsed)) {
    throw new Error(`Invalid project manifest at ${path}`);
  }

  return parsed;
}

export function inspectProjectManifest(cwd = process.cwd()): ManifestInspection {
  const path = getProjectManifestPath(cwd);

  if (!existsSync(path)) {
    return {
      path,
      exists: false,
      valid: false,
      parseError: null,
      manifest: null
    };
  }

  try {
    const manifest = loadProjectManifest(cwd);

    return {
      path,
      exists: true,
      valid: manifest !== null,
      parseError: null,
      manifest
    };
  } catch (error) {
    return {
      path,
      exists: true,
      valid: false,
      parseError: error instanceof Error ? error.message : String(error),
      manifest: null
    };
  }
}

export function createProjectManifest(
  cwd = process.cwd(),
  options: CreateProjectManifestOptions = {}
) {
  const path = getProjectManifestPath(cwd);
  const existing = inspectProjectManifest(cwd);

  if (existing.exists && existing.valid && !options.force) {
    return {
      created: false,
      path,
      manifest: existing.manifest
    };
  }

  const now = new Date().toISOString();
  const projectRoot = resolve(cwd);
  const manifest: ProjectManifest = {
    version: 1,
    projectName: options.projectName?.trim() || basename(projectRoot),
    projectRoot,
    runtimeTarget: options.runtimeTarget ?? "roblox-studio-mcp",
    connectors: {
      roblox: { enabled: true },
      luau: { enabled: options.enableLuau ?? true },
      blender: { enabled: options.enableBlender ?? true }
    },
    runtime: {
      luauPluginScaffoldPath: getLuauPluginScaffoldPath(projectRoot),
      luauBridgeUrl: getLuauRuntimeBridgeConnectionInfo().url
    },
    assetMappings: {},
    syncState: {
      initializedAt: now
    },
    templates: {},
    createdAt: now,
    updatedAt: now
  };

  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");

  return {
    created: true,
    path,
    manifest
  };
}
