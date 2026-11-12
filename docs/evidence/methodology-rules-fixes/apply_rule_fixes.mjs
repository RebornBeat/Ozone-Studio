// apply_rule_fixes.mjs — applies fixes_final.json (rule retire / rewrite / add, providers, renames) through pipeline 12 `Update`.
//
//   run ON the EU from /opt/ozone-build/tools/e2e (uses ./lib.mjs like docs/evidence/methodology-regen/apply.mjs):
//     node apply_rule_fixes.mjs                       DRY RUN (default): reads containers+bodies, simulates every Update, prints the diff, writes NOTHING
//     node apply_rule_fixes.mjs --apply               applies, one methodology at a time (order = plan order), /health between
//     node apply_rule_fixes.mjs --apply --only 40396,40125
//     node apply_rule_fixes.mjs --restore <run-dir>   DRY RUN of the restore; add --apply to point the containers back at their saved bodies
//   env: PLAN (default ./fixes_final.json next to this file) | PLAN_B64 (gzip+base64 plan, for runs without a file) | OUT_DIR | PIPELINE_BIN
//        OZONE_ZSEI_DATA_DIR (default /opt/ozone-studio/zsei_data) | OZONE_APP_ROOT (default /opt/ozone-studio)
//
// FAIL CLOSED. `Update` is `#[serde(tag="action")]` with unknown fields ignored: an OLD pipeline binary silently drops remove_decision_rules /
// remove_heuristics and would only APPEND the additions (every rule duplicated). So before any write the script requires BOTH
//   (1) the deployed pipeline binary (default /opt/ozone-studio/pipelines/methodology_create) is newer than the commit that added remove_*
//       (88d700a, 2026-11-10T17:30:24-04:00 = epoch 1794346224), and
//   (2) the binary really contains the retire code (the string "refusing to retire" — a literal in `Update`'s guard; absent in an old binary).
// Either check failing (or the binary missing/unreadable) = stop with exit 3. --apply never runs unless both pass. The dry run still prints the
// verdict. (As of 2026-11-11 the EU binary is from 2026-11-09 01:06 -04:00 and lacks the string -> this script refuses to apply.)
//
// Per methodology: save BEFORE container + body (+ the payload) -> check every removal text matches the CURRENT body exactly (a drifted body = anomaly, stop)
// -> simulate -> Update -> re-read container + body -> body must equal the simulation exactly (rules, heuristics, principles, name, description)
// -> /health. ANY anomaly stops the run (nothing further is applied) and prints how to restore. Every Update writes a NEW body file and keeps the
// old one (the pipeline never deletes bodies): restoring = pointing the container's object_store_path back (see --restore).
import fs from "node:fs";
import path from "node:path";
import zlib from "node:zlib";
import { fileURLToPath } from "node:url";
import { http, pipeline } from "./lib.mjs";

const args = process.argv.slice(2);
const APPLY = args.includes("--apply");
const RESTORE = args.includes("--restore") ? args[args.indexOf("--restore") + 1] : null;
const ONLY = args.includes("--only") ? args[args.indexOf("--only") + 1].split(",").map(Number) : null;
const DATA = process.env.OZONE_ZSEI_DATA_DIR ?? "/opt/ozone-studio/zsei_data";
const APP = process.env.OZONE_APP_ROOT ?? "/opt/ozone-studio";
const BIN = process.env.PIPELINE_BIN ?? "/opt/ozone-studio/pipelines/methodology_create";
const HERE = (() => { try { return path.dirname(fileURLToPath(import.meta.url)); } catch { return process.cwd(); } })();
const TS = new Date().toISOString().replace(/[:.]/g, "-");
const OUT = process.env.OUT_DIR ?? path.resolve(HERE, "../../docs/evidence/methodology-rules-fixes/run-" + TS);

const die = (code, msg) => { console.error("\nSTOP: " + msg); process.exit(code); };
const J = (x) => JSON.stringify(x);
const real = (t) => { t = String(t ?? "").trim(); return t !== "" && t !== "..." && t !== "…"; };

