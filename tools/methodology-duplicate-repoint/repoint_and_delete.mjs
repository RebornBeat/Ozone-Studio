#!/usr/bin/env node
// REPOINT the records that cite a merged duplicate methodology at its canonical, then HARD-DELETE the duplicate.
// Pairs (decided 2026-11-11; 40348 -> 40350 was already deleted 2026-11-11, docs/evidence/methodology-duplicate-delete/REPORT.md):
//   40394 -> 40396   cited by Blueprint 40397                      (pilot first, as the operator asked)
//   40252 -> 40305   cited by Blueprint 40255 and Derived (AMT) 40253
// RUNS ON THE HOST THAT OWNS THE DATA DIR (the EU, in the APPLY phase after the build gate; local first, then EU). DRY RUN BY DEFAULT.
//
//   node repoint_and_delete.mjs --data /opt/ozone-studio/zsei_data                    dry run: checks + the exact plan, writes plan.json, changes NOTHING
//   node repoint_and_delete.mjs --data /opt/ozone-studio/zsei_data --apply            apply pair by pair (40394 first), verifying after each; STOPS at the first failure
//   options: --host http://127.0.0.1:50051   --pair 40394|40252   --out <dir>   (session token, if the host demands one: env OZONE_SESSION_TOKEN; never printed)
//
// FAIL CLOSED. For a pair the script STOPS (touches nothing for that pair, exit code 2) when ANY of these fails:
//   1. /health is not 200, the duplicate or the canonical is not a Methodology, or the before-copies of the 2026-11-11 delete run are missing;
//   2. the canonical LOST content: every decision rule / heuristic of the canonical's saved before-copy must still be in the canonical's
//      CURRENT body (rules only ever get added to a canonical; a missing one means someone removed it after the coverage proof);
//   3. the live citers of the duplicate (containers whose local state names it, found by scanning <data>/local/*.json) are not EXACTLY the expected
//      set, or a body file of ANOTHER record under blueprints/ amt/ methodologies_created/ names it (a live reference we did not plan for);
//   4. after the planned rewrite a numeric reference to the duplicate would remain in a citer (context or body).
// Only `methodologies` / `methodology_ids` integer arrays are rewritten (dup -> canonical, de-duplicated, order kept). HISTORY is never rewritten:
// runs/, capability_gaps.json, hit/judgment logs, host.log, rollback snapshots and ledgers keep naming the old id (logs are never rotated).
// Every change is preceded by a before-copy under <out>/before/ and recorded in <out>/applied.json; restore_repoint.mjs undoes the repoint.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";

const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf("--" + n); return i >= 0 && argv[i + 1] && !argv[i + 1].startsWith("--") ? argv[i + 1] : d; };
const APPLY = argv.includes("--apply");
const DATA = opt("data", process.env.OZONE_DATA ?? "");
const HOST = opt("host", process.env.OZONE_HOST ?? "http://127.0.0.1:50051");
const ONLY = opt("pair", null);
const here = path.dirname(fileURLToPath(import.meta.url));
const REPO = path.resolve(here, "..", "..");
const BEFORE_DEL = path.join(REPO, "docs", "evidence", "methodology-duplicate-delete", "before");
const stamp = new Date().toISOString().replace(/[-:]/g, "").slice(0, 15) + "Z";
const OUT = opt("out", path.join(REPO, "docs", "evidence", "methodology-duplicate-repoint", `run-${stamp}${APPLY ? "" : "-dry"}`));
const TOKEN = process.env.OZONE_SESSION_TOKEN ?? "";

// citers = the records the 2026-11-11 citation scan found (eu_container_citations.json); anything else that cites the duplicate STOPS the pair
const PAIRS = [
  { dup: 40394, canon: 40396, citers: [40397] },
  { dup: 40252, canon: 40305, citers: [40255, 40253] },
].filter((p) => !ONLY || String(p.dup) === ONLY);

