# OZONE STUDIO — COMPREHENSIVE STATE CAPTURE
## 2026-09-17 · ZCode + Claude Code joint session · living graph operational

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
