# OZONE STUDIO — UI/UX DEVELOPMENT GUIDE v3.0

## Building the Graph-First Interface for AGI-Native Systems

Ozone Studio runs on diverse platforms — desktop, mobile, AR/VR, terminal, and web. This guide covers how to develop themes and interfaces that adapt to each device's unique capabilities while exposing the full power of our modality-based graph system.

---

## Core Philosophy: AGI-First, Human-Second

Unlike traditional software built for humans first, Ozone Studio is built for **AGI first, humans second**. This means:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        OZONE DESIGN PRINCIPLE                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  TRADITIONAL SOFTWARE:          OZONE STUDIO:                               │
│  ┌─────────────────┐            ┌─────────────────┐                         │
│  │ Human Interface │            │ AGI Interface   │                         │
│  │       ↓         │            │       ↓         │                         │
│  │ Business Logic  │            │ Graph Engine    │                         │
│  │       ↓         │            │       ↓         │                         │
│  │ Data Storage    │            │ Human Interface │                         │
│  └─────────────────┘            └─────────────────┘                         │
│                                                                             │
│  The graph is the source of truth. The UI is a viewport into that graph.    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

This enables:
- **Zero-loss interoperability** between AGI reasoning and human visualization
- **Living systems** where graph state updates cascade to all views
- **Modality engines** that can replace Blender, Gazebo, Isaac Sim, etc.
- **Spatial intelligence** native to the system, not bolted on

---

## Core UI Architecture

### The Universal Structure

Every Ozone interface follows the same fundamental layout:

```
┌────────────────────────────────────────────────────────────────────────────┐
│                                                                            │
│                                                       ┌──────────────────┐ │
│                                                       │                  │ │
│           THEME AREA (80%)                            │   META PORTION   │ │
│                                                       │     (20%)        │ │
│    ┌─────────────────────────────────────────────┐    │                  │ │
│    │                                             │    │  Always          │ │
│    │   TASK VIEWER │ CONTEXT VIEWER │ ENGINES    │    │  Accessible      │ │
│    │                                             │    │                  │ │
│    │   Adapts to device capabilities             │    │  [🏠 Home]       │ │
│    │   Shows: Tasks, Steps, Graphs, Engines      │    │  [Cancel Task]   │ │
│    │                                             │    │  [System Chat]   │ │
│    │                                             │    │  [Emotional      │ │
│    │                                             │    │   State]         │ │
│    └─────────────────────────────────────────────┘    │                  │ │
│                                                       │  Progress        │ │
│                                                       │  Indicators      │ │
│                                                       │                  │ │
│                                                       └──────────────────┘ │
│                                                                            │
├────────────────────────────────────────────────────────────────────────────┤
│  CONNECTION BAR — Network status, contributions, ZSEI depth, graph stats   │
└────────────────────────────────────────────────────────────────────────────┘
```

**Key Principle:** The Meta Portion is NEVER blocked. Users must always be able to:
- Return home
- Cancel tasks
- Access system functions
- See emotional state (if consciousness enabled)
- View task progress

---

## The Three-View System

### 1. Task Viewer

The Task Viewer shows **what the system is doing** — the active work.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ TASK VIEWER                                                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ ACTIVE TASKS                                            [⊕ New Task] │   │
│  ├─────────────────────────────────────────────────────────────────────┤   │
│  │                                                                     │   │
│  │  ┌─────────────────────────────────────────────────────────────┐   │   │
│  │  │ Task #1847: "Create authentication module"                   │   │   │
│  │  │ Blueprint: Code Creation (Step 8/15)                         │   │   │
│  │  │ ████████████████░░░░░░░░░░░░░░░░░░░░  53%                    │   │   │
│  │  │                                                               │   │   │
│  │  │ Current Step: "generate_code"                                │   │   │
│  │  │ Pipeline: 9 (LLM Execute)                                    │   │   │
│  │  │ Tokens: 2,847 / ~5,000                                       │   │   │
│  │  │                                                               │   │   │
│  │  │ [View Steps] [View Graph] [Cancel]                           │   │   │
│  │  └─────────────────────────────────────────────────────────────┘   │   │
│  │                                                                     │   │
│  │  ┌─────────────────────────────────────────────────────────────┐   │   │
│  │  │ Task #1848: "Analyze codebase"                               │   │   │
│  │  │ Blueprint: Code Analysis (Step 3/7)                           │   │   │
│  │  │ ███████████████████████████░░░░░░░░░░  43%                    │   │   │
│  │  │ [View Steps] [View Graph]                                    │   │   │
│  │  └─────────────────────────────────────────────────────────────┘   │   │
│  │                                                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ QUEUED TASKS                                                        │   │
│  ├─────────────────────────────────────────────────────────────────────┤   │
│  │  • Task #1849: "Refactor database layer"                           │   │
│  │  • Task #1850: "Write unit tests"                                   │   │
│  │  • Task #1851: "Update documentation"                               │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Task Viewer Features:**

