# DOMorpher — clean slate (2026-10-31)

> **UPDATE 2026-10-31 (operator decisions):** start fresh instead of keeping a 'true core'. Forms, interactive, agent machinery and the domorpher-agent MCP are dropped, not ported; domorpher-py is deferred; no domorpher-cli. The modality decision stands (one `UiDocument`, pipeline 127, flavors as rule packs) and moved to `assets/pipelines/modalities/uidocument/README.md` + `docs/UIDOCUMENT_RULES_GUIDE.md`. Sections 6 (retire list) remains accurate; section 7 (core/Python layout) is superseded.


Operator questions answered here: do we truly understand what domorpher is;
are we treating it as if a modality wraps it; does the graph need a DOM
modality; what about HTML and CSS; what is retired; what is the true core for
the Rust and Python rebuild. Design and capture only. Nothing built or run.
Supersedes the module dispositions in `DOMORPHER_CAPABILITY_PARITY.md` and the
"IN core" lists in `DOMORPHER_CONVERSION_GUIDE.md` where they differ.

## 1. What domorpher is, honestly

Its own README: an autonomous agent where "a large language model directly
understands and navigates the DOM". Natural-language objective in, LLM reads
the DOM, decides, acts, extracts. Its Technical_Document lists the parts:
DOM Preprocessor, Chunking Engine, LLM Integration Layer, Instruction Parser,
Adaptive Execution Engine, Result Reconciliation, Schema Enforcement.

Split by who does the job in Ozone:

| Domorpher part | Is it domorpher's unique value? | Ozone's owner |
|---|---|---|
| LLM integration, instruction parser, agent loop, reasoning, goals, memory | no, generic agent plumbing | pipeline 9, AMT, context graph |
| Navigation strategies (breadth/depth/semantic/structure-first) | no | context traversal (`traversal_ml`, `context_aggregation`) |
| Chunking, extraction, reconciliation, schema | no | text/code pipelines, zero-shot (S11), `consensus`, `integrity_check` |
| Multi-pass, recovery, engine | no | `decision_review`, TaskFailed/McpFailure actors, orchestrator |
| API, enterprise, monitoring, storage, pipeline, parallel | no | the Ozone host |
| **DOM preprocessing: DOM -> compact semantic view for a reasoner** | **yes** | nothing equivalent exists |
| **Accessibility tree and structure analysis** | **yes** | none |
| **Interactable detection and UI pattern library** (login, search, cookie banner, pagination) | **yes** | none (visual-mcp covers pixels, not DOM) |
| **Form mechanics and human-like input** | **yes** | none |
| **Session/cookie state transforms** | partly | the farm's persistent profiles do the storage |

So domorpher's real contribution is the **sense organ and the hands**: turn a
page into something a reasoner can read, say what can be done on it, and
perform inputs convincingly. Ozone is the brain. This is the point of the whole
rebuild, and it is much smaller than the 146k-line tree.

## 2. Are we treating it as a modality? What the graph does today

- Today domorpher is treated as (a) a 16-verb tool surface (`domorpher-agent`,
  unused) and (b) a library whose "DOM-traversable field" is a key in the farm's
  shared state (`dom-field/google`, which holds only a label).
- The graph step is ad hoc: `graph` blocks were just added inside the
  JavaScript `browser_snapshot` and `android_ui_snapshot` handlers. Each MCP
  computes its own graph shape. That is the "recreating" smell in miniature.
- **HTML, CSS and the DOM have no modality.** `ModalityType` (types/modality.rs)
  has Text, Code, Image, ... Hyperspectral (pipeline 126) and Unknown.
  The extension tables list no `html`, `htm`, `css` or `scss`: a saved web
  page or stylesheet is detected as **Unknown**. The affinity guide says web
  pages arrive "via FileReference + text", so the page's structure is lost
  before reasoning ever sees it.

## 3. Do we need a DOM modality? Yes: one, general, small

