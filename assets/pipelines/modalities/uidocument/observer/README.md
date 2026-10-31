# uidocument-observer (in-page JavaScript, read-only)

Status: WRITTEN, SYNTAX-CHECKED (`node --check`), NEVER RUN in a browser. The
live-DOM half of the UiDocument modality (pipeline 127): see
`docs/UIDOCUMENT_RULES_GUIDE.md` (dom-live) and `docs/SPATIAL_GEOMETRY_LAYER.md`.
Language tag: `lang:javascript`, `kind:library` (it runs inside the page).

## What it emits
`window.__uidocument = { start(sink), stop(), snapshot() }`. `sink(batch)` receives:

    { seq, full, nodes:[{id,parent,tag,attrs,text,geom,style,idx,...}],
      removed:[id], changed:[{id,attrs?,text?,geom?,style?,parent?,idx?,
      value_len?,checked?,in_view?,ratio?}], meta:{url,vw,vh,sx,sy,dpr,active,...} }

- First batch is `full:true` (the whole tree, stable ids). Later batches are deltas.
- A mutation burst over the record budget triggers one `full:true, resync:true`
  batch with `burst:{count,limit,overflow}` (consumer replaces its graph).
- `geom` is intrinsic and captured with every node: `{frame, x, y, w, h}` in
  scroll-adjusted CSS px (`frame:"page"`, or `"frame:<iframeNodeId>"`); `null`
  means the element has no box (display:none), never a guess.
- Geometry refreshes on ResizeObserver, IntersectionObserver, scroll (fixed /
  sticky / scroll containers), window resize, and layout-affecting mutations,
  through a bounded rolling queue (400 elements per flush).
- Shadow roots (open) are pierced and their nodes flagged `shadow:true`.
  Same-origin iframes are walked (`in_frame`); cross-origin / unloaded iframes
  are opaque stubs (`opaque:true`).
- Node budget 5,000 (the host graph cap): `truncated:true` when hit.
- Nodes in a batch may reference a parent that appears later in the same batch.
- Computed-style subset (`display, visibility, opacity, position, zIndex,
  pointerEvents, cursor, overflow, transform`) rides along because the css
  visibility/stacking rules need it.

## Read-only guarantees (checked by `selftest.mjs` against the source)
Never clicks/types/focuses/scrolls/navigates/evals, never creates or alters
DOM, no network, no storage, no cookies. Password inputs: nothing about the
value is read (not even its length). Other text controls: only `value.length`.
`value`/hidden-input attribute values, `on*` handler bodies and `data-*` values
are never emitted (names only). Page text is capped (200 chars/element; 64 KB
for `<style>`), whitespace-collapsed, and is untrusted data.

## Injection (done by the bridge, outside this repo)
The Browser-Farm CC bridge script is where this is wired; the Browser-Farm
repo is not edited by this change. Playwright (Python):

    page.expose_binding("__uidocument_emit", lambda source, batch: forward(batch))
    page.add_init_script(path="uidocument-observer.js")   # runs before page scripts

If `window.__uidocument_emit` exists when the script loads, it auto-starts with
it; otherwise call `window.__uidocument.start(fn)`. CDP equivalent:
`Page.addScriptToEvaluateOnNewDocument` + `Runtime.addBinding`. `forward(batch)`
sends the batch to Ozone's ingest route (strict gate), or runs
`uidocument.analyze_dict(...)` from `../py` locally.
Every navigation creates a new window: `seq` restarts at 1 with `full:true`.

## Known limits
- Closed shadow roots and attachShadow after start are not observed until a resync.
- Iframe contents: MutationObserver + load/scroll/resize refresh; no per-element
  Resize/IntersectionObserver inside iframes (different window).
- `changed.in_view` exists only for the first 1,000 observed elements.
- `.value.length` of a text control is the one permitted value access.

## Tests
`node selftest.mjs` (pure helpers + a source-level read-only guard). Not run yet.
