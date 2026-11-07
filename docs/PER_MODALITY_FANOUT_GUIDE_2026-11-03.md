# Per-modality fan-out — the rule of the call graph, for every modality (2026-11-03)

Operator direction: *span-out must follow the structure each modality actually captured — for code that is
the call graph with its entry points; every other modality needs the equivalent rule, and the code
workspace, 3D, CAD and their geometry must be tested with it.*

## 1. What was wrong (verified on real data, not assumed)

| Finding | Evidence |
|---|---|
| Span-out walked only container→container `context.relationships` + parent/child ids. A modality graph's **graph file** (`graphs/<modality>_<id>.json`: `nodes`, `edges`, `analysis`) was never opened for traversal — only text-ish keys (`text`, `summary`…) were read. | `src/orchestrator/span_out.rs::read_content_text`; code graph `code_40432.json` has `analysis.entry_points=['main','helper_parse']`, `call_sequence`, 9 `Calls` edges — none reached a step's context. |
| `relation_weight` knew ~25 generic names; every modality-specific edge type (`ParametricDrivesFeature`, `JointConstrains`, `RoutedThrough`, `Activates` …) fell to one flat 0.6. | survey of 27 modality edge enums (§3). |
| Entry points existed (code `is_entry_point`, `entry_points`, `call_sequence`) but nothing seeded from them or walked in call order. | `assets/pipelines/modalities/code/main.rs:883-922,1520,2699`. |

## 2. The rule (implemented in `src/orchestrator/graph_fanout.rs`, used by `span_out.rs`)

1. **Edge classes** — every edge type of every modality is classified:
   `Order 0.95 · Structure 0.90 · Flow 0.88 · CrossModal 0.85 · Spatial 0.75 · Similarity 0.70 · Other 0.60 · Provenance 0.35`.
   *Flow* = directed dependency / data / control / causal (`Calls`, `Imports`, `ParametricDrivesFeature`, `FeatureCreatesBody`,
   `Controls`, `FeedbackFrom`, `RoutedThrough`, `Activates`, `TranscribesTo`, `Animates`, `JointConstrains`, `BondedTo` …).
   *Order* = sequence (`Follows`, `Precedes`, `FollowedByJoint`, `TransitionsTo` …). *Structure* = containment
   (`Contains`, `PartOf`, `ChainContainsLink`, `HasMaterial`, `ParentFeature` …). *CrossModal* = leaves the modality graph
   (`DescribedBy`, `ImplementedInCode`, `ParametricGeneratesMesh`, `TranscribedAs`, `PlottedOnGeoMap`, `Feeds*` …).
   New edge types that follow the naming convention land in the right class automatically (prefix families), not a flat default.
2. **Entries** — explicit when the pipeline recorded them (`analysis.entry_points`: code), otherwise the *roots*:
   nodes with no inbound Structure edge; if everything is structural, nodes with no inbound Flow/Order edge.
3. **Sequence** — breadth-first from the entries over Flow/Order edges (Structure after), class-ranked: the
   "call sequence" of that modality (code: caller→callee; CAD: sketch→feature→body; kinematics: base→end-effector;
   network: source→hops; text: Follows chain).
4. **Digest** packed by span-out *most important first, whole lines only*:
   `ENTRIES` → Flow/Order lines in sequence order → node facts (type, name, properties) → Structure → CrossModal →
   Spatial → Similarity → Other → Provenance. Stops when the token budget is full (no arbitrary count cap).
5. **Container relations** use the same classifier (`relation_weight` falls through to `classify`), so a
   cross-modal link between two modality graphs (CAD→3D `ParametricGeneratesMesh`) is followed at 0.85, lineage at 0.35.

Span-out items now carry `fanout {modality, nodes, edges, entries, sequence_head, class_counts, lines_total, lines_packed}`
— the evidence that a graph was fanned out, not flat-rendered.

## 3. Per-modality capture — what exists today (from the survey + `zsei_data/graphs`)

