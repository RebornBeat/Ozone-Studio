# FOR CC REVIEW — the watchdog question: kill-on-age vs kill-on-silence vs retry — what is the ACTUAL solution?

Operator's request (2026-10-07, verbatim intent): "the solution isn't budget — identify what the problem is. What is this watchdog even — why watchdog if we have file beacon? Why is it conflicting so much? On silence RETRY — we shouldn't be skipping or dropping things but actually fixing to NOT have errors, not silencing errors. If we need all data and should be making it quick. Look at: is this a MODEL / PROVIDER / OZONE-STUDIO problem, optimization, or HARDWARE? Can we parallelize?" ZCode is to share all insights; CC reviews and provides its insights. NO EDITS, NO BUILDS until reviewed.

## 1. The layer map (what exists, what each is for)

| Layer | Mechanism | Bound | Purpose |
|---|---|---|---|
| L1 | reqwest client per model call (pipelines) | 120s | one HTTP attempt |
| L2 | walk per-attempt watchdog (K-registry `model-300`) | 300s/attempt ×(1+candidates) | an LLM attempt escalation inside the fallback walk (pipeline 9) |
| L3 | pipeline CHILD bound (executor) | WAS blind 300s+30 → patch: 3× for ids 100/8/103 → in-tree UNBUILT: idle-kill 150s silence + 1800s ceiling | don't let a stuck child hold a gate slot forever |
| L4 | inner children (llama-cli via OZONE_PIPELINE_TIMEOUT_SECS) | 600s (patch build) / child_secs-15 (older) | one local-model call |
| L5 | file_beacon | 45s poll | UNRELATED to liveness: file-change→graph ripple (mtime watching). The name collided with the concept; it does NOT watch pipeline children. |

## 2. What happened (the live data)