function loadPlan() {
  if (process.env.PLAN_B64) return JSON.parse(zlib.gunzipSync(Buffer.from(process.env.PLAN_B64, "base64")).toString("utf8"));
  return JSON.parse(fs.readFileSync(process.env.PLAN ?? path.join(HERE, "fixes_final.json"), "utf8"));
}

// ---- fail-closed capability check -------------------------------------------------------------------------------------------------------
function preflight(plan) {
  const pf = plan.preflight;
  let st, buf;
  try { st = fs.statSync(BIN); buf = fs.readFileSync(BIN); } catch (e) { return { ok: false, why: `cannot read the pipeline binary ${BIN}: ${e.message}` }; }
  const mtime = Math.floor(st.mtimeMs / 1000);
  const hasMarker = buf.includes(Buffer.from(pf.binary_marker));
  const newer = mtime >= pf.commit_time_utc_epoch;
  const ok = hasMarker && newer;
  return { ok, why: `binary ${BIN} mtime ${new Date(mtime * 1000).toISOString()} (${newer ? ">=" : "<"} commit ${pf.commit} at epoch ${pf.commit_time_utc_epoch}); retire code marker "${pf.binary_marker}" ${hasMarker ? "PRESENT" : "ABSENT"}` };
}

// ---- reading -----------------------------------------------------------------------------------------------------------------------------
async function health() {
  const r = await http("GET", "/health");
  return r.status === 200 && (r.json?.healthy === true);
}
async function getContainer(id) {
  const r = (await http("POST", "/zsei/query", { query: { GetContainer: { container_id: id } }, session_token: "" })).json;
  const c = r?.result?.Container ?? r?.Container ?? r?.output?.Container;
  if (!c) throw new Error(`GetContainer ${id}: no container in ${J(r).slice(0, 200)}`);
  return c;
}
function readBody(rel) {
  for (const base of [DATA, APP]) {
    const f = path.join(base, rel);
    if (fs.existsSync(f)) return { file: f, body: JSON.parse(fs.readFileSync(f, "utf8")) };
  }
  throw new Error(`body file ${rel} not found under ${DATA} or ${APP}`);
}
async function read(id) {
  const c = await getContainer(id);
  const rel = c?.local_state?.storage?.object_store_path;
  if (!rel) throw new Error(`methodology ${id} has no object_store_path`);
  return { container: c, rel, ...readBody(rel) };
}

// ---- simulation of methodology_create Update (mirrors main.rs: retire by exact trimmed text, THEN additions, never-empty guard) -------------
function simulate(body, p) {
  const b = JSON.parse(JSON.stringify(body));
  b.decision_rules ??= []; b.heuristics ??= [];
  if (p.name != null) b.name = p.name;
  if (p.description != null) b.description = p.description;
  const anomalies = [];
  if (p.remove_principles) {
    for (const s of p.remove_principles) if (!(b.principles ?? []).some((x) => x?.statement === s)) anomalies.push(`principle "${s}" not found`);
    b.principles = (b.principles ?? []).filter((x) => !p.remove_principles.includes(x?.statement));
  }
  for (const [field, key, list] of [["decision_rules", "condition", p.remove_decision_rules ?? []], ["heuristics", "when_to_apply", p.remove_heuristics ?? []]]) {
    for (const t of list.map((x) => x.trim())) {
      const before = b[field].length;
      b[field] = b[field].filter((e) => String(e?.[key] ?? "").trim() !== t);
      if (b[field].length === before) anomalies.push(`${field}: no entry has ${key} exactly ${J(t)}`);
    }
  }
  for (const a of p.add_principles ?? []) (b.principles ??= []).push({ statement: a.name, rationale: a.description, applicability: [] });
  for (const a of p.add_heuristics ?? []) b.heuristics.push({ name: a.condition, when_to_apply: a.condition, description: a.action, examples: [] });
  for (const a of p.add_decision_rules ?? []) b.decision_rules.push({
    condition: a.condition, action: a.outcome, priority: 5, exceptions: [],
    requires: (a.requires ?? []).filter((r) => String(r.name ?? "").trim()).map((r) => ({ kind: r.kind ?? "provider", name: r.name.trim(), purpose: r.purpose ?? null, optional: !!r.optional })),
  });
  const removing = (p.remove_decision_rules?.length ?? 0) + (p.remove_heuristics?.length ?? 0) > 0;
  const hasReal = b.decision_rules.some((r) => real(r.condition) && real(r.action ?? r.outcome)) || b.heuristics.some((h) => real(h.when_to_apply) && real(h.description));
  if (removing && !hasReal) anomalies.push("would leave the methodology with no real rule");
  return { body: b, anomalies };
}
const norm = (b) => J({
  name: b.name ?? null, description: b.description ?? null,
  principles: (b.principles ?? []).map((p) => [p.statement, p.rationale]),
  rules: (b.decision_rules ?? []).map((r) => [r.condition, r.action ?? r.outcome, (r.requires ?? []).map((q) => [q.kind, q.name, q.purpose ?? null, !!q.optional])]),
  heur: (b.heuristics ?? []).map((h) => [h.when_to_apply, h.description]),
});
function findKey(o, k) { if (o && typeof o === "object") { if (k in o) return o[k]; for (const v of Object.values(o)) { const r = findKey(v, k); if (r !== undefined) return r; } } return undefined; }

