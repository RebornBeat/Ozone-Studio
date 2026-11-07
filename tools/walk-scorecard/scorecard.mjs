#!/usr/bin/env node
// WALK SCORECARD (operator 2026-11-07): "track the percentage of progression throughout walks — how close each is to a full walk completion — and
// per-stage findings; keywords to 100%". Reads the host's durable per-stage metrics (zsei_data/walk_metrics.jsonl, one line per stage event WITH
// its trace_id — accurate even when walks run concurrently) and, for walks that predate it, falls back to the persistent host log (stage lines carry
// no walk id, so concurrent walks can interleave: those rows are marked `log`). No dependencies.
//
// usage: node tools/walk-scorecard/scorecard.mjs [--data-dir DIR] [--log FILE ...] [--since YYYY-MM-DD] [--out DIR] [--json]
import fs from "node:fs";
import path from "node:path";

const args = process.argv.slice(2);
const opt = (k, d) => { const i = args.indexOf(k); return i >= 0 ? args[i + 1] : d; };
const DATA = opt("--data-dir", "target/release/zsei_data");
const LOGS = args.reduce((a, x, i) => (x === "--log" ? [...a, args[i + 1]] : a), []);
if (LOGS.length === 0) LOGS.push("target/release/host.log", path.join(DATA, "logs/host.log"));
const SINCE = opt("--since", "2026-11-03");
const OUT = opt("--out", "docs/evidence/walk-scorecards");

// The 15 stages of /orchestrate. CORE = required for a "full walk"; 8 and 12 are the consciousness stages (reported, not required, because a
// walk with consciousness disabled legitimately skips them).
const STAGES = [
  [1, "Input Capture"], [0, "Jurisdiction Gate"], [2, "Text Normalization"], [3, "Gather Methodologies"], [4, "Initial Graph Creation"],
  [5, "Build AMT"], [6, "Blueprint Assignment"], [7, "Zero-Shot Simulation"], [8, "Consciousness Gate"], [14, "Jurisdiction Full Gate"],
  [9, "Task Creation"], [10, "Step Execution"], [11, "Result Collection"], [12, "Post-execution Consciousness"], [13, "Response Delivery"],
];
const CORE = new Set([1, 0, 2, 3, 4, 5, 6, 7, 14, 9, 10, 11, 13]);
const ORDER = STAGES.map((s) => s[0]);
const NAME = Object.fromEntries(STAGES);

const median = (xs) => { if (!xs.length) return null; const s = [...xs].sort((a, b) => a - b); return s[Math.floor((s.length - 1) / 2)]; };
const p90 = (xs) => { if (!xs.length) return null; const s = [...xs].sort((a, b) => a - b); return s[Math.min(s.length - 1, Math.floor(s.length * 0.9))]; };
const fmt = (ms) => (ms == null ? "—" : ms >= 60000 ? `${(ms / 60000).toFixed(1)} min` : `${(ms / 1000).toFixed(1)} s`);

// ---- 1. walks from walk_metrics.jsonl (accurate)
const walks = []; // {id, source, stages: Map(stage -> {ok, ms, summary, ts})}
const mfile = path.join(DATA, "walk_metrics.jsonl");
if (fs.existsSync(mfile)) {
  const by = new Map();
  for (const l of fs.readFileSync(mfile, "utf8").split("\n")) {
    if (!l.trim()) continue;
    let r; try { r = JSON.parse(l); } catch { continue; }
    if (!r.trace_id) continue;
    const day = new Date(r.ts_ms).toISOString().slice(0, 10);
    if (day < SINCE) continue;
    if (!by.has(r.trace_id)) by.set(r.trace_id, { id: r.trace_id, source: "metrics", stages: new Map(), first: r.ts_ms });
    const w = by.get(r.trace_id);
    // markers arrive as stage-5 rows named "AMT — …": only the real stage record counts for completion
    if (String(r.stage_name).startsWith("AMT — ")) continue;
    w.stages.set(r.stage, { ok: !!r.success, ms: r.duration_ms || 0, summary: r.summary || "", ts: r.ts_ms });
    w.last = r.ts_ms;
  }
  walks.push(...by.values());
}

