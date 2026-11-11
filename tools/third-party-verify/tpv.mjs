// Third-party verification helpers (2026-11-11).
// Rule: a verifier of Ozone-Studio acts as an OUTSIDE client. It uses only the public HTTP surface
// (same device-key session auth as tools/ozone-cli and tools/e2e/lib.mjs) and never reads or edits
// the code tree, the data dir, or the store directly. No mocks, no fabricated numbers: every
// PASS/FAIL row carries the value that was actually read, and a thing that is absent is reported
// as SKIP/FAIL with the reason, never defaulted.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { HOST, http, sessionToken } from "../e2e/lib.mjs";

export { HOST, http, sessionToken };
const HERE = path.dirname(fileURLToPath(import.meta.url));
const EVIDENCE_DIR = path.resolve(HERE, "../../docs/evidence/third-party-verify");

/** HTTP call with the strict-session Bearer header (accepted whether or not OZONE_REQUIRE_SESSION=1). */
export async function api(method, p, body) {
  const tok = await sessionToken();
  return http(method, p, body, { authorization: `Bearer ${tok}` });
}

/** POST /zsei/query — the same read surface the UI uses. Returns the unwrapped result or throws. */
export async function zsei(query) {
  const r = await api("POST", "/zsei/query", { query, session_token: "" });
  if (r.json?.success === false) throw new Error(r.json.error || "zsei query failed");
  return r.json?.result ?? r.json;
}

/** Recursively collect object keys (lower-cased) — used for the "no confidence" doctrine check. */
export function keysDeep(v, out = new Set()) {
  if (Array.isArray(v)) v.forEach((x) => keysDeep(x, out));
  else if (v && typeof v === "object") for (const [k, x] of Object.entries(v)) { out.add(k.toLowerCase()); keysDeep(x, out); }
  return out;
}

export function suite(name) {
  const rows = [];
  const date = new Date().toISOString().slice(0, 10);
  const t0 = Date.now();
  const say = (tag, id, what, ev) => { rows.push([tag, id, what, ev]); console.log(`${tag.padEnd(5)} ${id}  ${what}${ev ? "  — " + String(ev).slice(0, 200) : ""}`); };
  return {
    pass: (id, what, ev = "") => say("PASS", id, what, ev),
    fail: (id, what, ev = "") => say("FAIL", id, what, ev),
    skip: (id, what, why) => say("SKIP", id, what, why),
    check(id, what, ok, ev = "") { ok ? this.pass(id, what, ev) : this.fail(id, what, ev); },
    finish() {
      const c = (k) => rows.filter((r) => r[0] === k).length;
      const md = `# ${name} — ${date}\n\nhost: ${HOST} · ${Math.round((Date.now() - t0) / 1000)}s · PASS ${c("PASS")} · FAIL ${c("FAIL")} · SKIP ${c("SKIP")}\n\n| result | id | check | evidence (value actually read) |\n|---|---|---|---|\n` +
        rows.map((r) => `| ${r[0]} | ${r[1]} | ${r[2]} | ${String(r[3]).replace(/\|/g, "\\|").replace(/\n/g, " ").slice(0, 300)} |`).join("\n") + "\n";
      fs.mkdirSync(EVIDENCE_DIR, { recursive: true });
      const file = path.join(EVIDENCE_DIR, `${date}-${name}.md`);
      fs.writeFileSync(file, md);
      console.log(`\n${name}: PASS ${c("PASS")} FAIL ${c("FAIL")} SKIP ${c("SKIP")} -> ${file}`);
      return c("FAIL");
    },
  };
}
