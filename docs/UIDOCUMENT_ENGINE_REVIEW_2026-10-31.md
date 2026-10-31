# UiDocument engine review — measured on a live page, vs BS4 and vs domorpher (2026-10-31)

Captured from the post-Gate-3 review. Measurement is real (EU, www.google.com fetched
with curl, 69,178 bytes, run through the deployed `uidocument` binary and compared with
the standard-library HTML parser, which builds the same element tree BS4 builds; BS4
itself is not installed on either host). Decisions D23-D30 are closed; work items are
queued, none edited here (ZCode owns Gate 2 in `src/zsei/mod.rs` / `src/mcp_graph.rs`).

## 1. What was measured
| | Reference parser | Engine |
|---|---|---|
| Elements | 80 (23 distinct tags; 143 attributes) | all 80 present as nodes; nothing missed |
| Graph | n/a | 160 nodes, 25 edges, `truncated:false` |
| Node mix | n/a | StyleRule 66, Element 58, Link 11, ScriptRef 10, Field 9, Media 2, UiDocument 1, Form 1, StyleSheet 2 |
| Edges | n/a | Includes 11, FormOwns 9, StyledBy 5 (ChildOf is carried by `parent`, not an edge) |
| Flags | none | role 21, name 16, interactable 14, href 12, field_type 9, display_none 22, stacking_context 2 |
| Reasoner view | none | 1,005 chars |
| Geometry | none | 0 nodes with geometry (static input; reported absent, correct) |
| Cascade | none | **2 selector matches; 9 unsupported selectors** (note in `bounds`) |
Discrepancy to resolve: ZCode's Gate-3 report said "164 entities" for a 90 KB page; this
fetch is 69 KB / 80 elements / 160 nodes. Different fetches; not reconcilable from here.

## 2. Findings
- F1 Coverage is complete: every source element became a node.
- F2 The hand-written cascade/selector matcher is the weak part (2 matches of 66 rules;
  pseudo-classes and sibling combinators unsupported). 5 `StyledBy` edges for 66 rules.
- F3 `StyleRule` nodes are 41% of the graph; on heavily styled sites they would consume
  the 5,000-node cap before elements do.
- F4 `normalize_input` reads files with `read_to_string`: any non-UTF-8 file fails with a
  read error. Real bug (BS4 and the old domorpher both detected encodings).
- F5 `rule_hits` aggregation in `bounds` stays empty while per-node/per-edge provenance is
  real (display-only counting gap, already noted by ZCode).
- F6 On the live path the engine would recompute what the browser already knows: computed
  style, accessibility tree, layout, paint order.

## 3. Decisions (closed)
| # | Decision |
|---|---|
| D23 | **Nothing is dropped from the graph.** Style rules are stored losslessly in the `StyleSheet` node payload (object store above 64 KB). A `StyleRule` node + `StyledBy` edge is materialized when a rule matches an element (including later, on a live mutation). A standalone `.css` file keeps every rule as a node (no document to match). Separate node budgets per class (elements / style / scripts) so styles cannot crowd out elements. **Until matching is fixed, change nothing: keep all rules as nodes.** |
| D24 | Static path: replace the hand selector matcher with the `selectors` crate (the matcher behind `scraper`); keep `rules.json` on top. Static facts are marked `derived`. |
| D25 | Live path: do not recompute. Use the browser: `DOMSnapshot.captureSnapshot` for computed style subset, boxes and **paint order**, and `Accessibility.getFullAXTree` for role/name/description. Facts carry `source:"browser"` and confidence `certain`; `StyledBy` edges only for chosen nodes via `CSS.getMatchedStylesForNode`. `a11y.rs` stays as the static-only fallback. |
| D26 | Paint order from `DOMSnapshot` supplies z-order for `InFrontOf/Behind`; this closes G3 without observer changes. |
| D27 | SVG: add `svgtypes` (lengths, transforms, path data) and `kurbo` (analytic bezier bounding boxes, arc conversion) — curve bboxes become `certain`, ~300 lines of own geometry retire. `usvg` only if `use` instantiation / stroke-inclusive boxes / text are needed, and only as a lookup by element id (it normalizes the tree and can lose identity). Crate versions/APIs to be verified at the compile. |
| D28 | Encoding: read bytes and decode at the boundary with `encoding_rs` (+ `chardetng`); the Document node records `source_encoding` and detection confidence. Browser-sourced content arrives decoded. |
| D29 | Fix the `rule_hits` aggregation (counts onto the root node's bounds/metadata). |
| D30 | Positioning vs BS4: BS4 returns an in-memory tree (parse/search/edit); UiDocument returns provenance-tagged facts the host persists as containers/edges. Do not compete on parsing (`scraper` over html5ever is the standard parser). Our unique value: untrusted marking and the suspicious-instruction flag, rule provenance on every item, form/label/link edges, the compact reasoner view, live deltas with geometry, persistence into traversable context, the shared geometry layer. |

## 4. BS4 vs domorpher vs UiDocument
| | BS4 | Original domorpher | UiDocument |
|---|---|---|---|
| Is | tolerant parser + tree search/edit | LLM-driven autonomous web agent | rule-driven analyzer producing a graph |
| Decides/acts | no | yes (LLM) | no (Ozone decides; farm acts) |
| Semantics | none | DOM preprocessing for an LLM; interactable detection; UI patterns | roles, names, form ownership, links, visibility, untrusted marking, provenance |
| Live/geometry | no | partly via its own drivers | live observer with deltas and boxes (designed; not yet run on a real page) |
| Output | Python tree object | agent actions | graph nodes/edges + reasoner view |
| Persistence | none | its own storage | Ozone ZSEI containers with ripple |
Missing vs BS4 (ours to add or accept): encoding detection (F4), an ad-hoc query API
(covered by graph traversal), XML-mode parsing (SVG goes through the HTML parser).

## 5. Work queue (not started; ordered)
1. F4 encoding fix (D28) — small, real bug.
2. F5 `rule_hits` aggregation (D29).
3. Cascade: `selectors` crate; re-measure the 2-of-66 on the same page; then apply D23
   storage (matched-only nodes + payload).
4. SVG: `svgtypes` + `kurbo` (D27).
5. Live path: `DOMSnapshot` + AX tree through the farm bridge (D25/D26) — needs the bridge
   permission (operator item).
6. Per-class node budgets (D23).

## 6. Test rows (§24 of MASTER_TEST_GUIDE)
UE-ENC1 Latin-1 / Shift-JIS file ingests with `source_encoding` recorded · UE-ENC2 invalid
bytes degrade, never fail · UE-CSS1 re-measure google.com: selector matches rise from 2 and
unsupported falls from 9 · UE-CSS2 matched rule materializes `StyleRule` + `StyledBy`;
unmatched rule is recoverable from the StyleSheet payload (lossless round trip) ·
UE-CSS3 standalone `.css` keeps every rule as a node · UE-CAP1 page with 10k CSS rules
keeps all elements (separate budgets) · UE-HITS1 `rule_hits` counts equal per-node flags
sum · UE-SVG1 curve bbox equals analytic extrema (`certain`) · UE-LIVE1 browser computed
style/AX/paint order appear as `source:"browser"` facts · UE-LIVE2 paint order produces
`InFrontOf/Behind` · UE-CMP1 element coverage = 100% vs the reference parser on 5 real
pages (google, wikipedia, a form-heavy page, an SVG-heavy page, a table-heavy page).
