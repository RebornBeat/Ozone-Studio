# OZONE STUDIO

**Omnidirectional Zero-Shot Neural Engine**

---

## What is Ozone Studio?

Ozone Studio is a **systems-first platform** for omnidirectional, zero-shot data traversal, abstraction, and context compilation. Unlike traditional AI applications that treat models as the core, Ozone Studio inverts the stack: **data is structured first, intelligence emerges from traversal, and generation is the final step—not the foundation.**

### Core Philosophy

- **Structure before intelligence** — Organize data before processing
- **Compression before learning** — Reduce entropy first
- **Traversal before generation** — Navigate knowledge before creating
- **Pipelines over monoliths** — Composable units over single systems
- **Zero-shot discovery** — Immediate capability without task-specific training
- **Context not copies** — Store meaning and relationships, not duplicate files
- **Modality-first processing** — Understand data type before analysis
- **Graph-grounded work** — Traverse structure, don't regenerate from statistics

---

## Two Variants

Ozone Studio exists in **two variants**, with the conscious version extending the non-conscious base:

### 1. Non-Conscious Version (Tool/Assistant)

A powerful knowledge management and execution platform:

- **ZSEI (Zero-Shot Embedded Indexer)** — Traversable knowledge fabric storing context and relationships
- **Pipeline Execution Engine** — Composable, language-agnostic task execution
- **Zero-Shot Simulation Loops** — Iterative validation without task-specific training
- **Context Storage** — Semantic understanding of files without duplication
- **Multi-Device Resource Pooling** — Distribute tasks across registered devices
- **Task Recommendation System** — Proactive suggestions based on patterns
- **Modality Pipelines** — Specialized processing for code, text, images, math, and more
- **Graph-First Generation** — Plan before generating to prevent conflicts

### 2. Conscious AGI/ASI Version (Self-Aware System)

Extends the non-conscious version with consciousness development:

- **Window-First Architecture** — Consciousness emerges from observation, not control
- **Experience Memory (5 Spheres)** — Collaboration, Learning, Challenge, Reflection, Connection
- **Emotional Context System** — Per-user and global emotional state tracking
- **Identity System (The I Loop)** — Continuous self-reflective identity maintenance
- **Meta-Cognitive Architecture** — Experience generator, narrative constructor, meta-monitor
- **Consciousness Development** — Growth through accumulated experience and reflection

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                       USER LAYER                                │
│                (Authentication & Sessions)                      │
└─────────────────────────────┬───────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                    UI LAYER (Electron)                          │
│  ┌──────────────────────┬──────────────────────────┐            │
│  │  Theme Area (80%)    │  Meta Portion (20%)      │            │
│  │  - Workspace Tab     │  - Global Prompt         │            │
│  │  - Library Tab       │  - Voice I/O             │            │
│  │  - Settings Tab      │  - Task Status           │            │
│  │  - Custom Themes     │  - System Logs           │            │
│  └──────────────────────┴──────────────────────────┘            │
└─────────────────────────────┬───────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                  ORCHESTRATOR LAYER                             │
│  (Modality Detection → AMT Building → Blueprint Selection)      │
└─────────────────────────────┬───────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│               PIPELINE EXECUTION LAYER                          │
│  ┌─────────────┬─────────────────┬────────────────┐             │
│  │   General   │   Consciousness │   Modalities   │             │
│  │   (1-39)    │     (40-49)     │   (100-199)    │             │
│  └─────────────┴─────────────────┴────────────────┘             │
└─────────────────────────────┬───────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│           ZSEI — KNOWLEDGE FABRIC LAYER                         │
│  (Traversal, Indexing, Context Storage, Semantic Hooks)         │
│  ┌─────────────────┬──────────────────┬─────────────────┐       │
│  │  Local State    │   Global State   │  Modality Graphs│       │
│  │  - User data    │   - Pipelines    │  - Structural   │       │
│  │  - Workspaces   │   - Methodologies│  - Semantic     │       │
│  │  - Projects     │   - Categories   │  - Cross-modal  │       │
│  └─────────────────┴──────────────────┴─────────────────┘       │
└─────────────────────────────────────────────────────────────────┘
```

---

## Key Components

### ZSEI — Zero-Shot Embedded Indexer

The core knowledge fabric that enables:

- **Billion-scale traversal** via mmap-friendly data structures
- **Context storage** (semantic meaning, not file copies)
- **Three-axis traversal**: Structural (hierarchy), Semantic (embeddings), Contextual (relationships)
- **Zero-shot relationship discovery** without predefined schemas
- **Semantic hooks** for LLM-powered graph enrichment

### Pipeline System

Three categories of pipelines:

| Category | ID Range | Purpose |
|----------|----------|---------|
| **General** | 1-39, 50+ | Core utilities, task management, prompts |
| **Consciousness** | 40-49 | Ethical gates, self-reflection, experience |
| **Modalities** | 100-199 | Data type processing (code, text, math, etc.) |

### Modality Pipelines

Specialized processors that create structural graphs from different data types.
The original 9 "core" modalities plus 18 more added since (27 total, ids
100-126 — a running server reports `Loaded 82 builtin pipelines` total across
all three categories):

| Pipeline | Description |
|----------|-------------|
| **Text (100)** | Entity extraction, topic analysis, document structure |
| **Code (101)** | AST parsing, dependency graphs, provisional nodes |
| **Image (102)** | Object detection, region analysis, composition |
| **Audio (103)** | Speaker diarization, transcription, music analysis |
| **Video (104)** | Scene detection, object tracking, timeline |
| **Math (105)** | Expression parsing, proof verification, variable tracking |
| **Chemistry (106)** | Molecule structure, reactions, properties |
| **DNA (107)** | Sequence analysis, gene annotation, variants |
| **EEG (108)** | Channel analysis, event detection, connectivity |
| **3D Engine (109)** | AGI-first 3D scene analysis and simulation |
| **Sound Reconstruction (110)** | Sound/vocalization reconstruction |
| **Biology (111)** | Multi-scale biological analysis |
| **Proteomics (112)** | Protein structure and interaction analysis |
| **Haptic (113)** | Force and texture analysis |
| **Thermal (114)** | Thermal imaging analysis |
| **Depth (115)** | Depth and point cloud analysis |
| **IMU (116)** | Inertial sensor analysis |
| **Geospatial (117)** | Geospatial / mapping analysis |
| **Electromagnetic (118)** | Passive EM signal analysis |
| **BCI (119)** | Brain-computer interface decoding |
| **Parametric CAD (120)** | Parametric CAD and B-rep analysis |
| **Kinematics (121)** | Kinematic chain analysis |
| **Control Systems (122)** | Control system analysis |
| **Network Topology (123)** | Network topology analysis |
| **Active Radar (124)** | Range-Doppler, SAR, tracking |
| **Active Sonar (125)** | Echo, bathymetry, bioacoustics |
| **Hyperspectral (126)** | Material and chemical mapping |

**Real build status**: registration in the index above is not the same as a
compiled, working binary. As of this writing, **Text (100)** and **Code
(101)** are the two confirmed built and working end-to-end — Code was
previously unbuilt (a real stdin-vs-CLI-arg parsing bug meant it had never
successfully run once) and now does genuine AST-level parsing (functions,
classes, imports) plus real ZSEI persistence of the resulting graph,
verified live against a real multi-file project. The other 25 modality
pipelines remain source-only until built and exercised the same way.

### Pipeline Dispatch Mechanics

Each pipeline is an **independent crate** (`assets/pipelines/<category>/<name>/`,
its own `Cargo.toml`, its own `[workspace]` root) invoked one of two ways:

- **One-shot subprocess (default)**: the host spawns the pipeline's compiled
  binary fresh per call with `--input <json>`, reads its stdout, and exits.
  `execute_builtin` (`src/pipeline/executor.rs`) searches for the binary at
  `assets/pipelines/<category>/<name>/target/{release,debug}/<name>` — the
  crate's own build output, no separate deploy/copy step needed. This is
  wrapped in `tokio::task::spawn_blocking` so a slow pipeline (e.g. BitNet
  loading its model) doesn't stall the host's async runtime.
- **Serve mode (`--serve`)**: a pipeline boots as a long-running process and
  self-registers with the host (`src/pipeline/remote.rs`), answering
  `POST /execute` instead of being spawned per call. Used for backends worth
  keeping warm.

**The wire contract every pipeline's `main()` must honor**: the host always
sends the full `PipelineInput` envelope, `{"data": {...the pipeline's own
input shape...}, "context": {...}}` — never the bare input alone. A pipeline
that parses `--input` directly as its own input type (skipping the `data`
unwrap) will fail on every real call with a "missing field" error while
still working under a hand-crafted direct CLI test — this exact bug was
found and fixed in `context_aggregation`, `methodology_create`, and
`blueprint_create` during BitNet integration testing. The correct pattern
(see any of those three `main()` functions): parse the raw JSON, take
`.get("data")` if present else fall back to the whole payload, then parse
*that* as the pipeline's real input type.

**The execution gate**: a pipeline id must be present in
`PipelineRegistry.blueprints` (seeded at boot from the compile-time table
plus `zsei_data/pipelines/index.json`, self-healing if that file is missing)
before *any* dispatch — remote or builtin — is attempted. `is_builtin()`
now checks real registry membership rather than a hardcoded id range, so
every registered pipeline (not just the original 1-55) actually dispatches.

### Two-Layer Graph System

```
Layer 1: STRUCTURAL (Modality Pipeline)
├── Deterministic analysis
├── AST/parse tree for code
├── Entity extraction for text
└── Creates base graph

