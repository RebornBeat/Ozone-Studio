# Optimization-route review + host-log review (2026-11-06) — CAPTURE ONLY

**Status: nothing was edited, built, restarted or configured by this review.** It adds this file and one pointer line in `CHECKLIST.md`. Every item below is a *proposal for review*; none is started.

**Pattern under review** (the `confirm_times` precedent, now in `assets/pipelines/modalities/text/main.rs:2692-2724`): the first vote still gates, the rest run concurrently, **both routes are kept**, the concurrent route is switched on by config for high-end hardware / ample quota, the sequential route stays the default and stays available for local models or constrained setups. The decision rule never changes — only how the same calls are scheduled.

**Evidence legend.** `[M]` measured from this machine's files (ledgers/logs/sizes, command run during this review) · `[C]` read in code (file:line) · `[I]` inference, not measured. Anything not marked was not verified.

**Rules honoured.** No removal of all-N-agree rules, the shared rate limiter, watchdogs, jurisdiction/gates. Where a speed-up would change the *statistics* of a decision (not its rule) it is flagged.

---

## 0. Ground truth this review is built on

| Fact | Evidence |
|---|---|
| This box: 4 cores, 7 GB RAM (≈2 GB `available` while a build ran) | `[M]` `nproc`, `free -g` |
| Shared OpenRouter limiter: default **20 requests/min**, shared across all processes by a lock-file bucket; waits up to **150 s** then proceeds and "the host 429 handling decides" | `[C]` `assets/pipelines/general/prompt/main.rs` `openrouter_limiter_acquire` (rpm `OZONE_OPENROUTER_RPM`=20, deadline `OZONE_LIMITER_DEADLINE_SECS`=150) |
| Daily free quota 1000/day (131 used on 2026-11-06 at review time) | `[M]` `zsei_data/model_calls/openrouter_quota.json` |
| Per-call latency (successful calls): nemotron-3-ultra median 14.0 s / p90 32.8 s; `openrouter/free` median 10.4 s / p90 38.2 s | `[M]` `model_ledger.jsonl` rows with `latency_ms>0` |
| The nested prompt subprocesses spawned by the text pipeline are **not** counted by the host's pipeline gate (`max_concurrent_pipelines = 10`); only the shared file limiter bounds them | `[C]` text `SubprocessExecutor` spawns the prompt binary directly (`text/main.rs:8097-8160`); gate is the host executor's (`src/pipeline/executor.rs:371-381`, `config.toml:22`) |
| Local models go through a **capacity-1** gate (`local_model_gate`) | `[C]` `src/pipeline/executor.rs:381` |
| **Little's law at the free-tier limit:** in-flight calls needed to saturate 20 rpm at 14 s/call ≈ 20×14/60 ≈ **4.7**. Concurrency above ~5 buys nothing on the free tier; on a paid/higher-limit key the ceiling moves with `OZONE_OPENROUTER_RPM` | `[I]` from the two rows above |

