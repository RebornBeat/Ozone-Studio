// KEYWORD PROGRAM E2E (docs/KEYWORD_CAPTURE_AND_ML_GUIDE.md) — real host, real pipeline 17, no mocks of the system under test.
// Rows: search strategies (substring flood vs token-aware), ranked-order determinism, the hit log, pipeline 17's model actions
// (status / prepared slots / honest unavailability / training contract on a CRAFTED dataset in a temp dir — a contract test of the real
// trainer, NOT evidence that the model helps real queries; that evidence comes only from held-out real hits), and the retired stubs.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { suite, http, pipeline } from "./lib.mjs";

const s = suite("e2e-keyword-program");
const DATA = process.env.OZONE_DATA ?? new URL("../../target/release/zsei_data", import.meta.url).pathname;
const out = (r) => r?.output ?? r?.data ?? r;
const search = async (keywords, strategy) =>
  (await http("POST", "/zsei/query", { query: { SearchContainersByKeywords: { keywords, container_type: null, strategy } }, session_token: "" })).json?.result?.Containers ?? [];

// ---- K1: the substring flood is gone under the token strategy (same store, same terms)
// (token searches the NAME as well as keywords/topics, so it can legitimately return containers `scan` never looked at — e.g. generated
// names "<x> code graph (…)" — so the claim is not token ≤ scan; it is: no exact match is lost, and substring-only noise is dropped)
const flood = {};
const exactKept = {};
for (const term of ["file", "code", "review", "graph"]) {
  const [scan, token, exact] = [await search([term], null), await search([term], "token"), await search([term], "exact")];
  flood[term] = { scan: scan.length, token: token.length, exact: exact.length };
  const tset = new Set(token);
  exactKept[term] = exact.every((id) => tset.has(id));
}
s.check("K1", "token keeps every exact match and drops the substring flood ('file': far fewer than scan)", Object.values(exactKept).every(Boolean) && flood.file.token * 2 < flood.file.scan, JSON.stringify({ flood, exactKept }));

// ---- K2: ranked order is deterministic
const a = await search(["code review", "static analysis"], "token");
const b = await search(["code review", "static analysis"], "token");
s.check("K2", "the ranked token search returns the same ORDER twice (deterministic v0 key)", a.length === b.length && a.every((id, i) => id === b[i]), `${a.length} ids`);

// ---- K3: every search is logged as a hit event (the training data)
const hitsPath = path.join(DATA, "keyword_hits.jsonl");
const before = fs.existsSync(hitsPath) ? fs.statSync(hitsPath).size : 0;
const marker = `kwprog${Date.now().toString(36)}`;
await search(["code", marker], "token");
await new Promise((r) => setTimeout(r, 500));
const after = fs.existsSync(hitsPath) ? fs.statSync(hitsPath).size : 0;
const tail = fs.existsSync(hitsPath) ? fs.readFileSync(hitsPath, "utf8").split("\n").filter(Boolean).slice(-5).join("\n") : "";
s.check("K3", "a search appends a `search` hit event with its terms and returned ids", after > before && tail.includes('"event":"search"') && tail.includes(marker), `bytes ${before} -> ${after}`);

// ---- K4: pipeline 17 model status shows the PREPARED slots (pretrained fine-tune, embedder) — slot exists, backend not runnable
const modelsDir = path.join(os.tmpdir(), `kwprog-models-${Date.now()}`);
const st = out(await pipeline(17, { action: "ModelStatus", models_dir: modelsDir }));
const prepared = (st?.registry?.models ?? []).filter((m) => m.status === "prepared").map((m) => m.id);
s.check("K4", "ModelStatus lists the prepared pretrained-finetune and embedder slots", prepared.includes("pretrained-finetune") && prepared.includes("embedder"), JSON.stringify(st?.registry?.models?.map((m) => [m.id, m.status])));

// ---- K5: the prepared backends refuse HONESTLY instead of fabricating
const emb = out(await pipeline(17, { action: "TrainEmbedder" }));
const ft = out(await pipeline(17, { action: "FineTunePretrained" }));
s.check("K5", "TrainEmbedder / FineTunePretrained report success:false, available:false and what they require", emb?.success === false && emb?.available === false && ft?.success === false && ft?.available === false && Array.isArray(ft?.requires), JSON.stringify({ emb: emb?.error?.slice(0, 80), ft: ft?.error?.slice(0, 80) }));