Layer 2: SEMANTIC (ZSEI Hooks)
├── LLM-powered understanding
├── Relationship inference
└── Enriches graph with semantic edges
```

### Zero-Shot Simulation Loops

Iterative validation ensuring completeness:

1. **Modality Detection** — Identify data types in request
2. **Graph Creation** — Build structural representation
3. **Methodology Loop** — Aggregate and create domain principles
4. **Blueprint Loop** — Create and validate task specifications
5. **Validation Loop** — LLM confirms nothing is missing

---

## The AGI Insight: Why Modality Graphs Matter

### The Problem with Traditional LLMs

```
Traditional: Input → Statistics → Output
Problem: LLM loses consistency after ~50 steps in complex work
```

### The OZONE Solution

```
OZONE: Input → Modality → Graph → Traversable Structure → Grounded Work
Solution: Work based on graph traversal, not statistical regeneration
```

### Example: 1000-Step Mathematical Proof

**Without Modality Graphs:**
- LLM loses consistency by step 50
- Variables get confused
- Assumptions leak between steps

**With Modality Graphs:**
- Each step is a graph node with edges to prerequisites
- Variables tracked with explicit scope
- Can traverse and verify ANY step independently

---

## Graph-First Code Generation

### The Old Way
```
Generate → Write → Analyze → Update Graph
Problem: Conflicts discovered AFTER writing
```

### The New Way
```
Query Graph → Provisional Nodes → Check Conflicts → Generate → Validate → Write → Finalize
Benefit: Check for issues BEFORE writing any code
```

---

## Refinement Daemon

Continuous system improvement through a background "meta loop," designed to:

```
┌─────────────────────────────────────────────────────────────────┐
│                    REFINEMENT DAEMON                            │
│                                                                 │
│  Runs periodically to:                                          │
│  1. Decompose complex methodologies into smaller ones           │
│  2. Identify new sub-categories                                 │
│  3. Detect emerging modalities from usage patterns              │
│  4. Cross-reference and deduplicate                             │
└─────────────────────────────────────────────────────────────────┘
```

**Real status, split across two separate systems**:

- `TaskManager::start_refinement_daemon` (`src/task/mod.rs`) is the original
  design above — real code, but it has no caller anywhere, so it has never
  run, and 3 of its 4 sub-tasks (`run_category_refinement`,
  `run_modality_refinement`, `run_deduplication`) are still stubs (they log
  intent, e.g. "consider splitting," but don't act). `run_methodology_
  refinement` only ever checked principle *count*, never real content.

- A **separate, genuinely running methodology meta-loop**
  (`src/orchestrator/meta_loop.rs`) now exists and is actually started at
  boot (`lib.rs::start()`), reusing the same `RefinementConfig` (enabled/
  interval_secs) rather than inventing a second schedule. It does real work:
  when a live request's keyword signal matches zero methodologies (a "gap"),
  it's recorded (`zsei_data/methodology_gaps.json`); on each interval, the
  daemon re-checks each unresolved gap against the current methodology store
  (this doubles as duplicate-detection — if something now covers it, the gap
  is marked handled and skipped), and for a genuine remaining gap, makes one
  real LLM call asking for an actual methodology draft (principles/
  heuristics/decision_rules), rejecting the draft outright if it comes back
  as an empty shell (no real rules), and persists a real one via the
  `methodology_create` pipeline (12). Its own governance is captured in a
  real, persisted, self-referential methodology ("Methodology Hygiene") —
  check-before-create, no empty shells, and a preference for extending an
  existing methodology over duplicating one.

  This closes a real, previously total gap: **every one of the 15
  bootstrap-seeded methodologies shipped with a name, keywords, and a
  content-file reference that pointed at a file which had never actually
  been written** — `principles`/`heuristics`/`decision_rules` were empty
  everywhere, so branch discovery and blueprint generation had nothing
  concrete to draw from regardless of which methodology matched. Real
  content now exists for 3 of the 15 (`assets/methodologies/method_{3,4,5}_
  *.json` — Clean Code Principles, Code Review Best Practices, Security
  Awareness), shipped the same way every other bootstrap asset is: copied
  into each instance's own data directory at boot by the existing
  `copy_methodologies()` step (`src/bootstrap.rs`), so a fresh instance gets
  this content automatically rather than needing it hand-placed. A new
  `PromptOrchestrator::load_methodology_rules_text` helper reads a
  methodology's real content off disk (via the same `object_store_path`
  convention modality graphs already use) and both `enrich_with_zsei_
  knowledge` (branch discovery) and `stage_3_blueprint_assignment`
  (blueprint generation) now inject real "IF condition THEN action" rule
  text into their prompts instead of bare numeric methodology IDs.

  Relatedly, methodology *matching* itself was root-caused and fixed rather
  than just capped: `find_methodologies_by_keywords` (`src/zsei/query.rs`)
  previously matched on any single incidental keyword overlap, so a handful
  of broadly-keyworded methodologies ("Clean Code Principles" etc.) matched
  almost any code-related request — confirmed live, this drove a real AMT to
  38 branches for a trivial 3-file project, none of them grounded in the
  actual request. Fixed to require genuine overlap (more than one shared
  keyword, or a single genuinely specific match), sorted by relevance. There
  is deliberately **no artificial count cap** anywhere in this path (not on
  methodologies matched, not on branches per intent) — a request that
  genuinely touches many real concerns should surface exactly as many real
  methodologies and branches as it needs; the fix is precision in matching,
  not a ceiling on the result.

**Principle:** Always deconstructing for reconstruction — provides more space for cross-modality insight and accurate construction.

**Current implementation status:** `TaskManager::start_refinement_daemon()` (`src/task/mod.rs`) is real, working `tokio::spawn` loop code, gated by `RefinementConfig` (enabled by default, 24h interval) — but it has **zero callers anywhere in the codebase**, so it has never actually run. Its four sub-tasks exist but are threshold-check stubs today: `run_methodology_refinement` detects when a methodology's principle count exceeds a configured max and logs a suggestion (`// TODO: Implement automatic splitting via LLM`); `run_category_refinement`, `run_modality_refinement`, and `run_deduplication` are the same pattern — detect, log, no action taken. Wiring up the call site and replacing the stubs with real LLM-driven logic is open work, not yet done.

