# Jurisdiction deepening — report (2026-11-11)

Commit 337f43d (code, parse-only: rustfmt / node --check / ts.transpileModule; nothing compiled or run; EU untouched).
Engineering input, not legal advice. Sources: SOURCES.md. Checklist lines: CHECKLIST_LINES.md (L170–L177).
Report text was returned by the implementing fork (harness refuses fork-written reports) and written by the parent.

## 1. What existed
- Gate: Stage 0 intake and Stage 8b on the resolved plan, re-run after each walk expansion (`walk_expansion.rs:135`).
- Call gates: `/mcp/call` gate; call gate in `mcp.rs:171` (Block only; used by `/pipeline/execute` and in-process tool calls); per-step gate in `tool_routing.rs` only when `OZONE_TOOL_ROUTING=1`.
- Content: `global.json` + 131 national/local files, 648 rules, almost all `Log` (now surfaced as cited warnings).
- Missing: per-requirement agreements, outbound text check, decision log, erasure path, residency note. Policy was hard-coded `{enabled, instance_region}`.

## 2. Policy surface (per instance, not one fixed rule)
Edit in UI (Marketplace → Jurisdiction; compact form in setup wizard) or `POST /jurisdiction/policy`.
- Stored `<data_dir>/jurisdiction_policy.json`; every version appended to `jurisdiction_policy_history.jsonl` (never rotated).
- Defaults shipped as content `assets/jurisdiction/policy.default.json` (`include_str!`, `policy.rs` `shipped_defaults`). A corrupt saved file never opens the gate: defaults apply, logged.
- Sharing: `GET /jurisdiction/policy/export`; `POST /jurisdiction/policy/adopt` (explicit operator call, never automatic).

| Key | Default | Enforced at |
|---|---|---|
| enabled | true | policy.rs:760 |
| providers[] (endpoint substring -> region) | localhost/127.0.0.1 -> local; openrouter.ai -> third_country_unverified | policy.rs:551 |
| default_region_unlisted_endpoint | third_country_unverified | policy.rs:551 |
| allow_personal_data_regions | local, eea | policy.rs:760 |
| on_personal_data_outbound | await_operator (block, allow_logged) | policy.rs:760 |
| require_agreements_before_outbound + requirements[] | true; 3 shipped (transfers, erasure and logs, peer exposure) | policy.rs:760; acceptance :372 |
| detection.deterministic, kinds | on; email, phone, iban, card (Luhn) | policy.rs:651 |
| detection.vote, vote_model, vote_model_region | vote on, runs only with a named in-region model | policy.rs:994 |
| unknown_jurisdiction | global_only_flagged (await_operator, block) | jurisdiction.rs:472 |
| conflict_resolution | strictest | recorded; others behave as strictest |
| erasure.enabled, keep_before_copy | true; false | policy.rs:1094, :1146 |
| erasure.retention_days, covers | none (never rotated) | not enforced yet |
| p2p.recall_notice | true | UI text |
| p2p.publish_kinds, share_policy_as_offer | four kinds; true | NOT enforced (network/mod.rs ALLOWED_ITEM_TYPES :117 hard-codes) |
| hosting_region, residency_note | unspecified; empty | stored/shown |
| exempt_pipelines[] | empty | policy.rs:760 |

**Deploy consequence:** with defaults, first boot of this build holds every call to a third-country/unverified provider (OpenRouter on the EU) until the operator accepts the three requirements; background loops wait too. Nothing auto-accepts consent.

## 3. Outbound gate
`policy::check_outbound` runs at both seams every pipeline call crosses: `RegistryExecutorAdapter::execute` (`adapters.rs:68`, with vote executor) and `PipelineRegistry::execute` (`pipeline/mod.rs:249`, deterministic only; closes the `/pipeline/execute` bypass at `lib.rs:1695`). A cleared call carries a task-local mark (`policy::scoped_passed`), not an input key. Order: enabled → destination region → region allowed for personal data → agreements accepted → deterministic shapes (kinds recorded, never values) → optional strength-N yes/no vote → operator approvals (content hash / region) → policy action. Uncleared call returns `AWAITING_OPERATOR: ...` + durable awaiting item; never let through unjudged. No confidence scores.

## 4. Decision log
`jurisdiction_decisions.jsonl`, append-only, never rotated, no content: time, entry point, subject hash, region (+known), policy version/fingerprint, rule-set fingerprint, outcome, matched conditions/detector kinds, detail. Unwritable rows stay queued and retry. Read via `GET /jurisdiction/decisions?since=` or UI.

