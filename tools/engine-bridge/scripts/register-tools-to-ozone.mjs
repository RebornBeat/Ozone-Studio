// Bulk-register the engine bridge's tools into Ozone-Studio's MCP tool registry (docs/TOOLS_PIPELINES_MCP_GUIDE.md §2.3).
//
// No hardcoded list: spawns this package's own dist/index.js as an MCP server over stdio, connects a real MCP client, calls listTools, then POSTs
// every tool into the host's /mcp/tools/register with endpoint = the bridge's /call surface (default http://127.0.0.1:3210/call — the same
// endpoint the 67 already-registered rows use, so re-running this is an idempotent refresh). `bridge_status` (engine + project status, served
// by the bridge's /call) is registered too. The registry entry is the capability catalog; invocation flows through /mcp/call (jurisdiction
// gate, usage ledger, loopback guard).
//
// /mcp/tools is a PROTECTED host route on a strict host: the call carries the host's internal token (tools/shared/guard.mjs lookup).
//
// Usage: node scripts/register-tools-to-ozone.mjs
// Env:   OZONE_HOST (default http://127.0.0.1:50051), BRIDGE_CALL_URL (default http://127.0.0.1:3210/call)

import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { internalHeaders } from "../../shared/guard.mjs";

const OZONE_HOST = (process.env.OZONE_HOST ?? "http://127.0.0.1:50051").replace(/\/+$/, "");
const BRIDGE_CALL_URL = process.env.BRIDGE_CALL_URL ?? "http://127.0.0.1:3210/call";
const HERE = path.dirname(fileURLToPath(import.meta.url));
const SERVER_JS = path.join(HERE, "..", "dist", "index.js");

const transport = new StdioClientTransport({
  command: "node",
  args: [SERVER_JS],
  // a second instance spawned just to list tools must not fight the running bridge for its ports
  env: { ...process.env, OZONE_BRIDGE_PORT: "0", OZONE_SUPERVISED: "" },
});
const client = new Client({ name: "ozone-tool-registrar", version: "1.0.0" });
await client.connect(transport);

const { tools } = await client.listTools();
const all = [
  ...tools.map((t) => ({ name: t.name, description: t.description ?? "" })),
  { name: "bridge_status", description: "Engine bridge status: Roblox / Luau / Unity / Unreal / Blender connectors, runtimes and the project manifest." },
];
console.error(`derived ${all.length} tools from the engine bridge`);

let ok = 0;
let fail = 0;
for (const t of all) {
  const res = await fetch(`${OZONE_HOST}/mcp/tools/register`, {
    method: "POST",
    headers: internalHeaders(),
    body: JSON.stringify({
      name: t.name,
      transport: "http",
      endpoint: BRIDGE_CALL_URL,
      capabilities: [t.description],
      server_version: "bridges-0.1.0",
    }),
  });
  if (res.ok) ok++;
  else fail++;
  console.error(`  ${res.ok ? "✓" : "✗"} ${t.name}`);
}

console.error(`registered ${ok}/${all.length} into ${OZONE_HOST}/mcp/tools (${fail} failed)`);
await client.close();
process.exit(fail > 0 ? 1 : 0);
