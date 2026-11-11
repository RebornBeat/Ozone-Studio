// TPV-03 capability gaps + methodology refinement report. Read-only. Doctrine checks: no confidence fields, honest counts.
import { suite, api, keysDeep } from "./tpv.mjs";
const s = suite("tpv-03-gaps-refinement");
const g = (await api("GET", "/capability/gaps")).json;
s.check("G1", "/capability/gaps success with count + by_status + gaps", g?.success === true && Array.isArray(g.gaps) && typeof g.count === "number", `count=${g?.count} by_status=${JSON.stringify(g?.by_status)}`);
s.check("G2", "count equals the number of gap rows returned", g?.count === g?.gaps?.length, `${g?.count} vs ${g?.gaps?.length}`);
const gk = keysDeep(g);
s.check("G3", "no model-confidence field anywhere in the gaps payload (doctrine: yes/no only)", ![...gk].some((k) => k.includes("confidence")), [...gk].filter((k) => k.includes("confid")).join(","));
const created = (g?.gaps ?? []).filter((x) => String(x.status ?? "").includes("tool_created"));
s.pass("G4", "gaps whose tool was created automatically (provenance read, not inferred)", `${created.length}`);
const offerFill = (g?.gaps ?? []).filter((x) => JSON.stringify(x).includes("offer"));
s.check("G5", "no gap is filled by an offer (superseded: gaps are filled by creation)", offerFill.length === 0, `${offerFill.length} rows mention offer`);
const r = (await api("GET", "/methodology/refinement")).json;
s.check("R1", "/methodology/refinement success with report/actions/candidates", r?.success === true && "report" in r && "actions" in r && "candidates" in r, Object.keys(r ?? {}).join(","));
s.check("R2", "no confidence field in the refinement payload", ![...keysDeep(r)].some((k) => k.includes("confidence")), "");
const cands = Array.isArray(r?.candidates) ? r.candidates : [];
s.pass("R3", "candidate/action counts as read", `${cands.length} candidates, ${Array.isArray(r?.actions) ? r.actions.length : "n/a"} actions`);
if (!cands.length && !(r?.actions?.length)) s.skip("R4", "refinement acted on candidates", "no candidates/actions yet — the rule gate has not produced any (expected before the first walk after deploy)");
process.exit(s.finish() ? 1 : 0);
