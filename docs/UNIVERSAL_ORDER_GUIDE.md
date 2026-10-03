# The Universal Order — native project management, personal assistance, and the everything-is-one timeline

> Eleventh doctrine doc. Extends TOOLS_PIPELINES_MCP_GUIDE §9 (universal
> native task ordering) into the full system the operator specified
> (2026-09-29, referencing Monday.com-style management): **a native
> notes/checklist/personal-assistant/project-manager where everything is
> one** — todos, code work, file changes, meetings, follow-ups, notes,
> external calendars — aggregated per workspace/project and globally,
> ordered by dependencies AND by time (due dates, follow-ups, meetings).
> External APIs (Google Calendar, Zoom, etc.) join through the standard
> MCP/bridge contract. New chats start inside this order. The codebase
> gets its own lane in the same timeline. AMT expansion identifies and
> maintains the order.

---

## 1. The principle: everything is one order

The system already has the skeleton (all landed, verified):

| Piece | State |
|---|---|
| Task store (per-task status, workspace/project scoping) | ✓ live |
| Task-lifecycle ripple (created/completed/failed/paused move the graph) | ✓ live |
| `/order/global` (aggregate view: live/paused/queued/interrupted/done) | ✓ live |
| `paused`/`interrupted` native states | ✓ live |
| AMT candidates register as real tasks (discovery → execution) | ✓ landed |
| File-change events (beacon → ripple) | ✓ live |
| Code call order (per-file entry points + sequence) | ✓ landed |
| Coordination events (notes/handoffs/claims as graph containers) | ✓ live |
| Insight envelope (order state in every tool call's reply) | ✓ live |

**What's missing is the rest of life**: due dates, meetings, follow-ups,
personal notes, external calendars. The design adds them as FIRST-CLASS
items in the SAME order — not a second system, not an integration bolt-on.

**The hierarchy is already native**: Workspace → Project → AMT →
Blueprint → Steps. A personal todo, a meeting, a code fix, and a project
milestone are all steps at some level of that hierarchy. One order,
many lenses.

---

## 2. The item model (one shape, many kinds)

Every item in the order is a task record (existing store, additive
fields):

```text
Task {
  ...existing fields...
  kind:        "todo" | "meeting" | "followup" | "note" | "code" |
               "milestone" | "external",     // additive, defaulted "todo"
  due_at:      Option<u64>,   // unix secs — the WHEN axis
  remind_at:   Option<u64>,   // follow-up nudge time
  recurrence:  Option<Recurrence>,  // daily/weekly/... recurring items
  meeting_url: Option<String>,// zoom/meet/jitsi link for meeting items
  external_ref: Option<{       // provenance when synced from outside
    provider: "google-calendar" | "zoom" | ...,
    external_id: String,
    last_synced: u64,
  }>,
  checklist: Vec<{ text, done }>,   // sub-items on any task
  note_body: Option<String>,        // notes are tasks too (kind "note")
}
```

Rules:
- **Aggregation is derived, never copied**: `/order/global` joins task
  records by scope (workspace/project) and orders by (due_at, priority,
  created) — the same derived-view discipline as the existing route.
- **Time buckets are computed, not stored**: overdue / today / this week /
  upcoming / someday — derived from due_at at read time.
- **Meetings are tasks with a time and a link** — they appear in the same
  day view, can have checklists (agendas), follow-ups (auto-created child
  tasks), and notes (the meeting's output).
- **Notes are tasks** (kind "note") — a note without a due date is a
  captured thought; promote it (add due_at / convert to todo) and it
  enters the active order. Capture-then-triage, the personal-assistance
  loop.
- **Code work is tasks** — already true (amt-loop tasks, orchestration
  tasks, session work are all source-tagged task records).

---

## 3. External APIs — the connector contract

Google Calendar, Zoom, and any external system join through the SAME
pattern as engines and RE targets (TOOLS_PIPELINES_MCP_GUIDE §4): register
→ connect (OAuth per account; tokens stored per user, never in the graph)
→ expose tools → sync into the order.

**Google Calendar MCP** (`tools/gcal-mcp/`):
- OAuth 2.0 installed-app flow; tokens per user in the host's auth store
  (never in graph containers, never in plaintext)
- Tools: `gcal_list_events` (time range), `gcal_create_event`,
  `gcal_update_event`, `gcal_delete_event`
- **Sync**: events in the synced range become/UPDATE task records (kind
  "meeting", external_ref set, external_id = Google event id) — sync is
  idempotent on external_id, deletions mark the task cancelled (never
  deleted — history is preserved)
- Pull cadence: on demand + interval; pushes (webhooks) later

**Zoom MCP** (`tools/zoom-mcp/`): same shape — `zoom_list_meetings`,
`zoom_create_meeting`, `zoom_join_url`; meetings sync as meeting tasks
with meeting_url set.

**The general rule** (any external API): a connector MCP with OAuth,
idempotent external_id-keyed sync into task records, and the connector's
native tools registered into `/mcp/tools`. The order never stores foreign
schemas — external items land as tasks with external_ref provenance.

**Conflict rule**: the graph record is authoritative for Ozone-side
edits; the external system for its own fields. Last-synced-wins per
field group (title/time from outside; checklist/notes/follow-ups from
inside), surfaced honestly in the order view when both sides changed.

---

## 4. The surfaces (how a human meets the order)

### New chat / session start
`context_summary` + `/order/global?project=<current>` compose the opening
context: what's live, what's due, what's paused. The assistant opens
already inside the order — no "what was I doing" ever again.

### The day view (personal assistant)
`GET /order/global` gains time filters (`?due=today|overdue|week`) and
the day timeline: meetings at their times, todos by due, follow-ups due,
code work in progress. The insight envelope's `order` block extends with
the same buckets.

### The project view (project manager)
Per-project: the plan (AMT → blueprint → steps), the todos, the
milestones, the meetings, the file changes (beacon), the code call
order — one lens per project, all from the same derived view.

### Capture surfaces
- Chat: "note that X" / "remind me to Y by Friday" → note/task creation
  through the assistant (S13-captured like every call)
- Beacon: file changes → events (already live)
- External sync: calendar/meeting changes → task updates (connector)
- AMT: expansion candidates → tasks (already landed)

### UI/UX (capture for the UI family)
- Day view panel (today/overdue/upcoming + meetings inline)
- Project board (existing /order/global buckets per project)
- Quick-capture input (note → task promotion flow)
- Meeting cards (link, agenda checklist, follow-up children)
- All reading the derived views — no duplicated state in the UI

---

## 5. Build order (staged, additive)

```
Stage 1 (schema): additive task fields — kind, due_at, remind_at,
          recurrence, meeting_url, external_ref, checklist, note_body
          (all serde-defaulted; zero migration)
Stage 2 (views):  /order/global gains time buckets + kind filter;
          day view + project view reads
Stage 3 (connectors): gcal-mcp + zoom-mcp (OAuth, idempotent sync)
Stage 4 (surfaces): quick-capture in chat; meeting cards; follow-up
          auto-creation ("meeting ended → create follow-up tasks")
Stage 5 (intelligence): AMT expansion proposes re-planning when the
          order slips (overdue cascade detection); the assistant surfaces
          the proposal, never auto-reorders the operator's plan
```

---

## 6. The AMT tie (operator: "with the AMT expansion identifying the order")

The AMT is the PLAN; the order is the PLAN MOVING. Tie-ins:

- Task ripples wake AMT expansion (landed) — progress feeds planning
- Order slippage (overdue cascade) is a REAL SIGNAL the AMT expansion can
  consume: the plan needs re-cutting → expansion candidates propose the
  re-cut; the assistant surfaces the proposal
- Call order (code) feeds code-task ordering: the next task for a
  codebase follows its own call graph (entry points first)
- External events (meetings) constrain the schedule the plan must fit —
  the day view shows plan-vs-meetings honestly

---

## 7. Non-goals (stated)

- NOT a second task system — the existing task store is the only store
- NOT auto-managing the operator's plan — proposals yes, reordering no
- NOT storing foreign schemas — external items land as tasks with refs
- NOT blocking on external APIs — the order is complete without them;
  connectors enrich, never gate
