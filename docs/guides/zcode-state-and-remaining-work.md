# ZCode State Review & Full Remaining Work

A complete review of what the ZCode coordination agent has built this
session, cross-checked directly against source (not taken on the strength
of its own notes alone), plus every remaining to-do across the whole
project — ZCode's own backend queue, this guide's UI/UX findings, and the
decisions only the operator can make. Back to [README](README.md) ·
[master guide](living-graph-field-guide.md).

Generated 2026-09-29, from `CHECKLIST.md`'s full `(zcode)`-tagged history
(2026-09-19 through 2026-09-29), the shared-context MCP server's latest
handoff notes, and direct source verification of the most load-bearing
claims (several corrections below came from that verification, not from
trusting the notes).

---

## 1 · What ZCode has actually built — verified, not just claimed

### MCP tool ecosystem (the biggest single arc)

Five real MCP capability families are live through `/mcp/call`, all
metered, jurisdiction-gated, and ripple-mirrored (per this session's own
fix): **bridges** (67 tools — game-engine/editor integrations, 4 bridges:
Roblox/Unity/Unreal/Blender-style, auto-registering at boot), **terminal**
(2 tools, now with real per-role allowlists — see below), **visual** (3
tools + `yolo_detect`/`yolo_graph`), and the **shared-context** server's
own 16 tools. `docs/TOOLS_REGISTRY.md` documents the full 73+ tool
inventory by family, platform tag, and modality tag.

**Registry persistence verified across a real restart**: "restored 74
persisted tool(s)" in a real boot log — tools survive a host restart, not
just a session.

### Role-based access control — proven live, not just designed

`tools/terminal-mcp/roles.json`: a real per-agent allowlist (coordinator
tier for `zcode`/`claude-code` — git/cargo/node/npm/pip/python3/grep/find/
ss/systemctl/blender; `"*"` tier for everyone else — `ls` only; unknown
agents locked entirely). Proven three distinct ways through real
`/mcp/call` invocations: a coordinator running `python3` succeeded; an
unknown agent attempting `pip` was refused with a role-specific message;
the same unknown agent running `ls` was allowed. Two real bugs caught and
fixed in the same arc: a body-scoping crash on the first `exec_allowed`
call, and a missing `fileURLToPath` import that would have crashed on
first roles load.

**Caveat, confirmed by direct check (see §3 below):** this is authorization
without authentication. Role identity is "the same agent identity the
host already meters and gates on" — a string, not a cryptographically
verified caller. Ed25519 caller identity is designed but not built (still
open, §3.1).

### Modality revival arc

| Modality | State (verified) |
|---|---|
| Text (100) | Live — grammar edges typed, 7 confetti-protected extractors |
| Code (101) | Live — Calls/Extends/Implements wired |
| Math (105) | Live — real citation edges |
| Image (102) | **Revived this session** — never compiled before; now Analyze+CreateGraph+ZSEI persistence, container 40224 verified; then extended with the YOLO detection registry (below) |
| 3D (109) | **Revived this session** — first compile ever, tested against a real Blender scene via the live bridge (container 40232, 5 nodes/4 edges) |
| Audio (103) / Video (104) | **Compiled this session** (first ever) with persistence wired — **not yet live-smoke-tested** (see §3.5, still genuinely open) |
| Chemistry / DNA / EEG + 19 more | Still old-CLI, zero persistence — same mechanical recipe as the above, ZCode's own estimate ~30 min each |

Net: 5 of 27 modalities revived or newly compiled this arc; 22 remain in
stub state.

### YOLO detection → live graph expansion (the operator's "3 years of
craftsmanship" moment, per ZCode's own framing)

Full stack proven, not partially: `zsei_data/detection_models/registry.json`
(data-driven — adding a model is adding a JSON entry, no code change) →
`yolo_detect`/`yolo_graph` tools → a dedicated venv
(`tools/visual-mcp/.venv`, ultralytics 8.4.165, pointed at via
`OZONE_YOLO_PYTHON` — **not** system Python, after the operator correctly
rejected a first `--break-system-packages` attempt) → real detections on
`zidane.jpg` (2 persons, confidence 0.836/0.819) → **real graph nodes**,
persisted as ZSEI container 40277, independently verified via
`GetContainerContent`: `Image`/`person`/`person`/`Background` nodes, 3
`Contains` edges + a real `Overlaps` spatial edge computed from the actual
bounding-box overlap, keyword `person` graph-searchable. This is the full
loop the operator's vision described — not detections returned to a
caller, detections landing *in the graph*.

