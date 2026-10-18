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

## Batch W1 — Quick, well-scoped fixes (low risk, ready now) — ✅ DONE 2026-10-18

- [x] **W1.1 — Workspace-list crunched-display bug.** FIXED: collapsed
  into a new compact `.compact-prompt-banner` class (top-aligned, not
  full-height centered).
- [x] **W1.2 — Project view's "No linked content yet" panel is too tall.**
  FIXED: same `.compact-prompt-banner` treatment.
- [x] **W1.3 — Notification positioned in the middle instead of the top.**
  FIXED — real cause was different from the guess above: not
  `ThemeArea.tsx`'s toast, but `DiscoveryEffects.tsx`'s toast
  (`position:absolute; bottom:64; right:12`) mounted inside `MetaPortion`'s
  `.meta-prompt` box, which had no `position:relative` anywhere in
  `App.css` — the toast resolved its offset against the whole page
  instead of the small box it was meant to float above. Fixed by adding
  `position:relative` to `.meta-prompt`.

---

## Batch W2 — Project-scoped chat (extend `MetaPortion`, do not rebuild) — ✅ DONE 2026-10-18

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

- [x] **W2.1 — REVISED 2026-10-18, operator-corrected**: the first pass
  locked `MetaPortion` itself (`'project'` variant + `lockedContext`) —
  the operator explicitly corrected this: `MetaPortion` must stay fully
  untouched, always-visible, non-interfering. **Reverted in full**
  (confirmed via `git diff` against the base commit — byte-identical
  except for the extraction below). Real fix: extracted `MetaPortion`'s
  rendering engine (transcript types, markdown, model-accent styling,
  attempt-trail summary) into shared `ui/src/chatRendering.tsx`, which
  `MetaPortion` now imports instead of defining locally — a pure
  extraction, no behavior change to the global sidebar.
- [x] **W2.2 — BUILT, THEN REMOVED 2026-10-18 (operator correction #2)**:
  first built a genuinely separate `ProjectChatPanel.tsx` mounted as a
  React sibling from `pipeline-ui.tsx`'s `DynamicPipelineUI`, scoped to
  `pipelineId === 6`. After live-testing it, the operator correctly
  identified it as redundant: `MetaPortion` already has a real, working
  global/workspace chat-context switcher (`MetaPortion.tsx:88-91,877-897`,
  pre-existing, untouched) giving a separate transcript per workspace —
  a second embedded chat surface duplicated that. **Removed in full**:
  `ProjectChatPanel.tsx`/`useChatSession.ts` deleted, `DynamicPipelineUI`
  reverted to always rendering the plain container (no sibling, no
  shared-state subscription for it), dead CSS removed. `chatRendering.tsx`
  (the W2.1 extraction) stays — `MetaPortion` still imports it; that part
  was a real simplification independent of the panel that got removed.
  **Real gap found while fixing the modal copy that claimed projects "have
  their own chat scope"**: traced the actual context path
  (`MetaPortion.handleSubmit` → `orchestrate({workspace_id, project_id:
  undefined})` → `orchestrator/stages.rs` `ForStep` → `context_aggregation`
  `ForStep` handler) and confirmed `workspace_id`/`project_id` there only
  scope the `/SharedContext` coordination layer, never a project's linked
  files — a separate `ForProject` action does real file retrieval but
  `ForStep` (what chat actually uses) never calls it. So the workspace
  switcher gives a real separate conversation, but linking files to a
  project does **not** currently feed them into that conversation as
  context. Modal copy corrected to stop claiming otherwise; closing the
  actual gap is new scope, not done here.
- [x] **W2.3** — DONE: the old `ProjectChat` stub (232 lines) stays
  deleted; `window.__ozoneMetaPortion` calls (added then reverted) removed
  entirely — confirmed zero references anywhere via `grep`.
- [x] **W2.4 — bonus, found while fixing W2.2's copy** (2026-10-18):
  Create-Project/Create-Workspace modals share `newName`/`newDescription`
  state; neither backdrop-click nor "Cancel" ever cleared it (only the
  success path did), so cancelling one modal left stale text pre-filled
  in the other. Fixed with one shared `closeCreateModals()` wired to all
  four dismiss points (`workspace_tab/ui/component.js`). Also: the
  "Link Folder" toolbar button was real all along (W4) but unreadable —
  three toolbar buttons plus title crammed into the pre-W2.2-fix 300px
  side column; fixed by W2.2's layout correction (column stack, full
  width on top) rather than a separate change.

