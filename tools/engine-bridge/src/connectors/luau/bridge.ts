import { createServer, type IncomingMessage, type Server, type ServerResponse } from "node:http";
import { randomUUID } from "node:crypto";
import { z } from "zod";

export type LuauRuntimeHandshake = {
  protocolVersion: 1;
  runtimeName: string;
  runtimeVersion: string;
  bridgeMode: "http-long-poll";
  capabilities: string[];
  sessionId?: string;
  studioPlaceId?: string;
  lastSeenAt: string;
};

export type LuauCommandKind =
  | "run_code"
  | "create_workspace_part"
  | "get_script_source"
  | "set_script_source"
  | "edit_script_lines"
  | "grep_scripts"
  | "create_instance"
  | "delete_instance"
  | "set_property"
  | "clone_instance"
  | "reparent_instance"
  | "get_instance_properties"
  | "get_instance_children"
  | "search_instances"
  | "get_file_tree"
  | "set_gravity"
  | "set_physics"
  | "add_constraint"
  | "raycast"
  | "simulate_physics";

export type LuauRuntimeCommandRequest = {
  id: string;
  kind: LuauCommandKind;
  createdAt: string;
  payload: Record<string, unknown>;
};

export type LuauRuntimeCommandResponse = {
  requestId: string;
  status: "ok" | "error";
  completedAt: string;
  output?: string;
  data?: unknown;
  error?: string;
};

type PendingPoll = {
  response: ServerResponse<IncomingMessage>;
  timer: ReturnType<typeof setTimeout>;
};

const DEFAULT_HOST = "127.0.0.1";
const DEFAULT_PORT = 3002;
const HANDSHAKE_STALE_MS = 30_000;
const MAX_REQUEST_BODY_BYTES = 1_048_576; // 1 MiB
const RESPONSE_TTL_MS = 300_000; // 5 minutes
const RESPONSE_CLEANUP_INTERVAL_MS = 60_000;
const DEFAULT_COMMAND_POLL_TIMEOUT_MS = 25_000;
const MIN_COMMAND_POLL_TIMEOUT_MS = 1_000;
const MAX_COMMAND_POLL_TIMEOUT_MS = 60_000;

const ALLOWED_HOSTS = new Set(["127.0.0.1", "localhost", "[::1]"]);

const handshakeSchema = z.object({
  protocolVersion: z.literal(1),
  runtimeName: z.string().min(1),
  runtimeVersion: z.string().min(1),
  bridgeMode: z.literal("http-long-poll"),
  capabilities: z.array(z.string()),
  sessionId: z.string().min(1).optional(),
  studioPlaceId: z.string().min(1).optional(),
  lastSeenAt: z.string().min(1).optional()
}).strict();

const commandResponseSchema = z.object({
  requestId: z.string().uuid(),
  status: z.enum(["ok", "error"]),
  completedAt: z.string().min(1),
  output: z.string().optional(),
  data: z.unknown().optional(),
  error: z.string().optional()
}).strict();

class BridgeRequestError extends Error {
  constructor(
    readonly statusCode: number,
    message: string
  ) {
    super(message);
  }
}

function parseCommandPollTimeout(rawValue: string | null) {
  if (rawValue === null) {
    return DEFAULT_COMMAND_POLL_TIMEOUT_MS;
  }

  const parsed = Number(rawValue);
  if (!Number.isFinite(parsed)) {
    throw new BridgeRequestError(400, "Invalid timeoutMs query parameter");
  }

  return Math.min(
    Math.max(Math.trunc(parsed), MIN_COMMAND_POLL_TIMEOUT_MS),
    MAX_COMMAND_POLL_TIMEOUT_MS
  );
}

class LuauRuntimeBridge {
  private handshake: LuauRuntimeHandshake | null = null;
  private readonly queue: LuauRuntimeCommandRequest[] = [];
  private readonly responses = new Map<string, { response: LuauRuntimeCommandResponse; storedAt: number }>();
  private readonly pendingResults = new Map<string, (response: LuauRuntimeCommandResponse) => void>();
  private readonly pendingPolls: PendingPoll[] = [];
  private server: Server | null = null;
  private responseCleanupTimer: ReturnType<typeof setInterval> | null = null;
  private started = false;

