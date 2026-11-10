#!/usr/bin/env node
// BACKLOG CLEANUP — a one-time, reviewed, DETERMINISTIC patch for keywords stored before the write-time gate existed (docs/KEYWORD_CAPTURE_AND_ML_GUIDE.md §21).
// NO model calls. It removes ONLY what is provably not a keyword by shape or by an explicitly reviewed list, on the types the gate covers:
//   R1 empty · R2 sentence-like (> 4 words, not a marker) · R3 case-insensitive duplicate (the first spelling is kept) · R4 digit-only (an id/number)
//   R5 a REVIEWED list of English function words, exact whole-term match only (never a substring, never an acronym).
// Everything else — including every unvalidated expansion / extracted term — is left alone (it is judged by the gate, not removed by shape).
// Dry run by default (reads the data dir, writes docs/evidence/keyword-measure/backlog-cleanup-<label>-<date>.json, changes NOTHING).
//   node backlog_cleanup.mjs --data <zsei_data dir> --label eu            (dry run)
//   node backlog_cleanup.mjs --data <dir> --label eu --apply --host http://127.0.0.1:50051   (applies through the host API, one container at a time)
// Every removal is recorded (container, term, rule, previous meta) so it can be restored with restore_term.mjs.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const args = Object.fromEntries(process.argv.slice(2).reduce((a, x, i, all) => (x.startsWith("--") ? [...a, [x.slice(2), all[i + 1] && !all[i + 1].startsWith("--") ? all[i + 1] : true]] : a), []));
const label = args.label ?? "local";
const apply = args.apply === true;
// NOT JurisdictionRuleSet: its keywords are routing keys (region / ISO country codes such as is, no, do, it) the jurisdiction loader searches by.
const GATED = new Set(["Methodology", "Project", "Workspace", "ModalityGraph", "Blueprint", "Pipeline", "FileReference", "Category", "McpEntity"]);
// REVIEWED 2026-11-10 from the actual stores (both): unambiguous English function words only. Acronyms (qa, tdd, ssh, mcp, ufw, llm, git …) and content words are NOT here.
const FILLER = new Set(["a", "an", "the", "and", "or", "is", "are", "it", "you", "to", "of", "i", "not", "no", "do", "why"]);
const isMarker = (t) => { const i = t.indexOf(":"); return !/\s/.test(t) && i >= 1 && i < t.length - 1; };
const here = path.dirname(fileURLToPath(import.meta.url));

const plans = []; // { id, type, name, remove: [{term, rule, validated, meta}] }
const dir = path.join(args.data, "local");
for (const f of fs.readdirSync(dir).filter((x) => x.endsWith(".json"))) {
  let c; try { c = JSON.parse(fs.readFileSync(path.join(dir, f), "utf8")); } catch { continue; }
  const md = c.metadata ?? c.local_state?.metadata ?? {}, ctx = c.context ?? c.local_state?.context ?? {};
  if (!GATED.has(md.container_type)) continue;
  const meta = ctx.keyword_meta ?? {};
  const seen = new Set(), remove = [];
  for (const k of Array.isArray(ctx.keywords) ? ctx.keywords : []) {
    const t = String(k).trim();
    const why = !t ? "R1-empty" : isMarker(t) ? null : t.split(/\s+/).length > 4 ? "R2-sentence" : seen.has(t.toLowerCase()) ? "R3-duplicate" : /^\d+$/.test(t) ? "R4-digit-only" : FILLER.has(t.toLowerCase()) ? "R5-filler" : null;
    if (t) seen.add(t.toLowerCase());
    if (why) remove.push({ term: k, rule: why, validated: (meta[k] ?? {}).validated ?? null, meta: meta[k] ?? null });
  }
  if (remove.length) plans.push({ id: md.container_id ?? Number(f.replace(".json", "")), type: md.container_type, name: String(md.name ?? "").slice(0, 60), remove });
}
const byRule = {};
for (const p of plans) for (const r of p.remove) byRule[r.rule] = (byRule[r.rule] ?? 0) + 1;
const total = Object.values(byRule).reduce((a, b) => a + b, 0);
const validatedOverridden = plans.flatMap((p) => p.remove.filter((r) => r.validated === true).map((r) => `${p.id}:${r.term}`));
console.log(`backlog-cleanup [${label}] ${apply ? "APPLY" : "DRY RUN"}: ${total} term(s) in ${plans.length} container(s) | by rule ${JSON.stringify(byRule)}`);
console.log(`  of which the gate had marked VALIDATED (overridden by this review): ${validatedOverridden.length} ${JSON.stringify(validatedOverridden.slice(0, 12))}`);
for (const p of plans.slice(0, 8)) console.log(`  ${p.type} #${p.id} "${p.name}": ${p.remove.map((r) => `${r.rule.split("-")[0]}:${String(r.term).slice(0, 34)}`).join(" | ").slice(0, 200)}`);

const out = { label, date: new Date().toISOString(), apply, total, by_rule: byRule, containers: plans };
const outDir = path.resolve(here, "../../docs/evidence/keyword-measure");
fs.mkdirSync(outDir, { recursive: true });
const file = path.join(outDir, `backlog-cleanup-${label}-${out.date.slice(0, 10)}.json`);

if (apply) {
  const { http } = await import(args.lib ?? "../e2e/lib.mjs");
  let ok = 0, failed = 0;
  for (const p of plans) {
    const get = async () => (await http("POST", "/zsei/query", { query: { GetContainer: { container_id: p.id } }, session_token: "" })).json?.result?.Container;
    const c = await get();
    if (!c) { failed++; p.applied = "container not readable"; continue; }
    const ctx = c.local_state.context;
    ctx.keyword_meta ??= {}; // a container with no counters yet comes back without the field
    const drop = new Map(); // term -> first occurrence index to remove handled by position for duplicates
    const seen = new Set(); const kept = [];
    for (const k of ctx.keywords) {
      const t = String(k).trim();
      const rule = p.remove.find((r) => r.term === k);
      if (rule && !(rule.rule === "R3-duplicate" && !seen.has(t.toLowerCase()))) {
        // a removed DUPLICATE leaves the kept copy's metadata alone; every other rule records the term as rejected so it is never proposed again
        if (rule.rule !== "R3-duplicate") ctx.keyword_meta[k] = { ...(ctx.keyword_meta[k] ?? {}), source: (ctx.keyword_meta[k] ?? {}).source || "extracted", validated: false };
        continue;
      }
      if (t) seen.add(t.toLowerCase());
      kept.push(k);
    }
    ctx.keywords = kept;
    const r = await http("POST", "/zsei/query", { query: { UpdateContainer: { container_id: p.id, updates: { metadata: null, context: ctx, storage: null, hints: null } } }, session_token: "" });
    if (r.status === 200) { ok++; p.applied = true; } else { failed++; p.applied = `status ${r.status}`; }
  }
  console.log(`applied: ${ok} container(s) ok, ${failed} failed`);
  out.applied_ok = ok; out.applied_failed = failed;
}
fs.writeFileSync(file, JSON.stringify(out, null, 1));
console.log("record →", file);
