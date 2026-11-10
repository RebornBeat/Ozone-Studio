#!/usr/bin/env node
// NAMESPACE BACKFILL — one-time, DETERMINISTIC rewrite of the bookkeeping tags already stored as plain words into namespaced markers
// (operator 2026-11-10, tag-namespacing option A; docs/KEYWORD_CAPTURE_AND_ML_GUIDE.md §17 finding 1 / §18.1; STATUS §J).
// It applies EXACTLY the rules of `keyword_program::namespace_bookkeeping_tags` (the write-time chokepoint), derived from each emitter's own
// keyword layout — no word list, no model call:
//   CoordinationEvent  [kind, agent, file:…, scope…]   position 0 → kind:<x>, position 1 → agent:<x> (only when position 0 is the kind)
//   Derived            run-span → span:run, run-root → span:root (every other Derived tag is content and is left alone)
//   McpResult          [tool, mcp_result, …]            position 0 → tool:<x>, mcp_result → kind:mcp_result
//   McpEntity          [tool, node kind, call:…, …]     position 0 → tool:<x>, position 1 → node:<x>   (only when position 0 became the tool)
// Markers, content terms and every other container type are never touched. Idempotent: a second run finds nothing.
// A renamed term's PENDING-ONLY registration in keyword_meta (validated null, source created:*, no hits) is dropped with it (a marker is never
// judged); a term with a verdict or hit history keeps its record.
// Dry run by default (reads the data dir, writes docs/evidence/keyword-measure/namespace-backfill-<label>-<date>.json, changes NOTHING):
//   node namespace_backfill.mjs --data <zsei_data dir> --label eu                              (dry run)
//   node namespace_backfill.mjs --data <dir> --label eu --apply --host http://127.0.0.1:50051   (applies through the host API, one container at a time)
// Every change is recorded (container id, type, before / after keywords, dropped meta entries) so it can be restored from the record.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const args = Object.fromEntries(process.argv.slice(2).reduce((a, x, i, all) => (x.startsWith("--") ? [...a, [x.slice(2), all[i + 1] && !all[i + 1].startsWith("--") ? all[i + 1] : true]] : a), []));
const label = args.label ?? "local";
const apply = args.apply === true;
const here = path.dirname(fileURLToPath(import.meta.url));

// shared/contracts/keyword_model.rs::is_marker, ported
const isMarker = (term) => {
  const t = term.trim();
  if (/\s/.test(t)) return false;
  const c = t.indexOf(":");
  if (c < 0) return false;
  const ns = t.slice(0, c), rest = t.slice(c + 1);
  if (!/^[a-z][a-z0-9_-]*$/.test(ns)) return false;
  return rest.length > 0 && !rest.startsWith(":") && !rest.startsWith("//");
};
const plain = (k) => k.trim() !== "" && !isMarker(k);
// keyword_program::tag_marker, ported
const tagMarker = (ns, raw) => {
  const v = raw.trim().toLowerCase().split(/\s+/).filter(Boolean).join("-");
  const m = `${ns}:${v}`;
  return v && isMarker(m) ? m : null;
};

/** Returns { keywords, renamed: [[old,new]] } — the same rewrite as keyword_program::namespace_bookkeeping_tags (before its de-dup). */
export function namespace(type, keywords) {
  const kws = [...keywords];
  const renamed = [];
  const set = (i, ns) => {
    if (i < kws.length && plain(kws[i])) {
      const m = tagMarker(ns, kws[i]);
      if (m) { renamed.push([kws[i], m]); kws[i] = m; }
    }
  };
  if (type === "CoordinationEvent") {
    set(0, "kind");
    if (kws[0]?.startsWith("kind:")) set(1, "agent");
  } else if (type === "Derived") {
    for (const [p, m] of [["run-span", "span:run"], ["run-root", "span:root"]]) {
      for (let i = 0; i < kws.length; i++) if (kws[i] === p) { kws[i] = m; renamed.push([p, m]); }
    }
  } else if (type === "McpResult") {
    set(0, "tool");
    for (let i = 0; i < kws.length; i++) if (kws[i] === "mcp_result") { kws[i] = "kind:mcp_result"; renamed.push(["mcp_result", "kind:mcp_result"]); }
  } else if (type === "McpEntity") {
    set(0, "tool");
    if (kws[0]?.startsWith("tool:")) set(1, "node");
  } else {
    return { keywords: [...keywords], renamed: [] };
  }
  const seen = new Set();
  return { keywords: kws.filter((k) => { const l = k.toLowerCase(); if (seen.has(l)) return false; seen.add(l); return true; }), renamed };
}