  async start() {
    if (this.started) {
      return this.getConnectionInfo();
    }

    const port = Number(process.env.ROBLOX_LUAU_BRIDGE_PORT || DEFAULT_PORT);
    const server = createServer(async (request, response) => {
      try {
        const host = request.headers.host || "";
        if (!host) {
          throw new BridgeRequestError(400, "Missing host header");
        }

        const hostname = host.replace(/:\d+$/, "");
        if (!ALLOWED_HOSTS.has(hostname)) {
          throw new BridgeRequestError(400, "Invalid host header");
        }

        const url = new URL(request.url || "/", `http://${DEFAULT_HOST}:${port}`);

        if (request.method === "POST" && url.pathname === "/runtime/handshake") {
          const body = handshakeSchema.parse(await this.readJson(request, { requireBody: true }));
          this.handshake = {
            ...body,
            protocolVersion: 1,
            lastSeenAt: new Date().toISOString()
          };
          this.json(response, 200, { ok: true, handshake: this.handshake });
          return;
        }

        if (request.method === "GET" && url.pathname === "/runtime/health") {
          this.json(response, 200, this.getStatus());
          return;
        }

        if (request.method === "GET" && url.pathname === "/runtime/commands/next") {
          const timeoutMs = parseCommandPollTimeout(url.searchParams.get("timeoutMs"));
          const next = this.queue.shift();
          if (next) {
            this.json(response, 200, { ok: true, command: next });
            return;
          }

          const timer = setTimeout(() => {
            this.removePendingPoll(response);
            this.json(response, 200, { ok: true, command: null });
          }, timeoutMs);

          this.pendingPolls.push({ response, timer });
          request.on("close", () => this.removePendingPoll(response));
          return;
        }

        if (request.method === "POST" && url.pathname === "/runtime/commands/result") {
          const result = commandResponseSchema.parse(await this.readJson(request, { requireBody: true }));
          this.responses.set(result.requestId, { response: result, storedAt: Date.now() });
          const pending = this.pendingResults.get(result.requestId);
          if (pending) {
            this.pendingResults.delete(result.requestId);
            pending(result);
          }
          this.json(response, 200, { ok: true, requestId: result.requestId });
          return;
        }

        this.json(response, 404, { error: "Not found" });
      } catch (error) {
        if (error instanceof BridgeRequestError) {
          this.json(response, error.statusCode, { error: error.message });
          return;
        }

        if (error instanceof z.ZodError) {
          this.json(response, 400, { error: "Invalid request body" });
          return;
        }

        console.error("gamedev-all-in-one bridge request failed:", error);
        this.json(response, 500, { error: "Internal server error" });
      }
    });

    this.server = server;

    await new Promise<void>((resolvePromise) => {
      server.once("error", (err: NodeJS.ErrnoException) => {
        if (err.code === "EADDRINUSE") {
          console.error(`[luau-bridge] port ${port} in use, bridge disabled — Roblox tools will be unavailable`);
          this.server = null;
          resolvePromise();
        } else {
          console.error(`[luau-bridge] failed to start:`, err);
          this.server = null;
          resolvePromise();
        }
      });
      server.listen(port, DEFAULT_HOST, () => {
        this.started = true;
        this.startResponseCleanup();
        resolvePromise();
      });
    });

    return this.getConnectionInfo();
  }

  async stop() {
    const server = this.server;

    this.server = null;
    this.started = false;
    this.handshake = null;
    this.queue.length = 0;
    this.responses.clear();
    this.pendingResults.clear();

    if (this.responseCleanupTimer) {
      clearInterval(this.responseCleanupTimer);
      this.responseCleanupTimer = null;
    }

    for (const pendingPoll of this.pendingPolls.splice(0)) {
      clearTimeout(pendingPoll.timer);
      pendingPoll.response.destroy();
    }

    if (!server) {
      return;
    }

    await new Promise<void>((resolvePromise, rejectPromise) => {
      server.close((error) => {
        if (error) {
          rejectPromise(error);
          return;
        }

        resolvePromise();
      });
    });
  }

  getConnectionInfo() {
    const port = Number(process.env.ROBLOX_LUAU_BRIDGE_PORT || DEFAULT_PORT);
    return {
      host: DEFAULT_HOST,
      port,
      url: `http://${DEFAULT_HOST}:${port}`
    };
  }

  getStatus() {
    const checkedAt = new Date().toISOString();
    if (!this.handshake) {
      return {
        connected: false,
        checkedAt,
        stale: true,
        ageMs: null,
        handshake: null,
        queueLength: this.queue.length
      };
    }

    const ageMs = Date.now() - Date.parse(this.handshake.lastSeenAt);
    const stale = Number.isNaN(ageMs) || ageMs > HANDSHAKE_STALE_MS;
    return {
      connected: !stale,
      checkedAt,
      stale,
      ageMs: Number.isNaN(ageMs) ? null : ageMs,
      handshake: this.handshake,
      queueLength: this.queue.length
    };
  }

