# Static/Mechanical Logic Registry — every regex, keyword-match, threshold, and extension-lookup decision site

> Companion to `docs/ZERO_SHOT_CALL_REGISTRY.md` (the judgment-call side) and
> `docs/ZERO_SHOT_EXPANSION_GUIDE.md` (where mechanical logic should become a
> real call). This doc catalogs the OTHER side of that doctrine's mechanical-
> vs-judgment classifier: every site that decides something via regex,
> `.contains`/keyword matching, a fixed arithmetic threshold, or an
> extension/suffix lookup — not to upgrade them (most should stay mechanical,
> per the doctrine: mechanical is the efficiency), but so each one's actual
> correctness can be tracked and checked, not assumed from the fact that it
> exists. Built 2026-10-08 per direct operator request, after a correction
> that review work was inventing ad-hoc structure instead of grounding in
> this project's own documented methodology.

**Methodology**: two independent read-only sweep forks (orchestrator-layer;
modality-pipeline + graph layer), cross-checked against the two existing
docs to avoid re-cataloging what's already there, then spot-verified against
the real source directly (not trusted as self-report) — every entry below
marked **[verified]** was independently re-read by CC against the live file;
entries without that marker came from a fork's report and were not
individually re-read line-by-line, though the fork's track record on this
pass was 6-for-6 on direct spot checks, including exact line numbers.

## 1. Headline findings (ranked by consequence)

1. **A better implementation is dead; a worse one is live — the clearest
   concrete case of the operator's "different paths" concern.** Pipeline 18
   (`assets/pipelines/general/code_analysis/main.rs`) has a real tree-sitter
   AST-based `extract_function_calls` (per-language dispatch:
   `tree_sitter_rust`/`_python`/`_javascript`/`_go`, regex only as a parse-
   failure fallback) — genuinely more capable than pipeline 101's live,
   same-file-only, bare-regex version (`(\w+)\s*\(` + a same-file name
   lookup), which is exactly the implementation `ZERO_SHOT_EXPANSION_GUIDE.md`
   E6 documents as structurally unable to resolve cross-file calls. **[verified]**
   pipeline 18 is registered (`zsei_data/pipelines/index.json:263`,
   `"pipeline_id": 18`) but has **zero dispatch call sites** anywhere in
   `src/orchestrator/*.rs` or `src/pipeline/*.rs` (grepped, confirmed empty) —
   orphaned since whenever it was built. Neither registry doc mentions
   pipeline 18 exists at all. This is not hypothetical redundancy-risk; it's
   a real, better, already-built solution sitting unreachable while the
   worse one runs in production.
   **[verified 2026-10-08, operator-directed re-check]**: confirmed pipeline
   18 is NOT secretly feeding the UI's real "Code Call Graph" feature
   (`ui/src/views/engines/CodeCallGraph.tsx`) either — that component's own
   sourcing comment names pipeline 101's `Calls` edges directly
   (`assets/pipelines/modalities/code/main.rs` ~2748-2765), not pipeline 18.
   So pipeline 18 isn't duplicating a live "native graph events" path; it's
   simply unreached by anything. Found a related, separate piece of dead
   code while checking this (see §1.9 below).
9. **A third, richer call-graph shape exists, fully dead, independent of
   pipeline 18.** `src/types/container.rs:1035-1055` defines a `CallGraph`
   struct (`nodes: Vec<CallGraphNode>` with `call_depth`/`fan_in`/`fan_out`,
   `edges: Vec<CallGraphEdge>` with `call_count`/`is_recursive`) — richer
   than the plain `Calls` relationship edges pipeline 101 actually produces
   (which carry none of depth/fan-in/fan-out/recursion). **[verified]**
   grepped every `.call_graph` read/write site in `src/`: zero matches
   outside the struct's own definition — nothing populates it, nothing reads
   it. **[verified]** the UI doesn't reference `fan_in`/`fan_out`/`call_depth`/
   `recursive` anywhere in `CodeCallGraph.tsx` or `EnginesPanel.tsx` either —
   this isn't a known-missing feature the UI is blocked on, just unconnected
   scaffolding. Plausible (not confirmed) relationship to finding #1: this
   struct's shape is exactly what a whole-program AST-based call analysis
   (pipeline 18) could populate but pipeline 101's same-file regex can't —
   three pieces (pipeline 18, this struct, a richer call-graph UI) look like
   an abandoned, partially-scaffolded feature, not three unrelated gaps.