// ---- self-check (pure, runs every time; the same cases as the Rust unit test)
const eq = (a, b) => JSON.stringify(a) === JSON.stringify(b);
if (!eq(namespace("CoordinationEvent", ["tool_call", "sensor-tick", "file:a.rs", "scope:global"]).keywords, ["kind:tool_call", "agent:sensor-tick", "file:a.rs", "scope:global"])) throw new Error("self-check: CoordinationEvent");
if (!eq(namespace("McpEntity", ["uidocument_ingest", "StyleRule", "call:tool:x"]).keywords, ["tool:uidocument_ingest", "node:stylerule", "call:tool:x"])) throw new Error("self-check: McpEntity");
if (!eq(namespace("Methodology", ["tool_call", "Link"]).keywords, ["tool_call", "Link"])) throw new Error("self-check: other types untouched");
if (namespace("CoordinationEvent", ["kind:tool_call", "agent:sensor-tick"]).renamed.length !== 0) throw new Error("self-check: idempotent");

const plans = [];
const dir = path.join(args.data ?? ".", "local");
if (args.data) {
  for (const f of fs.readdirSync(dir).filter((x) => x.endsWith(".json"))) {
    let c; try { c = JSON.parse(fs.readFileSync(path.join(dir, f), "utf8")); } catch { continue; }
    const md = c.metadata ?? c.local_state?.metadata ?? {}, ctx = c.context ?? c.local_state?.context ?? {};
    const kws = Array.isArray(ctx.keywords) ? ctx.keywords.map(String) : [];
    const { keywords, renamed } = namespace(md.container_type, kws);
    if (!renamed.length) continue;
    const meta = ctx.keyword_meta ?? {};
    const dropped = [];
    for (const [old] of renamed) {
      if (keywords.includes(old)) continue;
      const m = meta[old];
      const h = m?.hits ?? {};
      const noHits = Object.values(h).every((v) => !v);
      if (m && (m.validated ?? null) === null && String(m.source ?? "").startsWith("created:") && noHits) dropped.push({ term: old, meta: m });
    }
    plans.push({ id: md.container_id ?? Number(f.replace(".json", "")), type: md.container_type, name: String(md.name ?? "").slice(0, 60), before: kws, after: keywords, renamed, dropped_meta: dropped });
  }
}
const byTag = {}, byType = {};
for (const p of plans) {
  byType[p.type] = (byType[p.type] ?? 0) + 1;
  for (const [o, n] of p.renamed) { const k = `${p.type}: ${o} → ${n.split(":")[0]}:`; byTag[k] = (byTag[k] ?? 0) + 1; }
}
console.log(`namespace-backfill [${label}] ${apply ? "APPLY" : "DRY RUN"}: ${plans.length} container(s) | by type ${JSON.stringify(byType)}`);
console.log("  by tag:", JSON.stringify(byTag, null, 1));
for (const p of plans.slice(0, 6)) console.log(`  ${p.type} #${p.id} "${p.name}": ${p.renamed.map(([o, n]) => `${o} → ${n}`).join(" | ")}`);

const out = { label, date: new Date().toISOString(), apply, containers_changed: plans.length, by_type: byType, by_tag: byTag, dropped_pending_meta: plans.reduce((a, p) => a + p.dropped_meta.length, 0), containers: plans };
const outDir = path.resolve(here, "../../docs/evidence/keyword-measure");
fs.mkdirSync(outDir, { recursive: true });
const file = path.join(outDir, `namespace-backfill-${label}-${out.date.slice(0, 10)}.json`);

if (apply) {
  const { http } = await import(args.lib ?? "../e2e/lib.mjs");
  let ok = 0, failed = 0;
  for (const p of plans) {
    const get = async () => (await http("POST", "/zsei/query", { query: { GetContainer: { container_id: p.id } }, session_token: "" })).json?.result?.Container;
    const c = await get();
    if (!c) { failed++; p.applied = "container not readable"; continue; }
    const ctx = c.local_state.context;
    // recompute from the LIVE container (it may have changed since the dry-run scan); nothing else in the container is touched
    const { keywords, renamed } = namespace(c.local_state.metadata.container_type, ctx.keywords ?? []);
    if (!renamed.length) { p.applied = "already namespaced"; continue; }
    ctx.keyword_meta ??= {};
    for (const [old] of renamed) {
      const m = ctx.keyword_meta[old];
      const noHits = Object.values(m?.hits ?? {}).every((v) => !v);
      if (!keywords.includes(old) && m && (m.validated ?? null) === null && String(m.source ?? "").startsWith("created:") && noHits) delete ctx.keyword_meta[old];
    }
    ctx.keywords = keywords;
    const r = await http("POST", "/zsei/query", { query: { UpdateContainer: { container_id: p.id, updates: { metadata: null, context: ctx, storage: null, hints: null } } }, session_token: "" });
    if (r.status === 200) { ok++; p.applied = true; } else { failed++; p.applied = `status ${r.status}`; }
  }
  console.log(`applied: ${ok} container(s) ok, ${failed} failed`);
  out.applied_ok = ok; out.applied_failed = failed;
}
fs.writeFileSync(file, JSON.stringify(out, null, 1));
console.log("record →", file);
