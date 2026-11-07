# UI animation design — fan-out per modality, aggregation → blueprint, AMT expansion, span-out, gates (2026-11-03)

Status: **design only** (no code changed). Everything below was verified by reading the repo; where a claim could
not be verified it is marked **UNVERIFIED**. Hard rule from `AGENTS.md`: *no fabricated metrics* — every animation is
driven by a real captured value and **degrades to a static, labelled state** when that value is absent (§9).

Related: `docs/PER_MODALITY_FANOUT_GUIDE_2026-11-03.md` (what the fan-out computes), `docs/UI_UX_FORK_PLAN.md` (Batch G engines,
Batch J living network), `docs/developer/UI_UX_DEVELOPMENT_GUIDE.md` §"Modality Engines" (l.534), `docs/guides/living-graph-field-guide.md`.

---

## 1. "Modality engine" — what the repo already means by it

| Where | What it says |
|---|---|
| `docs/developer/UI_UX_DEVELOPMENT_GUIDE.md:534-600` | A Modality Engine = an AGI-first application environment (3D Engine "replaces Blender/Gazebo/Isaac"; Code Intelligence Engine = file tree + graph; CAD/Blueprint, Kinematics, Geospatial, Genome, Molecular, Proof/Formula engines). Trigger: ≥2 of {spatial/temporal depth, native-form artifact, AGI works on it, expert tools set expectations}. |
| `ui/src/panels/EnginesPanel.tsx:13` + `ui/src/panels/enginesRegistry.tsx` | The live registry: `GraphNativePanel section="engines"`, "Per-modality graph engines built on the real modality graphs — Code/Math/Text/Image/3D/Audio-Video/Geospatial/CAD have dedicated views, every other modality has the All-modalities table". |
| `ui/src/views/engines/` | `CodeEngine.tsx`, `CodeCallGraph.tsx` (caller→callee over real `Calls` edges), `ProofEngine.tsx`, `VariableScopeTree.tsx`, `TextEntityBrowser.tsx`, `TextGrammarViewer.tsx`, `ImageEngine.tsx`, `ThreeDEngine.tsx`, `CADEngine.tsx`, `GeospatialEngine.tsx`, `TimelineEngine.tsx`. |
| `CHECKLIST.md:5216-5235` (2026-10-18) | "Orchestration-driven live engine switching": backend gave `orchestration_stage` real `project_id/workspace_id`; `store.ts` field `liveEngineActivity` + `ProjectWorkspaceViewer.tsx` inline banner; global activity reuses `notifyTabNeedsAttention`. 26 modality pipelines are real; 3 originally had engine UI (now 11). |
| `ui/src/graphkit/GraphWorld3D.tsx` | The 3D substrate every engine can borrow: `@react-three/fiber` + `drei` (`ui/package.json`: three 0.169, r3f 8.17, drei 9.114), instanced nodes per shape, per-class edge batches (`WorldEdgeClass` in `layout3d.ts:24` = structural / dependency / semantic / cross-modal / lineage / governance / link / file / root), camera fly-to (`useFrame`, 0.85 s ease), **real construction pulses** (`PULSE_SECONDS = 5`, fed by `node_event` frames into the `useGraphKit` store `pulses`). |

**Conclusion:** the per-modality animation belongs *inside the engine layer*, as a "Fan-out lens" that every engine
(and the generic project graph world) can switch on — not a separate app. The substrate already has the three things
needed: a graph provider (`graphkit/graphProvider.ts`, one fetch per project), a renderer with edge classes + camera, and a
pulse channel fed by real events.

## 2. Inventory — animations that exist today

