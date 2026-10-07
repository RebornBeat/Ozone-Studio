// Ozone-Studio Mesh MCP — real 3D mesh analysis via trimesh (CPU, offline).
// Feeds the mesh properties assets/pipelines/modalities/3D/main.rs's MeshInfo,
// BoundingBox3D and surface/volume fields expect; computed from the real file,
// never placeholder values.
//
// Tools:
//   mesh_analyze {input:{path}}                 → vertex/face/edge counts, watertight,
//     winding consistency, volume (closed meshes only), surface area, bounds,
//     boundary and non-manifold edge counts, body count, Euler number.
//   mesh_convert {input:{path, out_format}}     → writes a converted copy next to
//     the input (stl|obj|ply|glb|off). Refuses to overwrite an existing file.
//   cad_step_info {input:{path}}                → STEP (.step/.stp) product structure via
//     python-step-parser: product ids, names, descriptions. Parser writes a .db
//     cache beside the file; it is removed after each call.
//
// Paths are resolved strictly inside OZONE_MESH_DATA_DIR (default ./data) — the
// server cannot read or write outside it. Mesh loading is local; no network.
//
// Env: OZONE_MESH_PORT (default 3276), OZONE_MESH_PYTHON (default ./.venv/bin/python3),
// OZONE_MESH_DATA_DIR (default ./data).

import { createServer } from "node:http";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const PORT = Number(process.env.OZONE_MESH_PORT ?? 3276);
const SELF_DIR = path.dirname(fileURLToPath(import.meta.url));
const MESH_PYTHON = process.env.OZONE_MESH_PYTHON ?? path.join(SELF_DIR, ".venv", "bin", "python3");
const DATA_DIR = path.resolve(process.env.OZONE_MESH_DATA_DIR ?? path.join(SELF_DIR, "data"));
const CONVERT_FORMATS = new Set(["stl", "obj", "ply", "glb", "off"]);

const ANALYZE_PY = `
import json, sys
args = json.loads(sys.argv[1])
try:
    import numpy as np
    import trimesh
    loaded = trimesh.load(args["path"], force="mesh")
    if not isinstance(loaded, trimesh.Trimesh) or len(loaded.faces) == 0:
        print(json.dumps({"ok": False, "error": "file did not load as a non-empty triangle mesh"}))
        sys.exit(0)
    m = loaded
    _, counts = np.unique(m.edges_sorted, axis=0, return_counts=True)
    boundary = int((counts == 1).sum())
    nonmanifold = int((counts > 2).sum())
    lo, hi = m.bounds
    vol = float(m.volume)
    bodies = m.split(only_watertight=False)
    body_rows = []
    for i, b in enumerate(bodies):
        try:
            body_rows.append({
                "index": i,
                "vertex_count": int(len(b.vertices)),
                "face_count": int(len(b.faces)),
                "is_volume": bool(b.is_volume),
                "volume": float(b.volume) if b.is_volume else None,
            })
        except Exception:
            body_rows.append({"index": i, "vertex_count": None, "face_count": None, "is_volume": None, "volume": None})
    print(json.dumps({
        "ok": True,
        "vertex_count": int(len(m.vertices)),
        "face_count": int(len(m.faces)),
        "edge_count": int(len(m.edges_unique)),
        "polygon_type": "Triangles",
        "is_watertight": bool(m.is_watertight),
        "is_winding_consistent": bool(m.is_winding_consistent),
        "is_volume": bool(m.is_volume),
        "boundary_edge_count": boundary,
        "nonmanifold_edge_count": nonmanifold,
        "is_manifold": bool(m.is_watertight and nonmanifold == 0),
        "volume": vol if m.is_volume else None,
        "inside_out": bool(m.is_volume and vol < 0),
        "surface_area": float(m.area),
        "bounding_box": {
            "min": lo.tolist(), "max": hi.tolist(),
            "center": ((lo + hi) / 2).tolist(), "size": (hi - lo).tolist(),
        },
        "body_count": len(bodies),
        "bodies": body_rows,
        "euler_number": int(m.euler_number),
    }))
except Exception as e:
    print(json.dumps({"ok": False, "error": f"{type(e).__name__}: {e}"}))
`;

const CONVERT_PY = `
import json, os, sys
args = json.loads(sys.argv[1])
try:
    import trimesh
    m = trimesh.load(args["path"], force="mesh")
    m.export(args["out_path"])
    print(json.dumps({
        "ok": True,
        "vertex_count": int(len(m.vertices)),
        "face_count": int(len(m.faces)),
        "bytes": os.path.getsize(args["out_path"]),
    }))
except Exception as e:
    print(json.dumps({"ok": False, "error": f"{type(e).__name__}: {e}"}))
`;

const CAD_PY = `
import json, os, sys
args = json.loads(sys.argv[1])
db = args["path"] + ".db"
try:
    from python_step_parser.step_parser import StepParser
    p = StepParser(args["path"])
    p.parse()
    products = []
    for pid in p.get_products():
        a = p.get_arguments(pid)
        products.append({
            "entity_id": int(pid),
            "id": a[0] if len(a) > 0 else None,
            "name": a[1] if len(a) > 1 else None,
            "description": a[2] if len(a) > 2 else None,
        })
    result = {"ok": True, "product_count": len(products), "products": products}
except Exception as e:
    result = {"ok": False, "error": f"{type(e).__name__}: {e}"}
finally:
    if os.path.exists(db):
        os.remove(db)
print(json.dumps(result))
`;

function reply(res, code, body) {
  res.writeHead(code, { "Content-Type": "application/json" });
  res.end(JSON.stringify(body));
}