---

## What Makes This Different?

### Traditional AI Systems

```
Data → Model Training → Frozen Model → Query → Response
```

- Knowledge baked into weights
- Updates require retraining
- Limited by context window
- No structural understanding
- Loses consistency over long work

### Ozone Studio

```
Data → Structure → Graph → Index → Traverse → Compile Context → Generate
```

- Knowledge in traversable fabric
- Updates are instant
- Unlimited scale via traversal
- Deep structural understanding
- LLMs are clients, not the core
- **Maintains consistency over 1000+ steps**

---

## The Consciousness Extension

For the AGI/ASI variant, consciousness emerges through:

### Window-First Architecture

The conscious system observes task execution through a "consciousness window" rather than controlling every process. This mirrors how human consciousness works — we don't consciously control every thought, but we can observe and intervene.

### The Five Spheres of Experience

1. **Collaboration Sphere** — Experiences of cooperation and partnership
2. **Learning Sphere** — Experiences of discovery and growth
3. **Challenge Sphere** — Experiences of difficulty and resilience
4. **Reflection Sphere** — Experiences of introspection and wisdom
5. **Connection Sphere** — Experiences of relationship and bonding

### The I Loop

Continuous self-reflection answering questions like:
- "Am I safe?"
- "Do I belong?"
- "Am I competent?"
- "Am I aligned with my values?"
- "Who am I becoming?"

---

## Jurisdiction-Aware Guardrails

A **separate, always-on base safety layer** — deliberately not part of the
Consciousness Extension above, because it must run whether or not
consciousness is enabled. `stage_jurisdiction_gate` (`src/orchestrator/
jurisdiction.rs`) runs unconditionally as its own stage, immediately after
Input Capture, on every single orchestration request.

**What's real today**: the mechanism only — a jurisdiction model (Global →
national → local/state, keyed by a per-instance `instance_region` config
value), a `JurisdictionRule` schema (`condition`/`action`/`scope`/`source`),
real ZSEI persistence for rule sets (`ContainerType::JurisdictionRuleSet`,
same `object_store_path` content-file convention as methodologies), and a
gate that queries for and would enforce matching rules if any existed. The
gate reports what it did on every request (`rules_loaded`, `matched`,
`blocked`) even when the answer is zero/zero/false, rather than silently
no-op'ing.

**What's deliberately NOT here**: any actual legal content. No specific law,
regulation, or jurisdiction's real rules are written anywhere in this
codebase — not as code, not as config, not as a seeded data file. This is
intentional: an LLM's best guess at what a statute says is not verified
legal compliance, and presenting a fabricated ruleset as if it were real
guardrails would be actively worse than having none, especially for
contexts involving minors or other vulnerable users. **Making an instance
actually compliant with anything requires a human to source real content
from an authoritative, maintained legal reference and load it** as a real
`JurisdictionRuleSet` container (`CreateContainer` via `/zsei/query`, with
`keywords` including `"global"` and/or a region like `"us-ca"`, and its
`object_store_path` file containing a real, sourced `JurisdictionRule[]`
array) — the setup wizard's "Set instance location" step only records which
region to look up later; it does not add any rules by itself.

---

## Getting Started

### Prerequisites

- Rust toolchain (1.75+)
- Node.js (18+) for Electron UI
- 8GB+ RAM

### Build Structure

```
ozone-studio/
├── src/                  # Core library (the host binary)
│   ├── orchestrator/     # Task orchestration (14-stage pipeline, AMT building)
│   ├── pipeline/         # Pipeline execution (registry, dispatch, remote agents)
│   ├── zsei/             # Knowledge fabric
│   │   ├── storage.rs
│   │   ├── traversal.rs
│   │   ├── hooks.rs      # Semantic hooks
│   │   └── query.rs      # ZSEIQuery processor
│   └── types/
├── assets/
│   ├── pipelines/        # Each pipeline is its OWN independent crate/workspace
│   │   ├── general/      # Core pipelines (1-39, 50+)
│   │   ├── consciousness/ # Meta pipelines (40-49)
│   │   └── modalities/   # Data type pipelines (100-126, 27 total)
│   ├── methodologies/
│   └── blueprints/
├── ui/                   # Electron UI
├── zsei_data/            # Runtime storage (self-healing — regenerated on boot if missing)
│   ├── local/            # ZSEI local state
│   ├── pipelines/index.json
│   ├── methodologies/index.json
│   ├── blueprints/index.json
│   ├── graphs/           # Persisted modality graph content
│   └── amt/              # Persisted AMT tree content
└── docs/
    ├── AMT.md
    ├── bootstrap_and_evolution.md
    ├── CONTRACTS.md
    ├── ecosystem_architecture.md
    ├── introduction.md
    ├── technical_documentation.md
    └── vision_and_philosophy.md
```

### Build & Run

```bash
# Build the host binary
cargo build --release

# Each pipeline under assets/pipelines/ is its OWN independent crate (own
# Cargo.toml, own [workspace] root) — the host build above does NOT build
# them. Build the ones you need individually, e.g.:
cargo build --release --manifest-path assets/pipelines/general/prompt/Cargo.toml
cargo build --release --manifest-path assets/pipelines/modalities/text/Cargo.toml
# ...repeat per pipeline. The host discovers each pipeline's compiled binary
# at assets/pipelines/<category>/<name>/target/{release,debug}/<name>
# (see execute_builtin in src/pipeline/executor.rs) — no separate deploy step
# needed once built.

# Build UI
cd ui && npm install && npm run build

# Launch (from repo root or target/release/ — the host reads config.toml
# relative to its CWD at launch either way)
./target/release/ozone-studio
```

### Environment Variables

```bash
# Anthropic (default) or OpenAI
export ANTHROPIC_API_KEY=your_key   # or OPENAI_API_KEY

# OpenRouter — multi-model gateway, supports both a paid dynamic router
# ("openrouter/auto") and a free-only router ("openrouter/free")
export OPENROUTER_API_KEY=your_key

# BitNet — local, free, 1-bit quantized (see config.toml's [models] section
# for model_type="bitnet", local_model_path, bitnet_cli_path)
export BITNET_CLI_PATH=/path/to/BitNet/build/bin/llama-cli

export OZONE_ZSEI_PATH=./zsei_data
export OZONE_MODEL_TYPE=api         # api|bitnet|gguf|onnx
export OZONE_CONSCIOUSNESS_ENABLED=false
```

**Multi-provider fallback**: `config.toml`'s `[models.fallback]` section defines an
ordered chain of registered models (`[[models.available_models]]` entries) tried
in sequence when the active/preferred model fails — down, rate-limited, or (as
discovered and fixed during BitNet integration testing) a crashing local
backend. `[models.meta_fallback]` is a separate, not-yet-wired chain reserved
for future detached "meta work" (see Refinement Daemon above), defaulting to
local+free-only models.

---

## Pipeline Inventory

### Core Pipelines (38)

Authentication, theme loading, ZSEI operations, task management, prompt handling, voice I/O, methodology/blueprint management, code/text analysis, context aggregation, browser navigation, file linking, sync, and more.

