import { Socket } from "node:net";
import { randomUUID } from "node:crypto";

export type TcpBridgeConfig = {
  host: string;
  port: number;
  name: string;
  reconnectIntervalMs: number;
  commandTimeoutMs: number;
};

export type TcpCommandRequest = {
  id: string;
  method: string;
  params: Record<string, unknown>;
};

export type TcpCommandResponse = {
  id: string;
  status: "ok" | "error";
  result?: unknown;
  error?: string;
};

type PendingCommand = {
  resolve: (resp: TcpCommandResponse) => void;
  timer: ReturnType<typeof setTimeout>;
};

export class TcpBridge {
  private socket: Socket | null = null;
  private connected = false;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private pending = new Map<string, PendingCommand>();
  private buffer = "";
  private lastConnectedAt: string | null = null;
  private stopped = false;

  constructor(private config: TcpBridgeConfig) {}

  get isConnected(): boolean {
    return this.connected;
  }

  get status() {
    return {
      connected: this.connected,
      host: this.config.host,
      port: this.config.port,
      name: this.config.name,
      lastConnectedAt: this.lastConnectedAt,
      pendingCommands: this.pending.size
    };
  }

  start(): void {
    this.stopped = false;
    this.tryConnect();
  }

  stop(): void {
    this.stopped = true;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    for (const [id, p] of this.pending) {
      clearTimeout(p.timer);
      p.resolve({ id, status: "error", error: "Bridge shutting down" });
    }
    this.pending.clear();
    if (this.socket) {
      this.socket.destroy();
      this.socket = null;
    }
    this.connected = false;
  }

  async dispatch(method: string, params: Record<string, unknown>, timeoutMs?: number): Promise<TcpCommandResponse> {
    if (!this.connected || !this.socket) {
      return { id: "", status: "error", error: `${this.config.name} bridge not connected (port ${this.config.port})` };
    }

    const id = randomUUID();
    const request: TcpCommandRequest = { id, method, params };
    const payload = JSON.stringify(request) + "\n";

    return new Promise((resolve) => {
      const timeout = timeoutMs ?? this.config.commandTimeoutMs;
      const timer = setTimeout(() => {
        this.pending.delete(id);
        resolve({ id, status: "error", error: `Command ${method} timed out after ${timeout}ms` });
      }, timeout);

      this.pending.set(id, { resolve, timer });

      if (!this.socket) {
        this.pending.delete(id);
        clearTimeout(timer);
        resolve({ id, status: "error", error: `${this.config.name} socket is not connected` });
        return;
      }
      try {
        this.socket.write(payload);
      } catch {
        this.pending.delete(id);
        clearTimeout(timer);
        resolve({ id, status: "error", error: `Failed to write to ${this.config.name} socket` });
      }
    });
  }

  private tryConnect(): void {
    if (this.stopped) return;
    if (this.socket) {
      this.socket.destroy();
      this.socket = null;
    }

    const sock = new Socket();
    this.socket = sock;

    sock.setEncoding("utf-8");
    sock.setTimeout(10_000);

    sock.on("connect", () => {
      this.connected = true;
      this.lastConnectedAt = new Date().toISOString();
      this.buffer = "";
      sock.setTimeout(0);
      console.error(`[${this.config.name}] connected to ${this.config.host}:${this.config.port}`);
    });

    sock.on("data", (chunk: string) => {
      this.buffer += chunk;
      this.processBuffer();
    });

    sock.on("close", () => {
      this.connected = false;
      this.socket = null;
      this.scheduleReconnect();
    });

    sock.on("error", (err) => {
      this.connected = false;
      for (const [id, p] of this.pending) {
        clearTimeout(p.timer);
        p.resolve({ id, status: "error", error: `Socket error: ${err.message}` });
      }
      this.pending.clear();
    });

    sock.on("timeout", () => {
      sock.destroy();
    });

    sock.connect(this.config.port, this.config.host);
  }

  private processBuffer(): void {
    let newlineIdx: number;
    while ((newlineIdx = this.buffer.indexOf("\n")) !== -1) {
      const line = this.buffer.slice(0, newlineIdx).trim();
      this.buffer = this.buffer.slice(newlineIdx + 1);
      if (!line) continue;

      try {
        const resp = JSON.parse(line) as TcpCommandResponse;
        if (resp.id && this.pending.has(resp.id)) {
          const p = this.pending.get(resp.id)!;
          this.pending.delete(resp.id);
          clearTimeout(p.timer);
          p.resolve(resp);
        }
      } catch {
        // malformed JSON line from editor plugin — skip
      }
    }
  }

  private scheduleReconnect(): void {
    if (this.stopped || this.reconnectTimer) return;
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.tryConnect();
    }, this.config.reconnectIntervalMs);
    this.reconnectTimer.unref();
  }
}