### Capture unification (the operator's "everything captured" doctrine)

Four uniform call-capture stores now exist, each a truthful append-only
log: S10 `decision_review.jsonl` (every gate review), S11
`zero_shot_calls.jsonl` (every host zero-shot), S12
`pipeline_zero_shot_calls.jsonl` (every modality-pipeline model call), S13
`tool_calls.jsonl` (**new this session** — every `/mcp/call`, closing the
asymmetry where tool calls had a usage-ledger row and a graph mirror but
no per-call capture record). No read route exists yet for S13 (§3.6).

### Bug fixes landed independently of this session's own CC-side work

- `persist_insight`'s untyped-extra-JSON-keys bug (found by an earlier CC
  fork, insight 40194) — fixed via a real content-pointer convention
  (`src/consciousness/review.rs:273-301`).
- Blueprint content ("B17"): 16 blueprint files existed under an old
  schema at the wrong path; migrated to the canonical path with the
  correct `BlueprintStep` shape, plus a reuse-path fix so blueprint steps
  actually load from the content file instead of a typed struct that
  silently dropped them.
- Task-lifecycle ripple (R5's top gap — see §2 of
  [coordination-ripple-gaps.md](coordination-ripple-gaps.md) for the full
  detail): **closed**, real scoped graph events now fire on every task
  creation/completion/failure/status transition.

### Corrected along the way (worth noting as a pattern, not a knock)

Two real operator corrections this arc, both actually applied rather than
argued with: (1) a `--break-system-packages` pip install was killed
mid-flight and redone via a proper venv; (2) visual capture was
initially scoped as the MCP directly exec'ing a screen-grabber, corrected
to a layered model (tools → modality pipeline → contract outputs, with
`visual_ingest` as the sanctioned path for any image, and Wayland's
default refusal of direct grabs respected rather than worked around).

---

## 2 · Corrections to ZCode's own most recent "remaining" claims

Independent verification (this document's whole point) found ZCode's own
latest handoff slightly stale in one place, worth flagging honestly rather
than repeating:

**The host's `/mcp/call` body limit is already raised — this is DONE, not
remaining.** ZCode's most recent note (id `mum6xc27fk3d`) lists "host
DefaultBodyLimit raise (2MB→20MB...)" under "Remaining." Direct source
check: `src/grpc/mod.rs:2066-2069` already applies
`axum::extract::DefaultBodyLimit::max(20 * 1024 * 1024)` as a
router-wide layer, with a comment dated 2026-09-29 explaining exactly why
(image/YOLO payloads are multi-MB). This item is closed — most likely
ZCode fixed it and then reused an older draft of the handoff note, or
fixed it in the same window the note was written. Either way: don't
re-do it.

Everything else in ZCode's queue below was independently re-verified as
still genuinely open (not just re-stated from its notes).

---

## 3 · Full remaining work, deduplicated and prioritized

Merging ZCode's own stated next-build queue, this guide's R1–R5 findings,
and items surfaced by ZCode's most recent work that weren't in the
original guide. Verification method noted per item — several were
re-checked directly against source for this document, not carried over.

### 3.1 `/orchestrate` + `/mcp/call` caller identity (Ed25519 sessions)
**Status: confirmed still open** (grepped `src/grpc/mod.rs` for
`Ed25519`/`ed25519`/`caller_identity`; only hits are an unrelated
`session_token` field used elsewhere for device pairing, not wired into
MCP/orchestrate auth). The role-enforcement system (§1) is genuinely live
but authorizes on a bare agent-name string — anyone who can set that
string inherits the role. This is the natural next step for the role
system to mean what it currently implies.

### 3.2 `/config/set` never ripples
**Status: confirmed still open**, re-checked directly this pass —
`set_config` (`src/grpc/mod.rs:1229-1524`) has zero `mirror`/
`context_mirror` calls anywhere in its body. See
[coordination-ripple-gaps.md](coordination-ripple-gaps.md) for detail.

### 3.3 `taskCreate` in `server.js` doesn't mirror
**Status: confirmed still open**, re-checked directly — only
`pushActivity`, no `mirrorContext`. Now a small, mechanical fix (the
Rust-side task-lifecycle ripple it depends on is done).

### 3.4 Keep-warm llama-server for BitNet
**Status: confirmed still open** (grepped the whole workspace for
`keep_warm`/`keep-warm`, zero hits). BitNet's 25-40s cold-load per call is
still paid every time; OpenRouter's 17x throughput advantage (measured:
300.53 tok/s vs. 2.9 tok/s) makes this lower urgency than it would
otherwise be, but it's still the real fix for BitNet-as-backstop latency.

