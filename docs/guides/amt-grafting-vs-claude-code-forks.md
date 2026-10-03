# AMT Grafting vs. Claude Code Forks — Full Comparison

Two systems named "fork" show up in this project and they are not the same
thing. This document explains each in full, with real source citations, and
then compares them directly.

- **Ozone-Studio's AMT fork/graft system** — a durable, persisted,
  automatically-reconciling knowledge-graph branching mechanism. Lives in
  `src/orchestrator/amt.rs`.
- **Claude Code's fork mode** — the `Agent` tool's `subagent_type: "fork"`,
  an ephemeral, conversational context-inheritance mechanism used throughout
  this session to dispatch parallel review/build work.

They solve different problems. Neither is a smaller or larger version of
the other. Reading one as a model for extending the other is the mistake
this document exists to head off.

---

## Part 1 — Ozone-Studio's AMT fork/graft system, in full

### 1.1 What an AMT is

An AMT (the orchestrator's intent/branch/detail tree) is built once per
orchestration request in `build_amt` (`src/orchestrator/amt.rs:9-101`) —
either by graph-native traversal of sentence/grammar evidence
(`build_amt_from_graphs`) or a legacy per-chunk zero-shot loop
(`build_amt_layer_by_layer`), depending on whether the request's processed
chunks carry real grammar-relationship graph data. Whichever builder runs,
the result is an `AMTNode` tree — root, with `IntentCapture`/`BranchCapture`/
`DetailCapture` content folded into nodes with `children: Vec<AMTNode>`,
`relationships: Vec<AMTRelation>` (`target_id`, `relation_type`,
`confidence`), and a `verified: bool` flag that is a real provenance signal,
not a fabricated score (see the doc comment on `find_unverified_node`,
`amt.rs:463-465`).

### 1.2 Persistence: an AMT becomes a real, queryable container

Before this session, `state.amt` lived only in the transient per-request
`OrchestrationState` and was discarded the moment `orchestrate()` returned —
only unrelated methodology/blueprint containers were ever persisted, never
the AMT itself. `persist_amt_container` (`amt.rs:108-360`) fixes that: it
writes the full node/edge tree to a real file under `zsei_data/amt/` and
creates a real ZSEI container referencing it via `store.create_container`,
parented under the request's real `project_id` when one exists
(`amt.rs:271-272`).

### 1.3 The island model — MAIN vs. FORK

This is the core of the "grafting" design (`amt.rs:152-172`):

- The **first** AMT ever persisted for a project is that project's **MAIN**
  tree. It gets the keyword `amt-main`.
- **Every subsequent generation** — i.e., every later prompt against that
  same project — is a **FORK**: a new island, tagged with the keyword
  `amt-fork-of:<prior_container_id>`, and given a real `Continues` relation
  (`AMTRelationType::Continues`) pointing at the prior generation's
  container id.
- "Prior generation" is found by walking the project container's
  `child_ids` in reverse and taking the most recent child whose
  `container_type == "Derived"` (`amt.rs:124-151`) — i.e., the most recent
  AMT, not necessarily the literal MAIN, so lineage chains generation →
  generation rather than every fork pointing back to the same root.

### 1.4 Two relation systems, kept in sync on purpose

There are two places lineage gets recorded, and the code is explicit about
why both exist:

1. **The AMT's own `relationships` field** (`AMTRelation`, AMT-internal
   JSON) — gets the `Continues` edge.
2. **The ZSEI container's generic `context.relationships` array**
   (`RelationType::ForkOf`, `amt.rs:180-188`) — because
   `TraversalEngine` (the thing that answers real graph queries) only ever
   reads `Context.relationships`, never the AMT-only JSON blob. Before this
   was added, this container hardcoded an **empty** relationships array and
   fork/main lineage was completely invisible to any traversal or relevance
   query, even though the AMT blob itself "knew" about it.

