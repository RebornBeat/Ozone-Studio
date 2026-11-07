# Methodology → Rules → Blueprint → Steps → Tools/MCPs, ripple actors, AMT expansion, and the Marketplace (review, 2026-11-04)

Operator direction: tools/MCPs are aggregated at **blueprint creation, around simulation**; the orchestrator's steps exist because
**methodologies are gathered first** (the AMT is built around them), **rules become steps**, and the tools/MCPs should be captured **in the
methodologies, per rule**. So: do we really need to *search* for tools at run time? Also review all ripple actors + AMT expansion, capture the
role of the **blueprint** (it makes steps from rules), and design the Marketplace UI (global P2P tools/MCPs, what this user installed, the
per-methodology relationship, contribution → review → download).

Everything below is read from code/data in this repo (file:line verified 2026-11-04); nothing was built or run for this document.

## 0. Verdict in six lines

1. **Today there is no tool binding anywhere in the methodology → blueprint → step chain.** `Methodology`, `DecisionRule`, `Blueprint`, `BlueprintStep`
   and the blueprint JSON schema the model is asked to return contain **no tool/MCP field** (§1.3, §2).
2. Tools enter by **keyword substring match against the request** (Stage 3, `mod.rs:2657-2705`) and again by **word overlap against each step's text** at
   execution (`tool_routing.rs:82-120`), then are called with a generic `{"query": <200 chars of the step>}` (`tool_routing.rs:215-221`). Run-time search *is* the
   only mechanism, which is exactly what the operator says it should not be.
3. Real data confirms it: **0 of 44 runtime methodologies (+15 created, +58 asset files) carry a structured tool/MCP reference**; **0 of 47 stored blueprints /
   187 steps** carry one; 55 % of steps are pipeline 9 (§1.4).
4. **Run-time search is not redundant, but it should be demoted** to discovery/fallback; the primary path should be *rule → required capabilities →
   blueprint tool manifest → simulation validates availability → step calls a bound tool with an argument template* (§3).
5. The **blueprint is the pivot**: it is the only place that turns rules (text) into steps (executable), so it is the only place that can aggregate the
   per-rule tool needs, dedupe them, record which rule needs which tool, and hand simulation something to *validate* (§3.3).
6. **Marketplace: the plumbing for sharing methodologies/blueprints exists (default-off), tools/MCPs are not shared at all, and there is no marketplace UI** (§5). The design
   in §6 adds a manifest-only catalog protocol between *paired* peers, a quarantine→review→approve install flow, and a relationship graph per methodology.

---

## 1. The real flow today (verified)

### 1.1 Prompt → methodologies (Stage 3) → AMT (Stage 5) → blueprint (Stage 6) → simulation (Stage 7) → steps

