# Semantic Graph Reality Guide — all 27 modality pipelines, verified 2026-10-12

> This document is the ground-truth counterpart to the aspirational "OZONE STUDIO —
> Complete Modifications Guide v2.0" planning doc (pasted into a CC session
> 2026-10-12, not committed anywhere). Where that doc describes `CrossModalIndex`,
> `NodeCore`, a `ModalityEngine` trait, granular `EdgeProvenance`, etc., this doc
> states what's actually built, checked by direct code read (CC: 7-fork dispatch
> across all 27 pipelines + a dedicated shared-infra fork; ZCode: an independent
> parallel census across the same 27 pipelines, same day). Both audits converge on
> every substantive finding below — convergent numbers are stated once; any
> framing difference between the two is noted where it occurs. Nothing in this doc
> is aspirational — every row has file:line evidence behind it in CHECKLIST.md's
> 2026-10-12 entries and the shared-context finding notes (`mv62t4ei04ak`,
> `mv649yv3ojo1`, and the chain of `cc`/`zcode` notes preceding them).

## 1. The core finding

Every modality creates a real, typed internal domain graph. 20 of 27 modalities
use genuine zero-shot LLM calls to extract semantic structure. **Keywords are
always an extraction *output*, never a graph-construction *input*** — confirmed
true system-wide, text/code/math included. The actual problem was never "we need
embeddings" or "we need semantic search bolted on top" — it's that almost none of
this real semantic work ever leaves the modality's own private graph sidecar
file. Retrieval (`enrich_with_zsei_knowledge`, `find_methodologies_by_keywords`)
stays a flat keyword scan that never traverses any of it.

## 2. Per-modality capability matrix

| Modality (pipeline ID) | Zero-shot sites | Validator (`semantic_relations::`) called? | Edges gated on validator result? | Cross-modal link category | Writes `context.relationships`? | `trigger_semantic_hook` honest? | Zero-shot capture wired? |
|---|---|---|---|---|---|---|---|
| text (100) | many | yes | yes | **1 — fabricated stub** | yes (own `link_related_containers`) | yes (honest 0 when nothing done) | yes |
| code (101) | 0 (100% deterministic AST) | n/a | n/a | **2 — missing handler** | no | n/a | yes (dormant — no calls to capture) |
| image (102) | 0 | no | n/a | **1 — fabricated stub** | no | **NO — hardcoded 10/5/15** | n/a |
| audio (103) | 0 | no | n/a | **1 — fabricated stub** | no | **NO — hardcoded 8/4/12** | n/a |
| video (104) | 0 | no | n/a | **1 — fabricated stub** | no | **NO — hardcoded 12/6/18** | n/a |
| math (105) | 1 (`resolve_implicit_step_references`) | **no** (per ZCode census — the one zero-shot call has no validator gate) | n/a | **4 — real bidirectional** (the only correct example; non-idempotent, see §4) | yes (real) | yes (real counts) | yes |
| chemistry (106) | 0 | no | n/a | **1 — fabricated stub** | no | **NO — hardcoded 10/5/15** | n/a |
| dna (107) | 0 | no | n/a | **1 — fabricated stub** | no | **NO — hardcoded 12/6/18** | n/a |
| eeg (108) | 0 | no | n/a | **1 — fabricated stub** | no | **NO — hardcoded 15/8/22** | n/a |
| 3D (109) | 4 | yes | yes | **3 — real, one-sided/local-only** | no (zero `UpdateContainer` calls in the whole file; uses its own direct-`reqwest` persistence, not the shared helper) | yes | no |
| sound (110) | 6 | yes | yes | **2 — missing handler** | no | yes | no |
| biology (111) | 6 | yes | yes | **2 — missing handler** | no | yes | no |
| proteomics (112) | 5 | yes | yes | **2 — missing handler** | no | yes | no (also see §4 — fabricated science actions) |
| haptic (113) | 4 | yes | yes | **2 — missing handler** | no | yes | no |
| thermal (114) | 4 | yes | yes | **2 — missing handler** | no | yes | no |
| depth (115) | 4 | yes | yes (internally) | **2 — missing handler** | **NEVER A CONTAINER AT ALL** — see §4 | yes | no |
| IMU (116) | 2 | yes | yes | **2 — missing handler** | no | yes | no |
| geospatial (117) | 6 | yes | **no — validator called, result discarded** | **2 — missing handler** | no | yes | no |
| electromagnetic (118) | 4 | yes | **no — same ungated pattern** | **2 — missing handler** | no | yes | no |
| BCI (119) | 5 | yes | yes (correct) | **2 — missing handler** | no | yes | no |
| CAD (120) | 4 | yes | yes (correct) | **3 — real, one-sided/local-only** | no | yes | no |
| kinematics (121) | 2 | yes | yes (correct, instrumented) | **3 — real, one-sided/local-only** | no | yes (real validated-edge counts) | no |
| control (122) | 6 | **no validator call at all** | n/a (no gate exists) | **2 — missing handler** | no | yes | no |
| network (123) | 3 | yes | **CORRECTED 2026-10-12 — NOT gated.** Direct code read during the fix pass for control (below) found network logs `accepted`/`rejected` as side metadata but still pushes every proposed relation into real edges, filtered only by endpoint existence — the exact same ungated pattern as geospatial/electromagnetic, not the "correct" example this table originally claimed. BCI is the real correct-gating template. | **2 — missing handler** | no | yes | no |
| radar (124) | 5 | yes | not re-checked for this nuance | **2 — missing handler** | no | yes | no |
| sonar (125) | 5 | yes | not re-checked for this nuance | **2 — missing handler** | no | yes | no |
| hyperspectral (126) | 6 (`validate_mapped` variant) | yes | not re-checked for this nuance | **2 — missing handler** | no | yes | no |

