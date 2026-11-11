// TPV-01 health + session + public surface reachability. Read-only.
import { suite, HOST, http, api } from "./tpv.mjs";
const s = suite("tpv-01-surface");
let h;
try { h = (await http("GET", "/health")).json; } catch (e) { s.fail("S0", "host reachable", String(e)); process.exit(s.finish() ? 1 : 0); }
s.check("S1", "/health reports healthy", h?.healthy === true, JSON.stringify({ version: h?.version, uptime_secs: h?.uptime_secs, active_tasks: h?.active_tasks, peer_count: h?.peer_count, p2p_enabled: h?.p2p_enabled }));
let authed = false;
try { const r = await api("GET", "/mcp/tools"); authed = r.status === 200 && Array.isArray(r.json?.tools); s.check("S2", "device-key session authenticates and a protected route answers", authed, `HTTP ${r.status}, ${r.json?.tools?.length ?? "?"} tools`); } catch (e) { s.fail("S2", "device-key session authenticates", String(e)); }
// every read route used by the other TPV scripts must answer (status only; bodies are checked in their own scripts)
for (const [id, p] of [["S3a", "/capability/gaps"], ["S3b", "/methodology/refinement"], ["S3c", "/order/global"], ["S3d", "/marketplace/overview"], ["S3e", "/ripple/meta/status"], ["S3f", "/coordination/presence"], ["S3g", "/orchestrate/status?run_token=tpv-none"]]) {
  try { const r = await api("GET", p); s.check(id, `GET ${p} answers (not 404/5xx)`, r.status < 400 || (p.startsWith("/orchestrate/status") && r.status === 200), `HTTP ${r.status}`); } catch (e) { s.fail(id, `GET ${p}`, String(e)); }
}
process.exit(s.finish() ? 1 : 0);
