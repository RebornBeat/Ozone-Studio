// Per-modality fan-out E2E (docs/PER_MODALITY_FANOUT_GUIDE_2026-11-03.md §4).
// FO-E1: a real multi-file CODE workspace is linked+analysed by Ozone's own file_link/code pipelines;
//        span-out (POST /context/span_out) from the resulting code-graph containers must report the
//        REAL entries and call sequence (entry-rooted), not flat text.
// FO-E3: coverage over every modality graph already in the host: edge-class coverage + empties.
import fs from "node:fs";
import { suite, pipeline, http } from "./lib.mjs";

const s = suite("e2e-fanout");
const out = (r) => r?.output ?? r?.data ?? r;
const dir = "/tmp/e2e_codews"; fs.rmSync(dir, { recursive: true, force: true }); fs.mkdirSync(dir, { recursive: true });
// A tiny but real program with a known call structure:
//   main -> load_config, drive ; drive -> parse, render ; parse -> tokenize ; render -> fmt_line ; orphan_util is never called
fs.writeFileSync(`${dir}/app.py`, `
from util import tokenize, fmt_line

def load_config(path):
    return {"path": path}

def parse(text):
    return tokenize(text)

def render(tokens):
    return [fmt_line(t) for t in tokens]

def drive(cfg):
    toks = parse(cfg["path"])
    return render(toks)

def main():
    cfg = load_config("x.txt")
    return drive(cfg)

if __name__ == "__main__":
    main()
`);
fs.writeFileSync(`${dir}/util.py`, `
def tokenize(text):
    return text.split()

def fmt_line(tok):
    return "> " + tok

def orphan_util():
    return 42
`);
fs.writeFileSync(`${dir}/lib.rs`, `
pub fn entry() -> i32 { helper_a() + helper_b() }
fn helper_a() -> i32 { leaf() }
fn helper_b() -> i32 { leaf() + 1 }
fn leaf() -> i32 { 7 }
`);

const tag = Date.now().toString(36);
const ws = out(await pipeline(6, { action: "CreateWorkspace", name: `E2E CodeWS ${tag}`, description: "fan-out" }));
const pr = out(await pipeline(6, { action: "CreateProject", workspace_id: ws?.workspace?.id, name: `Code ${tag}`, description: "" }));
const projId = pr?.project?.id;
s.check("FO0", "code workspace project created", !!projId, `proj=${projId}`);

const graphIds = [];
for (const f of ["app.py", "util.py", "lib.rs"]) {
  // Ozone-native path: the host's Code modality (pipeline 101) analyses the file, then builds its modality graph.
  const code = fs.readFileSync(`${dir}/${f}`, "utf8");
  const an = await pipeline(101, { action: { type: "Analyze", code, language: null, file_path: `${dir}/${f}` } }, { project_id: projId });
  const analysis = out(an)?.analysis;
  s.check(`FO-A-${f}`, `${f}: Code modality analysis returned entry points`, Array.isArray(analysis?.entry_points) && analysis.entry_points.length > 0, JSON.stringify(analysis?.entry_points));
  const g = await pipeline(101, { action: { type: "CreateGraph", analysis_result: analysis, project_id: projId, link_to_existing: false } }, { project_id: projId });
  const gid = out(g)?.graph_id;
  console.log(f, "graph", gid);
  if (gid) graphIds.push(gid);
}
s.check("FO1", "each code file produced a modality graph container", graphIds.length === 3, JSON.stringify(graphIds));

if (graphIds.length) {
  const sp = (await http("POST", "/context/span_out", { seeds: graphIds, budget_tokens: 4000, include_text: true })).json;
  s.check("FO2", "span-out succeeds on the code graphs", sp.success === true && sp.items?.length > 0, JSON.stringify(sp).slice(0, 200));
  const fo = (sp.items ?? []).filter((i) => i.fanout);
  s.check("FO3", "graph files were FANNED OUT (fanout evidence present), not flat-rendered", fo.length >= 2, `${fo.length} items with fanout`);
  const app = fo.find((i) => i.fanout.entries?.includes("main"));
  s.check("FO4", "entry points found: main (app.py) is an ENTRY", !!app, JSON.stringify(fo.map((i) => i.fanout.entries)).slice(0, 200));
  const seq = app?.fanout?.sequence_head ?? [];
  s.check("FO5", "sequence is entry-rooted: main first, then load_config/drive before parse/render", seq[0] === "main" && seq.indexOf("drive") > 0 && seq.indexOf("parse") > seq.indexOf("drive"), JSON.stringify(seq));
  s.check("FO6", "the Calls edges are Flow-class lines in the packed text (main -Calls-> drive)", /main -Calls-> drive/.test(sp.text ?? ""), (sp.text ?? "").slice(0, 160));
  const rs = fo.find((i) => i.fanout.entries?.includes("entry"));
  s.check("FO7", "Rust file: entry -> helper_a/helper_b -> leaf", !!rs && rs.fanout.sequence_head.indexOf("leaf") > rs.fanout.sequence_head.indexOf("helper_a"), JSON.stringify(rs?.fanout?.sequence_head));
  s.check("FO9", "cross-file call captured: parse -Calls-> util.tokenize (external node, import-resolved)", /parse -Calls-> util\.tokenize/.test(sp.text ?? "") && (app?.fanout?.sequence_head ?? []).includes("util.tokenize"), JSON.stringify(app?.fanout?.sequence_head));
  s.check("FO8", "unreached function (orphan_util) is reported, not dropped", /orphan_util/.test(sp.text ?? ""), "");
}
process.exit(s.finish() ? 1 : 0);
