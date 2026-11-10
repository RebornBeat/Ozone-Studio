#!/usr/bin/env node
// FORK V2 apply — usage (on the EU): node apply.mjs <decisions.json> <outdir> --ids 1,2 | --all
// For each container: save the full before-state, apply yes (validated:true) / no (removed + keyword_meta validated:false, as backlog_cleanup does),
// leave = untouched; re-read and verify; stop on the first anomaly.
import fs from "node:fs";
import path from "node:path";
import { http } from "/opt/ozone-build/tools/e2e/lib.mjs";
const [decFile, outDir] = [process.argv[2], process.argv[3]];
const idsArg = process.argv.indexOf("--ids");
const only = idsArg > 0 ? new Set(process.argv[idsArg + 1].split(",").map(Number)) : null;
const decisions = JSON.parse(fs.readFileSync(decFile, "utf8"));
fs.mkdirSync(path.join(outDir, "before"), { recursive: true });
fs.mkdirSync(path.join(outDir, "after"), { recursive: true });
const by = new Map();
for (const r of decisions) { if (only && !only.has(r.container_id)) continue; if (!by.has(r.container_id)) by.set(r.container_id, []); by.get(r.container_id).push(r); }
const get = async (id) => (await http("POST", "/zsei/query", { query: { GetContainer: { container_id: id } }, session_token: "" })).json?.result?.Container;
const health = async () => (await http("GET", "/health")).status;
const ledger = [];
let ok = 0, anomalies = 0, skipped = 0;
for (const [id, rows] of by) {
  const c = await get(id);
  if (!c) { console.log("ANOMALY: container not readable", id); anomalies++; break; }
  const ctx = c.local_state.context;
  ctx.keyword_meta ??= {};
  const decision = new Map(rows.map((r) => [r.term, r.decision]));
  // already applied by an earlier run (idempotent re-run): every yes is validated, every no is gone and rejected -> skip, never overwrite its before-state
  const applied = rows.every((r) => r.decision === "leave" ? true : r.decision === "yes" ? ctx.keywords.includes(r.term) && (ctx.keyword_meta[r.term] ?? {}).validated === true : !ctx.keywords.includes(r.term) && (ctx.keyword_meta[r.term] ?? {}).validated === false);
  if (applied && rows.some((r) => r.decision !== "leave")) { skipped++; continue; }
  fs.writeFileSync(path.join(outDir, "before", `container_${id}.json`), JSON.stringify(c, null, 1));
  // every decided term must exist now as an UNVALIDATED term (never act on something that changed under us)
  const stale = rows.filter((r) => !ctx.keywords.includes(r.term) || (ctx.keyword_meta[r.term] ?? {}).validated != null);
  if (stale.length) { console.log("ANOMALY: stale decision(s) for", id, stale.map((r) => r.term).slice(0, 5)); anomalies++; break; }
  const prevMeta = {};
  for (const r of rows) prevMeta[r.term] = ctx.keyword_meta[r.term] ?? null;
  const kept = [];
  for (const k of ctx.keywords) {
    const d = decision.get(k);
    if (d === "no") { ctx.keyword_meta[k] = { ...(ctx.keyword_meta[k] ?? {}), source: (ctx.keyword_meta[k] ?? {}).source || "extracted", validated: false }; continue; }
    if (d === "yes") ctx.keyword_meta[k] = { source: "", hits: {}, ...(ctx.keyword_meta[k] ?? {}), validated: true };
    kept.push(k);
  }
  ctx.keywords = kept;
  const r = await http("POST", "/zsei/query", { query: { UpdateContainer: { container_id: id, updates: { metadata: null, context: ctx, storage: null, hints: null } } }, session_token: "" });
  const a = await get(id);
  if (r.status !== 200 || !a) { console.log("ANOMALY: update status", r.status, id); anomalies++; break; }
  fs.writeFileSync(path.join(outDir, "after", `container_${id}.json`), JSON.stringify(a, null, 1));
  const actx = a.local_state.context;
  let bad = [];
  for (const [t, d] of decision) {
    const present = actx.keywords.includes(t), v = (actx.keyword_meta[t] ?? {}).validated;
    if (d === "yes" && !(present && v === true)) bad.push(`${t}: yes but present=${present} validated=${v}`);
    if (d === "no" && (present || v !== false)) bad.push(`${t}: no but present=${present} validated=${v}`);
    if (d === "leave" && !(present && (v === null || v === undefined))) bad.push(`${t}: leave changed present=${present} validated=${v}`);
  }
  // nothing outside the decided terms may have changed in keywords
  const before = c.local_state.context.keywords ?? [];
  const expect = before.filter((k) => decision.get(k) !== "no");
  if (JSON.stringify(expect) !== JSON.stringify(actx.keywords)) bad.push("keyword list differs from the expected result");
  if (bad.length) { console.log("ANOMALY: verify failed", id, bad.slice(0, 6)); anomalies++; break; }
  for (const x of rows) ledger.push({ ...x, previous_meta: prevMeta[x.term] });
  ok++;
  if (ok % 10 === 0 && (await health()) !== 200) { console.log("ANOMALY: health not 200 after", ok); anomalies++; break; }
}
fs.appendFileSync(path.join(outDir, "applied_ledger.jsonl"), ledger.map((x) => JSON.stringify(x)).join("\n") + (ledger.length ? "\n" : ""));
console.log(`containers ok ${ok}, skipped(already applied) ${skipped}, anomalies ${anomalies}, health ${await health()}`);
