# Efficiency Guide — making this system fast without degrading it

> Companion to `docs/ZERO_SHOT_EXPANSION_GUIDE.md` (the mechanical-vs-judgment
> doctrine) and `docs/STATIC_LOGIC_REGISTRY.md` (the mechanical-logic side).
> This doc is the third leg: given a call site or a code path, how do you
> make it genuinely faster — on any hardware, for any user — without
> trading away correctness to get there. Built 2026-10-08 per direct
> operator request, after a live, severe correctness bug was found in a
> same-day performance refactor (§1 below) — the doc exists specifically
> because "faster" and "correct" are not automatically compatible, and this
> session proved it the hard way once already.

---

## 1. Case study: a concurrency refactor that compiled clean and shipped a 100%-waste bug

`extract_grammar_from_graphs` (`assets/pipelines/modalities/text/main.rs`)
was refactored same-day to dispatch its per-sentence grammar extraction in
parallel waves instead of one call at a time. The refactor compiled clean,
passed `cargo check` three times across its own debugging process, and was
deployed. It was still wrong: an id-stabilization step that used to run
*first*, by construction of the original single sequential loop, got left
in its *old relative position* when the loop was split into "dispatch
phase" + "apply phase" — which was now *after* the dispatch phase instead
of before it. The dispatch phase keyed its results map by the pre-
stabilization id; the apply phase looked results up by the post-
stabilization id. Every lookup missed. Every real LLM call's result was
silently discarded — real tokens, real time, zero effect, live in
production for the gap between that deploy and this doc being written.

**The lesson, generalized**: splitting one sequential piece of code into a
"compute everything" phase and an "apply everything" phase is exactly the
kind of change that silently breaks an *implicit* ordering dependency
(a stabilization step, a side effect, a mutation another step relied on
having already happened) that the original single-pass loop satisfied for
free, just by executing top-to-bottom. **Compiling is necessary, not
sufficient.** After any refactor that reorders or parallelizes
previously-sequential steps, explicitly ask: *what did step N assume had
already happened to the data by the time it ran, and does that still hold
in the new order?* — not just "does this type-check." The fix here was
mechanical once found (move the stabilization earlier), but finding it
required reading the actual data flow, not trusting that a clean compile
plus a passing type system meant the refactor preserved behavior.

---

## 2. The operator's question: does the number of zero-shot calls matter, if each one prevents a more expensive failure?

**Mostly no, with one real, concrete exception: local models without
keep-warm.**

The core intuition is sound and this session has already built one real
example of it: E8's confetti adjudication spends one small, cheap call
(`max_tokens: 200`, ~sub-second to a few seconds on a cloud API) to avoid a
full blind regeneration (another complete call at full `max_tokens`, full
latency, plus the risk it produces confetti *again*). If the adjudication
call succeeds, you've paid a small cost to avoid a large one. If it fails,
you fall back to the original behavior — no worse than before. That's a
genuinely good trade, and the doctrine should keep applying it wherever a
cheap, bounded call can reduce the probability of hitting an expensive
retry/fallback path.

**The exception that matters**: on a local model invoked without a
keep-warm server (`assets/pipelines/general/prompt/main.rs`'s own documented
measurement — "the per-call `llama-cli` spawn pays ~25-40s model load EVERY
call"), this trade inverts. The cold-start cost is paid **per call,
regardless of call size** — a 200-token adjudication call costs the same
25-40s spawn tax as a full 800-token generation. Adding a cheap call here
doesn't avoid an expensive one; it **adds a second full-price call on top
of the first**. The "number of calls doesn't matter" framing holds for
cloud/API-backed calls (OpenRouter, Anthropic — marginal latency per call
is genuinely small), and does **not** hold for local models until
keep-warm (the already-identified, not-yet-built D3 subsystem) removes the
per-call spawn tax.

**Concrete implication for E8's judge selection** (`run_one_candidate`,
`src/orchestrator/mod.rs`): the current design picks "the first other
candidate in the fallback pool" as judge, with no special-casing for
local-vs-API. This is *not* a bug to fix with a hand-tuned "always prefer
non-local" heuristic (that was correctly declined earlier this session —
the real root fix is proper modality/capability categorization in
settings, and a hand-tuned judge-selection rule would paper over that). But
it IS worth naming plainly: in the common case, a sensibly-ordered fallback
chain (cloud/free models first, BitNet as offline backstop — already the
documented convention elsewhere in this codebase, e.g. `i_loop.rs`'s
`resolve_meta_model`) already keeps the judge off BitNet in practice. The
risk is narrow — only a request where BitNet is the *only* other
configured candidate would pay the cold-start tax twice. Not fixed here;
named so it isn't mistaken for "already handled by design" when it's
actually "handled by circumstance, in the common configuration."

