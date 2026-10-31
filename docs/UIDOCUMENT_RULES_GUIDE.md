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

### dom-live (a rendered snapshot, from the farm)
- *geometry*: boxes -> `Above`, `NearTo`, `Overlaps`
- *occlusion*: box + z-order -> `covered_by` (can the user actually click it)
- *scroll containers and fixed/sticky layers*
- *shadow DOM*: open roots are flattened with `Embeds`; closed roots are opaque
- *dynamic change*: two snapshots -> a graph delta (added/removed/changed)

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

## 5. What is deliberately NOT here
Forms filling, human-like input, session/cookie aging, navigation, extraction,
instruction parsing, multi-pass, recovery, schema, any model. Acting is the
farm and android tools; deciding is Ozone's reasoning; cookies are the farm's
persistent profiles. If one of these is later needed it is designed from a
need, not carried over.

## 6. Language and Python
Engine in Rust (a pipeline like the 82 others). A Python library is **deferred**:
the Browser-Farm already has its own human-like behaviors and persistent
profiles, and the farm only needs to hand over a snapshot. Revisit when a real
farm-side need appears; the language-tagging scheme (`TOOLS_BY_LANGUAGE.md`)
already allows it.

## 7. First build order (when the gate lifts)
1. Add html/css to `detect_modality`; stub the pipeline registry entry 127.
2. Rule-pack loader + html pack (structure, forms, links, a11y name, visibility).
3. css pack (stylesheet facts, cascade, computed visibility).
4. Ingest route for snapshots; farm snapshot via CDP `DOMSnapshot` (verify).
5. dom-live pack (geometry, occlusion); android-xml pack.
6. patterns pack; remove the JavaScript `graph` stopgaps.
Tests: UD1-UD12 plus one row per rule family (id = rule id prefix).