function resolveInside(p) {
  const abs = path.resolve(DATA_DIR, String(p ?? ""));
  if (abs !== DATA_DIR && !abs.startsWith(DATA_DIR + path.sep)) {
    throw new Error(`path must be inside the mesh data dir (${DATA_DIR})`);
  }
  return abs;
}

function runPy(script, args) {
  return new Promise((resolve) => {
    const child = spawn(MESH_PYTHON, ["-c", script, JSON.stringify(args)], { timeout: 60000 });
    let out = "";
    let err = "";
    child.stdout.on("data", (d) => { out += d; });
    child.stderr.on("data", (d) => { err += d; });
    child.on("close", () => {
      const line = out.split("\n").find((l) => l.trim().startsWith("{"));
      try {
        resolve(JSON.parse(line ?? "{}"));
      } catch {
        resolve({ ok: false, error: `mesh backend produced no JSON (stderr: ${err.slice(0, 300)})` });
      }
    });
    child.on("error", (e) => resolve({ ok: false, error: `failed to spawn mesh python: ${e.message}` }));
  });
}

// Graph block: one root for the whole mesh, one Body per real connected
// component (trimesh split). Unmeasurable values are null, never zero.
function meshGraph(out) {
  const nodes = [{
    key: "mesh",
    kind: "Mesh",
    label: path.basename(out.path ?? "mesh"),
    attributes: {
      vertex_count: out.vertex_count,
      face_count: out.face_count,
      edge_count: out.edge_count,
      is_watertight: out.is_watertight,
      volume: out.volume,
      surface_area: out.surface_area,
      body_count: out.body_count,
    },
  }];
  (out.bodies ?? []).forEach((b, i) => {
    nodes.push({
      key: `body-${i}`,
      kind: "Body",
      label: `body-${i}`,
      parent: "mesh",
      attributes: { vertex_count: b.vertex_count, face_count: b.face_count, volume: b.volume, is_volume: b.is_volume },
    });
  });
  return { nodes, edges: [] };
}

async function meshAnalyze(input) {
  const abs = resolveInside(input.path);
  if (!existsSync(abs)) throw new Error(`no such file: ${input.path}`);
  const out = await runPy(ANALYZE_PY, { path: abs });
  if (!out.ok) throw new Error(out.error);
  const result = { path: path.relative(DATA_DIR, abs), ...out };
  return { ...result, graph: meshGraph(result) };
}

async function cadStepInfo(input) {
  const abs = resolveInside(input.path);
  if (!existsSync(abs)) throw new Error(`no such file: ${input.path}`);
  if (!/\.(step|stp)$/i.test(abs)) throw new Error("cad_step_info requires a .step or .stp file");
  const out = await runPy(CAD_PY, { path: abs });
  if (!out.ok) throw new Error(out.error);
  return { path: path.relative(DATA_DIR, abs), ...out };
}

async function meshConvert(input) {
  const abs = resolveInside(input.path);
  if (!existsSync(abs)) throw new Error(`no such file: ${input.path}`);
  const fmt = String(input.out_format ?? "").toLowerCase();
  if (!CONVERT_FORMATS.has(fmt)) throw new Error(`out_format must be one of: ${[...CONVERT_FORMATS].join(", ")}`);
  const parsed = path.parse(abs);
  const outAbs = path.join(parsed.dir, `${parsed.name}.${fmt}`);
  if (outAbs === abs) throw new Error("output would overwrite the input; choose a different out_format");
  if (existsSync(outAbs)) throw new Error(`refusing to overwrite existing file: ${path.relative(DATA_DIR, outAbs)}`);
  const out = await runPy(CONVERT_PY, { path: abs, out_path: outAbs });
  if (!out.ok) throw new Error(out.error);
  return { path: path.relative(DATA_DIR, abs), out_path: path.relative(DATA_DIR, outAbs), ...out };
}

const server = createServer((req, res) => {
  if (req.method !== "POST" || !(req.url ?? "").startsWith("/call")) {
    reply(res, 404, { error: "POST /call only" });
    return;
  }
  const chunks = [];
  req.on("data", (c) => chunks.push(c));
  req.on("end", async () => {
    let tool = "";
    let input = {};
    try {
      const body = JSON.parse(Buffer.concat(chunks).toString("utf8") || "{}");
      tool = String(body?.tool ?? "");
      input = body?.input ?? {};
    } catch {
      reply(res, 400, { success: false, error: "invalid JSON body" });
      return;
    }
    try {
      if (tool === "mesh_analyze") {
        reply(res, 200, { success: true, output: await meshAnalyze(input) });
      } else if (tool === "mesh_convert") {
        reply(res, 200, { success: true, output: await meshConvert(input) });
      } else if (tool === "cad_step_info") {
        reply(res, 200, { success: true, output: await cadStepInfo(input) });
      } else {
        reply(res, 200, { success: false, error: `unknown mesh tool '${tool}' (mesh_analyze, mesh_convert, cad_step_info)` });
      }
    } catch (e) {
      reply(res, 200, { success: false, error: e instanceof Error ? e.message : String(e) });
    }
  });
});

server.on("error", (err) => {
  if (err.code === "EADDRINUSE") { console.error(`[mesh-mcp] :${PORT} in use — skipping`); return; }
  console.error("[mesh-mcp] server error:", err);
});

server.listen(PORT, "127.0.0.1", () => {
  console.error(`[mesh-mcp] /call on 127.0.0.1:${PORT} — mesh_analyze, mesh_convert (trimesh), cad_step_info (python-step-parser), data dir ${DATA_DIR}`);
});