A **reverse edge** is written too: `ContinuedBy` is pushed onto the *prior*
container's own `context.relationships` (`amt.rs:274-337`, read-modify-write,
idempotent — it checks for an existing `(ContinuedBy, new_id)` edge before
appending). Before this, the `amt-fork-of:<id>` keyword was one-directional:
the prior container had no way to know a fork of it existed at all.

### 1.5 File naming is project-scoped, deliberately

AMT files are named `amt_p{project_id}_{millis_timestamp}_{node_id}.json`
(`amt.rs:199-202`). The project id is baked into the filename specifically
so that `merge_back_to_main`'s scan (next section) can find *this project's*
main tree by prefix-matching `amt_p{project_id}_`, rather than scanning
every AMT file globally and grafting one project's branches into whichever
project happened to build the very first AMT on the whole host. Legacy
files with no project id (`amt_{ts}_{id}.json`, predating multi-project AMT
traffic) are deliberately skipped by merge-back rather than guessed at.

### 1.6 `merge_back_to_main` — the actual graft, in full

`amt.rs:365-442`. Called at the end of every `persist_amt_container`
(`amt.rs:355-357`), for every request that has a `project_id`. Best-effort:
every failure path just logs and returns, never propagates — a failed graft
never breaks the orchestration response itself.

Step by step, exactly as written:

1. **Find main.** Scan `zsei_data/amt/` for files starting with
   `amt_p{project_id}_`. Parse the millisecond timestamp out of each
   filename (`name.trim_start_matches(prefix).split('_').next()`) and keep
   the one with the smallest timestamp — that's main, by construction,
   since main is always the first AMT ever persisted for the project.
   Sub-second precision here is intentional: `fs::metadata().created()`
   truncates to whole seconds, which isn't fine-grained enough to reliably
   order same-second AMT generations.
2. **Load and parse it** as an `AMTNode`. Any read or parse failure logs a
   warning and returns — no partial/corrupt graft.
3. **Collect existing content.** A recursive `collect_contents` walk builds
   a `HashSet<String>` of every node's `content` field currently in main —
   the dedup key.
4. **Graft.** For each **direct child** of the fork's root (not a deep walk
   of the whole fork — only the fork's top-level branches are graft
   candidates), graft it onto main's `children` **only if**
   `child.verified && !existing.contains(&child.content)`. Both conditions
   matter: an unverified branch never grafts regardless of novelty, and a
   verified-but-duplicate branch never grafts regardless of verification.
5. **Persist**, but only if `grafted > 0` — an unchanged main tree is never
   rewritten.

This is a real, automatic, content-deduplicating merge — not a queue of
proposed changes a human approves. It runs on every project-scoped request,
silently, as a side effect of that request's own AMT persisting.

### 1.7 What triggers deeper forks: re-expansion candidates

Grafting only has something to graft once a fork *diverges* from main in a
way worth keeping. Two real, observable signals feed the separate
re-expansion loop (`orchestrator/amt_loop.rs`, boot-spawned in `src/lib.rs`)
that decides what to deepen next:

- **`UnverifiedNode`** (`amt.rs:452-461`) — a freshly-persisted AMT still
  has a node with no source provenance behind it (`find_unverified_node`,
  a depth-first walk, `amt.rs:466-476`). Only recorded for project-scoped
  requests; a one-off chat AMT with no `project_id` has nowhere to be
  "revisited" later.
- **`ThinTree`** (`amt.rs:79-91`) — the persisted tree has **no branches at
  all** (`amt.children.is_empty()`). Nothing unverified to point at, but
  exactly what the re-expansion loop exists to deepen: the loop's target
  selection treats a childless root as the deepening target.

Both routes append to a shared `amt_candidates` store
(`orchestrator::amt_candidates::append`), which also unconditionally
records every persisted AMT's route (`"Continuation"` or `"InitialBuild"`,
`amt.rs:339-348`) regardless of verification state — a full audit trail of
every AMT generation, not just the ones that triggered re-expansion.

