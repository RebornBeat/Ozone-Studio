# Workspace UI Revamp — Plan (2026-10-17)

> Companion to `docs/UI_UX_FORK_PLAN.md` (the prior, already-largely-DONE UI
> build effort this plan builds on top of). Triggered by the operator
> actually using the live app and finding the Workspace/Project flow
> rough, under-featured, and visually unbalanced. Investigated with 5
> forks before writing anything down — every finding below is grounded,
> not guessed. Maintained additively like every other doc in this stack.

---

## The root cause, found first, that explains almost everything else

**The "Workspace" tab — the FIRST tab in the entire app (📁, order 0,
`ui/src/pipeline-ui.tsx:216`) — is wired to the OLD legacy
`assets/pipelines/general/workspace_tab/ui/component.js` pipeline UI.**
This is a thin, separately-maintained module: its own minimal file list
(bare icon + filename, no path/provenance), its own from-scratch chat
renderer (dumps a raw response string into a plain div, none of
`MetaPortion`'s real capability), its own "Link File"/"Link URL" buttons,
and a real logic bug in its empty state (next section).

**The REAL, rich, already-built file system — `WorkspaceBrowser.tsx` +
`FileReferenceViewer.tsx` + `CodeViewer.tsx` + `FileEditor.tsx` +
`CodeCallGraph.tsx` + `CodeEngine.tsx`, all confirmed real and working by
the prior `UI_UX_FORK_PLAN.md` effort — lives behind the "Files" tab,
13th in tab order** (`ui/src/pipeline-ui.tsx:237`, `files-viewer`,
pipeline id 1007, mounted via `FilesPanel.tsx` → `WorkspaceBrowser`).

This single fact explains nearly every complaint in this session's live
walkthrough: no file-provenance display, no real file viewer, no call
graphs reachable, an under-powered chat — **all of it already exists**,
just not reachable from the tab a new user naturally lands on first.

**The decision this plan makes**: do not duplicate the rich file system
inside the legacy module. Make the Workspace tab, once a project is
selected, route into the SAME real views "Files" already has (one file
system, reachable from two entry points), rather than rebuilding a
second, parallel file browser inside `workspace_tab`. Batch W3 below
investigates the exact mechanism before implementation, since pipeline-UI
modules (`component.js`, eval'd via a separate loader,
`pipeline-ui/PipelineUILoader.tsx`-equivalent) and the main `ui/src/`
React app are different bundles/loading mechanisms — embedding one
inside the other may not be trivial.

---

## Batch W1 — Quick, well-scoped fixes (low risk, ready now)

- [ ] **W1.1 — Workspace-list crunched-display bug.** Confirmed real logic
  bug, not just CSS: `workspace_tab/ui/component.js:990-1018` renders a
  full-height, centered "Select a workspace to view projects" block
  UNCONDITIONALLY whenever nothing is selected — it never checks whether
  real workspaces exist in the sidebar list. `.empty-state-centered`
  (`App.css:3656`) is `height:100%; justify-content:center`, and
  `.workspace-content` (`App.css:5031`) is `flex:1`, so this one static
  block claims all remaining width/height regardless of context. Fix:
  collapse this into a compact banner/prompt (not a full-height block),
  freeing the content area — especially once W2 lands and there's a real
  project-scoped chat that deserves that space instead.
- [ ] **W1.2 — Project view's "No linked content yet" panel is too tall.**
  Same shape as W1.1 — a full-height empty state crowding out space that
  should go to the (post-W2) project-scoped chat. Collapse to a compact
  top banner.
- [ ] **W1.3 — Notification positioned in the middle instead of the top.**
  INCONCLUSIVE from investigation — `.notification-dropdown`
  (`App.css:5723`) anchors correctly below its bell icon, so that's
  likely not what the operator saw. The real culprit is probably
  `ThemeArea.tsx`'s `addNotification` toast system (confirmed real this
  session for other fixes), but its positioning wasn't checked in this
  pass — needs its own dedicated look before fixing.

---

## Batch W2 — Project-scoped chat (extend `MetaPortion`, do not rebuild)

**Confirmed this is a small, well-scoped extension, not a rewrite.**
`MetaPortion.tsx` already has a real `ChatContext` union
(`{type:'global'} | {type:'workspace', id, name}`, line ~375) with
**per-context transcript isolation** (`transcriptsByContext`, keyed by
context) — the component's own doc comment confirms this was built
deliberately for exactly this kind of scoped reuse. The legacy chat
(`workspace_tab/ui/component.js:44`) already calls the identical real
backend path (`window.ozone.orchestrate({..., project_id, workspace_id,
context_scope:"Project"})`) — only the FRONTEND is a from-scratch minimal
stub with none of `MetaPortion`'s real capability (no thinking indicator,
no markdown rendering, no attempt trail, no model-accented bubbles).