2. **627 lines of dead regex classifier.** `src/orchestrator/modality_detector.rs`
   — a complete, tested, 9-modality `ModalityDetector` (extension map + 24
   `Regex::new()` text patterns + 6 task-type patterns). **[verified]** file
   exists, exactly 627 lines, zero references anywhere in the repo
   (case-insensitive grep across every `.rs` file), not declared as a
   `mod`/`pub mod` in `src/orchestrator/mod.rs` or `src/lib.rs` — does not
   even compile into the binary. Pre-dates the live 27-modality system (only
   knows 9) and isn't mentioned in either zero-shot doc. Should be deleted or
   consciously revived — currently neither.
3. **`link_related_containers` is copy-pasted across 3 of 4 modality
   binaries, and has already drifted once.** Defined independently (not
   shared) in `text/main.rs:237`, `code/main.rs:385`, `math/main.rs:2696`;
   absent entirely from `image/main.rs`. **[verified]** diffed all three
   function bodies directly: code and math are byte-identical to each other;
   text carries an extra 11-line comment (an E2 architectural note — "pipelines
   can't call pipeline 9, no auth... confirmation belongs at the orchestrator
   level") that was never backported to code/math's copies. This is not a
   theoretical risk — it's already-observed drift, caught in the act: a
   documentation fix landed in one copy and silently never reached the other
   two. The `(0.3 + 0.15×shared_count).min(0.9)` confidence formula itself
   (already cataloged as a single conceptual site in both zero-shot docs) is
   actually 3 separately-maintained copies of that formula, not one.
4. **The most safety-relevant deterministic logic in the codebase has no
   word-boundary protection.** `jurisdiction.rs:469`,
   `categorize_jurisdiction_matches`: **[verified]**
   `haystack.contains(&rule.condition.to_lowercase())` — a raw substring
   match gating `Block`/`Warn`/`RequireConfirmation` jurisdiction actions.
   A condition string that's a substring of an unrelated word (e.g. a short
   condition matching inside a longer unrelated token) would false-positive
   with no guard. This is the one deterministic site in the registry whose
   failure mode is a policy miss, not just a quality/attribution miss.
5. **An already-admitted bug pattern, reused unfixed two lines away from its
   own warning.** `amt.rs:2873-2880`: **[verified]**
   `mname.to_lowercase().contains(&n.to_lowercase()) || n.to_lowercase().contains(&mname.to_lowercase())`
   — bidirectional substring match, used to attribute a branch's methodology
   string back to a known methodology ID. The very next lines (2881-2882,
   a comment on a *different* dedup check right below) say: "was two-way
   substring, which dropped branches silently" — i.e. this exact pattern was
   already found to silently lose data at a sibling site in the same
   function and fixed there (switched to normalized-equality), but the
   attribution site above it still runs the risky version.
6. **An unverified 5-keyword classifier is the sole basis for an operational
   conclusion already being cited.** `model_ledger.rs:434-480`,
   `cause_of_with_cap`: **[verified]** first-match-wins chain ending in
   `t.contains("pipeline") || t.contains("executor") || t.contains("watchdog")
   || t.contains("missing") || (t.contains("parse") && !t.contains("response"))`
   to attribute a failure to `Cause::OzoneStudio`. This chain is the entire
   basis for CHECKLIST's "provider almost never the cause (2/83)" conclusion
   — never systematically tested against the actual corpus of captured error
   strings it classifies. A false match (a model's own response text happens
   to contain "missing") silently misattributes blame.
7. **E4 (web-search-need gate) was open in the guide but is actually
   resolved** — traced and confirmed (now corrected in
   `ZERO_SHOT_EXPANSION_GUIDE.md` directly): no deterministic gate exists;
   pipeline 56 routing is decided by the #10 blueprint-assignment zero-shot
   call itself. Nothing to fold, nothing to build.
8. **Two independent, unrelated error-text classifiers exist with no shared
   vocabulary.** `model_ledger.rs::cause_of_with_cap` (above) and
   `amt_loop.rs:376-378`'s permanent-vs-transient retry gate
   (`e.contains("has no object_store_path") || e.contains("No such file or
   directory") || e.contains("EOF while parsing")`) each reinvent
   "classify this error string by substring," independently, for different
   purposes, with no shared helper or vocabulary between them.

## 2. Full table