// ---- 2. fallback: the host log (older walks; segmented at each "Stage 1 (Input Capture)" — concurrent walks may interleave)
const metricsFirst = walks.length ? Math.min(...walks.map((w) => w.first)) : Infinity;
const ansi = /\x1b\[[0-9;]*m/g;
const re = /^(\d{4}-\d\d-\d\dT[\d:.]+)Z\s+\w+\s+\S+ Stage (\d+)[a-z]? \(([^)]*)\) completed(?: in (\d+)ms)?:?\s*(.*)$/;
let cur = null;
const seen = new Set();
for (const f of LOGS) {
  if (!fs.existsSync(f)) continue;
  for (const raw of fs.readFileSync(f, "utf8").split("\n")) {
    if (raw.length < 30 || raw.slice(0, 2) !== "20" && raw.indexOf("2026-") < 0) continue;
    const l = raw.replace(ansi, "");
    const m = l.match(re);
    if (!m) continue;
    const ts = Date.parse(m[1] + "Z");
    const key = `${m[1]}|${m[2]}`;
    if (seen.has(key)) continue; // the same line appears in host.log and zsei_data/logs/host.log
    seen.add(key);
    if (m[1].slice(0, 10) < SINCE || ts >= metricsFirst) continue; // after metrics exist the jsonl is the truth
    const stage = Number(m[2]);
    if (stage === 1) { cur = { id: `log-${m[1]}`, source: "log", stages: new Map(), first: ts }; walks.push(cur); }
    if (!cur) continue;
    cur.stages.set(stage, { ok: true, ms: Number(m[4] || 0), summary: m[5] || "", ts });
    cur.last = ts;
  }
}

// ---- 3. per-walk scorecard
const now = Date.now();
const card = walks.map((w) => {
  const done = ORDER.filter((s) => w.stages.has(s));
  const coreDone = [...CORE].filter((s) => w.stages.has(s) && w.stages.get(s).ok).length;
  const pct = Math.round((100 * coreDone) / CORE.size);
  const lastStage = done.length ? done[done.length - 1] : null;
  const nextExpected = ORDER.find((s) => CORE.has(s) && !w.stages.has(s));
  const idleMs = now - (w.last || w.first);
  const status = pct === 100 ? "COMPLETE" : idleMs < 15 * 60 * 1000 ? `RUNNING (next: ${nextExpected != null ? `${nextExpected} ${NAME[nextExpected]}` : "—"})` : `STOPPED before ${nextExpected != null ? `${nextExpected} ${NAME[nextExpected]}` : "end"}`;
  const sum = (s) => w.stages.get(s)?.summary || "";
  const num = (s, re2) => { const m = sum(s).match(re2); return m ? Number(m[1]) : null; };
  return {
    id: w.id, source: w.source, started: new Date(w.first).toISOString(), completion_pct: pct, core_done: coreDone, core_total: CORE.size, status,
    stage_ms: Object.fromEntries(done.map((s) => [s, w.stages.get(s).ms])),
    methodologies: num(3, /Methodologies:\s*(\d+)/), kw_usable: num(3, /Keywords:\s*(\d+) usable/), kw_extracted: num(3, /usable of (\d+) extracted/),
    floor: /Floor:\s*yes/.test(sum(3)) ? true : /Floor:\s*no/.test(sum(3)) ? false : null,
    branches: num(5, /Branches:\s*(\d+)/), steps: num(6, /with (\d+) steps/), missing: num(6, /missing:\s*(\d+)/),
    feasibility: (sum(7).match(/Feasibility:\s*(\w+)/) || [])[1] || null, gate: (sum(8).match(/Decision:\s*(\w+)/) || [])[1] || null,
    steps_executed: num(10, /(\d+) steps executed/),
  };
});