if (!DATA) { console.error("--data <zsei_data dir> (or OZONE_DATA) is required"); process.exit(64); }
if (!fs.existsSync(path.join(DATA, "local"))) { console.error(`${DATA}/local not found — run this ON the host that owns the data dir`); process.exit(64); }
fs.mkdirSync(path.join(OUT, "before"), { recursive: true });

const log = [];
const say = (s) => { console.log(s); log.push(s); };
const stop = (pair, why) => { say(`STOP pair ${pair.dup}->${pair.canon}: ${why}`); return { ok: false, why }; };

async function q(query) {
  const r = await fetch(HOST + "/zsei/query", { method: "POST", headers: { "content-type": "application/json", ...(TOKEN ? { authorization: `Bearer ${TOKEN}` } : {}) },
    body: JSON.stringify({ query, session_token: TOKEN }) });
  const text = await r.text();
  let j = null; try { j = JSON.parse(text); } catch { /* keep text */ }
  return { status: r.status, json: j, text };
}
const getContainer = async (id) => { const r = await q({ GetContainer: { container_id: id } }); return r.json?.result?.Container ?? null; };
const health = async () => { try { return (await fetch(HOST + "/health")).status; } catch { return 0; } };
const readJson = (f) => JSON.parse(fs.readFileSync(f, "utf8"));
const writeAtomic = (f, s) => { const t = f + `.tmp-${process.pid}`; fs.writeFileSync(t, s); fs.renameSync(t, f); };
const norm = (s) => String(s).replace(/\s+/g, " ").trim().toLowerCase();

// the strings of a methodology body's decision rules and heuristics (rules may be strings or {text|rule|description} objects)
function ruleTexts(body) {
  const out = [];
  for (const k of ["decision_rules", "heuristics"]) for (const v of body?.[k] ?? []) {
    if (typeof v === "string") out.push(v);
    else if (v && typeof v === "object") out.push(v.text ?? v.rule ?? v.description ?? JSON.stringify(v));
  }
  return out.map(norm).filter(Boolean);
}
const bodyPath = (c) => c?.local_state?.storage?.object_store_path ? path.join(DATA, c.local_state.storage.object_store_path) : null;
function currentBody(c) {
  const p = bodyPath(c);
  if (!p || !fs.existsSync(p)) return null;
  try { return readJson(p); } catch { return null; }
}
// before-copy body of a container saved by the 2026-11-11 run
function savedBody(id) {
  const c = readJson(path.join(BEFORE_DEL, `container_${id}.json`)).result.Container;
  const f = path.join(BEFORE_DEL, "bodies", path.basename(c.local_state.storage.object_store_path));
  return fs.existsSync(f) ? readJson(f) : null;
}

// ---- rewrite helpers ------------------------------------------------------------------------------------------------------------------
const idKeys = new Set(["methodologies", "methodology_ids"]);
function repointJson(node, dup, canon, stats) {
  if (Array.isArray(node)) return node.map((x) => repointJson(x, dup, canon, stats));
  if (node && typeof node === "object") {
    const o = {};
    for (const [k, v] of Object.entries(node)) {
      if (idKeys.has(k) && Array.isArray(v) && v.every((x) => Number.isInteger(x))) {
        if (v.includes(dup)) {
          const seen = new Set(); const nv = [];
          for (const x of v) { const y = x === dup ? canon : x; if (!seen.has(y)) { seen.add(y); nv.push(y); } }
          stats.changed += 1; o[k] = nv;
        } else o[k] = v;
      } else o[k] = repointJson(v, dup, canon, stats);
    }
    return o;
  }
  return node;
}
// numeric occurrences of `id` anywhere (as a number, not inside a string) — what must be zero after the rewrite
function numericHits(node, id, acc = []) {
  if (Array.isArray(node)) node.forEach((x) => numericHits(x, id, acc));
  else if (node && typeof node === "object") Object.values(node).forEach((x) => numericHits(x, id, acc));
  else if (node === id) acc.push(node);
  return acc;
}
const textRe = (id) => new RegExp(`(?<![\\d.])${id}(?![\\d.])`);