---

## Batch W3 — File-list unification (show real data, not a rebuild) — ✅ DONE 2026-10-18

**Confirmed**: `FileReferenceViewer.tsx` already shows real path
(parsed from the real `"File: {path}"` container-name convention),
real "Linked via" provenance, timestamps, keywords, and real bidirectional
graph relationships with confidence/discovered_via — fully built,
nothing missing. The legacy `workspace_tab`'s own file list
(`loadProjectFiles`, line 331) shows only an icon + bare filename — no
path, no provenance, nothing else. **The gap is not "build file
provenance display," it's "the Project view uses the thin component
instead of the rich one that already exists."**

- [x] **W3.1 — ANSWERED and BUILT**: not directly mountable, confirmed
  (same cross-bundle boundary as W2.2). Extended `window.__ozoneThemeArea`
  with `navigateTo(target)`/`setSelectedProject(id)` (thin pass-throughs
  to the real `navigation.ts`/`projectSelection.ts`). Project-view file
  rows now jump into the real `CodeViewer`/container views via
  `{kind:"code-file"}`/`{kind:"container"}`. Added a "view call graph →"
  link from `CodeViewer.tsx` to Engines (shares project-selection state).
  Bonus bug found+fixed: `workspace_tab/main.rs`'s `FileRef` response
  struct never declared the `zsei_container_id` field `file_link` was
  already saving — silently dropped before reaching the UI; added it.
  Honestly NOT done: a direct file→History sub-tab cross-link —
  `VersionHistory.tsx`/`CodeViewer.tsx` are `PanelShell`-local sub-tabs
  with no external hook without changing `PanelShell.tsx` itself (shared
  by 4 panels) — bigger than a one-liner, flagged not forced.
- [x] **W3.2 — DONE**: Files tab moved from order 12 (13th) to order 1
  (right after Workspace) in `ui/src/pipeline-ui.tsx`'s
  `CORE_TAB_DEFINITIONS`, rest renumbered sequentially.

---

## Batch W4 — Folder-linking (net-new, but well-scoped) — ✅ DONE 2026-10-18

**Confirmed feasible, nothing exists yet.** `file_link`'s `execute()`
(`assets/pipelines/general/file_link/main.rs:599`) is strictly
single-file. `DirectoryReference = 51` already exists as a real
container-type discriminant (`src/types/container.rs:471`, with a working
`Display` arm) — just never constructed anywhere. No Electron
directory-picker exists (`ui/electron/main.js`/`preload.js` only have
`selectFile`, not `selectFolder`/`selectDirectory`). No UI button exists
for this at all.

- [x] **W4.1 — DONE**: real Electron `selectFolder` IPC bridge, mirrors
  `selectFile` exactly.
- [x] **W4.2 — DONE**: `FileLinkInput::LinkFolder{project_id, folder_path,
  analyze}` action added. Walks non-recursively (subdirectories honestly
  reported in a new `skipped_entries` field, not silently dropped),
  creates one real `DirectoryReference` (type 51) container, links each
  child file via the existing `LinkMultiple` logic plus a `Contains`
  relation to the directory container.
- [x] **W4.3 — DONE**: "📁 Link Folder" button added next to Link
  File/Link URL in `workspace_tab/component.js`.

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

---

## Batch W7 — Full immersive revamp: Graph embedding, workspace-level
## aggregation, CRUD completion, cross-link/consolidation audit (2026-10-18)

