# Built-in blueprint bodies (19) — REPORT v2 (corrected) — 2026-11-10 — NOTHING COMPILED OR RUN

## What was wrong in v1, and what v2 does
v1 (first conversion) remapped legacy pipeline ids to 4/11/12/21/39/100/101/105. **That cannot run:** `stages.rs` (~1695-1725) coerces every non-provider step whose `pipeline_id` is not 9 or 56 to pipeline 9 (warning), and strips a consciousness-category pipeline such as 39 as a SECURITY violation. A blueprint step is therefore exactly one of:
- a **zero-shot step on pipeline 9** (the model gets the step description + the gathered methodology rules + tool results; `action` is only a label), or
- a **provider step** (`step_type: "provider"`, `pipeline_id: 0`, provider id from the registry) that calls a **registered MCP tool** through the gated MCP path.

Graph create/query/link/update, provisional nodes, conflict checks, methodology loading, context aggregation, hook firing and the consciousness check are done by the **walk's own stages** (3 methodology gather, 4 graph creation, 5 AMT, 6 blueprint, 10 execution; consciousness gate internal) — a blueprint step cannot call them.

v2 (`convert.py`, hand-written SPEC, re-run from the untracked legacy bodies in `zsei_data/local/blueprints/`): every legacy step is mapped explicitly —
- **KEEP**: becomes a pipeline-9 step whose description carries the legacy name, inputs, outputs, config, conditional / on_failure / critical / comment meaning in plain words (what it consumes, what it hands on).
- **WALK**: a pure walk-internal operation; removed from `steps`, recorded in body `walk_covered` with the walk stage that covers it (step indices renumbered contiguously, `depends_on` rebuilt).
- **CAP**: a real-world action with no registered provider (write file, run sandbox/script/test, index, submit to /orchestrate, read repo files, repair data, register a methodology): stays a pipeline-9 step that delivers the content/decision, AND the capability is listed in body `missing_capabilities` (plain strings — the exact shape `stages.rs` reads, `Vec<String>`; capability closure captures them to `capability_gaps.json`).
- **PROVIDER**: only where a REGISTERED provider truly fits: `document-mcp` (EU registry port 3282: document_extract / document_info / document_images / document_formats), as an **optional** step (bp 3 existing docs, bp 13 document text). Legacy-only fields are kept verbatim in `legacy_step_fields` keyed by LEGACY step id. `rule_refs` stay empty everywhere (not verified against the regenerated rule bodies — none cited rather than guess); `methodology_ids` unchanged (container ids 30000+N).

## Per blueprint (steps = pipeline-9 + provider; missing_capabilities; walk-covered legacy steps)
| bp | name | steps | pipeline | provider | missing_capabilities | legacy steps covered by the walk |
|---|---|---|---|---|---|---|
| 1 | General Assistant | 3 | 3 | 0 | 0 | 3 |
| 2 | Code Review | 4 | 4 | 0 | 0 | 6 |
| 3 | Documentation | 9 | 8 | 1 | 1 | 7 |
| 4 | Code Creation | 8 | 8 | 0 | 2 | 7 |
| 6 | Data Analysis | 10 | 10 | 0 | 2 | 5 |
| 7 | Mathematical Proof Verification | 10 | 10 | 0 | 0 | 4 |
| 8 | Modality Pipeline Onboarding | 4 | 4 | 0 | 2 | 0 |
| 9 | Jurisdiction Expansion | 5 | 5 | 0 | 3 | 0 |
| 10 | Methodology Creation and Registration | 3 | 3 | 0 | 1 | 0 |
| 11 | Living-Graph Linking Audit | 3 | 3 | 0 | 0 | 0 |
| 12 | Cross-Process Retrieval Hardening | 3 | 3 | 0 | 2 | 0 |
| 13 | Text Document Analysis | 4 | 3 | 1 | 1 | 1 |
| 14 | Incident Postmortem to Methodology | 3 | 3 | 0 | 2 | 0 |
| 15 | Agent Onboarding Protocol | 3 | 3 | 0 | 2 | 0 |
| 16 | Data Integrity Audit and Repair | 4 | 4 | 0 | 2 | 0 |
| 17 | Fallback and Capture Verification | 3 | 3 | 0 | 1 | 0 |
| 18 | TD-1 Regulated Plan Review | 3 | 3 | 0 | 2 | 0 |
| 19 | TD-2 Consciousness Full-Context | 3 | 3 | 0 | 2 | 0 |
| 20 | TD-3 Switch Coherence | 3 | 3 | 0 | 2 | 0 |
| **total** | | 88 | 86 | 2 | 27 (distinct per blueprint) | 33 |