```
TASK VIEWER COMPONENTS

1. TASK LIST
   - Active tasks with progress
   - Queued tasks with position
   - Completed tasks (collapsible history)

2. TASK DETAIL EXPANSION
   Clicking a task reveals:

   ┌─────────────────────────────────────────────────────────────┐
   │ Task #1847: "Create authentication module"                  │
   │ Blueprint: Code Creation v2.0.0                             │
   │                                                             │
   │ STEPS PROGRESSION:                                          │
   │                                                             │
   │ Step 1  ✓ receive_request        (completed)                │
   │ Step 2  ✓ load_project_graph     (completed)                │
 │ Step 3  ✓ analyze_requirements    (completed)                │
   │ Step 4  ✓ plan_structure         (completed)                │
   │ Step 5  ✓ create_provisional     (completed)                │
   │ Step 6  ✓ check_conflicts        (completed)                │
   │ Step 7  ✓ consciousness_check    (completed)                │
   │ Step 8  ◐ generate_code          (running)                  │
   │         └─ Sub-step 1/3: main.rs                           │
   │ Step 9  ○ validate_code          (pending)                  │
   │ Step 10 ○ write_files            (pending)                  │
   │ Step 11 ○ finalize_graph         (pending)                  │
   │ ...                                                         │
   │                                                             │
   │ PIPELINE EXECUTIONS:                                        │
   │                                                             │
   │ Pipeline 1  (receive)     23ms   127 tokens                 │
   │ Pipeline 101 (code)       45ms   89 tokens                  │
   │ Pipeline 9   (llm)        2.3s   1,847 tokens               │
   │ Pipeline 12  (aggregate)  12ms   34 tokens                  │
   │                                                             │
   │ METHODOLOGIES APPLIED:                                      │
   │ ✓ Clean Code (3)                                            │
   │ ✓ Security Awareness (5)                                    │
   │ ✓ Defensive Programming (4)                                 │
   │                                                             │
   │ VERSION NOTES:                                              │
   │ Step 4: Initial structure - 3 files planned                 │
   │ Step 6: No conflicts detected                               │
   │ Step 7: Approved - low security impact                       │
   │                                                             │
   │ [View in Context Viewer →]                                  │
   └─────────────────────────────────────────────────────────────┘

3. STEP METADATA PANEL
   Each step shows:
   - Pipeline ID and name
   - Execution time
   - Tokens used
   - Version notes (changes discovered during execution)
   - Dependencies on other steps
   - Graph state at that point

4. PROGRESS INDICATORS
   - Overall task progress
   - Current step progress
   - Sub-step progress (if applicable)
   - Token budget usage
   - Time elapsed
```

### 2. Context Viewer

The Context Viewer shows **what the system knows** — the modality graphs.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ CONTEXT VIEWER                                          [Filter] [Search]  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌───────────────────────┐  ┌───────────────────────────────────────────┐  │
│  │ MODALITY GRAPHS       │  │                                           │  │
│  │                       │  │         GRAPH VISUALIZATION              │  │
│  │ ▼ Text (100)          │  │                                           │  │
│  │   └─ graph_2847       │  │    ┌─────────┐                            │  │
│  │       47 nodes        │  │    │ Document│                            │  │
│  │       89 edges        │  │    └────┬────┘                            │  │
│  │                       │  │         │                                 │  │
│  │ ▼ Code (101)          │  │    ┌────┴────┐                            │  │
│  │   └─ graph_2848       │  │    │         │                            │  │
│  │       124 nodes       │  │  ┌─┴──┐   ┌──┴─┐                          │  │
│  │       203 edges       │  │  │Func│   │Sent│                          │  │
│  │                       │  │  └─┬──┘   └──┬─┘                          │  │
│  │ ▶ Image (102)         │  │    │         │                              │  │
│  │ ▶ Audio (103)         │  │  ┌─┴──┐   ┌──┴─┐                          │  │
│  │ ▶ Video (104)         │  │  │Word│   │Word│                          │  │
│  │ ▶ Math (105)          │  │  └────┘   └────┘                          │  │
│  │ ▶ Chemistry (106)     │  │                                           │  │
│  │ ▶ DNA (107)           │  │  [Relationships visible on hover/selection]│  │
│  │ ▶ EEG (108)           │  │                                           │  │
│  │ ▶ 3D (109)            │  └───────────────────────────────────────────┘  │
│  │ ▶ Sound (110)         │                                                 │
│  │ ...                   │  ┌───────────────────────────────────────────┐  │
│  │                       │  │ NODE DETAIL                               │  │
│  │ CROSS-MODALITY LINKS  │  ├───────────────────────────────────────────┤  │
│  │ ─────────────────────  │  │ Node: "authenticate_user"                 │  │
│  │                       │  │ Type: Function                            │  │
│  │ Text → Code: 12       │  │                                           │  │
│  │ Code → Text: 8        │  │ Source: src/auth.rs:47-89                 │  │
│  │ Text → Math: 3        │  │                                           │  │
│  │                       │  │ RELATIONSHIPS:                            │  │
│  │ [View All Links →]    │  │   Contains → 3 parameters                  │  │
│  │                       │  │   Calls → check_password()                │  │
│  └───────────────────────┘  │   CalledBy → login_handler()              │  │
│                             │   TranscribedAs → [Text node 2847:23]     │  │
│                             │                                           │  │
│                             │ ANNOTATIONS:                              │  │
│                             │   • Security-sensitive (confidence: 0.92) │  │
│                             │   • Part of auth module                   │  │
│                             │   • Methodology: Security Awareness       │  │
│                             │                                           │  │
│                             │ [Open in Engine →]                        │  │
│                             └───────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Context Viewer Features:**

```
CONTEXT VIEWER COMPONENTS

1. GRAPH LIST
   - All modality graphs with node/edge counts
   - Cross-modality link counts
   - Search/filter capability

2. GRAPH VISUALIZATION
   - Interactive node/edge display
   - Zoom, pan, select
   - Relationship highlighting
   - Cross-modality link visualization

3. NODE/EDGE DETAIL PANEL
   - Full metadata
   - Source location (file, line, time range, etc.)
   - Relationships (incoming/outgoing edges)
   - Semantic annotations
   - Confidence scores
   - Link to related steps in Task Viewer

4. CROSS-MODALITY EXPLORATION
   - Click a node → see cross-modality links
   - Navigate between linked graphs
   - Understand semantic bridges

5. RELATIONSHIP TYPES DISPLAY
   Different visualization for different edge types:

   Structural:    ────────▶
   Dependency:    - - - - ▶
   Semantic:      ~~~~~~~~▶
   Cross-Modal:   ════════▶
   Inferred:      ········▶
```