## 3. The cross-modal-link taxonomy (all 27 accounted for)

| Category | Meaning | Modalities | Count |
|---|---|---|---|
| **1 — Fabricated stub** | Mints an id, claims success, writes nothing | text, image, audio, video, chemistry, dna, eeg | 7 |
| **2 — Missing handler** | The action doesn't exist in the enum; PASS 3's dispatch fails deserialization and is silently swallowed by the caller's `if let Ok(...)` | code, sound, biology, proteomics, haptic, thermal, depth, IMU, geospatial, electromagnetic, BCI, control, network, radar, sonar, hyperspectral | 16 |
| **3 — Real, one-sided/local-only** | Pushes a real `CrossModalRef` node + typed edge into its *own* graph, referencing the target by id; never calls `zsei_query` against the target, never touches `context.relationships` | 3D, CAD, kinematics | 3 |
| **4 — Real bidirectional** | The one fully correct example — both containers updated via real `UpdateContainer` calls | math (non-idempotent, see §4) | 1 |

Only math does this right, and even math has a bug (duplicate edges on repeated walks).

**Inferred consequence (flagged, not re-verified against the exact loop code this
pass)**: the confirmed "PASS 3's until-stable loop always burns all 30 passes"
bug requires at least one Category-1 modality among the verified set in a given
walk — Category-1's fake-success return is what resets the stability counter.
Category-2 modalities return a real `Err` that's swallowed before reaching the
truthy check, so they likely don't trigger the burn on their own. Needs direct
verification before being treated as certain.

## 4. Severe fabrication / architecture findings (beyond the cross-modal taxonomy)

1. **Proteomics' `PredictStructure`/`DockLigand` (`proteomics/main.rs:2141-2174`)
   return hardcoded fake science as `success: true`** — `plddt_mean: 0.87`,
   `docking_score: -8.5` kcal/mol, a fixed secondary-structure split
   (0.30/0.25/0.35/0.10) that never varies by input. Explicitly commented as
   stand-ins for real AlphaFold2/ESMFold and AutoDock Vina/GNINA/Glide calls.
   **The most severe finding in this audit** — a direct no-fabricated-metrics
   violation, presented as a successful real computation.
