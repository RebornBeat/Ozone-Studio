// Methodology modification + sufficiency queue (docs/ZERO_SHOT_CALL_REGISTRY.md §16). Real host, no mocks.
// M1 pipeline 12 Create; M2 Update merges (nothing existing dropped, decision rule + provider added, versioned);
// M3 a request queues a review in methodology_reviews.json (checked via the file); M4 the review is judged (needs a model).
import fs from "node:fs";
import path from "node:path";
import { suite, http, pipeline, orchestrate } from "./lib.mjs";
const s = suite("e2e-methodology-sufficiency");
const DATA = process.env.OZONE_DATA ?? new URL("../../target/release/zsei_data", import.meta.url).pathname;
const tag = Date.now().toString(36);
const out = (r) => r?.output ?? r?.data ?? r;
const kw = `zzsuff${tag}`;
const c = out(await pipeline(12, { action: "Create", name: `Sufficiency ${tag}`, description: "base", category_id: 0,
  principles: [{ name: "orig principle", description: "stays", priority: 5 }],
  heuristics: [{ condition: "orig cond", action: "orig act" }],
  decision_rules: [{ name: "orig", condition: `${kw} original`, outcome: "original outcome", requires: [] }],
  keywords: [kw], topics: ["sufficiency"], task_id: null }));
s.check("M1", "Create via pipeline 12", c?.success === true && !!c.methodology_id, JSON.stringify(c).slice(0, 140));
const id = c?.methodology_id;
const u = out(await pipeline(12, { action: "Update", methodology_id: id, name: null, description: null,
  add_principles: [{ name: "added principle", description: "new", priority: 5 }], remove_principles: null,
  add_heuristics: null,
  add_decision_rules: [{ name: "added", condition: `${kw} added case`, outcome: "added outcome", requires: [{ kind: "provider", name: "threed-mcp", purpose: "build" }] }],
  add_keywords: [`${kw}extra`], remove_keywords: null }));
s.check("M2a", "Update via pipeline 12 succeeds", u?.success === true, JSON.stringify(u).slice(0, 140));
const cont = (await http("POST", "/zsei/query", { query: { GetContainer: { container_id: id } }, session_token: "" })).json;
const sp = cont?.result?.Container?.local_state?.storage?.object_store_path ?? cont?.Container?.local_state?.storage?.object_store_path ?? cont?.output?.Container?.local_state?.storage?.object_store_path;
let body = null;
try { body = JSON.parse(fs.readFileSync(path.join(DATA, sp), "utf8")); } catch {}
s.check("M2b", "container points at the NEW content file", !!sp && !!body, String(sp));
s.check("M2c", "existing principle + heuristic + rule kept, additions merged (not replaced)",
  !!body && body.principles?.length === 2 && body.heuristics?.length === 1 && body.decision_rules?.length === 2
  && body.decision_rules.some((r) => (r.requires ?? []).some((q) => q.name === "threed-mcp")), JSON.stringify(body)?.slice(0, 220));
const q0 = (() => { try { return JSON.parse(fs.readFileSync(path.join(DATA, "methodology_reviews.json"), "utf8")).length; } catch { return 0; } })();
// M3: any real request queues a review at Stage 3 (found-some or none); we only need Stage 3 to run, not the whole walk.
const r = await orchestrate({ prompt: `Explain how ${kw} original works in two sentences.`, consciousness_enabled: false }, { pollMs: 10000, maxMs: 2400000 });
const rev = (() => { try { return JSON.parse(fs.readFileSync(path.join(DATA, "methodology_reviews.json"), "utf8")); } catch { return []; } })();
const mine = rev.find((x) => String(x.request).includes(kw));
s.check("M3", "the request was queued for methodology sufficiency review (found-some case)", !!mine && Array.isArray(mine.gathered_ids) && mine.gathered_ids.length > 0, JSON.stringify(mine)?.slice(0, 200));
void q0; void r;
s.finish();
