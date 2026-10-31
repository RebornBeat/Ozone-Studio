# UiDocument rules guide (fresh start, 2026-10-31)

Operator direction: start fresh, Ozone-first; drop the baggage (forms,
interactive, agent, UI machinery); focus on **rules per format**; use this as
the example of how rules work for every modality. Design only; nothing built.

## 1. One modality or several? One, with a rule pack per flavor

`UiDocument` (pipeline 127) is a single modality. HTML, CSS, DOM snapshots,
Android view XML and accessibility trees are **flavors**. Each flavor =
one parser + one rule pack. Reasons:
1. They share one graph shape (a tree of elements with attributes, optional
   geometry and style) and one set of relations.
2. The valuable links cross flavors: CSS rule -> HTML element (`StyledBy`),
   Android WebView -> HTML document. Separate modalities would make those
   cross-pipeline links, which Ozone has no clean mechanism for.
3. One detect entry, one pipeline id, one ingest route, one test family.
4. A new flavor is a new pack plus a parser, no host change (the bridge
   expansion principle).
Split a flavor out only if it needs a different graph shape or an engine of a
different weight or license (SVG vector graphics stay with Image).

## 2. Rule packs are data, not code

Precedent already in the repo: jurisdiction rules (`assets/jurisdiction/*.json`)
and methodologies (`assets/methodologies/*.json`) are JSON data loaded by the
host. UiDocument rules follow that pattern so rules can be reviewed, versioned,
diffed and extended without recompiling the engine.

```
assets/pipelines/modalities/uidocument/rules/<flavor>.json
{
  "pack": {"id": "html", "version": "0.1.0", "flavor": "html", "spec_refs": ["WHATWG HTML", "HTML-AAM", "ARIA 1.2"]},
  "rules": [{
    "id": "html.form.label-for",
    "family": "forms",
    "when": {"selector": "label[for]", "target": "#{attr:for}"},
    "emit": {"edge": "LabelFor", "from": "$match", "to": "$target"},
    "confidence": "certain",
    "source": "WHATWG HTML 4.10.4",
    "rationale": "A label's for attribute names the control it labels."
  }]
}
```
- `when` uses a small declarative vocabulary: CSS-style selectors, attribute
  tests, role tests, ancestor/descendant tests, computed-style tests, geometry
  tests. No arbitrary code in a rule.
- `emit` is a node, an edge, or a flag. Nothing else.
- `confidence`: `certain` (a spec says so), `derived` (follows from certain
  facts, e.g. cascade result), `heuristic` (a pattern; must list `evidence`).
- **Absence is honest**: if a rule's facts are missing (no geometry, no style),
  it emits nothing. It never guesses.
- Every emitted item records its rule id, so traversal and review can say why.

## 3. Rule families per flavor

### html (static document)
- *structure*: `ChildOf`; landmarks from tags and ARIA roles; heading outline
- *semantics*: lists, tables (row/column/header relations), figures, details
- *forms*: `FormOwns` (including the `form` attribute), `LabelFor`, field type,
  required/min/max/pattern constraints as properties
- *links*: `LinksTo` with `rel`, target, download; same-page anchors
- *media and embeds*: images with alt, video/audio sources, `Frame` nodes;
  cross-origin frames become opaque stub nodes (honest)
- *metadata*: title, lang, canonical, viewport, robots, Open Graph
- *scripts*: `ScriptRef` (inline or src); content goes to the Code modality;
  never executed
- *accessible name and role*: accname computation and HTML-AAM role mapping
- *static visibility*: `hidden`, `aria-hidden`, `type=hidden`, `inert`
- *interactability*: native controls, `tabindex`, `contenteditable`, inline
  handler attributes
- *safety*: all text is `untrusted`; a flag `suspicious_instruction_text` on
  text that addresses an AI (flag only, nothing is removed)

### css (style layer)
- *stylesheet facts*: rules, selectors, specificity, `@media`, `@supports`,
  `@layer`, `@import`, custom properties, `@keyframes`, `@font-face`
- *cascade* (needs a document): origin, importance, layer, specificity, order ->
  the winning declaration per element property (`derived`)
- *computed visibility*: `display`, `visibility`, `opacity`, `clip`, off-screen
  positioning -> flag `visible` with the rule that decided it
- *stacking*: `position`, `z-index`, stacking contexts -> `InFrontOf` ordering
- *interaction effects*: `pointer-events`, `user-select`, `cursor`
- *generated content*: `::before/::after` text counted as text
- *responsive*: `@media` variants recorded as alternates, not merged
- A standalone `.css` file yields the StyleSheet/StyleRule subgraph only.

### dom-live (the LIVE DOM, handled in JS — not a picture, not a one-shot dump)
Correction (operator, 2026-10-31): my earlier "rendered snapshot with geometry"
wording was wrong. The DOM is not an image. It is a living tree inside the
page that scripts read and the page itself mutates. Pixels are the image
modality (102); this flavor is the tree, live.