  async dispatch(
    request: Omit<LuauRuntimeCommandRequest, "id" | "createdAt">,
    waitForResponseMs = 2_000
  ) {
    const payload: LuauRuntimeCommandRequest = {
      id: randomUUID(),
      createdAt: new Date().toISOString(),
      ...request
    };

    const waitingPoll = this.pendingPolls.shift();
    if (waitingPoll) {
      clearTimeout(waitingPoll.timer);
      this.json(waitingPoll.response, 200, { ok: true, command: payload });
    } else {
      this.queue.push(payload);
    }

    if (waitForResponseMs <= 0) {
      return {
        status: "queued" as const,
        request: payload,
        response: null
      };
    }

    const response = await new Promise<LuauRuntimeCommandResponse | null>((resolvePromise) => {
      const timer = setTimeout(() => {
        this.pendingResults.delete(payload.id);
        resolvePromise(null);
      }, waitForResponseMs);

      this.pendingResults.set(payload.id, (result) => {
        clearTimeout(timer);
        resolvePromise(result);
      });
    });

    return {
      status: response?.status || "queued",
      request: payload,
      response
    };
  }

  private startResponseCleanup() {
    if (this.responseCleanupTimer) {
      return;
    }

    this.responseCleanupTimer = setInterval(() => {
      try {
        const now = Date.now();
        for (const [id, entry] of this.responses) {
          if (now - entry.storedAt > RESPONSE_TTL_MS) {
            this.responses.delete(id);
          }
        }
      } catch (err) {
        console.error("[luau-bridge] response cleanup error:", err);
      }
    }, RESPONSE_CLEANUP_INTERVAL_MS);
  }

  private async readJson(
    request: IncomingMessage,
    options: {
      requireBody?: boolean;
    } = {}
  ) {
    const chunks: Buffer[] = [];
    let totalBytes = 0;

    for await (const chunk of request) {
      const buf = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk);
      totalBytes += buf.byteLength;
      if (totalBytes > MAX_REQUEST_BODY_BYTES) {
        request.destroy();
        throw new Error(`Request body exceeds ${MAX_REQUEST_BODY_BYTES} bytes`);
      }
      chunks.push(buf);
    }

    const body = Buffer.concat(chunks).toString("utf8");
    if (!body) {
      if (options.requireBody) {
        throw new BridgeRequestError(400, "Empty request body");
      }

      return {};
    }

    try {
      return JSON.parse(body);
    } catch {
      throw new BridgeRequestError(400, "Invalid JSON in request body");
    }
  }

  private json(response: ServerResponse<IncomingMessage>, statusCode: number, payload: unknown) {
    if (response.writableEnded) {
      return;
    }

    response.writeHead(statusCode, { "content-type": "application/json" });
    response.end(`${JSON.stringify(payload)}\n`);
  }

  private removePendingPoll(response: ServerResponse<IncomingMessage>) {
    const index = this.pendingPolls.findIndex((entry) => entry.response === response);
    if (index === -1) {
      return;
    }

    clearTimeout(this.pendingPolls[index].timer);
    this.pendingPolls.splice(index, 1);
  }
}

const bridge = new LuauRuntimeBridge();

export async function startLuauRuntimeBridge() {
  return bridge.start();
}

export async function stopLuauRuntimeBridge() {
  return bridge.stop();
}

export function getLuauRuntimeBridgeStatus() {
  return bridge.getStatus();
}

export function getLuauRuntimeBridgeConnectionInfo() {
  return bridge.getConnectionInfo();
}

export async function dispatchLuauRunCode(
  code: string,
  mode: "edit" | "playtest",
  waitForResponseMs?: number
) {
  return bridge.dispatch(
    {
      kind: "run_code",
      payload: {
        code,
        mode
      }
    },
    waitForResponseMs
  );
}

export async function dispatchLuauCreateWorkspacePart(
  input: {
    name: string;
    anchored: boolean;
    position: { x: number; y: number; z: number };
    size: { x: number; y: number; z: number };
  },
  waitForResponseMs?: number
) {
  return bridge.dispatch(
    {
      kind: "create_workspace_part",
      payload: input
    },
    waitForResponseMs
  );
}

export async function dispatchLuauCommand(
  kind: LuauCommandKind,
  payload: Record<string, unknown>,
  waitForResponseMs?: number
) {
  return bridge.dispatch({ kind, payload }, waitForResponseMs);
}