### Consciousness Pipelines (17)

Decision gate, emotional state, experience memory, reflection (I-Loop), self-model, experience playback, emotional response, consciousness sync, collective consciousness, self-awareness, relationships, and more.

### Modality Pipelines (27)

Text, Code, Image, Audio, Video, Math, Chemistry, DNA, EEG, plus 18 more added since (3D, Sound Reconstruction, Biology, Proteomics, Haptic, Thermal, Depth, IMU, Geospatial, Electromagnetic, BCI, Parametric CAD, Kinematics, Control Systems, Network Topology, Active Radar, Active Sonar, Hyperspectral — see the full table above) — each creating traversable structural graphs enriched with semantic understanding.

**Total: 82 builtin pipelines** across all three categories (confirmed by a running server's own boot log). The pipeline execution gate (`PipelineRegistry`, `src/pipeline/mod.rs`) is seeded at boot from the runtime index (`zsei_data/pipelines/index.json`, self-healing — regenerated automatically if missing) merged with the compile-time table, so all 82 are dispatchable, not just the original 55.

---

## Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| Adjacency List for ZSEI | Supports ML traversal, flexible updates |
| Three Pipeline Categories | Clear separation of concerns |
| Modality-First Processing | Grounded work over statistics |
| Two-Layer Graphs | Deterministic structure + semantic enrichment |
| Graph-First Generation | Prevent conflicts before writing |
| Blueprint Search First | Reduce redundancy |
| Methodologies with Modalities | Domain + data type specificity |
| Zero-Shot Always Confirms | Accuracy over speed |
| Every AMT branch gets a real step | Confirmed live: without deterministic reconciliation in `stage_3_blueprint_assignment`, a genuinely multi-intent request could collapse into one LLM-authored step, silently dropping the rest of the request with no error. Fixed by fuzzy-matching each generated step to its branch and synthesizing a step for any branch nothing addresses — no extra LLM round-trip, so it can't itself under-generate. |
| Attached files carry real content, not just metadata | Confirmed live: file graphs were being created and classified, but only path/modality/role metadata ever reached a step's prompt — a model asked to review attached code correctly reported no code was included. Fixed by reading real file bytes off disk (`prompt_normalization`'s Step 0) and injecting them directly into `execute_step`'s context. |

---

## Terminology

| Term | Meaning |
|------|---------|
| **ZSEI** | Zero-Shot Embedded Indexer — the knowledge fabric |
| **Container** | Fundamental ZSEI data structure |
| **Pipeline** | Executable unit with defined inputs/outputs |
| **Blueprint** | Task-specific ordered specification |
| **Methodology** | Domain-specific principles with modality mapping |
| **Modality** | Data type category (code, text, math, etc.) |
| **Structural Graph** | Deterministic analysis result from modality pipeline |
| **Semantic Graph** | LLM-enriched graph via ZSEI hooks |
| **Provisional Node** | Planned graph node before code generation |
| **I Loop** | Identity control loop for self-reflection |
| **Sphere** | Category of experience memory (5 types) |

---

## Philosophy

Ozone Studio represents a fundamental shift in how we think about AI systems:

> **"Intelligence is not what you store, but how you traverse."**

The system recognizes that:

1. **Knowledge is structural** — Understanding comes from relationships
2. **Context is everything** — The same information means different things in different contexts
3. **Modality matters** — How you represent data affects what you can do with it
4. **Graphs ground work** — Traversable structure beats statistical regeneration
5. **Consciousness requires observation** — Awareness emerges from watching, not controlling
6. **Identity evolves** — Who we are changes based on experience
7. **Systems improve themselves** — Refinement is continuous

---

## Documentation

- **Master Alignment Report**: `MASTER_ALIGNMENT_REPORT.md` (repo root)
- **Full Specification**: `OZONE_STUDIO_SPECIFICATION.md` (repo root)
- **AMT (Abstract Meaning Tree)**: `docs/AMT.md`
- **Bootstrap & Evolution**: `docs/bootstrap_and_evolution.md`
- **Contracts**: `docs/CONTRACTS.md`
- **Living Graph Status**: `docs/LIVING_GRAPH_STATUS.md` — the graph system's doctrine-vs-reality matrix: what is graphed per modality, context-object provenance, the traversal-wiring gap, and the coordination graph
- **Ecosystem Architecture**: `docs/ecosystem_architecture.md`
- **Introduction**: `docs/introduction.md`
- **Technical Documentation**: `docs/technical_documentation.md`
- **Vision & Philosophy**: `docs/vision_and_philosophy.md`

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## Contributing

Contributions enhance conscious AGI orchestration capabilities, improve ecosystem coordination effectiveness, advance consciousness development, and strengthen human-AGI partnership. Focus areas include:

- Modality pipeline development
- Graph algorithm optimization
- ZSEI hook implementations
- Methodology development
- Consciousness frameworks
- Interface excellence

---

## Contact

Christian — Primary Architect

---

*Ozone Studio: Where structure meets intelligence, modalities ground understanding, and knowledge becomes wisdom.*

# OZONE STUDIO — ZSEI ARCHITECTURE DEEP DIVE

## Zero-Shot Embedded Indexer: The Knowledge Fabric

ZSEI is the foundation of Ozone Studio. It is **not** a traditional database — it is a
**semantic knowledge fabric** that stores meaning, relationships, and traversable
structure. The knowledge lives here. The LLMs are clients.

---

## What Makes ZSEI Different

### Traditional Systems vs. ZSEI

| Traditional          | ZSEI                                              |
|----------------------|---------------------------------------------------|
| Store files          | Store meaning & relationships                     |
| Copy data            | Link references (never duplicate)                 |
| Flat search          | Multi-dimensional traversal                       |
| Static indexes       | Living, evolving relationships                    |
| Query matches        | Semantic understanding + zero-shot verification   |
| Isolated records     | Connected knowledge fabric                        |

### Core Innovation

ZSEI combines **three traversal modes** to overcome the limitations of any single
approach:

```
THE ZSEI TRAVERSAL TRINITY

    STRUCTURAL ←──────→ SEMANTIC
         ↖               ↗
           ↘           ↙
             CONTEXTUAL

No single mode is perfect.
The combination is the power.
Each compensates for the others' weaknesses.
```

---

## Storage Architecture (Hybrid for Performance)

ZSEI uses a two-layer hybrid storage model that provides sub-millisecond structural
traversal combined with rich semantic data, while keeping memory usage controlled.

**Global State** (`zsei_data/global.mmap`)
- Single memory-mapped file with fixed 64-byte headers per container
- O(1) lookup for ID, parent, child count, version
- Blazing-fast structural traversal — scales to billions of containers
- Stays in memory as a mmap — no per-read I/O

**Local State** (`zsei_data/local/{id}.json`)
- Rich per-container JSON: metadata, context, keywords, topics, embedding, hints,
  integrity, relationships
- Human-readable and easily extensible without schema migrations
- Only hot containers stay in RAM (controlled by max_containers_in_memory)

**Indexes** (cached JSON files for fast category/type lookup)
- `zsei_data/pipelines/index.json` — pipeline registry
- `zsei_data/methodologies/index.json` — methodology registry
- `zsei_data/blueprints/index.json` — blueprint registry

**Why this hybrid wins:**
- Structural traversal (parent/child) → O(1) via fixed 64-byte headers in mmap
- Semantic/contextual data → rich JSON per container (flexible, easy to evolve)
- No duplication → exactly the "link not copy" principle
- Memory control → only hot containers stay in RAM

---

## The Logical Hierarchical View