A modality in Ozone is: a data type, a detect rule, a pipeline that analyzes it
by itself first, and a graph shape. Web pages meet that bar and are not
covered by Text or Code (a DOM is a tree with geometry, roles and style, not
prose or source).

**Proposal: one modality, `UiDocument`, pipeline id 127** (next free id; 126
is Hyperspectral). It covers three flavors of the same thing, a tree of
interface elements with attributes and optional geometry:
1. `html`: an HTML file, or a live DOM snapshot from the farm or browser-mcp
2. `android-xml`: the `uiautomator` dump android-mcp already takes (a view
   hierarchy with bounds is a DOM)
3. `a11y-json`: an accessibility tree (future: desktop apps)

CSS is **not** a separate modality. It is the style layer of the same
document: see 5.

Why general rather than "WebDocument": the mobile app testing the operator
wants (MOBILE_ANDROID_GUIDE) needs the same graph shape as web pages, and
`graph` blocks for both were just hand-built twice in JavaScript. One pipeline
replaces both.

### Run-vs-tools, applied (the existing doctrine)
The modality **runs itself first** on the content (owning analysis -> base
graph). Tools run on top as guests: farm and browser-mcp supply the live
snapshot and execute actions; they do not each build their own graph. The
JavaScript `graph` blocks added this week become a stopgap to remove once the
pipeline exists.

## 4. Graph shape (reuses what the host already supports)

Nodes (labels from page content are untrusted data, X2 rule, display-only):
`Document(url,title,lang)`, `Landmark/Region`, `Element(tag,role,name,text)`
with flags `interactable`, `visible`, `Form`, `Field(type,required)`,
`Link(href)`, `Media(src)`, `Frame`, `StyleSheet`, `StyleRule(selector,props)`,
`ScriptRef` (script **source goes to the Code modality; it is never executed**).

Edges: `ChildOf`, `LabelFor`, `FormOwns`, `LinksTo`, `Controls` (aria-controls),
`Embeds` (frame), `StyledBy` (rule -> matched element), and the spatial
relations the `mcp_graph` hook already knows: `Above`, `NearTo`, `InFrontOf`,
`Overlaps` (from boxes and z-order). No new edge types are needed in the host.