### 1.8 Summary of the real mechanism

A project's knowledge grows like this: prompt 1 creates MAIN. Prompt 2
creates a FORK, linked to MAIN via `Continues`/`ForkOf` (forward) and
`ContinuedBy` (reverse), and its verified, novel top-level branches are
immediately, automatically grafted into MAIN's own content file. Prompt 3
does the same against whichever generation is now most recent. Over many
prompts, MAIN accretes every verified, non-duplicate branch any fork ever
produced, while every individual fork generation remains independently
inspectable and walkable via the lineage graph
(`ui/src/data/amtLineage.ts` is the UI's read path for this).

---

## Part 2 — Claude Code's fork mode, in full

Sourced via a real web search this session (5 sources, listed at the
bottom), cross-checked against this session's own empirical behavior
(forks dispatched via the `Agent` tool with `subagent_type: "fork"`
throughout Batches C–J and the R1–R5 research pass).

### 2.1 What it is

A fork is a background subagent that **inherits the parent's full
conversation context verbatim** — conversation history, system prompt,
tool access, and model settings — rather than starting cold like a normal
`subagent_type` agent. It also **shares the parent's prompt cache**, which
is why forking is cheap relative to spinning up a fresh agent that has to
re-derive context from scratch.

### 2.2 Rollout status

GA (generally available) since Claude Code v2.1.117; **default-on** in
interactive sessions since v2.1.232. Not an opt-in experiment at this
point in the product's life — it's the normal path for delegating
sub-tasks that don't need their own fresh context.

### 2.3 Execution model: background-only, no foregrounding

A fork always runs in the background. The parent cannot pull a running
fork into the foreground to interact with it directly — the parent gets
the fork's result only when the fork finishes (or via the fork's own
reporting mechanism), and the fork's own tool-call noise never enters the
parent's context. This is the actual mechanism behind this session's
repeated pattern of dispatching several forks and then continuing other
work while they ran.

### 2.4 No nested forks — confirmed by a real, public issue