2. **Sound's `classify_call_types_llm` (`sound/main.rs:1037`) feeds a fabricated
   placeholder into a real zero-shot call** — `"species_hint":
   vocalization_id % 3`, literally commented "placeholder grouping" — tainting a
   real call's output with fake input data.
3. **Depth modality's graphs never enter the ZSEI Container system at all**
   (`depth/main.rs`, exhaustive grep, zero `zsei_query` calls anywhere). It
   writes raw JSON straight to disk via `std::fs` at
   `{zsei_path}/local/depth_graph_{id}.json`, bypassing `global.mmap`, the
   parent/child structural graph, and every search/traversal path that exists —
   the same "dangling local-state" shape the project's own 231-container purge
   (mentioned in ZCode's census) was built to catch. The `semantic_fingerprint`/
   `embedding` fields don't even exist in this file — there's no container
   schema to populate, because depth never creates one.
4. **Math's `add_relationship` (`math/main.rs:3277`) is not idempotent** — no
   existing-edge check before push. Combined with text's Category-1 stub (when
   both are verified in a walk), produces up to ~29 duplicate edge pairs per
   walk, confirmed earlier this session.
5. **Image/audio/video/dna/eeg/chemistry's `trigger_semantic_hook` returns
   hardcoded non-zero fake metrics** (e.g. image: `nodes_processed: 10,
   edges_added: 5, annotations_added: 15`) — each with a comment admitting "In
   production, this would call the ZSEI hook processor." This is the exact
   pattern math's own code comment says was de-fabricated in math; the fix was
   never propagated to these 6.

## 5. Validator-gating inconsistency (new finding — the old "16-modality group" was not uniform)

- **Gate correctly** (edges only come from the validator's accepted set, confirmed
  via explicit code comments or direct read): BCI, CAD, kinematics.
- **Call the validator but discard the result** — rejected relations still become
  real edges: geospatial (fixed 2026-10-12), electromagnetic (fixed 2026-10-12),
  **network (corrected finding — previously misclassified as "gates correctly"
  in this table; direct code read during the control fix pass found it's
  ungated, same as geospatial/electromagnetic — not yet fixed).**
- **No validator call at all** — raw LLM JSON accepted with only an
  endpoint-existence check: control (fixed 2026-10-12 — module wasn't even
  included before; now gates via `semantic_relations::validate_structural`,
  using BCI as the correct-gating template since network turned out not to be
  one).
- Not re-checked for this specific nuance (validator presence confirmed, gating
  behavior not): radar, sonar, hyperspectral, 3D, sound, biology, proteomics,
  haptic, thermal, depth, IMU.

## 6. The "shared store" question — answered directly

The pasted planning doc's `CrossModalIndex` (with its four lookup maps),
`NodeCore`, granular `EdgeProvenance`, and `ModalityEngine` trait are **100%
aspirational — none exist in real code.** The only trace of the planned
`CrossModalIndex` is a reserved-but-empty bootstrap container
(`ContainerType::CrossModalIndexRoot = 242`, fixed id 77, created at
`bootstrap.rs:654-657`) — zero children ever created under it. `NodeCore` exists
only as a dead local struct in `text/main.rs:707` (the compiler's own dead-code
warning confirms its sole constructor is unused).

**What IS real and shared, three layers (per ZCode's framing, confirmed by CC's
independent fork):**
1. **Shared code modules** in `assets/pipelines/shared/`: `capture.rs` (110
   lines, the zero-shot audit-trail logger), `semantic_relations.rs` (313 lines,
   the verbatim-evidence validator — confirmed wired into 18+ modalities),
   `context_assemble.rs` (143 lines), `ozone_serve.rs` (211 lines). All
   textually `#[path="../../shared/X.rs"]`-included (not a Cargo dependency —
   pipeline binaries are deliberately excluded from the workspace) by all 27
   modality `main.rs` files.
