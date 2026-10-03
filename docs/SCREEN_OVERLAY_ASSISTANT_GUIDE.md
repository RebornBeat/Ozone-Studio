# The Screen-Overlay Assistant — design, not built

> Fourteenth doctrine doc. Scoped explicitly as design-only per the
> operator's own answer earlier this session, when asked how to approach
> a screen-overlay assistant "like Gemini": **"design.plan but take into
> account the assistant is built around this — we already have
> everything in where we can create... but now we have a meta loop, a
> personal assistant aware of timers, due dates, all left... in our
> Universal Order — pushing us or reminding us... anything left or
> things coming up... things left to finish."** This is the precise,
> real scope: **not** continuous screen capture/vision understanding —
> a persistent, always-visible overlay window that surfaces the Universal
> Order's own real due-date/reminder state and the Acting Loop's real
> findings, using mechanisms that are now genuinely real and running
> (`docs/UNIVERSAL_ORDER_GUIDE.md`, `docs/ACTING_LOOP_GUIDE.md` — both
> built and live this session).

---

## 1. What this explicitly is NOT

Stated first because "screen overlay like Gemini" invites a much bigger,
different, privacy-sensitive feature the operator did not ask for:

- **NOT continuous screen capture.** No `desktopCapturer` polling, no
  frame-by-frame vision analysis of what's on screen. `visual-mcp`'s own
  doctrine (`TOOLS_REGISTRY.md` §3) already draws this exact line: "the
  MCP never execs screen-grabbers; sanctioned sources are the Ozone
  Electron app's own `desktopCapturer` (explicit user action), the xdg
  portal with explicit approval, or explicit X11 opt-in" — this doc does
  not touch that boundary or ask for anything beyond it.
- **NOT a new capture/vision pipeline.** Everything this overlay shows is
  DERIVED from data that already exists and is already real:
  `/order/global`'s computed buckets, the living graph's own ripple, the
  Acting Loop's real `amt_candidates` findings, the I-Loop's real
  reflections. No new sensing, only new PRESENTATION of what's already
  tracked.
- **NOT a second window into everything.** A focused nudge surface —
  what's overdue, what's coming up, what the Acting Loop just found — not
  a miniature copy of the full app.

## 2. What's real today to build on (verified, not assumed)

- **`/order/global`** (`src/grpc/mod.rs`) already computes real
  overdue/today/week/upcoming/someday buckets from `due_at` at read time
  — the exact data this overlay surfaces, already proven end-to-end this
  session (`ui/src/components/OrderPanel.tsx`, real task 84 round-tripped
  correctly).
- **The living graph's ripple** (`src/graph_events.rs`'s `GraphEventHub`)
  is the real, already-running mechanism this session's Acting Loop build
  proved wakes on every real task/coordination/tool-call event — the
  SAME signal this overlay subscribes to for "something changed, refresh
  the nudge" rather than polling on a blind interval.
- **The Acting Loop's real findings** (`src/orchestrator/actors.rs`, built
  and live this session) — a `TaskFailure`/`SecurityFinding`/
  `CoordinationReceived` candidate is a genuine, concrete "something
  happened" signal this overlay can surface directly, not a placeholder.
- **The I-Loop's real reflections** (`src/orchestrator/i_loop.rs`, built
  and live this session, currently gated off by
  `consciousness.enabled=false`) — when consciousness is on, a real
  periodic self-reflection persisted as a discoverable
  `CoordinationEvent` (`kind:"i_loop_reflection"`) is exactly the kind of
  "personal assistant" content the operator described — this overlay is
  the natural place to surface one when a fresh one lands.
- **The real device-pairing + bearer-token architecture**
  (`ui/src/ozoneClient.ts`'s own header, confirmed real this session
  while scoping the mobile app) — already proven for LAN access; the
  overlay reuses the SAME session mechanism the main Electron window
  already uses (`ensureSessionToken` in `ui/electron/main.js:378`), not a
  new auth path.
- **A real, working Electron `BrowserWindow` precedent**
  (`ui/electron/main.js:331`, `createWindow()`) — the overlay is a SECOND
  `BrowserWindow` in the SAME existing Electron app (same main process,
  same session token, same IPC/preload conventions already proven), not a
  separate application to stand up and authenticate independently.

## 3. The design

### 3.1 The window itself

A second `BrowserWindow`, created alongside the existing `mainWindow` in
`ui/electron/main.js`, with real, standard Electron overlay properties:

