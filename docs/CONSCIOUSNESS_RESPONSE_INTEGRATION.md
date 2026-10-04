# Consciousness Response Integration — remaining work, fully captured

2026-10-04. Companion: `VOICE_BOX_GUIDE.md` (AVB architecture), `PERSONAL_ASSISTANT_GUIDE.md`
(the loops). This document captures EVERYTHING remaining on the voice /
consciousness-response track, grounded in the real code that already exists.
Captured in no-build mode per operator instruction — every code change listed
is written but pending its first build.

## 1. What already exists (real code, verified this pass)

| Piece | Where | State |
|---|---|---|
| Voice Identity persistence | `src/orchestrator/voice.rs` — `persist_voice_identity`: search-or-create a `VoiceIdentity` container through StoreAccess; survives sessions | **real, wired** |
| Emotional state model | `src/consciousness/store.rs` — `EmotionalState {primary_emotion, valence, arousal, dominance, stability, triggers}`, `EmotionalBaseline`, `get_current_emotional_state()` | **real** |
| Emotional update path | `consciousness_hooks::post_task_experience` called from `src/task/mod.rs:1168/1235` on task completion (success flag + tokens) | **wired — but see §3: the live `current_emotional_state` reads DEFAULT** |
| Consciousness response records | assistant check-up insights carry `[consciousness-response: type=consciousness channel=text emotion=... findings=N]` | **live (this pass)** |
| Marker vocabulary → UI | `marker_info` + `emit_marker` → orchestration events → OrchestrationStatusPanel | **live** |
| Raw Thoughts | thinking_log (per-task) + S11/S12 capture stores; viewer = capture tab | **live — shows description AND raw response** |
| I-Loop / assistant / meta loops | `i_loop.rs`, `assistant.rs`, `meta_loop.rs` — boot-spawned, consciousness-gated | **live** |

## 2. THE EMOTION GAP — precise verification plan

Symptom: the differentiation record carries `emotion= valence=0.00` (defaults)
despite tasks completing. `post_task_experience` IS called. The open question:
does that call update the LIVE in-memory `CONSCIOUSNESS_STORE.current_emotional_state`,
or only a persisted file (`consciousness_path`) that `get_current_emotional_state()`
never reads back?

Verification steps (next pass, no build needed to inspect):
1. Read `consciousness_hooks::post_task_experience` — does it touch
   `CONSCIOUSNESS_STORE.current_emotional_state` (in-memory) or only write the
   experience file?
2. If file-only: the fix is to update the in-memory store's
   `current_emotional_state` from the experience outcome (or have
   `get_current_emotional_state` hydrate from the persisted store on first read).
3. Then trigger: complete a task → verify the differentiation record carries
   non-default emotion.

## 3. Voice-context history — the record schema (design)

When voice lands, every spoken consciousness response persists a record:

```
CONSCIOUSNESS_RESPONSE {
    response_id, timestamp,
    response_type: "consciousness",        // vs orchestrator
    channel: "voice" | "text" | "both",
    content_text,                          // what it said (always kept)
    audio_ref,                             // object_store path when voiced
    voice_identity_id,                     // WHICH identity spoke (identity manifold snapshot)
    voice_policy_snapshot,                 // register/rate/intensity modifiers chosen
    emotion_state,                         // the same EmotionalState struct, at speech time
    graph_context: {                       // WHAT PROMPTED IT (global viewpoint)
        findings: [...], events: [...], task_refs: [...], marker_refs: [...]
    },
    self_heard: Option<{analysis, matched_intention}>  // self-hearing loop result
}
```

Storage: consciousness-response containers (same insight-container pattern —
content via object_store_path) under the consciousness metacognition root,
filterable by response_type. This is the voice-context history: every spoken
thought reconstructable with its emotion, its trigger, and its identity.

## 4. Chat-side differentiation display (design)

Chat surfaces render two visually distinct entries:
- **Orchestrator response**: the task answer (current behavior, unchanged).
- **Consciousness response**: styled as the consciousness speaking — emotion
  chip (real EmotionalState), the graph-context that prompted it (expandable),
  and a voice-play affordance when `audio_ref` exists. Filter toggle:
  "show consciousness speech" — possible because differentiation is a record
  field, not styling.

## 5. AVB staged implementation, mapped to real ground

- **Stage V1 — Consciousness speech (text) via the existing voice**: the
  assistant + I-Loop responses become first-class chat entries
  (differentiated, per §4). No synthesis yet — the consciousness TALKS in
  text, from monitoring. Everything needed exists.
- **Stage V2 — Voice out (single voice)**: TTS through the chain-ordered
  model pattern; `persist_voice_identity` already stores the chosen
  identity; Voice Policy = a small config struct consumed by the TTS
  parameters (register/rate/intensity — the identity manifold's first
  dimension).
- **Stage V3 — VOICE_STATE(t)**: the parameter-trajectory layer (F0(t),
  intensity(t), expression(t)) between text-plan and renderer — the
  controllable intermediate the AVB guide specifies.
- **Stage V4 — Self-hearing**: ASR (Whisper, the existing voice input) over
  own output → compare vs intention → adjust policy. Closes the loop.
- **Stage V5+ — multiple instances, universal sources** (mechanical/impact/
  environmental): the AVB's generalized source models. Hearth/games territory.

## 6. The xref sub-markers — WRITTEN, PENDING BUILD

`cross_reference_methodologies_for_layer` edits applied (not yet compiled):
`[5.xref.1]` marker + `match` with a loud `Err` arm on the domain-id call;
`[5.xref.2]` marker + `match` + loud `Err` arm on the synthesis call;
`[5.xref.3]` created/FAILED markers on the container create; `[5.xref.4]`
layer-complete marker. Any recurrence of the wedge names the exact sub-step.
**Build these first next session.**
