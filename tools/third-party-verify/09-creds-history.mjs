// TPV-09 credentials + history surface (2026-11-12). READ-ONLY: never restores, never mutates. Uses only the public HTTP surface.
// Checks (a) protected routes refuse an anonymous caller on a strict host and accept the device session, (b) the history read routes and the
// held-calls / open-agent views answer with the documented shapes. On a non-strict host (OZONE_REQUIRE_SESSION off) the refusal checks are SKIPped.
import { suite, HOST, http, api } from "./tpv.mjs";
const s = suite("tpv-09-creds-history");

let strict = false;
try {
  // /mcp/tools is a long-protected route: an anonymous 401 means strict mode is on
  const anon = await fetch(HOST + "/mcp/tools");
  strict = anon.status === 401;
  s.pass("C0", "strict-session state read", `anonymous GET /mcp/tools -> HTTP ${anon.status} (strict=${strict})`);
} catch (e) { s.fail("C0", "host reachable", String(e)); process.exit(s.finish() ? 1 : 0); }

const anonymous = [
  ["C1", "POST", "/history/status", {}],
  ["C2", "POST", "/pipelines/unregister", { pipeline_id: 0 }],
  ["C3", "POST", "/orchestration/node-event", {}],
  ["C4", "POST", "/monitor/activity", { kind: "tpv", level: "info", source: "tpv-09", message: "anonymous write probe" }],
  ["C5", "POST", "/consciousness/review_pass", {}],
  ["C6", "POST", "/pipeline/cancel", { pipeline_id: 0, session_token: "" }],
  ["C7", "GET", "/jurisdiction/held"],
  ["C8", "GET", "/agent/open"],
];
for (const [id, method, p, body] of anonymous) {
  if (!strict) { s.skip(id, `${method} ${p} refuses an anonymous caller`, "host is not in strict-session mode"); continue; }
  try {
    const r = await fetch(HOST + p, { method, headers: { "content-type": "application/json" }, body: body ? JSON.stringify(body) : undefined });
    s.check(id, `${method} ${p} refuses an anonymous caller`, r.status === 401, `HTTP ${r.status}`);
  } catch (e) { s.fail(id, `${method} ${p}`, String(e)); }
}
if (strict) {
  try {
    const r = await fetch(HOST + "/orchestrate", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ prompt: "tpv anonymous probe", user_id: 1, device_id: 1, session_token: "", submit_only: true }) });
    const j = await r.json();
    s.check("C9", "POST /orchestrate refuses a caller with no credential (and starts nothing)", j.accepted !== true && j.success === false && /session/i.test(String(j.error ?? "")), JSON.stringify({ accepted: j.accepted, success: j.success, error: j.error }));
  } catch (e) { s.fail("C9", "POST /orchestrate anonymous", String(e)); }
}

// with the device session
try {
  const st = await api("POST", "/history/status", {});
  s.check("H1", "POST /history/status answers {success, items[]}", st.status === 200 && st.json?.success === true && Array.isArray(st.json?.items), `HTTP ${st.status}, ${st.json?.items?.length ?? "?"} item(s)`);
  const item = st.json?.items?.[0];
  if (!item) s.skip("H2", "history list/diff/read on a real item", "no item has history yet");
  else {
    const l = await api("POST", "/history/list", { scope: item.scope, id: item.id });
    s.check("H2", "POST /history/list returns versions[] with v/ts/reason/size/hash", l.json?.success === true && Array.isArray(l.json?.versions) && l.json.versions.length === item.versions, `${item.scope}/${item.id}: ${l.json?.versions?.length} version(s), status says ${item.versions}`);
    const last = l.json?.versions?.at(-1);
    if (last) {
      const rd = await api("POST", "/history/read", { scope: item.scope, id: item.id, version: last.v });
      s.check("H3", "POST /history/read returns the whole content of the newest version (total == size)", rd.json?.success === true && rd.json?.total === last.size, `total ${rd.json?.total}, recorded size ${last.size}`);
      const half = Math.floor(last.size / 2);
      if (half > 0) {
        const part = await api("POST", "/history/read", { scope: item.scope, id: item.id, version: last.v, offset: 0, length: half, encoding: "base64" });
        s.check("H4", "chunked read returns exactly the requested length", part.json?.success === true && part.json?.returned === half, `returned ${part.json?.returned} of requested ${half}`);
      }
      const d = await api("POST", "/history/diff", { scope: item.scope, id: item.id });
      s.check("H5", "POST /history/diff answers (hunks, binary summary, or a one-version note)", d.json?.success === true, JSON.stringify(Object.keys(d.json?.diff ?? {})));
    }
  }
  const bad = await api("POST", "/history/list", { scope: "../x", id: "1" });
  s.check("H6", "an invalid scope is refused (no path traversal)", bad.json?.success === false, JSON.stringify(bad.json));
  // restore without the confirm phrase must refuse (never actually restores)
  const nr = await api("POST", "/history/restore", { scope: "file", id: "0", version: 1 });
  s.check("H7", "POST /history/restore without confirm:\"RESTORE\" refuses", nr.json?.success === false && /RESTORE/.test(String(nr.json?.error ?? "")), JSON.stringify(nr.json));
  const hc = await api("GET", "/jurisdiction/held");
  s.check("H8", "GET /jurisdiction/held answers {held[], unaccepted_requirements[]}", hc.json?.success === true && Array.isArray(hc.json?.held) && Array.isArray(hc.json?.unaccepted_requirements), `held ${hc.json?.held?.length}, unaccepted ${hc.json?.unaccepted_requirements?.length}`);
  const ao = await api("GET", "/agent/open");
  s.check("H9", "GET /agent/open answers {requests[]}", ao.json?.success === true && Array.isArray(ao.json?.requests), `${ao.json?.requests?.length ?? "?"} open request(s)`);
} catch (e) { s.fail("H0", "authenticated history/held checks", String(e)); }
process.exit(s.finish() ? 1 : 0);