Per-step walk coverage (which legacy step went to which stage) is in each body's `walk_covered`; per-step legacy names/ids in each step's `name` / `legacy_step_ids`.

## Answers to the operator's questions
### bp 8 — Modality Pipeline Onboarding
**What it is:** a developer procedure, not a user task: the exact arc the text/code/math modality pipelines followed — implement the pipeline binary (Analyze / CreateGraph / QueryGraph with real ZSEI persistence), persist a real ModalityGraph container keyed by container id, link to existing graphs (hop-ordered, uncapped, bidirectional SimilarTo with provenance), and prove retrieval from a fresh process. Its methodologies are 30019, 30037, 30038, 30040 (the persistence / retrieval / linking rules).
**Why a request walk cannot do it:** every legacy step was `101` (the code-analysis pipeline used as a build/graph tool). A walk step is a model prompt or a registered-MCP call; nothing registered builds a Rust pipeline binary or runs a fresh-process retrieval test. So v2 keeps the four steps as pipeline-9 steps that produce the design, the persistence/link specification and the pass condition for the person doing the work, and records two missing capabilities (build-and-run a modality pipeline; fresh-process test). It will NOT make a pipeline.
**Recommendation:** keep it as a guidance blueprint (a request like 'onboard a new modality' gets the correct checklist and methodology rules), or retire it — nothing is lost either way because the arc is also carried by methodologies 30037/30038/30040. Operator decision; low cost to keep.