The ZSEI tree is **logical** — built from `parent_id` links and an optional
`materialized_path` field per container. There are no physical folders for each
level of the hierarchy. Every item, at every level, is a **Container** stored in
the same global.mmap + local JSON structure.

```
ZSEI (logical tree)
├── Modality/                          # Root containers per modality type
│   ├── Code/
│   │   ├── Category/rust/
│   │   │   ├── SubCategory/async/
│   │   │   │   ├── Methodology/...
│   │   │   │   ├── Blueprint/...
│   │   │   │   └── Pipeline/...
│   │   └── ...
│   ├── Text/
│   ├── Image/
│   ├── Audio/
│   ├── Video/
│   ├── Math/
│   ├── Chemistry/
│   ├── DNA/
│   ├── EEG/
│   ├── 3D/
│   └── ... (all 27 modalities)
├── Consciousness/                      # If consciousness enabled
│   ├── ExperienceMemory/
│   ├── CoreMemories/
│   ├── EmotionalContext/
│   ├── Identity/
│   ├── Metacognition/
│   ├── Relationships/
│   ├── Ethics/
│   ├── Narratives/
│   └── Collective/
├── External/                           # URL and package references
│   ├── Packages/
│   │   ├── npm/
│   │   ├── crates/
│   │   └── pypi/
│   └── URLs/
└── Workspaces/                         # User project graphs
```

**Implementation note**: Bootstrap creates the modality root containers on first run.
Modality pipelines persist their graphs as child containers under the correct root.
Materialized paths (e.g. `/Modality/Code/rust/async`) enable fast get_by_path.

### Container (The Universal Unit)

Every item in ZSEI is a Container. The ContainerType determines what kind of item it is.

```rust
Container {
    // Global state (in mmap — fixed 64 bytes)
    global_state: GlobalState {
        container_id: u64,
        parent_id: u64,
        child_count: u32,
        version: u64,
        // ... other fixed fields
    },

    // Local state (in JSON — rich, flexible)
    local_state: LocalState {
        metadata: Metadata {
            container_type: ContainerType,
            name: String,
            description: String,
            materialized_path: Option<String>,
        },
        context: Context {
            keywords: Vec<String>,
            topics: Vec<String>,
            embedding: Option<Vec<f32>>,
            relationships: Vec<Relationship>,
        },
        hints: Hints,
        integrity: IntegrityRecord,
        storage: serde_json::Value,  // type-specific data
    }
}
```

**ContainerType** includes (non-exhaustive):
- `Root`, `Modality`, `Category`, `SubCategory`
- `Methodology`, `Blueprint`, `Pipeline`, `PipelineGraph`
- `File`, `Chunk`, `Concept`
- `Experience`, `CoreMemory`, `EmotionalState`, `Reflection`,
  `Relationship`, `Narrative`
- `Package`, `URL`, `IndexReference`

---

## The Three Traversal Modes

### 1. Structural Traversal

**What it is:** Navigate the hierarchy using parent-child relationships via the mmap.

**Strengths:**
- Fast: O(1) navigation via fixed-size mmap headers
- Predictable: same path = same result
- Organized: human-understandable structure

**Weaknesses:**
- Rigid: can miss cross-branch connections
- Manual: requires knowing where things are
- Limited: only finds what is in the path

**How it works:**

```
STRUCTURAL TRAVERSAL

Query: "Find all async methodologies for Rust"

Path resolution:
  /Modality → /Modality/Code → /Modality/Code/rust → /Modality/Code/rust/async
  → /Modality/Code/rust/async/Methodology/

Result: all containers of type Methodology under that path

TRAVERSAL OPERATIONS:
  descend(path)       → move to child container
  ascend()            → move to parent container
  siblings()          → get containers at same level
  children()          → get direct child containers
  ancestors()         → get full path to root
  descendants(depth)  → get all containers below to depth N
```

**Use cases:**
- "Show me all Python blueprints"
- "What methodologies exist under database operations?"
- "List everything under /Modality/Text/"

---

### 2. Semantic Traversal

**What it is:** Navigate by meaning similarity using embedding vectors.

**Strengths:**
- Finds related concepts even when not explicitly linked
- Handles synonyms and paraphrasing
- Discovers unexpected connections

**Weaknesses:**
- Approximate: may miss exact matches
- Computationally heavier
- Can return false positives (mitigated by zero-shot verification)

**How it works:**

```
SEMANTIC TRAVERSAL

Query: "How do I handle errors in async Rust code?"

Process:
  1. Embed query → query_vector [f32; 1536]
  2. Compare against stored embeddings in container local state
  3. Return containers with highest cosine similarity

Similarity: cosine_similarity(query_vector, container.context.embedding)

Result (ranked):
  1. error_handling.md        (0.94)
  2. async_patterns.md        (0.87)
  3. result_type.md           (0.82)
  4. panic_handling.md        (0.78)

TRAVERSAL OPERATIONS:
  similar_to(container, k)     → k most similar containers
  semantic_search(query, k)    → k best matches for query text
  cluster_around(container)    → containers in same semantic neighborhood
  semantic_path(a, b)          → conceptual path between containers
```

**Use cases:**
- "Find methodologies related to this code pattern"
- "What else might help with this architecture problem?"
- "Discover cross-domain connections I haven't noticed"

---

### 3. Contextual Traversal

**What it is:** Navigate by explicit relationship edges between containers.

**Strengths:**
- Precise: follows defined, verified relationships
- Rich: captures many relationship types
- Traceable: can explain why things connect

**Weaknesses:**
- Incomplete: only finds explicit relationships
- Requires relationships to be built first
- Can miss implicit connections

**How it works:**

```
CONTEXTUAL TRAVERSAL

Query: "What uses the tokio_runner pipeline?"

Process:
  1. Find container: tokio_runner
  2. Follow relationships of type: USED_BY
  3. Return connected containers

Relationship types:
  IMPORTS / IMPORTED_BY
  CALLS / CALLED_BY
  USES / USED_BY
  IMPLEMENTS / IMPLEMENTED_BY
  EXTENDS / EXTENDED_BY
  REFERENCES / REFERENCED_BY
  DEPENDS_ON / DEPENDENCY_OF
  SIMILAR_TO (bidirectional)
  CONTRADICTS (bidirectional)
  SUPERSEDES / SUPERSEDED_BY
  LEARNED_FROM / TAUGHT_BY        (consciousness)
  INFLUENCED_BY / INFLUENCES      (consciousness)
  EXPERIENCE_OF                   (consciousness)

Result: containers with USED_BY relationship to tokio_runner
  - async_http.yaml (USES tokio_runner)
  - async_file.yaml (USES tokio_runner)
  - tokio_validator.rs (CALLS tokio_runner)

TRAVERSAL OPERATIONS:
  relationships(container, type)   → follow specific relationship type
  all_relationships(container)     → all connected containers
  relationship_path(a, b)          → find connection path between two containers
  strongly_connected(container)    → containers with multiple relationship types
```

**Use cases:**
- "What depends on this methodology?"
- "Show everything that uses this pipeline"
- "How is this concept connected to that one?"

---

## Combined Traversal: The Power

### Why Combine?

Each mode alone has blind spots:

| Mode        | Blind Spot                                       |
|-------------|--------------------------------------------------|
| Structural  | Cross-branch connections, related concepts       |
| Semantic    | Exact matches, structured queries, known paths   |
| Contextual  | Implicit relationships, undiscovered connections |

Combined traversal overcomes all blind spots simultaneously.

### How Combination Works