**Second real constraint, separate from latency**: OpenRouter's free tier
has a finite, tracked daily quota (`src/openrouter_quota.rs`'s `FreeQuota`
— this session's own walks measured it depleting, e.g. ~895-948/1000
remaining after a day's work). "More calls, if cheap and fast" is true for
*latency*; it is not free for *quota*. A design that adds a judgment call
at every one of the ~20 real call sites in this codebase, fired on every
request, would measurably shorten how long the free tier lasts in a day —
bounded, rare triggers (confetti specifically, not "every call") are what
keep this sustainable, which is exactly how E8 and the C6 capture mechanism
are already scoped. Any future expansion should keep that same bound:
fire on a real, detected anomaly, not unconditionally.

---

## 3. Hardware-adaptive performance — what's done, what's open

Established earlier the same day (see `CHECKLIST.md`'s hardware-review
entries for full detail, summarized here for the guide's own completeness):

- **Fixed**: `execute_bitnet` never passed llama.cpp's `-ngl` (GPU layer
  offload) flag at all — its sibling `execute_gguf` did. BitNet, the exact
  model this session spent the most time measuring as slow, could never
  use a GPU regardless of user configuration. Fixed with the identical,
  already-proven-safe pattern (`unwrap_or(0)` — cannot regress a GPU-less
  machine).
- **Established as the right primitive**: `std::thread::available_parallelism()`
  (stable `std`, platform-generic — Linux/macOS/Windows, any CPU vendor) is
  now the sizing basis for the grammar-wave's concurrency width, rather
  than a hardcoded constant. This is the first hardware-scaled concurrency
  pattern in the codebase; extend it rather than reinventing a new sizing
  approach elsewhere.
- **Open, smaller follow-ons, not yet built**: thread-count (`-t`) tuning
  is wired nowhere on any local path (llama.cpp uses its own internal
  default); ONNX's execution path (`execute_onnx`) has no execution-provider
  selection, defaulting to CPU-only via `optimum`'s own default; no
  hardware-detection code of any kind exists anywhere in the repo (every
  knob is manual config); the Settings UI exposes `gpu_layers` with zero
  guidance on what value to pick.
- **Open, bigger, operator-gated**: the parallelization width multiplier
  (currently conservative at 1× cores — raise only with ledger evidence,
  per this session's own stated plan); the keep-warm subsystem (D3) that
  would resolve §2's local-model cold-start exception entirely rather than
  just working around it.

---

## 4. Where this doctrine still has real work to do

Candidates for the "cheap check before expensive path" pattern, not yet
built, each requiring the same scoped-bound discipline as E8 (fire on a
detected anomaly, never unconditionally):

1. **E8's remaining 5 real confetti sites** (`assistant_check_up`,
   `i_loop_reflection`, `meta_loop_draft`, `zero_shot_simulation`,
   `amt_reexpansion`) — all background-loop or stateful-method sites that
   still hit a blind discard-and-retry on confetti today. Each already has
   its own task-prompt and some model-resolution mechanism in scope (see
   this session's own review of this), so wiring the same adjudication
   pattern is thin integration work, not a new design.
2. **Empty-response retries generally**: today, an empty/unusable response
   triggers a blind same-model retry or a fallback-chain walk — a full-price
   repeat. Reasoning-salvage already does a cheaper version of this specific
   idea (check the surfaced reasoning field before declaring failure, no new
   call needed) — worth asking, site by site, whether a similarly free
   (no-new-call) recovery exists before reaching for a new judgment call.
3. **Pipeline 18 / E6** (already flagged in `ZERO_SHOT_EXPANSION_GUIDE.md`):
   the inverse case — a *mechanical* analysis (tree-sitter AST) that may
   already solve a problem a zero-shot call was proposed for. Cheaper than
   either a blind retry OR a judgment call, when it applies.

---

## 5. The standing checklist, going forward

Before adding a new call anywhere in this codebase for efficiency reasons,
check in this order:

1. Does the failure this call would prevent actually cost more than the
   call itself, on the hardware/backend that will actually run it? (§2 —
   check keep-warm status before assuming "cheap.")
2. Is there a free (no-new-call) recovery already available — better
   parsing, a field already in the response, a mechanical analysis that's
   dormant but real — before reaching for a model? (§4.2, §4.3, and the
   whole Review-before-judge rule in `ZERO_SHOT_EXPANSION_GUIDE.md` §1.)
3. Is the trigger condition bounded to a real, detected anomaly, not
   "every call" or "every request"? (§2's quota point.)
4. After building it: did you verify the *data flow*, not just that it
   compiles? (§1 — this is the one that actually bit this session.)

No exceptions, same discipline as every other doctrine rule in this
project's docs.

---

## 6a. Order is real dependency structure — not a thing to remove, and not a thing to fake (2026-10-09, operator-directed)

A direct correction, worth stating as its own principle before the sweep
findings below: parallelizing a loop is not "remove the order." It's
**identify the REAL dependency structure between items, and model it
exactly** — items with no real dependency on each other should run with
maximum concurrency and zero artificial synchronization; items that DO
depend on another specific item's result must wait for exactly that, not
for an unrelated batch boundary. Getting this wrong in either direction is
a real cost: serializing something independent wastes speed for no
correctness gain; parallelizing something dependent without preserving the
dependency (or band-aiding it with a coarse "collect-then-serial-apply"
pattern when a finer one would do) risks silent wrong answers, same as the
bugs found earlier today.

**Re-reviewed all 5 real parallelization cases in this codebase (3 shipped,
2 specced) against this framework, with a careful, file-verified answer for
each — not assumed from the loop's general shape:**

| Call site | Real cross-item dependency at APPLICATION time? | What kind |
|---|---|---|
| Grammar-wave (`extract_grammar_from_graphs`, shipped) | **No** — each sentence's grammar node/relationships are written to its own independent graph entry; confirmed no cross-sentence dedup/conflict check in the write-back loop. | — |
| Cross-reference pairs (shipped) | **No** — confirmed by re-reading the application loop (`amt.rs:2831+`): each pair's `record_thinking`/`cross_refs.push` is independent, no cross-pair dedup or conflict check exists. | — |
| AMT branch-discovery lanes (shipped, Tier 0) | **Yes, real** — `amt.rs:3009-3012`: `state.branch_captures.iter().any(|bc| bc.parent_intent == resolved_parent && norm_intent_key(&bc.branch) == norm_intent_key(&branch_str))` — lane N's dedup check reads every branch ALREADY captured from earlier lanes in the same pass; a duplicate branch merges into the existing entry instead of double-counting. | Dedup-against-canonical-order |
| AMT per-chunk intent extraction (specced) | **Yes, real** — the post-response merge/dedup (`amt.rs:1452-1498`) reads `state.intent_captures` live, so chunk 3's dedup correctly sees intents chunk 1 already captured. | Dedup-against-canonical-order |
| AMT per-domain methodology loop (specced) | **Yes, real** — `!state.methodologies.contains(&existing_id)` (`amt.rs:3487`): domain B's dedup check depends on whether domain A (processed earlier) already added the same methodology. | Dedup-against-canonical-order |

**The honest nuance across all three real-dependency cases**: in every one,
"canonical order" is the ORIGINAL array order the upstream step (batching,
chunking, or a model's own domain-list output) happened to produce — not a
deep semantic truth about which item is more "correct" or "important." A
duplicate's survivor is "whichever came first in that order," which is
**internally consistent and deterministic** (same input always produces
the same output — the property that actually matters for reproducibility
and debugging) but should not be mistaken for "the dependency tells us
domain A's result is semantically prior to domain B's." It's a real
dependency in the sense that B's correct handling REQUIRES knowing what A
already did; it's an arbitrary tie-break in the sense that nothing says A
*should* win over B on the merits. Both things are true at once — name
them separately so a future reader doesn't over-read the tie-break as
meaningful or under-read the dependency as removable.

**The real, concrete "go as fast as possible" finding**: the shipped
fixes' wave-barrier pattern (`work.chunks(width)`, wait for the ENTIRE
width-N batch before starting the next) is itself a form of **fake order**
— it forces a synchronization point between items that have no real
dependency on each other (confirmed above: none of the three batching
layers have a REAL same-wave dependency; the dependency that exists is
always against the FULL canonical history up to that point, which the
*serial application phase after each wave* already correctly provides —
nothing requires the DISPATCH side to wait wave-by-wave too). A straggler
in wave 2 (e.g. one call hitting a same-model retry) delays wave 3 from
even starting, even though most of wave 3's work has no relationship to
wave 2's straggler at all. This was flagged earlier (§6 Tier 3 #14) as a
"modest, low-urgency" cost — re-assessed here as the actual answer to
"how do we go as fast as possible through all of them": replace the
wave-barrier with a **continuously-fed, concurrency-limited dispatcher**
(a bounded semaphore or `buffer_unordered`-style stream) that starts the
next item the instant any slot frees, never idling capacity while waiting
for a sibling that has nothing to do with it.

**Recommendation: build ONE shared pattern, not five ad hoc wave-barrier
sites.** Concretely, a small reusable helper shaped like: `async fn
dispatch_concurrent<K, T>(work: Vec<(K, Fut)>, width: usize) -> HashMap<K,
T>` — runs up to `width` futures concurrently at all times (continuous
dispatch, not wave barriers), keyed by real identity `K` (the now-proven
pattern — never positional), panics handled by elimination (a missing key
after full drain is definitionally a panic, never guessed). Callers
(grammar-wave, cross-reference, the lanes retry set, and both pending
specs) all hand it their own `(identity, future)` pairs and get back a
`HashMap` to apply however their OWN real dependency structure requires —
fully serial if there's a real canonical-order dependency (lanes, intent
extraction, per-domain methodology), fully free-order if there isn't
(grammar-wave, cross-reference). This is the "capture order neatly, so
dependent work can wait on exactly what it depends on while independent
work moves as fast as possible" the operator asked for — one place that
gets the dispatch-and-collect mechanics right, instead of re-deriving them
(and re-risking the ordering-bug class) at every new call site.

**A real, separate gap found in the ALREADY-SHIPPED lanes fix while
re-reviewing for this — "fast" must never mean "unboundedly fast on a
straggler," which is its own form of degradation (operator correction,
2026-10-09: the goal is genuine efficiency, never speed bought by quietly
dropping a guarantee).** `metered_execute_resilient` (the sequential path)
deliberately sizes its outer timeout proportionally to chain length —
`secs = per_attempt_secs * (1 + walk_candidates + pool_extra)`
(`mod.rs:3100-3119`) — specifically because a multi-candidate fallback walk
can legitimately need several attempts' worth of time, and its own comment
names exactly why: "a walk of N candidates cannot legitimately take longer
than (1+N) attempts." **The already-shipped AMT lanes fix calls the same
underlying `walk_fallback_chain_standalone` — which can walk a FULL
fallback chain per lane ("each lane walks the health-reordered fallback
chain," `amt.rs:2766-2769`) — with NO outer timeout at all** (confirmed:
re-read the exact spawn body, `amt.rs:2770-2786` — no `tokio::time::
timeout` anywhere in it). This wasn't caught when that fix's ordering/
panic-safety was independently verified earlier, because that review
checked correctness, not bounding — a different question. Net effect: a
single lane whose primary model fails and walks through several slow
fallback candidates (worst case, multiple BitNet attempts at up to 1800s
each) can occupy a concurrency slot for a correspondingly large multiple of
that, with no explicit ceiling matching the sequential path's own
deliberate one. This directly undercuts "maximum efficiency," especially
once dispatch moves from wave barriers to the recommended continuous
model above: an unbounded straggler in a continuous-dispatch pool actively
reduces the real concurrency available to everything else waiting for a
slot, not just delaying one wave.

**This also corrects the cross-reference fix-shape recommendation
(`BUILDER_REGISTRY.md` §3) one more time**: "swap the bare `executor.
execute` call for `walk_fallback_chain_standalone`" is necessary but not
sufficient on its own — done alone, it would fix the resilience/ledger gap
while silently *removing* the flat `per_attempt_secs` timeout the current
code already has (confirmed: the current wave code wraps its bare call in
exactly that, `amt.rs:2002-2016`), leaving the swapped-in walk with no
bound at all, the same gap as the lanes fix. The complete fix needs both
halves together: reuse `walk_fallback_chain_standalone` for the real
retry/fallback/ledger behavior, AND wrap it in a timeout sized by the same
proportional formula `metered_execute_resilient` already uses (`per_
attempt_secs * (1 + real_candidate_count + pool_extra)`), not the flat
single-attempt timeout the bare-executor version used. **The shared
dispatcher helper above should bake this in as a required parameter** (a
per-item timeout budget, proportionally sized by the caller to however
many candidates that item's own future might walk through) rather than
leaving it optional or assumed — this is the concrete mechanism that keeps
"as fast as possible" from ever meaning "unboundedly slow on the one item
that happens to need every fallback candidate."

**Not built this round** — this is a design recommendation, surfaced by
review, not unilaterally implemented. If pursued, the retrofit order by
value: build the shared helper once, prove it by retrofitting the AMT
lanes fix (since it already has the most complex real dependency of the
three shipped sites — the best test of whether the helper's contract is
right), then the grammar-wave and cross-reference fixes become simpler
(delete their own ad hoc wave loops), then both pending Tier-1 specs are
written directly against the shared helper from the start instead of
inventing their own wave code.

---

## 6. Full E2E efficiency sweep (2026-10-08) — every area, read-only, ranked

Six parallel read-only review forks swept every major area of the system
(modality pipelines, the four background loops, the core orchestration
walk, AMT specifically, gate/capture/ZSEI infrastructure, and a structural
sweep of every remaining loop construct), each held to this doc's own
standard: verify against real code, report honest negatives, don't force
findings. Every claim below was independently spot-verified before being
recorded here — this is not a transcription of fork output.

### Confirmed clean — checked and cleared, not just unexamined

Worth recording as explicitly reviewed, per this doc's own "report honest
negatives" standard — these are NOT findings, they're the record that the
check happened:

- Code/image modality pipelines make zero pipeline-9 calls (regex/structural
  only) — no parallelization need.
- Math's one real pipeline-9 call site (`resolve_implicit_step_references`)
  is already correctly batched (one call for all unresolved steps, not
  per-item).
- `assistant.rs`'s background loop: the one clean example of "cheap check
  before expensive call" already native to this codebase — zero LLM calls
  on an idle tick, by explicit design. The pattern every other loop below
  should be measured against.
- ZSEI's `ContainerStorage`/`TraversalEngine`: real `HashMap`-indexed lookup
  (not a scan) and real visited-set tracking (no re-visits). Sound as built.
- The gate's 5s keeper-thread poll and the 3x per-call-open capture-file
  write pattern: both real but categorically negligible next to the LLM
  call latency they ride alongside (microseconds vs. seconds). Not worth
  engineering time at today's scale.
- The User-tier parallel-batch collection loop (`mod.rs`'s `run_one_candidate`
  callers): already efficient — breaks on first usable result, drains
  losers in the background, no stale-data risk.
- The grammar-wave fix (§1 above): re-verified independently as correctly
  applied — stabilization now runs before `work` is built, both sides of
  the round-trip agree on the key.

### Ranked findings — real, verified, not yet fixed (this was a read-only pass)

**Tier 0 — a correctness bug, not an efficiency one, found by the same sweep:**

0. **AMT's parallel branch-discovery lanes silently misattribute results
   when lanes complete out of spawn order** — `methodology_branch_discovery_
   parallel` (`src/orchestrator/amt.rs:2514`, live on BOTH AMT-building
   routes). Full writeup in `docs/BUILDER_REGISTRY.md`'s §3 (where this
   function is already catalogued) — this is the single most severe
   finding across the entire 6-fork sweep, pre-existing (not introduced by
   today's text-pipeline refactor), the same ordering-bug CLASS found
   independently in a second, already-live mechanism. Listed here only as
   a pointer, not duplicated in full — see `BUILDER_REGISTRY.md` for the
   complete trace and fix shape.

**Tier 1 — clear parallelization wins, same proven shape as the grammar-wave fix:**

1. **AMT cross-reference pairwise loop** (`src/orchestrator/amt.rs:1923`,
   `'pairs: for i in 0..branch_list.len() { for j in (i+1)..branch_list.len() {...} }`,
   bounded by `max_pairs` — confirmed real default **50 or 100**,
   `src/k_registry.rs:81,85`). Up to 100 fully-independent sequential
   pipeline-9 calls (each pair's "are these two branches related" judgment
   doesn't depend on any other pair). The cleanest parallelization
   candidate found in this entire sweep — no collect-then-apply nuance,
   directly reusable as the same wave/`available_parallelism()`-sized
   dispatch pattern already proven in `extract_grammar_from_graphs`.
   **SHIPPED (ZCode, 2026-10-09), independently verified correct on the
   ordering/panic question (HashMap keyed by real pair identity `(i, j)`,
   not position — robust by construction).** But verifying it surfaced a
   real, separate finding: the parallel version drops the retry/fallback-
   chain/ledger-capture guarantees `metered_execute_resilient` gave each
   pair call sequentially (no same-model retry, no fallback to a different
   model on failure, no `model_ledger`/S11 capture entry at all going
   forward for this call site). Full writeup in `docs/BUILDER_REGISTRY.md`
   §3 — not fixed, flagged for a decision, same as the rest of this list.
   **Fix-shape corrected 2026-10-09**: reuse `walk_fallback_chain_
   standalone` (`mod.rs:4029`, already `&mut`-free, already proven under
   concurrent dispatch by the Tier-0 lanes fix) instead of building new
   helper code — see `BUILDER_REGISTRY.md` §3 for the full correction.
2. **AMT per-chunk intent-extraction loop** (`amt.rs:1353`) — parallelizable,
   but needs a different collection shape than #1: the prompt's context is
   already snapshotted once before the loop (safe to parallelize), but the
   dedup check currently runs immediately after each chunk's own call
   returns, benefiting from already-completed earlier chunks — parallelizing
   requires collecting a full wave's results first, then deduping across
   all of them together.
3. **AMT per-domain methodology loop** (`amt.rs:3478`) — same
   collect-then-batch-dedup shape as #2, smaller scale (bounded by
   `required_domains.len()`). **Skipped, 2026-10-09 (ZCode) — decision
   likely still reasonable, stated reason was factually wrong, corrected
   here.** The call site was skipped on the claim that it's "store-local
   searches, not LLM calls... microsecond-scale." Re-read directly: the
   per-domain `search_by_keywords` call IS store-local and fast, but when
   that search finds nothing, the loop falls through to a real,
   conditional `self.metered_execute_resilient(state, synth_input,
   "methodology_synthesis")` call (`amt.rs:3515`, `max_tokens: 400`) — a
   genuine pipeline-9 call, not microsecond-scale, unchanged and still
   present. Both this session's own fork (§6 above) and this correction
   confirm the real LLM call exists; the DECISION to deprioritize is still
   likely sound (realistic domain counts are small per the fork's own
   finding, so the absolute win is modest either way), but "there's no
   LLM call here" is not an accurate reason for it — don't let a future
   reader conclude this call site has nothing left to parallelize.