Operator's ask, after W1-W6 landed and `ProjectWorkspaceViewer` (Files/
Fabric/Hierarchy, project-scoped) replaced the bare file area: do a full
review of the ENTIRE Workspace tab experience — every screen, every
button, every back-navigation, every cross-link — and capture + build
whatever's still missing for a genuinely complete, immersive, interactive
experience. This section is that review's findings plus the build plan.
Grounded in a full re-read of `workspace_tab/ui/component.js` (997 lines,
every branch) and `workspace_tab/main.rs`'s real `WorkspaceInput` enum —
not assumed.

### W7.1 — Embed Graph as a real 4th tab in `ProjectWorkspaceViewer`

**Gap**: `ProjectWorkspaceViewer.tsx` (built this session) has Files/
Fabric/Hierarchy tabs, each reusing a real component that already takes a
plain `projectId` prop. `GraphView.tsx` is the one holdout — it manages
project selection as its OWN internal `useState` (line ~162), driven by
its own `<select>`, not a prop. Embedding it unmodified would show a
redundant project picker inside a view that's already project-scoped by
the tab it's in.

**Plan**: give `GraphView` an optional `projectId` prop. When provided,
skip its own project-fetch/dropdown entirely and drive `loadGraphData`
directly from the prop (falling back to today's dropdown-driven behavior
when the prop is absent, so the GLOBAL Graph tab — which has no fixed
project — is completely unaffected). Add `{ id: 'graph', label: '🕸 Graph' }`
to `ProjectWorkspaceViewer`'s `SECTIONS` array, rendering
`<GraphView projectId={projectId} />`.

### W7.2 — Consolidate the Linked-Content list with the new viewer (remove duplication, keep Unlink)

**Gap**: `component.js`'s PROJECT VIEW (lines 486-590) still renders its
own compact file list (or "No linked content yet") ABOVE the new
`ProjectWorkspaceViewer`, which ALSO shows the project's files (via
`WorkspaceBrowser` in its Files > Browser tab). Two file listings stacked
on top of each other is not "immersive," it's cluttered — and it happened
because the old list's Unlink button (line 565-587, a real, working
`FileLink#30 Unlink` call) has no equivalent in `WorkspaceBrowser`, which
is a generic, project-agnostic-of-this-context browser also used by the
GLOBAL Files tab.

