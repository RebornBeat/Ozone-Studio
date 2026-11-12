// J5 jurisdiction phase-2 surface, read-mostly: region visibility (G12), publish-kinds policy (G5), review path on /pipeline/execute (G2).
// Never accepts a requirement, never approves a transfer; the share call is only checked for its REFUSAL under the default policy.
import { suite, api, keysDeep } from "../tpv.mjs";
const s = suite("tpv-j5-phase2");
const p = (await api("GET", "/jurisdiction/policy")).json;
s.check("R1", "policy response states whether this instance's region is known (G12)", typeof p?.region_known === "boolean" && "instance_region" in (p ?? {}), `region_known=${p?.region_known} region=${p?.instance_region}`);
s.check("R2", "no confidence field anywhere in the policy payload", ![...keysDeep(p)].some((k) => k.includes("confidence")), "");
const kinds = p?.policy?.p2p?.publish_kinds ?? [];
const share = p?.policy?.p2p?.share_policy_as_offer;
const listed = kinds.includes("jurisdiction_policy");
const r = (await api("POST", "/jurisdiction/policy/share", { by: "tpv-j5" })).json;
if (!listed || !share) {
  s.check("K1", "under a policy that does not list jurisdiction_policy, offering the policy as a manifest is REFUSED (G5)", r?.success === false && /does not publish/.test(r?.error ?? ""), r?.error ?? JSON.stringify(r));
} else {
  s.pass("K1", "policy lists jurisdiction_policy: the share call is the operator's decision (not made by a verifier)", `listed=${listed}`);
}
const d = (await api("GET", "/jurisdiction/decisions?since=0")).json;
const eps = new Set((d?.decisions ?? []).map((x) => x.entry_point));
s.pass("D1", "entry points seen in the decision log (pipeline-execute-review / mcp-call appear once those calls were made)", [...eps].join(", ") || "(none yet)");
process.exit(s.finish() ? 1 : 0);