| file:line | function | mechanism | what it decides | exact logic | already cataloged elsewhere? | correctness concern |
|---|---|---|---|---|---|---|
| `orchestrator/modality_detector.rs:1-627` | `ModalityDetector` | regex (24 patterns) + extension-lookup | modality classification (text/file/task-type) | `Regex::new(r"\b(theorem\|lemma\|...)\b")` etc. | NOT CATALOGED | **[verified]** dead code — uncompiled, unreferenced, 9-of-27 modality coverage |
| `orchestrator/graphs.rs:706-726` | `detect_file_modality` | path-keyword hints, then extension-lookup | file → modality pipeline routing | 5 `path_lower.contains("radar"\|"sar"\|...)` hints, then extension `match` | ZERO_SHOT_EXPANSION_GUIDE E1 (describes as "pure extension-lookup" — incomplete; doesn't mention the keyword-hint layer) | E1's proposed fix (content classification) doesn't address the path-hint layer |
| `orchestrator/graphs.rs:752-772` | `modality_name_to_pipeline_id` | fixed string→id match table | modality string → pipeline_id | `match modality { "text"=>100, ... _=>0 }` | NOT CATALOGED | checked downstream-safe: `pipeline_id > 0` gated before use; cross-checked against `index.json`, no mapping bugs |
| `assets/pipelines/modalities/{text,code,math}/main.rs` (237/385/2696) | `link_related_containers` ×3 | keyword search + fixed arithmetic confidence | container relationship edge creation | `(0.3 + 0.15 * shared_count as f32).min(0.9)` | ZERO_SHOT_EXPANSION_GUIDE E2 / ZERO_SHOT_CALL_REGISTRY §3.2 (as ONE site) | **[verified]** actually 3 maintained copies (code≡math byte-identical; text has an un-backported comment) — real, already-occurred drift risk; `image` has no copy at all |
| `code/main.rs:1945-1973` | `extract_function_calls` (pipeline 101, live) | regex `(\w+)\s*\(` + same-file lookup | `Calls` edge extraction | callee must be in same-file `function_names`; `is_method` via `line.contains(&format!(".{}", callee))` | ZERO_SHOT_EXPANSION_GUIDE E6 | `is_method` substring-contains can false-positive (e.g. a string literal containing `.{callee}`) — not previously flagged |
| `assets/pipelines/general/code_analysis/main.rs:490-540+` | `extract_function_calls` (pipeline 18, **orphaned**) | tree-sitter AST (real per-language parse) + regex fallback | same job as above, better | `collect_calls_recursive` over real ASTs (`tree_sitter_rust/_python/_javascript/_go`) | NOT CATALOGED ANYWHERE — pipeline 18 isn't mentioned in either zero-shot doc | **[verified]** registered, zero dispatch sites — see §1.1 |
| `math/main.rs:1641-1673` | `extract_step_references` | regex (2 patterns) + implicit-phrase keyword list | proof-step citation resolution | `r"(?i)\b(?:steps?\|eqs?\.?\|...)\s*#?\s*(\d+)\b"`, `r"\((\d+)\)"`, `IMPLICIT_MARKERS` contains-check | ZERO_SHOT_EXPANSION_GUIDE E7 | none beyond E7's documented gap; bounds check (`n < current_step_number`) confirmed correct, never fabricates a forward ref |
| `stages.rs` blueprint-generation prompt (~560-650) + coercion at `stages.rs:829` | — (E4, resolved) | — | whether a step gets pipeline_id 56 (WebSearch) | no separate gate; decided entirely inside the #10 zero-shot call | now marked RESOLVED in ZERO_SHOT_EXPANSION_GUIDE E4 | n/a |
| `mod.rs:2740-2774` | `classify_attempt` | keyword (`.contains("timed out")`, `.starts_with("watchdog")`) | raw result → `Outcome` enum | `None if e.contains("timed out") && !e.starts_with("watchdog") => Outcome::Timeout` | NOT CATALOGED | **[verified]** exact-prefix dependency on the literal string `"watchdog"` — any upstream error-text rewording silently reclassifies as Timeout |
| `mod.rs:1558-1565` | `http_status_of` | strict string-prefix parse | HTTP status extraction from error text | `error.strip_prefix("HTTP ")?` then take digits | NOT CATALOGED | **[verified]** requires the exact literal prefix `"HTTP "`; any differently-formatted error string silently returns `None` |
| `model_ledger.rs:409-411` | `is_local_model_id` | prefix-keyword (4 literals) | local vs. remote model classification | `["bitnet","gguf","onnx","local"].iter().any(\|p\| model.starts_with(p))` | NOT CATALOGED | **[verified]** any local model id not starting with one of these 4 exact prefixes is silently misattributed as remote |
| `model_ledger.rs:434-480` | `cause_of_with_cap` | HTTP-status buckets + 5-keyword chain | `Cause` attribution for every ledger failure | see §1.6 | NOT CATALOGED | **[verified]** untested keyword list underlies a cited operational stat — see §1.6 |
| `jurisdiction.rs:462-483` | `categorize_jurisdiction_matches` | raw substring match | jurisdiction rule routing (Block/Warn/RequireConfirmation/Log) | `haystack.contains(&rule.condition.to_lowercase())` | prose-level only (ZERO_SHOT_CALL_REGISTRY §7/§12: "pure deterministic string/keyword logic"), never at this exact mechanism | **[verified]** no word-boundary guard — see §1.4 |
| `stages.rs:1949-1956` | web-search date/time fast-path | keyword + word-count guard | bypasses search+decompose, routes straight to pipeline 56 `CurrentDateTime` | `lower.contains("current date")\|\|...` guarded by `< 12 words` | NOT CATALOGED | narrow, reasonable; word-count bound limits false-positive blast radius |
| `amt.rs:2873-2880` | branch methodology attribution | bidirectional substring | attribute a branch's `methodology` string to a known ID | `mname.contains(&n) \|\| n.contains(&mname)` (lowercased) | NOT CATALOGED | **[verified]** reuses a pattern the adjacent comment admits already dropped data silently elsewhere in this same file — see §1.5 |
| `amt_loop.rs:376-378` | re-expansion retry gate | keyword (`.contains`, 3 literals) | permanent- vs. transient-failure classification for retry budget | `e.contains("has no object_store_path")\|\|"No such file or directory"\|\|"EOF while parsing"` | NOT CATALOGED | independent, unrelated error-classifier #2 — see §1.8 |
| `amt.rs:2423`, `amt.rs:2493` | keyword-overlap scoring | raw keyword count | methodology/intent relevance scoring | `sorted_keywords.contains(k)` / `text.contains(t)` counts | related to the already-cataloged `link_related_containers` formula, different site | same class of risk: untested, arbitrary count-based scoring |
| `response.rs:266-276` | response-graph render check | keyword | verifies rendered prose mentions the frame's subject | `!lower.contains(&frame.subject.to_lowercase())` | NOT CATALOGED | quality gate, not a routing decision — low risk |
| `decision_review.rs:217` | decision normalization | string normalize | matches "decline" after trim+lowercase | — | NOT CATALOGED | trivial, no concern |
| `src/types/container.rs:1035-1055` | `CallGraph`/`CallGraphNode`/`CallGraphEdge` | n/a — struct, not logic | would hold depth/fan-in/fan-out/recursion call-graph metrics | fields only, never populated | NOT CATALOGED | **[verified]** dead — zero read/write sites anywhere in `src/`; UI doesn't reference these fields either — see §1.9 |
| `stages.rs:2493-2499` | loop-continuation check (name unconfirmed) | keyword | appears to gate a loop-continue/complete decision from model output text | `output_text.to_lowercase().contains("complete"\|"done"\|"continue")` | NOT CATALOGED | **not independently re-read** — flagged by the sweep fork as unread/out-of-budget; needs a follow-up read before trusting this row |

## 3. Not re-litigated (already correctly cataloged, confirmed unchanged)

Both sweeps re-checked these against the existing docs' "currently" claims
and found them accurate as written, no drift: `detect_file_modality`'s
extension-lookup (E1's core claim), `link_related_containers`'s formula
value itself (0.3/0.15/0.9 unchanged), `extract_function_calls`'s same-file-
only limitation (E6), `extract_step_references`'s explicit-citation-only
limitation (E7), `is_unusable_pipeline9_result`'s confetti/empty-response
detection (not re-described here, see ZERO_SHOT_CALL_REGISTRY §13).

## 4. Open follow-ups from this pass

- `stages.rs:2493-2499` needs an actual read (table row above is sweep-fork-
  reported, not independently verified) — do this before relying on it.
- §1's 8 headline findings are each independently actionable; none have been
  fixed in this pass except E4 (doc correction only, no code) and C6's
  adjacent bug (`src/consciousness/review.rs`, fixed same day — see
  `ZERO_SHOT_EXPANSION_GUIDE.md`'s C6 resolution note). The rest (dead
  pipeline 18, dead `modality_detector.rs`, triplicated `link_related_
  containers`, jurisdiction's substring match, `cause_of_with_cap`'s
  unverified keyword list, `amt.rs:2873`'s reused risky pattern, the two
  independent error-classifiers) are flagged for operator decision, same
  discipline as C6 was before it was built — not unilaterally fixed here.

## Files

- `src/orchestrator/modality_detector.rs` — dead regex classifier (§1.2)
- `src/orchestrator/graphs.rs` — `detect_file_modality`, `modality_name_to_pipeline_id`
- `assets/pipelines/modalities/{text,code,math,image}/main.rs` — triplicated `link_related_containers`, live `extract_function_calls`/`extract_step_references`
- `assets/pipelines/general/code_analysis/main.rs` — orphaned pipeline 18 (§1.1)
- `src/orchestrator/mod.rs` — `classify_attempt`, `http_status_of`
- `src/model_ledger.rs` — `is_local_model_id`, `cause_of_with_cap`
- `src/orchestrator/jurisdiction.rs` — `categorize_jurisdiction_matches`
- `src/orchestrator/amt.rs`, `src/orchestrator/amt_loop.rs` — branch attribution, retry-gate classifier
