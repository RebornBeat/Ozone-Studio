// Same-origin harness so an agent (via browser-mcp) can drive the REAL Ozone web UI against a REAL host
// without CORS: serves ui/dist and reverse-proxies every other path to the host.
//   node tools/browser-mcp/ui-harness.mjs [port=8767] [hostUrl=http://127.0.0.1:50051]
// Then: browser_open {url:"http://localhost:8767/index.html", globals:{OZONE_HOST_URL:"http://localhost:8767"}}
// (needs OZONE_BROWSER_ALLOW_HOSTS=localhost and OZONE_BROWSER_ALLOW_GLOBALS=OZONE_HOST_URL on the browser MCP).
// Loopback-only. Adds the host's x-oz-internal token when readable so a strict-session host answers —
// meaning anything that can reach this port acts with the local-operator's authority: keep it off shared machines.
import http from "node:http";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const PORT = Number(process.argv[2] ?? 8767);
const HOST = new URL(process.argv[3] ?? "http://127.0.0.1:50051");
const dist = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "ui", "dist");
const token = (() => { for (const c of [process.env.OZONE_INTERNAL_TOKEN_FILE, process.env.OZONE_DATA_DIR && path.join(process.env.OZONE_DATA_DIR, ".internal_token")].filter(Boolean)) { try { return fs.readFileSync(c, "utf8").trim(); } catch { /* next */ } } return ""; })();
const MIME = { ".html": "text/html; charset=utf-8", ".js": "text/javascript", ".css": "text/css", ".json": "application/json", ".png": "image/png", ".svg": "image/svg+xml", ".woff2": "font/woff2" };
http.createServer((req, res) => {
  const p = new URL(req.url, "http://x").pathname;
  const f = path.join(dist, p === "/" ? "index.html" : p);
  if (f.startsWith(dist) && fs.existsSync(f) && fs.statSync(f).isFile()) { res.writeHead(200, { "Content-Type": MIME[path.extname(f)] ?? "application/octet-stream" }); fs.createReadStream(f).pipe(res); return; }
  const up = http.request({ host: HOST.hostname, port: HOST.port, path: req.url, method: req.method, headers: { ...req.headers, host: HOST.host, ...(token ? { "x-oz-internal": token } : {}) } }, (r) => { res.writeHead(r.statusCode ?? 502, r.headers); r.pipe(res); });
  up.on("error", () => { res.writeHead(502); res.end("host unreachable"); });
  req.pipe(up);
}).listen(PORT, "127.0.0.1", () => console.error(`[ui-harness] ui/dist + proxy -> ${HOST.origin} on 127.0.0.1:${PORT}`));