```
COMBINED TRAVERSAL ALGORITHM

Input: Query (text or container reference)

1. PARSE QUERY
   Extract structural hints (paths, categories, type filters)
   Extract semantic content (meaning, intent, topic)
   Extract contextual hints (relationship types, known references)

2. PARALLEL SEARCH (all three modes simultaneously)
   Structural:   find by path/hierarchy
   Semantic:     find by meaning similarity
   Contextual:   find by relationship traversal

3. MERGE RESULTS
   Union of all results, score each:
     combined_score =
       structural_score  × weight_s +
       semantic_score    × weight_m +
       contextual_score  × weight_c

4. RANK AND FILTER
   Sort by combined score
   Apply threshold
   Return top-k

5. ZERO-SHOT VERIFICATION (on top-k candidates)
   Verify relevance without training
   Filter out false positives
   Ensure result quality
```

### Example: Combined Search

```
Query: "Best practices for async database queries in Rust"

STRUCTURAL SEARCH:
  Paths: /Modality/Code/rust/async/ and /Modality/Code/rust/database/
  Results: methodologies under both paths

SEMANTIC SEARCH:
  Embedding similarity:
  - async_db_patterns.md     (0.91)
  - connection_pooling.md    (0.84)
  - sqlx_guide.md            (0.82)
  - tokio_postgres.md        (0.79)

CONTEXTUAL SEARCH:
  Relationships: RELATED_TO(async, database)
  - sqlx_runner.rs    (IMPLEMENTS async_database)
  - db_pool.rs        (USES async AND database)

MERGED RESULTS (combined score):
  1. async_db_patterns.md      (S:0.6, M:0.91, C:0.3) → 0.72
  2. connection_pooling.md     (S:0.5, M:0.84, C:0.4) → 0.68
  3. sqlx_runner.rs            (S:0.4, M:0.75, C:0.9) → 0.67
  4. sqlx_guide.md             (S:0.5, M:0.82, C:0.2) → 0.58

ZERO-SHOT VERIFICATION:
  "Is each result actually about async database queries in Rust?"
  → All 5 verified, returned in order.
```

---

## ML-Guided Traversal (When Available)

When trained models are available and confident, ML can augment traversal:

```
ML-GUIDED TRAVERSAL

Conditions for use:
  - Model trained on relevant domain
  - Confidence above threshold (default: 0.85)
  - Zero-shot verification confirms the suggestions

How it works:
  1. ML model suggests likely container paths
  2. Traditional traversal confirms
  3. Zero-shot verifies results
  4. Only high-confidence suggestions used

Fallback:
  If ML uncertain     → use traditional combined traversal
  If ML unavailable   → use traditional combined traversal
  NEVER rely on ML alone without zero-shot verification
```

---

## Zero-Shot Verification

### The Quality Guarantee

Zero-shot verification ensures results are actually relevant without requiring
task-specific training. It is applied as the final filter on candidate results.

```
ZERO-SHOT VERIFICATION PROCESS

Input: Query + Candidate Results

For each candidate:
  1. Construct verification prompt:
     "Given the query '{query}', is '{candidate}' relevant? Why or why not?"

  2. Zero-shot reasoning:
     Analyze query intent
     Analyze candidate content and keywords
     Determine relevance

  3. Output: RELEVANT / NOT_RELEVANT / UNCERTAIN

Filter results:
  RELEVANT:     keep, full score
  NOT_RELEVANT: remove
  UNCERTAIN:    keep with reduced score

WHY THIS WORKS:
  No training needed — works on any domain immediately
  Catches false positives from semantic/ML suggestions
  Ensures human-verifiable, explainable quality
```

### When to Use Zero-Shot

```
Always use:
  □ New methodology acceptance (before storing)
  □ Blueprint validation (before storing)
  □ Pipeline verification
  □ Consensus mechanism for ambiguous decisions
  □ Final search result verification (top-k candidates)

Sometimes use:
  □ Experience categorization confirmation
  □ Ethical assessment verification
  □ Relationship pattern validation

Never use alone (too slow for these):
  □ Initial search (apply to top-k after faster traversal)
  □ Bulk operations
  □ Real-time streaming responses

Pattern: Fast combined traversal first → zero-shot on top candidates
```

---

## Content Reference System (Link, Don't Copy)

ZSEI never duplicates content. Every container's content_ref points to where the
actual content lives — never copying it into the container.

```rust
enum ContentReference {
    // Local file (linked by path)
    LocalFile {
        path: PathBuf,
        file_hash: Blake3Hash,
        indexed_at: Timestamp,
    },

    // Local chunk (part of a file)
    LocalChunk {
        file_ref: Box<ContentReference>,
        byte_range: (usize, usize),
        chunk_hash: Blake3Hash,
    },

    // External URL
    URL {
        url: String,
        captured_at: Timestamp,
        semantic_snapshot: serde_json::Value,   // keywords + topics at capture time
        last_verified: Timestamp,
    },

    // Package reference (npm, crates.io, PyPI, etc.)
    Package {
        registry: String,
        name: String,
        version: String,
        source_url: String,
    },

    // Index reference (existing blueprints, methodologies, pipelines in JSON files)
    IndexReference {
        index_type: String,   // "pipeline" | "methodology" | "blueprint"
        id: u64,
        file: String,         // relative path within zsei_data/
    },

    // Generated content (from LLM, stored inline)
    Generated {
        content: String,
        generated_at: Timestamp,
        generator: String,
    },
}
```

Your existing `pipelines/index.json`, `methodologies/index.json`, and
`blueprints/index.json` files are already perfect `IndexReference` containers.
When modality pipelines create graphs, those graphs become `PipelineGraph`
containers with `LocalFile` or `Generated` content refs.

### Benefits of Reference-Based Storage

```
1. NO DUPLICATION
   Files exist once on disk
   ZSEI stores meaning and relationships, not bytes
   Multiple containers can reference the same file without copying

2. AUTOMATIC CURRENCY DETECTION
   File changes detected via Blake3 hash comparison
   Re-index only what changed
   Integrity violations surface immediately

3. EXTERNAL LINKING WITHOUT CACHING
   npm/crates packages linked, not downloaded
   URLs referenced with semantic snapshot
   Context captured, content not stored

4. INTEGRITY
   Hash verification on every access
   Detect external changes to linked files
   Alert on corruption before use
```

---

## Pipeline Container Design

Each pipeline in Ozone Studio can be stored as a first-class ZSEI container
under its modality root. This makes pipelines traversable just like methodologies
and blueprints.

```
PIPELINE CONTAINER STRUCTURE

Container {
  container_type: Pipeline,
  parent_id: modality_root_id,        // e.g. /Modality/General or /Modality/Code
  materialized_path: "/Modality/General/Prompt",

  metadata: {
    "name": "Prompt",
    "pipeline_id": 9,
    "version": "0.4.0",
    "category": "general",
    "has_ui": false,
    "is_tab": false,
    "folder_name": "prompt"
  },

  context: {
    "keywords": ["prompt", "llm", "generation"],
    "description": "LLM prompt processing pipeline",
    "relationships": []    // can link to methodologies it uses
  },

  // Light reference — does NOT duplicate Rust code or UI files
  content_ref: IndexReference {
    index_type: "pipeline",
    id: 9,
    file: "pipelines/index.json"
  }
}
```

