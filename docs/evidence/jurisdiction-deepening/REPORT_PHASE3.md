# Jurisdiction deepening, phase 3 — full call-site review (2026-11-12)

Commit 04a2e7b (parse-only; nothing compiled or run; EU untouched). Engineering input, not legal advice.
Inventory: CALL_SITE_INVENTORY.md. Checklist lines: CHECKLIST_LINES_PHASE3.md (L204–L210). Written by the parent from the fork's output.

## Bypasses / fail-opens found and fixed
| # | Finding | Fix |
|---|---|---|
| 1 | Every pipeline was classified by the MODEL endpoint: web search (56) looked local with a local model, so the query reached Brave unjudged; pure-local pipelines were held when the model endpoint was remote. | `policy::pipeline_egress` gives each pipeline its own destination: web search by search provider, voice `api` by endpoint (forced `voice_recording` kind), browser navigation by URL host. Pipelines 1–127 have no egress. Unknown ids are `unaudited-pipeline:<id>`, never judged local. |
| 2 | Modality / zero-shot pipelines spawn the prompt binary directly; their model calls never crossed the registry check (R4 hole). | `prompt/main.rs::jurisdiction_clearance` asks the host (`POST /jurisdiction/outbound/check`) before every provider request; unreachable host = not sent. `OZONE_JURISDICTION_CHECK=off` only for standalone use. |
| 3 | HTTP/SSE tool call in `mcp.rs` had no outbound check; localhost sidecars (web_search, geo_*, gcal_*, zoom_*, farm_*) reach external services. | `check_outbound_to_endpoint` (ignores the adapter's cleared mark); new policy key `tool_egress` with shipped rows for those sidecars. |
| 4 | Self-registered remote pipelines (`pipeline/remote.rs`) never checked against `execute_url`. | Same endpoint gate before the POST; held call = error, never a spawn fallback. |
| 5 | `/jurisdiction/` policy/agreement/approval/adopt/erasure routes were not session-protected on a strict host. | Protected; only `/jurisdiction/outbound/check` exempt (the spawned prompt binary has no session). |
| 6 | Bodies served to P2P peers were not scanned for personal data. | `policy::check_p2p_body` withholds a body with personal-data shapes until the operator approves its content hash. |
| 7 | Region lookup sends the host IP to ip-api.com over plain HTTP. | Switch `OZONE_REGION_IP_LOOKUP`, default ON. |

Verified already routed: in-process `executor.execute` callers reach adapter/registry; other JS tools only call the host; constant probes carry no user content.
Expansions: policy key `tool_egress`; entry kinds for endpoint, P2P-serve, clearance hop; forced detector kind for voice. No new content detectors.

## tools/bridges (submodule, not edited)
Direct provider calls from the control-plane/web console: `providers.ts:303,349,402,454` (OpenAI, Anthropic, z.ai, MiniMax); `llm-proxy.ts:151,212,261` (Anthropic, OpenAI, Google). Engine bridges (pipelines 200–203) are loopback.
- A (recommended): patch so each fetch first POSTs `/jurisdiction/outbound/check` (one helper, 8 sites; unreachable host = not sent).
- B: route its model calls through host `/pipeline/execute`.
- C: do not run the console on instances whose agreements forbid third-country calls.

## Consequences
Strict host: UI/e2e tools need a session for `/jurisdiction/*`. The prompt binary needs the host at `OZONE_HOST`. The vote model endpoint must be listed in `providers` with an allowed region or the vote call is itself held (documented, not auto-fixed).

## Check first at cargo check
`Egress` match and `forced_kind` flow in `check_outbound_inner`; early `return McpResult` in `mcp.rs`; `withheld` captured in the `filter` closure in `network/mod.rs`; `PROTECTED_EXEMPT`; the prompt crate (built via the pipelines script). Not run: j6-phase3.mjs (syntax-checked).