2. **The universal per-container `Context` schema** — `categories`,
   `methodologies`, `keywords`, `topics`, `relationships`,
   `learned_associations`, `embedding`, plus the `integrity` block's
   `semantic_fingerprint` — carried identically by every modality's container.
   This is the real "shared semantic store" the operator remembered; each
   modality's own domain graph is depth layered *on top of* this shared shape,
   not a replacement for it.
3. **Domain-graph sidecars** (`graphs/<type>_<id>.json`,
   `local/<type>_graph_<id>.json`) — per-modality, not shared, but structurally
   consistent across modalities.

**The capture gap**: `capture.rs`'s JSONL audit trail is only ever called by
text (multiple sites), math (1 site), and code (1 site, dormant — its "capture"
is presently just a comment). **Roughly 20 other modalities make dozens of real
`llm_zero_shot` call sites with zero logging anywhere** — `GRAPH_AUTHORING_GUIDE.md`
§9's claim that "every call is logged" is false for pipeline-internal calls.

Cross-checked against the project's own docs: `GRAPH_RELATIONSHIP_REGISTRY.md:181`
and `LIVING_GRAPH_STATUS.md:173` already independently state cross-modal linking
is "purely lexical... no semantic understanding" at the container level —
consistent with every finding here.

## 7. Plan v2.0 vs. reality (per ZCode's independent census)

**Built**: post-creation file-graph classification (§C.1.1 of the pasted plan —
metadata-only, exactly as the plan corrects itself to), graph-first `Traverse`
linking (text/code/math, floor-differential scoring triplicated identically —
independently verified by CC), the `GraphState` machine on the orchestrator side,
5x-consecutive-stable validation (`text/main.rs:4416` and sibling "5-YES" loops),
`NodeCore` (partial, text-only, dead), `EdgeProvenance`, `ChangeType`.

**Not built**: the `CrossModalIndex` struct, the `ModalityEngine` trait, real
semantic hooks for the 6 old-wave fabricated modalities, per-node
`materialized_path`, hotness as a real consumed signal (write-side exists,
nothing reads it yet).

**One live architectural conflict, needs an operator decision, not a unilateral
pick**: the pasted plan removes `confidence: f32` everywhere in favor of 5x-stable
validation as the sole correctness mechanism. The real codebase keeps
`confidence` everywhere — but its actual convention (`1.0` = "evidence-verified,"
not a continuous semantic probability) is defensible on its own terms. Either
amend the plan to match reality's convention, or decide to actually strip
`confidence` project-wide — this doc doesn't make that call.

**One superseded design, not a regression**: the plan's §C.2 LLM-based chunk
overlap resolution was replaced by chunk-once-with-carry-state-machines-owning-
boundaries — a better design than the plan's own proposal, and the code records
why (the plan's overlap approach duplicated content and forced an LLM boundary
call for no real benefit).

## 8. The fix ladder (priority order, both audits converge on this ordering)

1. **Fabrication fixes** (correctness, not new capability): proteomics'
   `PredictStructure`/`DockLigand`; sound's fabricated `species_hint`; the 6
   hardcoded-fake `trigger_semantic_hook` metrics (image/audio/video/dna/eeg/
   chemistry) → honest zero or real counts, matching the pattern math's own
   hooks and kinematics already use correctly.