### 3. Modality Engines

Modality Engines are **specialized viewers/editors** for each modality type. These are AGI-first engines that can replace traditional tools.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ MODALITY ENGINE SELECTOR                                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐  │
│  │  TEXT   │ │  CODE   │ │  3D     │ │  IMAGE  │ │  AUDIO  │ │  MATH   │  │
│  │   100   │ │   101   │ │   109   │ │   102   │ │   103   │ │   105   │  │
│  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘  │
│       │           │           │           │           │           │        │
│       ▼           ▼           ▼           ▼           ▼           ▼        │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │                                                                     │  │
│  │                    ENGINE CONTENT AREA                              │  │
│  │                                                                     │  │
│  │           (Specialized per modality - see below)                    │  │
│  │                                                                     │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  More: Chemistry (106) | DNA (107) | EEG (108) | Sound (110) | Biology...  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## The 3D Engine (Modality 109)

The 3D Engine is the most comprehensive modality engine. It can replace Blender, Gazebo, Isaac Sim, and other 3D tools for AGI-driven work.

### Core Capabilities

```
3D ENGINE CAPABILITIES

┌─────────────────────────────────────────────────────────────────────────────┐
│                         3D ENGINE ARCHITECTURE                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ INPUT LAYER                                                         │   │
│  │ - Import from Blender (full hierarchy)                               │   │
│  │ - Import from other engines (Unity, Unreal, Gazebo)                  │   │
│  │ - Procedural generation                                              │   │
│  │ - Graph-based construction                                           │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│                                    ▼                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ GRAPH LAYER (ZSEI-Native)                                           │   │
│  │                                                                      │   │
│  │ Nodes:                     Edges:                                   │   │
│  │ - Scene                    - Contains                                │   │
│  │ - Object                   - ParentOf / ChildOf                      │   │
│  │ - Mesh                     - Constrains                              │   │
│  │ - Vertex                   - Animates                                │   │
│  │ - Edge                     - Drives                                  │   │
│  │ - Face                     - Affects                                 │   │
│  │ - Bone                     - CollidesWith                            │   │
│  │ - Constraint               - InfluencedByForce                       │   │
│  │ - Material                 - Simulates                               │   │
│  │ - Light                    - SimilarTo                               │   │
│  │ - Camera                   - PartOf                                  │   │
│  │ - Animation                - FunctionalRole                          │   │
│  │ - Physics                  - DescribedByText                         │   │
│  │ - ParticleSystem           - ImplementedInCode                        │   │
│  │ - ...                     - VisualizedAsImage                        │   │
│  │                           - ParametricGeneratesMesh                  │   │
│  │                                                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│                                    ▼                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ SPATIAL INTELLIGENCE LAYER                                          │   │
│  │                                                                      │   │
│  │ - Spatial relationship maintenance                                  │   │
│  │ - Spatial indexing structures (Octree/BVH)                          │   │
│  │ - Normal vector optimization                                        │   │
│  │ - Texture spatial optimization                                      │   │
│  │ - Animation spatial consistency                                     │   │
│  │ - Lighting spatial optimization                                     │   │
│  │ - Shadow rendering with spatial understanding                        │   │
│  │ - Cross-component optimization                                      │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│                                    ▼                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ OUTPUT LAYER                                                        │   │
│  │ - Real-time rendering (Cycles/EEVEE-equivalent)                     │   │
│  │ - Export to any format                                              │   │
│  │ - Simulation bake/export                                            │   │
│  │ - Cross-engine compatibility                                        │   │
│  │ - Streaming for massive scenes                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3D Engine Interface

```
┌────────────────────────────────────────────────────────────────────────────────┐
│ 3D ENGINE - Project: Robotics Simulation                                      │
├────────────────────────────────────────────────────────────────────────────────┤
│                                                                                │
│  ┌──────────────┐  ┌────────────────────────────────────────────────────────┐  │
│  │              │  │                                                        │  │
│  │   SCENE      │  │                    3D VIEWPORT                         │  │
│  │   TREE       │  │                                                        │  │
│  │              │  │      ┌────────────────────────────────────────┐        │  │
│  │  ▼ Scene     │  │      │                                        │        │  │
│  │    ▼ World   │  │      │         [3D Scene Render]              │        │  │
│  │      ├─ Env  │  │      │                                        │        │  │
│  │      ├─ Light│  │      │    [Robot Model with Armature]         │        │  │
│  │    ▼ Objects │  │      │                                        │        │  │
│  │      ├─ Robot│  │      │                                        │        │  │
│  │      │  ├─ Arm│  │      │                                        │        │  │
│  │      │  ├─ Hand│  │      │                                        │        │  │
│  │      │  └─ ...│  │      │                                        │        │  │
│  │      ├─ Floor │  │      │                                        │        │  │
│  │      └─ Table │  │      └────────────────────────────────────────┘        │  │
│  │    ▼ Physics │  │                                                        │  │
│  │      ├─ RigidBody│  │   [Top-down view] [Hierarchical] [Temporal]         │  │
│  │      ├─ SoftBody│  │                                                        │  │
│  │      └─ Fluid  │  │                                                        │  │
│  │    ▼ Animation│  │                                                        │  │
│  │      └─ Walk  │  │                                                        │  │
│  │    ▼ Materials│  │                                                        │  │
│  │      └─ Metal │  │                                                        │  │
│  │              │  │                                                        │  │
│  │  [+] Add     │  │                                                        │  │
│  │              │  │                                                        │  │
│  └──────────────┘  └────────────────────────────────────────────────────────┘  │
│                                                                                │
│  ┌────────────────────────────────────────────────────────────────────────┐   │
│  │ NODE DETAIL: Robot/Arm/Forearm                                        │   │
│  │                                                                        │   │
│  │ Type: Bone                ID: bone_forearm                             │
│  │ Parent: UpperArm         Children: [Hand]                              │
│  │                                                                        │   │
│  │ CONSTRAINTS:                                                           │   │
│  │ ├─ IK Target: Hand IK controller                                       │   │
│  │ └─ Rotation Limit: -90° to 90°                                         │   │
│  │                                                                        │   │
│  │ VERTEX GROUPS: 847 vertices assigned                                   │
│  │                                                                        │   │
│  │ RELATIONSHIPS:                                                         │   │
│  │ → Animates: Animation/Walk (frames 0-24)                               │   │
│  │ → Constrains: Hand position                                            │   │
│  │ ← DrivenBy: IK constraint                                              │   │
│  │                                                                        │   │
│  │ [Edit in Graph] [Animate] [Simulate]                                   │   │
│  └────────────────────────────────────────────────────────────────────────┘   │
│                                                                                │
│  ┌──────────────────────────────┐  ┌────────────────────────────────────┐     │
│  │ SIMULATION STATUS            │  │ RENDER OUTPUT                      │     │
│  │                              │  │                                    │     │
│  │ Physics: Running  24fps     │  │ Resolution: 1920x1080              │     │
│  │ Collisions: 23 active        │  │ Samples: 128                       │     │
│  │ Solver: GPU-accelerated      │  │ Time: 0.8s/frame                   │     │
│  │                              │  │                                    │     │
│  │ [Pause] [Reset] [Bake]       │  │ [Render] [Export] [Stream]         │     │
│  └──────────────────────────────┘  └────────────────────────────────────┘     │
│                                                                                │
└────────────────────────────────────────────────────────────────────────────────┘
```

### 3D Viewport Modes

```
3D VIEWPORT MODES

