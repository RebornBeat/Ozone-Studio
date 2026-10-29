// Agent-style walkthrough of the REAL Ozone web UI through the governed door (/mcp/call):
//   perceive (snapshot) -> decide (by label, not hard-coded selector) -> act (click/type) -> verify -> screenshot.
// Prereqs: host on $OZONE_HOST, browser-mcp registered with ALLOW_HOSTS=localhost + ALLOW_GLOBALS=OZONE_HOST_URL,
// and `node ui-harness.mjs 8767 <host>` running. Usage: node demo_ozone_ui.mjs
const HOST = process.env.OZONE_HOST ?? "http://127.0.0.1:50051";
const UI = process.env.OZONE_UI ?? "http://localhost:8767";
const call = async (tool, input) => {
  const r = await (await fetch(`${HOST}/mcp/call`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ tool, agent: "claude-code", input }) })).json();
  if (!r.output?.success) throw new Error(`${tool}: ${r.output?.error ?? r.error}`);
  return r.output.output;
};
const snap = async (s) => (await call("browser_snapshot", { session: s })).untrusted_page_content;
const find = (sn, re) => { const e = sn.elements.find((x) => re.test(x.text)); if (!e) throw new Error(`no element matching ${re}; have: ${sn.elements.map((x) => x.text).filter(Boolean).slice(0, 25).join(" | ")}`); return e; };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const shots = [];
const { session: s } = await call("browser_open", { url: `${UI}/index.html`, width: 1500, height: 900, globals: { OZONE_HOST_URL: UI } });
try {
  await sleep(5000);
  let sn = await snap(s);
  console.log(`1. perceived "${sn.title}": ${sn.elements.length} interactive elements`);
  await call("browser_click", { session: s, index: find(sn, /^Workspace$/).i }); await sleep(2500);
  sn = await snap(s);
  console.log("2. clicked Workspace ->", sn.outline.filter((o) => /^(main|h\d|aside)/.test(o)).slice(-3).join(" ; "));
  shots.push((await call("browser_screenshot", { session: s })).path);
  const plus = sn.elements.find((e) => /^\+$/.test(e.text));
  if (plus) {
    await call("browser_click", { session: s, index: plus.i }); await sleep(600);
    sn = await snap(s);
    const nameBox = sn.elements.find((e) => e.tag === "input" && /name/i.test(e.text + (e.placeholder ?? "")));
    const box = nameBox ?? sn.elements.filter((e) => e.tag === "input").pop();
    await call("browser_type", { session: s, index: box.i, text: "Agent-driven Workspace" });
    await call("browser_click", { session: s, index: find(await snap(s), /^Create/).i }); await sleep(1500);
    sn = await snap(s);
    console.log("3. created a workspace through the UI; sidebar now shows:", sn.text.includes("Agent-driven Workspace") ? "the new workspace ✓" : "(not found)");
    shots.push((await call("browser_screenshot", { session: s })).path);
  }
  console.log("screenshots:\n  " + shots.join("\n  "));
} finally { await call("browser_close", { session: s }).catch(() => {}); }