| Animation | File:line | Driven by | Honesty / gaps |
|---|---|---|---|
| Discovery toasts (methodologies gathered, AMT built, blueprint created, ModalityGraph created) | `ui/src/components/DiscoveryEffects.tsx` (keyframes `discoveryToastIn/Out` l.272-278; parses real `orchestration_stage` summaries, stage 3/5/6) | `onOrchestrationStage` + `graph_event` | Real. Hard-coded `CHAT_USER_ID=1` filter. This is the seed of the aggregation→blueprint finale (§4). |
| Creation activity chip | `components/CreationActivity.tsx` (`oz-activity-in`, `oz-activity-glow` l.67-92) | `graph_event` created | Real. |
| Live stage list | `components/OrchestrationStatusPanel.tsx` (STAGE_ORDER 0-13, pulse l.208, fades l.253/279) | `orchestration_stage`, `pipeline_progress` (0/100 only — never a smooth bar, correct) | Stage 14 "Jurisdiction Full Gate" exists on host (`record_stage`) but STAGE_ORDER lists 0-13 → add. |
| Project live-activity banner / tab flash | `App.css` `project-live-activity-pulse/fade-in` (l.6307-6311), `tab-attention-flash` (l.5724); `ProjectWorkspaceViewer.tsx:141`, `services/store.ts:113` | `orchestration_stage` project-scoped | Real. |
| I-Loop dot + "just reflected" glow | `App.css` `iloop-pulse` l.2261, `iloop-reflected-glow` l.2310 | I-Loop reflection arrival | Real; idle pulse is a constant "loop exists" signal (documented as such). |
| Meta orb (breathe/pulse/ring-rotate/listen/mic) | `App.css` `orb-pulse` l.1988, `ring-rotate` l.2021, `orb-breathe` l.5923, `ring-pulse` l.5944, `listen-glow` l.6115, `mic-pulse` l.6141; mic level via `requestAnimationFrame` `MetaPortion.tsx:331`, `hooks/useVoiceIO.ts:74` | voice level / chat state | Real (mic RMS). |
| Gate state dot | `graphkit/indicators.tsx:217` (`pulse 1s` when `active`) | `GateState` (awaiting gate etc.) | Real; header: "No fabricated percentages, no fake pulses". |
| 3D graph world: construction pulses, camera fly-to, selection ring | `graphkit/GraphWorld3D.tsx` (pulse l.216-275, camera `useFrame` l.359-395, ring l.397+) | `node_event` frames, user focus | Real, but today only **3D ComputeSpatial** emits `node_event` (`assets/pipelines/modalities/3D/main.rs:2756`). |
| Confetti | `views/capture/ConfettiViewer.tsx` | **Not an animation**: "confetti" = a model response with >1 JSON candidates (BitNet failure mode) — a raw-response *viewer*. Do not confuse with celebratory confetti. | n/a |
| Generic spin/fade | `App.css` `spin` l.137, `pulse` l.143, `fadeIn` l.1083, `message-in` l.2178, `dialog-in` l.3032, `gnw-pulse/gnw-spin` l.7175/7208 | UI state | n/a |
| Components with **no** motion today | `PausedBanner.tsx`, `ThinkingRow.tsx`, `AwaitingBadge.tsx`, `WorkspaceStatusStrip.tsx` (grep for keyframes/animation/transition: none) | — | candidates in §7. |

**Accessibility gap (verified):** `grep -rn "prefers-reduced-motion" ui/src` returns **nothing**. Every existing animation
ignores the OS setting. A single global guard (§8) fixes all of them.

## 3. What the host actually emits (the data contract) — verified

