#!/usr/bin/env python3
"""Persistent local HTTP service for fly_circuit_subgraph.

Real fix for a real, measured problem: circuit_worker.py, spawned fresh
per call, took 16.5s (mostly I/O) to reload the 1GB male-cns connectivity
feather file from disk every single time — well past /mcp/call's own
timeout to tool endpoints. This process loads both real feather tables
ONCE at startup and serves subgraph requests from memory — stdlib only
(http.server), no new dependency.

Listens on 127.0.0.1:3256 (loopback only). The main connectome-mcp
Node server proxies fly_circuit_subgraph calls here instead of spawning
a fresh python3 process per call.
"""
import json
import sys
from http.server import BaseHTTPRequestHandler, HTTPServer

sys.path.insert(0, "/home/rebornbeat/Projects/Ozone-Studio/tools/connectome-mcp")
from giant_fiber_circuit import build_subgraph, GF_BODY_ID

PORT = 3256

print("[circuit-service] loading real male-cns feather tables into memory…", file=sys.stderr, flush=True)
_ = build_subgraph(seed_body_id=GF_BODY_ID, top_k=5)  # forces the real module-level table cache to populate now
print("[circuit-service] warm, ready", file=sys.stderr, flush=True)


class Handler(BaseHTTPRequestHandler):
    def log_message(self, fmt, *args):
        pass  # quiet; the Node server logs the outer call

    def do_POST(self):
        if self.path != "/query":
            self.send_response(404)
            self.end_headers()
            return
        length = int(self.headers.get("Content-Length", 0))
        try:
            req = json.loads(self.rfile.read(length) or b"{}")
            seed = int(req.get("seed_body_id", GF_BODY_ID))
            top_k = max(1, min(100, int(req.get("top_k", 25))))
            result = build_subgraph(seed_body_id=seed, top_k=top_k)
            if "error" in result:
                body = json.dumps({"success": False, "error": result["error"]}).encode()
            else:
                result.pop("adjacency", None)
                result["success"] = True
                body = json.dumps(result, default=str).encode()
        except Exception as e:  # noqa: BLE001
            body = json.dumps({"success": False, "error": str(e)}).encode()

        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


if __name__ == "__main__":
    HTTPServer(("127.0.0.1", PORT), Handler).serve_forever()
