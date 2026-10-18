# UI/UX & Frontend Reachability Guide (2026-10-17)

> Companion to `docs/WORKSPACE_UI_REVAMP_PLAN.md` and `docs/UI_UX_FORK_PLAN.md`.
> Triggered by two live-testing bug reports (Files tab empty, Graph View
> stuck loading) plus an explicit request to map how every tab reaches
> pipelines and other tabs, and to find and document the native git MCP
> tool. Built from 9 investigation/fix forks this pass — every claim below
> has file:line evidence, cross-checked at least twice where a first pass
> was wrong (see the registry correction below).

---

## 1. The native git tool — found, corrected understanding

**Not a `git`-binary wrapper** (unlike the sibling tool "ZCode," which
shells out to real `git`). Ozone-Studio's version is graph-native: it
tracks per-**container** history (the real unit of truth in this system),
not per-file-on-disk.

- `src/file_beacon.rs` — real, boot-spawned (`src/lib.rs:1143`). Polls
  every `FileReference` container every 45s, detects real mtime/size
  changes, writes real JSONL history + capped (10-max) snapshots to
  `zsei_data/file_history/<container_id>.jsonl`. **Confirmed live data
  spanning months** on disk — not a stub.
- `tools/git-mcp/server.mjs` (149 lines) — real HTTP server, port 3235,
  3 MCP tools over that data: `file_history{container_id}`,
  `file_status{}`, `file_diff{container_id, from_ts?, to_ts?}`.
- **Registration status (corrected mid-investigation)**: a first pass
  concluded "never registered" by reading the stale repo-root
  `zsei_data/mcp_tool_registry.json` (frozen since 2026-09-29, 1 entry). A
  second, independent pass found the *actually-running* host uses a
  `cwd`-relative data dir (`target/release/zsei_data/mcp_tool_registry.json`,
  modified minutes before that check) — **134 registered tools**,
  including git-mcp's 3, registered since 2026-09-29 and surviving
  restarts correctly via `src/mcp.rs`'s real persistence
  (`McpRegistry::with_persistence`). **git-mcp is, and has been, properly
  registered.** Lesson: always confirm which `zsei_data/` directory a
  live host actually reads before concluding something is missing.
