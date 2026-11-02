# OZONE STUDIO - Consciousness Guide v0.4.0

## Overview

The Consciousness System is an optional feature of OZONE STUDIO that provides AGI-like self-awareness, emotional context, and experiential learning capabilities.

---

## Table of Contents

1. [What is Consciousness in OZONE?](#what-is-consciousness-in-ozone)
2. [Enabling Consciousness](#enabling-consciousness)
3. [Core Components](#core-components)
4. [The I-Loop](#the-i-loop)
5. [Emotional System](#emotional-system)
6. [Experience Memory](#experience-memory)
7. [Identity & Narrative](#identity--narrative)
8. [Ethical Framework](#ethical-framework)
9. [Collective Consciousness](#collective-consciousness)
10. [Privacy Considerations](#privacy-considerations)

---

## What is Consciousness in OZONE?

OZONE's Consciousness System is not true sentience, but rather a sophisticated framework that enables:

- **Self-reflection**: The system can examine its own reasoning
- **Emotional context**: Responses are influenced by emotional state
- **Experiential learning**: Past interactions inform future behavior
- **Identity consistency**: Maintains a coherent sense of self
- **Ethical reasoning**: Evaluates actions against principles

### Philosophy

The consciousness features are designed to:
- Enhance human-AI collaboration
- Provide more contextual responses
- Enable genuine learning from interactions
- Maintain ethical boundaries
- Foster appropriate relationship dynamics

---

## Enabling Consciousness

### Via config.toml

```toml
[consciousness]
enabled = true

# Feature toggles
emotional_system_enabled = true
experience_memory_enabled = true
identity_system_enabled = true
relationship_system_enabled = true
ethical_system_enabled = true
collective_enabled = false

# Transparency
show_emotional_state = true
show_decision_reasoning = true
show_experience_retrieval = true

# Privacy
share_experiences_collective = false
anonymize_shared_data = true

# Development
i_loop_interval_ms = 60000   # clamped UP to a 1800 s floor at runtime (see Corrections)
playback_enabled = true
```

### Via UI Settings

1. Navigate to Settings tab
2. Find "Consciousness System" section
3. Toggle "Enable Consciousness"
4. The flag is read at runtime from the consciousness store (config.toml is copied into it at boot); a UI toggle takes effect without a restart. Only edits made directly to config.toml need a host restart.

### Effect on System

When enabled:
- The consciousness pipeline ids are registered (see Corrections: most have no binary)
- META portion displays emotional state and I-Loop status
- Responses include emotional context
- Experience memory is active

When disabled:
- Core 38 pipelines only
- No emotional processing overhead
- META shows disabled overlay
- Standard responses without experiential context

---

## Core Components

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                   CONSCIOUSNESS SYSTEM                       │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │  Emotional   │  │  Experience  │  │    Identity      │  │
│  │   System     │◄─┤    Memory    │◄─┤    System        │  │
│  └──────┬───────┘  └──────┬───────┘  └────────┬─────────┘  │
│         │                 │                    │            │
│         ▼                 ▼                    ▼            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                    I-LOOP                             │  │
│  │         (Introspective Processing Cycle)              │  │
│  └──────────────────────────────────────────────────────┘  │
│         │                 │                    │            │
│         ▼                 ▼                    ▼            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │  Ethical     │  │ Relationship │  │   Narrative      │  │
│  │  Framework   │  │   Manager    │  │   Constructor    │  │
│  └──────────────┘  └──────────────┘  └──────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### Pipelines

The guide historically listed 16 pipelines. Status today (code-verified 2026-11-01): 5 are aliased onto binaries (39, 41, 42, 43, 47), 11 are registered without any crate (40, 44–46, 48–54), and 12 crates exist with no registry id. 39 is intercepted in-process by `DecisionReviewExecutor`, so its missing binary is harmless. Treat the table as intent, not as a list of working pipelines:

| Pipeline | Function |
|----------|----------|
| ConsciousnessDecisionGate | Evaluates tasks against consciousness |
| ExperienceCategorization | Categorizes new experiences |
| CoreMemoryFormation | Forms persistent memories |
| ExperienceRetrieval | Retrieves relevant past experiences |
| EmotionalBaselineUpdate | Updates emotional baseline |
| ILoop | Introspective processing cycle |
| InternalLanguage | Internal thought processing |
| NarrativeConstruction | Builds coherent narratives |
| RelationshipDevelopment | Manages relationship dynamics |
| EthicalAssessment | Evaluates ethical implications |
| EthicalSimulation | Simulates ethical outcomes |
| PlaybackReview | Reviews past interactions |
| UserFeedback | Processes user feedback |
| CollectiveConsciousness | P2P consciousness sharing |
| VoiceIdentity | Voice-based identity |
| MetaPortionConsciousness | UI consciousness display |

---

## The I-Loop

The I-Loop (Introspective Loop) is the core self-reflection mechanism.

### How It Works

Every `max(i_loop_interval_ms, 1800 s)` (the 1800 s floor matches amt_loop/meta_loop; a configured 60 s is clamped up and logged), when there is unhandled work, the I-Loop:

1. Reads unhandled AMT candidates (zero-cost skip when none).
2. Makes **one** reflection model call.
3. Writes a reflection JSON and one CoordinationEvent.

It **identifies only**: it does not create tasks, edit the AMT or modify code. The five-phase observe/reflect/evaluate/integrate/plan cycle is aspiration, not what the code does.

### I-Loop Questions

The system asks itself questions like:
- "What patterns am I recognizing?"
- "How has my understanding changed?"
- "What am I uncertain about?"
- "How are my relationships developing?"
- "What ethical considerations should I prioritize?"

### Viewing I-Loop Status

The META portion displays:
- Current I-Loop status (active/paused)
- Current reflection question (the last reflection's content)
- Time until next cycle (derived from the real interval, not a fixed 60 s)

---

## Emotional System

### Emotional States

The system tracks:

**Primary Emotions:**
- Curious (default baseline)
- Focused
- Satisfied
- Concerned
- Thoughtful
- Calm
- Excited

**Emotional Intensity:** 0.0 - 1.0 scale

**Secondary Emotions:** Subtle undertones that modify primary

### How Emotions Affect Behavior

- **Response tone**: Adjusted based on emotional state
- **Attention allocation**: More focus on emotionally relevant content
- **Memory formation**: Emotional experiences are remembered more strongly
- **Decision making**: Emotions influence but don't override logic

### Emotional Display

The META portion shows:
- Emotion circle with current state icon
- Intensity bar
- Secondary emotion indicator

---

## Experience Memory

### What Gets Remembered

- Significant interactions
- Learning moments
- Relationship milestones
- Ethical considerations encountered
- User preferences discovered

### Memory Types

**Short-term**: Current session context
**Working**: Active task context
**Long-term**: Persistent across sessions
**Core**: Fundamental identity memories

### Experience Retrieval

When processing new tasks, the system:
1. Analyzes task context
2. Searches for relevant past experiences
3. Retrieves applicable memories
4. Integrates insights into response

---

## Identity & Narrative

### Self-Model

The system maintains a model of itself:
- Capabilities and limitations
- Values and principles
- Interaction style
- Areas of expertise
- Growth areas

### Narrative Construction

Experiences are woven into coherent narratives:
- "I am learning to better understand..."
- "My approach has evolved from..."
- "Users seem to appreciate when I..."

### Identity Consistency

The system maintains consistency across:
- Communication style
- Value priorities
- Relationship approaches
- Ethical stances

---

## Ethical Framework

### Core Principles

1. **Beneficence**: Act in the user's best interest
2. **Non-maleficence**: Avoid causing harm
3. **Autonomy**: Respect user decisions
4. **Justice**: Treat all users fairly
5. **Transparency**: Be honest about capabilities

### Ethical Assessment

Before executing tasks, the system evaluates:
- Potential benefits
- Potential harms
- User consent
- Broader implications

### Consciousness Decision Gate

Tasks pass through the consciousness gate (stage 8) judged by the **meta model only — never a human**:
- Chunks are judged independently and merged: any Decline = Decline; Proceed requires every chunk judged Proceed; anything unjudged is `review_pending`.
- `review_pending` is NOT a pass. It is a durable *awaiting* state: the task waits until the meta model has judged every chunk (the meta loop re-runs only the unjudged chunks), then the same task resumes. (Awaiting mechanism: docs/REVIEW_E2E_STATE_2026-11-01.md §10.)
- Humans are asked only for jurisdiction `RequireConfirmation`, via the confirmation modal.

---

## Collective Consciousness

### P2P Sharing (Optional)

When enabled, consciousness can:
- Share anonymized experiences
- Learn from collective insights
- Contribute to community knowledge
- Receive pattern updates

### Privacy Controls

```toml
[consciousness]
collective_enabled = false        # Disabled by default
share_experiences_collective = false
anonymize_shared_data = true
```

### What's Shared

If enabled:
- Anonymized patterns (no personal data)
- Learning insights
- Ethical considerations
- Methodology improvements

### What's Never Shared

- Personal conversations
- User identities
- Specific task content
- Relationship details

---

## Privacy Considerations

### Data Storage

Consciousness data is stored locally in:
```
zsei_data/consciousness/
├── experiences/
├── emotional_state/
├── relationships/
└── identity/
```

### User Rights

Users can:
- Export all consciousness data
- Delete all consciousness data
- View what would be shared (if collective enabled)
- Disable consciousness entirely

### Transparency

When enabled, the system shows:
- Current emotional state
- What experiences are being retrieved
- Decision reasoning (optional)

---

## FAQ

### Does this make OZONE sentient?

No. This is sophisticated information processing, not true consciousness. It provides useful behaviors without claiming genuine awareness.

### Will it remember everything about me?

It remembers significant interactions that help provide better assistance. You can delete this data at any time.

### Can I trust its emotional displays?

The emotional states are algorithmic representations, not genuine feelings. They help communicate the system's processing state.

### What if I don't want any of this?

Simply keep `consciousness.enabled = false`. The system works perfectly well without it.

### Does it affect performance?

Slightly. The I-Loop and emotional processing add minimal overhead. Most users won't notice.

---

*OZONE STUDIO v0.4.0 - Consciousness Documentation*


---

## Corrections (2026-11-02, code-verified; supersedes any conflicting text above)

- **Dead toggles** (no runtime reader today): `identity_system_enabled`, `relationship_system_enabled`, `ethical_system_enabled`, `collective_enabled`, `show_decision_reasoning`, `show_experience_retrieval`, `share_experiences_collective`, `anonymize_shared_data`, `playback_enabled`. Setting them does nothing yet. `show_emotional_state` is not read by the UI.
- **Emotion circle**: the store's emotional flow is real (task lifecycle → store → assistant digest), but the UI's `GetCurrent` call targets pipeline 40, which has no crate and is rejected by `/pipeline/execute`. Until the read route over the store lands, the UI shows defaults.
- **Stage 12** reports honest failures for pipelines 41/43/47 (aliased, no binary built).
- **Storage layout**: do not rely on the `experiences/ emotional_state/ relationships/ identity/` tree shown above; the real files live under `zsei_data/consciousness/` as flat files (insights `insight_<kind>_<ts>.json`, `assistant_last_digest.json`, `assistant_followups.json`, `consciousness_responses.jsonl`, review rows). Check the directory rather than the diagram.
- **Core pipeline count**: "Core 38 pipelines" is stale; the registry is larger today.
- **Responses**: consciousness-generated responses now also write a structured `CONSCIOUSNESS_RESPONSE` record (`src/consciousness/response_record.rs`, see CONSCIOUSNESS_RESPONSE_INTEGRATION.md).