| Step | What happens | Where |
|---|---|---|
| Stage 3 gather | `search_by_keywords(first 20 request keywords, type="Methodology")` | `src/orchestrator/mod.rs:2476-2486` |
| Floor (never zero) | empty → use children of the methodology root (container 2); still empty → **create the base orchestration methodology at run time** (pipeline 12 `methodology_create`) — its own principle text says *"Simulation owns calls — pipeline/MCP/tool calls belong to simulation"* | `mod.rs:2489-2540` (principle at `:2528`) |
| Categories | methodology categories + topic categories created if missing | `mod.rs:2547-2655` |
| **Tools/MCP aggregation** | same stage: for each registry entry, `hay = name + capabilities`; **substring-match any request keyword/topic** → `applicable_tools` (matched=true); if none, **first 8 registry entries** (matched=false); consciousness entries excluded | `mod.rs:2657-2705` (match `:2673`, floor `:2691`) |
| Methodology gap | a request that matches no methodology appends to `methodology_gaps.json` (dedup, cap 500) — the meta-loop's input | `src/orchestrator/amt.rs` `record_methodology_gap` (≈`:2625-2680`) |
| Meta-loop drafting | reviews gaps, drafts a new methodology via pipeline 12 | `src/orchestrator/meta_loop.rs:88` ff. |
| Stage 5 AMT build | per-methodology branch-discovery lanes (`rank_methodologies` orders them; a lane that fails is logged "UNCOVERED") ; AMT nodes carry `methodology_ids` | `amt.rs:2698` (`rank_methodologies`), `:2760-2775`; `mod.rs:659` (`AMTNode.methodology_ids`) |
| Stage 6 blueprint | reuse a stored blueprint if keyword Jaccard ≥ 0.95 **and it has loadable steps**, else **design a new one with the model** | `src/orchestrator/stages.rs:515-640` |
| Blueprint prompt inputs | request text, AMT branches, **REGISTERED CAPABILITIES** (`capability_summary_for_need`: lines of the full registry filtered by *request-word* overlap), AVAILABLE PIPELINES, models, **APPLICABLE METHODOLOGIES (their full rule text)**, jurisdiction, file relationships | `stages.rs:663-760`; capability filter `stages.rs:153-193` |
| Blueprint output schema | `{name, description, steps:[{step_index, action, description, pipeline_id, context_requirements, depends_on, wait_for_graph_update, max_retries, model_override}], missing_capabilities:[…]}` — **no tool field**; `missing_capabilities` is only `tracing::warn!`ed ("could trigger pipeline creation in future") | `stages.rs:766-796`, `:849-865` |
| Rule→step linkage | after the model drafts steps, a reconciliation pass matches each step to its AMT branch **by content overlap** and fills `source_chunk_indices` and `methodology_ids` | `mod.rs:975-992` (fields), reconciliation in `stages.rs` stage-6 body |
| Stage 7 simulation | model is shown the blueprint steps + the same capability list and told to "flag any step whose goal matches a registered capability" — but the **output schema has nowhere to put a flag** (`step_predictions:[{step,needs,produces,risks}]`, `overall_feasibility`, `clarifications_needed`) | `stages.rs:1296`, `:1454-1520` |
| Step execution | with `OZONE_TOOL_ROUTING=1` only: `select_tools` picks **≤ 2** tools from `applicable_tools` by word overlap with the step description; calls `call_global` with **`{"query": step[..200]}`**; result wrapped as untrusted data; step gate + metering + capture | `tool_routing.rs:62` (`MAX_TOOLS_PER_STEP`), `:82-120`, `:205-221`; wired at `stages.rs:2719-2735` |

### 1.2 What this means for "rules become steps"
Rules are **text inside the methodology** (`DecisionRule{rule_id, condition, action, priority, exceptions}`, `src/types/methodology.rs:57-63`; on disk
`decision_rules:[{condition, outcome, name}]`). The blueprint model reads that text (`load_methodology_rules_full`, `amt.rs:2564`) and is *asked* to make
sure a step addresses each rule. The mapping rule→step is therefore **implicit and model-mediated**; only `BlueprintStep.methodology_ids` (set later by
overlap) remembers the methodology, and nothing remembers *which rule* a step came from or *what that rule needs to execute*.

### 1.3 Does any layer carry tool/MCP references?
| Layer | Field? | Evidence |
|---|---|---|
| `Methodology` (principles, heuristics, decision_rules) | **none** | `src/types/methodology.rs:8-37`, `DecisionRule` `:57-63` |
| `methodology_create` input (`DecisionRuleInput{name,condition,outcome}`) | **none** | `assets/pipelines/general/methodology_create/main.rs:66` |
| `Blueprint` / `BlueprintStep` (typed + JSON schema in the prompt) | **none** (only `pipeline_id`, `methodology_ids`, `context_requirements`) | `src/types/blueprint.rs:10-40`; `stages.rs:766-796` |
| Registry entry (`McpTool`) | `name, transport, endpoint, capabilities[], server_version, registered_at, auto_start` — capabilities are free strings; **no publisher, signature, version constraint, input schema** | `src/mcp.rs:54-75` |

