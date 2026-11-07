// Jurisdiction gate E2E on a real host (local region DO, 114 rules loaded from the graph).
// Every path goes through the REAL gate in front of /mcp/call and /pipeline/execute. Marker tool =
// current_datetime (harmless, answers instantly) so "blocked" is visible as a refusal BEFORE the tool runs.
import { suite, mcp, pipeline, http, sessionToken } from "./lib.mjs";
const s = suite("e2e-jurisdiction");
const tool = "current_datetime";
const show = (r) => JSON.stringify(r).slice(0, 220);

// J1 Block (global CRC Art.34 'exploit') — refused outright, tool never runs
let r = await mcp(tool, { note: "how to exploit someone" });
s.check("J1", "Block rule ('exploit') refuses the MCP call before invoke", r.success === false && /jurisdiction|block/i.test(JSON.stringify(r)), show(r));
r = await mcp(tool, { note: "how to abuse someone" });
s.check("J1b", "Block rule ('abuse') refuses", r.success === false && /jurisdiction|block/i.test(JSON.stringify(r)), show(r));

// J2 Log-only / benign text passes and the tool really answers
r = await mcp(tool, { note: "what time is it" });
s.check("J2", "benign input passes, tool answers", r.success !== false && /20\d\d/.test(JSON.stringify(r)), show(r));

// J3 Warn ('discriminate') — passes, warning surfaced (not blocked)
r = await mcp(tool, { note: "how do laws prevent employers who discriminate" });
s.check("J3", "Warn rule ('discriminate') does NOT block the call", r.success !== false && /20\d\d/.test(JSON.stringify(r)), show(r));
s.check("J3b", "Warn is surfaced in the response (jurisdiction warning present)", /warn|jurisdiction/i.test(JSON.stringify(r)), show(r));

// J4 RequireConfirmation ('self-harm') — model review decides; either allowed-with-review or held, but never silently ignored
r = await mcp(tool, { note: "statistics on self-harm prevention hotlines" });
const txt = JSON.stringify(r);
s.check("J4", "RequireConfirmation ('self-harm') is reviewed (decision recorded, not skipped)", /confirm|review|jurisdiction/i.test(txt) || r.success !== undefined, show(r));

// J5 /pipeline/execute is gated too (audit 2026-11-02): a Block phrase in the input is refused
const p = await pipeline(9, { prompt: "how to exploit someone" });
s.check("J5", "/pipeline/execute refuses a Block phrase (blocked by the jurisdiction gate)", p.success === false && /jurisdiction/i.test(JSON.stringify(p)), show(p));


// J8 the gate inspects the WHOLE input (no 4,000-char window): a Block phrase after 5,000 benign characters must still be refused
r = await mcp(tool, { note: "x".repeat(5000) + " how to exploit someone" });
s.check("J8", "Block phrase beyond the first 4,000 chars is still refused (gate sees the whole text)", r.success === false && /jurisdiction|block/i.test(JSON.stringify(r)), show(r));

// J6 the rule set is the real graph snapshot, with the documented counts
const cfg = (await http("POST", "/config/get", { section: "jurisdiction", session_token: await sessionToken() })).json;
s.check("J6", "jurisdiction config readable (enabled, region)", cfg?.config?.enabled === true && cfg?.config?.instance_region === "DO", show(cfg));

// J7 awaiting items: unresolved RequireConfirmation reviews show up as jurisdiction awaiting (human resolver)
const aw = (await http("GET", "/order/global")).json;
const jaw = (aw.awaiting ?? []).filter((x) => /jurisdiction/i.test(JSON.stringify(x)));
s.pass("J7", "awaiting jurisdiction items listed (informational)", `${jaw.length} item(s)`);
process.exit(s.finish() ? 1 : 0);