| Channel | Producer | Wire frame (UI type) | Carries | Gaps for animation |
|---|---|---|---|---|
| `orchestration_stage` | `src/orchestration_events.rs:26` `OrchestrationStageEvent`, `emit` l.254, from `record_stage`/`record_stage_timed` (`orchestrator/mod.rs:5141/5183`) and `emit_marker` (`mod.rs:3646`); forwarded `grpc/mod.rs:4148-4190`; UI `graphEventClient.ts:110` | `{stage 0-14, stage_name, success, summary, duration_ms, project_id, workspace_id}` | stage names: 0 Jurisdiction Gate, 1 Input Capture, 2 Text Normalization, **3 Gather Methodologies**, 4 Initial Graph Creation, **5 Build AMT**, **6 Blueprint Assignment**, 7 Zero-Shot Simulation, 8 Consciousness Gate, 9 Task Creation, **10 Step Execution**, 11 Result Collection, 12 Post-execution, 13 Response Delivery, 14 Jurisdiction Full Gate | (a) `trace_id` is on the Rust event (l.47-49) but **not forwarded** in the wire JSON (`grpc/mod.rs:4159-4170`) → UI cannot join frames to a run; (b) **no `task_id`/`step_index`** on stage frames; (c) stage 10 emits at completion only. |
| Markers via `emit_marker` | `mod.rs:3618-3680` vocabulary; call sites actually present: `amt.rs:3248/3544/3554` (`[5.enrich]`, `[5.xref.*]`), `graphs.rs:994`, `mod.rs:5043` (`[5.think]`) | arrive as stage **5** named `"AMT — <name>"` | methodology enrichment, cross-reference domains, lanes | `[6.tools]`, `[7.0]`, `[8.1-3]`, `[8b]`, `[10.step]`, `[13.0]`, `[5.lanes]` are **defined in `marker_info` but have no emit site** (verified by grep) — so *tools/MCPs aggregated* (`state.applicable_tools`, only `tracing::info!` at `stages.rs:82`) and *per-step start* are **not visible to the UI today**. |
| `graph_event` | `src/graph_events.rs` `GraphEventHub`; frame `grpc/mod.rs` `graph_event_frame` (~l.4098); UI `graphEventClient.ts:52` | `{event: created/updated/deleted/linked/context_trimmed/session_context_trimmed, container_id, parent_id, container_type, source, scope_keywords, timestamp}` | every graph write | `trace_id`/`span_id` exist on the Rust `GraphEvent` (graph_events.rs:35-37) but are **not in the wire frame** (same gap). AMT re-expansion = `updated` on the AMT container (candidate `source` may be empty — choke-point doc l.23). |
| `pipeline_progress` | executor, polled every 500 ms (`grpc/mod.rs:4238-4262`) | `{execution_id, pipeline_id/name, status Queued/Running/Completed/Failed/Cancelled, progress_percent 0|100, task_id, step_index, tokens_used}` | **the only per-step live signal** (task_id + step_index + pipeline) | status-change only; fine for lighting a step node. |
| `node_event` | `orchestration_events.rs:184`, `POST /orchestration/node-event` (`grpc/mod.rs:2161`), forwarded l.~4193-4230; UI `graphEventClient.ts:190` | `{project_id, modality, graph_id, node_id, related_node_id, node_type, operation}` | per node/edge construction | only **3D** posts it (`3D/main.rs:2756`). All other modality pipelines should post it when they add typed edges (also fixes the sparse-capture gap in the fan-out guide §3). |
| `pipeline_location` | `orchestration_events.rs:139 emit_location`, UI type `graphEventClient.ts:158` | file/object a pipeline touches | **server forwarding missing**: `grep -c pipeline_location src/grpc/mod.rs` = 0, and the UI file says no producer exists. |
| **Span-out** | `orchestrator/span_out.rs`, `span_out_for_step` | `tracing::info!` only ("span-out for step …", `items/tokens/fetched/budget_filled/frontier_left`) | — | **not emitted as an event, not persisted.** New: `POST /context/span_out` (added 2026-11-03, `grpc/mod.rs` `context_span_out`, in the current build) returns `items[{id,container_type,name,score,hops,via,tokens,fanout{modality,nodes,edges,entries,sequence_head,class_counts,lines_total,lines_packed}}]` on demand; a **live** feed needs the event in §5. |
| Pause / resume / awaiting / jurisdiction | `/task/pause_info`, `/task/resume_run`, `/order/global` (`paused`, `awaiting`, `paused_for_failure`, `error`), `PausedBanner.tsx`, `AwaitingBadge.tsx`, `JurisdictionGateSection.tsx` | polling, no WS frame | stage 0 / 14 summaries carry `matched=…, warnings=…, confirmations_reviewed=…` (jurisdiction.rs:468/574) | no push for pause/await — UI polls; add one tiny `run_state` frame (§5, E3). |

## 4. The headline: **Fan-out Lens** (per-modality span-out animation)

### 4.1 Concept — "the wavefront"
A step's context is *not* a list — it is a wave leaving the things the step is about. Show exactly that, per modality, with the
same grammar as the engine the user already knows:

1. **Seeds ignite.** Seed containers (score 1.0: AMT, attached-file graphs, prompt modality graphs; 0.5 keyword hits) light up
   with a ring whose brightness = seed score.
2. **Entries flash.** Inside each seeded modality graph the **ENTRIES** (`fanout.entries`) flash first — code: `main`,
   `helper_parse` (from `analysis.entry_points`); CAD: the document/assembly root; 3D: the Scene; kinematics: the base link.
3. **The call chain travels.** Flow/Order edges light *in `sequence_head` order* as a travelling pulse (a bright dot moving
   along the edge, then the target node blooms) — literally the "call sequence" of that modality: `main → parse → tokenize`,
   `sketch1 → extrude1 → body`, `base → shoulder → elbow → end-effector`. Pulse speed is constant (no fake easing of data);
   order is real.