// ---- restore -----------------------------------------------------------------------------------------------------------------------------
// The saved <run-dir>/<id>/container.json is the container as it was BEFORE the Update. The old body file is still on disk (the pipeline never deletes
// one), so restoring = UpdateContainer back to the saved metadata/context/storage (object_store_path -> the old body). Dry run unless --apply.
async function restore(dir) {
  const ids = fs.readdirSync(dir).filter((d) => /^\d+$/.test(d));
  for (const id of ids) {
    const saved = JSON.parse(fs.readFileSync(path.join(dir, id, "container.json"), "utf8"));
    const ls = saved.local_state;
    const updates = { metadata: { ...ls.metadata, updated_at: Math.floor(Date.now() / 1000) }, context: ls.context, storage: ls.storage };
    const oldBody = readBody(ls.storage.object_store_path);
    console.log(`${id}: container -> object_store_path ${ls.storage.object_store_path} (name "${ls.metadata.name}"; body file exists: ${oldBody.file})`);
    if (!APPLY) continue;
    const r = (await http("POST", "/zsei/query", { query: { UpdateContainer: { container_id: Number(id), updates } }, session_token: "" })).json;
    console.log(`   UpdateContainer ->`, J(r).slice(0, 160));
    const now = await read(Number(id));
    if (now.rel !== ls.storage.object_store_path) die(4, `${id}: container still points at ${now.rel} after restore`);
    if (!(await health())) die(4, "/health not healthy after restore of " + id);
  }
  console.log(APPLY ? "restore applied" : "restore DRY RUN (add --apply)");
}

// ---- main --------------------------------------------------------------------------------------------------------------------------------
if (RESTORE) { await restore(RESTORE); process.exit(0); }

const plan = loadPlan();
const pf = preflight(plan);
console.log(`preflight: ${pf.ok ? "OK" : "FAIL"} — ${pf.why}`);
if (APPLY && !pf.ok) die(3, "the deployed methodology_create pipeline does not support remove_decision_rules/remove_heuristics (or cannot be verified). Deploy the rebuilt crate (commit 88d700a or later) and re-run. Nothing was written.");
if (!APPLY) console.log("DRY RUN — nothing will be written (use --apply after the preflight passes)\n");
if (!(await health())) die(2, "/health is not healthy");

