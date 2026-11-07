// Provider-step E2E (docs/MCP_TOOLS_MODALITY_ARCHITECTURE_REVIEW_2026-11-04.md; operator: "provider is just a step type").
// Real host, no mocks. Needs: host with OZONE_TOOL_ROUTING not required (provider steps are bound steps, not search routing),
// threed-mcp registered + startable. Methodologies are created through Ozone's own methodology_create pipeline (12).
import fs from "node:fs";
import path from "node:path";
import { suite, http, pipeline, orchestrate } from "./lib.mjs";
const s = suite("e2e-provider-step");
const DATA = process.env.OZONE_DATA ?? new URL("../../target/release/zsei_data", import.meta.url).pathname;
const tag = Date.now().toString(36);
const out = (r) => r?.output ?? r?.data ?? r;

// P1: a methodology whose rule DECLARES provider threed-mcp (kind=provider) — created by Ozone's own pipeline
const kw = `zzclock${tag}`;
const create = await pipeline(12, { action: "Create", name: `Clock building ${tag}`, description: "Build a 3D clock when asked", category_id: 0,
  principles: [], heuristics: [],
  decision_rules: [
    { name: "3d clock", condition: `a ${kw} 3D clock is requested`, outcome: "generate the 3D clock asset with the 3D provider", requires: [{ kind: "provider", name: "threed-mcp", purpose: "build the clock asset" }] },
    { name: "wanted", condition: `a ${kw} hologram is requested`, outcome: "render it with the holographic provider", requires: [{ kind: "provider", name: `hologram-mcp-${tag}`, purpose: "not installed on purpose" }] },
  ],
  keywords: [kw, "clock", "3d"], topics: ["3d", "clock"], task_id: null });
s.check("PS1", "methodology with per-rule providers created through pipeline 12", out(create)?.success === true, JSON.stringify(out(create)).slice(0, 160));

// P2: it appears in the marketplace with DECLARED bindings
const o1 = (await http("GET", "/marketplace/overview")).json;
const m = o1.methodologies.find((x) => x.name === `Clock building ${tag}`);
s.check("PS2", "marketplace lists the methodology with 2 rules, both with declared providers", !!m && m.rule_count === 2 && m.rules_with_declared_tools === 2, JSON.stringify(m?.rules?.map((r) => r.requires)).slice(0, 200));

// P3: a request matching it → blueprint contains a PROVIDER STEP for threed-mcp placed before its carrier, with rule_refs
// The walk may legitimately be HELD at the Jurisdiction Full Gate for a human RequireConfirmation (3D plans): the request is NOT
// approved, the error names "[ReviewPending review_id=…]" and the product's contract is "resolve it, then re-submit". The test
// plays that human: resolve every unresolved review (decision-review rows AND jurisdiction_confirm awaiting items, by review_id)
// and re-submit — bounded by PROGRESS (another round only if a NEW review was resolved this round).
const resolvedIds = new Set();
async function resolvePending() {
  const pend = (await http("GET", "/consciousness/reviews/pending")).json ?? {};
  const ids = [...(pend.rows ?? []).map((r) => r.review_id ?? r.id), ...(pend.awaiting ?? []).filter((a) => a.kind === "jurisdiction_confirm").map((a) => a.review_id)].filter(Boolean);
  let fresh = 0;
  for (const id of ids) { if (resolvedIds.has(id)) continue; resolvedIds.add(id); fresh++;
    await http("POST", "/consciousness/reviews/resolve", { review_id: id, decision: "Proceed", note: "e2e provider-step: operator confirms" }); }
  return fresh;
}
let r;
for (;;) {
  r = await orchestrate({ prompt: `Please build a ${kw} 3D clock showing 10:10.`, consciousness_enabled: false }, { pollMs: 10000, maxMs: 3600000 });
  const held = /ReviewPending|held for human review/.test(JSON.stringify(r));
  if (!held || (await resolvePending()) === 0) break;
}
const taskId = Number((String(r.error ?? "").match(/task (\d+)/) ?? [])[1] ?? r.task_id ?? 0);
const cpPath = path.join(DATA, "runs", `${taskId}.json`);
const cp = fs.existsSync(cpPath) ? JSON.parse(fs.readFileSync(cpPath, "utf8")) : null;
const steps = cp?.blueprint_steps ?? [];
const prov = steps.filter((st) => st.step_type === "provider");
s.check("PS3", "blueprint has a provider step for threed-mcp (type 'provider', pipeline 0, rule_refs set)", prov.some((st) => st.provider === "threed-mcp" && st.pipeline_id === 0 && st.rule_refs.length > 0), `steps=${steps.length} provider=${JSON.stringify(prov.map((p) => p.provider))}`);
const pi = steps.findIndex((st) => st.step_type === "provider" && st.provider === "threed-mcp");
const carrier = pi >= 0 ? steps.find((st) => st.depends_on.includes(steps[pi].step_index)) : null;
s.check("PS4", "the step carrying the rule depends on the provider step (translation: rule → provider step → carrier)", !!carrier, JSON.stringify(carrier?.depends_on));
s.check("PS5", "step indexes are contiguous after insertion and every depends_on resolves", steps.every((st, i) => st.step_index === i && st.depends_on.every((d) => d >= 0 && d < steps.length)), "");

// P4: the missing provider is a CAPTURED missing capability, not papered over
const gaps = JSON.parse(fs.existsSync(path.join(DATA, "capability_gaps.json")) ? fs.readFileSync(path.join(DATA, "capability_gaps.json"), "utf8") : "[]");
const g = gaps.find((x) => String(x.name).includes(`hologram-mcp-${tag}`));
s.check("PS5b", "missing provider captured durably as a wanted capability (capability_gaps.json)", !!g && g.status === "wanted", String(JSON.stringify(g)).slice(0, 200));
const o2 = (await http("GET", "/marketplace/overview")).json;
s.check("PS6", "…and shown in the Marketplace as WANTED", o2.wanted.some((w) => String(w.name).includes(`hologram-mcp-${tag}`)), `${o2.wanted.length} wanted`);

// P5: the provider step REALLY ran: its result reached the carrier as a previous-step output, containers carry called_from
const done = cp?.completed_steps ?? [];
const provDone = done.find((c) => prov.some((p) => p.step_index === c.step_index));
s.check("PS7", "provider step completed with a real provider result", !!provDone && JSON.stringify(provDone.output).includes("PROVIDER RESULT"), String(JSON.stringify(provDone?.output)).slice(0, 160));
const logTxt = fs.existsSync(path.join(DATA, "..", "host.log")) ? fs.readFileSync(path.join(DATA, "..", "host.log"), "utf8") : "";
s.pass("PS8", "(informational) host log mentions the provider step", /provider step begin \(threed-mcp\)/.test(logTxt) ? "found" : "not found in host.log");
process.exit(s.finish() ? 1 : 0);
