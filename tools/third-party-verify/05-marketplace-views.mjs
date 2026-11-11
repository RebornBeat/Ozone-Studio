// TPV-05 marketplace views (manifest-only, operator-gated). Read-only: never calls /marketplace/offers/decide.
import { suite, api, keysDeep } from "./tpv.mjs";
const s = suite("tpv-05-marketplace");
const o = (await api("GET", "/marketplace/overview")).json;
s.check("M1", "overview succeeds", o?.success === true, Object.keys(o ?? {}).join(","));
s.check("M2", "policy: downloads need review, executables need human approval", o?.policy?.downloads_require_review === true && o?.policy?.executables_require_human_approval === true, JSON.stringify(o?.policy ?? {}).slice(0, 160));
s.pass("M3", "publish/accept switches as the host reports them", JSON.stringify({ publish: o?.policy?.marketplace_publish, accept: o?.policy?.marketplace_accept_offers, legacy: o?.policy?.auto_broadcast_sync }));
const offers = o?.peers?.offers ?? o?.peer_offers ?? [];
s.pass("M4", "peer offers listed", `${Array.isArray(offers) ? offers.length : "n/a"}`);
const cls = new Set(["have", "conflict", "same_name", "new", "not_checked"]);
const bad = (Array.isArray(offers) ? offers : []).filter((x) => x.local_status !== undefined && !cls.has(x.local_status));
s.check("M5", "every offer carries a known local classification", bad.length === 0, `${bad.length} unknown`);
const leak = JSON.stringify(o?.peers ?? {}).match(/"(steps|blueprint_body|nodes)"/);
s.check("M6", "peer-side listing carries no blueprint body (blueprints are never shared)", !leak, leak ? leak[0] : "");
s.check("M7", "no confidence field in the marketplace payload", ![...keysDeep(o)].some((k) => k.includes("confidence")), "");
process.exit(s.finish() ? 1 : 0);
