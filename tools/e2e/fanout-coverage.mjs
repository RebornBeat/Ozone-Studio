// FO-E3: edge-class coverage over EVERY modality graph file the host holds (real data, no mocks).
// For each graphs/<modality>_<id>.json -> POST /context/span_out seeded on <id> -> fanout.class_counts.
// Reports per modality: graphs, nodes, edges, share of edges classified (not "other"), graphs with ZERO edges.
import fs from "node:fs";
import path from "node:path";
import { suite, http } from "./lib.mjs";
const s = suite("e2e-fanout-coverage");
const DATA = process.env.OZONE_DATA ?? new URL("../../target/release/zsei_data", import.meta.url).pathname;
const files = fs.readdirSync(path.join(DATA, "graphs")).filter((f) => /^[a-z0-9]+_\d+\.json$/.test(f));
const by = {};
let missing = 0;
// graph FILE -> owning CONTAINER (file ids are local graph ids; containers live in local/<id>.json and point at the file)
const owner = {};
for (const f of fs.readdirSync(path.join(DATA, "local"))) {
  if (!f.endsWith(".json")) continue;
  const raw = fs.readFileSync(path.join(DATA, "local", f), "utf8");
  const m = raw.match(/"object_store_path"\s*:\s*"graphs\/([a-z0-9]+_\d+\.json)"/);
  if (m) owner[m[1]] = Number(f.replace(".json", ""));
}
for (const f of files) {
  const [mod] = f.replace(".json", "").split("_");
  const id = owner[f];
  if (!id) { (by[mod] ??= { graphs: 0, nodes: 0, edges: 0, other: 0, empty_edge_graphs: 0, empty_edges: 0, fanned: 0, orphan_files: 0 }).orphan_files = ((by[mod]?.orphan_files) ?? 0) + 1; missing++; continue; }
  const g = JSON.parse(fs.readFileSync(path.join(DATA, "graphs", f), "utf8"));
  const edges = (g.edges ?? []).length, nodes = (g.nodes ?? []).length;
  const sp = (await http("POST", "/context/span_out", { seeds: [id], budget_tokens: 200000 })).json;
  const item = (sp.items ?? []).find((i) => i.id === id);
  const e = (by[mod] ??= { graphs: 0, nodes: 0, edges: 0, other: 0, empty_edges: 0, fanned: 0, orphan_files: 0 });
  e.graphs++; e.nodes += nodes; e.edges += edges; if (!edges) e.empty_edges++;
  if (item?.fanout) { e.fanned++; e.other += item.fanout.class_counts?.other ?? 0; } else missing++;
}
for (const [mod, e] of Object.entries(by).sort()) {
  const classified = e.edges ? Math.round(100 * (1 - e.other / e.edges)) : null;
  console.log(`${mod.padEnd(16)} graphs=${String(e.graphs).padStart(3)} nodes=${String(e.nodes).padStart(5)} edges=${String(e.edges).padStart(5)} classified=${classified === null ? "n/a" : classified + "%"} empty_edge_graphs=${e.empty_edges} fanned=${e.fanned}/${e.graphs} orphan_files=${e.orphan_files ?? 0}`);
  if (e.edges) s.check(`COV-${mod}`, `${mod}: ≥95% of edges classified into a fan-out class`, 100 * (1 - e.other / e.edges) >= 95, `other=${e.other}/${e.edges}`);
  if (e.empty_edges) s.skip(`CAP-${mod}`, `${mod}: ${e.empty_edges}/${e.graphs} graph(s) captured NO edges (capture gap, not a fan-out gap)`, "tracked");
}
if (missing) s.skip("COV-orphans", `${missing} graph file(s) of ${files.length} have NO owning container (data hygiene finding: leftovers, mostly pre-existing code graphs)`, "tracked — not a fan-out defect");
s.check("COV-all", "every graph that HAS an owning container is fanned out (fanout evidence present)", Object.values(by).every((e) => e.fanned === e.graphs), JSON.stringify(Object.fromEntries(Object.entries(by).filter(([, e]) => e.fanned !== e.graphs).map(([k, e]) => [k, `${e.fanned}/${e.graphs}`]))));
process.exit(s.finish() ? 1 : 0);