// ---- K6: training refuses when there is nothing to learn from, and trains/registers on a labelled dataset (CONTRACT test, temp dir)
const emptyDs = path.join(os.tmpdir(), `kwprog-empty-${Date.now()}.jsonl`);
fs.writeFileSync(emptyDs, "");
const none = out(await pipeline(17, { action: "TrainModel", dataset_path: emptyDs, models_dir: modelsDir }));
s.check("K6a", "TrainModel on an empty dataset trains nothing and says why (models untouched)", none?.success === true && none?.report?.trained === false && none?.report?.activated === false, String(none?.report?.message).slice(0, 120));
const mk = (id, label, term) => ({ id, terms: [term], facts: { in_scope: true, class: "WholeToken", matched_terms: 1, validated: false, applied: 0, kept: 0, packed: 0, returned: 0, dropped: 0, hops: null, last_accessed: 0 }, label });
const groups = Array.from({ length: 12 }, (_, n) => ({
  group: `g${n}`, heldout: n >= 9, query_terms: ["static analysis"],
  candidates: [mk(1, true, "code review"), mk(2, false, "pasta recipe"), mk(3, true, "linting"), mk(4, false, "garden")],
}));
const ds = path.join(os.tmpdir(), `kwprog-ds-${Date.now()}.jsonl`);
fs.writeFileSync(ds, groups.map((g) => JSON.stringify(g)).join("\n") + "\n");
const tr = out(await pipeline(17, { action: "TrainModel", dataset_path: ds, models_dir: modelsDir, params: { dim: 8, buckets: 1024, encoder_epochs: 4, ranker_epochs: 5 } }));
const rep = tr?.report;
const reg = JSON.parse(fs.existsSync(path.join(modelsDir, "registry.json")) ? fs.readFileSync(path.join(modelsDir, "registry.json"), "utf8") : "{}");
s.check("K6b", "TrainModel on a labelled dataset trains, evaluates on the held-out groups, and registers encoder + ranker with the MEASURED decision", tr?.success === true && rep?.trained === true && rep?.train_groups === 9 && rep?.heldout_groups === 3 && (reg.models ?? []).some((m) => m.kind === "hashed_ngram") && (reg.models ?? []).some((m) => m.kind === "ranker"), JSON.stringify({ activated: rep?.activated, wins: rep?.decision?.wins, losses: rep?.decision?.losses, p: rep?.decision?.p_value, reason: rep?.decision?.reason }));

// ---- K7: inference through the pipeline: fusion falls back to / honours v0 and returns every candidate exactly once
const cands = [1, 2, 3].map((id) => ({ id, terms: ["x"], facts: { in_scope: true, class: id === 2 ? "ExactTerm" : "WholeToken", matched_terms: 1, validated: false, applied: 0, kept: 0, packed: 0, returned: 0, dropped: 0, hops: null, last_accessed: 0 } }));
const rk = out(await pipeline(17, { action: "RankCandidates", models_dir: path.join(os.tmpdir(), `kwprog-none-${Date.now()}`), query_terms: ["x"], candidates: cands }));
s.check("K7", "RankCandidates with no active model returns the deterministic v0 order (exact term first) with every candidate once", rk?.success === true && rk?.order?.[0] === 2 && [...rk.order].sort().join() === "1,2,3" && rk?.backends?.ranker === null, JSON.stringify(rk?.order));

// ---- K8: the former keyword-overlap stubs are gone: honest refusal, not an empty success
const sem = out(await pipeline(17, { action: "TraverseSemantic", start_container: 1, similarity_threshold: 0.5, max_results: 5 }));
s.check("K8", "TraverseSemantic refuses honestly (success:false + pointer to RankCandidates), no fake empty success", sem?.success === false && /RankCandidates/.test(String(sem?.error)), String(sem?.error).slice(0, 120));

// ---- K9: pipeline 47 is consciousness-category — the external contract refuses it by design, so it cannot be exercised through /pipeline/execute.
// Its envelope fix is verified where it actually runs: Stage 12 of a real walk (tools/e2e/walk-completion.mjs reads the Stage 12 line).
s.skip("K9", "pipeline 47 accepts the {data, context} envelope", "consciousness pipelines are internal meta, not callable via external contracts — verified by Stage 12 of a real walk");

process.exit(s.finish() ? 1 : 0);
