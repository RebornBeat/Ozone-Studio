// WALK COMPLETION (docs/WALK_COMPLETION_GUIDE.md): run ONE real walk and assert the per-stage 100% criteria from the durable metrics
// (zsei_data/walk_metrics.jsonl, trace-attributed) — completion %, usable keywords, floor not used, missing capabilities resolved,
// feasibility not low, all steps executed. Real host, no mocks. A long walk: default wait 1 h (WALK_MAX_MS to change).
import fs from "node:fs";
import path from "node:path";
import { suite, orchestrate } from "./lib.mjs";
const s = suite("e2e-walk-completion");
const DATA = process.env.OZONE_DATA ?? new URL("../../target/release/zsei_data", import.meta.url).pathname;
const prompt = process.env.WALK_PROMPT ?? "Explain in two sentences why graph databases suit highly connected data.";
const t0 = Date.now();
const r = await orchestrate({ prompt, consciousness_enabled: false }, { pollMs: 15000, maxMs: Number(process.env.WALK_MAX_MS ?? 3600000) });
s.check("W0", "the walk finished (not timed out)", r.state && r.state !== "timeout" && r.state !== "running", JSON.stringify({ state: r.state, task: r.task_id }).slice(0, 160));

const rows = fs.existsSync(path.join(DATA, "walk_metrics.jsonl"))
  ? fs.readFileSync(path.join(DATA, "walk_metrics.jsonl"), "utf8").split("\n").filter(Boolean).map((l) => { try { return JSON.parse(l); } catch { return null; } }).filter(Boolean)
  : [];
const mine = rows.filter((x) => x.ts_ms >= t0 - 5000 && x.trace_id && !String(x.stage_name).startsWith("AMT — "));
// the walk of THIS prompt = the trace whose Stage 1..3 appeared first after t0
const traces = [...new Set(mine.map((x) => x.trace_id))];
const walk = traces.map((id) => mine.filter((x) => x.trace_id === id)).sort((a, b) => b.length - a.length)[0] ?? [];
const st = (n) => walk.find((x) => x.stage === n);
const CORE = [1, 0, 2, 3, 4, 5, 6, 7, 14, 9, 10, 11, 13];
const done = CORE.filter((n) => st(n)?.success);
s.check("W1", `completion 100% (${done.length}/${CORE.length} core stages)`, done.length === CORE.length, `missing: ${CORE.filter((n) => !st(n)).join(",") || "none"}`);
const num = (n, re) => { const m = (st(n)?.summary ?? "").match(re); return m ? Number(m[1]) : null; };
const kw = num(3, /Keywords:\s*(\d+) usable/);
s.check("W2", "Stage 2/3: ≥3 usable request keywords", kw !== null && kw >= 3, `usable=${kw} extracted=${num(3, /usable of (\d+)/)}`);
s.check("W3", "Stage 3: methodology floor NOT used (keyword search found matches)", /Floor:\s*no/.test(st(3)?.summary ?? ""), (st(3)?.summary ?? "").slice(0, 160));
const missing = num(6, /missing:\s*(\d+)/);
s.check("W4", "Stage 6: no unresolved missing capabilities after closure", missing === 0 || missing === null, `missing=${missing}`);
const feas = ((st(7)?.summary ?? "").match(/Feasibility:\s*(\w+)/) || [])[1];
s.check("W5", "Stage 7: feasibility not low", !!feas && feas !== "low", `feasibility=${feas}`);
const executed = num(10, /(\d+) steps executed/), planned = num(6, /with (\d+) steps/);
s.check("W6", "Stage 10: every planned step executed", executed !== null && planned !== null && executed === planned, `executed=${executed} planned=${planned}`);
s.check("W7", "Stage 13: response delivered", !!st(13)?.success, `${st(13)?.duration_ms ?? "—"} ms`);
s.finish();