4. **Scaffolding fades in.** Structure edges (`Contains`…) appear as faint thin lines *after* the flow, pulled to the graph hub.
5. **Bridges arc.** Cross-modal edges (`ParametricGeneratesMesh`, `DescribedBy`, `ImplementedInCode`, …) draw as magenta
   arcs to the *other* modality's cluster — GraphWorld3D already colours cross-modal magenta; the wave literally crosses
   modalities (CAD → 3D mesh → text description).
6. **Score decays visibly.** Node glow/size = `score` (seed × relation weight × confidence); thickness of an edge = its class
   weight (Order .95 · Structure .90 · Flow .88 · CrossModal .85 · Spatial .75 · Similarity .70 · Provenance .35). Faint
   far nodes = honest low score.
7. **The budget fills.** A ring around the focused graph (or a "context window" gauge in the HUD) fills with
   `tokens / budget`; when `budget_filled` the wave stops at a crisp frontier and the unreached frontier (`frontier_left`) stays
   dark with a count — "N more reachable, not packed" — never silently cut (matches the repo's "every cut is visible" contract,
   `graph_event` `context_trimmed`).
8. **Context tray.** Packed items drop into a right-hand tray in packed order with `via` ("12 -Calls-> this"), `score`, `hops`,
   `tokens` — the same list the model receives. Clicking an item flies the camera to it (existing fly-to).

### 4.2 Per-modality choreography (rule = `graph_fanout::classify` + entries)
| Modality engine | Layout the engine already uses / should use | Wave choreography |
|---|---|---|
| Code (`CodeEngine`/`CodeCallGraph`) | file tree left, caller→callee layered DAG | entries (main-like, zero in-degree) flash → `Calls` pulses layer by layer along `call_sequence` → imports as dashed arcs → unreachable functions stay dim at the bottom ("not reached from any entry") |
| CAD (`CADEngine`) | feature-tree/timeline left→right | document root → sketch → `ParametricDrivesFeature` pulse → feature → `FeatureCreatesBody` → body; `MatesWith` pairs flash simultaneously; `ParametricGeneratesMesh` arc to 3D |
| 3D (`ThreeDEngine`) | scene hierarchy + viewport | Scene → objects by `Contains`; `Animates/Drives` pulses; `Above/NearTo` as thin spatial links; mesh bounding boxes glow with score |
| Kinematics | chain along a spline | base → `FollowedByJoint` → … → end-effector, link by link, joint angle ticks if captured |
| Text | document → section order | `Follows/Precedes` read-head sweep; `Supports/Elaborates` arcs |
| Chemistry / DNA / Biology / Proteomics | molecule / sequence track / pathway | `BondedTo` ripple; `TranscribesTo → TranslatesTo` along the central dogma track; `Activates/Inhibits` green/red pulses |
| Image / Geospatial | image regions / map | regions glow by `Overlaps/Contains`; routes pulse along `RoutesThrough` |
| Control / Network / EM / Radar / Sonar / Thermal / IMU / Haptic / Depth / HSI / Audio / Video / Sound / EEG / BCI / Math | generic graph-world | generic wave (entries = roots) until the pipeline captures typed edges (sparse-capture gap) |

Engines without a dedicated view use the **generic wave in GraphWorld3D** (works for all 27 because the rule is class-based).

### 4.3 Technical design
- **Component:** `ui/src/graphkit/FanoutLens.tsx` (new; a lens in the existing lens set `graphkit/lens.tsx`) + `ui/src/graphkit/useFanoutPlayback.ts` (timeline) + `ui/src/components/FanoutTray.tsx`.
  Rendering reuses `GraphWorld3D` instanced meshes: add a `fanout: { t, order: Map<nodeIndex, appearTime>, edgePulse: … }` uniform and a per-instance `appearAt` attribute; edges: a shader-less approach (per-class `LineSegments` whose vertex `uProgress` is updated in the existing `useFrame`) keeps it to one draw call per class like today.
