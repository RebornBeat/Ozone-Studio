// browser-mcp self-test (NEW_MCP_GUIDE §6: positive path AND refusal paths).
// Usage: node selftest.mjs <fixtureBaseUrl on an allowlisted host> [mcpPort=3280] [lockedPort=3282]
//   The MCP on <mcpPort> must run with OZONE_BROWSER_ALLOW_HOSTS=<fixture host> + OZONE_BROWSER_ALLOW_EVAL=1;
//   the one on <lockedPort> with NO env (proves locked-by-default). Fixture needs: #q input, a Search
//   submit button, #add button, and a link to a host that is NOT allowlisted.
const [, , FIX, P = "3280", LP = "3282"] = process.argv;
let pass = 0, fail = 0;
// OZONE_THROUGH_HOST=http://127.0.0.1:50051 routes EVERY call through the host's /mcp/call (metered,
// jurisdiction-gated, rippled) — the only door agents are supposed to use. The locked-by-default checks
// (port LP) always hit the second MCP directly because the host registry points at one endpoint.
const HOST = process.env.OZONE_THROUGH_HOST;
const call = async (port, tool, input = {}) => {
  if (HOST && String(port) === String(P)) {
    const r = await (await fetch(`${HOST}/mcp/call`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ tool, agent: "claude-code", input }) })).json();
    if (r.output && typeof r.output === "object" && "success" in r.output) return r.output.success ? { success: true, output: r.output.output } : { success: false, error: r.output.error };
    return { success: false, error: r.error ?? "host call failed" };
  }
  return (await fetch(`http://127.0.0.1:${port}/call`, { method: "POST", body: JSON.stringify({ tool, agent: "selftest", input }) })).json();
};
const ok = (name, cond, extra = "") => { cond ? pass++ : fail++; console.log(`${cond ? "PASS" : "FAIL"}  ${name}${extra ? "  — " + extra : ""}`); };

// refusal paths first
let r = await call(LP, "browser_open", { url: FIX });
ok("locked-by-default refuses open", !r.success && /LOCKED/.test(r.error), r.error);
r = await call(P, "browser_open", { url: "http://example.com/" });
ok("off-allowlist host refused", !r.success && /not in the allowlist/.test(r.error), r.error);
r = await call(P, "browser_open", { url: "javascript:alert(1)" });
ok("javascript: refused", !r.success && /scheme/.test(r.error), r.error);
r = await call(P, "browser_open", { url: "file:///etc/passwd" });
ok("file: refused", !r.success && /file:/.test(r.error), r.error);
r = await call(P, "browser_snapshot", { session: "nope" });
ok("unknown session refused", !r.success, r.error);

// positive path
r = await call(P, "browser_open", { url: FIX });
ok("open allowlisted page", r.success && r.output?.session, JSON.stringify(r.output));
const s = r.output?.session;
r = await call(P, "browser_snapshot", { session: s });
const snap = r.output?.untrusted_page_content;
ok("snapshot sees title + interactive elements", snap?.title && snap.elements?.length >= 3, `${snap?.title}, ${snap?.elements?.length} elements`);
ok("snapshot omits hidden text", !/hidden text/.test(snap?.text ?? ""));
const input = snap.elements.find((e) => e.tag === "input");
const searchBtn = snap.elements.find((e) => /^search$/i.test(e.text));
const addBtn = snap.elements.find((e) => /add laptop/i.test(e.text));
r = await call(P, "browser_type", { session: s, index: input.i, text: "RTX 4080" });
ok("type into input", r.success, JSON.stringify(r.output ?? r.error));
r = await call(P, "browser_click", { session: s, index: searchBtn.i });
ok("click search", r.success);
r = await call(P, "browser_snapshot", { session: s });
ok("page reacted to type+click", /searched: RTX 4080/.test(r.output.untrusted_page_content.text), r.output.untrusted_page_content.text.split("\n")[0]);
const snap2 = r.output.untrusted_page_content;
r = await call(P, "browser_click", { session: s, index: snap2.elements.find((e) => /add laptop/i.test(e.text)).i });
r = await call(P, "browser_eval", { session: s, expression: "document.getElementById('add').textContent" });
ok("eval (enabled) reads click result", r.success && r.output.untrusted_result === "Added!", JSON.stringify(r.output ?? r.error));
r = await call(P, "browser_screenshot", { session: s });
ok("screenshot written", r.success && r.output.bytes > 2000, `${r.output?.bytes} bytes @ ${r.output?.path}`);
// origin guard: click the external link (a host that is not allowlisted), then try to act on it
const ext = snap2.elements.find((e) => /external link/i.test(e.text));
await call(P, "browser_click", { session: s, index: ext.i });
r = await call(P, "browser_snapshot", { session: s });
ok("origin guard blocks acting on an off-allowlist page", !r.success && /origin guard/.test(r.error), r.error);
r = await call(P, "browser_eval", { session: s, expression: "1+1" });
ok("origin guard also blocks eval there", !r.success && /origin guard/.test(r.error), r.error);
r = await call(P, "browser_navigate", { session: s, url: FIX });
ok("navigating back to allowlisted works", r.success);
r = await call(P, "browser_close", { session: s });
ok("close", r.success);
r = await call(LP, "browser_eval", { session: "x", expression: "1" });
ok("eval disabled by default", !r.success && /disabled/.test(r.error), r.error);
console.log(`\n${pass} passed, ${fail} failed`);
process.exit(fail ? 1 : 0);