// ---- 4. aggregate: funnel + per-stage health + findings
const stageRows = ORDER.map((s) => {
  const ms = walks.filter((w) => w.stages.has(s)).map((w) => w.stages.get(s).ms).filter((x) => x > 0);
  const reached = walks.filter((w) => w.stages.has(s)).length;
  return { stage: s, name: NAME[s], core: CORE.has(s), reached, reach_pct: walks.length ? Math.round((100 * reached) / walks.length) : 0, median_ms: median(ms), p90_ms: p90(ms), max_ms: ms.length ? Math.max(...ms) : null };
});
const complete = card.filter((c) => c.completion_pct === 100).length;
const findings = [];
const row = (s) => stageRows.find((r) => r.stage === s);
if (row(2).median_ms > 600000) findings.push(`Stage 2 (Text Normalization) median ${fmt(row(2).median_ms)}, p90 ${fmt(row(2).p90_ms)} — the K-algorithm 1×1 dual-5× loops (≥34 sequential model calls for one sentence); biggest single cost.`);
const floorUsed = card.filter((c) => c.floor === true).length, floorKnown = card.filter((c) => c.floor !== null).length;
if (floorKnown) findings.push(`Methodology floor used in ${floorUsed}/${floorKnown} walks that report it (target 0%: the floor means keyword search found nothing).`);
const kwBad = card.filter((c) => c.kw_usable != null && c.kw_usable < 3).length, kwKnown = card.filter((c) => c.kw_usable != null).length;
if (kwKnown) findings.push(`Keyword quality: ${kwKnown - kwBad}/${kwKnown} walks had ≥3 usable keywords after hygiene (target 100%).`);
const lows = card.filter((c) => c.feasibility === "low").length, feas = card.filter((c) => c.feasibility).length;
if (feas) findings.push(`Simulation feasibility "low" in ${lows}/${feas} walks that report it (target 0).`);
const miss = card.filter((c) => c.missing != null);
if (miss.length) findings.push(`Blueprint reported missing capabilities in ${miss.filter((c) => c.missing > 0).length}/${miss.length} walks (max ${Math.max(...miss.map((c) => c.missing))}); each is judged + closed on the spot by the capability-closure step (target 0 unresolved).`);
const dead = card.filter((c) => c.status.startsWith("STOPPED")).length;
findings.push(`Walks: ${card.length} since ${SINCE}; fully complete ${complete} (${card.length ? Math.round((100 * complete) / card.length) : 0}%); stopped before completion ${dead}; running ${card.filter((c) => c.status.startsWith("RUNNING")).length}.`);
const worst = [...stageRows].filter((r) => r.core && r.reach_pct < 100).sort((a, b) => a.reach_pct - b.reach_pct)[0];
if (worst) findings.push(`Funnel: the lowest-reach core stage is ${worst.stage} ${worst.name} (${worst.reach_pct}% of walks reach it).`);

// ---- 5. output
const date = new Date().toISOString().slice(0, 10);
let md = `# Walk scorecard — ${date}\n\nGoal: every walk 100% complete, every stage 100% reached and healthy. Source: ${fs.existsSync(mfile) ? "walk_metrics.jsonl (trace-attributed) + host log for older walks" : "host log only (no walk_metrics.jsonl yet — concurrent walks may interleave)"}. Core stages (required for 100%): ${[...CORE].join(", ")}; 8 and 12 are consciousness stages (reported, not required).\n\n## Findings\n${findings.map((f) => `- ${f}`).join("\n")}\n\n## Funnel and per-stage health (all walks since ${SINCE})\n| stage | name | core | reached | reach % | median | p90 | max |\n|---|---|---|---|---|---|---|---|\n`;
for (const r of stageRows) md += `| ${r.stage} | ${r.name} | ${r.core ? "yes" : "no"} | ${r.reached}/${walks.length} | ${r.reach_pct}% | ${fmt(r.median_ms)} | ${fmt(r.p90_ms)} | ${fmt(r.max_ms)} |\n`;
md += `\n## Per walk\n| walk | src | started (UTC) | completion | status | method. | kw usable/extracted | floor | AMT br. | steps | missing | feasibility | gate |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|\n`;
for (const c of card.slice(-60)) md += `| ${c.id.slice(0, 22)} | ${c.source} | ${c.started.slice(0, 16)} | **${c.completion_pct}%** (${c.core_done}/${c.core_total}) | ${c.status} | ${c.methodologies ?? "—"} | ${c.kw_usable ?? "—"}/${c.kw_extracted ?? "—"} | ${c.floor == null ? "—" : c.floor ? "yes" : "no"} | ${c.branches ?? "—"} | ${c.steps ?? "—"} | ${c.missing ?? "—"} | ${c.feasibility ?? "—"} | ${c.gate ?? "—"} |\n`;
fs.mkdirSync(OUT, { recursive: true });
fs.writeFileSync(path.join(OUT, `${date}.md`), md);
fs.writeFileSync(path.join(OUT, `${date}.json`), JSON.stringify({ date, since: SINCE, findings, stages: stageRows, walks: card }, null, 2));
if (args.includes("--json")) console.log(JSON.stringify({ findings, stages: stageRows, walks: card }, null, 2));
else { console.log(md.split("\n").slice(0, 28).join("\n")); console.log(`\n→ ${path.join(OUT, date + ".md")} and .json`); }
