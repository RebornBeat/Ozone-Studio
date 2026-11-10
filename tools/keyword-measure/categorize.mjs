#!/usr/bin/env node
// KEYWORD CATEGORIZE — read-only. Breaks the keywords of CONTENT containers down by SHAPE, SOURCE, VALIDATION and HITS, and measures
// redundancy and genericness from the DATA ITSELF (no word lists, no stoplist). docs/KEYWORD_CAPTURE_AND_ML_GUIDE.md §18.
// Usage: node tools/keyword-measure/categorize.mjs --data <zsei_data dir> --label local|eu     (NO writes to the store)
// "Bookkeeping" container types (system events, derived rows, tool results) are reported separately and excluded from the content view.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const args = Object.fromEntries(process.argv.slice(2).reduce((a, x, i, all) => (x.startsWith("--") ? [...a, [x.slice(2), all[i + 1] && !all[i + 1].startsWith("--") ? all[i + 1] : true]] : a), []));
const label = args.label ?? "local";
const here = path.dirname(fileURLToPath(import.meta.url));
const BOOKKEEPING = new Set(["CoordinationEvent", "Derived", "McpResult"]);
const words = (t) => String(t).trim().split(/\s+/).filter(Boolean);
const isMarker = (t) => { t = String(t).trim(); if (/\s/.test(t)) return false; const i = t.indexOf(":"); return i >= 1 && i < t.length - 1; };
const digitShare = (t) => { const s = String(t); const d = (s.match(/\d/g) ?? []).length; return s.length ? d / s.length : 0; };
const isIdentifier = (t) => /^[a-z0-9]+([_-][a-z0-9]+)+$/i.test(t) || /^[a-z]+[A-Z][A-Za-z]+$/.test(t); // snake/kebab/camel single tokens
const shape = (t) => {
  const w = words(t).length;
  if (isMarker(t)) return "marker";
  if (digitShare(t) >= 0.5) return "numeric-id";
  if (w > 4) return "sentence-like";
  if (w >= 2) return `phrase-${w}`;
  if (isIdentifier(t)) return "identifier";
  return "single-word";
};
const stem = (t) => String(t).toLowerCase().replace(/(ies)$/, "y").replace(/(es|s)$/, "");

