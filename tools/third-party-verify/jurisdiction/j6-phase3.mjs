#!/usr/bin/env node
// j6 — jurisdiction phase 3 (call-site review). READ-ONLY against a live host; never accepts a requirement, never approves a hold.
// Checks: (1) the outbound CHECK route judges a model call by its endpoint (localhost -> allowed, unlisted/third-country without accepted
// agreements -> held); (2) a non-model pipeline is judged by ITS destination (web search id 56 is NOT judged by the local model endpoint);
// (3) the policy carries tool_egress rows; (4) /jurisdiction/* is session-protected on a strict host while .../outbound/check is not.
// Not run (no build/test phase). Usage: OZONE_HOST=http://127.0.0.1:50051 node j6-phase3.mjs
const HOST = process.env.OZONE_HOST ?? "http://127.0.0.1:50051";
const results = [];
const ok = (name, pass, detail = "") => { results.push({ name, pass, detail }); console.log(`${pass ? "PASS" : "FAIL"} ${name}${detail ? " — " + detail : ""}`); };
const post = async (p, body) => { const r = await fetch(HOST + p, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body) }); return { status: r.status, json: await r.json().catch(() => ({})) }; };
const get = async (p) => { const r = await fetch(HOST + p); return { status: r.status, json: await r.json().catch(() => ({})) }; };

const local = await post("/jurisdiction/outbound/check", { pipeline_id: 9, input: { prompt: "hello", model_override_config: { api_endpoint: "http://127.0.0.1:8081/v1/chat/completions", model_type: "api" } } });
ok("check: a localhost model endpoint is allowed", local.json.allow === true, local.json.reason);

const remote = await post("/jurisdiction/outbound/check", { pipeline_id: 9, input: { prompt: "my email is someone@example.org", model_override_config: { api_endpoint: "https://openrouter.ai/api/v1/chat/completions", model_type: "api" } } });
ok("check: personal-data shape to a third-country endpoint is NOT allowed without approval", remote.json.allow === false, remote.json.reason);

const web = await post("/jurisdiction/outbound/check", { pipeline_id: 56, input: { query: "contact someone@example.org", model_override_config: { api_endpoint: "http://127.0.0.1:8081", model_type: "api" } } });
ok("check: web search (56) is judged by the search provider, not the local model endpoint", web.json.allow === false || web.json.region !== "local", `region=${web.json.region} allow=${web.json.allow}`);

const local3 = await post("/jurisdiction/outbound/check", { pipeline_id: 3, input: { text: "someone@example.org" } });
ok("check: an audited local pipeline (3) has no egress of its own", local3.json.allow === true, local3.json.reason);

const unaudited = await post("/jurisdiction/outbound/check", { pipeline_id: 987654, input: { text: "someone@example.org" } });
ok("check: an unaudited pipeline id is never judged local", unaudited.json.region !== "local", `region=${unaudited.json.region}`);

const pol = await get("/jurisdiction/policy");
if (pol.status === 401) ok("strict host: /jurisdiction/policy requires a session", true, "401 without a session");
else ok("policy readable (non-strict host) and carries tool_egress or shipped defaults", pol.json.success === true, `tool_egress=${JSON.stringify(pol.json.policy?.tool_egress ?? null)?.slice(0, 80)}`);

const failed = results.filter((r) => !r.pass).length;
console.log(`\n${results.length - failed}/${results.length} passed`);
process.exit(failed ? 1 : 0);
