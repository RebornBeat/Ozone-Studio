// TPV-07 tool registry + coordination surface as an outside agent. Read-only (no /mcp/call, no start/stop).
import { suite, api } from "./tpv.mjs";
const s = suite("tpv-07-tools-coordination");
const t = (await api("GET", "/mcp/tools")).json;
const tools = t?.tools ?? [];
s.check("T1", "registry lists tools", tools.length > 0, `${tools.length}`);
const created = tools.filter((x) => String(x.source ?? x.origin ?? "").includes("created") || String(x.name ?? "").startsWith("created_"));
s.pass("T2", "auto-created tool rows (host registry)", `${created.length}`);
const spawned = created.filter((x) => x.running === true);
s.check("T3", "no auto-created tool is running (host never spawns created tools; auto_start=false)", spawned.length === 0 && created.every((x) => x.auto_start !== true), `${spawned.length} running`);
const u = (await api("GET", "/mcp/usage")).json;
s.check("T4", "/mcp/usage answers", u && typeof u === "object", Object.keys(u ?? {}).slice(0, 6).join(","));
const p = (await api("GET", "/coordination/presence")).json;
s.check("T5", "/coordination/presence answers (real captured values or explicitly absent)", p && typeof p === "object", JSON.stringify(p).slice(0, 140));
const c = (await api("GET", "/coordination/claims")).json;
s.check("T6", "/coordination/claims answers", c && typeof c === "object", JSON.stringify(c).slice(0, 140));
process.exit(s.finish() ? 1 : 0);
