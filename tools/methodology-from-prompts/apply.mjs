// Apply the doctrine methodologies drafted from the operator's prompts (docs/evidence/methodology-from-prompts/drafts/*.json).
//
//   node apply.mjs                         dry run (default): resolves ids, checks names, writes nothing
//   node apply.mjs --apply                 creates / updates through the real pipeline 12 (methodology_create) route
//   node apply.mjs --only <slug,slug>      restrict to some drafts
//   node apply.mjs --include-unconfirmed   also take drafts whose status is "needs_confirm" (operator has not confirmed them)
//
// Run it on the host that serves /pipeline/execute (the EU, from /opt/ozone-build/tools/..., or locally). It uses the same real
// route every other methodology write uses; no mock, no direct file writes. The OpenRouter key is never read or printed.
//
// Fail closed:
//   * the deployed methodology_create binary must be located and must carry the new crate's `remove_decision_rules` field
//     (the crate that has the never-empty guard and the versioned Update); an old or unlocatable crate stops an --apply run;
//   * a Create is refused if a methodology with the same name already exists (merge instead: add a draft of kind "update");
//   * an Update is refused unless its target resolves to exactly one methodology by name;
//   * every target's container is saved (before copy) before anything is written, and a rollback file lists what was created.
// Applying is a data mutation on the operator's store: run it only when the operator has said to.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const flag = (n) => args.includes(n);
const opt = (n) => { const i = args.indexOf(n); return i >= 0 ? args[i + 1] : null; };
const APPLY = flag("--apply");
const INCLUDE_UNCONFIRMED = flag("--include-unconfirmed");
const ONLY = (opt("--only") || "").split(",").filter(Boolean);
const DRAFTS = path.resolve(opt("--drafts") || path.join(HERE, "../../docs/evidence/methodology-from-prompts/drafts"));
const LIB = opt("--lib") || process.env.OZONE_E2E_LIB || (fs.existsSync("/opt/ozone-build/tools/e2e/lib.mjs") ? "/opt/ozone-build/tools/e2e/lib.mjs" : path.join(HERE, "../e2e/lib.mjs"));
const { http, pipeline } = await import(LIB);
const METHODOLOGY_ROOT = 2;

const stamp = new Date().toISOString().replace(/[:.]/g, "-");
const BEFORE = path.resolve(opt("--before-dir") || path.join(HERE, "../../docs/evidence/methodology-from-prompts/before-" + stamp));
const LEDGER = path.join(HERE, "../../docs/evidence/methodology-from-prompts/applied.json");

const die = (m) => { console.error("STOP: " + m); process.exit(2); };
const getContainer = async (id) => (await http("POST", "/zsei/query", { query: { GetContainer: { container_id: id } }, session_token: "" })).json?.result?.Container;

// 1. crate check (fail closed) ------------------------------------------------------------------------------------------
function locateCrate() {
  const c = [process.env.OZONE_PIPELINE_BIN, "/opt/ozone-build/target/pipelines/release/methodology_create", path.resolve(HERE, "../../target/pipelines/release/methodology_create"),
    "/opt/ozone-studio/target/pipelines/release/methodology_create"].filter(Boolean);
  return c.find((p) => fs.existsSync(p)) || null;
}
const crate = locateCrate();
let crateOk = false, crateWhy = "";
if (!crate) crateWhy = "methodology_create binary not found (set OZONE_PIPELINE_BIN)";
else {
  const bytes = fs.readFileSync(crate);
  crateOk = bytes.includes(Buffer.from("remove_decision_rules"));
  crateWhy = crateOk ? `${crate} carries remove_decision_rules (new crate)` : `${crate} lacks remove_decision_rules: the crate is OLD (rebuild first)`;
}
console.log("crate:", crateWhy);
if (APPLY && !crateOk) die("refusing to apply: " + crateWhy);

// 2. drafts ------------------------------------------------------------------------------------------------------------------
const files = fs.readdirSync(DRAFTS).filter((f) => /^\d+_.*\.json$/.test(f)).sort();
let drafts = files.map((f) => ({ file: f, ...JSON.parse(fs.readFileSync(path.join(DRAFTS, f), "utf8")) }));
if (ONLY.length) drafts = drafts.filter((d) => ONLY.includes(d.slug));
const skippedUnconfirmed = drafts.filter((d) => d.status === "needs_confirm" && !INCLUDE_UNCONFIRMED);
drafts = drafts.filter((d) => d.status === "ready" || INCLUDE_UNCONFIRMED);
console.log(`drafts: ${drafts.length} selected; ${skippedUnconfirmed.length} held (needs_confirm): ${skippedUnconfirmed.map((d) => d.slug).join(", ") || "-"}`);