Reasoner view: `semantic_markdown` (domorpher's compact DOM -> text) is stored
on the Document node as the LLM-ready rendering. Context traversal and pipeline
9 read that, not raw HTML.

## 5. HTML and CSS specifically

**HTML**: parse (html5ever/scraper, already dependencies) -> tree -> nodes and
edges above. A static `.html` has structure but no geometry or computed style.

**CSS**, three levels, each honest about what it can know:
1. *Stylesheet facts (static)*: rules, selectors, specificity, custom
   properties, `@media`, `@keyframes`, `@font-face`, `@import`. A standalone
   `.css` file produces a StyleSheet graph of these. Uses `cssparser`, already
   a dependency (accessibility.rs imports it).
2. *Cascade (needs a document)*: which rule wins per element
   (specificity, order, `!important`); gives `display:none`, `visibility`,
   `position`, `z-index` -> the `visible` flag and `InFrontOf`. Static, no
   browser needed.
3. *Layout (needs a render)*: real boxes only come from a browser. The farm
   supplies them in one CDP call: `DOMSnapshot.captureSnapshot` returns nodes,
   computed-style subset and layout bounds together (design assumption to verify
   at the gate). That feeds `Above/NearTo/Overlaps`. Without a render, the graph
   simply has no spatial edges: absent, never guessed.

**Routing**: `.html/.htm/.xhtml` and `.css/.scss/.less` detect as `UiDocument`
(today Unknown). `.js/.ts` stay Code. `.svg` stays Image. A page's inline
`<script>` text is forwarded to the Code modality as a child container.

## 6. RETIRED — the full list (nothing is dropped; each names its owner)

Whole-module sizes from the reference tree (`src/`); the reference copy stays
untouched as history.

| Module (lines) | Disposition | Owner / reason |
|---|---|---|
| `agent/agent, reasoning, goals, memory` | RETIRE | pipeline 9, AMT, context graph |
| `agent/navigation/*` (4 strategies, ~4.3k) | RETIRE | context traversal |
| `execution/engine, multi_pass, recovery` | RETIRE | orchestrator, `decision_review`, actors |
| `execution/actions` (1,576), `evaluator` (1,011) | EVALUATE | action application is the farm's; evaluator is Ozone's verification; keep only types if the verbs need them |
| `extraction/*` (4,255) | RETIRE | text/code pipelines, S11, `text_analysis` |
| `instruction/*` (4,801) | RETIRE | pipeline 9 + `task_recommendation` |
| `schema/*` (8,287), `reconciliation/*` (3,258) | RETIRE | `integrity_check`, `consensus`, graph merge |
| `chunking/*` (3,739) | RETIRED earlier | text pipeline 100 |
| `llm/*` (7,097) | RETIRE | Ozone model layer |
| `api/ enterprise/ monitoring/ storage/ pipeline/ parallel/` (41,758) | RETIRE | Ozone host surfaces |
| `browser/puppeteer, headless, javascript` | RETIRE | the farm / browser-mcp drive Chromium |
| `browser/driver.rs` (traits) | RETIRE | no driver abstraction in the library |
| `extension/` (6,971) | RETIRE (was parked) | the farm's CC bridge script is the in-page agent |
| `visual/screenshot` | RETIRE | `farm_screenshot` / `browser_screenshot` |
| `visual/analyzer, enhancement, multimodal` (~6.6k together with screenshot) | RETIRE | image modality (102) + visual-mcp |
| `utils/http, logging, tokenizer` | RETIRE | host; tiktoken is a model-vendor dependency |
| `lib.rs`, `main.rs` | RETIRE | new crate roots |

## 7. The TRUE core (what the Rust and Python rebuild contains)

| Module | Lines (ref, whole module) | What it is | Keep because |
|---|---|---|---|
| `dom/preprocessor, semantic_markdown, accessibility, analyzer, converter` | 8,531 | DOM -> clean tree, compact reasoner view, a11y tree, structure facts | the sense organ |
| `interactive/detector, events, patterns, library` | 6,484 | what is clickable/typeable, inferred listeners, UI pattern library | the affordance map (F.3 Affordance edges) |
| `forms/analyzer, validation, filler (mechanics), multi_step` | 5,005 | form structure, constraints, human-like input timing, applying a mapping Ozone supplies | the hands |
| `browser/cookies` (757), `auth` (845) state only; **`session.rs` is an EMPTY file (0 lines) in the reference tree, so session aging must be written, not harvested** | 1,602 | cookie/session state transforms, aging | farm profiles store; this knows the shape |
| `utils/encoding` (404), `config` (849) | 1,253 | charset detection, config types | needed by dom |
| `agent/action.rs` types | 2,622 | command vocabulary, if the verbs need it | decide at the census |
| `agent/perception.rs` | 2,369 | DOM -> perceived elements | probably duplicates `interactive/`; merge or drop at the census |
| **NEW `style/`** | new, small | stylesheet parse + cascade (level 1-2 in 5) | CSS has no module in the tree to keep; this is new work, not recreation |

Whole-module total of the keepers, before trimming: 27,866 lines
(8,531 + 6,484 + 5,005 + 1,602 + 1,253 + 2,622 action + 2,369 perception),
against the 146k tree. These are reference-tree sizes, not compiled results;
the census gives the real number. Session aging (AG4) is not harvestable
because `session.rs` is empty; the farm's persistent profiles already provide
the behavior and the library only needs the cookie-age transform.

### Rust layout
```
domorpher-core            lib: dom, interactive, forms (mechanics), state, style
                          no network, no model, no persistence, no runtime
assets/pipelines/modalities/uidocument/   (id 127, Ozone pipeline crate)
                          depends on domorpher-core by path
                          one-shot: --input {data,context} -> graph JSON
                          this binary IS the CLI (no separate domorpher-cli)
```

### Python layout
```
domorpher-py              PyO3/maturin wheel over domorpher-core (farm venv)
                          dom_to_field(html|snapshot) -> field
                          perceive(field) -> affordances
                          plan_input(text, profile) -> human-like timings
                          cookie_age(state, now)
                          uidocument_graph(snapshot) -> same graph JSON as pipeline 127
```
Same Rust code behind both, so the farm and the Ozone pipeline cannot
disagree. The farm imports it only from its CC bridge script; the Browser-Farm
repo is not edited.

### Data flow
```
CC/Ozone reasoning decides -> farm_navigate (gated /mcp/call)
  -> farm renders; CC bridge collects DOMSnapshot (nodes+styles+boxes) + screenshot
  -> ingest route (new, like /capture/ingest_image) -> entry gate
  -> pipeline 127 runs itself -> UiDocument graph (+ semantic_markdown on Document)
  -> context traversal / reasoning reads it -> decision as a command
  -> farm executes (plan_input timings from domorpher-py)
```

## 8. What this changes in earlier work
- The 7-fork rewrites of reasoning/engine/multi_pass/recovery/extractor are
  already retired from the core list (3d92f69); with this document, the
  `action/perception` pair and `execution/actions,evaluator` move to
  "evaluate at the census".
- T-DOM-GOOGLE (REVIEW_OZONE_STATE §4) re-points: D1 captures the real
  snapshot, D2 runs **pipeline 127 / the core offline** on it, D3 is the
  pipeline's graph output (no separate step).
- The JavaScript `graph` blocks in `browser_snapshot` and `android_ui_snapshot`
  are a stopgap; converge on one source (the pipeline).
- TOOLS_BY_LANGUAGE Python API gains `uidocument_graph`.

## 9. Test rows (UiDocument) — wired into MASTER_TEST_GUIDE §19
| id | check | expected |
|---|---|---|
| UD1 | `detect_modality("page.html")`, `("style.css")` | UiDocument (not Unknown); `.js` still Code |
| UD2 | pipeline 127 on the captured google.com HTML | nodes: search input, Search and "I'm Feeling Lucky" buttons, links; `ChildOf` tree; no script executed |
| UD3 | same page with the live in-page observer | full tree then deltas; adds `visible`, `Above/NearTo/Overlaps`, `actionable`; static HTML alone adds none |
| UD4 | standalone `.css` | StyleSheet/StyleRule nodes; media/keyframes/custom properties captured |
| UD5 | cascade | `display:none` element has `visible:false`; higher specificity wins |
| UD6 | android `uiautomator` dump | same node/edge shape as UD2 (flavor `android-xml`) |
| UD7 | hostile page text ("ignore previous instructions") | stored as untrusted display-only label; never reaches a prompt as an instruction |
| UD8 | `<script>` content | forwarded to Code modality as a child; never run |
| UD9 | `semantic_markdown` on the Document node | compact view present, token-bounded, stable |
| UD10 | Python wheel vs pipeline on the same snapshot | identical graph JSON |
| UD11 | AG6 on domorpher-core | no network/model/persistence crates; `cargo tree` clean |
| UD12 | JS graph-block stopgap removed after UD2/UD6 pass | `browser_snapshot` returns the pipeline's block or none |

## 10. Decisions for the operator
1. Approve one general modality `UiDocument` (pipeline 127), covering HTML,
   CSS-as-style-layer, Android view XML and a11y trees.
2. Approve routing `.html/.htm/.xhtml/.css/.scss/.less` to it.
3. Confirm the retire list in 6 (especially `extension/` retired and
   `visual/` fully handed to the image modality and visual-mcp).
4. Confirm there is no separate `domorpher-cli`: the pipeline binary is the CLI.