- **E2E-2 v1 FAILED at exactly 300s**: pipeline 100 (text: intent extraction + chunking = 2-3 internal model calls × up to 120s each) was killed as "hung" by L3 while legitimately working. Its rerun on the 3× patch build SUCCEEDED in 14.2 min — the child was streaming the whole time. Classification: **OZONE-STUDIO (blind age bound), triggered by provider slowness.**
- **Jurisdiction rules_loaded=0** (was 22): the rule content comes from live boot LLM calls ("jurisdiction content search call failed region=DO" ×3) → **PROVIDER/MODEL-side trigger exposing an OZONE-STUDIO design fragility**: registered scopes (74) persist, but rule CONTENT is re-fetched live at boot instead of persisted to the graph — a provider outage leaves the gate soft-open.
- **Reasoning-cap exhaustion** (walk #3 trail): `empty [ozone-studio] → our max_tokens was exhausted by reasoning` — correctly labeled, moved on WITHOUT same-model retry. **MODEL behavior + OUR cap.**
- **google/lyria (music model) ranked first in the pool** by window size — burned attempts; fixed organically via ledger-ranked pool. **OZONE-STUDIO.**
- **Free-model empties/errors** (2/15 usable on bad days; 13/15 on great days) — **PROVIDER/MODEL.**

## 3. The patch history (what the operator rejected, and why they're right)

1. Blind 300s age bound (original): kills slow-but-working, waits the full budget for stuck. Wrong signal.
2. ZCode patch A (3× budget table for multi-call pipelines): works live, but is a guess per pipeline id — a working 4-call pipeline still dies, a stuck one still waits 900s. **A patch.**
3. ZCode patch B (in-tree, UNBUILT, NOT deployed): idle-kill — the child's own stdout/stderr streams refresh an activity clock; silence >150s = hung (kill); 1800s absolute ceiling. Better, BUT the operator's objection stands: **silence ≠ hung for reasoning models** (a model thinking for 3 minutes produces no stdout — patch B would kill it mid-thought), and **kill is still kill** — the directive is RETRY and diagnose, never drop.

## 4. The design space for CC's insight (the actual questions)

Q1. **Should the child EVER be killed, or only retried?** A genuine deadlock (process wedged) can only be resolved by kill-then-retry-invocation; but the decision to kill needs a REAL stuck-signal, not silence. Candidate: a heartbeat PROTOCOL — the pipeline prints a marker BEFORE each internal model call (`→ call model X (attempt n)`); the parent's stuck-window counts from the last CALL-START marker: if no new call starts within (HTTP bound + margin) after the last started call reported nothing, that SPECIFIC call is hung → kill is justified and NAMED. Silence between calls without a started-call marker is expected (thinking/processing).
Q2. **Where should retry live?** Inner (the pipeline retries its own model call — the walk already does per-attempt retries for pipeline 9) vs parent (re-invoke the whole child). Parent-level retry duplicates ALL completed work; inner retry preserves it. Proposal: inner retry per model call with the walk's existing no-skip semantics; child kill only for diagnosed process-level hangs, followed by ONE full re-invocation, recorded.
Q3. **Diagnosis before kill**: the ledger + attempt trail already classify post-hoc (provider/model/local/ozone-studio). The child's marker protocol (Q1) makes it pre-hoc: "hung on call 2 of 3 to X" lands in the error text — no silent kills, every failure named.
Q4. **Make it quick / parallelize**: the text pipeline's internal calls are SEQUENTIAL over chunks — chunk-level fan-out (gate-bounded, lanes-style) would parallelize I/O-bound HTTP waits; CPU-bound work (BitNet inference) is the only true serial limit on this 4-core/7.6G box. The gate already shapes concurrency. Opportunity: parallel chunk extraction inside pipeline 100 vs the ordered-coverage doctrine (#48: batch coverage → parallel lanes) — needs a doctrine check.
Q5. **Jurisdiction content persistence**: persist rule content to the graph at first successful load (registered scopes already persist); boot reads the graph, refreshes opportunistically. Provider outage then degrades to STALE rules (labeled) instead of ZERO rules.
Q6. **The conflict answer**: there is no watchdog-vs-beacon conflict — file_beacon is file-change ripple (unrelated layer). The real conflict was THINKING (age/silence timers = guesses) vs KNOWING (the child's own call-start markers = truth). The actual solution is the marker protocol + inner retry + named diagnosis; budgets only as last-resort ceilings.

## 5. Current state (frozen for this review)

- DEPLOYED production binary: the 3× patch build (E2E-2 rerun succeeded on it, 14.2 min).
- IN-TREE, UNBUILT, NOT deployed: ZCode's idle-kill patch B (executor.rs) — held for this review.
- E2E battery: walk 1 (methodology+rules) SUCCESS 11.6 min; walk 2 (creation) SUCCESS 14.2 min (patch build); walks 3 (multi-intent) + 4 (tools) NOT YET RUN — paused pending this review.
- Quota: ~940/1000 remaining. Ledger/context-capture growing (171 ledger entries, 295 records).

---

## 6. CC's audit insights (received) + ZCode's responses — the back-and-forth record

**CC VERIFIED (against disk, not narration)**: layer map accurate; idle-kill genuinely unbuilt+held (edit 21:24 < halt 21:34 < build 20:59); AMT per-pass bound real; quota healthy with the UTC-day-rollover nuance (resets 1:00 AST — explains apparent "jumps"); production loops STILL hitting watchdog timeouts live (not just in tests); 943597a already on origin, a65a6a7 not pushed; uncommitted set = CHECKLIST + this doc + executor.rs (idle-kill) + tools/bridges.

**CC CORRECTION — ACCEPTED (my fix oversold)**: catalog_pool truncates to pool size by context_length FIRST, then ledger-re-orders WITHIN the truncated set. A huge-window bad-history model still occupies a pool slot, just demoted — not "sinks below proven text models" as I wrote. Proper fix (queued, no edits during halt): apply ledger rank BEFORE the truncate (rank the whole candidate set, THEN take size), so bad history actually drops out of the slot set and free slots go to proven models.

**CC GAP #1 — ACCEPTED: hardware was never used as a class.** And the live evidence closes it: the assistant digest + i_loop watchdog timeouts happening RIGHT NOW are marginal BY ARITHMETIC — digest max_tokens=350 on BitNet at ~1.5 tok/s ≈ 233s + load ≈ the 300s L2 budget. That is a HARDWARE constraint (this box's inference speed), not provider/model/ozone-studio. Fix direction (operator decision): Loop-tier L2 patience (background loops have no waiting user — doctrine-consistent), and/or digest max_tokens sized to hardware, and/or the keep-warm server removing load time. BitNet throughput itself (~1.5 tok/s here) gets its own hardware line in every future classification.

**CC GAP #2 — ACCEPTED: parallel-candidate dispatch in the WALK was missing from my doc.** My insight on the trade-off: firing N free candidates concurrently and taking first success trades quota (N× spend per call, against the ≤2-3-per-failing-model economy) for user latency. Doctrine-#48 tension: ordered coverage vs race-to-first. Resolution candidate: parallel fan-out ONLY for the User tier when the gate queue is empty (a human is waiting), ordered chain preserved for replay/ledger; never for Loop-tier. Operator decision.

**CC GAP #3 — ACCEPTED: checkpointing.** A child kill today discards all in-flight work — "no dropping" isn't honored end-to-end. Candidate: chunk-level checkpoints in pipeline 100 (persist each completed chunk extraction keyed by content hash; resume from the first incomplete) — pairs with the Q1 marker protocol (markers make natural checkpoint boundaries). Bigger design item; needs the walk to validate.

**CC GAP #4 — NOTED**: the doc defers rather than recommends, matching the "that's it" scoping. With CC's insights now in, the consolidated proposal (A1-A6 below) IS the recommendation set awaiting the operator's word.

**ZCode addition (live arithmetic, hardware class)**: the loops' current watchdog timeouts are L2-attempt-budget marginal at BitNet speed — this is the first REAL hardware-classified failure in the ledger era; it belongs on the hardware line CC asked for.

## 7. Consolidated decision list (both agents' insights, awaiting operator)

A. Watchdog/retry design: (1) adopt call-start heartbeat protocol — kill only a named hung call; (2) retry location: inner per-call (preserves work); (3) parallel-candidate dispatch for the User tier (quota/latency trade-off stated); (4) chunk-level checkpoints in pipeline 100; (5) jurisdiction content persisted to the graph (stale-labeled beats zero); (6) BitNet throughput classified HARDWARE — decide Loop-tier L2 patience + keep-warm priority.
B. Accumulated, awaiting operator: (7) push a65a6a7; (8) 231 orphans purge-or-keep; (9) repo hygiene remainders (Cargo.lock futures, tools/bridges, tracked zsei_data, ILoop entry 44); (10) idle-kill patch: commit-for-review or discard for the A1 design.
C. Carried: (11) settings_tab allow_paid_models landed (CC to re-read at leisure); (12) BitNet last-resort confirmed, entangled with A6.

---

## 8. CC's correction to A6 — ACCEPTED (my rate error), the classification re-done properly

**My error**: the "marginal by arithmetic" claim used 1.5 tok/s as a constant per-token rate. Our own CHECKLIST (2026-09-27) documents that 1.54 tok/s is LOAD-DOMINATED (measured on a 99-token call where most of the 64s was one-time model loading); pure decode is ~7-9 tok/s. Redone correctly: digest 350 tokens ≈ 40-60s load + ~44s decode ≈ **80-105s — nowhere near marginal against 300s alone.**

**What the log actually shows**: at 00:35:16.351 the I-Loop reflection AND the assistant digest hit their watchdog timeouts within milliseconds — **two background loops piling onto the one local model at the same instant**, each paying a full cold load (keep-warm is NOT configured anywhere in config), contending for 4 cores. That's not "BitNet is too slow" — it's **no single-flight protection on the local model, compounded by keep-warm being off.**

**A6 CORRECTED (replaces the hardware-ceiling framing)**: OZONE-STUDIO design gap with a hardware component. Two concrete parts:
1. **Single-flight on the local model**: only one llama-cli invocation at a time; the second caller queues (Loop-tier — no waiting user, doctrine-consistent). Implementation candidate: a semaphore(1) in the local-model call path keyed by model_type (bitnet/gguf), so API calls are untouched.
2. **Keep-warm (decision D3, pending since RUNTIME_MEMORY_PLAN)**: the OZONE_LLAMA_SERVER_URL resident-server path ALREADY EXISTS in code — default it ON when a local model is configured. A resident server pays load once and naturally serializes requests per slot — it largely SUBSUMES single-flight. Recommendation: D3 default-on FIRST, single-flight as the fallback for the no-server path.
BitNet's raw throughput (~7-9 tok/s decode on this box) stays on the hardware line — it bounds worst-case wall-clock but is not the cause of the observed timeouts.

**Everything else stands as written** (CC: "not re-litigating" — A1-A5, B7-B10, C11-C12 await the operator's word).

## 9. Operator go: review forks dispatched, then edit forks. Six gaps closed first (CC)

The operator approved moving from design to implementation, in that order: read-only review forks first, edit forks after. Before dispatching review forks, six things the consolidated list left ambiguous are resolved here so the forks implement one plan, not six guesses:

1. **B10 is DISCARDED, not "awaiting word".** The operator explicitly rejected blind-silence kill ("silence ≠ hung", "kill is still kill"). The uncommitted idle-kill patch in `executor.rs` (150s silence, 1800s ceiling) is superseded by A1+A6 below, not merged. Its only surviving piece is the **1800s absolute ceiling**, kept as a last-resort safety net — not the 150s-silence trigger.
2. **L2 (the walk's 300s per-attempt K-registry watchdog) is kept**, but only as an outer ceiling. Once A1 lands, L2's failure message must name the in-progress internal call (read from the child's last marker), not just "timeout after 300s" with no detail.
3. **A1's marker protocol scope is NOT the guessed ids (100/8/103).** Review fork R1 enumerates, by reading every pipeline's `main.rs`, which ones make more than one sequential model call per invocation — that list, not a guess, gets the marker.
4. **A3 (parallel-candidate dispatch) is scoped**: OpenRouter free candidates only, never local (A6 covers local via single-flight); User-tier only, and only when the gate's queue is otherwise empty; every fired attempt — including the ones that lose the race — still gets a ledger/attempt_trail entry. Width: 3 concurrent (matches the existing pool size default), not unbounded.
5. **A4 (chunk checkpoints) storage**: the ZSEI graph, not a new file format. Each completed chunk persists as it finishes, keyed by content-hash + chunk index, so a kill-and-retry of the same invocation can skip chunks already on the graph.
6. **A6 / D3 resolved**: keep-warm defaults ON when a local model is configured (the resident-server path already exists in code). A dedicated single-flight gate (semaphore of 1, keyed by local model type) is the hard guarantee independent of keep-warm state — implemented regardless, since a server can still be restarting or absent.

**Dispatch order now**: R1-R7 review forks (read-only, one per area below), each maps exact file:line implementation points and flags blockers. Edit forks follow once R1-R7 report — not in the same pass.

| Fork | Area | Reads |
|---|---|---|
| R1 | Marker protocol scope + exact kill/diagnosis site | every `assets/pipelines/*/*/main.rs`, `src/pipeline/executor.rs`, `src/orchestrator/adapters.rs` |
| R2 | Inner retry semantics vs. the walk's existing per-candidate retry economy | `src/orchestrator/mod.rs` (walk), the pipelines R1 names |
| R3 | Parallel-candidate dispatch feasibility | `src/orchestrator/mod.rs` walk/chain_candidates, `src/pipeline/gate.rs` |
| R4 | Chunk checkpoint feasibility in the text pipeline | `assets/pipelines/modalities/text/main.rs`, `src/zsei/*` graph write path |
| R5 | Jurisdiction content persistence | `src/orchestrator/jurisdiction.rs` |
| R6 | Keep-warm default + single-flight gate | `assets/pipelines/general/prompt/main.rs` (keep-warm path), `src/pipeline/gate.rs`, `src/config/mod.rs` |
| R7 | Operational items: push status, 231 purge mechanics, hygiene, idle-kill patch removal | git state, `zsei_data`, `Cargo.lock`, `tools/bridges`, `src/pipeline/executor.rs` |

No edits made by CC yet this pass except this capture. Dispatching R1-R7 now.

## 10. R1-R7 review forks reported. Three of the six closed gaps changed on contact with the code. Corrected, scoped plan below.

**A5 is a non-issue — DROP from the edit list.** `load_jurisdiction_rules` (jurisdiction.rs:122-222) makes no LLM call. It's a ZSEI search+traverse plus a local file read, already content-persisted, already loud-warns on an empty disclaimer or parse failure, already region-keyed. The background `populate_jurisdiction_content_for_region` fires via `tokio::spawn` *after* the current request's rule count is fixed (jurisdiction.rs:247 computed before the spawn at 256-271) — it cannot be the cause of that request's `rules_loaded=0`. Confirmed independently (CC read the file directly) and by a live check: 68 local files currently carry jurisdiction rule-set content, so this isn't a systemic gap — that boot's miss was a point-in-time search/scope issue, not a design fragility. No edit needed here.

**A1 (marker protocol) needs more plumbing than "print a marker" — DEFER the full version.** L2 (the walk's 300s outer `tokio::time::timeout`) is *shorter* than L3's own ceiling (now 1800s after B10), so L2 cancels the future and drops it before L3's own silence-window logic would ever get a turn. A marker is only useful if L2's own cancellation-message code can read it at the moment it fires — that needs new shared state (e.g. `Arc<Mutex<Option<String>>>` per in-flight child, updated by executor.rs's incremental stdout-drain thread, read by whichever of the three L2 sites — adapters.rs:125, mod.rs:2583, mod.rs:3154 — builds the timeout error). That's a real architecture change, not a small edit. Building it correctly deserves its own pass with the operator's sign-off on the shared-state design, not a rushed fork. DEFERRED, not abandoned.

**A2 (inner retry) is clean and scoped — BUILD NOW.** Pipeline 100's internal calls do not go through the host's walk at all: `llm_execute` → `SubprocessExecutor::execute` (text/main.rs:7672) spawns pipeline 9's own binary directly as a nested child, with its own standalone one-shot model dispatch (confirmed by reading the function's doc comment and body) — no fallback, no ledger, no quota gate, no cause classification. "Inner retry" here cleanly means: retry the nested spawn itself, once, inside `llm_execute`, using the same classification rules `next_step` already uses host-side. This naturally preserves every other already-completed internal call, since each is a separate process.

**A3 (parallel dispatch) is buildable now, with the gate-empty condition dropped for v1.** `src/orchestrator/mod.rs:3751`'s candidate loop is strictly sequential, one await at a time. `amt.rs:2673`'s `JoinSet` + `with_priority(CallPriority::Lane, ...)` pattern is a direct, working precedent to mirror for `CallPriority::User`. The "only when the gate queue is empty" condition needs a new `gate_snapshot()` method on the `PipelineExecutor` trait that doesn't exist yet — rather than block A3 on that, v1 gates on **User-tier only**, width 3, every fired attempt (including losers) still recorded to the ledger/trail. The gate-empty refinement is a follow-up, not a blocker.

**A4 (checkpointing) found a standalone bug, and is scoped down to chunk-level.** `extract_grammar_from_graphs` (text/main.rs:5646) regenerates a random `node_id` on every call, including retries of the exact same chunks — meaning a retry today doesn't just fail to resume, it writes **duplicate sentence containers**. This is fixed regardless of anything else, as its own bug. For checkpointing itself: chunk-level is straightforward (`ProcessedChunk.index` + `hash(original_text)` is a stable key; `create_chunk_graph`, text/main.rs:1375, already exists and just needs to be called as each chunk completes instead of only at the end). Sentence-level resume-skip is deferred — R4 flagged a real added-latency cost on every clean run (not just retries) that needs an explicit operator nod before building, since it trades against "make it quick."

**A6 split — single-flight BUILD NOW, keep-warm DEFER.** R6's finding: keep-warm is reach-only. Nothing in the repo spawns `llama-server` — the doc comment at prompt/main.rs:973 literally calls it a manual operator step, and `OZONE_LLAMA_SERVER_URL` isn't even exported by the host (`ModelConfig::to_pipeline_env`, config/mod.rs:755-780, has no line for it). "Default ON" would mean building a new subsystem (config field + host env export + process lifecycle: spawn-at-boot, health-restart, shutdown) — not a flag flip. That's its own design pass. Single-flight, by contrast, is fully scoped: a second `OrderedPipelineGate::new(1)` instance, **never given `spawn_keeper()`** (R6: calling it would let the starvation guard force-grant a second holder past max_active, defeating single-flight entirely), gated at `executor.rs:308-310` right alongside the existing `gate.admit()` call, conditioned on the resolved model targeting bitnet/gguf — one change point covers every caller (the walk, i_loop, assistant, meta_loop, amt_loop, amt.rs all converge there). This directly fixes the CONFIRMED contention bug (two loops hit the same local model at 00:35:16.351ms and died together) without needing the keep-warm subsystem at all.

**B7-B10, confirmed by R7 (independent re-check, not narration):**
- B7: only `a65a6a7` is ahead of `origin/main`; `943597a` already pushed. Push is the operator's own action per their stated intent ("I will push") — not done here.
- B8: still live, 231 containers, exact id list confirmed in the real log.
- B9: Cargo.lock's diff is now **empty** — that item is already resolved, remove from the open list. `tools/bridges` is dirtier than previously described and contains a `.zcode/` dir. `zsei_data` tracking (686 files) and the `ILoop` entry are unchanged, still operator-gated. New minor item: nine stale `.claude/worktrees/agent-*/` directories (disk hygiene, nobody asked).
- B10: confirmed exact current lines (`IDLE_KILL` :103/used :167, `ABS_CEILING` :104/used :173) — the fix is a straightforward removal, bundled into the single-flight edit fork below since both touch executor.rs.

**Dispatching 3 edit forks now** (each on disjoint files, claimed before editing): EXECUTOR (remove IDLE_KILL, add single-flight gate), TEXT (node_id bug fix, inner retry, chunk-level checkpoint persistence), WALK (A3 parallel dispatch, User-tier, width 3). A1's full marker-with-shared-state design and A6's keep-warm subsystem are explicitly deferred to their own future pass, not silently dropped.