**Tier 2 — real waste, not parallelization, fixable today:**

4. **`i_loop.rs` makes a full-price LLM call every single tick, even when
   there is nothing to reflect on** (`i_loop.rs:83-96`). When
   `amt_candidates::unhandled()` is empty, the code doesn't skip the call —
   it sends the literal string `"No unhandled findings or re-expansion
   candidates right now."` to the model anyway, paying full latency/token
   cost for a reflection on nothing. Guaranteed, perpetual waste on the
   (likely common) idle case. Fix shape: adopt `assistant.rs`'s exact
   pattern one file over — check for real content first, skip the call
   entirely when empty.
5. **`i_loop.rs` persists a new permanent file + graph container every
   tick, unconditionally, with no pruning** (`i_loop.rs:216-287`) — compounds
   #4. At the 1800s interval floor that's 48 new permanent nodes/day,
   minimum, whether or not anything occurred. Fix shape: gate persistence
   on real content the same way `assistant.rs` gates its own call.
6. **`meta_loop.rs` has no per-pass candidate bound** — missing the
   equivalent of `amt_loop.rs`'s `MAX_CANDIDATES_PER_PASS = 12`. A burst of
   many fresh gaps in one pass can run arbitrarily long, pushing out every
   subsequent pass's start time (the sleep only starts after the previous
   pass returns). Fix shape: copy `amt_loop.rs`'s bounded-batch pattern.
