# UI/UX Retrospective (R1)

Full findings from the read-only review fork dispatched to audit the
entire UI/UX Fork Plan (Batches A–J, ~93 items) against the real code —
not the plan doc's own summaries. Back to [README](README.md) ·
[master guide](living-graph-field-guide.md).

## The design-token bug (highest priority)

`ui/src/App.css:20-29` defines a real token set on `:root`:

```css
--color-bg: #0a0f1c;
--color-text: ...
--color-accent: ...
/* (full set, lines 20-29) */
```

A grep across `ui/src` for the most common literals used in Batches C–J
(`#dfe7f2`, `#0a0f1a`, `#1e2836`, `#8b98ab`, `#c7d0dc`) returns **51 files
and 266 occurrences**. None of them reference the token — every one is a
hand-typed literal. The drift is the tell: `#0a0f1a` appears constantly,
one hex digit off from the real `#0a0f1c` token, which is what happens
when many different forks each eyeball a color from a screenshot rather
than importing a shared constant.

**Consequence:** every surface built in Batches C through J — the entire
Graph View, all seven new panels, the three new orchestration-visibility
components — is permanently dark. None of it can respond to a theme
switch, because none of it reads the variable that would make a switch
meaningful.

**Fix scope:** mechanical, not architectural. One pass: replace literals
with `var(--color-*)`; extend `:root` for the handful of tones that
genuinely have no existing token (one real gap found: a graph-canvas
border shade).

**Fixed, 2026-09-29:** all 266 occurrences across all 51 files replaced —
`#0a0f1a`→`var(--color-bg)`, `#dfe7f2`→`var(--color-text)`,
`#c7d0dc`→`var(--color-text-secondary)`, `#8b98ab`→`var(--color-text-muted)`,
`#1e2836`→`var(--color-border-faint)` (the one real missing token,
confirmed — no existing token matched this shade, added rather than forced
onto a different real value). The app's other ~35 distinct hex colors
(modality accent hues, status colors) were deliberately left as literals —
verified they're intentional fixed data-encoding colors, not theme chrome,
and this finding's own 266 figure was precisely these 5 colors, not the
whole palette. Verified with `tsc --strict` (zero errors on all 51 files)
and a full `vite build` (clean, only pre-existing bundle-size warnings).

## Stale documentation found

- `ui/src/components/GraphView.tsx:1-19` — the file's own header comment
  still describes "generic placeholder shapes," despite being wired to the
  real per-modality renderers (`graphRenderers/`, Batch C2–C7) since J2's
  integration pass. The comment predates the wiring and was never updated.
- `docs/UI_UX_FORK_PLAN.md` — the phrase "not yet exercised live" appears
  37 times. J5's real Electron E2E pass (launched under `xvfb-run` with a
  throwaway Playwright driver) actually verified 7 of those items live:
  Graph View, Fabric, Hierarchy, Raw Thoughts, Files, Engines, Coordination
  all rendered with zero console errors and real populated data on two
  surfaces. Those 7 occurrences are now stale text describing a state that
  no longer holds.

## Known data gaps — re-verified live, not assumed

Checked directly against the running host, not carried over from an
earlier session note:

- **`user_id` fallback**: `GetUserWorkspaces{user_id:0}` returns real data
  (`{Containers:[1081]}`); `user_id:1` — the value hardcoded across the
  UI — returns `{Containers:[]}`. Every project-scoped panel is correctly
  wired but shows an honest empty state because of this mismatch.
- **Zero `FileReference` containers anywhere.** `SearchContainersByKeywords`
  for `container_type:"FileReference"` returns empty. Files/Editor/
  Reference views have nothing real to render, through no fault of their
  own wiring.
- **`zsei_data/blueprints/` still only has `index.json`.** No individual
  blueprint content files exist on disk; any read of blueprint content
  returns a real storage error.

None of these three are code bugs. They're a fallback-value decision
(`user_id`, in `ui/src/services/store.ts`, predates this session) and
missing seed data respectively.

## Accessibility

Only **11 of 73** `.tsx` files under `ui/src` reference `aria-`, `role=`,
or `tabIndex` at all (~15%). Nothing built this session — the
orchestration status panel, discovery effects, the MCP activity indicator,
the graph canvas itself — has any keyboard-navigation or screen-reader
affordance.

## Bundle size

`npm run build` emits a warning that the main chunk exceeds 500KB after
Batches C–J landed. No `manualChunks` configuration exists in
`ui/vite.config` to split it.

## Every previously-flagged-not-fixed item, re-confirmed still open

- `EdgeLegendFilters.tsx`'s own documented limitation: its hidden-edge-type
  filter is keyed by type name only, so it collides across modalities
  (a "Contains" edge hidden for code also hides "Contains" for math/text).
- H4's AMT-lineage/task cross-reference gap: branch ids live in a separate
  id space from persisted AMT container ids.
- H5/H6: jurisdiction/simulation results are request-scoped only —
  discarded once the response returns, never persisted onto the durable
  task record.
- I5: the text-modality `CausedBy` direction-collapse bug at
  `assets/pipelines/modalities/text/main.rs:7243` — both "causes" and
  "causedby" prompt outputs collapse to the same stored edge.
- G6/J5: the page CSP (`ui/index.html`'s `default-src 'self'`) blocks the
  browser-fallback data path — harmless today since Electron never takes
  that path, but real if the plain-browser path is ever made first-class.

## Full prioritized punch list

See the [master punch list in living-graph-field-guide.md](living-graph-field-guide.md#master-punch-list)
for all 21 items in one ordered list, merging this fork's findings with
R2/R3/R5's.