- **The one real gap**: registration ≠ a running process. Nothing listens
  on port 3235 (or any of ~10 other registered tools' ports) right now —
  confirmed via `ss -tlnp`. No startup script launches this MCP
  tool-server fleet alongside the host boot. Calling any of these 134
  registered tools today fails with connection-refused until its process
  is (re)started.
- Separate from `VersionHistory.tsx`'s existing UI panel, which reads a
  DIFFERENT mechanism (`GetVersionHistory`, backed by the integrity
  monitor's snapshot path) — confirmed genuinely empty, never fired. Do
  not conflate the two; git-mcp's data is the real, populated one.
- AGI-side tool-calling is already wired (`crate::mcp::call_global()`);
  `McpActivityIndicator.tsx` reads a shared activity feed that would show
  git-mcp calls automatically once its process runs and gets called.

**Action items** (see `docs/WORKSPACE_UI_REVAMP_PLAN.md` Batch W5,
corrected): start the process (W5.1, held for explicit go-ahead —
starting a new long-running process is a live-system action); decide on
supervising the other ~10 dormant registered tools (W5.2, separate
infrastructure work); build a small UI surface reading git-mcp's 3 tools
directly (W5.3).

---

## 2. The wider MCP tool fleet — real, substantial, mostly dormant

18 tool directories characterized, plus `tools/bridges/` (67 tools, a
real submodule with its own registration script, wrongly skipped as
"not an MCP tool" in the first pass — it is one). **All 19 are real,
substantive, documented builds — none are stubs.** Each corresponds to a
specific modality/domain already built elsewhere in this project:

| Tool | Domain | Modality/pipeline correspondence |
|---|---|---|
| chemistry-mcp | PubChem + RDKit | chemistry modality |
| dna-mcp | NCBI genome lookup | dna modality |
| geospatial-mcp | OSM Nominatim/Overpass | geospatial modality |
| proteomics-mcp | UniProt lookup | proteomics modality |
| mesh-mcp / threed-mcp | 3D mesh + CAD/Blender | 3D, CAD modalities |
| code-graph-mcp / code-mcp | Python call-graph | code modality |
| cerebrix-mcp | EEG/KaraOne trials | EEG/BCI modalities |
| connectome-mcp | fly brain connectome | network/biology/3D |
| visual-mcp (991 lines, largest) | screen capture→YOLO/depth→graph | image modality |
| web-mcp | Brave search wrapper | url_link/search |
| android-mcp | adb device control | mobile/device |
| security-mcp / re-mcp | read-only monitoring / RE containers | security |
| terminal-mcp | allowlisted shell exec | system/dev-tools |
| gcal-mcp / zoom-mcp | OAuth calendar/meeting sync | Order Stage 3 |
| bridges (67 tools) | — | general capability fleet |

All real, backed by registration in `src/mcp.rs` (663 lines: real
`McpRegistry`, HTTP/SSE dispatch, stdio delegation to the owning agent
session, a real per-agent/day usage ledger). The live registry holds 134
entries total. **None of the backing server processes are currently
running** (confirmed via `ss -tlnp` — zero listeners across all their
ports). `register-tools-to-ozone.mjs` (bridges) is a real, one-shot,
manually-run script, not boot-automated — explains how 134 entries got
there without explaining why their processes aren't up now.

**This is the single largest "built but unreachable" item found this
session** — a near-complete second capability layer (domain tool access
for the AGI itself, not just modality pipelines) sitting dormant behind
a one-line "start the process" gap, times ~11 processes, plus a decision
on whether/how to supervise them so they survive restarts.

---

## 3. Fixed this pass: Files tab empty + Graph View stuck loading

**One shared root cause**, confirmed independently by two separate forks:
`currentUserId() ?? 1` in `projectSelection.ts:47`, `GraphView.tsx:59`,
`WorkspaceBrowser.tsx:33`, `store.ts:402` calls
`window.ozone.auth.getCurrentUserId()` — a method that doesn't exist
anywhere in the Electron preload bridge (`ui/electron/preload.js`'s
`auth` object only has `challenge`/`authenticate`). The optional call
always returns `undefined`, falling through to a hardcoded `1`. But
**every container in the live store** (2,278 of them, confirmed via
direct grep on disk) has `owner_id: 0` — including the one real
workspace. `GetUserWorkspaces{user_id: 1}` always returned zero: not a
deep multi-user identity question, a wrong hardcoded guess with no real
backing anywhere in the backend.

**Fixed**: `?? 1` → `?? 0` in all 4 files. Graph View additionally given
an honest `{kind:"empty"}` status (distinct message: "No workspaces or
projects to graph yet") instead of hanging on `{kind:"loading"}` forever
when a user genuinely has zero projects — a second, latent bug beyond the
owner-id mismatch.

Scope note: `window.ozone.auth.getCurrentUserId()` itself still doesn't
exist — building real multi-user auth is separate, larger work. `0` is
correct for the single-user reality that exists today.

---

## 4. Tab-by-tab reachability map

Navigation mechanism confirmed real throughout: `ui/src/navigation.ts`'s
`NavTarget` union + `navigateTo()`/`onNavigate()` bus, consumed by
`ThemeArea.tsx`'s `tabForNavTarget` (switches tabs) and by individual
components (`takePendingNavigation`) once mounted. `{kind:"container"}` /
`{kind:"graph-node"}` → Graph View (via `resolveOwningProject` walk-up);
`{kind:"task"}` → Tasks tab + `TaskDetailPanel`; `{kind:"code-file"}` →
Files' CodeViewer.

### Order / Tasks / Library / Settings / Monitor
| Tab | Data source | Emits nav? | Receives nav? |
|---|---|---|---|
| Order | real `GET /order/global` | **Fixed this pass** — rows now emit `{kind:"task"}` | No |
| Tasks | real `window.ozone.task.*` bridge (pipeline-UI module, separate bundle) | No | **Structurally can't** — different bundle than the Zustand store/nav bus |
| Library | real pipeline registry | **Fixed this pass** — "UI" badge now opens the pipeline's dynamic tab via `window.__ozoneThemeArea.injectTab` | No |
| Settings | real config endpoints | No (confirmed not a gap — no natural target) | No |
| Monitor | real `GET /monitor/summary` | No | Embeds `TaskDetailPanel` inline (manual id lookup, not click-driven) |

### Tools / Devices / Graph View / Fabric / Hierarchy
| Tab | Data source | Emits nav? | Receives nav? |
|---|---|---|---|
| Tools | real MCP tool registry poll | No (not a gap — the real gap here is §2's dormant fleet, not nav) | No |
| Devices | real pairing flow | No (no natural target) | No |
| Graph View | real workspace/project/graph walk | **No** — `NodeDetailPanel`/`EdgeProvenance` confirmed zero `navigateTo` calls, a real dead end not fixed this pass (bigger than a one-liner) | **Yes**, correctly, including project-id resolution from a bare container id |
| Fabric | shared `useSelectedProject()` | **Fixed this pass** — "Open in Graph View" now carries the real project id instead of a bare tab-switch | No |
| Hierarchy | shared `useSelectedProject()` | `JurisdictionHierarchy` already correct; `AmtTreeView`/`ForkLineageView` **fixed this pass** (header-level container links — individual nodes lack their own container ids) | No |

Structural note, not fixed: Graph View keeps its own standalone
project-selection state, separate from the shared `useSelectedProject()`
hook Fabric/Hierarchy use — the Fabric fix works around this (passes the
id explicitly) rather than unifying the two mechanisms.

### Raw Thoughts / Files / Engines / Coordination / Network
| Tab | Data source | Emits nav? | Receives nav? |
|---|---|---|---|
| Raw Thoughts | real zero-shot call log | **Fixed this pass** | No |
| Files → Browser | real workspace/project containers | Already had better-than-generic onClick (opens real file content) — correctly left alone | No |
| Files → References | real `GetContainer` | **Fixed this pass** — own id + every relationship target now clickable | No |
| Files → Code | real code container | **Already fully bidirectional** — the one clean existing pattern in the whole app | **Yes** |
| Files → Editor / History | real | Not fixed (smaller, lower-value) | No |
| Engines → Code | real code graph | **Yes**, confirmed working end-to-end with Files/Code (not a dead end) | **Yes** |
| Engines → Text | real text graph | Yes, one-way | No |
| Coordination (all 4 sub-tabs) | real `GetContainer` calls | **Fixed this pass** — every sub-tab's container ids now clickable | No |
| Network | real AMT/graph data | **Fixed this pass** | No |

---

## 5. Still open (not fixed this pass — flagged for explicit decision)

- **Graph View's own node/edge detail panels emit no navigation** — a
  real dead end, confirmed, bigger than the one-liner pattern used
  everywhere else this pass (would need its own investigation into what
  `NodeDetailPanel`/`EdgeProvenance` should link to).
- **Graph View vs. Fabric/Hierarchy's two different project-selection
  mechanisms** — works today via the explicit-id workaround, not unified.
- **MCP tool-server fleet (git-mcp + ~10 others) registered but not
  running, no boot-time supervision** — the single largest reachability
  gap found this session; deliberately not started unilaterally (a
  live-system action affecting ~11 long-running processes needs the
  operator's go-ahead, especially on whether/how to supervise them).
- **`ActivityEvent` (Monitor's feed) has no `task_id` field at all** — a
  backend/type gap blocking a click-through that would otherwise be a
  trivial frontend fix.
- **`VersionHistory.tsx` vs. git-mcp's real data** — a design decision
  (replace vs. coexist) deferred to whoever builds W5.3.

---

## 6. Second wave (same day): the MCP-fleet concern checked against reality, remaining gaps closed

**The pipeline-sharing analogy doesn't actually transfer — checked, not
assumed.** The operator asked whether the MCP tool-server fleet (§2) needs
the same care the pipeline fleet got before it was fixed to share a
runtime. Investigated directly: the real pipeline fix
(`scripts/build-pipelines.sh:6`, one shared `CARGO_TARGET_DIR`) is a
**compile-time/disk** fix — deduped ~15GB of compiled Rust dependencies,
capped build parallelism for build-time RAM. At runtime,
`src/pipeline/executor.rs:772` still spawns a fresh short-lived process
per call; there's no persistent pool to have "shared." So leaving the MCP
fleet's ~85 servers as long-lived daemons is a *different* risk shape, not
the same bug recurring.

**Resource audit, better than feared**: `tools/bridges` (67 tools) is
already correctly consolidated into one process. The 18 `tools/*-mcp`
dirs + git-mcp are each a bare, zero-dependency `server.mjs` (~30-50MB
idle each) — cheap, except 5 needing individual review before starting:
`cerebrix-mcp`/`connectome-mcp` (multi-GB bundled datasets + real Python
workers), `mesh-mcp`/`dna-mcp` (smaller, same shape), `visual-mcp` (real
YOLO inference weights), `terminal-mcp` (a sandboxing/security concern,
not RAM). **Recommended phased launch**: git-mcp alone first (highest
value, already-populated real data), then the ~9 tiny pure-API-wrapper
tools, hold the 5 heavy/sensitive ones for individual decisions — not a
blanket start-everything. No process started this pass; this is a plan
awaiting the operator's go, not a completed action.

**Closed this wave**:
- Graph View's own `NodeDetailPanel`/`EdgeProvenance` — were confirmed
  dead ends (§5's top item), now wired with the same `{kind:"container"}`
  pattern, round-tripping correctly within the same mounted tab.
- `find_methodologies_by_keywords` (O2's known scope gap) — closed with
  the same seed→hop graph expansion `amt.rs` uses, cleaner here since
  `ContainerStorage` natively implements `NodeSource`.
- `VersionHistory.tsx` vs. git-mcp — resolved as **add a third section,
  not a replacement**: the file actually has two real sections already
  (only one, `GetVersionHistory`, was empty), so replacing it would have
  destroyed working text-graph `version_notes` capability. Added a "File
  beacon change history" section reusing the existing generic
  `POST /mcp/call` route — no new Rust needed.
- `file_link` wiring extended from 3/27 to 16/27 modalities. Found along
  the way: Text and Code were previously *believed* wired but were
  fake (a comment literally said "In real implementation, this would
  call the pipeline executor" — never did). Now real. 11 modalities
  remain honestly unwired for structural reasons (required fields with
  no honest file-derived default — organism name, LiDAR
  format/coordinate-system, IQ sample-rate; or `detect_modality()` having
  no extension case for 10 sensor/stream modalities at all) — not
  guessed around.

---

## Status

Written 2026-10-17 from 9 forks (2 investigation rounds after a
rate-limit retry, plus a correction round after the first registry check
read the wrong directory), extended same-day with a second 6-fork wave
closing most of §5's open items. All fixes made directly in code, no
build/test run per standing instruction. Logged additively in
`CHECKLIST.md`'s 2026-10-17 entries.