7. **`amt_loop.rs`/`meta_loop.rs`'s cross-pass escalation defers an
   already-known decision by up to ~5 hours worst case** (10 max attempts ×
   1800s interval) instead of retrying in-pass with the escalated model
   immediately. `meta_loop.rs` has no event-driven wake at all to shorten
   this gap (confirmed: no `Notify`/`wake()` pattern anywhere in the file);
   `amt_loop.rs` has one but it's tied to graph ripples, not specifically
   to retry-readiness. Fix shape: on `Err`, retry immediately in-pass with
   the escalated override before giving up the attempt slot for that cycle.
8. **`amt_loop.rs`/`meta_loop.rs` never prune handled entries from their
   candidate/gap log files** — both re-read and re-iterate the FULL
   historical list every pass, forever, just to skip past already-handled
   entries. Unbounded growth on a long-running install. Fix shape:
   periodically compact each file to unhandled-only (or unhandled +
   recent-N handled, if the handled record has diagnostic value).
9. **`file_relationship_summary`'s 3x redundant container fetches**
   (`stages.rs:301`, called from `stages.rs:546`, `amt.rs:680`, `amt.rs:1293`)
   — a real network/IPC round-trip per attached file, on data (`state.
   file_graphs`) that doesn't change between these 3 calls within one
   request. Fix shape: compute once, cache on `OrchestrationState`, thread
   the string to all 3 sites. (`jurisdiction_summary`, called 5 times at
   similar sites, is pure in-memory formatting — redundant CPU, not I/O;
   same fix shape applies but lower severity.)
