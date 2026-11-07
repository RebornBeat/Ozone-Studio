# MCP · tools · modalities · graph — architecture review and the binding decision (2026-11-04)

Operator question (verbatim intent): *Is having tools — and MCP calls — in the methodology the best place? MCPs come via modalities (per modality an MCP runs on graph MCP calls, which then graph the modality), but some MCPs are used apart from that. Who uses tools? Who uses MCPs? Are there cases where tools are used separately? I don't think so, because the context around a tool is always handled by the MCP — so focus on MCP first, decide whether the methodology is the right place, and how MCPs and tools work.*

Method: read-only. Every claim below cites `file:line` or a measured value from `target/release/zsei_data/mcp_tool_registry.json` (184 entries). No code was changed, built or run.

---

## 1. Findings (the short version)

1. **"Tool" and "MCP" are one thing at two granularities.** A *tool* is a registry entry — a named contract `{name, transport, endpoint, capabilities[], server_version, registered_at, auto_start}` (`src/mcp.rs:55-75`). An *MCP* is the server **process** the entry's `endpoint` points at (`docs/NEW_MCP_GUIDE.md:11-21`: "the tool is the contract, the MCP is the enforcement point"). Measured: **184 tools → 24 endpoints; 183 `http`, 1 `stdio`** (the shared-context entry, an agent-session MCP). One process backs many tool names (the `bridges` process :3210 backs 67).
2. **There is no tool without an MCP.** Registry entries carry no code; execution happens only in the server behind `endpoint` (`McpRegistry::invoke`, `src/mcp.rs:~515-600`), which adds the envelope (`tool, agent, input, context, identity_validated, session_user_id, agent_trust, trace`) and the `x-oz-internal` token. The only things people call "tools" that are *not* registry entries are (a) external agents' own native tools (Claude Code's Bash/Read — outside Ozone, un-metered: `src/mcp.rs` ledger comment "un-metered native calls"), and (b) pipelines — a different tier with a different dispatch (`PipelineExecutor`, `docs/TOOLS_PIPELINES_MCP_GUIDE.md` §1). Answer to Q6: **no — not inside Ozone.**
3. **The right unit differs by purpose.** *Call/contract unit* = tool name. *Install / run / trust / share unit* = **the provider = the MCP process** (the supervisor already models exactly this: port → `{dir, entry, port_env, device_requirement, credential_requirement, requires_p2p_enabled}`, `src/mcp_supervisor.rs:90-150`).
4. **`capabilities: Vec<String>` is not a taxonomy.** 181 distinct strings across 184 tools; **150 are free-text sentences** ("Convert a mesh between stl, obj, ply, glb and off"), 89 are `platform:linux/x64`, plus `version:`/`lang:`/`kind:` tags; only **9 tools** carry a `modality:<name>` tag (4 image, 4 security, 1 code). Stage 3 "matches" requests by substring against this soup.
5. **The operator's own doctrine already answers the placement question — "modality first, tools second."** `docs/MODALITY_TOOL_AFFINITY_GUIDE.md`: the modality runs itself on content first, then tools/MCPs run on top **as guests extending the same graph**; the affinity table `{modality, tool, autostart: none|mcps|scan}` is declarative and per-install; F-NEW-4/D18: tools should stamp `modality:<name>` at registration. `docs/FINAL_REVIEW_2026-10-31.md` D18/D14/G5/G10: *modality runs do not call tools themselves — the orchestrator/reasoning does*; routing stays off and modality-blind until the affinity + `modality:` tags exist. **That groundwork was never built** (`mcp_graph::modality_for` is still a tool-name-prefix table "until the D18 capability tag lands", `src/mcp_graph.rs:160-200`).
6. **Two paths through the host, only one of them feeds the graph.** `POST /mcp/call` (agents, UI, CLI) does gate → ledger → invoke → ripple mirror → **`mcp_graph::persist_from_output`** (a tool's `graph` block becomes ZSEI containers) → S13 `tool_calls.jsonl` → order-layer review (`src/grpc/mod.rs:5687-5990`). `call_global` (orchestrator tool routing, sensor tick, assistant proposals) does gate + ledger + invoke **only** (`src/mcp.rs:~152-190`). So a tool the **orchestrator** calls for a step returns text that is truncated to 2,000 chars and pasted into the step as untrusted data (`src/orchestrator/tool_routing.rs:213-280`) — **its `graph` block is never persisted, no S13 row, no ripple mirror** (the sensor tick mirrors by hand, `actors.rs:1183-1215`). That contradicts "tools extend the graph."
7. **Orchestrator-driven calls can't drive a real tool.** The call input is `{"query": step[..200]}` and the context carries `{source, internal, task_id, trace}` — **no `workspace_id`/`project_id`, no file refs, no schema** (`tool_routing.rs:196-224`). `McpTool` has no `input_schema` and no `timeout_ms`; the host timeout is a fixed 30 s (`src/mcp.rs:~574`) — shorter than a Blender/CAD build with margin.
8. **Methodologies/blueprints carry no tool reference anywhere** (0/44 runtime methodologies, 0/47 blueprints — `docs/METHODOLOGY_BLUEPRINT_TOOLS_REVIEW_2026-11-04.md` §1.4). The earlier "first 8 registry entries" floor in Stage 3 has been removed in the unbuilt edits.

---

## 2. Vocabulary (as the code uses it)

| Term | Meaning here | Evidence |
|---|---|---|
| **Tool** | A named contract in the registry; invocable by name through one standardized call | `McpTool` `src/mcp.rs:55-75`; `docs/NEW_MCP_GUIDE.md:11-21` |
| **MCP (server / provider)** | The loopback process that owns execution, security (allowlists, guard), platform/device/credential requirements; the thing that is started, stopped, installed | `src/mcp_supervisor.rs:90-150`; `tools/shared/guard.mjs:1-60` |
| **Capability** | Today: a free string on a tool. Intended: a modality-scoped verb ("3d.mesh.analyze") — not yet a real field | §1.4 above; D18/F-NEW-4 |
| **Pipeline** | Internal compiled unit shipped with Ozone, dispatched by `PipelineExecutor`; the *modality* runs here | `docs/TOOLS_PIPELINES_MCP_GUIDE.md` §1 |
| **Modality** | A content kind with an owning pipeline (100–127) that builds the base graph | `assets/pipelines/modalities/*`; `ModalityGraph` containers |
| **Bridge** | The long-lived transport half of a tool server holding a connection to an external system (`bridges` → Blender/Unity/Unreal/Roblox) | `docs/TOOLS_PIPELINES_MCP_GUIDE.md` §1 |
| **Graph MCP call** | A `/mcp/call` whose output has a `graph` block → containers + typed relations (`mcp_graph::persist*`) | `src/mcp_graph.rs:82-120`, `src/grpc/mod.rs:5937` |

---

## 3. Architecture map — MCP ↔ modality ↔ pipeline ↔ graph

(`provider` = MCP process; tool counts from the live registry.)

| Provider (port) | Tools | Modality it serves | Owning pipeline (what the modality does first) | How it reaches the graph |
|---|---|---|---|---|
| document-mcp :3282 | 4 (`document_extract/info/images/formats`) | Text | text (100) calls it through `/mcp/call` (`assets/pipelines/modalities/text/main.rs:8276-8294`) | text pipeline ingests the extracted text |
| web-mcp :3225 | 2 (`web_search`, `current_datetime`) | Text / web | web_search pipeline (56) — the MCP *wraps the pipeline binary* (`OZONE_WEB_PIPELINE`, `tools/web-mcp/server.mjs:11-18`) | via text modality on returned content |
| code-mcp :3230 | 1 (`code_callgraph`) | Code | code (101) — MCP wraps the pipeline (`OZONE_CODE_PIPELINE`, `tools/code-mcp/server.mjs:13-21`) | call graph = the modality's own graph |
| code-graph-mcp :3279 (not registered locally) | cross-file Python | Code | code (101) | cross-file edges |
| git-mcp :3235 | 3 (`file_status/diff/history`) | Code / Files | file beacon | beacon ripple |
| visual-mcp :3220 | 10 (`yolo_*`, `depth_graph`, `shape_detect`, `visual_graph`) | Image | image (102) decodes first (D17); MCP wraps image pipeline + OpenCV/ultralytics (`OZONE_IMAGE_PIPELINE`) | `graph` block "expands the native modality graph" → `mcp_graph` |
| browser-mcp :3280 / browser-farm-mcp :3281 | 11 / 13 | UiDocument (127) + Image (screenshots) | uidocument pipeline; `/capture/ingest_uidocument`, `/capture/ingest_image` | ingest route → `persist_from_output_scoped` (`grpc/mod.rs:~2996`) |
| bridges :3210 | 67 (`blender_*` 13, `unity_*` 15, `unreal_*` 15, `roblox_*` 20, 4 doctor/list) | 3D (Blender); gamedev engines | 3D (109) | tool results; Blender render → image chain (BL9) |
| threed-mcp :3250 | 3 (`eeg_cap_costume_generate` = rat 3D asset, `clock_generate`, `cad_clock_generate`) | 3D / CAD | 3D (109), CAD | generated files → ingest |
| mesh-mcp :3276 | 3 (`mesh_analyze`, `mesh_convert`, `cad_step_info`) | 3D / CAD | 3D (109) readers, CAD | facts about files |
| dna-mcp :3271 | 3 | DNA | dna (107) | — |
| chemistry-mcp :3272 | 2 | Chemistry | chemistry (106) | — |
| proteomics-mcp :3273 | 3 | Proteomics | proteomics | — |
| geospatial-mcp :3274 | 3 | Geospatial | geospatial | — |
| cerebrix-mcp :3245 | 3 (`eeg_window`, `karaone_*`) | EEG / BCI | eeg (108) | — |
| connectome-mcp :3255 | 3 | Biology / Network | biology, network | — |
| security-mcp :3240 | 10 | **none** (ops/posture; tagged `modality:security` ×4) | — | sensor tick → mirror → `SecurityFindingActor` |
| terminal-mcp :3215 | 2 | **none** (execution) | — | `/mcp/call` ripple only |
| android-mcp :3277 | 18 | device control | — | screenshots → image ingest |
| p2p-bridge-mcp :3278 | 2 | **none** (P2P ops; needs `enable_p2p`) | — | — |
| ozone-shared-context | 1 (stdio; 16 coordination tools) | **none** (multi-agent coordination) | — | `context_mirror` → CoordinationEvent |
| gcal-mcp :3241 / zoom-mcp :3265 | 7 / 7 | **none** (Order/calendar connectors; OAuth) | — | Order tasks |
| re-mcp :3260 | 3 (declaration-only) | **none** (reverse-engineering notes) | — | — |

**Reading the table.** (a) About half the providers are *tool faces over a modality* (document, web, code, visual, mesh, dna, chemistry, proteomics, geospatial, cerebrix, connectome, threed, bridges-blender) — some literally wrap the modality's own pipeline binary. (b) The rest are **infrastructure/ops** with no modality (security, terminal, p2p, shared-context, gcal/zoom, re, android device) — they feed the graph only through ripple/mirror. (c) **Audio, sound, video, math, thermal, depth, IMU, haptic, EM, radar, sonar, hyperspectral, kinematics, control: no provider at all** — the affinity guide's "biggest empty column". (d) **3D has three overlapping providers** (bridges Blender, threed-mcp, mesh-mcp) with no declared owner — a binding to tool *names* would hard-wire one of them.

Where "graph MCP calls → graph modalities" actually happens in code: `mcp_graph::persist_from_output` (`src/grpc/mod.rs:5937`, **HTTP `/mcp/call` only**), the ingest routes (`grpc/mod.rs:~2996` UiDocument; image), and `context_mirror::mirror` for the tool-call event itself (`grpc/mod.rs:5905-5925`). Modality assignment of the resulting containers is the tool-name **prefix** table `modality_for` (`src/mcp_graph.rs:160-205`) — interim.

---

## 4. Who calls what today

| Caller | Path | Gate / meter | Graph persist | S13 row | Notes |
|---|---|---|---|---|---|
| External agents (Claude Code, ZCode), UI, `ozone mcp call` | `POST /mcp/call` → `mcp_call` (`grpc/mod.rs:5687`) | session gate (strict hosts), jurisdiction, ledger, identity | **yes** (`graph` block) + ripple mirror | **yes** | the full door; UI: `ozoneClient.ts:235`; CLI: `ozone.mjs:169` |
| Orchestrator step routing | `tool_routing.rs` → `call_global` | jurisdiction (call gate + per-step gate), ledger | **no** | no (only `capture/call_spans.jsonl` verdict) | `{query}` only; ≤2 tools; routing default OFF (`OZONE_TOOL_ROUTING=1`) |
| Sensor tick (security posture, every 15 min) | `actors.rs:1183` → `call_global` | gate, ledger | no (hand-mirrors a CoordinationEvent) | no | read-only tools |
| Assistant proposals (whitelisted Action) | `consciousness/proposals.rs:629` → `call_global` | gate, ledger, whitelist | no | no | notification is the default kind (D21) |
| Text modality → document-mcp | host `/mcp/call` (`text/main.rs:8294`) | full door | via text ingest | yes | the clean modality→MCP pattern |
| MCPs wrapping pipelines (code, web, visual) | MCP spawns the pipeline binary directly | n/a (inside the provider) | provider-defined | — | provider-internal; not a bypass of the host door because the *caller* of the MCP is still `/mcp/call` |
| Anything on the box hitting a provider port directly | raw HTTP to :32xx | **only `tools/shared/guard.mjs`**, enforced iff `OZONE_MCP_REQUIRE_INTERNAL=1` or `OZONE_REQUIRE_SESSION=1` | no | no | EU enforces (401 verified); a default local dev host does not — the one real bypass surface |

No caller bypasses the jurisdiction gate *inside the host*; `call_global` was brought under it on 2026-11-02. The gaps are **(i)** graph persistence/S13/mirror missing on `call_global`, **(ii)** direct-port access on non-strict hosts.

---

## 5. Context flow around one MCP call

**Host → MCP** (`invoke_with_identity`, `src/mcp.rs:~535-570`): `{tool, agent, input, context, identity_validated, session_user_id, agent_declared, agent_trust, trace}` + header `x-oz-internal`. `context` is whatever the caller supplied: `/mcp/call` callers may pass `workspace_id`/`project_id` (used only for scoping the mirror/persist, `grpc/mod.rs:5668-5683`); the orchestrator passes `{source, internal, task_id, trace…}` and **not** ws/proj.

**What the MCP cannot see:** the graph. Providers are separate processes with no read access to ZSEI (no callback token, no `/context` read). Anything graph-derived must be put into `input` by the host. **What the host cannot do for the MCP today:** validate `input` — `McpTool` has no `input_schema`.

**MCP → host:** `{success, output}`; `output.graph` (validated, bounded: 5,000 nodes / 20,000 edges, relations whitelisted, spatial relations computed centrally, `src/mcp_graph.rs:36-60,785`) becomes containers **only on the `/mcp/call` path**.

**Consequence for "args templates at blueprint time":** only *static* arguments the request already fixes ("10:10", "radius 150 mm") can be known at blueprint time. Everything that depends on the graph (which file container, which workspace/project, a mesh path produced by an earlier step) exists only at **call time**. So the blueprint should record *which capability/tool a step calls and why* (`rule_ref`), and the **host should derive the arguments at call time** from the step text + step context + the tool's declared `input_schema`, validate them, and inject scope (`workspace_id`/`project_id`) and file refs — not store `{"query": …}` or a pre-filled template. (This needs two additive registry fields: `input_schema`, `timeout_ms`.)

---

## 6. The decision — where is "what is needed" bound?

| Criterion | (a) per rule → TOOL names | (b) per rule → MCP provider + capability | (c) per MODALITY (implicit) | (d) **hybrid**: rules declare modality-scoped CAPABILITIES; providers declare what they implement; blueprint resolves; marketplace shares PROVIDERS |
|---|---|---|---|---|
| Identity stability | poor — tool names are renamed/split (bridges 67 names; `eeg_cap_costume_generate` historical name) | medium — provider ids are stable (supervisor port table) but pin one implementation | good | **good** — capability ids are the stable contract; names/providers vary |
| Portability across users/peers | poor — a peer may have a different tool for the same job | poor — pins a provider the peer may not have | good | **good** — "3d.mesh.analyze" is satisfied by whatever provider the peer has |
| Version drift | high | medium | low | **low** — provider version recorded at resolution, not in the rule |
| Install/trust/share unit | mismatch (tool ≠ process) | match (process) | n/a | **match** — offers/reviews/installs are per provider; a methodology offer lists the *capabilities* it needs and the marketplace proposes providers |
| Fits "modality first, tools second" (D18, affinity guide) | no | partly | yes, but too coarse (a 3D rule may need `3d.generate`, not `3d.mesh.analyze`) | **yes** — modality namespace + per-capability verb; modality owns its provider set (the affinity table) |
| Fits "we have it or we don't → missing capability captured" | works but noisy (tool names) | works (provider missing) | too coarse to act on | **best** — gap = capability (+ modality), WANTED in the marketplace, satisfiable by any provider |
| Covers non-modality infra (security, terminal, calendar) | yes | yes | no — needs a pseudo-modality | **yes** — namespaces `security.*`, `exec.*`, `calendar.*`, `p2p.*` |
| Cost today | low (done in unbuilt edits) | medium | medium | medium — needs `modality`/`capability_ids`/`provider` on the registry + an affinity map |
| Existing assets reused | `mcp_graph` prefix table | supervisor port table | affinity guide | **all three** |

### Recommendation — (d), with these concrete rules

1. **Rules declare capabilities, not tools.** `requires: [{ capability: "3d.mesh.analyze", modality: "3D", purpose, optional }]`. Capability ids are `<modality-or-domain>.<verb>[.<object>]`, seeded from the existing tool families (e.g. `text.extract`, `web.search`, `code.callgraph`, `image.detect`, `image.depth`, `3d.generate`, `3d.mesh.analyze`, `3d.mesh.convert`, `3d.scene.edit` (Blender bridge), `cad.step.read`, `cad.solid.generate`, `dna.translate`, `chem.analyze`, `protein.search`, `geo.search`, `eeg.window`, `security.posture`, `exec.terminal`, `browser.navigate`, `calendar.sync`). A **pin** (`provider: "threed-mcp"`) stays available as an explicit escape hatch for the rare rule that truly needs one implementation.
2. **Providers declare what they implement.** Additive registry fields (serde-default): `provider` (the MCP process id — the supervisor's `dir`), `modality` (`modality:<name>` per D18, replacing the `modality_for` prefix table), `capability_ids` (the taxonomy above), `input_schema`, `timeout_ms`. The existing free-text `capabilities` stay for humans/search. Registrars (`tools/register-browser-tools.mjs` families, bridges shim) set them; a one-time migration derives them from today's names.
3. **Resolution is late and deterministic.** Stage 3 gathers the methodologies' capability requirements into the manifest; **blueprint creation resolves capability → tool** (prefer: modality match → healthy → running → operator affinity row → stable tiebreak), records `{capability, tool, provider, rule_ref}` per step; **simulation validates** availability (registered / running / requirements met / jurisdiction) with host-computed statuses; **a capability with no provider is a missing capability** (captured durably, ripple, methodology gap, WANTED in the Marketplace).
4. **Arguments are derived at call time** from step context + `input_schema` (host-validated, scope injected), never `{query}` and never a blueprint-time template except for static request-fixed values.
5. **The graph path is unified.** `call_global` gets the same post-processing as `/mcp/call` (one shared `post_call()` in the registry layer: ripple mirror, `mcp_graph` persist scoped to the run's ws/proj, S13 row, tool-health record) — "tools run on top as guests, extending the same graph" then holds for orchestrator-driven calls too.
6. **Marketplace shares providers.** An offer = a provider package (manifest: provider id, version, tool contracts, `capability_ids`, `modality`, requirements, signature). A methodology offer lists the capabilities its rules need; installing it proposes providers that satisfy them (dependency view). Review/trust/auto-review/approval apply to the provider (it is the executable).
7. **Search stays a fallback + learning path**, restricted to the step's modality when known (`state.modality_graphs`), and successful search-selected tools become *proposed capability bindings* (`tool_binding_proposals.json`), never auto-applied.

---

## 7. Impact on the already-edited-but-unbuilt code (read in full)

| File / item | Verdict | Change |
|---|---|---|
| `src/types/methodology.rs` — `ToolRequirement{kind, name, purpose, optional}`, `DecisionRule.requires` (`:62`, `:77`) | **KEEP + extend** | Make `capability` the primary `kind`; keep `provider` (pin) and `pipeline`; treat `mcp`/`tool` as legacy aliases = *pinned tool name*. Add optional `modality: Option<String>`. Serde-default keeps legacy methodologies loading. |
| `src/orchestrator/tool_binding.rs` (`Binding`, `gather_bindings`, `evaluate`, `manifest_block`, gaps, health, proposals) | **KEEP, rename semantics** | `find_provider` must resolve `capability` through `capability_ids` (+ modality) instead of substring-matching free text; `RegistryRow` grows `(name, capability_ids, endpoint, provider, modality)`; gap `kind` becomes `capability` (+ modality); **health keyed by provider and tool** (a dead provider degrades every capability it serves); proposals record `capability` not just tool. |
| Stage 3 rewrite `src/orchestrator/mod.rs:2704-2804` | **KEEP structure** (bindings first → search fallback → **no first-N floor**) | Search fallback prefers tools whose `modality` matches the run's `state.modality_graphs` keys; `applicable_tools` rows carry `provider`+`modality`+`capability_ids`; missing capabilities carry `modality`. |
| `stages.rs` `reconcile_tool_bindings` (`:18`) + blueprint prompt block (`:833-897`) | **KEEP, reword** | Prompt asks the model for **capabilities** (and optionally a registered tool name from REGISTERED CAPABILITIES); host resolves capability→tool and *drops* unknown names as missing; `StepToolCall` records the resolved tool, never trusting a model-invented name. Attach declared bindings authoritatively as today. |
| `StepToolCall` (`mod.rs:1008`) | **KEEP, extend** | Add `capability: Option<String>`, `provider: Option<String>`; `args: Null` means *derive at call time* (document it); `source` stays. |
| `BlueprintStep.rule_refs` / `tool_calls`, state `tool_manifest` / `missing_capabilities` | **KEEP** | no change |
| `McpTool` (`src/mcp.rs:55`) | **ADD (additive)** | `provider`, `modality`, `capability_ids`, `input_schema`, `timeout_ms` (all `#[serde(default)]`); `invoke` uses `timeout_ms` instead of the fixed 30 s. |
| `mcp_graph::modality_for` prefix table | **RETIRE** after tags exist | read `modality` from the registry (G5/D18). |
| `call_global` / `tool_routing.rs` | **CHANGE** | shared `post_call()`; build call input from `tool_calls[].args` or schema-derived args + ws/proj context; record tool health on every outcome. |
| `/marketplace/overview` + `MarketplacePanel.tsx` | **KEEP, add** | providers already group by endpoint ✓; add capability nodes to Relations (rule → capability → providers → tools), WANTED capabilities, provider health. |
| Ripple `CapabilityGap` event, `capability_gaps.json` | **KEEP** | unchanged. |

---

## 8. Open questions for the operator

1. **Taxonomy ownership.** Accept `<modality-or-domain>.<verb>[.<object>]` capability ids, defined alongside each modality (the affinity table) and migrated from today's tool names? Who approves new ids?
2. **Pinning.** Allow a rule to pin a provider (portable-breaking) or only capabilities?
3. **3D owner.** Bridges (Blender) vs threed-mcp vs mesh-mcp all offer 3D: which provider owns `3d.generate` / `3d.scene.edit` / `3d.mesh.analyze`, or is it resolved by health/affinity row?
4. **Unify `call_global` with `/mcp/call` post-processing** (graph persist + S13 + mirror for orchestrator-called tools)? This changes behavior for sensor ticks/proposals (they mirror by hand today).
5. **Registry fields** (`input_schema`, `timeout_ms`, `provider`, `modality`, `capability_ids`): additive and migratable — approve? `input_schema` also enables model-derived args at call time.
6. **Share unit.** Confirm the marketplace shares *providers* (process packages, executable → human approval), not individual tool names.
7. **Infra providers** (security, terminal, calendar, p2p, shared-context): give them real namespaces (`security.*`, `exec.*`, …) or exclude them from methodology binding entirely?
8. **D18 still holds?** "Modality runs do not call tools themselves — the orchestrator/reasoning does": keep, so tool use stays a blueprint/step concern and modality pipelines only *ingest* guest results.
