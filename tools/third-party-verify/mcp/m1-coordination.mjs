// MCP-1 three agents (Claude Code, ZCode, a third) coordinate through ozone-shared-context while using the ozone-mcp tools.
// Read-mostly: only coordination state in a THROWAWAY context dir is written. Needs a running host for the ozone-mcp reads.
import { threeAgents, suite } from "./harness.mjs";
const s = suite("m1-coordination");
const { agents, close } = await threeAgents();
const [cc, zc, third] = [agents["claude-code"], agents["zcode"], agents["third-agent"]];
try {
  // tool surface: read vs mutating is stated in every description; decide/accept must not exist
  const tools = await cc.ozone.tools();
  const names = tools.map((t) => t.name);
  s.pass("C1", "ozone-mcp tool list as served", names.join(","));
  s.check("C2", "marketplace decide/accept and task cancel are NOT exposed", !names.some((n) => /decide|accept|cancel|delete|unlink/.test(n)), "");
  s.check("C3", "every tool states READ or MUTATING", tools.every((t) => /^(READ|MUTATING)\./.test(t.description)), "");

  // presence + claims: the no-overwrite protocol across three live agents
  for (const [n, a] of Object.entries(agents)) await a.shared.call("presence_heartbeat", { agent: n, role: "agent", task: "mcp-1" });
  const live = (await third.shared.call("presence_list")).json?.live ?? [];
  s.check("P1", "all three agents are live to each other", ["claude-code", "zcode", "third-agent"].every((n) => live.some((l) => l.agent === n)), live.map((l) => l.agent).join(","));
  const c1 = await cc.shared.call("file_claim", { agent: "claude-code", files: ["demo/a.rs"], reason: "mcp-1" });
  s.check("P2", "first claim granted", c1.json?.ok === true, c1.text.slice(0, 120));
  const c2 = await zc.shared.call("file_claim", { agent: "zcode", files: ["demo/a.rs"], reason: "mcp-1" });
  s.check("P3", "second agent's claim on the same file is refused (as reported by the server)", c2.json?.ok === false || !c2.ok, c2.text.slice(0, 160));
  await cc.shared.call("file_release", { agent: "claude-code", files: ["demo/a.rs"] });
  const c3 = await zc.shared.call("file_claim", { agent: "zcode", files: ["demo/a.rs"], reason: "mcp-1 after release" });
  s.check("P4", "after release the other agent can claim", c3.json?.ok === true, c3.text.slice(0, 120));
  await zc.shared.call("file_release", { agent: "zcode", files: ["demo/a.rs"] });

  // handoff visible to the third agent
  await cc.shared.call("note_add", { agent: "claude-code", kind: "handoff", title: "mcp-1 handoff", body: "continue from here" });
  const notes = (await third.shared.call("notes_list", { limit: 10 })).json ?? [];
  s.check("H1", "third agent reads the handoff written by another", (notes.notes ?? notes).some?.((n) => n.title === "mcp-1 handoff" && n.agent === "claude-code"), "");

  // identity is validated, not trusted blindly
  const bad = await cc.ozone.call("ozone_health", { agent: "bad agent; rm -rf" });
  s.check("I1", "invalid agent identity is rejected loudly", !bad.ok && /agent must match/.test(bad.text), bad.text.slice(0, 120));

  // host reads from three agents (same session surface, separate identities)
  for (const [n, a] of Object.entries(agents)) {
    const h = await a.ozone.call("ozone_health");
    s.check(`R-${n}`, "health read through ozone-mcp", h.ok, h.text.slice(0, 120));
  }
  // an unknown tool and a non-read ZSEI variant are refused with real errors
  const q = await zc.ozone.call("ozone_query", { query: { DeleteContainer: { container_id: 1 } } });
  s.check("Q1", "non-read ZSEI variant refused", !q.ok && /not a read-only/.test(q.text), q.text.slice(0, 140));
  const chunk = await zc.ozone.call("ozone_callgraph_system", { offset: 0, length: 200 });
  s.check("Q2", "chunked read returns a bounded chunk with next_offset/total_chars", chunk.ok && chunk.json && "total_chars" in chunk.json, chunk.text.slice(0, 120));
} finally { close(); }
process.exit(s.finish() ? 1 : 0);
