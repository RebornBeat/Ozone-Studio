// J1 jurisdiction POLICY surface, read-only. The policy is the instance's own (versioned, durable); every gate reads it.
// Checks shape, doctrine (no confidence fields), agreements/requirements, and that the decision log carries no content.
import { suite, api, keysDeep } from "../tpv.mjs";
const s = suite("tpv-j1-policy-surface");
const p = (await api("GET", "/jurisdiction/policy")).json;
s.check("P1", "GET /jurisdiction/policy success with policy + fingerprint + agreements", p?.success === true && !!p.policy && typeof p.fingerprint === "string" && Array.isArray(p.agreements), Object.keys(p ?? {}).join(","));
const pol = p?.policy ?? {};
s.check("P2", "policy names providers and allowed regions (an empty policy must never exist)", (pol.providers?.length ?? 0) > 0 && (pol.allow_personal_data_regions?.length ?? 0) > 0, `providers=${pol.providers?.length} allow=${JSON.stringify(pol.allow_personal_data_regions)}`);
s.check("P3", "policy action keys hold known values", ["await_operator", "block", "allow_logged"].includes(pol.on_personal_data_outbound) && ["global_only_flagged", "await_operator", "block"].includes(pol.unknown_jurisdiction), `${pol.on_personal_data_outbound} / ${pol.unknown_jurisdiction}`);
s.check("P4", "no confidence field anywhere in the policy payload (doctrine: yes/no only)", ![...keysDeep(p)].some((k) => k.includes("confidence")), "");
const stale = (p?.agreements ?? []).filter((a) => a.stale);
s.pass("P5", "requirements: accepted-current vs awaiting (read, never auto-accepted by a verifier)", `${(p?.agreements ?? []).filter((a) => a.accepted_current).length} accepted, ${(p?.unaccepted ?? []).length} awaiting, ${stale.length} changed since accepted`);
const d = (await api("GET", "/jurisdiction/decisions?since=0")).json;
s.check("D1", "GET /jurisdiction/decisions success with count == rows", d?.success === true && d.count === d.decisions?.length, `count=${d?.count}`);
const rows = d?.decisions ?? [];
const forbidden = ["prompt", "text", "content", "body", "input"];
s.check("D2", "decision rows carry no content fields (hashes and rule vocabulary only)", rows.every((r) => !forbidden.some((k) => k in r)), `${rows.length} rows checked`);
s.check("D3", "decision rows are time-ordered as appended (append-only log)", rows.every((r, i) => i === 0 || r.at >= rows[i - 1].at), "");
s.check("D4", "every row names the policy version and an outcome", rows.every((r) => typeof r.policy_version === "number" && typeof r.outcome === "string"), "");
const ex = (await api("GET", "/jurisdiction/policy/export")).json;
s.check("E1", "policy export is a jurisdiction content file (disclaimer + >=1 rule + policy block) the staging validator accepts", ex?.success === true && !!ex.content?.disclaimer && (ex.content?.rules?.length ?? 0) >= 1 && !!ex.content?.policy, "");
process.exit(s.finish() ? 1 : 0);