**Plan**: give `WorkspaceBrowser.tsx` an optional `onUnlink?: (fileRef) =>
void` prop (undefined by default — zero behavior change for the global
Files tab, which never passes it). When present, render a small "Remove
link" action per row. `ProjectWorkspaceViewer` accepts an optional
`onUnlinkFile` passthrough to its Browser tab. `component.js` passes its
real `handleLinkFile`'s sibling unlink logic through
`props.initialData`/the shared-state bus (same cross-bundle-callback
pattern already used elsewhere — confirm the exact mechanism available
before picking one; `executePipeline(30, {action:"Unlink",...})` is
callable from the MAIN bundle too via `window.ozone.pipeline.execute`, so
`ProjectWorkspaceViewer` may just call it directly rather than needing a
callback from `component.js` at all — simpler, prefer this if it works).
Once Unlink is reachable from the new viewer, delete `component.js`'s own
file-list rendering (lines 486-590) entirely — keep only the toolbar/
stats/description above it. Verify no other real capability from that
block besides Unlink + the cross-tab `openFile()` jump (which
`WorkspaceBrowser`/`FileReferenceViewer` likely already do their own way —
confirm, don't assume) would be lost.

### W7.3 — Workspace-level aggregation (replace the grid-only PROJECTS VIEW and the top-level blank landing)

**Gap**: two different "nothing scoped yet" spots:
1. Top-level landing (no workspace selected) — `component.js` lines
   897-926 — just an info banner + "Select a workspace" prompt, real blank
   space the operator flagged.
2. PROJECTS VIEW (a workspace selected, no project yet) — lines 645-818 —
   a plain card grid. Functional, but not "immersive": no aggregate view
   of what's actually IN the workspace across its projects.

**Plan, scoped honestly** (no backend aggregate action exists —
`WorkspaceInput` has `ListProjects`/`GetProjectFiles` per-project only, no
`GetWorkspaceContext`-style merge; confirmed via `main.rs`'s real enum).
Building true graph/fabric/hierarchy aggregation across N projects is a
separate, larger backend+frontend effort (those views are fundamentally
single-project graph structures) — do NOT fake it. What's genuinely
buildable now, client-side, no backend change: a `WorkspaceAggregateView`
that, given the workspace's already-loaded `projects` array, calls
`GetProjectFiles` for each (parallel, already a real per-project action)
and renders a merged, clickable file list plus real aggregate counts
(total files/URLs/packages summed from the per-project counts already in
`projects`), replacing the plain info-banner+grid combo with something
actually useful. Mount it the same sibling way as `ProjectWorkspaceViewer`
(new `selectedWorkspaceId`-keyed shared-state read in `DynamicPipelineUI`,
scoped to when a workspace is selected but no project is), or inline
within `component.js` if it turns out to need only data already in
scope — pick whichever avoids a new cross-bundle mount if the data
doesn't require a full React component (e.g. if it's just a richer list,
it might render fine as plain `React.createElement` calls in `component.js`
directly, no new mounted component needed at all). Flag Graph/Fabric/
Hierarchy aggregation as explicitly deferred, not silently dropped.

### W7.4 — CRUD completion: delete + rename, for both workspaces and projects

**Gap**: `main.rs`'s `WorkspaceInput` already has real, working
`UpdateWorkspace`/`DeleteWorkspace`/`UpdateProject`/`DeleteProject`
actions (confirmed, lines 29-41) — ZERO of them are wired to any button
anywhere in `component.js`. Only Create exists in the UI. "All buttons"
can't be complete without the other half of basic CRUD.

**Plan**: add a small "⋮" menu (or two icon buttons) to each workspace
sidebar item and each project card, plus to the PROJECT VIEW's and
PROJECTS VIEW's toolbars, with Rename (reuses the existing create-modal
pattern, pre-filled) and Delete (real confirmation dialog first — this is
destructive and does cascade-delete a workspace's projects file,
confirmed at `main.rs:280-302` — never delete without the operator
confirming in the dialog, no silent/instant deletes).

### W7.5 — Minor polish found during the full read, bundle in if cheap

- `formatDate` (line 351-353) is defined and never called anywhere —
  either use it (e.g. "Created {date}" on project cards / workspace
  items) or remove it as dead code. Prefer using it — real timestamp data
  the UI currently throws away.
- No breadcrumb anywhere showing "Workspace > Project" together — the
  PROJECT VIEW's toolbar shows only the project name. Cheap to add next to
  the back button.
- Workspace sidebar items show name only, no project/file count — minor,
  skip unless trivial once W7.3's aggregate data is already being
  computed nearby.

### Dispatch

Four independent forks (file-overlap checked: W7.1 touches `GraphView.tsx`
+`ProjectWorkspaceViewer.tsx` only; W7.2 touches `WorkspaceBrowser.tsx`+
`ProjectWorkspaceViewer.tsx`+`component.js`'s file-list block; W7.3 is a
new component + `component.js`'s landing/grid blocks +
possibly `pipeline-ui.tsx`; W7.4 touches `component.js` broadly +
`App.css` for the new menu/dialog — claim files before editing, per
protocol, since W7.2 and W7.4 both touch `component.js`):

1. **Fork A — W7.1** (Graph embedding)
2. **Fork B — W7.2** (consolidate linked-content list)
3. **Fork C — W7.3** (workspace-level aggregation)
4. **Fork D — W7.4 + W7.5** (CRUD completion + polish)

No build/test run before dispatch, per standing instruction — forks
verify their own syntax (`node --check` for `.js`, careful manual review
for `.tsx` since no build is run) before reporting back.