2. **The 7 Category-1 fabricated `link_to_modality` stubs** (text + the 6
   above) → real, idempotent, bidirectional writes matching math's pattern
   (minus math's own non-idempotence bug — fix that too, see #4).
3. **control's missing validator gate**; **geospatial/electromagnetic's
   decorative gate** → make rejection actually block edge creation.
4. **Math's `add_relationship` idempotency fix**, then a one-time dedupe of
   already-duplicated edges from prior walks.
5. **Depth's complete bypass of the Container system** → wire its graph
   creation through `CreateContainer`/`UpdateContainer` so it finally enters
   the traversable fabric at all.
6. **Capture-store wiring**: `capture::capture_zero_shot_call` into the ~20
   modalities making real zero-shot calls with zero logging today (3D, sound,
   biology, proteomics, haptic, thermal, depth, IMU, geospatial,
   electromagnetic, BCI, CAD, kinematics, control, network, radar, sonar,
   hyperspectral) — cheapest, highest-value, makes every subsequent decision
   measurable instead of guessed.
7. **Validator wiring into the 6 zero-zero-shot old-wave modalities**
   (audio/chemistry/dna/eeg/image/video) — NOTE: this is not a "wire the
   validator" fix like #3, because these 6 have *no* zero-shot relation
   inference at all yet. This is a net-new feature build (add the whole
   `infer_semantic_relationships_validated` pattern from scratch, following the
   new-wave modalities as a template), not a bounded bug fix — scoped
   separately, deliberately excluded from the immediate fix-ladder edit pass.
8. **Registry-gate PASS 3** (`src/orchestrator/graphs.rs`) so the cross-modal
   dispatch only targets modalities that actually have a `LinkToModality`
   handler — removes the wasted calls to the 16 Category-2 modalities and the
   swallowed-`Err` noise.
9. **Architecture decisions, operator-gated, not unilaterally built**: what to
   do with Category 2's 16 modalities (build real bidirectional links for all
   of them vs. deliberately scope cross-modal linking down to semantically
   meaningful pairs only); what to do with Category 3's 3 one-sided
   annotations (promote to real bidirectional, or accept as the intended
   design); whether to build the planned `CrossModalIndex` for real or
   formally abandon it now that the reserved root container's purpose is
   understood.
10. **Retrieval** (once the above makes the edge store trustworthy): wire
    seed-keyword → one bounded structural-traversal hop into
    `enrich_with_zsei_knowledge`, with provenance carried into the injected
    context. Embeddings last, only for the true zero-lexical-overlap residual.

## 9. Finalized decisions (2026-10-13, operator delegated)

The architecture decisions §8.9 left open have been finalized:

- **Category 2 (16 modalities with no `LinkToModality` handler at all)**: scoped
  out, not built. The registry-gate fix in `graphs.rs` already stops the wasted
  dispatch calls; building 16 new cross-modal pairings is a real feature
  project with its own per-pair semantic design work, not a bounded fix.
  Revisit per-pair as concrete use cases emerge.
- **Category 3 (3D/CAD/kinematics' one-sided local annotations)**: upgraded to
  real bidirectional writes (matching the Category-1 fix pattern) — see the
  2026-10-13 edit-wave CHECKLIST entry.
- **`CrossModalIndex`**: formally abandoned. Real per-container
  `context.relationships` (now mostly correct after two edit waves) already
  serves the purpose; a separate four-map index duplicates state for no
  demonstrated benefit. The reserved root container (id 77) stays reserved but
  unused — harmless, not worth reclaiming.
- **The confidence-field conflict**: keep `confidence: f32` under reality's
  existing "1.0 = evidence-verified" convention. Not ripping it out
  project-wide — no justified benefit identified for that scope of refactor.
- **Deliberately still deferred** (not forgotten, not in scope for the
  2026-10-13 edit wave): the AMT cross-reference wave-barrier retrofit onto
  `dispatch_bounded` (performance, not correctness — amt.rs is already
  sensitive, kept out to bound this wave's blast radius); wiring
  `enrich_with_zsei_knowledge`'s seed→one-hop traversal (a substantial
  orchestrator-core feature add, deserves its own focused pass once the data
  it would read is proven correct by an actual build+test, not stacked
  unverified on top of unverified); the one-time dedupe of duplicate
  math↔text/math↔code edges already written into **production** ZSEI
  containers (a data-mutation against live storage, a different risk class
  than source edits — needs explicit operator go-ahead before anything runs
  against real data); TextContext's dead schema population (net-new capability
  build, not a fix).

## 10. Status

This document reflects source as read 2026-10-12. Edits against §8's ladder are
being tracked additively in `CHECKLIST.md` (dated entries) and shared-context
finding notes as they land — this file is a snapshot, not a live dashboard;
re-verify against source before trusting a specific line number months later.