// ---- one pair ---------------------------------------------------------------------------------------------------------------------------
async function runPair(pair) {
  say(`\n== pair ${pair.dup} -> ${pair.canon} (citers expected: ${pair.citers.join(", ")}) ==`);
  const plan = { pair, checks: {}, rewrites: [] };
  if ((await health()) !== 200) return { ...stop(pair, "/health is not 200"), plan };
  const dupC = await getContainer(pair.dup);
  const canC = await getContainer(pair.canon);
  if (!canC) return { ...stop(pair, `canonical ${pair.canon} not found`), plan };
  if (!dupC) { say(`  duplicate ${pair.dup} already gone — nothing to do`); return { ok: true, plan, already: true }; }
  for (const [id, c] of [[pair.dup, dupC], [pair.canon, canC]]) {
    if (c.local_state?.metadata?.container_type !== "Methodology") return { ...stop(pair, `${id} is not a Methodology`), plan };
  }
  for (const id of [pair.dup, pair.canon]) {
    if (!fs.existsSync(path.join(BEFORE_DEL, `container_${id}.json`))) return { ...stop(pair, `before-copy of ${id} missing under ${BEFORE_DEL}`), plan };
  }
  plan.checks.preflight = "ok";

  // 2. canonical lost nothing since the coverage proof
  const savedCanon = ruleTexts(savedBody(pair.canon));
  const nowCanon = new Set(ruleTexts(currentBody(canC)));
  const lost = savedCanon.filter((t) => !nowCanon.has(t));
  if (!savedCanon.length) return { ...stop(pair, "saved canonical body has no rules to compare (before-copy unreadable)"), plan };
  if (!nowCanon.size) return { ...stop(pair, "current canonical body unreadable or empty"), plan };
  if (lost.length) return { ...stop(pair, `canonical ${pair.canon} lost ${lost.length} rule(s)/heuristic(s) since the coverage proof, e.g. "${lost[0].slice(0, 90)}"`), plan };
  plan.checks.canonical_monotone = { saved: savedCanon.length, current: nowCanon.size, lost: 0 };
  say(`  canonical keeps all ${savedCanon.length} saved rules/heuristics (now ${nowCanon.size})`);

  // 3. citation discovery (scan the persisted local state of EVERY container)
  const re = textRe(pair.dup);
  const localDir = path.join(DATA, "local");
  const citers = [];
  for (const f of fs.readdirSync(localDir)) {
    const m = /^(\d+)\.json$/.exec(f); if (!m) continue;
    const id = Number(m[1]); if (id === pair.dup) continue;
    let s; try { s = fs.readFileSync(path.join(localDir, f), "utf8"); } catch { continue; }
    if (re.test(s)) citers.push(id);
  }
  citers.sort((a, b) => a - b);
  const expected = [...pair.citers].sort((a, b) => a - b);
  if (JSON.stringify(citers) !== JSON.stringify(expected)) return { ...stop(pair, `live citers ${JSON.stringify(citers)} != expected ${JSON.stringify(expected)} — decide before touching anything`), plan };
  plan.checks.citers = citers;

  const citerC = {};
  const bodyOf = new Set();
  for (const id of citers) {
    const c = await getContainer(id);
    if (!c) return { ...stop(pair, `citer ${id} unreadable through the API`), plan };
    citerC[id] = c; const bp = bodyPath(c); if (bp) bodyOf.add(path.resolve(bp));
  }
  // body files of OTHER records that name the duplicate = an unplanned live reference
  const stray = [];
  for (const dir of ["blueprints", "amt", "methodologies_created"]) {
    const d = path.join(DATA, dir); if (!fs.existsSync(d)) continue;
    for (const f of fs.readdirSync(d)) {
      const p = path.join(d, f); if (!fs.statSync(p).isFile() || bodyOf.has(path.resolve(p))) continue;
      if (p === bodyPath(dupC)) continue;
      let s; try { s = fs.readFileSync(p, "utf8"); } catch { continue; }
      if (re.test(s)) stray.push(path.join(dir, f));
    }
  }
  if (stray.length) return { ...stop(pair, `body files of other records name ${pair.dup}: ${stray.join(", ")}`), plan };
  plan.checks.stray_body_refs = 0;

  // 4. the planned rewrite, and what would remain
  for (const id of citers) {
    const c = citerC[id];
    const ctxStats = { changed: 0 };
    const newCtx = repointJson(c.local_state.context, pair.dup, pair.canon, ctxStats);
    const left = numericHits(newCtx, pair.dup).length;
    const bp = bodyPath(c); let bodyPlan = null;
    if (bp && fs.existsSync(bp)) {
      const body = readJson(bp); const st = { changed: 0 };
      const nb = repointJson(body, pair.dup, pair.canon, st);
      const bleft = numericHits(nb, pair.dup).length;
      bodyPlan = { file: path.relative(DATA, bp), arrays_changed: st.changed, numeric_left: bleft, newBody: nb };
      if (bleft) return { ...stop(pair, `after the rewrite ${path.relative(DATA, bp)} still holds ${bleft} numeric reference(s) to ${pair.dup}`), plan };
    }
    if (left) return { ...stop(pair, `after the rewrite container ${id} context still holds ${left} numeric reference(s) to ${pair.dup}`), plan };
    plan.rewrites.push({ citer: id, name: c.local_state.metadata.name, context_arrays_changed: ctxStats.changed, newCtx,
      body: bodyPlan ? { file: bodyPlan.file, arrays_changed: bodyPlan.arrays_changed } : null, _newBody: bodyPlan?.newBody ?? null });
    say(`  citer ${id} "${c.local_state.metadata.name}": context arrays ${ctxStats.changed}, body ${bodyPlan ? `${bodyPlan.file} arrays ${bodyPlan.arrays_changed}` : "none"}`);
  }
  say(`  then DeleteContainer ${pair.dup} ("${dupC.local_state.metadata.name}", ${(dupC.global_state.child_ids ?? []).length} children)`);
  if ((dupC.global_state.child_ids ?? []).length) return { ...stop(pair, `${pair.dup} has children — DeleteContainer is recursive`), plan };
  if (!APPLY) return { ok: true, plan, dry: true };

  // ---- APPLY --------------------------------------------------------------------------------------------------------------------------
  const pdir = path.join(OUT, "before", String(pair.dup)); fs.mkdirSync(path.join(pdir, "bodies"), { recursive: true });
  const saveCont = async (id, c) => {
    writeAtomic(path.join(pdir, `container_${id}.json`), JSON.stringify({ result: { Container: c } }, null, 1));
    const bp = bodyPath(c); if (bp && fs.existsSync(bp)) fs.copyFileSync(bp, path.join(pdir, "bodies", `${id}__${path.basename(bp)}`));
    const h = await q({ GetVersionHistory: { container_id: id } });
    writeAtomic(path.join(pdir, `history_${id}.json`), h.text);
  };
  await saveCont(pair.dup, dupC);
  for (const id of citers) await saveCont(id, citerC[id]);
  const applied = { pair, at: new Date().toISOString(), citers: [], deleted: false };
  for (const rw of plan.rewrites) {
    const c = citerC[rw.citer];
    const upd = await q({ UpdateContainer: { container_id: rw.citer, updates: { context: rw.newCtx } } });
    if (upd.status !== 200 || !/Success/.test(upd.text)) {
      applied.failed = { citer: rw.citer, step: "UpdateContainer", detail: upd.text.slice(0, 300) };
      writeAtomic(path.join(OUT, "applied.json"), JSON.stringify(applied, null, 1));
      return { ...stop(pair, `UpdateContainer ${rw.citer} failed: ${upd.text.slice(0, 200)} (earlier citers of this pair may already be repointed — restore_repoint.mjs ${OUT})`), plan, applied };
    }
    if (rw._newBody) writeAtomic(path.join(DATA, rw.body.file), JSON.stringify(rw._newBody, null, 2));
    const after = await getContainer(rw.citer);
    const ids = after?.local_state?.context?.methodologies ?? [];
    if (ids.includes(pair.dup) || !ids.includes(pair.canon)) {
      applied.failed = { citer: rw.citer, step: "verify", detail: JSON.stringify(ids) };
      writeAtomic(path.join(OUT, "applied.json"), JSON.stringify(applied, null, 1));
      return { ...stop(pair, `verify after repoint of ${rw.citer} failed (methodologies=${JSON.stringify(ids)})`), plan, applied };
    }
    applied.citers.push({ id: rw.citer, context_arrays: rw.context_arrays_changed, body: rw.body });
    say(`  repointed ${rw.citer} (version ${c.global_state.version} -> ${after.global_state.version})`);
  }
  // nothing may cite the duplicate any more (re-scan the persisted state), then delete
  const again = [];
  for (const f of fs.readdirSync(localDir)) { const m = /^(\d+)\.json$/.exec(f); if (!m || Number(m[1]) === pair.dup) continue;
    try { if (re.test(fs.readFileSync(path.join(localDir, f), "utf8"))) again.push(Number(m[1])); } catch { /* skip */ } }
  if (again.length) { applied.failed = { step: "rescan", detail: again }; writeAtomic(path.join(OUT, "applied.json"), JSON.stringify(applied, null, 1));
    return { ...stop(pair, `after the repoint ${JSON.stringify(again)} still cite ${pair.dup} — NOT deleting`), plan, applied }; }
  const del = await q({ DeleteContainer: { container_id: pair.dup } });
  const gone = !(await getContainer(pair.dup));
  const canAfter = await getContainer(pair.canon);
  const h = await health();
  applied.deleted = gone; applied.delete_response = del.text.slice(0, 200); applied.health_after = h; applied.canonical_version_after = canAfter?.global_state?.version ?? null;
  writeAtomic(path.join(OUT, "applied.json"), JSON.stringify(applied, null, 1));
  if (!gone || !canAfter || h !== 200) return { ...stop(pair, `post-delete verification failed (gone=${gone}, canonical=${!!canAfter}, health=${h})`), plan, applied };
  say(`  DELETED ${pair.dup}; ${pair.canon} loads (v${canAfter.global_state.version}); /health ${h}`);
  try { const j = execFileSync("journalctl", ["-u", "ozone-studio", "--since", "-2min", "--no-pager"], { encoding: "utf8", timeout: 15000 });
    const errs = j.split("\n").filter((l) => /ERROR|panic/.test(l)); say(`  journal (last 2 min): ${errs.length} ERROR/panic line(s)`); applied.journal_errors = errs.length; } catch { say("  (journalctl not available here — check the journal by hand)"); }
  return { ok: true, plan, applied };
}

// ---- main -------------------------------------------------------------------------------------------------------------------------------
say(`mode: ${APPLY ? "APPLY" : "DRY RUN (nothing is changed)"}; data=${DATA}; host=${HOST}; out=${OUT}`);
let exit = 0; const results = [];
for (const pair of PAIRS) {
  const r = await runPair(pair);
  results.push({ pair: `${pair.dup}->${pair.canon}`, ok: r.ok, why: r.why ?? null, dry: !!r.dry, already: !!r.already, plan: { ...r.plan, rewrites: (r.plan?.rewrites ?? []).map(({ _newBody, newCtx, ...x }) => x) } });
  if (!r.ok) { exit = 2; say("stopping at the first failing pair (the next pair is not attempted)"); break; }
}
writeAtomic(path.join(OUT, APPLY ? "result.json" : "plan.json"), JSON.stringify({ at: new Date().toISOString(), apply: APPLY, results }, null, 1));
writeAtomic(path.join(OUT, "run.log"), log.join("\n") + "\n");
console.log(`\nrecord: ${OUT}`);
process.exit(exit);
