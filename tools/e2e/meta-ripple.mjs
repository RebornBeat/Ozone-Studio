// Meta-ripple E2E (docs/RIPPLE_CATEGORIES_AND_META_RIPPLE_2026-11-04.md §5): real host, real ripple, no mocks.
// Needs the host started with OZONE_META_MIN_GAP_SECS=15 OZONE_ASSISTANT_MIN_GAP_SECS=15 so coalescing is observable in seconds.
import { suite, http, pipeline, sessionToken } from "./lib.mjs";
const s = suite("e2e-meta-ripple");
const st = async () => (await http("GET", "/ripple/meta/status")).json;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const tok = await sessionToken();

let a = await st();
s.check("MR0", "status route live: policy + per-loop counters present", !!a.policy && !!a.loops?.i_loop && a.loops?.assistant?.wakes !== undefined, JSON.stringify(a.policy));
const gap = a.policy.min_gap_secs;

// MR1 echo/structure/content events must NOT be meta
const tag = Date.now().toString(36);
const ws = (await pipeline(6, { action: "CreateWorkspace", name: `MR ws ${tag}`, description: "" })).output?.workspace?.id;
await pipeline(6, { action: "CreateProject", workspace_id: ws, name: `MR proj ${tag}`, description: "" });
await sleep(1500);
let b = await st();
s.check("MR1", "Workspace/Project (structure) creation does NOT wake the meta loops", b.meta_events === a.meta_events && b.loops.assistant.wakes === a.loops.assistant.wakes, `meta_events ${a.meta_events}->${b.meta_events}`);
s.check("MR1b", "...but the dispatcher did see the events", b.events_seen > a.events_seen, `events_seen ${a.events_seen}->${b.events_seen}`);

// MR2 a task transition IS a meta ripple: all loops woken within seconds
const t = await http("POST", "/task/create", { prompt: `MR task ${tag}`, name: `MR task ${tag}`, kind: "todo", session_token: tok });
await sleep(2000);
let c = await st();
s.check("MR2", "a Task ripple is a META event (meta_events rose)", c.meta_events > b.meta_events, `${b.meta_events}->${c.meta_events} task=${t.json.task_id}`);
for (const l of ["i_loop", "meta_loop", "assistant"]) s.check(`MR2-${l}`, `${l} was woken by the meta ripple`, c.loops[l].wakes > b.loops[l].wakes, `${b.loops[l].wakes}->${c.loops[l].wakes}`);

// MR3 burst of 20 task events: 20 wakes, but the minimum gap coalesces them into (at most) a couple of passes
const before = await st();
await Promise.all(Array.from({ length: 20 }, (_, i) => http("POST", "/task/create", { prompt: `MR burst ${tag} ${i}`, name: `MR burst ${tag} ${i}`, kind: "todo", session_token: tok })));
await sleep((gap + 12) * 1000);
const after = await st();
s.check("MR3", "burst of 20 task events → ≥20 meta events seen", after.meta_events - before.meta_events >= 20, `+${after.meta_events - before.meta_events}`);
for (const l of ["i_loop", "assistant"]) {
  const passes = after.loops[l].passes - before.loops[l].passes;
  s.check(`MR3-${l}`, `${l}: 20 events coalesce into ≤2 passes (min-gap ${gap}s), not 20`, passes <= 2, `passes +${passes}, wakes +${after.loops[l].wakes - before.loops[l].wakes}`);
}

// MR4 a loop's own pass events never wake it (echo): the I-Loop/assistant passes write insights; echoes counted, no runaway
const calm0 = await st(); await sleep((gap + 8) * 1000); const calm1 = await st();
const runaway = calm1.loops.assistant.passes - calm0.loops.assistant.passes;
s.check("MR4", "no self-trigger runaway: an idle window produces ≤1 further assistant pass", runaway <= 1, `passes +${runaway}, echoes ignored=${calm1.own_pass_echoes_ignored}`);

s.skip("MR5", "MethodologyGap ripple wakes the meta-loop (end-to-end)", "needs a request that matches no methodology; covered by unit tests (ripple_class::tests) + emit site in amt.rs; add a gap-forcing orchestrate case");
process.exit(s.finish() ? 1 : 0);