```js
overlayWindow = new BrowserWindow({
  width: 340, height: 220,
  x: <persisted or screen-right-anchored>, y: <persisted or screen-top-anchored>,
  frame: false,
  transparent: true,
  alwaysOnTop: true,
  skipTaskbar: true,
  resizable: false,
  focusable: false,        // never steals focus from whatever the operator is doing
  webPreferences: { nodeIntegration: false, contextIsolation: true, preload: <same preload.js> },
});
overlayWindow.setIgnoreMouseEvents(true, { forward: true }); // click-through by default
```

**Click-through by default, real hover-to-interact**: `setIgnoreMouseEvents`
toggles off when the renderer detects a real mouse-enter on the overlay's
own content (a tiny IPC round-trip, same preload-bridge convention the
main window already uses) — so it never blocks anything underneath
unless the operator is actually looking at it.

**Content, real and bounded** (not a kitchen sink):
- Top line: the single most urgent real `/order/global` item (an
  overdue item if any exist, else the next real due-today item).
- A real, small badge count for the rest of today's/this week's items —
  click opens the MAIN window's Order tab, not a duplicate list here.
- A transient "toast" region: the most recent real Acting Loop finding
  or I-Loop reflection, shown for a bounded real time (e.g. 20s) then
  cleared — not accumulated, not a second inbox to manage.

### 3.2 What wakes it (reusing the real ripple, not a new poller)

The renderer's overlay view subscribes to the SAME real WebSocket ripple
channel the main window already can (`graph_event_frame`,
`src/grpc/mod.rs`, proven live this session via `T-G4`'s wire-contract
test) — filtered client-side to the kinds this overlay cares about
(`Task` due-date changes, the Acting Loop's `AMTExpansion`-tagged
updates, `i_loop_reflection`-keyword coordination events). A real 60s
`/order/global` poll is the fallback for the due-date bucket itself
(due-ness is time-based, not purely event-based — a task doesn't "ripple"
merely because a clock ticked past its `due_at`), mirroring the exact
event-driven-with-interval-fallback discipline `amt_loop.rs` already
established this session, not a new, inconsistent polling model.

### 3.3 Budget discipline (the same constraint as everything else built this session)

The overlay does **zero** new LLM calls of its own — it is a pure
presentation layer over data the Acting Loop/I-Loop/Universal Order
mechanisms already produced. This is a real, load-bearing design
property, not an incidental one: nothing about "showing a nudge" should
ever cost a real call.

### 3.4 Platform-real constraint, stated honestly

`transparent: true` + always-on-top click-through overlays are
well-supported on Windows/macOS; on Linux (this development environment)
behavior varies by compositor (works cleanly under most Wayland
compositors and X11 window managers, but is not universally guaranteed
— GNOME/some tiling WMs can be inconsistent about true click-through).
Not a blocker for building it, but real testing on the operator's actual
target platform is needed before calling this done — not assumed to
"just work" everywhere Electron runs.

## 4. Build order (queued, not started)

1. `overlayWindow` creation in `ui/electron/main.js`, alongside
   `createWindow()` — position persistence (last real x/y) via the same
   local-storage/config convention the main window likely already has
   for its own bounds, if any; else a sane default (top-right, 24px
   margin).
2. A new, small renderer entry (`ui/src/overlay/OverlayApp.tsx` or
   similar) — reuses `ozoneClient.ts`'s existing `fetchGlobalOrder`
   directly, no new client code needed there.
3. The WebSocket ripple subscription + client-side kind filter (reuses
   the main window's existing connection pattern — check whether it's
   feasible to literally share one WS connection across both
   `BrowserWindow`s via the main process relaying frames over IPC,
   rather than opening a second independent WS connection to the host;
   real investigation needed, not assumed either way).
4. The hover-to-interact click-through toggle (small, real, IPC-based).
5. A real Settings toggle to enable/disable the overlay entirely
   (`SettingsPanel.tsx`) — this is exactly the kind of always-on
   background UI feature that needs an honest, easy off switch, matching
   this project's own "unconfigured/undesired = never forced on"
   doctrine already applied to every MCP and consciousness sub-feature
   this session.

## 5. Non-goals (stated, matching this doc-family's own convention)

- NOT screen capture, vision analysis, or OCR of on-screen content —
  explicitly out of scope, see §1.
- NOT a notification-history inbox — the toast is transient by design;
  the real, permanent record already lives in `/order/global` and the
  living graph itself, not duplicated here.
- NOT cross-platform-guaranteed on day one — Linux click-through
  behavior needs real, separate verification per §3.4.
- NOT a new LLM-call surface — presentation only, per §3.3.