- [ ] **W2.1** — Add a `'project'` variant to `MetaPortion`'s `ChatContext`
  union, and an optional `lockedContext` prop to `MetaPortionProps`. When
  `lockedContext` is set, skip rendering the context-switcher UI and force
  `chatContext` to that value permanently — everything else (thinking
  indicator, markdown, attempt trail, model bubbles) comes along for free
  since it's all downstream of `transcript`/`chatContext`, not duplicated
  logic.
- [ ] **W2.2** — Investigate whether `MetaPortion` (a component in the main
  `ui/src/` React bundle) can actually be mounted FROM a pipeline-UI
  module like `workspace_tab/component.js` (a separately eval'd module) —
  this is the same cross-bundle question Batch W3 needs answered for the
  file viewer. If mountable directly: wire the Project view to render
  `<MetaPortion lockedContext={{type:'project', id, name}} />` instead of
  the old thin chat. If NOT directly mountable (likely, given the
  separate-eval loading mechanism): the real fix is the same as W3's —
  route the user into the real `ui/src/` surface rather than reimplementing
  `MetaPortion`'s capability a second time inside the pipeline-UI module.
- [ ] **W2.3** — Once W2.1/W2.2 land and are verified, retire the old thin
  chat-rendering code in `workspace_tab/component.js`.

---

## Batch W3 — File-list unification (show real data, not a rebuild)

**Confirmed**: `FileReferenceViewer.tsx` already shows real path
(parsed from the real `"File: {path}"` container-name convention),
real "Linked via" provenance, timestamps, keywords, and real bidirectional
graph relationships with confidence/discovered_via — fully built,
nothing missing. The legacy `workspace_tab`'s own file list
(`loadProjectFiles`, line 331) shows only an icon + bare filename — no
path, no provenance, nothing else. **The gap is not "build file
provenance display," it's "the Project view uses the thin component
instead of the rich one that already exists."**

- [ ] **W3.1 — THE architectural question, investigate before building
  anything.** Can a pipeline-UI module (`workspace_tab/component.js`,
  loaded via a separate eval mechanism) embed/mount real `ui/src/views/
  files/*` React components directly? If yes: embed `WorkspaceBrowser`/
  `FileReferenceViewer` into the Project view directly. If no (more
  likely, given the different bundle/loading mechanism): the correct fix
  is making project-selection in the Workspace tab **navigate the user
  into the "Files" tab** with that project pre-selected (reusing
  `navigation.ts`'s real `onNavigate` mechanism, confirmed working this
  session for `graph-node`/`container`/`task` targets — a `project`
  target variant would be a small, analogous addition) — one real file
  system, two entry points, rather than two maintained file browsers.
- [ ] **W3.2** — Once W3.1's mechanism is decided, apply the same decision
  to "Files" tab's own discoverability: consider reordering it closer to
  the front (e.g. right after Workspace/Order), since it's one of the most
  complete, real surfaces in the whole app and is currently buried 13th.

---

## Batch W4 — Folder-linking (net-new, but well-scoped)

**Confirmed feasible, nothing exists yet.** `file_link`'s `execute()`
(`assets/pipelines/general/file_link/main.rs:599`) is strictly
single-file. `DirectoryReference = 51` already exists as a real
container-type discriminant (`src/types/container.rs:471`, with a working
`Display` arm) — just never constructed anywhere. No Electron
directory-picker exists (`ui/electron/main.js`/`preload.js` only have
`selectFile`, not `selectFolder`/`selectDirectory`). No UI button exists
for this at all.

- [ ] **W4.1** — Add a `selectFolder` IPC bridge (`showOpenDialog` with
  `properties: ["openDirectory"]`), mirroring the existing `selectFile`
  pattern exactly.
- [ ] **W4.2** — Add a `LinkFolder` action to `file_link` (pipeline #30):
  walk the directory, create one `DirectoryReference` parent container,
  call the EXISTING single-file link logic for each child file, linking
  them as children of the new directory container. Architecturally
  natural, not a redesign.
- [ ] **W4.3** — Add a "Link Folder" button next to the existing "Link
  File"/"Link URL" buttons (wherever the Project view ends up living per
  Batch W3's decision).

---

## Batch W5 — CORRECTED 2026-10-17: the native git tool already exists — it needs starting + surfacing, not building

**Original framing above (W5 as a from-scratch build, Option A/B) was
WRONG — the operator corrected this directly: "native git is a MCP we
have... review and find it."** This is NOT a `git`-binary wrapper like
ZCode's — it's better-fitting for this project: a graph-native file-history
system tracking per-CONTAINER history (the real unit of truth here), not
per-file-on-disk.

**What's real, confirmed by direct investigation (not guessed)**:
- `src/file_beacon.rs` — real, boot-spawned (`src/lib.rs:1143`), polling
  every `FileReference` container every 45s, detecting real mtime/size
  changes, writing real JSONL history + capped snapshots to
  `zsei_data/file_history/<container_id>.jsonl`. **Confirmed live data on
  disk spanning months** (`target/release/zsei_data/file_history/40380.jsonl`
  etc.) — not a stub, has been running the whole time.
- `tools/git-mcp/server.mjs` (149 lines) — a real HTTP server (port 3235)
  exposing 3 real MCP tools over that same data: `file_history`,
  `file_status`, `file_diff`.
- Registration is NOT a hand-edited file — `src/mcp.rs` (663 lines) is a
  real registry module (`POST /mcp/tools/register`, persisted
  automatically). **git-mcp IS already registered** — confirmed via the
  actually-running host's real data dir
  (`target/release/zsei_data/mcp_tool_registry.json`, 134 entries,
  registered since 2026-09-29, survives restarts correctly). An earlier
  pass in this same investigation wrongly concluded "never registered" by
  reading the stale, untouched repo-root `zsei_data/` copy instead of the
  one the running process actually uses (`cwd`-relative `data_dir` in
  `config.example.toml`) — corrected by a second, independent check.
- **The one real gap**: the registration persists, but the actual
  `node tools/git-mcp/server.mjs` process (and ~10 sibling MCP tool-server
  processes — the `tools/bridges/` fleet, `terminal-mcp`, `visual-mcp`,
  etc., all separately confirmed real and substantive, not stubs) is **not
  currently running** — nothing listens on port 3235 or the other
  registered tool ports right now. Calling any of these registered tools
  today fails with connection-refused until their process is (re)started.
  No startup script launches this MCP tool-server fleet alongside the
  host boot — registration ≠ running process, and only the latter is
  missing.
- `VersionHistory.tsx`'s existing (empty) history panel reads a
  DIFFERENT, unrelated mechanism (`GetVersionHistory`, backed by the
  integrity monitor's never-fired snapshot path) — not the same data as
  git-mcp's. Don't conflate the two; either replace or sit this alongside
  it with an honest distinct label once UI work happens.

**Revised scope, no longer an Option A/B guess**:
- **W5.1** — Start `tools/git-mcp/server.mjs` and confirm it answers real
  calls against the live container history. (Live-system action —
  starting a new process — hold for explicit operator go-ahead before
  doing this unprompted.)
- **W5.2** — Decide whether to also (re)start the other ~10 registered-
  but-dormant tool-server processes (bridges fleet, terminal-mcp,
  visual-mcp, etc.) and whether/how to supervise them so they survive
  restarts — this is real, separate infrastructure work (process
  supervision), not part of this UI revamp; flagging for its own batch,
  not solving here.
- **W5.3** — Build a small UI surface (Files tab, near the existing empty
  `VersionHistory.tsx` panel) that calls git-mcp's 3 tools directly and
  shows real per-container change history/diffs once W5.1 is done.

---

## Batch W6 — Call graphs per file (already built, just needs reachability)

**Confirmed: `CodeCallGraph.tsx`/`CodeEngine.tsx` are already real and
working** (verified by the prior `UI_UX_FORK_PLAN.md` effort, Batches
F/G) — Ozone-Studio is already AHEAD of ZCode here (ZCode has no
call-graph UI at all, confirmed this session). **No new building needed**
— this item is fully closed by Batch W3's reachability fix (once "Files"/
"Engines" are easier to reach from the Workspace flow, call graphs are
already there waiting).

---

## Recommended execution order

1. **W1** (quick fixes, no architectural decisions needed) — ready now.
2. **W3.1** (the cross-bundle investigation) — must land before W2.2/W3.2
   can be implemented correctly; both depend on its answer.
3. **W2** (project-scoped chat) and the rest of **W3** (file-list routing)
   — once W3.1's mechanism is known.
4. **W4** (folder-linking) — independent of the above, can run in parallel.
5. **W5 Option A** (git status MVP) — independent, can run in parallel;
   **Option B stays gated on the operator's explicit word.**
6. **W6** — no work, closes automatically once W3 lands.

## Status

Written 2026-10-17, grounded in 5 investigation forks (two-UI-surface
confirmation, git-tool/folder-link search, workspace-list-bug diagnosis,
file-provenance/chat-scoping check, ZCode comparison study) — every claim
above has file:line evidence behind it, logged in `CHECKLIST.md`'s
2026-10-17 entries. Nothing in this plan has been built yet.