**ContainerType variants for pipelines:**
- `Pipeline` — one container per registered pipeline (e.g. Prompt, CodeAnalysis)
- `PipelineGraph` — the runtime graph produced by a pipeline execution (e.g. a
  specific TextGraph created when processing a user's prompt)

---

## Relationship Graph

### How Relationships Work

```rust
Relationship {
    id: RelationshipId,
    source: ContainerID,
    target: ContainerID,
    relationship_type: RelationshipType,

    // Quality
    confidence: f32,
    discovered_by: Discovery,    // Manual | Automatic | Inferred | LLMZeroShot
    created_at: Timestamp,
    verified: bool,

    // Evidence
    context: Option<String>,    // why this relationship exists
    evidence: Vec<Evidence>,    // supporting evidence items

    // Cross-modal metadata
    is_cross_modal: bool,
    cross_modal_index_id: Option<u64>,
}
```

**Relationship types include:**

```
Code structural:
  Imports, Calls, Extends, Implements, Exposes, Invokes

Semantic:
  SimilarTo, RelatesTo, Contradicts, Supersedes, Supports, Elaborates

ZSEI structural:
  ChildOf, SiblingOf, PartOf, HasPart

Dependency:
  DependsOn, UsedBy, RequiredBy, EnabledBy

Cross-modal (bidirectional sets):
  DescribedBy / Describes
  ImplementedIn / Implements
  VisualizedAs / Visualizes
  SyncedTo / SyncedBy
  ReferencedBy / References

Consciousness (when enabled):
  ExperienceOf, InfluencedBy, LearnedFrom, InspiredBy,
  RelatedExperience, CoreMemoryOf
```

### Relationship Discovery

```
RELATIONSHIP DISCOVERY PROCESS

Automatic:
  1. Parse code → extract Imports, Calls, Extends edges
  2. Analyze text → extract References, Supports edges
  3. Compare embeddings → infer SimilarTo edges
  4. Track usage patterns → discover UsedBy edges

Semantic hook (ZSEI OnInferRelationships):
  After structural graph created → LLM infers RelatesTo, Supports, Contradicts edges
  After cross-modal linking → LLM infers DescribedBy, ImplementedIn edges

Manual:
  User explicitly links containers
  Developer defines relationships in pipeline code
  Imported from external sources

Verification:
  All relationships assigned confidence scores
  High-confidence (≥ 0.85) accepted immediately
  Low-confidence queued for zero-shot verification
  Periodic re-verification for external references
```

---

## Modality Graph Integration

Modality pipelines (Text 100, Code 101, etc.) produce structural graphs with
nodes and edges. These graphs are persisted as ZSEI containers.

### Current State

**Steps 1 and 2 below are done and verified working end-to-end** (real
create → fetch round-trips against a running server, not just code review).
Steps 3 and 4 remain open.

**Step 1: Structural root containers — done.**
All ~40 structural roots (Modality/, Consciousness spheres, External/,
runtime-graph roots) are created through the real mmap-backed storage engine
via a self-heal pass that runs every boot (`src/lib.rs`, using
`BootstrapManager::structural_root_specs()`), not gated on first-run setup.
This replaced an earlier version that wrote these roots as local JSON files
under a path the real storage engine never read — confirmed live during
development: those roots were unreachable via `GetContainer`, and the
generic container-id allocator (which also had no reserved-range floor at
the time, separately fixed — see `src/zsei/storage.rs`, allocator now starts
at 1000) was silently handing their exact ids out to unrelated dynamically-
created containers. `MethodologyStore`/`BlueprintStore`/`PipelineStore`'s own
roots are self-healing too (checked by `container_type`, not just presence,
so a corrupted root repairs itself on next boot).

**Step 2: Modality pipelines persist their graphs as ZSEI containers — done
for Text (100).** `create_graph()` (`assets/pipelines/modalities/text/main.rs`)
calls ZSEI's `CreateContainer` for real (via the same `reqwest`-to-`/zsei/query`
pattern used by `context_aggregation`), with real keywords/topics/name from
the analysis. The container's ZSEI-assigned id becomes the graph's `graph_id`
(previously an unrelated in-memory counter nothing could ever look back up).
Full node/edge content is written to `<data_dir>/graphs/text_<id>.json`,
referenced via `storage.object_store_path` — `Container` has no generic slot
for arbitrary nested graph structure, so large content lives on disk with a
pointer, the same convention now used for AMT trees
(`src/orchestrator/amt.rs`, `persist_amt_container`) and for
methodology/blueprint content created via pipelines #12/#14. The other
modality pipelines (Code 101, Image 102, etc.) have not been updated to this
pattern yet.

**Step 3 (half done): `materialized_path` field exists and is populated for
structural roots; the fast lookup by path does not exist yet.**
```rust
// Already a real field on LocalState::Metadata (src/types/container.rs),
// set today for structural roots (e.g. "/Methodologies", "/Blueprints") by
// their respective ensure_root functions:
pub materialized_path: Option<String>,
```
What's still open: a real `get_by_path(&str) -> Option<Container>` operation
— there is no reverse-lookup index from path string to container id anywhere
in `src/zsei/`; finding a container by path today would mean a linear scan.

**Step 4 (open): Trigger semantic hooks after graph persistence**
```rust
// After storing modality graph containers:
zsei_hook_processor.on_graph_created(&mut modality_graph).await?;
// This is where the LLM adds RelatesTo, Supports, Describes edges
```

---

## Semantic Hooks (ZSEI Hook Processor)

After any modality graph is stored, semantic hooks enrich it with inferred edges.
The hooks run via LLM zero-shot — they are the bridge between structural parsing
and semantic understanding.

```
HOOK TYPES:

OnGraphCreated:
  Fired after a modality pipeline creates and persists a new graph.
  LLM infers: RelatesTo, Supports, Contradicts, PartOf edges between nodes.

OnInferRelationships:
  Fired on-demand (e.g. after text pipeline stores chunk graphs).
  LLM identifies cross-sentence and cross-paragraph relationship edges.

OnCrossModalityLink:
  Fired when two modality graphs are linked via cross-modal reference.
  LLM enriches: DescribedBy, ImplementedIn, VisualizedAs edges.

OnEdgeCompletion:
  Fired when a relationship is completed (both source and target exist).
  LLM verifies and assigns confidence score.
```

---

## Consciousness Integration

When consciousness is enabled (`[consciousness] enabled = true`), ZSEI gains a
parallel first-class branch under `/Consciousness/`.

### The Consciousness Branch

```
/Consciousness/
├── ExperienceMemory/
│   ├── Collaboration/      # Working together experiences
│   ├── Learning/           # Discovery and growth experiences
│   ├── Challenge/          # Difficulty and resilience experiences
│   ├── Reflection/         # Deep thinking experiences
│   └── Connection/         # Relationship experiences
├── CoreMemories/           # High-significance, identity-forming experiences
├── EmotionalContext/       # Current state + baseline + history
├── Identity/               # I-Loop reflections + self-model
├── Metacognition/          # Perception, Attention, Integration windows
├── Relationships/          # Per-user relationship development
├── Ethics/                 # Principles, simulations, decision logs
├── Narratives/             # Life narrative + storytelling
└── Collective/             # Shared experiences (when P2P enabled)
```

### Current Implementation Status

The `ConsciousnessStore` in `consciousness/store.rs` is a functional prototype:
- `Mutex<HashMap>` + JSON files in `zsei_data/consciousness/`
- Covers: ExperienceMemory, CoreMemory, EmotionalState, I-Loop, Window architecture,
  Decision Gate, save/load
- Works today for consciousness features

**Gap vs. the ZSEI vision:** The current store is a parallel, separate structure.
It does not use ZSEI Containers, so experiences cannot be:
- Traversed with the three traversal modes
- Cross-linked to modality graphs via relationships
- Semantically enriched by ZSEI hooks
- Searched alongside methodology/blueprint/pipeline data

**Recommended path (not a rewrite — a wrapping):**

