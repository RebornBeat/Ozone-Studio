#!/usr/bin/env node
// KEYWORD MEASURE — read-only measurement of a store's keyword health (docs/KEYWORD_CAPTURE_AND_ML_GUIDE.md §1, §10 decision 5).
// Reproduces, on ANY host's data (local or the EU VPS), the numbers the keyword program was designed from:
//   * per container type: count, % with keywords, mean keywords, marker-shaped share, topics, sentence-like keywords;
//   * vocabulary size and the most frequent "keywords";
//   * which schema slots are actually used (embedding, learned_associations, access_frequency, keyword_meta, typed scope);
//   * (with --host) the substring-vs-token-vs-exact hit counts for flood terms, through /zsei/query.
// Usage:  node tools/keyword-measure/measure.mjs --data <zsei_data dir> [--host http://127.0.0.1:50051] [--label local|eu]
//         (EU: copy or mount its zsei_data read-only, and/or set --host to the tunnelled EU host)
// Writes docs/evidence/keyword-measure/<label>-<date>.json and prints a summary. NO writes to the store.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const args = Object.fromEntries(process.argv.slice(2).reduce((a, x, i, all) => (x.startsWith("--") ? [...a, [x.slice(2), all[i + 1] && !all[i + 1].startsWith("--") ? all[i + 1] : "true"]] : a), []));
const label = args.label ?? "local";
const here = path.dirname(fileURLToPath(import.meta.url));
const outDir = path.resolve(here, "../../docs/evidence/keyword-measure");

// shape rules identical to shared/contracts/keyword_model.rs (is_marker / is_sentence_like) — no word lists
const isMarker = (t) => { t = String(t).trim(); if (/\s/.test(t)) return false; const i = t.indexOf(":"); if (i < 1) return false; const ns = t.slice(0, i), rest = t.slice(i + 1); return /^[a-z][a-z0-9_-]*$/.test(ns) && rest.length > 0 && !rest.startsWith(":") && !rest.startsWith("//"); };
const isSentence = (t) => String(t).trim().split(/\s+/).filter(Boolean).length > 4;

const report = { label, date: new Date().toISOString(), data_dir: args.data ?? null, host: args.host ?? null };

if (args.data) {
  const dir = path.join(args.data, "local");
  const types = {};
  const vocab = new Map();
  const slots = { embedding: 0, learned_associations: 0, access_frequency_gt0: 0, keyword_meta: 0, typed_scope: 0 };
  let n = 0, unreadable = 0, sentenceLike = 0, slotsTotal = 0;
  for (const f of fs.readdirSync(dir).filter((x) => x.endsWith(".json"))) {
    let c; try { c = JSON.parse(fs.readFileSync(path.join(dir, f), "utf8")); } catch { unreadable++; continue; }
    // on-disk shape is flattened (metadata/context/hints at the top level); the API shape nests under local_state — accept both
    const md = c.metadata ?? c.local_state?.metadata ?? {}, ctx = c.context ?? c.local_state?.context ?? {}, hints = c.hints ?? c.local_state?.hints ?? {};
    n++;
    const t = md.container_type ?? "?";
    const e = (types[t] ??= { n: 0, with_keywords: 0, keyword_slots: 0, marker_slots: 0, with_topics: 0 });
    const kws = Array.isArray(ctx.keywords) ? ctx.keywords : [];
    e.n++; if (kws.length) e.with_keywords++; e.keyword_slots += kws.length; slotsTotal += kws.length;
    if (Array.isArray(ctx.topics) && ctx.topics.length) e.with_topics++;
    for (const k of kws) { if (isMarker(k)) e.marker_slots++; if (isSentence(k)) sentenceLike++; const key = String(k).toLowerCase(); vocab.set(key, (vocab.get(key) ?? 0) + 1); }
    if (ctx.embedding) slots.embedding++;
    if (Array.isArray(ctx.learned_associations) && ctx.learned_associations.length) slots.learned_associations++;
    if (hints.access_frequency > 0) slots.access_frequency_gt0++;
    if (ctx.keyword_meta && Object.keys(ctx.keyword_meta).length) slots.keyword_meta++;
    if (ctx.scope && (ctx.scope.workspace_id || ctx.scope.project_id || ctx.scope.global)) slots.typed_scope++;
  }
  const byType = Object.entries(types).sort((a, b) => b[1].n - a[1].n).map(([type, e]) => ({ type, ...e, pct_with_keywords: +(100 * e.with_keywords / e.n).toFixed(1), mean_keywords: +(e.keyword_slots / e.n).toFixed(2), marker_share_pct: e.keyword_slots ? +(100 * e.marker_slots / e.keyword_slots).toFixed(1) : 0 }));
  report.store = {
    containers: n, unreadable, keyword_slots: slotsTotal, distinct_vocabulary: vocab.size, sentence_like_keywords: sentenceLike, slots_in_use: slots,
    top_vocabulary: [...vocab.entries()].sort((a, b) => b[1] - a[1]).slice(0, 20),
    garbage_probe: Object.fromEntries(["you", "the", "it", "this", "and", "a"].map((w) => [w, vocab.get(w) ?? 0])),
    by_type: byType,
  };
  const hitLog = path.join(args.data, "keyword_hits.jsonl");
  report.hit_log = { exists: fs.existsSync(hitLog), bytes: fs.existsSync(hitLog) ? fs.statSync(hitLog).size : 0, judgment_samples_bytes: fs.existsSync(path.join(args.data, "judgment_samples.jsonl")) ? fs.statSync(path.join(args.data, "judgment_samples.jsonl")).size : 0, models_registry: fs.existsSync(path.join(args.data, "models/keyword/registry.json")) };
}

if (args.host) {
  const post = async (query) => { const r = await fetch(args.host + "/zsei/query", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ query, session_token: "" }) }); return (await r.json())?.result?.Containers ?? []; };
  const terms = (args.terms ?? "code,file,review,graph,data,explain,the,you").split(",");
  report.search_probe = {};
  for (const t of terms) {
    const row = {};
    for (const strat of ["scan", "token", "exact"]) {
      try { row[strat] = (await post({ SearchContainersByKeywords: { keywords: [t], container_type: null, strategy: strat } })).length; } catch (e) { row[strat] = `error: ${e.message}`; }
    }
    report.search_probe[t] = row;
  }
}

fs.mkdirSync(outDir, { recursive: true });
const file = path.join(outDir, `${label}-${new Date().toISOString().slice(0, 10)}.json`);
fs.writeFileSync(file, JSON.stringify(report, null, 2));
console.log(`keyword-measure [${label}] → ${file}`);
if (report.store) {
  console.log(`containers ${report.store.containers} · keyword slots ${report.store.keyword_slots} · vocabulary ${report.store.distinct_vocabulary} · sentence-like ${report.store.sentence_like_keywords}`);
  console.log("slots in use:", JSON.stringify(report.store.slots_in_use));
  for (const t of report.store.by_type.slice(0, 12)) console.log(`  ${t.type.padEnd(20)} n=${String(t.n).padStart(5)} withkw=${String(t.pct_with_keywords).padStart(5)}% mean=${String(t.mean_keywords).padStart(5)} markers=${String(t.marker_share_pct).padStart(5)}%`);
}
if (report.search_probe) for (const [t, r] of Object.entries(report.search_probe)) console.log(`  search "${t}": scan=${r.scan} token=${r.token} exact=${r.exact}`);
