// SPAN-OUT IN A REAL WALK over a CODE WORKSPACE (operator 2026-11-07: "code modality in a workspace, span outs again, test these").
// Builds a code workspace/project with real call structure through the Code modality (pipeline 101), submits ONE real walk scoped to that
// project asking about the call flow, then asserts from the finished task: step contexts carry span-out provenance, the span-out text
// reached the model-facing context with the entry-rooted call sequence, and the answer names the real functions.
import fs from "node:fs";
import { suite, pipeline, http, orchestrate } from "./lib.mjs";
const s = suite("e2e-span-walk");
const out = (r) => r?.output ?? r?.data ?? r;
const dir = process.env.SPAN_DIR ?? "/tmp/e2e_spanws"; fs.rmSync(dir, { recursive: true, force: true }); fs.mkdirSync(dir, { recursive: true });
fs.writeFileSync(`${dir}/pipeline_app.py`, `
from stages import ingest_records, normalize_records, score_records

def run_report(path):
    raw = ingest_records(path)
    clean = normalize_records(raw)
    return score_records(clean)

def main():
    return run_report("data.csv")

if __name__ == "__main__":
    main()
`);
fs.writeFileSync(`${dir}/stages.py`, `
def ingest_records(path):
    return [line.strip() for line in open(path)]

def normalize_records(rows):
    return [r.lower() for r in rows if r]

def score_records(rows):
    return {r: len(r) for r in rows}
`);
const tag = Date.now().toString(36);
const ws = out(await pipeline(6, { action: "CreateWorkspace", name: `E2E SpanWalk ${tag}`, description: "span-out in a walk" }));
const wsId = ws?.workspace?.id;
const pr = out(await pipeline(6, { action: "CreateProject", workspace_id: wsId, name: `Report code ${tag}`, description: "" }));
const projId = pr?.project?.id;
s.check("SW0", "code workspace + project created", !!wsId && !!projId, `ws=${wsId} proj=${projId}`);
const graphIds = [];
for (const f of ["pipeline_app.py", "stages.py"]) {
  const code = fs.readFileSync(`${dir}/${f}`, "utf8");
  const an = out(await pipeline(101, { action: { type: "Analyze", code, language: null, file_path: `${dir}/${f}` } }, { project_id: projId, workspace_id: wsId }))?.analysis;
  const g = out(await pipeline(101, { action: { type: "CreateGraph", analysis_result: an, project_id: projId, link_to_existing: false } }, { project_id: projId, workspace_id: wsId }));
  if (g?.graph_id) graphIds.push(g.graph_id);
}
s.check("SW1", "both code files became modality graphs in the project", graphIds.length === 2, JSON.stringify(graphIds));
const sp = (await http("POST", "/context/span_out", { seeds: graphIds, budget_tokens: 4000, include_text: true })).json;
s.check("SW2", "direct span-out from the code graphs is entry-rooted (run_report/main → ingest → normalize → score)", sp.success === true && /ingest_records/.test(sp.text ?? "") && /score_records/.test(sp.text ?? ""), String(sp.text ?? "").slice(0, 120));

const r = await orchestrate({ prompt: `Using the code in this project (${tag}), explain the order in which run_report calls its helper functions and what each one does.`, project_id: projId, workspace_id: wsId, consciousness_enabled: false }, { pollMs: 15000, maxMs: Number(process.env.WALK_MAX_MS ?? 3600000) });
s.check("SW3", "the walk finished", r.state && r.state !== "timeout" && r.state !== "running", JSON.stringify({ state: r.state, task: r.task_id, err: String(r.error ?? "").slice(0, 120) }));
const taskId = Number(r.task_id ?? (String(r.error ?? "").match(/task (\d+)/) ?? [])[1] ?? 0);
const t = taskId ? (await http("POST", "/task/get", { task_id: taskId })).json : null;
const steps = t?.task?.steps ?? t?.steps ?? [];
const withSpan = steps.filter((st) => (st.context_sources ?? []).some((c) => /^span-out:\d+ items/.test(c)));
s.check("SW4", "step contexts carry span-out provenance (span-out:K items, T tokens, F fetched)", withSpan.length > 0, `${withSpan.length}/${steps.length} steps; ${JSON.stringify(steps.map((x) => x.context_sources).slice(0, 3))}`);
const ctxText = steps.map((st) => st.context_assembled ?? "").join("\n");
s.check("SW5", "the assembled step context holds the graph span-out text with the real functions", /Graph span-out context/.test(ctxText) && /ingest_records/.test(ctxText), ctxText.slice(0, 120));
const answer = JSON.stringify(r.result ?? r.response ?? r.output ?? r);
s.check("SW6", "the delivered answer names the real call order (ingest_records → normalize_records → score_records)", /ingest_records/.test(answer) && /normalize_records/.test(answer) && /score_records/.test(answer), answer.slice(0, 200));
process.exit(s.finish() ? 1 : 0);