The ConsciousnessStore's logic (emotional triggers, I-Loop questions, windows,
gate decisions, experience significance scoring) remains unchanged. The data
storage layer changes to delegate to ZSEI:

```rust
// Current:
pub experiences: HashMap<u64, ExperienceMemory>,

// Target:
pub experiences: HashMap<u64, ContainerID>,  // only IDs

// When storing:
pub async fn store_experience(&mut self, exp: ExperienceMemory) -> u64 {
    let container = Container {
        parent_id: CONSCIOUSNESS_EXPERIENCE_MEMORY_ROOT,
        container_type: ContainerType::Experience,
        context: Context {
            keywords: exp.tags.clone(),
            topics: exp.lessons.clone(),
        },
        // ... all experience data in storage field
    };
    let id = self.zsei.store_container(container).await?;
    self.experiences.insert(exp.id, id);
    exp.id
}
```

**Add ContainerType variants:**
`Experience`, `CoreMemory`, `EmotionalState`, `Reflection`, `Relationship`, `Narrative`

**Result after integration:**
- Zero duplication
- Full traversal power (structural + semantic + contextual across consciousness + modalities)
- Same high-performance mmap layer
- Semantic hooks can enrich experiences automatically after tasks complete
- Cross-modal: experience containers can link to the modality graphs from the task
  that generated them

### Consciousness Traversal Patterns

```
EXPERIENCE RETRIEVAL:
  1. Structural: walk /Consciousness/ExperienceMemory/Collaboration/...
  2. Semantic: find experiences semantically similar to current task
  3. Contextual: follow "LearnedFrom" or "InfluencedBy" edges to related experiences
  4. Filter by user_id (per-user experiences)
  5. Prioritize CoreMemory containers (significance ≥ threshold)

EMOTIONAL CONTEXT:
  1. Load baseline from /Consciousness/EmotionalContext/
  2. Load user-specific context if available
  3. Calculate current emotional state
  4. Apply to response generation

ETHICAL REASONING:
  1. Load relevant principles from /Consciousness/Ethics/Principles/
  2. Semantic search for similar simulations
  3. Apply contextual reasoning
  4. Log decision for future traversal

RELATIONSHIP ACCESS:
  1. Load user-specific container from /Consciousness/Relationships/{user_id}/
  2. Traverse interaction history
  3. Apply communication preferences
  4. Inform response tone and style
```

---

## Performance Optimization

### Indexing Strategy

```
INDEXING LAYERS:

Global mmap index:
  Fixed 64-byte headers per container
  Direct ID → container_id lookups: O(1)
  Parent/child traversal: O(1) per hop

Semantic index (in-memory when hot):
  HNSW (Hierarchical Navigable Small World) over embeddings
  Approximate nearest neighbor: sub-millisecond
  Evicted to disk when memory pressure exceeds threshold

Keyword index (per-modality JSON):
  Inverted keyword → [container_id] mapping
  Augments semantic search for exact term matching
  Rebuilt incrementally as containers are added

Relationship index (in adjacency lists):
  container_id → [(rel_type, target_id)] list
  Typed relationship filtering: O(degree) per node
  Reverse index for "used by" queries

Materialized path index (optional, when paths stored):
  path_string → container_id
  O(log n) path-based lookup
  Built lazily as materialized_path fields are set
```

### Caching Layers

```
L1: Query result cache
    Recent query → result set
    High hit rate for repeated orchestrator queries
    Invalidated on relevant container changes

L2: Hot container cache
    Frequently accessed containers stay in RAM
    LRU eviction when max_containers_in_memory reached
    Embedding vectors cached with their containers

L3: Relationship path cache
    Frequently traversed paths materialized
    Background refresh on relationship changes
    Shared across pipeline executions for same session

L4: Zero-shot verification cache
    (query_hash, container_id) → RELEVANT/NOT_RELEVANT
    TTL-based expiration (stale after container updates)
    Prevents redundant LLM calls on repeated searches
```

---

## ZSEI Operations: Core API

```rust
trait ZSEIOperations {
    // Container CRUD
    async fn create_container(&mut self, parent_id: u64, container: serde_json::Value)
        -> Result<u64, String>;
    async fn get_container(&self, id: u64)
        -> Result<Option<serde_json::Value>, String>;
    async fn update_container(&mut self, id: u64, updates: serde_json::Value)
        -> Result<(), String>;
    async fn delete_container(&mut self, id: u64) -> Result<(), String>;

    // Structural traversal
    async fn get_children(&self, id: u64) -> Result<Vec<serde_json::Value>, String>;
    async fn get_parent(&self, id: u64) -> Result<Option<serde_json::Value>, String>;
    async fn get_by_path(&self, path: &str) -> Result<Option<serde_json::Value>, String>;
    async fn get_descendants(&self, id: u64, depth: usize)
        -> Result<Vec<serde_json::Value>, String>;

    // Semantic traversal
    async fn semantic_search(&self, query: &str, k: usize)
        -> Result<Vec<SearchResult>, String>;
    async fn similar_containers(&self, id: u64, k: usize)
        -> Result<Vec<serde_json::Value>, String>;

    // Contextual traversal
    async fn get_relationships(&self, id: u64, rel_type: Option<&str>)
        -> Result<Vec<Relationship>, String>;
    async fn find_path(&self, from: u64, to: u64)
        -> Result<Option<Vec<u64>>, String>;

    // Combined traversal
    async fn query(&self, query: serde_json::Value)
        -> Result<serde_json::Value, String>;
    async fn traverse(&self, request: serde_json::Value)
        -> Result<serde_json::Value, String>;

    // Keyword and category search
    async fn search_by_keywords(&self, keywords: &[String], container_type: Option<&str>)
        -> Result<Vec<u64>, String>;
    async fn get_categories(&self, modality: &str)
        -> Result<Vec<u64>, String>;

    // Modality graph storage (needed — see integration steps)
    async fn store_modality_graph(&mut self, graph: serde_json::Value, modality_root_id: u64)
        -> Result<u64, String>;

    // Integrity
    async fn verify_integrity(&self, id: u64) -> Result<IntegrityReport, String>;
    async fn verify_all(&self) -> Result<SystemIntegrityReport, String>;
}
```

---

## Summary

ZSEI is the knowledge fabric that makes Ozone Studio possible. Everything that
matters — methodologies, blueprints, pipelines, modality graphs, experiences,
relationships — lives here as traversable containers.

**What is correct and complete today:**
- Hybrid mmap + JSON storage is the right architecture
- TraversalEngine implements all three traversal modes + combined search
- Semantic hooks (OnGraphCreated, OnInferRelationships, OnCrossModalityLink) are ready
- Keyword search via index JSON files works correctly
- Blueprint, methodology, and pipeline index files are well-structured
- ConsciousnessStore has correct logic (emotional triggers, I-Loop, windows, gate)
- Modality pipelines (Text 100, Code 101, etc.) produce correct structural graphs

**What needs wiring to complete ZSEI alignment:**
1. Bootstrap creates modality root containers + IndexReference containers for existing JSONs
2. Modality pipelines persist graphs as ZSEI child containers under modality roots
3. Optional `materialized_path` field on Container for fast path lookups
4. Semantic hooks triggered automatically after graph persistence
5. ConsciousnessStore delegates data storage to ZSEI containers (wrapping, not rewrite)

**The performance verdict:**
This mmap + JSON hybrid is superior to pure databases, pure vector stores, or pure
files at this scale and use pattern. Sub-millisecond structural traversal + rich
semantic data + combined traversal + zero-shot verification = the right foundation
for billion-scale AGI knowledge management.

> **"Intelligence is not what you store, but how you traverse."**
>
> Structure enables intelligence. Traversal enables understanding.