A fork spawning its **own** nested fork does not behave like a normal
parent→child relationship. It produces an **unsupervised background
process** with no clean parent-child accounting. This is documented in a
real, public GitHub issue —
[anthropics/claude-code#81035](https://github.com/anthropics/claude-code/issues/81035),
"Nested Agent(subagent_type:\"fork\") call spawns an unsupervised
background process." This matches this session's own empirical finding,
reached independently before the issue was found: every attempt this
session made to have a dispatched fork itself dispatch a sub-fork behaved
unreliably, which is why the standing dispatch discipline for this session
became "the coordinator dispatches every fork directly; forks never
dispatch forks."

### 2.5 No automatic merge-back — none, of any kind

This is the single largest structural difference from AMT grafting.
There is **no documented mechanism** by which a fork's output is
automatically folded back into the parent's own state. A fork returns a
result (a final message, or — as used heavily by this session's own
practice — a `note_add` call into the shared-context MCP server for a
durable handoff record); the **parent's own reasoning** is what decides
what to do with that result: apply an edit, discard a claim, ask a
follow-up. There is no dedup pass, no verified-vs-unverified gate, no
"only graft what's novel" logic anywhere in the fork mechanism itself —
all of that, when it happens at all, is something the parent does by hand
each time.

This absence is precisely what made this session's own standing discipline
necessary in the first place: *never trust a fork's sweeping self-report
without independent verification* (a real diff, a real build, a real
direct read of what the fork actually touched) was adopted specifically
because nothing in the fork mechanism itself performs that check.

---

## Part 3 — Side-by-side

| | Ozone-Studio AMT fork/graft | Claude Code fork mode |
|---|---|---|
| **What forks** | A knowledge-graph branch (an `AMTNode` tree persisted as a ZSEI container) | A conversational agent (inherits history/tools/model) |
| **Durability** | Persisted to disk (`zsei_data/amt/*.json`) and to a real, queryable ZSEI container | Ephemeral — exists for the life of the background task; no persisted graph structure of forks themselves |
| **Lineage tracking** | Explicit, bidirectional, real relation types (`Continues`/`ForkOf` forward, `ContinuedBy` reverse), walkable by `TraversalEngine` and the UI's `amtLineage.ts` | None built in — a fork's relationship to its parent is conversational (\"I was dispatched to review X\"), not a graph edge anywhere |
| **Merge-back** | Automatic, on every persist, content-deduplicated (`merge_back_to_main`) | None. Manual — the parent's own reasoning decides what to keep |
| **Quality gate on merge** | Real: `verified && !existing.contains(content)` — a structural, code-enforced gate | None built in — this session's own \"never trust a self-report\" discipline is a substitute the *user* imposed, not something the tool enforces |
| **Nesting** | N/A — grafting is project-scoped, not fork-of-fork | Explicitly broken / unsupported (issue #81035); this session's rule is forks never dispatch forks |
| **Scope of what's shared** | One project's accumulated intent/branch/detail structure across many requests, over time | One parent conversation's full context, for one background task's duration |
| **Problem it solves** | Durable, structural, auto-reconciling knowledge-graph branching across many sessions and requests | Ephemeral, cheap, cache-sharing parallel delegation within one session |

## Part 4 — What each could learn from the other (design notes, not a build plan)

These are observations, not commitments — flagging them here because the
comparison surfaces them naturally, not because either is scoped for this
session:

- **AMT grafting has no equivalent of Claude Code's prompt-cache sharing.**
  Every fork this session dispatched was cheap partly *because* it shared
  the parent's cache; a new AMT fork build re-runs real LLM calls from
  scratch (intent discovery, branch discovery, methodology cross-reference)
  with no analogous cache-sharing concept. Not necessarily fixable — the
  two systems cache fundamentally different things (a conversation prefix
  vs. a knowledge structure) — but worth naming.
- **Claude Code's fork mode has no equivalent of AMT's verified-gate
  auto-merge.** If Claude Code ever wanted an automatic "fold this fork's
  findings back into the parent's working state" mode, AMT's
  `verified && !duplicate` gate is a real, working, shipped example of
  what that could look like structurally.
- **AMT's island model (MAIN vs. FORK) has no analog to background-only
  execution** — every AMT fork build actually runs synchronously as part
  of its own orchestration request; there's no "AMT fork running in the
  background while the main request continues" concept today. Whether
  that would even be desirable is a separate, unopened question.

---

## Sources consulted (Part 2)

- [Create custom subagents — Claude Code Docs](https://code.claude.com/docs/en/sub-agents)
- [Nested Agent(subagent_type:"fork") call spawns an unsupervised background process · Issue #81035](https://github.com/anthropics/claude-code/issues/81035)
- [How Claude Code's Subagent Forking Works (2026 Guide)](https://www.getclaudeskills.com/blog/claude-code-subagent-forking)
- [Subtask vs Fork vs Background Agent in Claude Code](https://startdebugging.net/2026/08/subtask-vs-fork-vs-background-agent-in-claude-code/)
- [Claude Code Fork: Subagents That Inherit Context](https://cybrec.com/blog/ai/claude-code-fork/)

## Source citations (Part 1)

All line numbers as of this session's build, `src/orchestrator/amt.rs`:
- `build_amt` — lines 9–101
- `persist_amt_container` — lines 108–360
- Island model / `Continues` relation — lines 152–172
- `ForkOf` mirrored into `context.relationships` — lines 174–188
- File naming, project-scoped — lines 190–206
- `ContinuedBy` reverse edge — lines 274–337
- `merge_back_to_main` — lines 365–442
- `record_amt_reexpansion_candidate` (`UnverifiedNode` route) — lines 452–461
- `find_unverified_node` — lines 463–476
- `ThinTree` route — lines 79–91
