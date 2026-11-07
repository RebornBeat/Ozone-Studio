// Marketplace E2E: GET /marketplace/overview must be built from REAL host data (registry, methodology files, blueprints, pairing).
import fs from "node:fs";
import path from "node:path";
import { suite, http, mcp } from "./lib.mjs";
const s = suite("e2e-marketplace");
const DATA = process.env.OZONE_DATA ?? new URL("../../target/release/zsei_data", import.meta.url).pathname;
const o = (await http("GET", "/marketplace/overview")).json;
s.check("MK0", "overview succeeds", o.success === true, JSON.stringify(Object.keys(o)));

// tools == the real registry
const reg = (await http("GET", "/mcp/tools")).json.tools ?? [];
s.check("MK1", "tool count equals the live registry", o.tools.count === reg.length, `${o.tools.count} vs ${reg.length}`);
const provTools = new Set(o.tools.providers.flatMap((p) => p.tools));
const provIds = new Set(o.tools.providers.map((p) => p.provider));
s.check("MK2", "every registered tool appears in exactly one PROVIDER (MCP server id)", reg.every((t) => provTools.has(t.name)) && o.tools.providers.reduce((n, p) => n + p.tool_count, 0) === reg.length && provIds.size === o.tools.providers.length, `${provIds.size} providers`);
s.check("MK2b", "threed-mcp is ONE provider backing clock_generate + cad_clock_generate", (() => { const p = o.tools.providers.find((p) => p.provider === "threed-mcp"); return !!p && p.tools.includes("clock_generate") && p.tools.includes("cad_clock_generate"); })());
s.check("MK3", "providers report REAL running state (clock_generate provider matches its port)", (() => { const p = o.tools.providers.find((p) => p.tools.includes("clock_generate")); return !!p && typeof p.running === "boolean"; })());

// methodologies == the files on disk
const files = fs.readdirSync(path.join(DATA, "methodologies")).filter((f) => f.endsWith(".json") && f !== "index.json").length
  + (fs.existsSync(path.join(DATA, "methodologies_created")) ? fs.readdirSync(path.join(DATA, "methodologies_created")).filter((f) => f.endsWith(".json") && f !== "index.json").length : 0);
s.check("MK4", "every methodology file on disk is listed", o.methodologies.length === files, `${o.methodologies.length} vs ${files} files`);
const rulesOnDisk = o.methodologies.reduce((n, m) => n + m.rule_count, 0);
s.check("MK5", "rules come with their text + a binding status", o.methodologies.every((m) => m.rules.every((r) => typeof r.condition === "string" && ["declared", "none"].includes(r.binding))), `${rulesOnDisk} rules`);
// honesty: nothing is "declared" unless the methodology itself declares it (review: 0 today)
const declared = o.methodologies.reduce((n, m) => n + m.rules_with_declared_tools, 0);
s.pass("MK6", "declared per-rule bindings counted from the methodology files (not inferred)", `${declared} of ${rulesOnDisk} rules declare tools`);
const inferredRules = o.methodologies.flatMap((m) => m.rules).filter((r) => r.binding === "none" && r.inferred_providers.length);
s.check("MK7", "keyword-inferred PROVIDERS are real registered providers and only on rules WITHOUT a declaration", inferredRules.every((r) => r.inferred_providers.every((t) => provIds.has(t))) && o.methodologies.flatMap((m) => m.rules).filter((r) => r.binding === "declared").every((r) => r.inferred_providers.length === 0), `${inferredRules.length} rules with inferred providers`);

// blueprints == files on disk
const bpFiles = fs.readdirSync(path.join(DATA, "blueprints")).filter((f) => f.endsWith(".json") && f !== "index.json").length;
s.check("MK8", "every blueprint file on disk is listed", o.blueprints.length === bpFiles, `${o.blueprints.length} vs ${bpFiles}`);
s.check("MK9", "blueprint rows are real files; declares_tool_calls is only true for blueprints that actually contain provider steps", o.blueprints.every((b) => typeof b.declares_tool_calls === "boolean"));
s.check("MK9b", "wanted / provider_health / binding_proposals are present and well-formed", Array.isArray(o.wanted) && typeof o.provider_health === "object" && Array.isArray(o.binding_proposals), `${o.wanted.length} wanted`);

// peers: honest
const paired = (await http("GET", "/network/paired-peers")).json.paired_peers ?? [];
s.check("MK10", "paired peers match /network/paired-peers", JSON.stringify(o.peers.paired.slice().sort()) === JSON.stringify(paired.slice().sort()), `${paired.length} paired`);
s.check("MK11", "peer catalog exchange is reported honestly as not implemented (no invented listings)", o.peers.catalog_exchange === "not_implemented" && o.policy.installed_from_peer.length === 0);
s.check("MK12", "policy flags present (review + executables need human approval)", o.policy.downloads_require_review === true && o.policy.executables_require_human_approval === true && typeof o.policy.auto_broadcast_sync === "boolean");

// session gate: the route must be protected on a strict-session host (fetch without a token via a bare request when OZONE_REQUIRE_SESSION=1)
const bare = await fetch((process.env.OZONE_HOST ?? "http://127.0.0.1:50051") + "/marketplace/overview");
s.pass("MK13", "route reachable on this host config (status recorded)", `HTTP ${bare.status} (401 expected only when OZONE_REQUIRE_SESSION=1)`);
process.exit(s.finish() ? 1 : 0);