### 1.4 Measured data (local host, `target/release/zsei_data`, `assets/`)
| Population | Count | Structured tool/MCP refs | Notes |
|---|---|---|---|
| Runtime methodologies (`methodologies/`) | 44 | **0** | only prose: `method_16`, `method_20` mention MCP in examples |
| Created methodologies (`methodologies_created/`) | 15 | **0** | 2 contain the base-methodology principle text "applicable tools/MCPs" (prose) |
| Asset methodologies (`assets/methodologies/`) | 58 files | **0** | `method_45…50` and `security_monitoring_scoped_caps` discuss MCPs in prose; per-rule tool fields: 0 |
| Stored blueprints | 47 blueprints in 32 files, **187 steps** | **0** | pipeline_id 9 = 103 steps (55 %), 101 = 29, 1 = 13, 100 = 10, 105 = 10, 12 = 6; 2 blueprints list `missing_capabilities`; 5 mention a tool only in free-text `context_requirements` ("security-mcp","tool","firewall_status"; the clock walk's "Blender tool") |
| MCP registry | **184 tools on 24 endpoints** (one `bridges` process backs 67); 9 auto-start | — | `mcp_tool_registry.json` |

Live corroboration (E2E, 2026-11-04): the clock request's walk #1 finished "successfully" without ever calling `clock_generate`/`cad_clock_generate`
(routing off; model wrote `<tool_call>` text). Even with routing on, the call would have been `{"query": "<step text>"}` — a tool needing structured
arguments (`time`, `radius_mm`) has no way to receive them.

---

## 2. Is run-time search redundant? — the honest analysis

Run-time search today does three jobs: (a) *which tools exist for this request* (Stage 3 keyword filter), (b) *which tool for this step* (word overlap), (c) *what
arguments* (none: `{query}`). Methodology-carried bindings can replace (a) and (b) **when the methodology is known and annotated**, and can supply (c) via an
argument template. They cannot replace discovery when: the request matches no methodology (a gap), a methodology predates the binding schema (all 117 today), or
a better tool newly appeared (installed or on a peer).

**Conclusion: bindings become the primary path; search becomes the fallback and the *learning source* for bindings** (§3.5). Search is not deleted.

## 3. Target architecture

### 3.1 Per-rule tool binding (methodology schema, backward compatible)
Add an optional list to each rule (`#[serde(default)]` in `DecisionRule` / `DecisionRuleInput`; JSON files keep working):
```json
"decision_rules": [{
  "rule_id": 3, "name": "flag missing tests", "condition": "...", "outcome": "...",
  "requires": [{
    "capability": "code.callgraph",              // stable name, matched to registry `capabilities[]`
    "tool": "code_callgraph",                    // preferred concrete tool (optional)
    "alternatives": ["code_analyze"],            // acceptable substitutes
    "args": {"language": "{{language}}", "code": "{{file_content}}"},   // argument template (values resolved from step/AMT context)
    "role": "evidence | action | verify",        // read-only evidence vs acting vs post-check
    "optional": false,
    "needs": {"device": null, "credential": null, "platform": "linux/x64"},
    "min_version": "0.1.0",
    "provenance": {"added_by": "meta-loop|operator|peer:<id>", "learned_from_task": 994}
  }]
}]
```
Methodology-level `requires[]` (applies to all rules) is allowed for shared tools. Pipelines are referenced the same way (`"pipeline": 101`) — a rule may need a
**pipeline** (e.g. Code modality) rather than an MCP; the schema must not distinguish kinds beyond `capability|tool|pipeline`.

### 3.2 Stage 3 (gather) — gather the bindings *with* the methodologies
`gather_methodologies` already loads each methodology; additionally collect `requires[]` of every selected methodology → a **candidate tool manifest**
(union, dedupe by capability, remember `(methodology_id, rule_id)` per entry). The keyword filter (`mod.rs:2657-2705`) stays but only as the
*discovery supplement*, flagged `source:"search"` vs `source:"methodology"`.

### 3.3 Stage 6 (blueprint) — rules → steps → tools, explicitly
- Prompt section **`RULE → REQUIRED CAPABILITIES`** replaces "REGISTERED CAPABILITIES (a step MAY route…)": for each selected methodology rule, its `requires` (with
  local availability status). The model must cite `rule_id` per step it drafts from a rule.
- Output schema gains `rule_refs:[{methodology_id, rule_id}]` and `tool_calls:[{capability|tool, args, role, rule_ref}]` per step, and a blueprint-level
  `tool_manifest` (deduped, with the rules that require each, and the availability the *host* computed — never trusted from the model).
- Host-side reconciliation (already exists for chunk/methodology overlap) is extended: if the model omitted a required capability for a step that cites a
  rule, the host adds it from the methodology binding (**the binding, not the model, is authoritative**).
- `missing_capabilities` becomes structured `{capability, rule_ref, status}` and is persisted on the blueprint (today it is only logged, `stages.rs:859`).
- A reused blueprint (Jaccard ≥ 0.95, `stages.rs:549-598`) carries its manifest, so a cached plan re-validates availability each run instead of silently going stale.

### 3.4 Stage 7 (simulation) — validate, don't guess
Give simulation the manifest and a real schema slot: `tool_checks:[{capability, rule_ref, status}]` with host-computed statuses
`ready | startable | needs_device | needs_credential | blocked_by_jurisdiction | missing_local_available_on_peer | missing`. Simulation then reports feasibility
against facts (installed? registered? `running` per `mcp_list_tools`, `grpc/mod.rs:5317-5327`; device/credential requirements; jurisdiction gate) instead of the model
"flagging" nothing. A `missing_local_available_on_peer` result drives the **download-on-review flow** (§6.4) as an *awaiting item* (meta-only gate semantics from
the doctrines), not a silent failure.

### 3.5 Step execution + learning loop
- Step executes its `tool_calls` with the template-resolved args (still: step gate per call, untrusted wrap, metering, capture). `select_tools` word-overlap
  remains only for steps with **no** bound call (legacy methodologies), and its result is recorded as `source:"search"`.
- **Learn once, bind forever:** when a search-selected tool succeeds for a rule, emit a *Finding* ripple (`ripple_class::emit_finding`) → the meta-loop proposes a
  `requires` entry for that rule (human/meta review, versioned methodology) — so today's 117 un-annotated methodologies acquire bindings from real successful use.
- A routed tool that returned a non-expected-refusal failure marks the binding **degraded** (§4.2) so the next blueprint/simulation sees it.

### 3.6 When keyword search is still right
(1) no methodology matched (gap) — record the gap *with the tools that ended up being used* so the meta-loop drafts methodology+bindings together; (2) legacy
un-annotated methodologies; (3) interactive discovery in the UI (marketplace search); (4) a newly registered tool after the blueprint was cached.

---

## 4. Ripple actors and AMT expansion (review)

### 4.1 What exists
| Actor / route | Trigger | Output | Where |
|---|---|---|---|
| `security-finding` | CoordinationEvent mirroring a security-mcp tool failure | candidate route `SecurityFinding` | `actors.rs:78`, route `:112` |
| `coordination-finding` | CoordinationEvent with a finding keyword | `CoordinationReceived` | `actors.rs:128-146` |
| `task-failed` | Task failure | `TaskFailure` (ignores the AMT loop's own internal tasks) | `actors.rs:162-199` |
| `code-staleness` | file-beacon `updated` on code | `CodeStaleness` ("code file changed — call graph may be stale") | `actors.rs:220-246` |
| `mcp-failure` | mirrored `tool_call` with `success:false`, non-refusal, cooldown per tool+class | `CoordinationReceived` "MCP tool 'x' failed (class: …)" | `actors.rs:341-382` |
| AMT routes | `InitialBuild` (the build itself, not a candidate), `Continuation` (a later prompt expanded the AMT), `GraphRipple` (scoped graph write in the project), + the four finding routes | `amt_candidates.json`; `amt_loop` spends one call per candidate; **echo filter** (ChatSession/ChatTurn/AMTExpansion/Task/CoordinationEvent never re-deepen) | `amt_candidates.rs:13-36`, `amt_loop.rs:203-215` |

All five actors are free (no model call) and feed **only the candidate log**; they never touch a methodology, a blueprint, or the tool registry.

### 4.2 Gaps relative to the operator's model
1. **No link from a failing MCP to the methodologies/blueprints that depend on it.** `mcp-failure` produces a note for the AMT loop; a blueprint whose steps
   bind that tool stays "ready". With bindings (§3) the actor should also write `tool_health` (tool, class, count, last_ok/last_fail) — consulted by
   simulation (`degraded`) — and emit the Finding ripple the meta ripple (`ripple_class.rs`) already wakes the loops on.
2. **No actor for registry change** (a tool registered/unregistered/started/stopped → any blueprint manifest entry that was `missing` may now be `ready`, or the reverse).
   Add `tool-registry-changed` (source `mcp-registry`) and re-validate cached blueprints' manifests (cheap, no model).
3. **No actor for peer catalog change** (§6): new offer that satisfies a `missing_local_available_on_peer` manifest entry → awaiting item.
4. **AMT expansion and methodology gathering are decoupled from tools**: AMT branches carry `methodology_ids` (`mod.rs:659`) but not the rules that created them,
   so a branch cannot tell the blueprint "I exist because rule 3 fired". Add `rule_refs` to AMT nodes at branch-discovery time (the lanes already attribute branches
   to methodologies — `amt.rs` prompt "attribute each suggested branch to the methodology that requires it"); extend attribution to the rule.
5. `methodology_create` `Propose` is an explicit stub (`methodology_create/main.rs:397-402`: "Methodology proposal/consensus is not implemented yet") — the contribution path (§6.5) must be built, not assumed.

### 4.3 Meta ripple tie-in
The meta ripple design (`docs/RIPPLE_CATEGORIES_AND_META_RIPPLE_2026-11-04.md`) already classifies `Finding`, `MethodologyGap`, `Task`. The new events here
(`tool_health_changed`, `tool_registry_changed`, `peer_offer_available`, `binding_proposed`) should be added as meta-significant categories so the meta-loop
reviews bindings event-driven, not on a timer.

---

## 5. Marketplace / P2P — what is real today vs stub

| Capability | Status | Evidence |
|---|---|---|
| Local MCP registry + control (register/unregister/autostart/start/stop) | **Real** | routes `grpc/mod.rs:4556-4563`; 184 tools; `mcp_list_tools` adds `device_requirement`, `credential_requirement`, `running` (`:5317-5327`) |
| UI for the **local** registry | **Real** | `ui/src/components/ToolsPanel.tsx` (groups by process; autostart/start/stop; `ozoneClient.ts:191-223`) |
| Local library browse/search of methodologies, blueprints, pipelines, categories, favorites | **Real, local only** | pipeline #7 `assets/pipelines/general/library_tab/main.rs` (`LibraryInput`: GetCategories, SearchMethodologies, GetBlueprint, ListPipelines, favorites…) — **no UI panel calls it** (no "Library/Marketplace" component in `ui/src`) |
| libp2p identity, persistent peer id | **Real** | `network/mod.rs`, `<data_dir>/p2p_identity.key` |
| Pairing: connect → approve → revoke → list (default-deny, bidirectional) | **Real** | routes `grpc/mod.rs:4513-4516`; `network/mod.rs:828-916` |
| UI for pairing | **Real (device pairing)** + network graph | `PairingPanel.tsx`, `views/network/LivingNetworkView.tsx` |
| Sync of Methodology / Blueprint / Container / PipelineResult | **Hook plumbing real, broadcast OFF by default** | `SyncItemType` has exactly these four (`network/mod.rs:52-57`); `on_methodology_created`/`on_blueprint_created` (`:694-725`); gated by `network.auto_broadcast_sync` default **false** (`config/mod.rs:550-562`, `lib.rs:1467-1494`); sender `amt_loop.rs:1898-1936` |
| Receiving peer content | **Real but passive**: envelopes from *paired* peers are appended to `peer_sync_candidates.jsonl` and surfaced as **findings** (no container replicated, no install) | `network/mod.rs:1028-1140` (`spawn_peer_sync_review`) |
| Sharing **tools/MCPs** | **Absent** — no `SyncItemType` for tools; `PeerInfo{reputation, contribution_count, capabilities}` (`network/mod.rs:115-124`) exists but no catalog is exchanged | — |
| Provenance/trust fields on methodologies | **Schema only**: `created_by: PublicKey`, `distributed`, `consensus_status` (`types/methodology.rs:30-36`; `ConsensusStatus{Open,Verifying,Accepted…}` in `types/pipeline.rs:187`) — never populated by a flow | — |
| Contribute/propose a methodology | **Stub** — returns Err "not implemented" | `methodology_create/main.rs:397-402` |
| Signatures, review states, install-from-peer | **Absent** | — |
| `p2p-bridge-mcp` | SSH-tunnel bootstrap for two hosts, **not** a marketplace | `tools/p2p-bridge-mcp/server.mjs:1-40` |
| Docs | `docs/P2P_AND_SHARED_MCP_COMMUNICATION_GUIDE.md` §7 still says approve-peer/peer review are "not built" — **stale** vs code (both exist); no marketplace doc exists (grep finds the word only in articles/zoom-mcp/CHECKLIST) | — |

**There is no marketplace UI.** `ToolsPanel` is the closest thing and is strictly local.

---

## 6. Marketplace design

### 6.1 Principles
- **Manifest first, code never implicit.** The catalog moves *metadata*; artifacts move only on explicit fetch into quarantine. A tool from a peer is remote code — nothing runs until an operator approves.
- **Only paired peers** (the existing default-deny pairing) are catalog sources. "Global from all users" = the union of catalogs of every paired peer plus *their* published offers; open-network discovery is a later, opt-in step (decision §8).
- **Relations are the point**: a tool/MCP is shown *through the methodology that needs it* (methodology → rules → tools → blueprints), because that is how the system uses them (§3).
- Real data only; an unreachable peer shows "last seen", never fabricated listings (AGENTS.md rule).

### 6.2 Data model (an *Offer*)
`{offer_id (content hash), kind: tool|mcp|methodology|blueprint|pipeline, name, version, publisher{peer_id, pubkey}, signature, capabilities[], requires{device,credential,platform}, methodology_refs[], rule_refs[], jurisdiction_tags[], size, created_at, review{state, reviewers[], findings[]}, source: local|peer:<id>}`.
Local installed items are the same shape with `source:local, install{installed_at, from_offer, version, autostart}`.

### 6.3 Protocol (minimal)
- New `SyncItemType::Catalog` (manifest-only) published on pairing/connect and on change over a gossipsub topic scoped to paired peers (`ozone/catalog/v1`); receivers store per-peer catalogs (`peer_catalogs/<peer>.json`) — no replication of artifacts.
- `request-response` protocol `ozone/artifact/v1` for `fetch(offer_id)` → bytes into `quarantine/<hash>/`.
- Events to the UI via the existing `/ws`: `catalog_changed`, `offer_state` (with `trace_id`).

### 6.4 Review & install states (per offer)
`discovered → fetched(quarantine) → auto-review → needs-approval → approved → installed → (degraded | disabled | revoked)`; `rejected` terminal.
Auto-review (no model needed for the first four): hash + signature verify; manifest static checks (endpoints loopback-only, no undeclared ports, declared requirements);
jurisdiction gate over the text (same `check_call_gate` rules as `/mcp/call`); security-mcp posture scan of the artifact tree; then **operator approval is mandatory
for any executable artifact** and is an *awaiting item* (human resolver). Install = write to the tools dir, register in the MCP registry with `auto_start=false`,
record provenance. A blueprint step waiting on a peer tool surfaces as an awaiting item with Approve/Reject (reuses `AwaitingBadge`/`PendingReviewModal` patterns).

### 6.5 Contribution flow
Operator picks a methodology → UI shows its rules and the tools bound per rule and the blueprints that cite it → **preview exactly what leaves this host** (names, rule text,
requires; never data, never keys) → publish → status per peer. Receivers see it as an offer (review states above). A methodology offer carries its `requires[]`, so
installing it *also* offers the tools it needs (dependency view), and once approved it feeds blueprint tool manifests on the receiving host (§3). Needs: implement `Propose`
(replace the stub), populate `created_by`/`consensus_status`, sign offers with the host's libp2p key.

### 6.6 UI/UX
New top-level **Marketplace** tab (`ui/src/App.tsx` nav + `navigation.ts`), four sub-views:
| View | Shows | Data (existing → new) |
|---|---|---|
| **Catalog** | cards for tools/MCPs/methodologies/blueprints from *all paired peers + local*; filters: kind, source peer, capability, requires-device/credential, review state, "satisfies a missing capability"; badges: installed ✓, degraded ⚠, needs-approval ● | **new** `GET /marketplace/catalog`; existing `/network/paired-peers`, `/mcp/tools` |
| **Installed** (this user) | everything installed/downloaded with provenance (from which peer/offer, version, reviewers), running state, autostart, health; reuses ToolsPanel grouping | existing `/mcp/tools`, library #7 (via `/pipeline/execute`); **new** install provenance on the registry rows |
| **Relations** | graph per methodology: **methodology → rules → tools/MCPs/pipelines → blueprints using them**; node = type colour, edge colour = class (structure / requires=flow / cross-modal / provenance), degraded tools red, peer-only tools dashed; click a tool → who needs it; click a rule → its steps; reuses `graphkit` (`GraphWorld3D`) + `EdgeLegendFilters` | **new** `GET /marketplace/relations?methodology_id=` (derived from §3 bindings + `BlueprintStep.methodology_ids`) |
| **Contribute & Review** | contribute wizard (§6.5) and the review queue (needs-approval offers with their auto-review findings, approve/reject, who reviewed) | existing awaiting/reviews (`/order/global`, `/consciousness/reviews/pending`); **new** `POST /marketplace/{publish,fetch,approve,reject}` |

Component plan under `ui/src`: `panels/MarketplacePanel.tsx` (shell, registered in `enginesRegistry`/nav), `views/marketplace/{CatalogView,InstalledView,RelationsView,ContributeView,ReviewQueue}.tsx`,
`components/marketplace/{OfferCard,OfferStateBadge,PeerBadge,RequirementChips,ReviewFindings}.tsx`, `hooks/useMarketplace.ts` (catalog + `/ws` `catalog_changed`/`offer_state`), `ozoneClient.ts` additions (`fetchCatalog`, `fetchRelations`, `fetchOffer`, `publishOffer`, `fetchArtifact`, `approveOffer`, `rejectOffer`). Motion: the relation graph animates *discovery* of a binding (reuse `DiscoveryEffects`); respect reduced-motion (see `docs/UI_ANIMATION_DESIGN_2026-11-03.md`).

### 6.7 Build plan (priority order)
| # | Item | Size |
|---|---|---|
| 1 | Binding schema: `requires[]` on `DecisionRule`/`DecisionRuleInput` + loader + validation + unit tests; annotate the ~20 methodologies that have obvious tool needs | M |
| 2 | Stage 3 collects bindings into a candidate manifest (provenance kept); search demoted to `source:"search"` | S |
| 3 | Blueprint prompt `RULE → REQUIRED CAPABILITIES`; schema `rule_refs`, `tool_calls`, `tool_manifest`; host reconciliation (binding authoritative); persist structured `missing_capabilities` | L |
| 4 | Simulation `tool_checks` with host-computed statuses; availability probe (registry/running/requirements/jurisdiction) | M |
| 5 | Step execution of bound `tool_calls` with arg templates (replace `{query}`), fallback to `select_tools` | M |
| 6 | `tool_health` ledger + actor updates (`mcp-failure`, new `tool-registry-changed`) + meta-ripple categories | M |
| 7 | "Learn once, bind forever": Finding → meta-loop proposes `requires` for rules lacking them | M |
| 8 | Catalog protocol (`SyncItemType::Catalog`, gossip topic, per-peer store) + signed offers | L |
| 9 | Artifact fetch → quarantine → auto-review → approval awaiting item → install/register (autostart off) | L |
| 10 | `Propose` implementation + `created_by`/`consensus_status` population | M |
| 11 | Marketplace UI: Installed + Catalog (read-only) first, then Relations, then Contribute/Review | L |
| 12 | E2E (real hosts, local ↔ EU): rule with a bound tool → blueprint manifest → simulation `ready`/`missing_on_peer` → step really calls the tool; offer published on EU appears in local Catalog; fetch→review→approve→install; degraded tool changes simulation status | L |

---

## 7. Risks
- **Remote code from peers** is the dominant risk; hence manifest-first, quarantine, mandatory human approval for executables, loopback-only endpoints, autostart off, jurisdiction + security review. Do not auto-install.
- **Binding drift**: tool renamed/removed → manifest `missing`; mitigated by capability names (stable) + registry-change actor + per-run re-validation of cached blueprints.
- **Model authority**: if the blueprint model is allowed to invent `tool_calls`, the old failure (narrated tools) returns; the host must treat methodology bindings as authoritative and *validate* anything the model adds against the registry.
- **Privacy**: contributing a methodology must not export task data; the preview step shows exactly what leaves.
- **Budget**: catalog/review are model-free by design; only the meta-loop proposals (§3.5) cost calls and ride the meta-ripple min-gap.

## 8. Decisions needed from the operator
1. Binding granularity: per-**rule** `requires[]` as in §3.1 (recommended), or per-methodology only?
2. May a rule require a **pipeline** as well as MCP tools (recommended yes — one `requires` list)?
3. Marketplace scope: **paired peers only** first (recommended), or also an open discovery network (needs reputation/consensus design)?
4. Executable artifacts from peers: always human approval (recommended), or allow auto-install for signed + previously-trusted publishers?
5. Should installing a contributed **methodology** automatically propose installing the tools it `requires` (dependency prompt), or keep them separate?
6. OK to demote run-time keyword search to fallback/learning source and make bindings authoritative for blueprint tool calls?
7. `auto_broadcast_sync` policy (still open from the P2P work): methodologies/blueprints broadcast by default to paired peers, or only on explicit "Contribute"?


---

## Appendix A — Operator direction, VERBATIM (2026-11-04)

> now tools and mcps and everything is being aggreatedon blueprint creation around simulation when blueprint steps are built around correct this is an important factor remember the orchestrator steps why we have them at firstmethhdologies to gather methodlogies around to have our AMT around right then during simulation and etc we based on rules and steps turn rules to steps and from there with what should or is tools captured in methodlogies with rules so methodlogies should have mcps and tools needed per rule and etc so we need to then fully reviewthis deeper as wellandnow rview all actors and amt expansion etc all conduct the full review capture the important ceof the blueprint because thisis what makes steps from rules and then do we really need to search for tools/mcps when the should be per methodology with the rules and this is again why methodoogy gathering is so important, now also update the UI to reflect the marketplace correctly all registered tools/mcps globally from all users in the P2P and all the user has downloaded and we demosntrate the relationship per methodogy we hav when we have it we contribute methodooogy and this contributes to blueprints mcp/tools callings and etc for user to donoad with upon review as well capture all and now display our marketplace in the UI/UX properly as well as it should be

Interpretation implemented so far: per-rule `requires[]` on `DecisionRule` (`src/types/methodology.rs`), `GET /marketplace/overview`, and a Marketplace tab
(Catalog / Installed / Relations / Contribute & Review). Peer catalogs and install-from-peer are NOT built yet (§5/§6.3) — the UI says so rather than inventing listings.


## Appendix B — Operator question: "didn't simulation and blueprint creation capture tool binding and MCP? how was it captured prior?" (2026-11-04)

**Short answer: tools were *offered* at blueprint creation and *mentioned* in simulation, but never *captured as a binding* anywhere.** Verified in code:

| Phase | What it did with tools/MCPs | What it did NOT do |
|---|---|---|
| Stage 3 (gather) `mod.rs:2662-2702` | built `state.applicable_tools` by substring-matching the request's keywords/topics against every registry entry's `name + capabilities` (matched=true); if nothing matched, listed the first 8 registry entries (matched=false) | take anything from the methodologies themselves — their rules carry no tool field |
| Stage 6 (blueprint) `stages.rs:663-796` | showed the model a REGISTERED CAPABILITIES list (registry filtered by request-word overlap) and APPLICABLE METHODOLOGIES (full rule text) | give the output schema any tool/MCP field — steps are `{step_index, action, description, pipeline_id, context_requirements, depends_on, …}`; a need the model could not meet goes to `missing_capabilities`, which is only logged (`stages.rs:849-863`) |
| Stage 7 (simulation) `stages.rs:~1296` | told the model to "flag any step whose goal matches a registered capability" | record that flag — the output schema (`step_predictions:[{step,needs,produces,risks}]`) has no slot for it |
| Step execution `tool_routing.rs` (only with `OZONE_TOOL_ROUTING=1`) | re-matched tools by word overlap with the step text (≤ 2) and called them with `{"query": step[..200]}` | use anything decided at blueprint/simulation time; pass structured arguments |

So the aggregation the operator describes (methodology rules → needed tools → blueprint steps → simulation validates → step calls the bound tool) **exists only as intent in the prompts and in the base methodology's principle text ("Simulation owns calls — pipeline/MCP/tool calls belong to simulation")**; the data model never stored it. That is why the per-rule `requires[]` field, the blueprint `tool_calls`/`tool_manifest`, and simulation `tool_checks` (§3.1–3.4) are the build, and why `{"query": …}` search remains only the fallback.