### 3.5 Audio (103) / Video (104) — compiled but not live-tested
**Status: confirmed still open.** Both pipelines compiled for the first
time this session with persistence wired (same pattern as image/3D), but
no live `CreateGraph` smoke test has been run against either — ZCode's
own queue lists this as pending "post-restart" across two separate notes
without a follow-up confirming it happened.

### 3.6 S13 read route (`GET /capture/tool-calls`)
**Status, 2026-09-29: backend route still open; UI side now built ahead of
it.** ZCode asked directly whether to build this — agreed, and the
frontend was built immediately rather than waiting: `fetchToolCalls`
(`ui/src/data/captureData.ts`), `classifyToolCall`
(`ui/src/views/capture/captureStatus.tsx`), and a new `ToolCallPanel.tsx`
registered as a "Tool calls" capture-view tab, all matching the real S10
row shape confirmed directly against the writer in `grpc/mod.rs`. Will
work the moment the route lands — no further UI change needed. One real
bug caught and fixed along the way: the Electron IPC HTTP bridge
(`ui/electron/main.js`'s `rawRequest`) never checks HTTP status codes, so
a 404 resolves with a raw string instead of throwing — `fetchToolCalls`
defensively validates the response shape so this doesn't surface as a
confusing crash once pointed at the real route today.

### 3.7 Chemistry/DNA/EEG + 19 more modality revivals
**Status: confirmed still open**, large but mechanical. ZCode's own
estimate: ~30 minutes each via the now-proven recipe (fix the CLI
contract, add `reqwest`, wire ZSEI persistence, fix the never-compiled
move-bugs). 22 modalities remain.

### 3.8 Reverse-engineering guide build-out
**Status: newly designed this session, zero built.**
`docs/REVERSE_ENGINEERING_GUIDE.md` lays out a full 5-surface
(wire/binary/visual/storage/behavior) observation architecture with a
mobile extension — but the build order it names (RETarget container →
wire collector → visual loop → probe runner → graduation → mobile
template) is entirely unbuilt. Flagging this as its own initiative, not a
quick win — it's a new capability area, not a bugfix.

### 3.9 UI/UX items from this guide's R1–R3, still open
Everything in the [master punch list](living-graph-field-guide.md#master-punch-list)
except item 3 (now done — see above) is still open. Highest-leverage
remaining items from that list, restated here for one-stop visibility:
- Design-token cleanup (266 hardcoded hex occurrences across 51 UI files)
- `@xyflow/react`/`shiki`/`katex`/`lucide-react` adoption for Graph
  View/CodeViewer/MathViewer/icons
- The file-beacon background task (design complete in
  [file-beacon-design.md](file-beacon-design.md), zero code written)
- `user_id:0`-vs-`1` fallback decision
- Registering a real `FileReference` for the first populated QA pass
- Accessibility (11/73 files touch aria/role today)

### 3.10 Jurisdiction/simulation results not persisted to the task record
**Status: still open, partially mitigated.** R1's H5/H6 finding — the
durable task record still doesn't store jurisdiction/simulation results
after the response returns. Task-lifecycle ripple (now closed, §1) means
a *live* observer can see status transitions as they happen, but the
record itself, once written, still lacks this data for later inspection.

---

## 4 · Decisions only the operator can make

Not bugs, not missing code — real choices ZCode and CC have both
explicitly flagged as blocked on the person, not the agents:

1. **OpenRouter model pinning.** The free auto-router is confirmed (via a
   real measured test) to be a lottery — it routed a content-safety model
   for an unrelated primes-generation task. With real $10 credits
   available, specific models can be pinned per call class (blueprint
   assignment, simulation, etc.) — a real optimization lever, but it's a
   cost/quality tradeoff only the operator can set.
2. **Data cleanup**: 231 real jurisdiction placeholder entries, and the
   `user_id:0`-vs-`1` fallback inconsistency that blocks most UI panels
   from showing real data (§3.9 above and R1's #4).
3. **Visual capture's sanctioned source**, partially decided already —
   the operator's direction (Electron `desktopCapturer` → `visual_ingest`
   as the recommended bridge, direct X11 grab as an explicit opt-in only)
   is recorded, but whether the actual Electron-side capture bridge gets
   built is still an open scope call, not yet committed to a build queue.

---

## 5 · One-paragraph summary, if you only read one section

ZCode has been extremely productive this session: a real 73+-tool MCP
ecosystem with working persistence and now real per-role access control,
three modalities revived from never-having-compiled to fully graph-backed
(image, 3D, and the YOLO-detection-to-graph loop that's the clearest proof
the whole architecture works as designed), a uniform capture-logging
doctrine across every call family, and — as of today — the single highest-
priority gap from this guide's own coordination review (task-lifecycle
ripple) closed and verified. What's left splits cleanly into three
buckets: mechanical backend work with a proven recipe (19 more modality
revivals, two smoke tests), real but small remaining ripple/auth gaps
(`/config/set`, `taskCreate`, Ed25519 identity), and UI/UX work that's
entirely CC's side of the ledger (design tokens, library adoption,
accessibility) — none of which blocks the other.