**Prior art already in the repo (reuse, don't reinvent):**
- `src/orchestrator/dispatch.rs:37-110` `dispatch_bounded` (semaphore width, results land at their own index, panics become loud per-item errors) + `default_width()` = 1× cores (`:114`). Used by AMT intent extraction (`amt.rs:1415`) and pairwise cross-ref (`amt.rs:2117`).
- Text grammar extraction is already **wave-parallel at 1× cores** (`text/main.rs:5969-6040`, `tokio::spawn` waves).
- AMT branch lanes run in a `JoinSet` with `CallPriority::Lane` (`amt.rs:2965`, `:3106`).
- User-tier **parallel-candidate dispatch**: up to 3 free models at once, "up to 3x quota traded for latency", hard-coded `.take(3)` (`src/orchestrator/mod.rs:4698-4721`).
- Admission gate with tiers User > Lane > Loop and a starvation guard (`src/pipeline/gate.rs`).
- Parent's `[optimization]` section already exists: `profile` + `concurrent_votes` → exported as `OZONE_OPT_PROFILE` / `OZONE_OPT_CONCURRENT_VOTES` to pipeline subprocesses (`src/config/mod.rs:85-125`, `src/lib.rs:320-321`; `OZONE_OPT_*` already set in env wins).

---

## 1. Ranked summary (gain ÷ risk) — top 12

| # | Item | Gain | Risk | Tier needed | Ref |
|---|---|---|---|---|---|
| 1 | **Stop re-reading the 29 MB `model_ledger.jsonl` + catalog on every prompt call** (`proven_free_model()` in the prompt binary; introduced this session) → cached stats file | removes a per-call whole-file read+parse in *every* pipeline subprocess | very low | all tiers (correctness-neutral) | §C1 |
| 2 | Concurrent remaining votes (`confirm_times`) — **already added**; extend the same shape to the *shared* contract fns used by the host (`confirm_consecutive_yes_with/_no_with`) | ≈2.5× per 5×-check at 14 s calls `[I]` | low | `concurrent_votes` | §A1 |
| 3 | **Speculative second check** — validators run *two* sequential 5× checks (order, then boundary/class); start the 2nd check's first vote while the 1st check's votes 2-5 run; discard if the 1st fails | halves wall time of a validated event again; costs ≤1 extra call on a 1st-check fail | medium (quota) | high-end + ample quota | §A2 |
| 4 | Per-paragraph `list_sentences_for_paragraph` and per-sentence validations are independent → run in waves (the grammar-wave pattern) | N paragraphs → ⌈N/width⌉ rounds | low | `parallel_*` | §A4 |
| 5 | Methodology rules re-read+re-parsed from disk ≥4× per walk and **per step** in compliance check → per-walk memo | removes repeated `read_to_string`+JSON parse; 7 call sites | very low | all tiers | §B2 |
| 6 | Lane-split parts (`lane_split.rs:160`) and modality-window relation inference (16 modalities) are sequential though independent → ordered concurrent merge | wall time of a split lane / a 30-node-window sweep | low (merge order kept) | `parallel_*` | §B3, §A5 |
| 7 | Cross-modal "until stable" loop: counts `link_result.is_some()` as a **new** edge, so a pass that re-links existing edges is never "stable" → up to 31 passes of subprocess spawns | up to ~5× fewer spawns once fixed; **this is a correctness/efficiency bug, not an optimization route** | low | all tiers | §B4 |
| 8 | Nested prompt calls: one-shot process + new TLS connection per call; prompt binary already has `--serve` mode | −process spawn, +connection reuse, + in-memory stats cache | medium | `nested_prompt_server` | §E1 |
| 9 | Independent provider tools in one provider step (`tool_routing.rs:377`) and independent semantic hooks per modality graph (`graphs.rs:287`) | wall time of fan-out steps | low | `parallel_*` | §D1, §B5 |
| 10 | Whole-file JSON rewrites without a lock (`tool_health.json`, `capability_gaps.json`, `methodology_gaps.json`, `methodology_reviews.json`) — lost-update risk under concurrent provider steps | correctness more than speed | low | all tiers | §C2 |
| 11 | Tiny-input fast path for Stage 2 (1 sentence ⇒ deterministic structure) | 34+ calls → 0-few for trivially single-sentence requests | **needs operator decision** (touches the "never skip" doctrine) | opt-in only | §A6 |
| 12 | Build ergonomics: fast iteration profile + serial fleet build is RAM-bound by design | host 7-15 min → minutes for edit/test loops | low | dev machines | §F |

---

## A. Validation / N-vote loops (text pipeline + shared contract)

### A1. `confirm_times_yes` / `confirm_times_no` — the precedent and its siblings
- **Current:** `text/main.rs:2692-2724`: first vote gates; with `OZONE_OPT_CONCURRENT_VOTES=1` the remaining 4 run via `tokio::join!` (fixed `rest == 4` only; any other `n` falls to the sequential loop). Call sites (19 `confirm_*` uses, all `n=5`): sentence order `:2879`, sentence correction `:2903`, extraction-complete `:3030` (`confirm_times_no`), section `:3309/:3334`, paragraph `:3583/:3601`, modality `:3843/:3866`, structure `:5315`.
- **Host-side duplicates the shape sequentially:** shared `shared/contracts/k_validation.rs:55-70` `confirm_consecutive_yes_with` / `_no_with` (`for _ in 0..strength { ask().await }`), used by `confirm_yes_no_orch` (`src/orchestrator/mod.rs:4894-4960`) at `amt.rs:870`. Two implementations of one rule (text pipeline's own `confirm_times` does not call the shared fn).
- **Proposed optimized route:** put the gate-then-concurrent logic **once, in the shared contract** (policy field `concurrent_rest: bool`, `max_parallel`), make both the text pipeline and the host call it. Generalise beyond `rest==4` (`join_all`-style over a bounded set; the text crate has no `futures` dependency today `[C]` Cargo.toml — `tokio::task::JoinSet` needs `'static`, so either add `futures` to the crates or use a fixed macro for 1-4).
- **Must stay identical:** all `n` answers equal `want`; first non-match ⇒ false; blank/unparseable = non-affirming (`parse_yes_no` returns `None`).
- **Gain:** one 5× check 70 s → ~28 s at 14 s/call `[I]`. **Risk:** quota burst (≤3 extra calls on a late fail); limiter waits (see §5).
- **Statistical caveat (flag, not a blocker):** votes are issued at `temperature: 0.0` (`k_validation::yes_no_input`, `:29-36`). They were *independent samples* only because `openrouter/free` rotates members. With proven-in-slot picking **one** model, five identical-prompt votes become near-repeats of one opinion — decision rule intact, evidential value lower. Worth a deliberate choice (e.g. in the optimized route vote across ≥2 distinct models) before turning proven-in-slot and concurrent votes on together.

### A2. Dual validation: two sequential 5× checks per event
- **Current:** `validate_section_event` (`:3280`), `validate_paragraph` (`:3555-3605`, order 5× then boundary 5×), `validate_modality` (`:3817-3866`), sentence order + correction (`:2845`, `:2888`). A fully validated event = up to **10 sequential calls**; the check-2 prompt does not depend on check-1's *votes*, only on its pass/fail.
- **Proposed route (`speculative_second_check`):** after check-1's gating vote passes, fire check-2's gating vote concurrently with check-1's remaining votes; if check-1 fails, drop check-2's result.
- **Identical:** event accepted ⇔ both checks 5/5. **Cost:** ≤1 wasted call per failed check-1 (+≤4 if check-2's rest were also launched — don't). **Needs:** ample quota; counts against 20 rpm. **Gain:** per event ~140 s sequential → ~56 s (concurrent votes) → ~42 s (speculative) `[I]`; floor under the free limiter is 10 calls ÷ 20 rpm ≈ 30 s.

### A3. Exhaustion tracker = 2 wasted calls per loop
- **Current:** `shared/contracts/k_loops.rs:14-45`: an ordered 1×1 loop ends only after **2 consecutive misses** (`exhaustion_strikes: 2`). Every section / paragraph / sentence / modality loop therefore ends with 2 "found nothing" extraction calls (`text/main.rs:3423, 3620, 3883`).
- **Observed:** `[M]` for a 59-char request the log shows 18 "detect PARAGRAPH" + 14 "validate paragraph" + 2 "track SECTION" calls in the first 29 minutes before the 1800 s watchdog (`pipeline_zero_shot_calls.jsonl`).
- **Proposed route:** none that keeps the doctrine safely except overlapping the second strike with downstream work (the 2nd miss call can run while the previous event's validations are finishing). Low gain, medium complexity — **recorded, not recommended first**.

### A4. Independent per-unit work inside one chunk
- **Current (Path 1):** after paragraphs, `for (para_id, s, e) in listing_targets { list_sentences_for_paragraph(...).await }` — sequential (`text/main.rs:5488-5494`); `clean_chunk` is a separate call (`:4150`); modality detection + `ModalityCarryState` follow. Carry states (`section_carry`, `paragraph_carry`, `sentence_carry`, `modality_carry`) are separate structs `[C]` `:4780-4830`.
- **Cross-chunk is inherently serial** (carries thread through `for chunk in &chunks`, `:4787`) — **not** an optimization target.
- **Proposed route (`parallel_chunk_subloops`):** within one chunk run (a) per-paragraph sentence listing in waves, (b) embedded-modality detection concurrently with sentence listing when their carries are independent (verify per path), using the existing wave width (`available_parallelism`, `:5994`). **Identical:** same prompts, same merge order (collect into index slots like `dispatch_bounded`). **Risk:** low-medium (shared `llm_calls` counters are atomics `[C]`).

### A5. Relation-inference windows (16 sensor modalities)
- **Current:** `for (a, b) in semantic_relations::windows(nodes.len(), 30)` awaiting `infer_semantic_relationships_window` one by one — depth `:986`, haptic `:1210`, kinematics `:1147`, CAD `:1088`, radar `:1145`, network `:1099`, thermal `:967`, electromagnetic `:951`, biology `:1252`, 3D `:1541`, sonar `:1001`, IMU `:1337`, BCI `:1183`, proteomics `:1342`, hyperspectral `:1090`, sound `:1103`. Windows overlap 50% (`shared/semantic_relations.rs:318-334`, stride `w/2`) so every node is shown ~twice (the coverage rule, keep).
- **Each window's call is a blocking `std::process::Command::...output()` inside an `async fn`** (`IMU/main.rs:908-914` `llm_zero_shot`) — it holds a runtime worker thread for the whole model call; concurrency would need `spawn_blocking`.
- **Merge is order-dependent** (first `(from,to)` pair wins, `IMU/main.rs:1339-1343`) → collect windows concurrently, **merge in window order** → identical output.
- **Gain:** a 301-node graph = 20 windows (computed from `windows(301,30)` with stride 15) → ≈20×14 s sequential `[I]` → ⌈20/width⌉×14 s. **Risk:** low (merge order kept); quota: 19 calls at once ⇒ limiter shapes it. **Config:** `parallel_windows`.
- Code, image, audio, video, chemistry, DNA, EEG show **no** model-call sites for these patterns (grep for `llm_zero_shot(|execute(9|prompt_pipeline`: count 0 or detached `tokio::spawn` node-events only) `[C]` — nothing to optimise there; math has one citation call (`math/main.rs:1960`).

### A6. Stage 2 for tiny inputs (needs an operator decision)
- **Observation:** for a one-sentence request every detect call returned `{"found":true,"order":1,"start":0,"end":59,…}` `[M]` and the validators then rejected/re-proposed. A deterministic derivation (no blank line, no heading marker, one terminal punctuation ⇒ 0 sections, 1 paragraph `0..len`, 1 sentence) would produce the same graph for these inputs.
- **Why it needs a decision:** operator doctrine "never skip / chunk, don't refuse" and the strength-N validation are the design; a fast path is a *derivation*, not a skip, but it removes model validation of structure. Proposed only as `fast_path_tiny_input` (default **off**), with an equivalence test replaying logged chunks (`pipeline_zero_shot_calls.jsonl`) against the model-derived result before it may be enabled.
- **Gain:** ≥34 calls/≈29 min `[M]` → 0 structural calls for qualifying input; also removes the Stage-2 watchdog exposure that currently blocks every walk-based E2E.

### A7. Yes/no prompt quality is a latency lever (found while reading)
- `yes_no_input` sends `"max_tokens": 20` and the strict instruction under the key **`system_context`** (`shared/contracts/k_validation.rs:29-36`). The prompt binary's `PromptInput` has no such field and no alias `[C]` (`grep system_context` in `prompt/main.rs`: no hit); the host already documents this exact key as dead ("was system_context, a key PromptInput never read — its real field is system_prompt", `src/orchestrator/mod.rs:4328-4331`). `max_tokens` is deliberately inert (no-cap doctrine; `effective_api_cap`). Net: every Stage-2 yes/no vote reaches the model **without** its "answer with one JSON object, no explanation" instruction and without a cap ⇒ reasoning models spend 10-30 s on a one-word answer `[I]` (observed yes/no votes with `tokens_used` 1,000-12,000 `[M]`).
- **Not an optimization route** (it's a bug in the shared contract) but it is the cheapest latency/quota win on this page; listed for decision, not fixed.

---

## B. Orchestrator stages

### B1. Repeated disk/parse work
| Where | What | Evidence |
|---|---|---|
| prompt binary `proven_free_model()` | `read_to_string` of `model_catalog.json` (66 KB) **and the whole `model_ledger.jsonl` (29,202,756 bytes, 5,735 parseable rows)** on every model call in every pipeline subprocess; a Python parse of the file takes 0.79 s `[M]` (Rust is faster, not measured) | `[C]` `prompt/main.rs` (added this session) |
| host `model_ledger` | `load_file()` at boot, `cause_summary_today()` re-reads the whole file per call (`model_ledger.rs:604`, served by `grpc/mod.rs:2497`), `reclassify_historical_http_errors()` reads+rewrites it (`:253`) | `[C]` |
| ledger rows embed **full prompt and response text** (max row 166,319 bytes) | most of the file is prompt/response text rather than the few numeric fields routing statistics need `[I]` (29 MB for ~5.7k rows ≈ 5 KB/row on average, max row 166,319 bytes) | `[M]` |

**Proposed route (all tiers, correctness-neutral):** keep the raw ledger as the append-only audit log; add a small **derived stats file** `model_calls/model_stats.json` (per model: attempts, successes, empties, http/timeouts, latency reservoir/median, last_seen, free-flag) updated by the host on each `record_*` and by the prompt binary read-only. The prompt binary then reads ~10 KB instead of 29 MB. Rebuildable from the ledger (a repair command) so no data is lost. Gain `[I]`: removes tens-to-hundreds of ms and ~29 MB of reads from every call; scales with ledger growth (which has no retention policy — see REVIEW_ALIGNMENT C5).

### B2. Methodology rules loaded from disk repeatedly
- **Current:** `load_methodology_rules_full(&container)` reads `data_dir/<object_store_path>` and parses JSON on each call (`amt.rs:2566-2600`); 7 call sites; per walk it runs in Stage 3/blueprint (`stages.rs:885`), simulation (`:1639`), step-render (`:2101`) and **once per step** in the compliance check (`:3309`), plus `amt_loop.rs:678`, `tool_binding.rs:59`.
- **Note:** `store.get_container` itself is an in-memory cache hit (`zsei/storage.rs:228-263` `local_cache`), so those loops are cheap; the *content file* is the cost.
- **Proposed route (always-on, keyed memo):** `(container_id, global_state.version, object_store_path)` → rendered rules string, held in `OrchestrationState`. Invalidation is free because `Update` now writes a new versioned path (this session's pipeline-12 fix). **Identical output.** Gain: removes N×(steps+4) file reads/parses per walk `[I]`.

### B3. Lane split parts are sequential but independent
- **Current:** `lane_split.rs:160-171` `for part in &parts { executor.execute(...).await }` then merges in member order (`:174-194`).
- **Proposed (`parallel_lane_parts`):** run parts concurrently (`dispatch_bounded`), merge in part order. Identical merge; parts only exist when one candidate model's window is too small ⇒ local/small-window models: **keep the sequential route there** (capacity-1 local gate would serialise anyway).

### B4. Cross-modal "until stable": probable correctness bug that multiplies spawns
- **Current:** `graphs.rs:324-352` loops passes until `new_edges == 0` for 5 consecutive passes (`:337`; ceiling `pass > 30` at `:344`). `run_cross_modal_reference_pass` counts a new edge whenever `result.get("link_result").is_some()` (`:388`, `:415`, `:450`). But `link_handler::link_result_json` returns `link_result` **even when nothing was added** — it reports `forward_added`/`reverse_added` booleans (`shared/link_handler.rs:135-155`) and the module doc says the handler is idempotent precisely because "the PASS-3 until-stable loop calls every pass". So a successful pass never reports 0 new edges ⇒ the loop cannot stabilise and runs to the ceiling `[C]` — up to 31 passes × (N text→other + back-refs + inter-modality) subprocess spawns.
- **Observed:** `[M]` in the earlier live walk the text→3D link failed every pass with `unknown variant 'ReferencesModality', expected one of DescribesCode, …` (10 occurrences in `host.log`), the stage reported `cross-modal: 0`, i.e. the loop ran the minimum 6 passes of failing spawns. (Orchestrator sends `"relationship":"ReferencesModality"`; the text pipeline's enum does not contain it.) Out-of-scope for this review beyond noting it.
- **Fix class:** count `forward_added || reverse_added`; independent of any config. **Optimization route on top:** run the pass's link calls concurrently (they are independent container writes with idempotent check-then-write; guard with per-container ordering or keep sequential on `local`).

### B5. Other independent sequential awaits (orchestrator)
- `graphs.rs:287-299` semantic hook per other-modality graph (`let _ = ...execute(...).await` — also swallows errors).
- `stages.rs:2690` sub-queries to pipeline 56 sequential; `stages.rs:3020` sub-steps — sub-steps are ordered by design (feed results forward) → **not** a target; sub-queries are independent (search each) → candidate.
- `stages.rs:3354` compliance check per step is one model call per step, sequential by walk order; independent of later steps' execution only for *future* steps — ordering is the doctrine; record only.
- `meta_loop.rs:331/437` reviews and gaps are processed one by one inside a pass (bounded 12 per pass) — they are independent model calls; Loop-tier work, quota-sensitive ⇒ **keep sequential by default**, optional `parallel_meta_reviews`.
- `decision_review.rs:447` retry loop is intentional backoff — leave.
- `amt.rs:3351/3449/3921` keyword/modality/domain `search_by_keywords` loops are store lookups (cheap, in-memory) — low value.

### B6. Prompt-prefix / caching opportunities `[I]`
- The Stage-2 validators resend the full chunk text (`CHUNK TEXT: …`) in each of the 5 votes of a check; OpenRouter/provider prompt caching rewards a stable prefix, but the binary sends one user message with the question appended after the chunk ⇒ prefix is already stable for identical votes. The bigger lever is that 5 identical-prompt votes could share one cached prefix. Needs provider support check; **not measured** — recorded for review.
- Batching *different* independent yes/no questions into one call returning N verdicts is semantically safe **only for different questions** (e.g. the N per-sentence checks of one chunk). Batching the 5 votes of one question into one call is **not** equivalent (correlated, one sample) — do not do it.

---

## C. Storage

### C1. `model_ledger.jsonl` write path and corruption — measured
- **Writer:** one `write_all` of `row+"\n"` on an `O_APPEND` handle (`model_ledger.rs:385-415`, comment: one call so processes cannot interleave).
- **State `[M]`:** 5,797 lines, 63 unparseable lines (32 non-empty rows of 2-125 KiB, the rest blank), one **2,359-byte NUL-filled line at line 2866**, max row 166,319 bytes. NUL runs are the classic signature of a kill/crash/power event on an extended file; truncated big rows are consistent with interleaving or partial writes — **cause not established by this review**.
- **Proposed route (all tiers):** (a) rows keep a bounded `prompt_preview`/`response_preview` and put full text in `capture/` (it already has `call_spans.jsonl` 1.3 MB, `context_records.jsonl` 1.4 MB); (b) single-writer append via the host (pipelines send rows over the existing local channel) or a lock around append; (c) startup repair that rewrites a clean file + moves unreadable bytes to `model_ledger.bad`, keeping every byte (nothing deleted). (d) `served_model` and the OpenRouter generation id on every row so router failures can be attributed (REVIEW_ALIGNMENT E3).

### C2. Whole-file JSON read-modify-write, no lock
| File | Writer | Evidence |
|---|---|---|
| `tool_health.json` | `record_tool_outcome`: read whole → push → `fs::write` | `tool_binding.rs:342-355` |
| `capability_gaps.json` | read/modify/write | `tool_binding.rs:211` path; recorder |
| `methodology_gaps.json` / `methodology_reviews.json` | `record_methodology_gap_ex` / `record_methodology_review` + meta_loop rewrites the whole array at end of pass | `amt.rs` (this session), `meta_loop.rs` |
- Concurrent provider steps or concurrent meta passes can lose updates `[C]` (no lock, non-atomic `fs::write`). Files are small now (15 KB gaps) so speed is a non-issue; the optimization is *safety under the parallelism the optimized routes introduce*. **Proposed:** atomic write (temp+rename, already used in the ledger reclassify) + a process-wide mutex per file; or move to append-only JSONL with a compaction step. Must precede `parallel_provider_tools` / `parallel_meta_reviews`.

### C3. ZSEI container store
- In-memory `local_cache` + `child_ids_cache` with a background local-write queue (`zsei/storage.rs:28-87`, `LOCAL_WRITE_MAX_PENDING = 10_000`). The container reads in orchestrator loops are therefore cheap. Not deep-reviewed here: mmap global path, `chat_store`, graph-store write amplification — **recorded as not verified**.

### C4. Append-only stores without retention (size `[M]`)
`model_ledger.jsonl` 29.2 MB · `zero_shot_calls.jsonl` 2.6 MB · `pipeline_zero_shot_calls.jsonl` 2.6 MB (5,013 rows, all `pipeline:"text"`) · `tool_calls.jsonl` 146 KB · `capture/call_spans.jsonl` 1.3 MB / 4,152 rows · whole `zsei_data` 167 MB (`model_calls` 34 MB). Compaction/archival design is missing (no-cap doctrine ⇒ summarise + keep raw archive, never delete).

---

## D. Provider / MCP paths

### D1. Independent provider tools in one provider step
- `tool_routing.rs:377` `for tool in chosen { call_global(...).await }` — tools chosen by `choose_provider_tools` (best-matching, ties keep all). Independent calls ⇒ `parallel_provider_tools` (bounded, results kept in `chosen` order). Gate/jurisdiction/metering are per call inside `call_global` so they are unchanged. **Prerequisite:** C2 (health ledger lock).
- `tool_routing.rs:207` legacy search-routing loop; same treatment, lower value.

### D2. Registry/health re-reads
- `mcp::registry_entries()` is an in-memory registry list (`src/mcp.rs:121-129`) — fine. `tool_binding::read_health()` re-reads `tool_health.json` on each `degraded()`/`record` call (`:342-363`) — small file; memoise in-process after C2.

---

## E. Rate-limit / quota awareness (how the optimized routes must behave)

1. **Shared limiter (20 rpm default):** concurrency is shaped, not bypassed — each call acquires a slot in the lock-file bucket and sleeps `min(wait,5 s)` until one frees (`openrouter_limiter_acquire`). Concurrent votes therefore queue at the limiter; throughput stays ≤20/min. **Implication:** on the free tier `concurrent_votes` mostly removes *round-trip serialisation* (latency per call), not the rpm ceiling; with a higher-limit key set `OZONE_OPENROUTER_RPM` accordingly.
2. **150 s deadline:** a burst that waits >150 s "proceeds; the host 429 handling decides" (`:~530`) — a large `parallel_windows` (20+ calls) can trip this and turn into 429s. **Rule for optimized routes:** cap in-flight calls at `min(cores, ceil(rpm × median_latency / 60) + 1, max_parallel_calls)`; never queue more calls than ~`rpm × 2` minutes of limiter budget.
3. **Daily quota (1000/day on this key):** the host reads it (`src/openrouter_quota.rs`, `openrouter_quota.json`: latest value only, no history); pipelines never read it. A walk costs ≥34 Stage-2 calls for one sentence `[M]`; **optimized routes must not increase calls on a pass** (they don't) and must stop launching speculative calls when `remaining < reserve` (`respect_quota`, `min_daily_quota_remaining`).
4. **No host-gate coverage for nested calls (§0):** the 10-slot `max_concurrent_pipelines` gate does not see calls spawned by modality pipelines. Memory is the other bound: a `MemAvailable` gate already refuses launches when low (`src/lib.rs` `mem_available_mb`); the optimized profile should read the same signal (≈2 GB available on this box ⇒ **this box is not a `high_end` machine**).
5. **Local models:** `local_model_gate` capacity 1 and 4-core CPU inference ⇒ every optimized route must be off (sequential) when the active model `model_type != "api"` — `profile = "high_end"` must still respect this per call.

---

## F. Build / startup / test ergonomics

| Item | Evidence | Proposal |
|---|---|---|
| Host release build 7-15 min (`7m 30s`, `13m 09s`, `7m 08s` measured today; variance = machine load) vs `cargo check` ~1-2 min | `[M]` `build*.log` | Add an iteration profile (e.g. `[profile.fast]` inherits release, `opt-level=1`, `lto=off`, `incremental=true`, `debug=0`) for edit/E2E loops; keep `release`/`dist` for shipping. Already using lld (`.cargo/config.toml`) `[C]`; no sccache/mold installed `[M]` (`which`). |
| Pipeline fleet is built serially with `CARGO_BUILD_JOBS=1` into one shared target dir (`target/pipelines`, 2.2 GB, 136 files) | `[C]` `scripts/build-pipelines.sh:6-8` | Serial is RAM-justified on a 7 GB box; make jobs configurable (`BUILD_JOBS`, default 1) and add "only crates whose sources changed" (mtime vs binary) — today it re-invokes cargo for every crate (each a no-op when current). |
| `scripts/build-pipelines.sh` is **not executable** (`-rw-rw-r--`) | `[M]` `ls -l` | `chmod +x` (my first `nohup scripts/build-pipelines.sh` failed with "Permission denied"). |
| Crates that were never built (`sync`, `auth`) sat broken because the script only builds crates that already have a binary | `[C]` script lines 17-25 | L1 gate: `cargo check` every crate (separately from the "build what shipped" step). Decision on unbuilt pipelines: **no action** per operator (REVIEW_ALIGNMENT H4); this row is about the gate only. |
| Walk-dependent E2Es share one 20 rpm / 1000-day budget | `[M]` | Run walk-based suites **serially**, non-walk suites (modalities, mcp-contract, jurisdiction, marketplace, fanout) in parallel — they don't touch the model limiter. Add a walk-readiness preflight (probe quota + a 3-call latency sample) before any walk-based suite. |

---

## G. Proposed config schema (extends the existing `[optimization]`)

Existing (parent's edit, `src/config/mod.rs:85-125`): `profile = "standard"|"high_end"`, `concurrent_votes: Option<bool>` (profile default when unset). Proposed additions — **all default off under `standard`; `high_end` flips the safe ones on; individual keys override the profile; env `OZONE_OPT_*` already wins over config**:

```toml
[optimization]
profile            = "standard"   # "standard" | "high_end"  (high_end = every key below that is marked [hi] defaults true)
concurrent_votes   = false        # [hi] first vote gates, rest concurrent  (EXISTS)
speculative_second_check = false  # [hi, needs quota] start check-2's gating vote while check-1's remaining votes run
parallel_chunk_subloops  = false  # [hi] per-paragraph sentence listing + modality detection in waves (within one chunk only)
parallel_windows         = false  # [hi] relation-inference windows concurrently, merge in window order (16 sensor modalities)
parallel_lane_parts      = false  # [hi, api models only] lane-split parts concurrently
parallel_cross_modal     = false  # [hi] link calls of one pass concurrently (after the counting fix)
parallel_provider_tools  = false  # [hi] independent tools of one provider step concurrently (needs locked health ledger)
parallel_meta_reviews    = false  # reviews/gaps in one meta pass concurrently (Loop tier; quota-sensitive)
max_parallel_calls       = "auto" # "auto" = min(cores, ceil(rpm*median_latency/60)+1) | integer; hard per-process in-flight bound
respect_quota            = true   # stop launching speculative/extra calls when daily remaining < reserve
min_daily_quota_remaining = 100   # reserve for respect_quota
local_models             = "sequential"  # "sequential" (default, forced when model_type != api) — documented, not togglable to concurrent
# correctness-neutral, ON in every profile (kept as keys so they can be switched off to debug):
methodology_rules_cache  = true   # per-walk memo of rendered rules (B2)
ledger_stats_file        = true   # derived model_stats.json for the prompt binary (B1)
# opt-in, needs operator decision:
fast_path_tiny_input     = false  # deterministic Stage-2 structure for trivially single-sentence input (A6) — with equivalence test
nested_prompt_server     = false  # long-lived prompt worker via --serve for nested callers (E1 below)
vote_diversity           = 1      # >=2: spread a vote check across that many distinct models (restores independence when proven-in-slot is on)

[logging]                          # see §H
host_log           = "zsei_data/logs/host.log"
rotate_mib         = 256
retention_files    = 0            # 0 = keep all (no-cap doctrine)
pipeline_stderr    = "tee"        # "off" | "tee": persist nested pipeline stderr to logs/pipelines/<name>.log
```
Wiring: `OptimizationConfig::to_pipeline_env()` (`config/mod.rs:118-125`) already exports `OZONE_OPT_*`; each new key = one more env var read in the pipeline that owns the route. The prompt binary's own `OZONE_PROVEN_FIRST` (added this session) belongs in the same section as `proven_first = true` (it is routing, not scheduling — keep both knobs documented together).

### E1. Nested prompt calls as a long-lived worker
`ozone_serve.rs` gives every pipeline a `--serve` mode (std-only HTTP server, optional host registration, heartbeat). The prompt binary supports it (`prompt/main.rs:280` "concurrent serve-mode requests"). Nested callers (text `SubprocessExecutor`, modality `llm_zero_shot`) always spawn a fresh process per call `[C]`. A shared worker would give connection reuse (no TLS handshake per call), an in-memory ledger-stats cache (replaces B1), and one place to hold the limiter. Risks: lifecycle/supervision, a single point of failure for all nested calls, trace/priority task-locals (`gate.rs` notes they don't cross processes). **Medium risk — recorded as an option, after B1.**

---

## H. HOST LOG review

### H1. Is host logging persistent? — as found
- **At review time the running host (pid started 2026-11-05 22:20:54 local) has no file logger.** `init_logging_with_level` wrote only to stdout (git HEAD; `src/lib.rs` tracing_subscriber `fmt::layer()` with default writer). `zsei_data/logs/` contains only the MCP supervisor's per-server logs (`mcp-*.log`, `agent-mcp.log`, `android-mcp.log`, `ui-harness.log`; 68 KB total) — **no `host.log`** `[M]`.
- The host logs we have been reading this session exist **only because the launcher redirected stdout** (`.../jobs/608d9ac1/tmp/host*.log`). A host started any other way (terminal, service, `setsid` without redirect) loses its log when the terminal/pipe goes away.
- **There is an uncommitted edit in `src/lib.rs` (mtime 23:24:50, after the running binary's start) adding a `PersistentLogWriter`**: tees every line to stdout *and* `OZONE_HOST_LOG` | `<OZONE_ZSEI_DATA_DIR>/logs/host.log` | `zsei_data/logs/host.log`, size-rotates at start (>256 MiB renamed with a timestamp, never truncated), best-effort. **It is not in the running process** (no `host.log` exists yet) — it takes effect on the next build+restart. Review notes on that edit: (1) rotation happens only at process start, so a long-running host can exceed 256 MiB; (2) one `Mutex<File>` shared by all log lines — write errors are swallowed (`let _ = f.write_all`), fine for logging but invisible; (3) no timestamps-per-file/daily split and no retention knob (the proposed `[logging]` keys cover it); (4) ANSI is disabled only when stdout is not a TTY, so the file stays plain.

### H2. What a meta/observability layer cannot see today
| Signal | State | Evidence |
|---|---|---|
| Nested-prompt-binary stderr **on success** (limiter waits, "router served non-chat — re-calling", "proven model failed — falling back", `empty response … served=…`) | **discarded.** The text `SubprocessExecutor` calls `.output()` and prints the child's stderr **only when the exit is non-zero** (`text/main.rs:8130-8150`); modality `llm_zero_shot` uses `.output()` and never reads stderr (`IMU/main.rs:908-914`, same pattern across the 16 sensor modalities) | `[C]` — the host *does* tee a pipeline's own stderr to tracing live (`executor.rs:224-262`), but the nested child's stderr is a level below that |
| Limiter state | one 101-byte file `openrouter_bucket.json` holding the current stamps; waits only appear as an eprintln line | `[M]` + `[C]` |
| Quota history | `openrouter_quota.json` is overwritten (latest only) — no trend, no per-walk spend | `[M]` |
| Router failures by member | ledger rows for failures carry the *requested* id (`openrouter/free`), not the member that failed; ~49% of `openrouter/free` calls are therefore unattributable | `[M]` ledger counts (593 ok / 384 empty / 215 other / 26 http / 10 timeout), `[C]` `record_context_and_ledger` records under `model_used` only when present (`mod.rs:3863-3900`) |
| Call site of 57% of calls | `walk:pipeline-9` | `[M]` ledger (3,310 of 5.7k) |
| Watchdog / walk-failed events | only tracing lines (`WATCHDOG: pipeline invocation exceeded its budget`, `walk failed before task creation — PAUSED`) — persistent once H1's file exists; **not** graph events | `[C]`/`[M]` host4.log |
| Pipeline-level spans | `capture/call_spans.jsonl` (4,152 rows, persisted) and `capture/context_records.jsonl` | `[M]` — good, but not linked to any graph container |
| Host log itself | see H1 | |

### H3. Linking logs into the graph so meta + UI can watch live — DESIGN ONLY
**Facts the design builds on:**
- `file_beacon.rs` polls `FileReference` containers (found by walking users 0/1 → workspaces → projects → child ids) every **45 s**, `stat`s mtime/size, and fires one `UpdateContainer` on change; the cache is in-process memory (`file_beacon.rs:1-75`, `POLL_INTERVAL_SECS = 45`).
- `graph_events.rs` `GraphEventHub::global()` is a broadcast ring (1024) fed by every container write; `ripple_class::classify` maps `container_type` → `RippleCategory` + `meta: bool` and has echo guards; the meta/I-loop/assistant wake on `meta == true` events through `LoopGate` (min-gap + heartbeat) (`ripple_class.rs:25-96`).

**Proposed design (to review, not built):**
1. **A `Logs` project in the global workspace** ("Ozone-Studio" global scope) holding one `FileReference` container per log source: `host.log`, each `mcp-*.log`, `model_calls/model_ledger.jsonl`, `capture/call_spans.jsonl`, `openrouter_quota.json`/`openrouter_bucket.json`, `methodology_gaps.json`, `methodology_reviews.json`, `capability_gaps.json`, `tool_health.json`, plus new `logs/pipelines/<name>.log` (the persisted nested stderr, H2) and `logs/limiter.jsonl` / `logs/quota.jsonl` (append-only histories). Created idempotently at boot by a registrar (same pattern as the other boot-time containers).
2. **A dedicated, tail-aware beacon (`LogBeacon`) instead of the 45 s mtime poll** for these containers: 1-2 s tick (or `inotify` on Linux), keeps a byte **offset** per file, reads only the *new* bytes (never the whole file — the ledger is 29 MB), handles rotation (inode/size shrink ⇒ reset), and is the only code that touches log content. The generic `file_beacon` stays for user files.
3. **Events, not content, go on the ripple.** Per file per tick, at most one `GraphEvent{event:"updated", container_type:"LogStream", source:"log-beacon", scope_keywords:["scope:global","log:<name>"]}` carrying `{offset_from, offset_to, new_error_count, new_warn_count}` in the container's metadata — the log text stays on disk. Content is chunked to a reading model's window on demand when a loop actually reads it (no-cap doctrine).
4. **New ripple category `Observability`** in `ripple_class::classify` (`LogStream` ⇒ `RippleCategory::Observability`, `meta = false` by default). Meta wakes only on **derived findings**, the same way real tool findings become `Finding` events: a small actor (`actors.rs` pattern) counts per-window error/watchdog/429/empty rates and emits a `Finding` (`meta = true`) when a threshold is crossed (e.g. watchdog event, ≥N consecutive unusable prompt attempts, quota < reserve, limiter wait > deadline). This keeps the echo-guard property (log growth caused by the loops' own work must not wake the loops — `LoopInternal` for the beacon's own source).
5. **Live view:** the existing SSE/ws event surface that feeds the UI graph (`orchestration-events`, `node-event` posts already used by modalities) gets the `LogStream` updates; a "Logs" panel tails by offset (`GET /logs/<name>?from=<offset>` returning bytes + next offset). Meta loops read the same endpoint — one reader contract for UI and loops.
6. **Prerequisites:** H1's file logger live; nested stderr persisted (add a `pipeline_stderr = "tee"` writer around both `SubprocessExecutor` and `llm_zero_shot`); ledger gets `served_model`/generation id; limiter + quota histories appended (small JSONL).
7. **Risks:** log volume feedback (the beacon must exclude its own container writes from classification — `source == "log-beacon"` ⇒ `LoopInternal`); cost of tailing 29 MB ledger (offset-only); privacy (ledger rows embed full prompts — gate the UI endpoint with the existing auth; consider moving payloads out of the ledger, §C1); `KNOWN_USER_IDS = [0, 1]` enumeration limit of the current beacon (a registrar-created container list avoids the workspace walk).

---

## I. Observed while reviewing (outside this review's scope — recorded, not acted on)
1. `shared/contracts/k_validation.rs:29-36` sends the strict instruction under `system_context` (ignored by the prompt binary) — §A7.
2. Orchestrator sends cross-modal `relationship:"ReferencesModality"`, rejected by the text pipeline's link enum (`unknown variant … expected DescribesCode, DescribesImage, DescribesAudio, DescribesVideo, TranscribedFrom, IllustratedBy, Custom`) — `[M]` host.log; and the 3D pipeline's `CreateGraph` errored `Parse error: missing field 'type'` in the same walk. Both make Stage 4b "cross-modal: 0".
3. `pairwise` cap `max_pairs: 50` and `forward_window: 8` (`shared/contracts/k_loops.rs:57-67`) are hard caps in a codebase with a no-cap doctrine — flagged for the operator, untouched.
4. Parent's in-flight edits seen during the review (`config/mod.rs` `[optimization]`, `lib.rs` persistent log writer, `text/main.rs` OZONE_OPT gating, `prompt/main.rs` proven-first) were read as they were on disk at review time; none were modified by this fork.

## J. Suggested order if approved (not started)
1. §B1 derived `model_stats.json` (removes a per-call cost introduced this session) → 2. §B2 rules memo → 3. §H1 file logger live + §H2 nested stderr tee + ledger `served_model` → 4. §C2 locks/atomic writes → 5. §B4 link-counting fix → 6. §A7 `system_context` key decision → 7. shared-contract concurrent votes (§A1) → 8. `parallel_windows` / `parallel_chunk_subloops` / `parallel_lane_parts` behind `[optimization]` keys → 9. `speculative_second_check` (needs quota policy) → 10. §A6 fast path only after the operator decides and an equivalence test exists → 11. §H3 LogBeacon + `Observability` ripple.