## 5. Gate coverage matrix
| Entry point | Rule gate | Outbound | Fail-open found → status |
|---|---|---|---|
| /orchestrate (HTTP/MCP/UI) | Stage 0, 8b, decision rows | yes | resumed walk lost full-gate result → fixed |
| Walk expansion | full gate re-run; block removes steps + pauses | yes | none |
| /pipeline/execute | call gate (Block only), logged | now yes | bypassed adapter → fixed |
| /mcp/call | own gate + call gate | yes | none |
| In-process tool calls | call gate | n/a | no gate => allowed (first ~5 s) → fixed (`mcp.rs:189`) |
| Per-step gate | Block only, only if OZONE_TOOL_ROUTING=1 | n/a | no gate => allowed → fixed (`:169`, `:303`) |
| MCP ozone_link / file/folder link | call gate + link guard | registry seam | none |
| graph_on_link | none by design | yes | judged when content leaves |
| Agent-provider steps | plan-level 8b | no | G4 |
| Tool creation draft/publish | via adapter; publish privacy scan + vote | yes | withheld stays pending operator |
| Refinement/meta/AMT/settlement | none (internal) | yes | none |
| Marketplace publish/accept | manifest-only, operator-gated | n/a | G5 |
| Search bridge | none | query text unjudged | G6 |
| Boot region population | label only | yes | none |
| tools/bridges, p2p-bridge-mcp | not reviewed | — | G7 |

## 6. Correctness findings
C1 fixed: call/step gates failed open when no gate installed (compiled Global set now judges). C2 fixed: unknown jurisdiction passed silently (now policy-driven, flagged, `region_known=false`). C3 fixed: outbound had no check. C4 fixed: resumed walk dropped full-gate result; call-gate runs unlogged. C5 finding: call/step gates use a rule snapshot built at boot, refreshed every 600 s (`lib.rs:1394`); Stage 0/8b reload per request. C6 finding: call/step gates check Block only. C7 finding: matcher whole-word English + aliases; new EU conditions "training data"/"legitimate interest" will warn on ordinary ML prompts (elevation is operator's decision). C8: changed rule applies next Stage 0; rows carry fingerprints. C9: conflicting jurisdictions: every applicable rule acts, Block wins; only `strictest` implemented.

## 7. Content, erasure, P2P, residency
- EU content: ten `Log` rules (DPIA Art.35, transfer Art.44 fetched; adequacy; legitimate interest; training data; GPAI; privacy by design Art.25; processor Art.28; records Art.30; minimisation Art.5(1)(c) cited by article only). `SOURCE_REGISTRY` regenerated. `docs/JURISDICTION_SETUP.md` not rewritten ("we never ask" superseded).
- Erasure: `POST /jurisdiction/erasure/plan` read-only (counts per file); `/apply` needs phrase `ERASE`, needle >= 4 chars, `erasure.enabled`; atomic rewrite replacing matches with `[erased]`; audit row holds needle hash only. Not covered: store-held container copies, derived indexes, binary files, peer copies, provider-side copies, backups.
- P2P: Marketplace tab + wizard state published items can't be recalled.
- Residency: note stored/shown; VPS hosting region NOT verified.

## 8. Hardening review
Content leaves via model calls (gated), /mcp/call (gated), marketplace manifests (tool privacy scan), agent steps (G4 open), search bridge (G6 open), bridges (G7 open), region-label-only searches, constant probes.

## 9. Remaining gaps / options
G1 no in-region vote model by default (name one in policy, or shapes-only + await_operator). G2 no confirm reviews on /pipeline/execute (route call gate through mcp_call review path). G3 step gate only with OZONE_TOOL_ROUTING=1 (default routing on). G4 agent-step content unjudged (check_outbound with agent region in agent_requests.rs). G5 p2p.publish_kinds unenforced (read policy in network/mod.rs; `jurisdiction_policy` item type). G6 search bridge queries unjudged (shape check in server.js). G7 bridges unreviewed. G8 retention_days/covers no consumer. G9 snapshot staleness (refresh on rule/policy change). G10 store-held copies not erased. G11 held calls return an Err callers may retry (verify no hot loop). G12 hardware detection may leave region unset.

## 10. Third-party suite (`tools/third-party-verify/jurisdiction/`, not run)
j1 read-only policy/log shape; j2 Block/Warn/Allow via /orchestrate with decision rows; j3 outbound hold probe (approves one hash only with OZONE_TPV_J_APPLY=1, never accepts requirements); j4 same via MCP + file link with personal-data shape.

## 11. Unverified — check first at cargo check
policy.rs: `tokio::task_local!`/`scoped_passed`; strength-N closure (`model.clone()` in FnMut; `scoped_passed(executor.execute(..))` on `&dyn PipelineExecutor`); `check_outbound` future `Send` inside adapter `async_trait`; `include_str!` path `../../../assets/jurisdiction/policy.default.json`. jurisdiction.rs: `pub(crate) mod policy;` in a non-mod.rs file; `state.task_id.map(..).as_deref()` temporary in `awaiting::open`; new `policy` field on `JurisdictionContentFile`. grpc/mod.rs: `get(h).post(h2)` chain; `matches!` mixing consts and string literal. pipeline/mod.rs: async call inside `execute`. Never exercised: held-call retry loops, agreements gate on live EU, region-unknown path, erasure on a real data dir.

Research: GDPR Art. 17, 35, 44 fetched; EDPB opinions/guidelines, DPF appeal, AI Act dates read only as search summaries — verify CJEU C-703/25 P and the Official Journal AI Act omnibus dates before relying.
