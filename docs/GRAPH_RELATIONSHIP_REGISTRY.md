# Graph Relationship Registry — nodes, edges, relationships, source of truth

> The single, current, source-verified inventory of every graph node type,
> edge type, and cross-cutting relationship mechanism in Ozone-Studio: what
> exists, what's real vs. schema-only, exactly where it's constructed (or
> isn't), and how to extend this to a new modality. Companion docs:
> `LIVING_GRAPH_STATUS.md` (doctrine-vs-implementation narrative status),
> `GRAPH_TEST_PLAN.md` (the test contract this registry feeds),
> `CONTEXT_REGISTRY.md` (S1-S12 context sources — §6 here maps them onto
> the graph), `BUILDER_REGISTRY.md`/`ZERO_SHOT_CALL_REGISTRY.md` (the
> functions and model calls that populate this graph).
>
> Built 2026-09-22 from a 6-fork parallel audit (code/math/text node-edge
> inventories, cross-cutting relationship structures, S1-S12 graph
> connectivity, future-modality template survey), each independently
> source-verified with file:line citations. Maintained additively per this
> repo's own convention — stale rows get corrected in place, not deleted.

---

## 1. Code modality (`assets/pipelines/modalities/code/main.rs`)

### CodeNodeType (enum, line 1018)

| Variant | Status | Constructor | Notes |
|---|---|---|---|
| File | Real | 2620-2622 | Root node per analyzed file |
| Function | Real | 2637-2639 | Per `CodeAnalysisResult.functions` entry |
| Class | Real | 2672-2674 | Per `CodeAnalysisResult.classes` entry |
| Import | Real | 2763-2765 | Per `CodeAnalysisResult.imports` entry |
| Module, Method, Variable, Type, Export, Parameter, Block | Schema-only | — | Data is extracted (`variables`/`type_definitions`/`exports` are real fields, populated) but never becomes a node. `Method` is the highest-value target: `ClassDef.methods: Vec<FunctionDef>` already holds data identical in shape to top-level functions. |

### CodeEdgeType (enum, line 1051)

| Variant | Status | Constructor | Derivation / limitation |
|---|---|---|---|
| Contains | Real | 2657-2663, 2693-2699 | File→Function, File→Class, unconditional |
| Imports | Real | 2782-2788 | File→Import, one per import statement |
| Extends | Real | 2709-2721 | Class→Class from `ClassDef.extends`. Same-file resolution only — bare identifier, not qualified path (comment at 2705-2708 states this) |
| Implements | Real | 2722-2731 | Class→Interface, same same-file-only limitation |
| Calls | Real | 2740-2757 | Function→Function from `analysis.function_calls`. Same-file resolution only (`extract_function_calls` only matches callees in the file's own function-name set). Carries `line`/`is_method` properties |
| Exports, DependsOn, References, TypeOf | Schema-only | — | Never constructed. `DependsOn` has a *separate*, unrelated `DependencyGraph`/`FileDependency` struct family for file-level deps, not wired to this enum |
| RelatesTo, SimilarTo, AlternativeTo, Refactors, Tests | Schema-only in this enum, by its own comment ("added by ZSEI") | — | Cross-container `SimilarTo` is real but lives in `link_related_containers` (§5), a separate mechanism from `CodeGraphEdge` |

**`AnalysisDepth`** real `#[default]` is `Standard`. Gating (line 1476): `extract_function_calls` runs for `Standard`/`Deep`, skipped only for `Surface` — confirmed fixed and live.

**Adjacent, not part of `CodeGraphEdge`**: `DependencyType` (line 1100, file-level deps), `ConflictType` (line 1180, provisional-change-conflict detection) — structurally separate from the AST graph.

**Bonus fix landed this session**: `extract_classes`'s per-language regex capture-group index was wrong for `extends`/`implements` (Java's extra "public " group was silently misread as the class name) — fixed alongside the edge-wiring.

**Highest-value next**: `Method` nodes — mechanical extension of the exact pattern already used for `Function`, would let `Calls`-edge resolution include method calls (likely the majority of real calls in OO code).

---

## 2. Math modality (`assets/pipelines/modalities/math/main.rs`)

### MathGraphNodeType (persisted graph nodes)

| Variant | Status | Constructor | Notes |
|---|---|---|---|
| Root | Real | `create_graph`, 2076-2085 | One per graph |
| ProofStep | Real | 2096-2110 | One per proof line |
| Variable | Real (`ParseExpression` path only) | 2237-2246 | The proof-side constructor (2184-2193) exists but is gated on `introduced_variables`, which `analyze_proof` always sets empty — unreachable via proofs |
| Assumption, Axiom, Theorem, Definition, Expression, Constant, Scope | Schema-only in practice | Constructors exist for Assumption (2158-2167) but gated on data `analyze_proof` never populates; Axiom/Theorem/Definition/Expression/Constant/Scope have **no constructor anywhere** | `axioms_used`/`theorems_used`/`definitions_used`/`scope_tree` always empty/None |

### MathEdgeType (persisted graph edges)

| Variant | Status | Constructor | Derivation / limitation |
|---|---|---|---|
| Contains | Real | 2113-2120, 2248-2255 | Root→step, root→variable |
| **FollowsStep** | Real, new this session | 2125-2138 (doc comment 1281-1285) | Unconditional step[i]→step[i-1] — a genuinely honest structural fact, always true by construction. Added specifically to stop conflating step-order with content-dependency |
| **Uses** | Real, fixed this session (was fabricated) | 2142-2153 | Only emitted when `extract_step_references` (1634-1673) finds a real citation: `step/eq/equation/result/assumption #N`, bare `(N)` mid-statement, or 6 implicit reference phrases, resolved only against real earlier step numbers. Previously hardcoded to `i-1` unconditionally — now empty when no evidence exists, never defaults |
| AssumesIn, DischargesIn, Defines | Schema-only in practice | Constructors exist (2169-2176, 2219-2226, 2195-2202) but gated on `step.assumptions`/`discharged_assumptions`/`introduced_variables`, which `analyze_proof` still hardcodes empty (lines 1824-1826) — **unchanged by this session's fix**, the highest-value next target (graph-construction side already works) |
| Derives, Requires, Implies, References, BindsVariable, UsesVariable, Generalizes, Specializes, Contradicts | Schema-only | — | Declared, zero producers |
| SimilarTo | Real, different layer | `link_related_containers` (§5) | Cross-container, unrelated to proof-internal edges |
| ImplementedBy, RepresentedBy | Real, manual-only | `LinkToModality` action | Explicit caller-supplied, not auto-discovered |

**Case/branch/piecewise**: confirmed still absent. `step.step_type` only ever assigns `Given`/`Conclusion`/`Deduction`; `CaseIntro`/`CaseAnalysis`/`InductionBase` (real `StepType` variants) are never assigned. `proof_technique` keyword-sniffs the whole proof into one top-level enum value — no per-step structure.

**`content_keywords` reaches proof analysis**: confirmed real (line 1872, `extract_content_keywords` over combined step text, merged into `derive_math_keywords` at line 114) — closes the gap where P4's isolation fix originally only benefited bare `ParseExpression` calls.

**Highest-value next**: making `Assumption`/introduced-`Variable`/discharged-`Assumption` real — the graph-construction code already exists and works (2158-2226), purely gated on `analyze_proof` never populating the source data. A real per-step extraction (regex or the E7 zero-shot expansion already specced in `ZERO_SHOT_EXPANSION_GUIDE.md`) would light up three dead edge types with zero graph-construction-side changes.

---

## 3. Text modality (`assets/pipelines/modalities/text/main.rs`)

**Three separate, non-overlapping relationship systems exist — not two, as first framed. This distinction matters for anyone building here.**

### System A — `TextGraphEdge`/`TextEdgeType` (in-graph structural edges, enum at 1930-1990, ~50 variants)

Exactly **one constructor**, `text_new_edge` (6949-6969), called at exactly 4 sites (6457, 6496, 6527, 6565) — **all four pass `TextEdgeType::Contains`**. Every other variant (`Follows`, `Precedes`, `References`, `Contradicts`, `Supports`, `Elaborates`, `Summarizes`, all cross-modality `DescribesX` variants, `Performs`, `Affects`, `Implies`, `TemporalPrecedes`, `TemporalFollows`, `CausedBy`, `Enables`, `Prevents`, `PartOf`, `HasPart`, `FunctionalRole`, `InstanceOf`, `HasInstance`, `SimilarTo`, `RelatesTo`, `DerivedFrom`, `VersionOf`, `RefinesTo`, `ForkedFrom`, and more) is **100% schema-only — never constructed as a real `TextGraphEdge` anywhere in this file, confirmed by exhaustive grep**, not merely unwired.

### System B — `TextGraphNode`/`TextNodeType` (enum at 1827-1845, 17 variants)

Only **5 are ever real**: `Document`(6414), `Section`(6438), `Entity`(6470), `Topic`(6509), `Keyword`(6540). The other 12 (`Paragraph`, `Sentence`, `Reference`, `Chunk`, `ModalityReference`, `TrueTextSpan`, `FileReference`, `ChunkReference`, `SupplementarySection`, `GrammarSubject`, `GrammarObject`, `InferredConcept`) are schema-only — some appear in filter predicates anticipating their existence, but nothing constructs them.

### System C — the "rich vocabulary" is NOT a graph-edge system at all

`ChunkGrammarRelationship` (1490-1500) — the struct the LLM's JSON populates at `extract_grammar_from_graphs`/`extract_grammar_relationships_from_text` — has `edge_type: String` (1493), a **raw string**, not `TextEdgeType`. It attaches to `SentenceAnalysis.relationships`, used for AMT/prompt-context building. Grep confirms **nothing in this file reads `.relationships` to construct a `TextGraphEdge`**. The second path doesn't produce a disconnected graph — it produces a parallel, string-typed, non-graph data structure that never becomes any real `TextEdgeType`/`TextGraphEdge`, anywhere. `CoreferenceChain`/`CoreferenceMention` (1168-1178) is the same shape: real struct, real data, never a graph edge.

`GrammarNodeType` (883-1052, ~158 variants — full clause/phrase/verb/noun taxonomy) is likewise never fed into `TextGraphNode`; 15 real construction sites, but they feed only grammar-tree-shaped output structures, disconnected from `TextGraph`.

`CrossModalityRelation` (2032+) is a second, overlapping enum duplicating names already in `TextEdgeType`'s cross-modality section — possible dead duplication, not traced further.

**Confirmed absent**: entity-to-entity relationships (no populated System-A variant), citation/cross-reference links (no `References`/`ReferencedBy` constructor).

**Confetti protection status** (7 of 13 `extract_json_from_response` call sites protected as of this session): protected — `extract_entities_from_text`, `extract_topics_attempt`, `extract_next_section_event`, `extract_grammar_relationships_from_text`, the sentence-grammar-tree and cross-sentence-relationships+coreference calls inside `extract_grammar_from_graphs`, `analyze_sentiment`. Still unprotected (6 sites, current line numbers): `extract_next_sentence`(2507), `extract_next_paragraph`(3204), `extract_next_modality`(3445), `clean_chunk`(3837), `list_sentences_for_paragraph`(5255), `parse_json_array`(7002, generic helper). These feed `Sentence`/`Paragraph`/`Chunk` boundaries and prompt text — since those node types are already schema-only (System B), these 6 sites are not currently the source of any real graph node/edge; their failure mode is chunking-quality/prompt-context degradation, not graph-edge loss.

**Highest-value next**: neither "bridge the two paths" nor "finish the 6 sites" is quite right — System C isn't a graph path at all. The real fix is making `text_new_edge` construct real edges from `ChunkGrammarRelationship`/`CoreferenceChain` data that's already being extracted (validate the string `edge_type` into a real `TextEdgeType` variant, resolve `from_text`/`to_text` to real node ids). This is a genuinely new build — there's no partial wiring to complete, the consumer doesn't exist yet.

---

## 4. Cross-cutting: `RelationType` (`src/types/container.rs:267-311`)

The canonical, host-side relation-type schema. **Important**: the 3 modality pipelines (and file-link pipelines) are separate compiled binaries that talk to the host over HTTP/JSON — they never share this Rust type at compile time. Every real producer writes `relation_type` as a JSON string literal matching a variant name, by convention, not by importing `RelationType::X`. `grep "RelationType::SimilarTo"` therefore finds **zero** hits despite `SimilarTo` being the single most common real edge in the live system.

| Variant | Discriminant | Status | Constructor |
|---|---|---|---|
| SimilarTo | 10 | **Real, highest-frequency edge in the system** | 3 independent copies of `link_related_containers`: `code/main.rs:580,586`, `math/main.rs:2555,2561`, `text/main.rs:444,450` |
| RelatedTo | 11 | Real | `lib.rs:134,149` (jurisdiction↔content), `jurisdiction.rs:615,630,647,915,929` |
| **ForkOf** | 60 | **Real, this session** | `amt.rs`'s `persist_amt_container` — written into the new fork container's own `Context.relationships` at creation |
| **ContinuedBy** | 61 | **Real, this session** | `amt.rs`, reverse edge into the prior container via read-modify-write, idempotency-guarded against duplicates |
| DependsOn, Contradicts | 1, 12 | Real, but as a *separate* `AMTRelationType` enum (AMT's own JSON blob), not this `RelationType` | `amt.rs:1135-1136,1977-1979` |
| ImportsFrom, ExportsTo, CallsTo, CalledBy, Implements, Extends | 20-25 | Real, but via code modality's own local `CodeEdgeType` (§1), not this enum | n/a |
| PartOf, Contains, Supersedes, Precedes, Follows, References, ReferencedBy, DocumentedAt, SourcedFrom, Custom | various | Schema-only | none confirmed |

### AMT lineage — now traversable

`ForkOf`/`ContinuedBy` are real, bidirectional, idempotency-guarded (checked against an existing `(ContinuedBy, new_id)` pair before insert). The pre-existing AMT-only `Continues` relation (separate enum, own JSON blob) is untouched — this is a pure addition. `TraversalEngine::structural_traversal` (`zsei/traversal.rs:154+`) already enqueues relation targets exactly like structural children, with zero type filtering — confirmed by reading the real loop (the file's own doc comment even documents this exact history: "Confirmed live 2026-09-15: none of the 6 traversal modes ever read `Context.relationships`... this is the fix"). So lineage rides this mechanism with **zero traversal-side changes required**.

### Jurisdiction

Still flat siblings under `JURISDICTION_ROOT_ID` — no meta-workspace hierarchy (UN→regional→national) edges beyond parent→root. `RelatedTo` edges connect *content* containers to jurisdiction scopes, not jurisdiction scopes to each other — that structural gap is unchanged. GAP-C1's keyword enrichment (`lib.rs`) is a real, separate, orthogonal fix (content-keyword richness on scope containers, no edge construction) — confirmed by re-read, not conflated with the structural gap.

**Proposed next (low-risk)**: a dedicated `RelationType::JurisdictionScope` edge from a content container directly to the specific rule/scope container that governs it, at the same call sites `RelatedTo` uses today. This is a rename/retype at existing, already-real call sites, not new discovery logic — and it closes "jurisdiction mixing is structurally indistinguishable from any other keyword-overlap edge."

### Coordination graph

`MirrorRequest` (`context_mirror.rs:30`) carries `kind: String` (free-text, not a typed enum: "note"/"decision"/"handoff"/"finding"/"claim"), scoped via keyword convention (`ws:<id>`/`proj:<id>`/`scope:global`). No evidence CoordinationEvent containers carry `Relation` edges to modality-content containers — it's a keyword-scoped island, discoverable only via keyword search/traversal, not explicit relationship edges to the content it discusses.

### Cross-modal linking

Confirmed 3 separate, independently-maintained copies of `link_related_containers` (`code/main.rs:375`, `math/main.rs:2350`, `text/main.rs:228`), identical signature, each independently constructing `SimilarTo` JSON. Purely lexical (keyword/topic overlap against a policy threshold) — no modality-type filter at the query level.

### Traversal modes (`zsei/traversal.rs`)

All 6 real, none dead stubs: `Structural` (parent/child + `Context.relationships`), `Semantic` (keyword/topic overlap), `Contextual`, `Hybrid` (literally composes Structural + Semantic), `BruteForce` (exhaustive scan) are all independently real. `MLGuided` checks for a real ML model file path; when absent (no trained model confirmed present today) it explicitly falls back to Hybrid per its own comment — the only mode without independent logic beyond its fallback.

---

## 5. Context-to-graph integration (S1-S12, per `CONTEXT_REGISTRY.md`)

Answers "how does context actually connect to the graph" — some sources are graph-native (live inside a traversable container), others are separate stores that may or may not bridge back.

| Source | Graph-native? | Mechanism | Consumed via |
|---|---|---|---|
| S1 Project main AMT | ✅ | ZSEI container + `amt-main` keyword; now also `ForkOf`/`ContinuedBy` edges | Direct fetch by known id, **not traversal** |
| S2 Per-request AMT fork | ✅ | Same container mechanism | Direct fetch |
| S3 Blueprint | ✅ | Real `container_type: "Blueprint"` container (`stages.rs:811,828`) | Parented to **root `0`, not the project/AMT container it belongs to** — no edge back to S1/S2 |
| S4 File graphs + relationships | ✅ | Real `SimilarTo` edges | **Real traversal** (`traverse_from_seeds`) |
| S5 Jurisdiction gate result | ❌ | Plain `OrchestrationState` field only | No container — dead-ends outside the single request/response |
| S6 Methodology rule text | ✅ | Real methodology containers | Direct fetch by id |
| S7 Simulation predictions | ❌ | Plain `OrchestrationState` field | No container — exists only for the request's lifetime |
| S8 Consciousness gate result | ❌ | Plain `OrchestrationState` field | No container — same dead-end pattern as S5/S7 |
| S9 Coordination events | ✅ | Real ZSEI containers under `/SharedContext` | Keyword-scoped, not edge-traversed |
| S10 Decision-review capture | ❌ (flat `decision_review.jsonl`) | **Real bridge**: `consciousness/review.rs`'s `read_capture_store` reads it, `persist_insight()` creates a real container under `CONSCIOUSNESS_METACOGNITION_ROOT_ID` | One-directional, insight-only (not every row becomes a container) |
| S11 General zero-shot capture | ❌ (flat `zero_shot_calls.jsonl`) | **Confirmed dead-end.** `amt_container_id`/`blueprint_id`/`project_id` markers (added this session) are write-only — zero readers anywhere join back on them | None — real correlation data, currently unused |

**Reality check on `traverse_from_seeds`**: only S4 (and S2 via seed ids) are actually pulled in through real graph traversal. S1/S3/S5/S6 are direct-fetch or state-thread-through, not discovery-by-edge-walk. Traversal is a thin slice of context assembly, not its backbone — most context arrives because the system already knows exactly which id to fetch.

**Most graph-isolated, highest-value targets**: S11 is the clearest waste — rich, real correlation data captured and never read back (a query joining `zero_shot_calls.jsonl` against its `amt_container_id`/`blueprint_id` would immediately answer "what did every model call for this AMT branch actually say," and nothing does this today). S5 (jurisdiction outcomes) is the best candidate to promote to a durable container — compliance-sensitive systems should keep enforcement history auditable, not request-scoped.

---

## 6. Extending to a new modality — the real template

`tools/PIPELINE_TRIAGE.md` shows ~25 modality pipelines sketched but not yet compiling (3D, BCI, CAD, IMU, audio, biology, chemistry, control, depth, dna, eeg, electromagnetic, geospatial, haptic, hyperspectral, image, network, proteomics, radar, thermal, video, kinematics, sonar, sound). These are a real, partially-built future roadmap, not empty stubs — most have a fully real `create_graph` already.

**Two design generations exist, confirmed by direct sampling** (image/audio/video/dna vs. 3D/network/geospatial/BCI):

- **Gen 1** (image, audio, video, dna, and similar): `{X}NodeType`/`{X}EdgeType` enums, `CrossModalityRelation`, `ZSEIHookType`, a real, substantive `create_graph` — but no provenance/lifecycle typing. Plain edges, no notion of "how sure are we this edge is right."
- **Gen 2** (3D, network, geospatial, BCI, and similar — evidently written later): everything Gen 1 has, **plus** a shared `ProvisionalStatus`/`ChangeType`/`EdgeProvenance`/`GraphStateType` vocabulary (locally duplicated per-file, not imported from a shared module — confirmed by grep, zero `use` statements pointing at a shared path, yet identical variant names/derives across all 4 sampled files — copy-paste convention, not inconsistency). **`EdgeProvenance`** in particular answers a question none of code/math/text (§1-3) ever asks: `Unknown, DerivedFromPrompt, DerivedFromChunk(u32), DerivedFromChunkGraph(u64), DerivedFromModalityGraph(u64), DerivedFromFile(String), DerivedFromAMT, DerivedFromBlueprint(u32), DerivedFromMethodology(u64), DerivedFromCrossModal, DerivedFromHook, VersionOf(u32), ForkedFrom(u64)`. Gen 2 also adds `{X}GraphQuery`, `{X}SemanticHook`, `{X}HeadlessOp`, `{X}ExportFormat` types absent from Gen 1.

**The real skeleton every modality file follows**: input/action enum (`{X}Action`) → analysis result struct → `{X}NodeType`/`{X}EdgeType` enums → `create_graph`/`update_graph`/`query_graph`/`get_graph` quartet → `CrossModalityRelation` (Gen 1) or `EdgeProvenance::DerivedFromCrossModal` (Gen 2) for inter-modality links → `ZSEIHookType` for semantic-enrichment callbacks.

**Template guidance for modality #4 (or beyond)**: follow Gen 2's shape and **reuse the `ProvisionalStatus`/`ChangeType`/`EdgeProvenance`/`GraphStateType` vocabulary verbatim** rather than re-deriving it — it's already proven out identically across 4 independent files. This also surfaces a real, separate future question worth a deliberate decision later: none of the 3 *live* modalities (code/math/text) have anything like `EdgeProvenance` — their edges carry only an ad hoc `discovered_via: String` at best. Retrofitting `EdgeProvenance` onto the live modalities is a real potential unification, not decided or scoped here.

**Why the stub pipelines don't compile**: not because the graph logic is fake — `create_graph` is real, substantive code in every sampled file. The triage's "unknown crates" column is mostly false-positive noise (std types the generator's regex can't distinguish from real external crates). By direct analogy to this session's `voice`/`whisper_rs` `DEP_TABLE` gap (found and fixed this session), a missing dependency-table entry for some real crate is a plausible primary suspect for at least some of these — not investigated further here, flagged for whoever picks up stub-pipeline work.

---

## 7. Prioritized next steps (consolidated from every section's "highest-value next")

Ordered by how close each is to real (mechanical wiring of already-real data first, new builds last):

1. **Math**: make `Assumption`/introduced-`Variable`/discharged-`Assumption` edges real — graph-construction code already works, purely gated on `analyze_proof` never populating source data. (§2)
2. **Code**: `Method` nodes — mechanical extension of the exact pattern already used for `Function`. (§1)
3. **Cross-cutting**: dedicated `RelationType::JurisdictionScope` — a rename/retype at existing real call sites, not new discovery logic. (§4)
4. **Context integration**: wire a reader for S11's already-written `amt_container_id`/`blueprint_id`/`project_id` correlation keys — the data exists, nothing joins on it. (§5)
5. **Code**: cross-file call resolution (specced as **E6** in `ZERO_SHOT_EXPANSION_GUIDE.md`). (§1)
6. **Math**: implicit/named proof-step reference resolution beyond regex (specced as **E7**). (§2)
7. **Text**: build the missing consumer — validate `ChunkGrammarRelationship`/`CoreferenceChain`'s string `edge_type` into real `TextEdgeType` variants and construct real edges via `text_new_edge`. This is a genuinely new build, not a wiring fix — there is no partial connection to complete. (§3)
8. **Future modalities**: evaluate retrofitting `EdgeProvenance`/`ProvisionalStatus` onto code/math/text for consistency with the Gen-2 template, before building modality #4 on the newer pattern while the 3 live ones stay on the older one. (§6)

---

## Registry-update rule

Every future node/edge/relationship addition MUST update this doc's relevant
section (real/schema-only status flips to real, with the new constructor's
file:line) — same discipline as `ZERO_SHOT_EXPANSION_GUIDE.md` §6. No
exceptions — this doc's entire value is being the place someone checks
*before* assuming a relationship type is populated.