// 3. name -> id map of the live methodology store ------------------------------------------------------------------------------
const root = await getContainer(METHODOLOGY_ROOT);
if (!root) die("cannot read the methodology root container " + METHODOLOGY_ROOT + " (host down or auth failed)");
const names = new Map(); // lower name -> [ids]
const snapshot = [];
for (const id of root.global_state?.child_ids ?? []) {
  const c = await getContainer(id);
  const n = c?.local_state?.metadata?.name;
  if (!n) continue;
  snapshot.push({ id, name: n, container_type: c.local_state.metadata.container_type });
  const k = n.trim().toLowerCase();
  names.set(k, [...(names.get(k) ?? []), id]);
}
console.log(`store: ${snapshot.length} methodology containers read`);

// 4. plan ---------------------------------------------------------------------------------------------------------------------
const applied = fs.existsSync(LEDGER) ? JSON.parse(fs.readFileSync(LEDGER, "utf8")) : {};
const plan = [];
for (const d of drafts) {
  if (applied[d.slug]) { plan.push({ d, verdict: "skip", why: `already applied (${JSON.stringify(applied[d.slug])})` }); continue; }
  if (d.kind === "create") {
    const same = names.get(d.payload.name.trim().toLowerCase()) ?? [];
    plan.push(same.length ? { d, verdict: "refuse", why: `a methodology named "${d.payload.name}" exists: ${same.join(",")} — merge as an update draft instead` } : { d, verdict: "create" });
  } else {
    const ids = names.get((d.target_name || "").trim().toLowerCase()) ?? [];
    plan.push(ids.length === 1 ? { d, verdict: "update", id: ids[0] } : { d, verdict: "refuse", why: `target "${d.target_name}" resolves to ${ids.length} methodologies (${ids.join(",")}); need exactly one` });
  }
}
for (const p of plan) console.log(`${p.verdict.toUpperCase().padEnd(7)} ${p.d.slug}${p.id ? " -> " + p.id : ""}${p.why ? "  (" + p.why + ")" : ""}`);
if (!APPLY) { console.log("\ndry run: nothing written. Re-run with --apply (only when the operator has said to)."); process.exit(0); }

// 5. before-copies, then write ---------------------------------------------------------------------------------------------------
fs.mkdirSync(BEFORE, { recursive: true });
fs.writeFileSync(path.join(BEFORE, "store_snapshot.json"), JSON.stringify(snapshot, null, 1));
for (const p of plan.filter((x) => x.verdict === "update")) {
  const c = await getContainer(p.id);
  if (!c) die("cannot read target " + p.id);
  fs.writeFileSync(path.join(BEFORE, `container_${p.id}.json`), JSON.stringify(c, null, 1));
  const op = c.local_state?.storage?.object_store_path;
  if (op) for (const base of [process.env.OZONE_ZSEI_DATA_DIR, "/opt/ozone-studio/zsei_data", "/opt/ozone-studio", "zsei_data"].filter(Boolean)) {
    const f = path.join(base, op);
    if (fs.existsSync(f)) { fs.writeFileSync(path.join(BEFORE, `body_${p.id}.json`), fs.readFileSync(f)); break; }
  }
}
const rollback = { created: [], updated: [] };
const out = (r) => r?.output ?? r?.data ?? r;
for (const p of plan.filter((x) => x.verdict === "create" || x.verdict === "update")) {
  const body = p.verdict === "update" ? { ...p.d.payload, methodology_id: p.id } : p.d.payload;
  const r = out(await pipeline(12, body));
  const ok = r?.success === true;
  console.log(p.d.slug, ok ? "OK" : "FAILED", JSON.stringify(r).slice(0, 300));
  if (!ok) { fs.writeFileSync(path.join(BEFORE, "rollback.json"), JSON.stringify(rollback, null, 1)); die(`stopped at ${p.d.slug}; earlier writes are listed in ${BEFORE}/rollback.json`); }
  const id = r.methodology_id ?? p.id;
  (p.verdict === "create" ? rollback.created : rollback.updated).push({ slug: p.d.slug, id, version: r.version ?? null });
  const c = await getContainer(id);
  if (!c || c.local_state?.metadata?.container_type !== "Methodology") die(`${p.d.slug}: container ${id} does not read back as a Methodology`);
  applied[p.d.slug] = { id, kind: p.verdict, version: r.version ?? null, at: new Date().toISOString() };
  fs.writeFileSync(LEDGER, JSON.stringify(applied, null, 1));
}
fs.writeFileSync(path.join(BEFORE, "rollback.json"), JSON.stringify({ ...rollback, how: "created: pipeline 12 {action:'Delete', methodology_id} or DeleteContainer; updated: point the container's storage.object_store_path back at body_<id>.json's original path (before copy in this folder)" }, null, 1));
console.log(`done: ${rollback.created.length} created, ${rollback.updated.length} updated. Before-copies and rollback.json in ${BEFORE}`);
