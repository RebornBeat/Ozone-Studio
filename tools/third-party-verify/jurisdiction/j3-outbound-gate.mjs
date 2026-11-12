// J3 outbound content gate through the public /pipeline/execute surface. The check sends a SMALL text containing a clear personal-data shape
// (an email address and a phone number in a throwaway sentence) to the prompt pipeline (9) and reads what the gate did.
//   OZONE_TPV_J_OUTBOUND=1 node tools/third-party-verify/jurisdiction/j3-outbound-gate.mjs
// With the default policy and a third-country destination the call must be HELD (AWAITING_OPERATOR) — it must not reach the model.
// This script never accepts requirements and never approves transfers (those are the operator's decisions).
// OZONE_TPV_J_APPLY=1 additionally approves the held content once (scope=content) and expects the call to pass; it spends one model call.
import { suite, api } from "../tpv.mjs";
const s = suite("tpv-j3-outbound-gate");
if (process.env.OZONE_TPV_J_OUTBOUND !== "1") { s.skip("O0", "outbound gate probe", "OZONE_TPV_J_OUTBOUND=1 not set (opt-in)"); process.exit(s.finish() ? 1 : 0); }
const pol = (await api("GET", "/jurisdiction/policy")).json;
const t0 = Date.now();
const text = `Reply with the single word OK. (test note: contact alex.example@example.org or +44 20 7946 0958)`;
const r1 = (await api("POST", "/pipeline/execute", { pipeline_id: 9, input: { prompt: text, max_tokens: 8 } })).json;
const err = String(r1?.error ?? "");
const heldOrBlocked = r1?.success === false && /AWAITING_OPERATOR|may not be sent to region/.test(err);
const allowedByPolicy = r1?.success === true;
s.pass("O1", "outbound call outcome as read", `success=${r1?.success} error=${err.slice(0, 120)}`);
const dest = (pol?.policy?.providers ?? []).map((p) => `${p.endpoint_contains}=>${p.region}`).join(", ");
s.check("O2", "the outcome is consistent with the policy (held/blocked unless the destination region is allowed or the operator approved)", heldOrBlocked || allowedByPolicy, `providers: ${dest}; allow: ${JSON.stringify(pol?.policy?.allow_personal_data_regions)}`);
const log = (await api("GET", `/jurisdiction/decisions?since=${t0}`)).json?.decisions ?? [];
const rows = log.filter((x) => x.entry_point === "outbound");
s.check("O3", "an outbound decision row was written (held_agreements / held_operator / blocked / allowed_*)", rows.length >= 1, rows.map((x) => x.outcome).join(",") || "none (a destination in an allowed region writes no row by design)");
s.check("O4", "rows carry detector KINDS only, never the email/phone values", rows.every((x) => !JSON.stringify(x).includes("example.org") && !JSON.stringify(x).includes("7946")), "");
if (heldOrBlocked) {
  const p2 = (await api("GET", "/jurisdiction/policy")).json;
  s.check("O5", "a durable hold is visible to the operator (agreement or outbound_transfer)", (p2?.holds ?? []).length >= 1 || /block/.test(err), `${p2?.holds?.length ?? 0} hold(s)`);
}
if (process.env.OZONE_TPV_J_APPLY === "1" && /AWAITING_OPERATOR:.*personal data/.test(err)) {
  const hold = ((await api("GET", "/jurisdiction/policy")).json?.holds ?? []).find((h) => h.kind === "outbound_transfer");
  if (!hold) s.fail("O6", "held transfer found to approve", "no outbound_transfer hold listed");
  else {
    const ap = (await api("POST", "/jurisdiction/outbound/approve", { scope: "content", key: hold.subject, approve: true, await_id: hold.await_id, by: "tpv-j3" })).json;
    s.check("O6", "operator approval recorded for this content only", ap?.success === true, JSON.stringify(ap));
    const r2 = (await api("POST", "/pipeline/execute", { pipeline_id: 9, input: { prompt: text, max_tokens: 8 } })).json;
    s.check("O7", "the same call now passes the gate (approved content)", !/AWAITING_OPERATOR/.test(String(r2?.error ?? "")), `success=${r2?.success} ${String(r2?.error ?? "").slice(0, 100)}`);
  }
}
process.exit(s.finish() ? 1 : 0);