const dir = path.join(args.data, "local");
const content = { n: 0, slots: 0 };
const types = {};
const df = new Map(); // term -> number of content containers carrying it
let all = 0;
for (const f of fs.readdirSync(dir).filter((x) => x.endsWith(".json"))) {
  let c; try { c = JSON.parse(fs.readFileSync(path.join(dir, f), "utf8")); } catch { continue; }
  const md = c.metadata ?? c.local_state?.metadata ?? {}, ctx = c.context ?? c.local_state?.context ?? {};
  const t = md.container_type ?? "?";
  all++;
  const kws = (Array.isArray(ctx.keywords) ? ctx.keywords : []).map(String);
  const meta = ctx.keyword_meta ?? {};
  const e = (types[t] ??= { n: 0, slots: 0, shapes: {}, sources: {}, validated: { yes: 0, no: 0, unknown: 0 }, hit_terms: 0, dup_case: 0, dup_stem: 0, in_name: 0, per: [], terms: new Map(), bookkeeping: BOOKKEEPING.has(t) });
  e.n++; e.slots += kws.length; e.per.push(kws.length);
  const name = String(md.name ?? "").toLowerCase();
  const seenLower = new Set(), seenStem = new Set();
  for (const k of kws) {
    const sh = shape(k); e.shapes[sh] = (e.shapes[sh] ?? 0) + 1;
    const m = meta[k] ?? meta[k.toLowerCase()];
    const src = m?.source ?? "(no meta)"; e.sources[src] = (e.sources[src] ?? 0) + 1;
    if (m) { if (m.validated === true) e.validated.yes++; else if (m.validated === false) e.validated.no++; else e.validated.unknown++; const h = m.hits ?? {}; if ((h.returned ?? 0) + (h.packed ?? 0) + (h.kept ?? 0) + (h.applied ?? 0) > 0) e.hit_terms++; }
    else e.validated.unknown++;
    const lo = k.toLowerCase(); if (seenLower.has(lo)) e.dup_case++; seenLower.add(lo);
    const st = stem(k); if (seenStem.has(st) && !seenLower.has(lo + "\u0000")) { if (!isMarker(k)) e.dup_stem++; } seenStem.add(st);
    if (name && !isMarker(k) && name.includes(lo)) e.in_name++;
    e.terms.set(lo, (e.terms.get(lo) ?? 0) + 1);
    if (!e.bookkeeping) df.set(lo, (df.get(lo) ?? 0) + 1);
  }
  if (!e.bookkeeping) { content.n++; content.slots += kws.length; }
}
const med = (a) => { const s = [...a].sort((x, y) => x - y); return s.length ? s[Math.floor(s.length / 2)] : 0; };
const pct = (x, y) => (y ? +(100 * x / y).toFixed(1) : 0);
const rows = Object.entries(types).sort((a, b) => b[1].slots - a[1].slots).map(([type, e]) => ({
  type, bookkeeping: e.bookkeeping, containers: e.n, slots: e.slots, mean: +(e.slots / Math.max(1, e.n)).toFixed(2), median: med(e.per), max: Math.max(0, ...e.per),
  no_keywords_pct: pct(e.per.filter((x) => x === 0).length, e.n), shapes: e.shapes, sources: e.sources, validated: e.validated, terms_with_any_hit: e.hit_terms,
  duplicate_case: e.dup_case, duplicate_stem: e.dup_stem, keyword_already_in_name_pct: pct(e.in_name, e.slots),
  top: [...e.terms.entries()].sort((a, b) => b[1] - a[1]).slice(0, 12),
}));
// genericness measured from the data: a term carried by a large share of ALL content containers discriminates little
const contentTerms = [...df.entries()].sort((a, b) => b[1] - a[1]);
const generic = contentTerms.filter(([, n]) => n / Math.max(1, content.n) >= 0.1).slice(0, 25);
const hapax = contentTerms.filter(([, n]) => n === 1).length;
const top20share = pct(contentTerms.slice(0, 20).reduce((s, [, n]) => s + n, 0), content.slots);
const out = { label, date: new Date().toISOString(), containers: all, content: { containers: content.n, slots: content.slots, distinct_terms: contentTerms.length, singleton_terms: hapax, singleton_pct: pct(hapax, contentTerms.length), top20_terms_share_of_slots_pct: top20share, generic_terms_in_10pct_or_more_of_containers: generic }, by_type: rows };
fs.mkdirSync(path.resolve(here, "../../docs/evidence/keyword-measure"), { recursive: true });
const file = path.resolve(here, `../../docs/evidence/keyword-measure/categorize-${label}-${out.date.slice(0, 10)}.json`);
fs.writeFileSync(file, JSON.stringify(out, null, 1));
console.log(`keyword-categorize [${label}] → ${file}`);
console.log(`content containers ${content.n}, slots ${content.slots}, distinct ${contentTerms.length}, singletons ${out.content.singleton_pct}%, top-20 terms = ${top20share}% of slots`);
console.log("generic (in >=10% of content containers):", JSON.stringify(generic));
for (const r of rows.filter((x) => !x.bookkeeping)) {
  console.log(`\n${r.type}  n=${r.containers} mean=${r.mean} median=${r.median} max=${r.max} none=${r.no_keywords_pct}%  inName=${r.keyword_already_in_name_pct}% dupCase=${r.duplicate_case} dupStem=${r.duplicate_stem}`);
  console.log("  shapes", JSON.stringify(r.shapes), " sources", JSON.stringify(r.sources), " validated", JSON.stringify(r.validated), " withHit", r.terms_with_any_hit);
  console.log("  top", JSON.stringify(r.top.slice(0, 10)));
}
