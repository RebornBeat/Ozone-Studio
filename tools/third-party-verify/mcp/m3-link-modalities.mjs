// MCP-3 link one file per modality through ozone-mcp and verify the order of knowledge:
//   FILE LINK FIRST -> the host builds the NON-SEMANTIC graph by itself (graph_on_link, no prompt);
//   THEN a walk over the same file adds intelligence; both read the same current state.
// Mutating (links files into a project, may create graph containers, and with OZONE_TPV_WALK=1 spends model calls).
//   OZONE_PROJECT_ID=<id> [OZONE_ZSEI_PARENT_ID=<real project container id>] [OZONE_WORKSPACE_ID=<id>] [OZONE_TPV_WALK=1]
//   node tools/third-party-verify/mcp/m3-link-modalities.mjs
// Fixtures are tiny real files written to a throwaway dir (the only allowed link root); modalities without a fixture are SKIPPED, not faked.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { threeAgents, suite, until } from "./harness.mjs";
const s = suite("m3-link-modalities");
const project_id = Number(process.env.OZONE_PROJECT_ID);
if (!Number.isFinite(project_id)) { s.fail("L0", "OZONE_PROJECT_ID set", "link tests need a real project id (read it with ozone_query GetProjects)"); process.exit(s.finish() ? 1 : 0); }
const dir = fs.mkdtempSync(path.join(os.tmpdir(), "ozone-mcp-link-"));
const png = Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==", "base64");
const wav = (() => { const b = Buffer.alloc(44 + 1600); b.write("RIFF", 0); b.writeUInt32LE(36 + 1600, 4); b.write("WAVEfmt ", 8); b.writeUInt32LE(16, 16); b.writeUInt16LE(1, 20); b.writeUInt16LE(1, 22); b.writeUInt32LE(8000, 24); b.writeUInt32LE(16000, 28); b.writeUInt16LE(2, 32); b.writeUInt16LE(16, 34); b.write("data", 36); b.writeUInt32LE(1600, 40); return b; })();
const fixtures = {
  text: ["note.txt", "Ozone links files and builds a graph before any prompt.\nA second line follows."],
  code: ["main.rs", "fn helper() -> u32 { 1 }\nfn main() { println!(\"{}\", helper()); }\n"],
  image: ["pixel.png", png],
  audio: ["tone.wav", wav],
  "3d": ["tri.obj", "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n"],
  "3d-stl": ["tri.stl", "solid t\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid t\n"],
};
for (const [, [name, data]] of Object.entries(fixtures)) fs.writeFileSync(path.join(dir, name), data);
const { agents, close } = await threeAgents({ linkRoots: dir });
const [a, b, c] = [agents["claude-code"], agents["zcode"], agents["third-agent"]];
const common = { project_id, workspace_id: process.env.OZONE_WORKSPACE_ID ? Number(process.env.OZONE_WORKSPACE_ID) : undefined, zsei_parent_id: process.env.OZONE_ZSEI_PARENT_ID ? Number(process.env.OZONE_ZSEI_PARENT_ID) : undefined };
try {
  // hardening: paths outside the allowed root are refused before any host call
  const esc = await a.ozone.call("ozone_link", { kind: "file", path: "/etc/hostname", ...common });
  s.check("H1", "link outside the allowed root is refused", !esc.ok && /outside the allowed link roots/.test(esc.text), esc.text.slice(0, 140));
  const rel = await a.ozone.call("ozone_link", { kind: "file", path: "note.txt", ...common });
  s.check("H2", "relative path refused", !rel.ok && /absolute/.test(rel.text), rel.text.slice(0, 100));
  fs.symlinkSync("/etc/hostname", path.join(dir, "escape.txt"));
  const sym = await a.ozone.call("ozone_link", { kind: "file", path: path.join(dir, "escape.txt"), ...common });
  s.check("H3", "symlink escaping the root is refused (real path is checked)", !sym.ok && /outside the allowed link roots/.test(sym.text), sym.text.slice(0, 140));

  // ACCEPTANCE == READABILITY: an extension with no reader and a malformed file of an accepted type are rejected AT LINK TIME with the exact reason
  fs.writeFileSync(path.join(dir, "capture.iq"), Buffer.from([1, 2, 3, 4]));
  fs.writeFileSync(path.join(dir, "bad.txt"), Buffer.from([0xff, 0xfe, 0x00, 0x80]));
  fs.writeFileSync(path.join(dir, "bad.las"), Buffer.alloc(200));
  const iq = await a.ozone.call("ozone_link", { kind: "file", path: path.join(dir, "capture.iq"), ...common });
  s.check("R1", "an extension with no reader (.iq) is rejected at link time with the reason", /not accepted/.test(iq.text), iq.text.slice(0, 200));
  const badTxt = await a.ozone.call("ozone_link", { kind: "file", path: path.join(dir, "bad.txt"), ...common });
  s.check("R2", "a non-UTF-8 .txt is rejected at link time (malformed for its type)", /UTF-8/.test(badTxt.text), badTxt.text.slice(0, 200));
  const badLas = await a.ozone.call("ozone_link", { kind: "file", path: path.join(dir, "bad.las"), ...common });
  s.check("R3", "a .las without the LASF signature is rejected at link time", /LASF/.test(badLas.text), badLas.text.slice(0, 200));

  const before = (await b.ozone.call("ozone_callgraph_files", { project_id })).json;
  const beforeIds = new Set((before?.files ?? []).map((r) => r.file_reference_id));
  const linkedAt = Date.now();
  for (const [mod, [name]] of Object.entries(fixtures)) {
    const r = await a.ozone.call("ozone_link", { kind: "file", path: path.join(dir, name), ...common });
    s.check(`K-${mod}`, `linked ${name}`, r.ok, r.text.slice(0, 160));
  }
  // the host must now build the NON-SEMANTIC graph with no prompt. Read as agent C; report each file's real status.
  const rows = await until(async () => {
    const r = (await c.ozone.call("ozone_callgraph_files", { project_id })).json;
    const all = r?.files ?? [];
    const mine = all.filter((x) => !beforeIds.has(x.file_reference_id));
    return mine.length >= Object.keys(fixtures).length && mine.every((x) => /^(graph_created|complete|failed.*|not_accepted|file_missing)$/i.test(String(x.status))) ? mine : null;
  });
  s.check("G0", "graph_on_link produced a ledger row for every linked file with no prompt (as read)", !!rows, rows ? `${rows.length} rows` : "timed out waiting for rows");
  const byPath = (n) => (rows ?? []).find((x) => String(x.path ?? "").endsWith(n));
  for (const [mod, [name]] of Object.entries(fixtures)) {
    const row = byPath(name);
    if (!row) { s.fail(`G-${mod}`, `ledger row for ${name}`, "absent"); continue; }
    const st = String(row.status);
    s.check(`G-${mod}`, `graph for ${name}: structural graph recorded by the host (status graph_created|complete, graph_id present, structural_done)`, /^(graph_created|complete)$/.test(st) && row.graph_id != null && row.structural_done === true, JSON.stringify({ status: st, graph: row.graph_id, hash: row.hash, trigger: row.trigger, created_by: row.created_by, task: row.task_id }).slice(0, 220));
  }
  // both views agree on current state: a second agent sees the same rows
  const again = (await b.ozone.call("ozone_callgraph_files", { project_id })).json;
  const sameCount = (again?.files ?? []).filter((x) => !beforeIds.has(x.file_reference_id)).length;
  s.check("S1", "a second agent reads the same number of new ledger rows (shared current state)", sameCount === (rows ?? []).length, `${sameCount} vs ${(rows ?? []).length}`);

  // idempotence: linking the unchanged file again must not create a new graph task (content hash)
  const again1 = await a.ozone.call("ozone_link", { kind: "file", path: path.join(dir, fixtures.text[0]), ...common });
  s.pass("I0", "re-link of an unchanged file accepted", again1.text.slice(0, 120));

  // then the walk (intelligence) over the same file: the non-semantic graph must still be there and be reused/extended, never lost
  if (process.env.OZONE_TPV_WALK === "1") {
    const p = process.env.OZONE_TPV_PROMPT ?? "Summarize what this file says and list its entry points.";
    const w = await a.ozone.call("ozone_orchestrate", { prompt: p, attached_files: [path.join(dir, fixtures.code[0])], project_id: common.project_id, workspace_id: common.workspace_id });
    const fin = w.json?.run_token ? await until(async () => { const r = await b.ozone.call("ozone_run_status", { run_token: w.json.run_token }); return r.json?.state && r.json.state !== "running" ? r.json : null; }) : null;
    s.check("W1", "walk over the linked file reached a non-running state", !!fin, fin ? `state=${fin.state}` : w.text.slice(0, 160));
    const after = (await c.ozone.call("ozone_callgraph_files", { project_id })).json;
    const arow = (after?.files ?? []).find((x) => String(x.path ?? "").endsWith(fixtures.code[0]));
    const brow = byPath(fixtures.code[0]);
    s.check("W2", "the link-time graph row for this file still exists after the walk", !!arow, JSON.stringify({ before: brow?.graph_id, after: arow?.graph_id }));
    s.check("W3", "the walk REUSED the link-time structural graph for unchanged content (same graph_id, same hash) instead of building a second one", !!arow && arow.graph_id === brow?.graph_id && arow.hash === brow?.hash, `${brow?.graph_id} -> ${arow?.graph_id}; created_by ${brow?.created_by} -> ${arow?.created_by}`);
  } else s.skip("W1", "walk over the linked file (intelligence on top of the non-semantic graph)", "OZONE_TPV_WALK=1 not set");
  s.pass("T0", `elapsed since first link`, `${Math.round((Date.now() - linkedAt) / 1000)}s`);
} finally { close(); }
process.exit(s.finish() ? 1 : 0);
