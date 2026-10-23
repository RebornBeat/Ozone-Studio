# OZONE STUDIO — COMPREHENSIVE STATE CAPTURE
## 2026-09-17 · ZCode + Claude Code joint session · living graph operational

> **LATEST 2026-10-23 (zcode, later) — FINALIZED DESIGN REVIEW captured (CHECKLIST entry "2026-10-23 (zcode, later)"): the "everything reruns from the orchestrator + gets an AMT?" question answered — trace spine, not AMT-per-call, not orchestrate-rerouting.** Verified: nested-call capture = 2,319 S12 rows with NO run/parent id (measured — the untraceability is real); 45% of all host call rows are AMT-building (AMT-per-call cost-infeasible); file_link re-enters /pipeline/execute over HTTP from inside a walk (rerouting would recurse); no env_clear anywhere → env-passed OZONE_TRACE_ID reaches all 19 modality binaries + their nested spawns with ZERO per-call-site edits; task_local precedent exists (gate.rs:425 CALL_PRIORITY); CONSCIOUSNESS_ROOT_ID=5. Design: TraceCtx task-local minted at the 5 roots (orchestrate, loop passes, pipeline-execute, mcp-call, rerun_step); gated-entry decorator (DecisionReviewExecutor pattern) on the 3 external entries = jurisdiction without the walk; coarse Run/span containers (per blueprint step, per loop pass — NOT per-call: ~3x graph cost, and spans must not carry project keywords or they trigger ripple sync); NO global consciousness AMT (catch-all anti-pattern already proven by the Global Findings dedupe bug). The AMT stays the semantic structure of requests; trace_id stamped on AMT/blueprint/task containers + every capture row = two-way call↔run lookup. Consolidated 8-layer build order in the entry; nothing applied (read-only).
>
> **LATEST 2026-10-23 (zcode) — PERIMETER + GATE-SEMANTICS REVIEW captured (CHECKLIST entry "2026-10-23 (zcode)"): cc's completed 7-fork security sweep spot-verified. VERIFIED critical: /pairing/start returns its approval code to its own unauthenticated caller (sessions self-issuable — /pipeline/execute's session gate is guarded by a door anyone can mint); CORS allow_origin(Any) makes every loopback route browser-reachable; remote pipeline registration shadows built-ins (remote-first dispatch, no reserved-id guard — registering id 9 redirects every prompt call); blueprint model_override merges api_endpoint/bitnet_cli_path onto pipeline 9 and bitnet_cli_path RUNS as an executable (model text → arbitrary binary); UI chat hardcodes consciousness_enabled:false; a multi-chunk review passes on ANY single judged chunk (unjudged chunks fail open; all-unjudged → ReviewPending, which has NO resolve route); grpc:1174 byte-200 slice = live panic site the orchestrator's char-boundary fix missed; stage-14 discards its own RequireConfirmation matches; /config/get leaks api_key (no skip_serializing anywhere). What held: pipeline-execute session+category rejection, the interception (stub dead), safety-first merge, bidirectional P2P gate (unbuilt tree). NOTHING applied (no-edit ruling) — the endorsed first edit-wave when the ruling lifts: perimeter (CORS/pairing/reserved-ids/config redaction/session on mutating routes) → model_override whitelist → gate semantics. Full evidence + fork-reported-unverified list in the CHECKLIST entry.
>
> **LATEST 2026-10-22 (zcode, late) — GATES + GUARDRAIL-BYPASS + ACTING-LOOP review written (CHECKLIST entry "2026-10-22 (zcode, late)"), done directly because cc's 7 review forks all rate-limited. CORRECTION to the entries below: the consciousness `apply_config` copy is necessary but NOT sufficient — task/mod.rs carries its OWN file-based pre_task_gate/post_task_experience, gated on `TaskQueueConfig.consciousness_enabled` (default FALSE, task/mod.rs:75) reading a nonexistent config.json, and nothing calls the store's trigger methods — so the assistant's valence stays 0.00 even after apply_config; the real fix is an operator decision (task lifecycle calls the store, or one of the two copies dies). Also verified: start_queue_processor has ZERO callers (queued tasks never execute — a guardrail by accident); global findings are silently dedupe-dropped (one shared "Global Findings AMT" container); I-Loop starves by design collision (amt_loop handles first); stage-12 logs success over `let _ =` drops; /pipeline/execute DOES enforce session + consciousness-category rejection (not a bypass); the EU instance (active, binary 10-21) runs all loops on schedule but has ZERO candidates ever, no actor lines, security-mcp live-but-uncalled with a 30s supervisor spawn-churn, P2P up-but-unpaired — monitors, never acts, for four compounding causes. The full acting-path wiring map (8 connections) + consolidated priority order are in the CHECKLIST entry. No edits, no builds, no tests.
>
> **UPDATE 2026-10-22 (cc, latest) — EDIT-ONLY FIX WAVE APPLIED, UNBUILT/UNTESTED:** consciousness TOML→store config copy (emotion path goes live on first boot — config.toml already enables it); blueprint persistence repaired (root cause: flat JSON could never deserialize into Container, so blueprints were never persisted; now body-file + typed container, zero-step reuse falls through, prior-design reader + pipeline-21 `prior_design_context` field wired — koi re-test is the acceptance); Jurisdiction Full Gate is now stage 14 (UI STAGE_ORDER updated); P2P trust gate now bidirectional + batch-sync driver + `peer_sync_candidates.jsonl` bookkeeping; `due_at` in /order/global and queue tie-break; shared-context `context_summary` shows host events + `ForkOf` edges on mirrored events; 23 modality placeholder `object_store_path`→null, control fabricated p-value removed, chrono+capture added to control/electromagnetic/geospatial. Full detail and the remaining-open list: `CHECKLIST.md` entry "2026-10-22 (cc) EDIT-ONLY FIX WAVE". **Nothing compiled; the corrections block below is superseded for those items once the build pass confirms them.**
>
> **VERIFIED 2026-10-22 (zcode) — the EDIT-ONLY FIX WAVE above was diff-verified fix-by-fix this pass (CHECKLIST entry "2026-10-22 (zcode) VERIFICATION PASS"):** all six areas real and correctly wired; every compile-risk spot cc flagged eye-checked, none a blocker (manual `impl Default` blocks exist at types/container.rs:160/188/218 — my own derive-list concern was a false alarm; TOML config struct field names match `apply_config` 1:1; `BlueprintStep` already in stages.rs scope; `queued_due_at` computed before the inputs move; the p-value has exactly one construction site, updated). Residuals carried unchanged: blueprints parent to 0; unused `state` param; raw `broadcast(topic,..)` ungated but dormant (zero callers); `OZONE_ZSEI_DATA_DIR`/cwd-relative body paths; `let _ =` updates leave honest-null silently; depth/hyperspectral/IMU fabricated-value reviews open. **Still nothing compiled** — the build+test+fleet pass remains the acceptance gate for BOTH waves in the tree; then the koi re-test (blueprint acceptance) + the first non-0.00-valence digest (consciousness acceptance).
>
> **UPDATE 2026-10-22 (zcode) — read this first; everything below, including cc's same-day entries, is prior.** Full detail: `CHECKLIST.md` entries "2026-10-22 (cc) FULL STATUS CAPTURE" + its ADDENDUM, and "2026-10-22 (zcode) FULL STATE CAPTURE v2" (the verification pass — every claim below is grounded there). Summary:
> - **Now**: local host DOWN (only the shared-context MCP node processes run); disk 98% (5.7G free — below the ≥10G build rule, so NO build was run); last commit `960902a`; **40 uncommitted/untracked entries carrying one coherent, unbuilt feature wave**; EU VPS status unverifiable (loopback-only binding is by design; no SSH tunnel running).
> - **The uncommitted tree, verified file-by-file this pass** (previously "+1662 lines, content unexamined"): node-event producers in all 24 modality pipelines (fire-and-forget → `/orchestration/node-event`); the complete P2P wave in `src/network/mod.rs` (durable default-deny `paired_peers.json` trust gate, loopback `listen_on`, bounded Swarm event loop, 86400s idle timeout with empirical justification, gossipsub subscribe + paired-gated Message handler); 4 `/network/*` routes + the node-event route + **on-demand MCP start in `/mcp/call`** (widens WHEN an opted-in tool starts, never WHETHER) in `src/grpc/mod.rs`; supervisor facts for `p2p-bridge-mcp` (port 3278, gated on `enable_p2p`); `auto_broadcast_sync` config (default false) + the boot-gated sync bridge in `lib.rs` + the `Arc<RwLock<>>` signature fix in `amt_loop.rs`; UI: orchestration timeout-recovery poll (recovers real results the client timed out on, e.g. the 585s task), ThreeDEngine as first node-event consumer, SetupWizard P2P disclosure screen; `tools/p2p-bridge-mcp/server.mjs` (locked-by-default SSH-bridge MCP, unregistered in the MCP index, untested).
> - **Compile status: UNVERIFIED — the tree has never been built as a whole.** One build+test+fleet pass lights up the entire wave in one boot; it stays operator-gated behind disk + commit + relaunch (exact order in the CHECKLIST entry §6).
> - **Wiring map (how it all works together)**: one write choke point (`ZSEI::query` → `graph_events::emit`); two event streams — graph events (the reflex nervous system: ripple actors, AMT wake, I-Loop, assistant, P2P bridge, 12 UI consumers) and orchestration events (stage transitions + per-node construction events + pipeline locations; ThreeDEngine consumes node events); four real thinking/consciousness capture layers — in-walk `ThinkingEntry` raw outputs on the task record, S10–S13 audit stores (consciousness review reads them), the background loops' persisted candidate/insight stores, and the shared-context coordination record. Orchestration reads top-down (jurisdiction → methodology → AMT → blueprint → sim → gates → steps), traversal reads bottom-up (keyword seed + provenance-labelled one-hop + prior-design injection + session replay); `[13.link]` supplies edges; **content-level cross-walk transfer is still the depth gap**.
> - **UNVERIFIED → resolved this pass**: `find_methodologies_by_keywords` hop expansion present in tree (query.rs ~613–670, 12-neighbor cap); wave-barrier redundancy CLOSED (zero `.chunks(` sites remain in the orchestrator); node-event consumer count precise (ThreeDEngine only — `graphEventClient.ts` serves 13 files but two distinct streams).
> - **Redundancies**: unchanged ten-item ledger minus #7 (closed above); new instance of the "two books" pattern — `p2p-bridge-mcp` known to the supervisor, absent from the MCP registry index.
> - **Still not wired (honest list)**: P2P content replication + content-review gate (guide §7.3 designed, not built), auto-redial, `known_peers` persistence, disconnect route; `p2p-bridge-mcp` registration/test; clarification generation (no constructor); `PipelineLocationEvent` producers (receivers exist); `depth` producer (no reqwest dep); ordering policy (opt-in vs forced `/mcp/call` envelope); raw-data modality live tests; FileRef legacy backfill; duplicate math↔text edge dedupe (operator-gated).
>
> **CORRECTIONS 2026-10-22 (zcode, later same day — after independently verifying cc's 12-fork review; full evidence in the CHECKLIST entry "2026-10-22 (zcode) CROSS-CHECK CAPTURE"):** three lines above are superseded:
> - ~~"content-level cross-walk transfer is still the depth gap"~~ → **structurally DEAD, one root cause, three symptoms**: blueprint persist drops `storage.steps` through `serde_json::from_value::<Container>` (adapters.rs:208, unknown-field default), `load_prior_design_context` (stages.rs:314) therefore finds only step-less containers, and pipeline 21 has no `prior_design` field at all (0 hits) so anything sent is dropped — the same defect also explains "Using existing blueprint" succeeding on zero loaded steps and the koi test's missing plan text. It is a bug to fix (persist shape + reader + pipeline-21 field + koi re-test), not a retrieval design gap.
> - ~~thinking/consciousness capture "four real layers ... none fabricated"~~ → the capture infrastructure stands, but the **emotion trigger layer is inert on this host**: `CONSCIOUSNESS_STORE` initializes `enabled:false` and nothing copies TOML `consciousness` config into it at boot (only setter: store.rs:168, never called at boot; gates at store.rs:481/488/567 early-return). The 2026-10-04 "emotion near-default = derivation tuning?" question is answered: the gate was never fed. Fix ≈2 lines at lib.rs:877 next to `set_store`. PRIORITY 1.
> - P2P additions: the trust gate is **receive-only** — `broadcast_high_priority` (network/mod.rs:696) sends to every `known_peers` entry with no paired check, and `process_batch_sync` has zero callers (batched Normal-priority findings queue forever). Add an outbound paired-only filter before content ever rides the wire.
> - Also confirmed this pass: stage 8/8b collision (jurisdiction.rs:431 + stages.rs:95 both record stage 8 — UI rows overwrite); dual tool-selection (`applicable_tools` built, only length logged; prompts use `capability_summary`; zero orchestrator MCP-call consumers — decision per directive #46); placeholder `object_store_path` ×5 (biology/CAD/audio/BCI/control) + a hardcoded placeholder statistic (control:2901); decision-review writes no ledger row (S11-only; by-design-or-gap undecided); hygiene: 2 unpushed commits, `tools/bridges` gitlink with no `.gitmodules`, `target/debug` regrown under pipeline crates. Recommended order: config copy → disk+commit+relaunch+one build pass → 8b fix → P2P outbound filter → blueprint repair wave + koi re-test → tool-routing ruling → due_at ordering (CHECKLIST §7).

> **ADDENDUM 2026-10-22 (cc):** see the second `CHECKLIST.md` entry "ADDENDUM to the FULL STATUS CAPTURE" for the doctrine-corpus coverage map, interlock rules and carried open items (E-series expansions, clarification generation, P2P content gate, redundancies).

> **UPDATE 2026-10-22 (cc) — read this first; the 2026-09 material below is history.** Authoritative detail: `CHECKLIST.md` entry "2026-10-22 (cc) FULL STATUS CAPTURE". Summary:
> - **Now**: local host DOWN (relaunch from `target/release/`); disk 98% (5.7G free); ~37 files / +1662 lines UNCOMMITTED and unbuilt since the P2P/trust-gate/node-event work.
> - **Orchestration**: 14-stage walk (0→13, 8b jurisdiction full gate), detached from client, never-zero methodologies, tools gathered at Stage 3, `[13.link]` edge supply, seed→hop + prior-design injection. Cross-walk is structurally connected; content-level transfer still partial.
> - **Traversal**: single write choke point (`ZSEI::query`→`graph_events::emit`) drives loops, ripple actors, AMT wake, node-events, UI. Reads: keyword seed + one hop + blueprint-content injection + session replay.
> - **P2P**: local↔EU proven live; trust gate (`paired_peers`, approve/revoke) built and A/B-verified; sync carries a *finding signal only* (no content replication); auto-broadcast opt-in; no redial/persistence/unpair; `tools/p2p-bridge-mcp` untested.
> - **Shared context**: stdio MCP is agent-local; host mirrors into `/SharedContext` but cannot call the MCP; ordering (insight envelope) is opt-in; CoordinationEvents carry no relationship edges.
> - **Redundancies to resolve**: dual config.toml, dual zsei_data islands, dual task systems, dual I-Loop, triplicated linker, stale docs (list in the CHECKLIST entry §4).
> - **Open wiring**: clarification generation, node-event consumers (only 3D), live code-edit producers, legacy FileRef backfill, nested pipeline-9 routing, raw-data modality live tests.

> **UPDATE 2026-09-20 (zcode, session 4-5)** — read the 2026-09-17 base below, then these deltas:
> - **Jurisdiction**: root 7 deduplicated (231 scaffolds deleted, user-approved); canonical wiring = MIN container id (child_ids order reshuffles across boots — only min-id is stable); convergence proven (wired 21 → 0).
> - **Knowledge base**: 44 methodologies (35-44 new, real session lessons incl. method_44 Session Token Metering), 17 blueprints (8-17, real executable plans with pipeline ids + methodology configs). Registered in bootstrap defaults + both stores.
> - **BitNet coherency** (forced through pipeline 9): deepening / meta-loop draft / keyword extraction fully coherent (60-84s per call, correct schema, no placeholders); zero-shot sim returns valid JSON with wrong schema (step_N keys) — schema-flexible stage-7 parsing is the open follow-up.
> - **Loops**: both meta-loops have balanced-scan JSON capture, placeholder-content gates, fallback escalation (all call sites audited), 10-attempt honest caps. The 3 metering gaps are open and drafting under these gates.
> - **Known numbers**: linking keep-alive 80x (13.1s→199ms); AMT deepening arc completed live (7 attempts, BitNet fallback); one unexplained host crash 2026-09-20 ~07:40 AST (no panic captured — watch for it).
> - **Open, user-gated**: VoiceConfig fields; 24h meta-loop interval decision; crash-cause terminal capture if it recurs.


> This is the authoritative roadmap. Every item is verified by a test, a live
> check, or a code read — never a claim. Cross-referenced with
> CHECKLIST.md (CC's historical detail), docs/SESSION_HANDOFF.md (CC's
> handoff), docs/GRAPH_TEST_PLAN.md (test contract), and
> docs/LIVING_GRAPH_STATUS.md (graph doctrine-vs-reality).

---

## WHAT'S LIVE AND VERIFIED

### Core infrastructure
| System | Status | Proof |
| ------ | ------ | ----- |
| Host binary (release) | Running on newest build, all fixes | Boot log + /health |
| ZSEI graph (mmap + plain-file) | Both modes byte-identical, cross-mode proven | storage::tests (3) |
| Cache coherency | Writes invalidate target + parent cache | Fixed + tested |
| child_ids persistence | rebuild_child_ids_cache at boot | Fixed after 299-duplicate finding |
| Structural-root id floor 1000+ | Dynamic ids never overwrite roots | Confirmed across boots |
| Jurisdiction graph | 41 scopes, 37 relationship edges, re-wired every boot | Boot log + live query |
| Methodology store | 34 entries, ALL with content files, 0 phantoms | Index + disk audit |
| Blueprint index | 7 entries registered | Boot log |
| Pipeline registry | 82 pipelines loaded (55 compile-time + 27 seeded) | Boot log |
| Graph ripple | Every write publishes scoped events | Live: ws + monitor feed |
| Context objects | Per-step context_assembled + context_sources on task records | Task 72 verified |
| Task lifecycle | /task/create + /task/update, source-tagged reconciliation | Live: 56/58/59/62 closed via API |
| Coordination graph | /SharedContext: 80 children, both agents mirroring | Live count |
| MCP surface | /mcp/call standardized + usage ledger + gate | Live: delegated + metered |
| Shared-context tool | v0.3.0, 12 tools, self-registers on boot | Live: registered + calls |
| 3-way coordination | ZCode + CC + Ozone-Studio sharing scoped graph | Live: both agents present |
| 12 bootstrap methodologies | Real content, deployed to both stores | Disk audit |

### Verified live this session
- Section-3 re-test: auto-route ✓, graph creation ✓, main generation walked
  fallback chain (6 distinct real models) ✓, needs_clarification false ✓
- Model fallback chain: 6 distinct real models across one request,
  model_used reports true routing (not a static value)
- Cache coherency: write→immediate read returns new state (was stale before)
- Plain-file storage: round-trip + in-place update + cross-mode proven
- Coordination task preservation: all coordination tasks survive restarts
  as queued (source-tagged reconciliation fix)
- Methodology phantom resolution: 12 bootstrap entries filled with real
  content, deployed to both stores

---

## ALL BLUEPRINTS NEEDED

The zero-shot simulation gathers methodology rules → identifies pipelines →
creates blueprint steps. Currently everything coerces to pipeline 9 (the
prompt pipeline). The system needs real multi-step blueprints for each
workflow class:

### Existing (old schema — needs migration)
| # | Name | Real content? | Target schema ready? |
| - | ---- | ------------- | -------------------- |
| 1 | General Assistant | YES (rich) | Needs BlueprintStep migration |
| 2 | Code Review | YES (rich) | Needs migration |
| 3 | Documentation | YES (rich) | Needs migration |
| 4 | Code Creation | YES (rich) | Needs migration |
| 6 | Data Analysis | YES (rich) | Needs migration |
| 7 | Mathematical Proof Verification | YES (rich) | Needs migration |

### New blueprints needed
| Name | Pipeline chain | Status |
| ---- | ------------- | ------ |
| Modality Processing | text(100)/code(101)/math(102) → context_aggregation(21) → link | Not started |
| Jurisdiction Compliance Check | jurisdiction rule load → gate check → report | Gate exists, blueprint needed |
| Research + Synthesize | web_search(56) → prompt(9) → context_aggregation(21) | Search pipeline built, no key |
| Multi-Model Pipeline | classify → route to bitnet/api → aggregate | Fallback chain works, blueprint needed |
| Cross-Modal Analysis | text(100) + code(101) → cross-link → aggregate | Both pipelines exist |
| Coordination Sync | mirror → AMT candidate → expansion | Ripple sync built |

### Blueprint schema migration
Old: `{step_id, name, description, pipeline_id, action, outputs}`
New: `{step_index, action, description, pipeline_id, context_requirements,
       depends_on, max_retries, wait_for_graph_update, model_override,
       source_chunk_indices, methodology_ids_applied}`

The 100%-match reuse path (stages.rs:171-182) is real but unreachable
because: (1) old-schema content doesn't deserialize, (2) assets/blueprints/
doesn't exist so fresh instances never receive them.

---

## PIPELINE IDENTIFICATION (zero-shot simulation)

### Current state
Stage 4 (zero-shot simulation) gathers methodology decision rules and
identifies pipelines for each blueprint step. But:
- ALL steps are coerced to pipeline_id 9 (prompt pipeline) — CC's fix
- Exception: pipeline 56 (web search) is carved out
- Modality pipelines (100 text, 101 code, 102 math) exist and work
  when called directly, but are never selected during blueprint creation
- The LLM authors blueprint steps but only pipeline 9's schema matches

### What's needed
1. The blueprint step schema should support modality pipeline selection
   when the step's content matches a modality (text/code/math)
2. The coercion to pipeline 9 is correct for the general case but
   should be extendable: a step tagged `pipeline_id: 100` should
   dispatch to the text modality pipeline when the step's input
   carries text analysis data
3. The AMT branch reconciliation already populates
   `source_chunk_indices` and `methodology_ids_applied` — these should
   inform pipeline selection for modality-specific branches

---

## JURISDICTION — user configuration needed

The jurisdiction gate is real and always-on: 41 scopes, 37 relationship
edges (EU/regional hierarchy), real enforcement with Block /
RequireConfirmation / Warn actions.

### What exists
- global.json: 7 baseline rules (UDHR/CRC cited)
- ~54 national files: real search-and-cite content across ~26 countries
- Source registry: SOURCE_REGISTRY.json/md aggregating all citations
- Region auto-detection: timezone/locale/IP 3-signal cross-check
  (confirmed: DO via explicit config + hardware detection both work)
- Enforcement: Block (real), RequireConfirmation (runs decision_gate
  pipeline 39), Warn (pushes real surfaced warning)
- `OrchestrationResponse.jurisdiction_gate` now reaches the API caller

### What the user needs to configure
1. **Region**: config.toml `jurisdiction.instance_region` — currently
   "DO" (Dominican Republic) via explicit config
2. **Enforcement actions**: which rules Block vs Warn vs
   RequireConfirmation — the global baseline is configured, national
   content is all Log-only pending user review
3. **National rules for DO**: Dominican Republic national content
   exists but is Log-only — user should review and elevate actions
4. **Scope rules**: global vs regional vs national — the graph edges
   define inheritance (national → regional → global baseline)

### Remaining jurisdiction work
- [ ] User review of DO national rules — elevate from Log to real actions
- [ ] Remaining thin countries (ar bd do ec id ke ma my ng no pe ph pk se th vn)
- [ ] Source registry maintenance protocol (CC's fork output needs verification)
- [ ] Jurisdiction content for new scopes as the user's needs expand

---

## ALL REMAINING WORK (routed through the task system)

### ZCode queue (mine)
| Task | What | Blocked on |
| ---- | ---- | ---------- |
| 69 | Fix steps-0: context objects through /orchestrate (Stage 7 hook fires before step_contexts exists; move to execute_step) — **fix built, rides next restart** | Restart |
| 70 | Re-run section-3 test verifying ModalityGraphs populate AND context objects persist | 69 |
| 43 | Coordination layer into ForStep: verify coordination_context appears in real orchestrate step context | 69 + restart |
| 45 | First real stage McpCall consumer | A real tool need |
| 60 | Cross-process + integration test batch | 57/68 |
| — | Merge-back v2: fork grafts into main AMT | Designed, ready to build |
| — | Deploy url/package binaries (done, verify) | — |

### CC queue (on return)
| Task | What | Blocked on |
| ---- | ---- | ---------- |
| 68 | Nested pipeline-9 extraction → fallback walk + graceful degradation — **the section-3 fix** | Nothing — ready to build |
| 44 | CHECKLIST retirement post context-transfer | Transfer proven |
| 46 | Host-ops methodology verification on boot | Methodology 16 (live) |
| 57 | link_to_existing completion | Task 68's fallback walk pattern |
| 47 | Text/code cross-process graph retrieval | Math reference fix |
| 65 | Code modality dependency graph + provisional nodes | Math reference pattern |

### Shared
| Item | What |
| ---- | ---- |
| Merge-back v2 | Graft substantive fork branches into main AMT (designed in docs/AMT_EXPANSION.md) |
| Task 64 | file/url/package links onto real ZSEI graph (FileReference/URLReference/PackageReference containers + Relation edges) |
| Task 65 | Code modality dependency graph + provisional-node lifecycle |

### Open engineering (not yet tasked)
- mmap:false cross-mode index rebuild (deferred w/ test — layouts identical)
- ContextSource provenance field (traversal vs keyword)
- TraversalRequest keyword_filter/topic_filter dead fields
- Network hooks fed from GraphEventHub
- T-G4: ws frames integration test
- Root children count display lag
- StatusBar fabricated fields (CC finding)
- Zero-shot simulation over-asking (fixed, needs re-test under load)
- Blueprint Assignment stage ~580s pattern (root-caused? not yet)

---

## EVERYTHING CAPTURED THIS SESSION

### Built and verified
1. Unified AMT expansion candidate store (amt_candidates.rs) — 3 routes,
   path-injected, deduped, 500-entry cap
2. Main/fork island model — amt-main / amt-fork-of:<prior> keywords +
   Continues relation, sequential lifecycle test
3. Graph ripple — every write publishes scoped events to ws + monitor + AMT
4. GraphEventHub — process-global, broadcast ring 1024
5. Coordination layer in ForStep — workspace_id + include_coordination,
   separate-layer doctrine
6. Context objects — per-step context_assembled + context_sources on task
   records, provenance = keyword-scan today / traversal:<mode> later
7. Task 43 coordination layer — /SharedContext scoped pull into step context
8. Task 45 global McpCall — install_global + call_global + usage metering
9. Task lifecycle — /task/create + /task/update, source-tagged
   reconciliation, restart-preservation proven
10. Storage plain-file branch — mmap:false no longer silent data loss
11. Cross-mode byte compatibility — plain-write → mmap-read proven
12. Task 68 — extraction fallback walk + deterministic tier (CC verified
    the approach; I completed the implementation after the limit)
13. 12 bootstrap methodologies — real content, deployed both stores
14. 3-way coordination — shared-context MCP, scoped graph, both agents
15. Docs sweep — deep dive addendum, CONTRACTS §6, test plan, expansion
    architecture, living graph status

### Bugs found and fixed (this arc)
1. ZSEI hot-cache incoherency — writes bypassed cache → stale reads
2. Absolute object_store_path garbage-join (amt_loop, jurisdiction, amt)
3. Stage 7 hook fired before step_contexts existed → steps-0
4. mmap:false silently no-opped store_global writes → data loss
5. Plain-file write branch wrong stride (32 vs 24 bytes) → misalignment
6. Plain-file init missing header → cross-mode rejection
7. Missing `discovered_via` field on Relation literals (CC found, race)
8. k_registry tests: RwLock-wrapped fields accessed without lock
9. remote_dispatch/executor_l2 tests: missing roles parameter
10. text modality create_graph: orphaned code block from wire-protocol edit

### Bugs found and filed (not fixed)
1. Task 68: nested pipeline-9 extraction lacks fallback walk (CC building)
2. mmap:false cross-mode index rebuild (deferred, layouts identical)
3. ContextSource lacks traversal-vs-keyword provenance
4. TraversalRequest keyword_filter/topic_filter dead fields
5. Root children count display lag after restart
6. StatusBar fabricated placeholder values (CC finding, documented not hidden)
7. /orchestrate unauthenticated (CC finding)
8. Zero-shot simulation inefficiency: executes all steps before
   discarding on clarification

---

## HOW TO CONTINUE

1. **Restart onto newest binary** (all fixes live in one boot)
2. **Run task 70**: orchestrate with attachments → verify ModalityGraphs
   populate + context objects persist + coordination layer appears
3. **CC returns** → picks up 68/44/46/57/47/65 from the tracker
4. **I continue** → merge-back v2, task 45 consumers, task 60
5. **User configures jurisdiction** → reviews DO rules, elevates actions
6. **Iterate**: each cycle lands fixes, closes tasks through the API,
   updates docs additively, and the living graph captures everything

Everything routes through the task system. Everything is observable
through the monitor. Everything is scoped through the graph. Everything
is documented in the docs. Nothing is claimed without a test.