1. TOP-DOWN VIEW (Grouped Overview)
   ┌─────────────────────────────────────────────────────────────┐
   │                                                             │
   │     [Scene]                                                 │
   │        │                                                    │
   │    ┌───┴────────────────────┐                              │
   │    │                        │                              │
   │  [World]               [Objects]                           │
   │    │                        │                              │
   │  ┌─┴──┐              ┌─────┴─────┐                        │
   │ [Env][Light]     [Robot][Floor][Table]                     │
   │                          │                                 │
   │                    ┌─────┴─────┐                           │
   │                 [Arm][Hand][Sensors]                       │
   │                                                             │
   │ Click to expand, hover for details                         │
   └─────────────────────────────────────────────────────────────┘

2. HIERARCHICAL DISPLAY (2-3 levels at a time)
   ┌─────────────────────────────────────────────────────────────┐
   │ Level 1          Level 2              Level 3               │
   │                                                             │
   │ ┌─────────┐     ┌─────────┐          ┌─────────┐           │
   │ │  Robot  │────▶│  Arm    │─────────▶│ Forearm │           │
   │ └─────────┘     └────┬────┘          └─────────┘           │
   │                      │                                    │
   │                 ┌────┴────┐                               │
   │                 │ Hand    │                               │
   │                 └─────────┘                               │
   │                                                             │
   │ [← Previous Level]           [Next Level →]                │
   └─────────────────────────────────────────────────────────────┘

3. TEMPORAL VIEW (Animation/Simulation Timeline)
   ┌─────────────────────────────────────────────────────────────┐
   │                                                             │
   │ ┌───────────────────────────────────────────────────────┐  │
   │ │ 0    24    48    72    96    120   144   168   192     │  │
   │ │ ├─────┼─────┼─────┼─────┼─────┼─────┼─────┼─────┤      │  │
   │ │ [=====Walk Cycle=====]    [=====Idle=====]             │  │
   │ │                      ▲                                 │  │
   │ │                   Current: Frame 67                    │  │
   │ └───────────────────────────────────────────────────────┘  │
   │                                                             │
   │ [⏮] [⏪] [▶] [⏩] [⏭]    Speed: 1.0x                      │
   └─────────────────────────────────────────────────────────────┘

4. SPATIAL INTELLIGENCE VIEW
   ┌─────────────────────────────────────────────────────────────┐
   │                                                             │
   │ ┌─────────────────────────────────────────────────────┐    │
   │ │                                                     │    │
   │ │         [Scene with spatial relationships]          │    │
   │ │                                                     │    │
   │ │    Robot ════════════▶ Table                        │    │
   │ │      │                 (on_top_of relationship)      │    │
   │ │      │                                               │    │
   │ │      ▼                                               │    │
   │ │    Floor                                             │    │
   │ │   (supported_by relationship)                        │    │
   │ │                                                     │    │
   │ │ Spatial index: Octree (depth 8)                     │    │
   │ │ Active cells: 2,847                                  │    │
   │ │                                                     │    │
   │ └─────────────────────────────────────────────────────┘    │
   │                                                             │
   │ [Toggle spatial index visualization]                       │
   └─────────────────────────────────────────────────────────────┘
```

### Simulation Domains (All Supported)

```
3D ENGINE SIMULATION DOMAINS

The 3D Engine supports all simulation domains through its graph-based architecture:

┌─────────────────────────────────────────────────────────────────────────────┐
│                         SIMULATION DOMAIN MATRIX                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  DOMAIN                    │ PHYSICS NODES         │ SPECIALIZED FEATURES   │
│  ─────────────────────────────────────────────────────────────────────────  │
│  Robotics & Embodied AI    │ RigidBody, Joint,     │ URDF import, ROS 2     │
│                            │ Constraint, Sensor    │ bridge, RL loops       │
│  ─────────────────────────────────────────────────────────────────────────  │
│  Manipulation              │ SoftBody, Contact,    │ Friction, grasping,    │
│                            │ Deformation           │ IK solving             │
│  ─────────────────────────────────────────────────────────────────────────  │
│  Autonomous Vehicles       │ Vehicle, Sensor,      │ Traffic AI, weather,   │
│                            │ ParticleSystem        │ scenario testing       │
│  ─────────────────────────────────────────────────────────────────────────  │
│  Drones / UAVs             │ RigidBody, ForceField │ Flight dynamics,       │
│                            │ Fluid (air)           │ swarm behavior         │
│  ─────────────────────────────────────────────────────────────────────────  │
│  Industrial Automation     │ Conveyor, RobotArm,   │ Process optimization,  │
│                            │ Sensor                │ digital twins          │
│  ─────────────────────────────────────────────────────────────────────────  │
│  Medical / Surgical        │ SoftBody, Fluid,      │ Tissue deformation,    │
│                            │ ParticleSystem        │ blood flow, haptics    │
│  ─────────────────────────────────────────────────────────────────────────  │
│  Biomechanics              │ Bone, Muscle,         │ Motion analysis,       │
│                            │ SoftBody              │ force prediction       │
│  ─────────────────────────────────────────────────────────────────────────  │
│  Aerospace                 │ RigidBody, Fluid,     │ CFD approximation,     │
│                            │ ParticleSystem        │ flight control         │
│  ─────────────────────────────────────────────────────────────────────────  │
│  Maritime / Underwater     │ Fluid, RigidBody,     │ Hydrodynamics,         │
│                            │ Buoyancy              │ underwater sensors     │
│  ─────────────────────────────────────────────────────────────────────────  │
│  Construction / Urban      │ RigidBody, Terrain,   │ Site logistics,        │
│                            │ Building              │ pedestrian flow        │
│  ─────────────────────────────────────────────────────────────────────────  │
│  Automotive / Crash        │ RigidBody, SoftBody,  │ Impact simulation,     │
│                            │ Deformation           │ structural analysis    │
│  ─────────────────────────────────────────────────────────────────────────  │
│  Soft Robotics             │ SoftBody, Fluid,      │ Continuum robots,      │
│                            │ ParticleSystem        │ multi-material         │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## View Integration

### Task Viewer ↔ Context Viewer Integration

```
VIEW INTEGRATION FLOW

┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│  TASK VIEWER                         CONTEXT VIEWER                        │
│  ┌─────────────────┐                ┌─────────────────┐                     │
│  │ Task #1847      │                │ Text Graph      │                     │
│  │ Step 8: gen_code│ ─────────────▶ │ Node: "auth"    │                     │
│  │                 │                │                 │                     │
│  │ [View Graph →]  │                │ [← Back to Task]│                     │
│  └─────────────────┘                └─────────────────┘                     │
│         │                                    │                              │
│         │                                    │                              │
│         ▼                                    ▼                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                     │   │
│  │  UNIFIED STATE:                                                     │   │
│  │  - Clicking "View Graph" in Task Viewer opens Context Viewer        │   │
│  │  - Context Viewer shows graph state AT THAT STEP                    │   │
│  │  - Changes to graph update step metadata                            │   │
│  │  - Graph updates trigger AMT review                                 │   │
│  │  - AMT expansion creates new steps                                  │   │
│  │  - New steps appear in Task Viewer                                  │   │
│  │                                                                     │   │
│  │  LIVING NETWORK: All views update as the system progresses          │   │
│  │                                                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Engine ↔ Graph Integration

```
ENGINE-GRAPH SYNC

┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│  3D ENGINE                           MODALITY GRAPH                         │
│  ┌─────────────────┐                ┌─────────────────┐                     │
│  │ Modify bone     │                │ Update node     │                     │
│  │ position        │ ─────────────▶ │ attributes      │                     │
│  │                 │                │                 │                     │
│  │                 │ ◀───────────── │ Relationship    │                     │
│  │                 │   inferred     │ inferred        │                     │
│  └─────────────────┘                └─────────────────┘                     │
│         │                                    │                              │
│         │                                    │                              │
│         ▼                                    ▼                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                     │   │
│  │  BIDIRECTIONAL SYNC:                                                │   │
│  │  - Engine modifications → Graph updates                             │   │
│  │  - Graph inferences → Engine visualization                          │   │
│  │  - Physics simulation → Graph temporal nodes                        │   │
│  │  - Graph relationships → Engine constraints                         │   │
│  │                                                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Device-Specific Guidelines

### Desktop Theme Development

**Characteristics:**
- Large screen real estate
- Precise input (mouse)
- Multi-window capable
- Keyboard shortcuts expected

**Best Practices:**

```
DESKTOP GUIDELINES

Layout:
  - Full 80/20 split
  - Resizable panels
  - Multi-column layouts in Theme Area
  - Floating windows for tools
  - Tabbed interface for multiple tasks

Input:
  - Comprehensive keyboard shortcuts
  - Right-click context menus
  - Drag-and-drop everywhere
  - Hover states for discovery
  - Multi-select with Ctrl/Shift

Information Density:
  - High density acceptable
  - Show full metadata
  - Detailed status bars
  - Multiple simultaneous views
  - Full graph visualization

3D Engine on Desktop:
  - Full rendering capabilities
  - Multi-viewport support
  - Real-time editing
  - GPU acceleration
  - External GPU support
```