- **Data (no fabrication):** topology from `useGraphKit` substrate (already fetched); wave *order and scores* from the span-out result (`items[].fanout.sequence_head`, `score`, `hops`, `via`) — **never invented client-side**. Playback = replay of the real result at a fixed visual speed (default 1 hop ≈ 350 ms, user scrub/pause/speed 0.5-4×), labelled **"replay of step N span-out"** (the host computes it instantly; the animation is a faithful *visualisation of order*, not a claim that it took that long — HUD shows real `fetched`, `tokens`, `duration` if the event carries it).
- **Triggers:** (1) *Replay on demand* — "Show fan-out" on a step in `StepDetailSection.tsx` / `TaskDetailPanel.tsx` calls `POST /context/span_out` with the step's seeds (works today after the current build). (2) *Live* — subscribe to the new `span_out` frame (§5 E1) and auto-play in the project's engine while a run is active.
- **Perf:** instanced meshes, ≤ 1 draw call per shape/class; wave state lives in refs mutated in `useFrame` (no React re-render per frame); cap animated edges to the packed set (≤ a few hundred — the span-out fetch cap is 400, `span_out.rs` `DEFAULT_FETCH_CAP`); pause the loop when the tab is hidden (`document.visibilityState`) and when `reducedMotion` (§8) → jump to final state with all items visible and the tray populated.
- **Degrade:** no `fanout` on an item → plain node pulse (score glow only, no sequence); no topology loaded → tray only; graph file with empty `edges` (sparse modalities) → entries flash + node list, banner "this modality captured no typed edges yet" (an honest gap, matches the guide).

## 5. New minimal host events (so animations are driven by real data, not polling/guessing)

| ID | Event | Payload | Emitted from | Size |
|---|---|---|---|---|
| E1 | `span_out` (WS frame + `OrchestrationEventHub`-style sibling) | `{trace_id, task_id?, step_index?, trigger: "step"|"amt_reexpansion"|"i_loop"|"ripple", budget, tokens, fetched, budget_filled, fetch_cap_hit, frontier_left, items:[{id,container_type,name,score,hops,via,tokens,fanout?}]}` (items without `text`) | end of `span_out()` / `span_out_for_step` (`orchestrator/span_out.rs`) — one `emit` call | S (host) |
| E2 | add `trace_id`/`task_id`/`step_index` to `orchestration_stage` and `trace_id`/`span_id` to `graph_event` wire frames | existing Rust fields already exist (`orchestration_events.rs:47-49`, `graph_events.rs:35-37`) | `grpc/mod.rs:4159-4170`, `graph_event_frame` | S |
| E3 | `run_state` | `{task_id, state: running|paused|awaiting|resumed|completed|failed, step_index?, reason_kind: step_error|jurisdiction_confirm|consciousness_gate|…, error?}` | `run_checkpoint::pause_on_step_error/pause_pre_task/resume_run`, `awaiting::mark_awaiting/resume_awaiting`, jurisdiction confirmation park | S |
| E4 | emit the already-defined markers that have no call site: `[6.tools]` (with `applicable_tools` count and names), `[10.step]` (step start, `step_index`, pipeline name), `[7.0]`, `[8.1-3]`, `[13.0]` | `emit_marker` already builds named+described stage events | `stages.rs:82` (`applicable_tools`), step loop in `stages.rs`, gate paths | S |
| E5 | AMT growth event | `{amt_container_id, branch_added|deepened, node_id?, parent_node_id?, depth, source: build|reexpansion_candidate|step, trigger_detail?}` | `amt_loop.rs:443` ("branch deepened"), `amt.rs` build, step-time expansion (if/when added) | S-M |
| E6 | every non-3D modality pipeline posts `node_event` per typed edge it adds (3D already does at `3D/main.rs:2756`) | existing `OrchestrationNodeEvent` | modality `main.rs` files | M (per pipeline; also closes the capture gap) |
| E7 | forward `pipeline_location` over `/ws` | existing `PipelineLocationEvent` | `grpc/mod.rs` (mirror the node-event block) | S |

All are fire-and-forget broadcast (same hub pattern as `orchestration_events.rs`), never block the walk.

## 6. Aggregation → **Blueprint Created** finale (operator's second ask)

**Story:** the run gathers everything *around the prompt* — methodologies, MCPs/tools, categories, AMT intents/branches — then
binds them into a blueprint of N steps. Animate it as *convergence*, then a finale.

**Phases (each from a real frame):**
1. **Gather** (stage 3, summary `"Methodologies: n, Categories: n (n created)"`, `DiscoveryEffects.tsx` already parses it): methodology
   chips fly in from the edges toward the prompt node in the centre (count = real n; "created" ones glow gold, reused ones blue).
2. **Tools & MCPs** (needs **E4 `[6.tools]`** — today only a `tracing::info!`, `stages.rs:82`): tool/MCP glyphs dock onto the
   prompt node; names real (from `state.applicable_tools`).
