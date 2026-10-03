# ZCode UI/UX Feature Extraction (R3)

Full findings from the read-only review fork dispatched to break apart
`~/Projects/ZCode` (the open-source product repo — real UI source under
`packages/ui/src/`, ~40 feature directories) and identify what's genuinely
worth adopting into Ozone-Studio's UI. Back to [README](README.md) ·
[master guide](living-graph-field-guide.md).

**Note on naming:** this is the actual ZCode *product* repo, distinct
from the "ZCode" AI agent this session coordinates with via the shared
Ozone-Studio MCP server. Don't conflate the two.

## Starting point: Ozone-Studio's UI has zero component libraries

Confirmed by direct dependency inspection: beyond `react`, `zustand`, and
`electron`, Ozone-Studio's UI has no component, animation, charting, or
graph-layout library installed. Every graph layout, syntax highlighter,
and animation built this session was hand-rolled specifically because
nothing else was available. ZCode already made these exact library
choices, in most cases for the exact same problem.

## Real dependencies worth adopting

| Library | Replaces | Why |
|---|---|---|
| `@xyflow/react` (React Flow) | `ui/src/components/GraphView.tsx`'s synthetic circular layout | Real force-directed graph rendering. Highest single-item leverage in this list — GraphView's own header comment already flags the hand-rolled layout as a placeholder. |
| `shiki` | `ui/src/views/files/CodeViewer.tsx`'s regex tokenizer | CodeViewer's own header comment recommends a real highlighter; this session built a tokenizer from scratch specifically because none was installed. |
| `katex` | `ui/src/views/files/MathViewer.tsx`'s ASCII/LaTeX→Unicode converter | Same story — MathViewer's own comment names KaTeX as the real fix. |
| `motion` (Framer Motion) | Hand-written CSS `@keyframes` in `OrchestrationStatusPanel.tsx` / `DiscoveryEffects.tsx` | Real animation primitives instead of ad hoc keyframes per component. |
| `lucide-react` | Emoji tab icons in `CORE_TAB_DEFINITIONS` (`ui/src/pipeline-ui.tsx`) and every status icon built this session | Trivial swap, immediate visual-quality gain everywhere. |
| `radix-ui` + `class-variance-authority` + `clsx` + `tailwind-merge` | Inline-style-everywhere across every component built this session | Headless, accessible-by-default primitives — directly addresses the accessibility gap (11/73 files touch aria/role today). |
| `cmdk` | — | Pairs with a real ⌘K `CommandCenterDialog.tsx` ZCode already has; no current Ozone-Studio equivalent. |
| `react-resizable-panels` | — | Real resizable-panel primitive; no current Ozone-Studio equivalent. |
| `recharts` | `ui/src/views/capture/ReliabilityDashboard.tsx`'s hand-rolled inline-SVG bars | Real charting instead of manually placed rects. |
| `@xterm/xterm` | — | Only relevant if/when a real terminal surface is ever added; flagged, not currently needed. |

## The subagent directory panel

`packages/ui/src/app-shell/SubagentDirectorySidePane.tsx` (210 lines) +
`SubagentSessionSidePane.tsx` (61 lines) — the closest real analog to
Ozone-Studio's own `ui/src/views/coordination/ForkDispatchPanel.tsx`.

ZCode's version has a genuine 7-state status model — `running`, `waiting`,
`blocked`, `success`, `failed`, `cancelled`, `lost` — each with its own
distinct `lucide-react` icon. This is meaningfully richer than what
Ozone-Studio's backend currently supports: a separate finding (I4, from an
earlier batch) already confirmed **no live fork-status signal exists in
Ozone-Studio's backend at all** — task notifications are internal to the
CLI session and never persisted anywhere queryable.

**Implication:** ZCode's status model is the right target to build toward
once that backend gap is closed — not something to copy visually onto
data that doesn't exist yet. Adopting the UI pattern before the backend
signal exists would just be a richer-looking version of the same
fabricated-state problem this session spent real effort eliminating
elsewhere (the MCP ripple/jurisdiction fix).

Also worth reading for reusable primitives, not just the directory panel
itself: `AnimatedSidePanePanel.tsx`, `useAnimatedResizablePanel.ts` — a
real reusable animated-panel abstraction vs. the one-off CSS this session
wrote per component.

## Theming implementation

`packages/ui/src/useTheme.ts`:

```ts
type Theme = "light" | "dark" | "zai-light" | "zai-dark" | "system"
```

Resolved via `matchMedia("(prefers-color-scheme: dark)")`, applied via a
DOM attribute plus explicit `documentElement.style.colorScheme` and
`<meta name="theme-color">` syncing (for browser chrome), with a distinct
code path for Electron's vibrancy effects.

This is a clean, portable reference architecture — but it's only useful
*after* the design-token cleanup (see
[ui-ux-retrospective.md](ui-ux-retrospective.md)) lands. There's no theme
to switch to today, since most of the UI hardcodes literals instead of
reading the token variables a theme switch would change.

## Settings organization pattern — inconclusive

ZCode's settings organization was reviewed but the finding here is lower
confidence than the rest of this document; flagged for a follow-up look
rather than a firm recommendation.

## What NOT to adopt

Explicit honesty check from the review: most of ZCode's ~40 feature
directories have no Ozone-Studio analog at all and would be pure scope
creep if adopted wholesale — i18n, remote-connection, settings-sync,
git-graph, browser-use, a full terminal subsystem, and more. This document
only recommends the specific items listed above; it is not an endorsement
of importing ZCode's architecture generally.

## Recommended adoption order

1. `@xyflow/react` for Graph View — highest leverage, replaces an
   explicitly self-flagged limitation.
2. `shiki` + `katex` together — both already self-documented as the
   intended fix in their respective files' own comments.
3. `lucide-react` — trivial, immediate, no dependencies on other work.
4. `motion` — for the orchestration-visibility components.
5. The theming pattern — as reference architecture only, once the
   design-token cleanup gives it real tokens to switch between.