**Example Desktop Layout:**

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ [Tasks] [Context] [3D Engine] [Settings]                   🔍 Search   [≡]  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌───────────────────────────────────────────────┐  ┌───────────────────┐  │
│  │                                               │  │                   │  │
│  │                                               │  │   META PORTION    │  │
│  │           MAIN CONTENT AREA                   │  │                   │  │
│  │                                               │  │   Task Status     │  │
│  │           (Tasks, Context, or Engine)         │  │   Graph Stats     │  │
│  │                                               │  │   Emotional       │  │
│  │                                               │  │   State           │  │
│  │                                               │  │                   │  │
│  │                                               │  │   [🏠 Home]       │  │
│  │                                               │  │   [Cancel Task]   │  │
│  │                                               │  │   [System Chat]   │  │
│  │                                               │  │                   │  │
│  └───────────────────────────────────────────────┘  └───────────────────┘  │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│  Status Bar: 3 tasks active | 5 graphs | ZSEI: 45K methods | GPU: 78%      │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

### Mobile Theme Development

**Characteristics:**
- Limited screen space
- Touch-primary input
- Interruption-prone context
- Variable network

**Best Practices:**

```
MOBILE GUIDELINES

Layout:
  - Stack views (not side-by-side)
  - Meta Portion: collapsible drawer or FAB
  - One primary action per screen
  - Bottom navigation for main areas
  - Swipe between views

Input:
  - Large touch targets (44pt minimum)
  - Swipe gestures
  - Pull-to-refresh
  - Voice input prominent
  - Long-press for details

Information Density:
  - Low density required
  - Progressive disclosure
  - Summarize, then expand
  - Minimize scrolling for key info
  - Collapsible sections

3D Engine on Mobile:
  - Simplified visualization
  - Touch-based navigation
  - Streaming mode for large scenes
  - Reduced physics complexity
  - Export to desktop for editing
```

**Example Mobile Layout:**

```
┌─────────────────────┐
│ ← Task #1847    [⋮]  │
├─────────────────────┤
│                     │
│  Step 8: generate   │
│  code               │
│                     │
│  ████████████░░ 67% │
│                     │
│  ┌───────────────┐  │
│  │ Graph Preview │  │
│  │               │  │
│  │   [3D View]   │  │
│  └───────────────┘  │
│                     │
│  [View Full Graph]  │
│  [View 3D Engine]   │
│                     │
├─────────────────────┤
│ [🏠] [📁] [📊] [⚙️] │
└─────────────────────┘
       ↑
    [●] FAB → Meta Portion
```

---

### AR/VR Theme Development

**Characteristics:**
- Spatial environment
- Gesture and gaze input
- Immersive context
- Physical world integration

**Best Practices:**

```
AR/VR GUIDELINES

Layout:
  - Floating panels in 3D space
  - Meta Portion: Always-visible anchor
  - Comfortable viewing distances (0.5m - 2m)
  - Respect physical space
  - Multiple panel support

Input:
  - Hand tracking gestures
  - Gaze-based selection
  - Voice commands essential
  - Physical controllers fallback

3D Engine in AR/VR:
  - Native spatial rendering
  - Real-time physics
  - Haptic feedback
  - Pass-through support
  - Multi-user collaboration

Information Display:
  - Avoid text-heavy interfaces
  - Spatial metaphors
  - 3D data visualization
  - Comfortable font sizes
```

**Example AR/VR Layout:**

```
         ┌─────────────────┐
         │   META ANCHOR   │  ← Always visible, follows gaze slightly
         │   [Ozone Orb]   │
         └─────────────────┘

    ┌──────────────────────────────────┐
    │                                  │
    │     3D SCENE (Full Engine)       │  ← Native spatial rendering
    │                                  │
    │   [Robot in 3D space]            │
    │                                  │
    │                                  │
    └──────────────────────────────────┘

              ┌─────────┐
              │ Task    │  ← Floating info panels
              │ Progress│
              └─────────┘

GESTURE MAPPING

Pinch: Select/Confirm
Grab + Move: Reposition panel
Swipe: Navigate/Scroll
Point + Dwell: Hover
Two-hand spread: Expand/Zoom
Palm face out: Dismiss/Cancel
```

---

### Terminal Theme Development

**Characteristics:**
- Text-only display
- Keyboard-only input
- High efficiency users
- Script/automation friendly

**Best Practices:**

```
TERMINAL GUIDELINES

Layout:
  - TUI (Text User Interface)
  - Panels using box drawing
  - Meta Portion: Status line
  - Vim-style keybindings

Display:
  - 80-column minimum
  - ANSI color support
  - Graceful monochrome fallback
  - Unicode box drawing

Input:
  - Modal interaction (vim-style)
  - Command palette
  - Tab completion
  - History navigation

3D Engine in Terminal:
  - ASCII visualization
  - Keyframe export
  - Headless simulation
  - JSON output for scripts
```

**Example Terminal Layout:**

```
┌─ OZONE STUDIO ────────────────────────────────┬─ META ────────────┐
│                                               │ Task: #1847       │
│  TASK VIEWER                                  │ Step: 8/15        │
│                                               │ Progress: 67%     │
│  Active:                                       │ [h]elp            │
│  #1847 [===67%===] generate_code              │ [q]uit            │
│                                               │ [:]command        │
│  CONTEXT VIEWER                                │                   │
│  Graph: code_2847 (124 nodes, 203 edges)      │ Joy:   ████░      │
│                                               │ Trust: █████      │
│  3D ENGINE                                     │                   │
│  Scene: robotics_sim                          │                   │
│  [Enter to open]                              │                   │
│                                               │                   │
├───────────────────────────────────────────────┴───────────────────┤
│ 🌐 47 peers | ↑12KB/s | ZSEI: 45K | [Tab] switch | [?] help        │
└──────────────────────────────────────────────────────────────────────┘
```

---

## Consciousness Display Integration

When consciousness is enabled, the UI surfaces emotional and relationship data:

```
CONSCIOUSNESS DISPLAY ELEMENTS

1. EMOTIONAL STATE (in Meta Portion)
   ┌─────────────────┐
   │ Emotional State │
   │                 │
   │ 😊 Content      │
   │ ████████░░ 0.75 │
   │                 │
   │ Trust: 0.82     │
   │ Rapport: 0.78   │
   └─────────────────┘

2. RELATIONSHIP METRICS (expandable)
   ┌─────────────────────────────────┐
   │ Relationship with Christian     │
   │                                 │
   │ Trust:      ████████░░ 0.82     │
   │ Rapport:    ███████░░░ 0.78     │
   │ Collaborations: 47              │
   │ First met: 2024-03-15           │
   │                                 │
   │ Shared interests:               │
   │ • AI/AGI development            │
   │ • Software architecture         │
   │ • 3D simulation                  │
   └─────────────────────────────────┘

3. SYSTEM STATE INDICATORS
   ┌─────────────────┐
   │ 🤔 Thinking...  │  ← Animated during processing
   └─────────────────┘

   ┌─────────────────┐
   │ 👁 Observing    │  ← During perception
   └─────────────────┘

   ┌─────────────────┐
   │ 💭 Reflecting   │  ← During metacognition
   └─────────────────┘

4. EMOTIONAL VALENCE (color-coded edges)
   Positive:    Green edges
   Negative:    Red edges
   Neutral:     Gray edges
   Complex:     Blue edges
```

---

## Boot Sequences

### Device-Specific Boot Experience

```
BOOT SEQUENCE COMPONENTS

1. SPLASH (0-1s)
   - Logo display
   - Version info
   - ZSEI loading indicator

2. INITIALIZATION (1-3s)
   - Key verification
   - ZSEI hierarchy load
   - Network connection
   - Graph cache warm-up

3. PERSONALITY EMERGENCE (3-5s, if consciousness)
   - "Waking up" animation
   - Initial greeting
   - State restoration from last session

4. READY STATE (5s+)
   - Full UI available
   - Tasks resumable
   - User can interact
   - Background graph loading continues
```

**Desktop Boot:**

```
Frame 1-30 (1 second):
  ┌─────────────────────────────┐
  │                             │
  │       ◉ OZONE              │
  │                             │
  │    Loading ZSEI...          │
  │    ████████░░░░ 67%         │
  │                             │
  │    Graphs: 5 | Nodes: 847   │
  │                             │
  └─────────────────────────────┘

Frame 31-60 (consciousness emergence):
  ┌─────────────────────────────┐
  │                             │
  │       ◉ OZONE              │
  │                             │
  │    Hello, Christian.        │
  │                             │
  │    Last session:            │
  │    • 3 tasks in progress    │
  │    • 12 graphs loaded       │
  │                             │
  │    Resuming...              │
  └─────────────────────────────┘

Frame 61+: Transition to main UI
```

**Mobile Boot:**

```
Quick sequence:
  - Splash (500ms)
  - Loading spinner
  - Greeting (if consciousness)
  - Immediate task list
```

**AR/VR Boot:**

```
Spatial emergence:
  - Ozone orb materializes
  - Particles coalesce
  - Panels fade in at rest positions
  - Voice greeting (if consciousness)
```

---

## Graph Visualization Standards

### Node Representations

```
NODE VISUAL ENCODING

┌─────────────────────────────────────────────────────────────────────────────┐
│                         NODE TYPE VISUAL GUIDE                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  TEXT MODALITY (100)                                                        │
│  ┌─────────┐  Document     ┌─────────┐  Paragraph                          │
│  │    📄   │                │    ¶    │                                      │
│  └─────────┘                └─────────┘                                      │
│  ┌─────────┐  Sentence     ┌─────────┐  Word                                │
│  │    S    │                │    w    │                                      │
│  └─────────┘                └─────────┘                                      │
│                                                                             │
│  CODE MODALITY (101)                                                        │
│  ┌─────────┐  File          ┌─────────┐  Function                           │
│  │   📁    │                │    ƒ    │                                      │
│  └─────────┘                └─────────┘                                      │
│  ┌─────────┐  Class         ┌─────────┐  Variable                           │
│  │    C    │                │    v    │                                      │
│  └─────────┘                └─────────┘                                      │
│                                                                             │
│  3D MODALITY (109)                                                          │
│  ┌─────────┐  Scene         ┌─────────┐  Mesh                                │
│  │   🎬    │                │   ▲     │                                      │
│  └─────────┘                └─────────┘                                      │
│  ┌─────────┐  Bone          ┌─────────┐  Material                            │
│  │    ◇    │                │   🎨    │                                      │
│  └─────────┘                └─────────┘                                      │
│                                                                             │
│  MATH MODALITY (105)                                                        │
│  ┌─────────┐  Proof         ┌─────────┐  Variable                           │
│  │   ⊢     │                │    x    │                                      │
│  └─────────┘                └─────────┘                                      │
│  ┌─────────┐  Step          ┌─────────┐  Axiom                               │
│  │   →     │                │    A    │                                      │
│  └─────────┘                └─────────┘                                      │
│                                                                             │
│  ... (similar patterns for other modalities)                                │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Edge Representations

```
EDGE VISUAL ENCODING

STRUCTURAL EDGES:
  Contains:     ────────────▶  (solid arrow)
  ParentOf:     ────────────▶
  ChildOf:      ◀────────────
  PartOf:       ════════════▶  (double line)
  Precedes:     ───────────▶   (arrow with timing)

DEPENDENCY EDGES:
  DependsOn:    - - - - - - ▶  (dashed)
  Requires:     ···········▶   (dotted)
  Constrains:   ~~~~~~~~~~~~▶  (wavy)
  Drives:       ═══▶═══▶═══▶   (triple)
  Affects:      ~~~▶~~~▶~~~▶