3. **AMT** (stage 5 summary `"Mode, Intents: n, Branches: n, Details: n, Cross-refs: n, Passes: n, Validated: bool"` + `[5.enrich]/[5.xref.*]` markers):
   a tree grows out of the prompt node, branches = real `Branches`, cross-refs as arcs, `Validated` ticks green only if true.
4. **Blueprint** (stage 6 summary `"Created new blueprint with N steps (missing: M)"` or `"Using existing blueprint {id} (match: P%)"`):
   the converged structure **crystallises** — N step nodes snap into a chain around the prompt (radial → linear), missing-M
   shown as hollow slots, reuse shows the matched blueprint's id and `match %`.
5. **FINALE (only on stage 6 success):** a 1.2 s "lock-in": the chain's edges draw start→end with a bright pulse, the N nodes
   scale-pop in order, a soft bloom, and the toast `Blueprint ready — N steps` replaces the stage row. **No confetti**; "confetti" is a repo term
   for a model failure (`ConfettiViewer.tsx`) — use *bloom/crystallise*. Never plays on `success:false` (instead: dimmed chain + the real error).

- **Component:** `ui/src/components/BlueprintForge.tsx` (extends `DiscoveryEffects.tsx`, mounted by `MetaPortion.tsx` and the project chat panel) driven by a small reducer over `orchestration_stage` frames; skeleton uses CSS transforms (≤ 40 elements) — no WebGL needed.
- **Timing:** driven by *arrival of real frames*, not a timeline: each phase animates when its frame arrives (stage 3 → 5 can be minutes apart; the idle state between is the existing I-Loop-style breathing, not progress bars). Finale fixed 1.2 s.
- **Degrade:** a missing phase frame (stage skipped, detected as a gap exactly like `OrchestrationStatusPanel`'s honest "skipped" inference) → that phase simply doesn't play; tools phase absent until E4 → chip row labelled "tools: not reported".

## 7. Per-step AMT expansion past the blueprint (operator's third ask) + the rest

### 7.1 Live AMT growth (stage-5 build *and* post-blueprint expansion)
- What exists: AMT is rebuilt/deepened by the **re-expansion loop** (`orchestrator/amt_loop.rs`, `run_amt_reexpansion_loop` l.273, ripple-woken l.299, `try_reexpand_one` l.583, "branch deepened" l.443) — a background loop reacting to graph ripples/candidates. **Verified: the step walk itself (`stages.rs`) does not expand the AMT per step** (grep for expand/reexpan finds only the candidate recorder at `amt.rs:204-220`); step context comes from span-out.
- Animation: a **living tree** (`ui/src/components/AmtGrowth.tsx`, reuse `views/task/AmtActivitySection.tsx` data): each `branch_added|deepened` (E5) grows a new twig from `parent_node_id` with a 600 ms ease-out; the touched branch glows violet (I-Loop colour family) then settles; a small "why" chip shows the trigger (`ripple: SecurityFinding`, `unverified node`, `step 4`) — only if the event carries `trigger_detail`.
- Without E5 (today): derive "AMT updated" from `graph_event` `updated` on the AMT container (type string of AMT containers is **UNVERIFIED** — `types/container.rs` has no literal "AMT" type name; resolve via the existing `AmtActivitySection` lookup) → pulse the whole tree, no twig-level animation.
- **If steps should expand the AMT as they run** (operator's wording): that is new host behaviour; the animation hooks in via E5 `source:"step"`. Capture that decision as an open question (§11).

### 7.2 Step wave (per-step progress through the blueprint)
`pipeline_progress` already gives `task_id + step_index + pipeline_name + status` live → light the step node in the blueprint chain (queued dim → running breathing → completed solid / failed red-pause). Add E4 `[10.step]` for the step *description*. Component: `views/task/StepExecutionList.tsx` gets a mini chain header; `TaskDetailPanel.tsx` shows it large. Each step node shows its span-out summary chip (`items`, `tokens`, `budget_filled`) from E1 and a **"Show fan-out"** button → Fan-out Lens replay (§4).

### 7.3 Span-out in the other callers
Same E1 frame, `trigger`:
- `amt_reexpansion` → wave on the AMT graph (twigs deepen where the wave stopped),
- `i_loop` → wave rendered in the consciousness colour family, one gentle sweep per reflection,
- `ripple` → wave anchored at the *triggering container* (the event's `container_id`), expanding outward — "what does this change touch?" (this is the living-graph ripple the repo is named for).

### 7.4 Pause / resume / awaiting / jurisdiction (states, not decoration)
| State | Source | Motion | Honesty |
|---|---|---|---|
| Failure pause | `/task/pause_info` poll today; E3 `run_state=paused` | the chain freezes: running node stops breathing and **holds a thin amber ring**; edges ahead go dashed; banner (`PausedBanner.tsx`) fades in with the real `error` and "Resume from step N" | amber = *paused for failure* (resumable); never red-flash-loop |
| Resume | `/task/resume_run` ack + E3 `resumed` | ring dissolves, a single pulse travels from the paused node forward, chain resumes | real step index |
| Awaiting gate (meta-only, doctrine) | `/order/global` `awaiting[]`, `AwaitingBadge.tsx`, E3 | slow hourglass-free **"held breath"**: node with a faint concentric ripple at 0.3 Hz + label "waiting on meta" (no human) vs **"waiting on you"** (resolver human: jurisdiction_confirm) with a distinct steady badge | the two kinds are visually distinct because their resolvers differ (`awaiting.rs` kinds) |
| Jurisdiction gate | stage 0 / 14 summaries (`matched, warnings, confirmations_reviewed, pending_reviews`, `jurisdiction.rs:468/574`); `/mcp/call` & `/pipeline/execute` refusals carry `jurisdiction_gate` | a **gate arch** at the head of the chain: pass = arch glows green and the wave passes through; Warn = amber shimmer + rule label; RequireConfirmation = arch holds, review chip; Block = arch closes (no wave beyond), real rule text from the refusal | only the real matched rule/condition is shown (`"exploit"`, `"personal data"`…); absent → neutral arch, no label |
| Provider exhaustion / retry | `orchestrator` rate-limit logs only (no event) | n/a (not animated; shown as the existing status line) | do not invent a retry animation without an event |

## 8. Cross-cutting rules
1. **`prefers-reduced-motion` (verified absent today).** Add one global layer: `ui/src/App.css` `@media (prefers-reduced-motion: reduce)` zeroing/shortening every keyframe + `ui/src/hooks/useReducedMotion.ts` for JS-driven loops (r3f `frameloop="demand"`, `requestAnimationFrame` skipped). Reduced mode = final state shown immediately, tray populated, no travelling pulses.
2. **Pause when hidden / low power:** stop `useFrame` work on `visibilitychange`; r3f `frameloop="demand"` when nothing is animating (today `GraphWorld3D` keeps its loop for camera).
3. **One timeline owner:** a single `useAnimationClock` (rAF) feeding all lenses; components never start their own timers.
4. **Colour semantics (stable across the app):** entries = white-gold; Order = cyan; Structure = slate; Flow = emerald; CrossModal = magenta (matches `GraphWorld3D`); Spatial = amber; Similarity = violet; Provenance = grey; paused = amber ring; awaiting = blue ripple; error = red. Colour-blind safe: every class also has a **shape/dash** (solid, dashed, dotted, double) and a legend (`EdgeLegendFilters.tsx` exists — extend it).
5. **Accessibility:** tray is a real list (`role="list"`), each item keyboard-focusable with the `via` text as accessible name; Fan-out Lens has a text-only mode; no information is conveyed by motion alone.
6. **Scope filter:** filter frames by the project/workspace in `orchestration_stage`/E1 (the hard-coded `user_id:1, device_id:1` filters in `DiscoveryEffects`/`OrchestrationStatusPanel` are a known simplification — use `trace_id` from E2 once forwarded).
7. **No fabricated data (AGENTS.md):** forbidden — synthetic progress %, invented node/edge order, "typing" fillers, fake counts. Allowed — easing between two real states, idle breathing labelled as idle (as the I-Loop dot is), replay speed controls with the "replay" label.

## 9. Degradation matrix
| Missing | Result |
|---|---|
| No `span_out` event (today) | on-demand replay via `POST /context/span_out`; live mode shows "no live span-out feed" |
| Graph file has nodes but empty `edges` | entries + node list; banner "no typed edges captured for this modality" |
| `fanout` absent on an item (ordinary container) | score-glow pulse only |
| Stage frame skipped | that finale phase doesn't play (gap inference, same as `OrchestrationStatusPanel`) |
| WS closed/error (`graphEventClient` real `GraphSocketState`) | all live animations freeze to last real state with "offline" badge; no looping idle faking |
| Reduced motion | final state, no travel |

## 10. Build plan (priority order) and tests

| # | Item | Size | Depends |
|---|---|---|---|
| 1 | `prefers-reduced-motion` global guard + `useReducedMotion` | S | — |
| 2 | **E1 `span_out` event + E2 trace/step ids on frames** (host) and UI types in `graphEventClient.ts` | S | `span_out.rs` (current work) |
| 3 | Fan-out Lens v1 — **replay** from `POST /context/span_out` in `StepDetailSection.tsx` using GraphWorld3D generic wave + `FanoutTray` | M | `/context/span_out` route (built this session) |
| 4 | Code engine wave (entries → call chain), then CAD feature-chain, then 3D scene, kinematics chain | M each | 3 |
| 5 | E4 emit missing markers (`[6.tools]`, `[10.step]`, `[7.0]`, `[8.x]`, `[13.0]`) + `BlueprintForge` finale | M | E4 |
| 6 | Step wave in `StepExecutionList`/`TaskDetailPanel` from `pipeline_progress` (+E1 chips) | S | 2 |
| 7 | E3 `run_state` + pause/resume/awaiting/jurisdiction-arch visuals | M | E3 |
| 8 | E5 AMT growth event + `AmtGrowth` living tree | M | E5 |
| 9 | E6: other modality pipelines post `node_event` per typed edge (also closes sparse capture) | L | per pipeline |
| 10 | E7 forward `pipeline_location`; live file/object cursor in engines | S | — |
| 11 | Fan-out Lens live mode (auto-play during runs), ripple anchored waves, I-Loop sweeps | M | 2, 4 |

**Tests (real host, no mocks):**
- *Contract:* `tools/e2e/fanout.mjs` (planned) opens `/ws`, runs a real `/orchestrate` on the code workspace, asserts an E1 frame arrives with `items[].fanout.entries` matching the ingested graph's `analysis.entry_points`, `sequence_head[0]` ∈ entries, `class_counts.flow ≥ 1`, and `tokens ≤ budget`.
- *Replay determinism:* call `POST /context/span_out` twice with the same seeds → identical `items` order/scores (the animation's order must be reproducible).
- *UI logic:* a node-script harness (no browser needed) feeds **recorded real frames** (captured from a real run into `docs/evidence/ws-captures/*.ndjson` — real data, not synthetic) to the playback reducer and asserts phase order (gather → tools → AMT → blueprint → finale) and that the finale does **not** fire on a `success:false` stage 6.
- *Visual:* run `vite preview` (served at :4173 in this env) + the browser-farm profile (`browser-farm-mcp`) to screenshot the lens at t = 0 / entries / mid-wave / budget-filled and attach to `docs/evidence/test-runs/`; mark SKIP if no browser tool.
- *Reduced-motion:* emulate `prefers-reduced-motion` in the farm profile; assert no `animation-duration > 0.01ms` on `.fanout-*` and that the tray is fully populated at load.
- *Perf:* 400-item span-out on a code graph with ≥ 100 `Calls` edges: ≤ 2 draw calls per class, frame time < 16 ms on the dev machine (record in evidence).

## 11. Open questions for the operator
1. **Replay vs live:** is a faithful *replay* of the (instantaneous) span-out result enough, or do you want the host to emit incremental `span_out_progress` frames (one per fetched node) so the wave shows the real fetch order/timing?
2. Should **steps expand the AMT as they run** (new host behaviour; today only the background re-expansion loop deepens it) — and if yes, should the finale-style animation fire for each expansion?
3. **Finale feel:** "bloom/crystallise" (proposed) vs literal celebratory confetti — note "confetti" is already a repo term for a model-failure viewer.
4. Is it OK to add `trace_id`/`task_id`/`step_index` to the `/ws` frames (E2) and to emit the currently-undefined markers (E4) — small host changes, but they alter the wire?
5. Which engines first for choreography beyond the generic wave: Code → CAD → 3D (proposed, matches the clock project), or another order?
6. Reduced-motion default: honour OS setting only, or also add an in-app "Animations: full / reduced / off" switch in Settings?
7. Do you want the Fan-out Lens on **EU-visible** UI only after the 11 sparse-capture modalities (E6) post typed edges, or ship the generic wave first (honestly labelled "no typed edges captured")?