How it works:
- A small **in-page JavaScript observer** (injected by the farm/CC bridge
  before page scripts, e.g. Playwright init script or CDP
  `Page.addScriptToEvaluateOnNewDocument`) walks the live DOM once and then
  streams **deltas** from `MutationObserver` (added / removed / attribute /
  text changes), plus on-demand queries: `getComputedStyle`,
  `getBoundingClientRect` (layout numbers the browser holds, not pixels),
  `elementFromPoint`, `document.activeElement`, shadow roots, same-origin
  frames. Event-listener inventory comes from CDP `DOMDebugger.getEventListeners`
  (listeners are not visible from plain DOM attributes).
- Batches go farm bridge -> ingest route -> pipeline 127. The first batch
  builds the base graph; later batches are **graph deltas** emitted through
  the existing GraphEventHub ripple, so the living graph in Ozone follows the
  living page.
- Rules apply to the stream: they re-evaluate only what a delta touched.

Rule families:
- *live structure*: delta -> `ChildOf` add/remove; node identity is stable
  across deltas (backend node id), so history is traceable
- *geometry (intrinsic, captured with every node)*: the observer records each
  node's box (`getBoundingClientRect`) when it is seen and keeps it current via
  ResizeObserver / IntersectionObserver / scroll / layout-affecting mutations.
  Relations (`Above`, `NearTo`, `Overlaps`, `Contains`, `InFrontOf`...) are
  computed centrally from `geom` + `frame` (SPATIAL_GEOMETRY_LAYER.md). Absent
  only when no layout engine ran, never guessed
- *occlusion*: `elementFromPoint` at an element's centre -> `covered_by`
- *real interactability*: listener inventory + not covered + visible +
  not `pointer-events:none` -> flag `actionable`
- *focus and state*: `activeElement`, `:checked`, `:disabled`, `value` length
  (never the value of password fields), scroll position
- *scroll containers, sticky/fixed layers*
- *shadow DOM*: open roots flattened with `Embeds`; closed roots opaque
- *frames*: same-origin walked; cross-origin = opaque stub node (honest)
- *change significance*: bursts of mutations (ads, tickers) are summarized
  into one delta with a count, so the graph and the ripple are not flooded
- *safety*: the observer is read-only: it never clicks, types, or evals
  caller-supplied code; page text in deltas is `untrusted`

### android-xml / a11y-json
- *view hierarchy*: class -> role map, resource-id, content-desc as name,
  `clickable/enabled/focusable/scrollable` flags, `bounds` -> spatial relations
- *WebView boundary*: a node that hands off to the html flavor
- *a11y-json*: the tree as given; role/name/state mapping

### patterns (heuristic, evidence required)
login form, search box, cookie/consent banner, pagination, modal dialog,
captcha presence (flag only, never solved), paywall, error/empty state. Each
pattern rule lists the facts that fired it (`evidence`) and maps to an
Affordance edge (F.3). These replace the old "interactive pattern library"
as data, not as machinery.

## 4. How other modalities use the same idea
The pack format (`when` / `emit` / `confidence` / `source` / `rationale`) is
not UiDocument-specific. It is a general way to state deterministic knowledge:
code (language lint and structure rules), chemistry (valence and bond rules),
network topology, jurisdiction (already data). UiDocument is the first example
because HTML/CSS/ARIA are public specs with clear rules. Each other modality
can adopt a `rules/` directory and the same loader.

## 4b. Geometry is a shared layer, not a UiDocument feature
See `docs/SPATIAL_GEOMETRY_LAYER.md`: all modalities contribute geometry in named frames; one engine computes relations; relations span graphs. UiDocument gains an `svg` flavor (exact lines/shapes) there.

## 5. What is deliberately NOT here
Forms filling, human-like input, session/cookie aging, navigation, extraction,
instruction parsing, multi-pass, recovery, schema, any model. Acting is the
farm and android tools; deciding is Ozone's reasoning; cookies are the farm's
persistent profiles. If one of these is later needed it is designed from a
need, not carried over.

## 6. Language and Python
Engine in Rust (a pipeline like the 82 others). The **in-page observer is JavaScript** (it must run inside the page), a small read-only script stored with the pipeline and injected by the farm; tagged `lang:javascript`, `kind:library` in the language registry. A Python library is **deferred**:
the Browser-Farm already has its own human-like behaviors and persistent
profiles, and the farm only needs to hand over a snapshot. Revisit when a real
farm-side need appears; the language-tagging scheme (`TOOLS_BY_LANGUAGE.md`)
already allows it.

## 7. First build order (when the gate lifts)
1. Add html/css to `detect_modality`; stub the pipeline registry entry 127.
2. Rule-pack loader + html pack (structure, forms, links, a11y name, visibility).
3. css pack (stylesheet facts, cascade, computed visibility).
4. Ingest route for batches (first full tree, then deltas); CDP `DOMSnapshot.captureSnapshot` may seed the first batch (verify), the in-page observer streams the rest.
5. dom-live pack (deltas, geometry, occlusion, actionable); android-xml pack.
6. patterns pack; remove the JavaScript `graph` stopgaps.
Tests: UD1-UD12 plus one row per rule family (id = rule id prefix).