### bp 18-20 (TD-1 / TD-2 / TD-3) — what 'capture-assertion' means
Source: `docs/TOP_DOWN_REVIEW_GUIDE.md` §4. These are **test blueprints**, not user tasks: each runs a REAL `/orchestrate` request end to end and then asserts, **from the capture store** (`{data_dir}/model_calls/decision_review.jsonl`, written by `src/orchestrator/decision_review.rs`), that the reviewing model really received the context the doctrine claims:
- **TD-1:** a prompt matching a RequireConfirmation jurisdiction rule → the confirmation review's `reasoning_preview` must cite the blueprint's own step content (proof the plan was traversed, not a summary).
- **TD-2:** a consciousness-enabled run → the stage-8 review must reference BOTH AMT branch content and zero-shot-simulation risk/feasibility content.
- **TD-3:** the same regulated prompt twice (primary model vs a `model_config` override forcing the local BitNet-class fallback) → compare the two capture lines for decision agreement and for the fallback still citing the same context markers ('switch coherence').
**The 'mechanism' they presume (and say does not exist yet):** an automated *assertion step* — something that (1) submits `/orchestrate` (with an optional model override), (2) reads and locates the right capture-store line by timestamp/task, and (3) asserts that `reasoning_preview` contains concrete step / AMT / simulation text (and, for TD-3, compares two lines). In the legacy bodies those were prompt-pipeline steps (`submit`, `read_capture`, `assert`) — a model cannot truly read the file or call the host, so the assertion would be asserted from nothing.
**What exists today that could serve:** the capture store itself (real, written per review); `tools/e2e/lib.mjs` (`suite`, `http`, `pipeline`, `sessionToken`) and the existing e2e scripts (`jurisdiction.mjs`, `walk-completion.mjs`, `span-walk.mjs`, `fanout.mjs`, ...) which already drive real walks against a real host and read results; `decision_review.rs::read_jsonl` and the `/decision-review`-style readers in `grpc/mod.rs`.
**What building it would involve:** a small e2e script per case under `tools/e2e/` (e.g. `top-down.mjs`: TD-1/TD-2/TD-3 as `s.check(...)` rows): submit the request through `/orchestrate`, poll the task, read `decision_review.jsonl` filtered by task/timestamp, then assert deterministically that `reasoning_preview` contains substrings taken from the run's own blueprint steps / AMT branches / simulation risks (and, for TD-3, run twice with `model_config` override and compare decision + marker overlap). That is deterministic code, no model judge — the honest way to assert capture. It is NOT a blueprint step. Until then v2 keeps TD-1..3 as pipeline-9 procedure steps and records the missing capabilities (orchestrate-call, read repository/data files); they read as a documented procedure, they assert nothing by themselves.
**Options:** (a) keep as documented procedure blueprints (v2 state); (b) retire them from the built-in index and track the three cases as e2e scripts; (c) build the e2e script (recommended — it makes the doctrine's claim checkable). Operator decision.

### bp 9 — Jurisdiction Expansion, methodology asset 34
Asset **34 = 'Jurisdiction Source Maintenance Protocol'** (`assets/methodologies/method_34_jurisdiction_source_maintenance_protocol.json`; also on the EU at `/opt/ozone-studio/methodologies/`). The blueprint's `required_methodologies` are [21, 34]: 21 is the real-sourcing standard for jurisdiction content, 34 is the maintenance protocol — jurisdiction content is real external law that changes, so a **generated** `SOURCE_REGISTRY.md/.json` (via `tools/generate_jurisdiction_registry.py`, walking every file under `assets/jurisdiction/`) beats a hand-maintained summary; regenerate it after any content change; and a heuristic 'check the registry before an independent new search' (an already-approved source for the same country is a signal for the regulator domains/citation conventions to reuse).
**Why it has no container:** the file exists, but the bootstrap default methodology index in `src/bootstrap.rs` lists 49 of the 50 asset files — **34 is the one entry missing from the index** — so no container is ever created and `30034` does not exist on the EU. v2 therefore keeps it out of `methodology_ids` and records it under `methodology_assets_without_container: [34]`. Once the main session registers it (container `30034`), add `30034` to bp 9's `methodology_ids`. bp 9's step texts already follow its protocol (check the registry first; regenerate it).
**Also in bp 9:** register-scope / regenerate-registry / wire steps need file-write, script-run and wiring capabilities that are not registered → recorded in `missing_capabilities`.

## Verification (no build)
All 19 bodies: parse as JSON; every step has `step_index/action/description/pipeline_id` with correct types; `step_index` contiguous from 0; `depends_on` only earlier indices; non-provider `pipeline_id ∈ {9,56}` (all are 9 — no 56 used); provider steps: `provider = document-mcp`, `pipeline_id = 0`, `optional = true`, pinned tool ∈ {document_extract…} all present in the EU registry (`/opt/ozone-studio/zsei_data/mcp_tool_registry.json`, port 3282 → `mcp_supervisor::facts_for_port` = `document-mcp`); `missing_capabilities` are strings; every `methodology_ids` entry is a real EU container id; every legacy step id is accounted for (kept or walk-covered). Extra step keys (`name`, `legacy_step_ids`) are ignored by serde (`BlueprintStep` has no `deny_unknown_fields`).
**NOT verified:** deserialization by the compiled binary; any runtime behaviour; whether the provider call can derive the `path` argument for `document_extract` from the step text (args are null → the call is `{"query": <step text>}` and the tool requires `{path}`; the steps are `optional`, so if it cannot, they are skipped — NEEDS-OPERATOR to confirm or to pin real `args` per request); whether capturing 1-3 `missing_capabilities` per walk of these blueprints creates repeated tool requests (the capture dedupes by name, not checked).

## NEEDS-OPERATOR
- bp 1 / bp 6 legacy `dynamic` (per-modality) step: now walk-covered (Stage 4 builds the per-modality graphs); no per-modality blueprint step remains.
- Steps that were pure file/hook/index actions now deliver content in text and list a capability (FW / IDX / SBX / SCR / TST ...): confirm that is what you want captured as 'missing' on every walk (the alternative is to drop them from the blueprint).
- bp 8 and bp 18-20 options above.
- bp 9 option: step 'research_sources' could be the web-search pipeline 56, but 56 uses the whole step description as the search query and fails the step when search is unavailable — not used.
- `rule_refs` empty on all steps (not verified against the 38 regenerated bodies).