const todo = plan.methodologies.filter((m) => !ONLY || ONLY.includes(m.methodology_id));
let applied = 0, anomalyStop = false;
for (const m of todo) {
  const id = m.methodology_id, p = m.payload;
  const cur = await read(id);
  const sim = simulate(cur.body, p);
  const nR = (p.remove_decision_rules ?? []).length, nH = (p.remove_heuristics ?? []).length;
  console.log(`${id} ${m.name}: remove ${nR} rule(s) + ${nH} heuristic(s) + ${(p.remove_principles ?? []).length} principle(s); add ${(p.add_decision_rules ?? []).length} rule(s) + ${(p.add_heuristics ?? []).length} heuristic(s) + ${(p.add_principles ?? []).length} principle(s)${p.name ? "; rename -> " + p.name : ""}${p.description ? "; new description" : ""}`
    + `\n     rules ${cur.body.decision_rules?.length ?? 0} -> ${sim.body.decision_rules.length}, heuristics ${cur.body.heuristics?.length ?? 0} -> ${sim.body.heuristics.length}  (body ${cur.rel})`);
  if (sim.anomalies.length) { console.error(`  ANOMALY: ${sim.anomalies.join("; ")}  (the body no longer matches the plan: it changed since the plan was made)`); anomalyStop = true; if (APPLY) break; else continue; }
  if (!APPLY) continue;

  fs.mkdirSync(path.join(OUT, String(id)), { recursive: true });
  fs.writeFileSync(path.join(OUT, String(id), "container.json"), JSON.stringify(cur.container, null, 1));
  fs.writeFileSync(path.join(OUT, String(id), "body.before.json"), JSON.stringify(cur.body, null, 1));
  fs.writeFileSync(path.join(OUT, String(id), "payload.json"), JSON.stringify(p, null, 1));
  const r = await pipeline(12, p);
  fs.writeFileSync(path.join(OUT, String(id), "result.json"), JSON.stringify(r, null, 1));
  const ok = findKey(r, "success"), err = findKey(r, "error");
  if (ok !== true) { console.error(`  Update FAILED: ${J(err ?? r).slice(0, 300)}`); anomalyStop = true; break; }
  const after = await read(id);
  fs.writeFileSync(path.join(OUT, String(id), "body.after.json"), JSON.stringify(after.body, null, 1));
  if (after.rel === cur.rel) { console.error("  ANOMALY: object_store_path did not change"); anomalyStop = true; break; }
  if (norm(after.body) !== norm(sim.body)) { console.error("  ANOMALY: the stored body differs from the simulation (see body.before/after.json)"); anomalyStop = true; break; }
  const removed = findKey(r, "removed");
  const nRemoved = (removed?.decision_rules?.length ?? 0) + (removed?.heuristics?.length ?? 0);
  const expectRemoved = cur.body.decision_rules.length + cur.body.heuristics.length - (sim.body.decision_rules.length + sim.body.heuristics.length) + (p.add_decision_rules?.length ?? 0) + (p.add_heuristics?.length ?? 0);
  if ((nR + nH) > 0 && nRemoved !== expectRemoved) { console.error(`  ANOMALY: pipeline reports ${nRemoved} removed entries, simulation expects ${expectRemoved}`); anomalyStop = true; break; }
  console.log(`  applied: ${cur.rel} -> ${after.rel}; verified identical to the simulation; before saved in ${path.join(OUT, String(id))}`);
  applied++;
  if (!(await health())) { console.error("  ANOMALY: /health not healthy after this Update"); anomalyStop = true; break; }
}
console.log(`\n${APPLY ? "applied" : "dry-run checked"}: ${APPLY ? applied : todo.length}/${todo.length} methodologies${anomalyStop ? " — STOPPED ON AN ANOMALY" : ""}`);
if (APPLY) console.log(`before-state saved in ${OUT}; restore a run with: node apply_rule_fixes.mjs --restore ${OUT} [--apply]`);
console.log("\ndelete candidates (NOT applied — operator decision; DeleteContainer is hard):");
for (const d of plan.delete_candidates_not_applied) console.log(`  ${d.id} ${d.name} — ${d.why}`);
process.exit(anomalyStop ? 5 : 0);