10. **Image's `update_graph` never persists its updates** — see
    `GRAPH_RELATIONSHIP_REGISTRY.md` §3b for the full writeup (this is
    primarily a correctness/reality finding, cataloged there; noted here
    for completeness of the ranked list since it was found in this same
    sweep).
11. **Math's (and, once fixed, image's) `update_graph` is O(total graph
    size) per update** — a full read-modify-write of the entire graph file
    on every single update call, plus an O(n) linear scan to find the node
    being updated, regardless of how small the delta is. Fix shape: switch
    in-memory node storage to a `HashMap<u64, Node>` for O(1) update, and/or
    confirm real graph sizes stay small enough that this doesn't matter in
    practice before investing effort.

**Tier 3 — real, verified, but low severity at today's scale (don't prioritize):**

12. **Jurisdiction's per-request `to_lowercase()` recompute**
    (`jurisdiction.rs:502`) — allocates a new lowercased `String` on every
    one of 161 real rules (confirmed count, `assets/jurisdiction/*.json`,
    42 files), every request, for the same unchanging config strings.
    Microseconds in absolute terms. Fix shape: precompute once at rule-load
    time instead of per-request.
13. **`context_budget::assemble`'s O(sections × total-size) re-concatenation**
    (`context_budget.rs:117,131`) — re-joins the full kept-sections vector
    into a fresh `String` on every trim-loop iteration instead of tracking
    a running token count incrementally. Real, but only 2 call sites exist
    and realistic section counts/sizes are small (hand-authored prompt
    templates, tens of KB) — sub-millisecond in practice. Fix shape: track
    `kept_tokens: usize` incrementally; not urgent.
