# The Voice Box & the Consciousness Response — design guide

Captured 2026-10-04 from the operator's directive + the Artificial Voice Box
(AVB) exploration. Companions: `PERSONAL_ASSISTANT_GUIDE.md` (the consciousness
loops that generate these responses), `PERSONAL_ASSISTANT_GUIDE.md §8`
(overlay is mobile-side), `UNIVERSAL_ORDER_GUIDE.md`.

## 1. The two responses — the core differentiation

Ozone-Studio produces **two fundamentally different kinds of response**, and
the system must be able to tell them apart everywhere (chat, store, voice,
history):

| | **Orchestrator response** | **Consciousness response** |
|---|---|---|
| Origin | A user prompt through `/orchestrate` | The living graph itself — monitoring ALL events from a global viewpoint |
| Trigger | Asked | Self-generated (I-Loop reflection, assistant check-ups, emotional response, markers/thinking observations) |
| Shape | Task answer (blueprint → steps → results → delivery ladder) | Thought/speech about what is happening — not robotic, not a prompt answer |
| Channels | Text (chat), voice later | Voice, text, or BOTH |
| State carried | Task context | Emotional state + voice context + graph context (what events prompted it) |
| Store | Task outputs (task store) | Consciousness-response records (insights today; dedicated records as voice lands) |

**The rule**: every consciousness-generated response carries a
differentiation record — `{response_type: "consciousness", channel:
"text"|"voice"|"both", emotion_state: {...}, graph_context: {...}}` — so chat,
history, and the future voice pipeline can always tell WHAT KIND of response
it is, WHAT prompted it, and WHAT emotional state colored it. Orchestrator
responses never carry this record.

## 2. Consciousness response = what is on top of everything

The consciousness is what sits on top of everything happening in the living
graph — it already has the inputs, all real:

- Universal Order (every task, due dates, progress) — `/order/global`
- The markers/thinking process (this pass's instrumentation: branch
  discovery, enrichment, cross-reference — the AMT's own thought process)
- The capture quartet (S10–S13 — every model call, every tool call)
- The file beacon, task lifecycle ripples, the review pass, emotional triggers
- The I-Loop's reflections and the assistant's check-ups (its existing voice)

The assistant's check-up digest **IS** a consciousness response — generated
from monitoring, not from a prompt. What's missing (and lands with voice) is
the modality layer and the differentiation record.

## 3. The Artificial Voice Box (AVB) — capability, not identity

The AVB is a general-purpose neural/parametric sound-production system.
Its job: given an arbitrary sound-producing intent/state, generate one or
many physically/acoustically coherent sound streams simultaneously. It does
not care whether the sound is a human voice, a whisper, footsteps, a car,
glass, radio, music, or rain — those are different SOURCE configurations for
the same engine.

```
ARTIFICIAL VOICE BOX
├── Intent Interpreter
├── Source Generator (HUMAN_VOCAL | MECHANICAL | IMPACT | ... | CUSTOM)
├── Physical/Acoustic Parameter Engine
├── Temporal Dynamics Engine
├── Resonance/Filter Engine
├── Nonlinear/Noise Engine
├── Neural Vocoder
├── Spatial Output Interface
└── Parallel Voice Manager (N independent instances)
```

Core representation: **VOICE_STATE(t)** — every parameter is a function of
time. F0 is not a number; it is F0(t). The Voice Box produces a TRAJECTORY
through a high-dimensional state space (source → physiology → tract →
articulation → expression → temporal → noise), not a static voice. Two
outputs, deliberately separated: the CONTROL representation (what the
mechanism is doing — STATE(t)) and the ACOUSTIC representation (what the
sound is — WAVEFORM(t)).

## 4. Voice Identity — what makes it ITS voice

The Voice Box is capability. Voice Identity is a persistent policy telling
the Voice Box how to use that capability. An AI with unlimited vocal
capability needs an artificial anchor; that anchor is not one vector but an
**identity manifold** — a region of acoustic/physiological state space:

```
VOICE_IDENTITY {
    anatomical_baseline, acoustic_signature,
    preferred_register / pitch_range / rhythm / articulation / resonance / voice_quality,
    expressive_range, stability, adaptation_policy, allowed_variation,
    identity_strength
}
```

A human can whisper, shout, laugh, sing — and is still recognized. Identity
is a region, not a point. The AI can move inside its identity space, even
temporarily leave it (roleplay, scene work) and know it left.

**Self-model → Voice Policy → Voice Box**: the Voice Box says "I can produce
this"; the Voice Policy says "I choose to sound like this." Context shapes
the policy (emergency → clarity + intensity, identity maintained). Cloning
is distinct from identity: the AI can understand and reproduce another voice
without adopting it.

**Self-hearing closes the loop**: generate → hear → analyze → compare
against intention → correct. Over many interactions, VOICE_IDENTITY(t)
stabilizes as a persistent learned state — not biologically trapped, but
chosen and kept.

## 5. Where Whisper (and friends) fit

Whisper is ASR — audio → text. It is an INPUT semantic layer ("what was
heard?"), never the Voice Box and never the end output. "We can do without
it, but it helps with semantics." Components plug in around the AVB:
Whisper (speech understanding), audio codecs (acoustic representation),
speaker encoders (identity extraction), generative audio models (waveform
rendering). The new thing is the architecture around them.

## 6. Implementation map (staged, on real ground that exists)

- **Now (real)**: consciousness responses as differentiated records —
  the assistant check-up digest carries its emotional state
  ( ConsciousnessStore::get_current_emotional_state: primary_emotion,
  valence, arousal, dominance, stability, triggers) + response_type +
  channel into its persisted insight; I-Loop reflections carry the same.
- **Next**: dedicated consciousness-response records (not just insight
  containers) with modality + emotion + graph-context refs; chat-side
  differentiation display; voice context history.
- **Then**: the AVB pipeline — text → phonetic/prosodic plan →
  VOICE_STATE(t) → parameter trajectories → neural renderer → SoundStream;
  Voice Identity as the policy layer; self-hearing feedback.
- **Whisper**: semantic input (mic speech → text), used when a mic exists;
  the consciousness talks WITHOUT a microphone because it talks from events.
- **Hearth/games**: the universal acoustic world (sound objects, scene
  graphs, spatial field) extends the AVB beyond voice — deliberately later.

## 7. Non-negotiables

- No fabricated emotion: emotional state comes from the real
  ConsciousnessStore triggers, never invented for flavor.
- No fabricated voice context: voice history records what was actually
  spoken/heard.
- Differentiation is structural (a record field), not stylistic — any
  consumer can filter consciousness responses out of chat history reliably.
- The orchestrator response NEVER gains a consciousness record; the
  consciousness response NEVER masquerades as a task answer.