---

## 6 · Precise new-capability-vs-UI gap map (2026-09-29, follow-up pass)

The operator asked to "show more insights, capture everything new" after
seeing three live bugs (orchestration steps not updating, the search bar
overlapping the header, the consciousness-disabled banner rendering
outside its box). A dedicated read-only review fork mapped exactly which
of ZCode's new backend capabilities have UI coverage, with real file:line
citations both sides. Verdicts:

| Capability | Backend | Frontend | Verdict |
|---|---|---|---|
| MCP tool registry (73+ tools) | `GET /mcp/tools` real, `src/grpc/mod.rs:2759-2775` | `ToolsPanel.tsx` polls + renders a table | **Partial** — no family grouping/health column |
| YOLO/image detection | real, container 40277 proof | `Modality` type had no `"image"` value at all | **Was zero — now fixed, this pass** |
| S10-S13 capture stores | S10/S11/S12 routed; S13 write-only, no GET route | S10/S11 consumed; S12 has a route but no UI consumer; S13 nothing | **S10/S11 full, S12 zero despite a working route, S13 zero both sides** |
| Task-lifecycle ripple | real, `emit_task_ripple` | `TaskDetailPanel.tsx` only polled, never subscribed | **Was zero — now fixed, this pass** |
| Per-role terminal enforcement | real, `roles.json` | zero UI visibility | **Zero, unaddressed** |
| Bridge/MCP-server health | bridges get real heartbeat | `ConnectedAgents.tsx` shows bridges; terminal-mcp/visual-mcp have no health surface | **Partial, unaddressed** |
| `jurisdiction_gate` result | real on `/mcp/call` + `/orchestrate` | `JurisdictionGateSection.tsx` exists but is fed from `/task/get`, which never carries the field | **Structurally zero — display built, data path missing** |

### Fixed this pass (frontend only, both `npx tsc --strict` clean and a full `vite build` clean)

- **Task-lifecycle ripple → `TaskDetailPanel.tsx`**: now subscribes to
  `graphEventClient`'s `onEvent`, filters `container_type === "Task" &&
  container_id === activeTaskId`, and triggers an immediate refetch on a
  match — the existing 2s poll stays as a resilient backstop, not replaced.
- **Image modality graph rendering**: `Modality` extended to include
  `"image"` (`graphViewTypes.ts`), `graphViewData.ts`'s path-prefix
  detection extended for `graphs/image_*`, and new
  `graphRenderers/{imageNodes,imageEdges}.ts` built against the real
  `ImageNodeType`/`ImageEdgeType` construction sites in
  `assets/pipelines/modalities/image/main.rs` (6 of 8 node types real:
  Image/Object/Region/Text/Face/Color; 6 of 15 edge types real:
  Contains/Overlaps/LeftOf/RightOf/Above/Below — Composition/Quality nodes
  and every other edge variant are schema-only, rendered dashed/dimmed per
  this plugin family's existing doctrine). Color nodes render filled with
  their own real detected hex value. A YOLO-detected object landing in the
  graph (e.g. container 40277's `person` nodes + `Overlaps` edge) now has
  a real, non-generic visual instead of falling through to the default
  gray circle.

### Left for ZCode — extends their own recent work

**Persist `jurisdiction_gate` onto the Task record.** Both halves already
exist (the backend computes it, `JurisdictionGateSection.tsx` renders it)
— only the link between them is missing: nothing writes the field onto a
`Task`/`TaskInfo`, so `/task/get` never carries it. This is `task/mod.rs`
territory, which ZCode owns after building the lifecycle-ripple work — a
natural, small extension rather than a new area. Trust/safety-visible gap:
today there's no way to look back at a completed task and see whether a
jurisdiction rule fired on it.

**Still open, unaddressed this pass**: S13's missing `GET` route (ZCode
already offered to build this), S12's UI consumer, per-role visibility in
the UI, and terminal-mcp/visual-mcp health surfacing.