14. **Grammar-wave's wave-barrier pattern** — `work.chunks(width)` processes
    strictly wave-by-wave (a straggler in wave N delays wave N+1's start
    even if siblings finished instantly), rather than a continuously-full
    concurrency pool. Bounded by wave-boundary count, not sentence count —
    modest in practice given typical per-chunk sentence counts and
    single-digit `width` on consumer hardware. Fix shape: a semaphore-gated
    continuous-dispatch stream would remove this, but only worth the added
    complexity if ledger data later shows retry-triggered stragglers are
    frequent enough to matter.

**AMT-dedicated follow-up pass, refining items #1-3 above**: a second,
deeper fork focused specifically on AMT confirmed and sharpened these
findings. Intent extraction (`amt.rs:1341-1502`) is confirmed a clean,
zero-semantic-change parallelization candidate — `known_intents_json` is
genuinely snapshotted once before the loop. Cross-reference's real live
`max_pairs` value is **50**, not the 50-100 range cited from `k_registry.rs`
generally — already governed by a real bound, not a complexity risk as
built, just a sequential-when-parallelizable one. Re-expansion
(`amt_loop.rs`) runs on a confirmed real 1800s (30 min) interval with
`MAX_CANDIDATES_PER_PASS = 12` — worst-case ~8 minutes of work fits with
large margin, so lower urgency than #1/#9. **New nuance on detail
extraction** (`amt.rs:1554+`, not previously captured): its per-chunk calls
look independent like intent extraction, but `known_details_json` is
rebuilt fresh each iteration from `state.detail_captures`, which GROWS as
earlier chunks in the same loop are processed — real cross-chunk dedup
context feeds directly into the prompt, not just a post-hoc check.
Parallelizing this one specifically requires explicitly accepting loss of
(or staging) that cross-chunk signal — not a free win like intent
extraction; named as a real trade-off, not a bug.

**Not findings — explicitly out of this sweep's depth, flagged honestly
rather than claimed clean**: code pipeline's `update_graph` and broader
redundant-computation patterns in code/math/image (not traced to a
conclusion within this pass's budget); `src/grpc/mod.rs`'s ~29 route-handler
loop sites (spot-scanned, nothing alarming surfaced, not exhaustively
vetted); UI `setInterval` polling cadences across components (individually
reasonable for a single-user desktop app, not checked for cross-component
consolidation opportunities).
