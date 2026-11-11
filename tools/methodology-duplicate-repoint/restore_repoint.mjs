#!/usr/bin/env node
// UNDO the repoint of repoint_and_delete.mjs: put each citer's saved context back (UpdateContainer) and its body file back from the run's before-copies.
// Does NOT re-create the deleted duplicate (a container restored by docs/evidence/methodology-duplicate-delete/restore.mjs gets a NEW id, so the old id
// can never be cited again — after a restore the citers must point at the NEW id: use --rebind <newId> to restore them pointing at it).
// DRY RUN BY DEFAULT.   usage (on the data-dir host):
//   node restore_repoint.mjs <run-dir> --data /opt/ozone-studio/zsei_data [--rebind <newId>] [--apply] [--host http://127.0.0.1:50051]
import fs from "node:fs";
import path from "node:path";

const argv = process.argv.slice(2);
const runDir = argv.find((a) => !a.startsWith("--") && fs.existsSync(a));
const opt = (n, d) => { const i = argv.indexOf("--" + n); return i >= 0 && argv[i + 1] && !argv[i + 1].startsWith("--") ? argv[i + 1] : d; };
const APPLY = argv.includes("--apply");
const DATA = opt("data", process.env.OZONE_DATA ?? "");
const HOST = opt("host", process.env.OZONE_HOST ?? "http://127.0.0.1:50051");
const REBIND = opt("rebind", null);
const TOKEN = process.env.OZONE_SESSION_TOKEN ?? "";
if (!runDir || !DATA) { console.error("usage: restore_repoint.mjs <run-dir> --data <zsei_data> [--rebind <newId>] [--apply]"); process.exit(64); }

const applied = JSON.parse(fs.readFileSync(path.join(runDir, "applied.json"), "utf8"));
const { dup, canon } = applied.pair;
const bdir = path.join(runDir, "before", String(dup));
const writeAtomic = (f, s) => { const t = f + `.tmp-${process.pid}`; fs.writeFileSync(t, s); fs.renameSync(t, f); };
const swap = (node, from, to) => Array.isArray(node) ? node.map((x) => swap(x, from, to)) : node && typeof node === "object"
  ? Object.fromEntries(Object.entries(node).map(([k, v]) => [k, v === from ? to : swap(v, from, to)])) : node === from ? to : node;
async function q(query) {
  const r = await fetch(HOST + "/zsei/query", { method: "POST", headers: { "content-type": "application/json", ...(TOKEN ? { authorization: `Bearer ${TOKEN}` } : {}) },
    body: JSON.stringify({ query, session_token: TOKEN }) });
  return { status: r.status, text: await r.text() };
}
console.log(`${APPLY ? "APPLY" : "DRY RUN"}: restore citers of ${dup} (repointed to ${canon})${REBIND ? `, rebinding to ${REBIND}` : ""}`);
for (const c of applied.citers) {
  const saved = JSON.parse(fs.readFileSync(path.join(bdir, `container_${c.id}.json`), "utf8")).result.Container;
  let ctx = saved.local_state.context;
  if (REBIND) ctx = swap(ctx, dup, Number(REBIND));
  console.log(`  citer ${c.id}: context methodologies -> ${JSON.stringify(ctx.methodologies)}${c.body ? `; body ${c.body.file} from the before-copy` : ""}`);
  if (!APPLY) continue;
  const r = await q({ UpdateContainer: { container_id: c.id, updates: { context: ctx } } });
  if (r.status !== 200 || !/Success/.test(r.text)) { console.error(`  UpdateContainer ${c.id} failed: ${r.text.slice(0, 200)}`); process.exit(2); }
  if (c.body) {
    const src = path.join(bdir, "bodies", `${c.id}__${path.basename(c.body.file)}`);
    let body = fs.readFileSync(src, "utf8");
    if (REBIND) body = JSON.stringify(swap(JSON.parse(body), dup, Number(REBIND)), null, 2);
    writeAtomic(path.join(DATA, c.body.file), body);
  }
}
console.log(APPLY ? "done" : "dry run only — add --apply");