| Modality | Graph file node types seen | Edge types seen | Entry rule | Flow edges that drive the walk | Honest status |
|---|---|---|---|---|---|
| code | File, Function, Class, Import | Contains, **Calls**, Imports | `analysis.entry_points` (main-like, zero in-degree) | Calls, Imports, Extends, Implements, DependsOn | **rich** (41 graphs, 34 Calls) |
| CAD | CADDocument, PartNode (feature tree: sketch/feature/body/face in the pipeline) | Contains, ParametricGeneratesMesh (+34 CADEdgeType) | roots of Contains (document/assembly) | ParametricDrivesFeature, DrivenBySketch, FeatureCreates/ConsumesBody, MatesWith, ConstrainedBy | live graph small (1 sample); pipeline enum is rich — STEP ingest of the clock will exercise it |
| 3D | Scene, Object | Contains (+45 ThreeDEdgeType: HasMaterial, Animates, CollidesWith, Above, NearTo …) | Scene root | Animates, Drives, DrivenBy, Deforms, Constrains, Simulates | geometry facts as node props; hierarchy via Contains |
| kinematics | KinematicsSystem, KinematicChainNode, LinkNode, JointNode | Contains, ChainContainsLink, JointConstrains, AnimatesViaBone, FeedsControlSystem | system root | FollowedByJoint, JointConstrains, DrivesJoint (base→end-effector) | rich |
| control | ControlSystem | (none captured in sample) | system root | Controls, FeedbackFrom, FeedforwardTo, ClosedLoopWith | **sparse capture** |
| network | NetworkScene | (none in sample) | zero in-degree (gateways) | RoutedThrough, ForwardedBy, TransmitsTo, DependsOnService | **sparse capture** |
| text | Document, Topic, Keyword, Entity | Contains (+47 TextEdgeType) | Document roots | Follows/Precedes (Order), Supports, Elaborates, Causes | wide, shallow (Contains only so far) |
| image | Image, Region, Object, Color | Contains, Overlaps | Image roots | (spatial) Above/Below/AdjacentTo | spatial-first |
| chemistry | Molecule, Atom, FunctionalGroup | Contains, BondedTo | Molecule | BondedTo, ReactsWith, Produces | rich |
| dna | Sequence, Gene, Exon | Contains | Sequence | TranscribesTo, TranslatesTo, Regulates, SplicedTo | structure only so far |
| biology / proteomics | BiologyScene, RegulatoryEdgeNode / ProteomeRoot | Contains, Inhibits | scene/root | Activates, Inhibits, Catalyzes, Phosphorylates … | partial |
| geospatial | GeoScene, RegionNode | Contains | scene | RoutesThrough, ConnectsTo, TrajectoryPassesThrough | spatial |
| math | Variable, Root | Contains | Root | Derives, Implies, Requires, DischargesIn | variables only |
| eeg / audio / video / sound / sonar / radar / thermal / IMU / haptic / depth / hyperspectral / EM / BCI | scene + event nodes | mostly Contains (+ a few typed) | scene root | Precedes/Follows, DetectedAtRange, TracksTarget, ThermallyCoupledTo, FeedsControlSystem … | **many pipelines emit scene nodes with empty edge lists** — fan-out can only be as rich as capture; tracked as a capture gap, not hidden |

## 4. Tests

| ID | What | Where |
|---|---|---|
| FO-U1 | classifier maps every modality family correctly (code, CAD, 3D, kinematics, control, network, text, bio/chem, geo, provenance) | `graph_fanout::tests::classifies_edges_of_every_modality_family` |
| FO-U2 | code digest starts at entries, follows `Calls` in order, reports unreachable last | `…::code_graph_digest_starts_at_entries_and_follows_the_call_chain` |
| FO-U3 | no explicit entries → structural roots; CAD sketch→feature flow line; class counts | `…::graph_without_explicit_entries_uses_structural_roots` |
| FO-U4 | empty/nodeless graph yields no digest (no fabricated structure) | `…::empty_or_nodeless_graph_has_no_digest` |
| FO-E1 | **code workspace E2E**: ingest a real multi-file project, `POST /context/span_out` seeded on its code graphs → items carry `fanout.entries` + `sequence_head` that match the real call graph | `tools/e2e/fanout.mjs` |
| FO-E2 | **CAD/3D E2E**: Ozone (via `/orchestrate`) generates the clock through its 3D/CAD tools; the outputs are ingested by the 3D/CAD modality pipelines; span-out from the CAD graph reaches the 3D graph via `ParametricGeneratesMesh` | `tools/e2e/clock.mjs` + `fanout.mjs` |
| FO-E3 | coverage report: for every graph file in `zsei_data/graphs`, share of edges whose class is not `Other` (target ≥ 95%) and list of modalities with empty edge lists | `tools/e2e/fanout.mjs --coverage` |

## 4b. Results so far (real host, 2026-11-04)

- FO-E1 code workspace (`tools/e2e/fanout.mjs`): **13/13** — entries, entry-rooted call sequence, Rust chain, orphan reported, and
  **cross-file calls** (`parse -Calls-> util.tokenize`, import-resolved external node; the code pipeline emits
  `FunctionCall.module` and an external Function node `<module>.<symbol>`).
- Real bug found + fixed by that test: the code analyser ended every Python function at EOF (brace-only block finder) and attributed
  calls to the first function in the list, counting `def` lines as self-calls → now indentation-aware, innermost caller, declarations skipped.
- FO-E3 coverage (`tools/e2e/fanout-coverage.mjs`, 281 graph files): every graph that has an owning container is fanned out;
  edge classification 100% except thermal (40% → classifier extended). The classifier now also has a keyword/shape tail
  (`heuristic_class`) and a unit test that classifies **all 562 edge names of all modality enums** (≤2% Other).
  Caveat (honest): the tail treats an unknown *typed domain relation* (verb / `…By|To|Of|With|From|In|At`) as Flow (0.88) rather than the flat
  0.6 — a deliberate choice that a typed relation carries context; refine per modality as real data shows over-weighting.
- 30 graph files have no owning container (25 code) — data-hygiene leftovers, tracked, not a fan-out defect.
- Capture gaps (graphs with ZERO edges): audio, control, electromagnetic, haptic, hyperspectral, network, proteomics, radar, video
  (and 54/176 text graphs, 1–2 of code/dna). Fan-out is ready; those pipelines need to emit typed edges.

## 5. Open items (captured, not hidden)

- Sparse-capture modalities (see 4b) — separate work: pipelines should emit their typed edges (the classifier is ready for them).
- Inbound edges (callers of X, "who contains me") are only reachable if the neighbour stores them; the digest lists both directions of
  a graph file, but container-level relationships remain source-stored.
- Keyword rediscovery seeds (0.5) still sit beside entry seeds; entries of the step's own graphs seed at 1.0 via the modality/file graphs.