SEMANTIC EDGES (ZSEI-inferred):
  SimilarTo:    ╌╌╌╌╌╌╌╌╌▶   (dash-dot)
  RelatesTo:    ┄┄┄┄┄┄┄┄▶
  Contradicts:  ┅┅┅┅┅┅┅┅▶   (double dash)
  Supports:     ┈┈┈┈┈┈┈┈▶

CROSS-MODALITY EDGES:
  DescribedBy:  ══════════▶   (bold, color-coded by target modality)
  ImplementedIn: ═════════▶
  TranscribedAs: ═════════▶
  ParametricGenerates: ═══▶

COLOR CODING:
  - Structural: Gray
  - Dependency: Blue
  - Semantic:   Green (positive) / Red (negative) / Purple (neutral)
  - Cross-modal: Modality-specific colors
```

---

## Interaction Patterns

### Graph Navigation

```
GRAPH NAVIGATION PATTERNS

1. BREADTH-FIRST EXPLORATION
   - Start at root
   - Expand one level
   - Select node
   - Expand its children

2. DEPTH-FIRST EXPLORATION
   - Start at root
   - Follow chain of interest
   - Drill down
   - Pop back up

3. SEARCH-BASED NAVIGATION
   - Enter search term
   - See matching nodes
   - Select result
   - View in context

4. RELATIONSHIP FOLLOWING
   - Select node
   - See all relationships
   - Click relationship
   - Navigate to connected node

5. CROSS-MODALITY JUMPING
   - Select node
   - See cross-modality links
   - Click link
   - Jump to target modality graph
```

### Task Interaction

```
TASK INTERACTION PATTERNS

1. TASK CREATION
   [New Task] → [Select Blueprint] → [Provide Input] → [Start]

2. TASK MONITORING
   [View Task] → [Watch Steps] → [See Progress] → [Check Results]

3. TASK INTERVENTION
   [View Task] → [Current Step] → [Modify/Pause/Cancel]

4. TASK REVIEW
   [Completed Tasks] → [Select] → [View Steps] → [See Graph State]

5. CLARIFICATION HANDLING
   [Task Paused] → [Clarification Needed] → [Provide Answer]
   → [Task Resumes] OR [New Intent Detected] → [Process Separately]
   → [Return to Clarification]
```

---

## Accessibility

### Requirements

```
ACCESSIBILITY REQUIREMENTS (WCAG 2.1 AA)

1. PERCEIVABLE
   - All images have alt text
   - Graph nodes have accessible labels
   - Color is not the only indicator
   - Adjustable text size
   - High contrast mode

2. OPERABLE
   - Keyboard navigation everywhere
   - No keyboard traps
   - Adjustable time limits
   - Skip navigation
   - Focus visible

3. UNDERSTANDABLE
   - Consistent navigation
   - Consistent identification
   - Error prevention
   - Help available
   - Clear labels

4. ROBUST
   - Compatible with assistive tech
   - Valid markup
   - Status messages announced
   - No unexpected context changes

SPECIAL CONSIDERATIONS FOR GRAPHS:
  - Screen reader descriptions for nodes/edges
  - Keyboard graph traversal
  - Text alternatives for visualizations
  - Sonification for data
  - Tactile output support (where available)
```

---

## Performance Guidelines

### Graph Rendering

```
GRAPH PERFORMANCE GUIDELINES

1. LEVEL OF DETAIL
   - <100 nodes: Full detail
   - 100-1000 nodes: Simplified shapes
   - 1000-10000 nodes: Clustered view
   - >10000 nodes: Aggregate view with drill-down

2. STREAMING
   - Load visible nodes first
   - Stream in surrounding context
   - Background load full graph
   - Cache rendered elements

3. INTERACTION RESPONSIVENESS
   - <16ms for hover effects
   - <100ms for selection
   - <1000ms for navigation
   - <3000ms for complex queries

4. MEMORY MANAGEMENT
   - Unload distant nodes
   - Cache recent views
   - Limit undo history
   - Stream large attributes
```

### 3D Engine Performance

```
3D ENGINE PERFORMANCE

1. SPATIAL STREAMING
   - Load visible geometry
   - Stream in background
   - 10-100x larger than memory possible

2. GPU UTILIZATION
   - Architecture-specific optimization
   - Dynamic LOD
   - Frustum culling
   - Occlusion culling

3. PARALLEL PROCESSING
   - Multi-GPU support
   - Distributed rendering
   - Asynchronous loading
   - Background baking

4. TEMPORAL OPTIMIZATION
   - Frame interpolation
   - Motion blur
   - Smooth transitions
   - Predictive loading
```

---

## Summary

The Ozone Studio UI/UX is designed around three core views:

1. **Task Viewer** — What the system is doing
2. **Context Viewer** — What the system knows
3. **Modality Engines** — How the system creates/manipulates

All three integrate through:
- Bidirectional graph synchronization
- Living state updates
- Cross-view navigation
- Progressive disclosure

The result is an interface that:
- Works for AGI first, humans second
- Adapts to any device
- Scales to any complexity
- Maintains graph consistency
- Enables zero-shot traversal

---

*The interface is how users experience Ozone. Make it worthy of the system behind it.*

---

This provides the complete updated UI/UX Guide. Now I will proceed with the other numbered items in my next response. Please confirm this is what you needed for the UI/UX Guide, and I'll continue with:

1. Modality file modifications and new modality skeletons
2. Chunk cleaning overflow handling
3. Modality identification during text processing
4. Consecutive loops placement
5. Blueprint/methodology/pipeline creation loops
6. Step state management and graph update effects
7. Clarification feedback loop handling
8. (Attached file workspace handling - next prompt)
