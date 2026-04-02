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

# OZONE STUDIO — UI/UX DEVELOPMENT GUIDE
## Version 2.0.0 — AGI-First Interface Design

---

## PHILOSOPHY: AGI-First, Human-Second

Ozone Studio is not a human-first productivity app wearing AI clothing. It is an AGI operating system that humans inhabit. Every interface decision flows from this principle:

- **The AGI is always working.** The UI must show a living, breathing intelligence — not a chat box that waits.
- **Context is spatial, not flat.** Information exists in a graph fabric, not a list. The UI must reflect that.
- **Progress is a web, not a line.** Tasks branch, loop back, spawn children, and converge. Visualization must honor that topology.
- **Every modality is a universe.** Text, Code, 3D, EEG, DNA — each has its own grammar. The interface must surface that grammar.

The closest real-world aesthetic reference: the Stark Tech holographic UI from Iron Man / Spider-Man: Far From Home — spatially layered, gesture-driven, information-dense but navigable, with an intelligence behind every panel that is actively organizing and synthesizing. Beck's illusion workstation multi-tiered timeline. Fury's point-cloud tactical hologram. These are the right emotional register.

---

## CORE LAYOUT ARCHITECTURE (All Platforms)

```
┌────────────────────────────────────────────────────────────────────┐
│                                              ┌─────────────────────┐│
│                                              │                     ││
│            THEME AREA (80%)                  │    META PORTION     ││
│                                              │       (20%)         ││
│   ┌──────────────────────────────────────┐   │  Always Accessible  ││
│   │                                      │   │                     ││
│   │         ACTIVE VIEW                  │   │  System Chat        ││
│   │   (Workspace / Task / Context /      │   │  Task Status        ││
│   │    Graph / Modality Engine)          │   │  Emotional State    ││
│   │                                      │   │  Quick Nav          ││
│   └──────────────────────────────────────┘   │                     ││
│                                              │  [🏠 Home]          ││
│                                              │  [✕ Cancel]         ││
│                                              └─────────────────────┘│
├────────────────────────────────────────────────────────────────────┤
│  CONNECTION BAR — Peers | Bandwidth | Score | ZSEI depth | Status  │
└────────────────────────────────────────────────────────────────────┘
```

**Inviolable rule:** The Meta Portion is NEVER obscured. It is the user's lifeline — home, cancel, system state. No modal, no overlay, no fullscreen graph may block it.

---

## NAVIGATION TAXONOMY

```
PRIMARY NAVIGATION (Theme Area tabs/modes)

  [Workspace]    — Projects, files, chat per project
  [Tasks]        — Task Viewer: live/queued/completed tasks
  [Context]      — Context Viewer: all modality graphs
  [Engines]      — Modality Engine interfaces (3D, etc.)
  [Library]      — Methodologies, Blueprints, Pipelines
  [Settings]     — Configuration

SECONDARY NAVIGATION (within each view)
  Breadcrumb trail + back navigation
  Cross-links between Task ↔ Context ↔ Engine views
```

---

## THE TASK VIEWER

### What It Is

The Task Viewer is the operational nerve center — a real-time window into everything Ozone is doing. It is not a log. It is a living display of a branching execution graph.

### Task States (Full Lifecycle)

```
TASK STATE MACHINE

  Created ──► Queued ──► Processing ──► ┬──► Completed
                                         ├──► Waiting (dep)
                                         ├──► Clarifying (human needed)
                                         └──► Failed

STEP STATE MACHINE (within a task)

  Planned ──► Active ──► ┬──► Stage_N_Complete
                          │         │
                          │         ▼
                          │    (loop back if graph updated)
                          │
                          └──► Completed (all stages done)

GRAPH STATE MACHINE (per modality graph)

  Initial ──► Building ──► ┬──► Enriched
                            ├──► Linked (cross-modality)
                            └──► Updated (from step progress)
```

### Task Viewer Layout — Desktop

```
┌─────────────────────────────────────────────────────────────────────┐
│  TASKS                        [Active: 3]  [Queued: 7]  [Done: 42] │
├──────────────┬──────────────────────────────────────────────────────┤
│              │                                                       │
│  TASK LIST   │              TASK DETAIL                             │
│              │                                                       │
│  ● Task #47  │  ┌─ Task #47: "Analyze protein folding dataset" ────┐│
│    Running   │  │                                                   ││
│    ──────    │  │  Blueprint: Scientific Analysis v2.1             ││
│  ○ Task #46  │  │  Started: 2m ago  |  Est: 8m remaining           ││
│    Waiting   │  │                                                   ││
│    ──────    │  │  PROGRESS ████████████████░░░░░░░░░░ 58%         ││
│  ○ Task #45  │  │                                                   ││
│    Queued    │  │  ─────────── BLUEPRINT STEPS ──────────────────  ││
│    ──────    │  │                                                   ││
│  ✓ Task #44  │  │  [✓] Step 1  Receive & Clean Request        0.3s ││
│    Done      │  │  [✓] Step 2  Build AMT (4 loops)            1.2s ││
│    ──────    │  │  [✓] Step 3  Aggregate Methodologies        0.8s ││
│  ✗ Task #43  │  │  [●] Step 4  Build Modality Graphs          ···  ││
│    Failed    │  │      ├─ [✓] Text Graph       nodes:47            ││
│              │  │      ├─ [●] DNA Graph         nodes:12 ···       ││
│              │  │      └─ [○] Chemistry Graph   pending            ││
│              │  │  [○] Step 5  Order Blueprint Steps         —      ││
│              │  │  [○] Step 6  Execute Steps                 —      ││
│              │  │                                                   ││
│              │  │  ─────────── ACTIVE PIPELINE ─────────────────  ││
│              │  │  Pipeline #107 (DNA Analysis)  [●] Running       ││
│              │  │  Tokens: 847 used                                 ││
│              │  │  [View in Context →]  [View Graph →]             ││
│              │  └───────────────────────────────────────────────────┘│
│              │                                                       │
│              │  ┌─ Step Metadata ───────────────────────────────────┐│
│              │  │  Step 4 › DNA Graph › Stage 2: Relationship Inf.  ││
│              │  │  Nodes created: 12  |  Edges: 34                  ││
│              │  │  Methodologies applied: [16] Scientific Data      ││
│              │  │  Versioning note: "Initial gene feature extract"  ││
│              │  └───────────────────────────────────────────────────┘│
└──────────────┴──────────────────────────────────────────────────────┘
```

### Step Detail Panel

Each step expands to show its full living state:

```
STEP DETAIL (expanded)

  Step 4: Build Modality Graphs
  ─────────────────────────────────────────────────────
  State History:
    [✓] Stage 1  Initial node extraction          1.1s
    [●] Stage 2  Relationship inference           ···
    [○] Stage 3  Cross-modality linking           —
    [○] Stage 4  Graph enrichment (ZSEI hooks)    —

  Context Snapshot:
    AMT branch: "Protein Structure Analysis"
    Methodologies: [14 Math Rigor] [16 Sci Data] [9 Data Analysis]
    Graphs feeding this step: text_graph_#47_v1

  Dependency Status:
    Step 3 (Methodologies): ✓ Complete
    Step 5 (Blueprint Order): ○ Waiting on this step

  Version Notes:
    v1.0 — "Gene annotations extracted from FASTA input"
    v1.1 — "Added regulatory edges after AMT expansion (loop 3)"

  [Jump to AMT Branch →]   [View DNA Graph →]   [Cancel Step]
```

### Task Viewer — Mobile

```
┌───────────────────────┐
│ ← Tasks          [+]  │
├───────────────────────┤
│ ● #47 Running   58%   │
│   Protein analysis    │
│   Step 4/6  ► DNA Gr. │
├───────────────────────┤
│ ○ #46 Waiting         │
│   Dep: #45 step 3     │
├───────────────────────┤
│ ○ #45 Queued          │
├───────────────────────┤
│ ✓ #44 Done  2m ago    │
├───────────────────────┤
│ [Active: 3] [Q: 7]    │
└───────────────────────┘
Tap task → full detail sheet
```

---

## THE CONTEXT VIEWER

### What It Is

The Context Viewer is OZONE's spatial intelligence interface — a navigable, interactive representation of all modality graphs. This is where context lives. Not as text. As a graph fabric that you can traverse, zoom, filter, and cross-link.

```
CONTEXT VIEWER MODES

  [Graph View]       — Interactive node-edge graph
  [Fabric View]      — Cross-modality spatial map
  [Timeline View]    — Temporal context progression
  [Hierarchy View]   — AMT tree aligned with graphs
```

### Graph View — Desktop (Primary Mode)

```
┌─────────────────────────────────────────────────────────────────────┐
│  CONTEXT: Task #47  ─  Protein Folding Analysis                     │
│  ┌─── MODALITY FILTER ──────────────────────────────────────────┐   │
│  │ [●Text] [●DNA] [○Chemistry] [○Math] [○Code] [+ Link Lines]  │   │
│  └──────────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│                    ┌────────────────────────────────────────────┐    │
│                    │                                            │    │
│    [TEXT GRAPH]    │         MAIN CANVAS                       │    │
│    47 nodes        │                                            │    │
│    ┌──────┐        │   ◉ Protein_Structure        ◈ ATGC_Seq  │    │
│    │ Doc  │        │     │  Describes              │           │    │
│    │ root │─────── │     ▼                         ▼           │    │
│    └──┬───┘        │   ◈ Gene_BRCA1    ──Codes──► ◆ Protein   │    │
│       │            │     │ RegulatedBy             │ FoldsInto │    │
│    Entities        │     ▼                         ▼           │    │
│    Topics          │   ◈ Promoter      ◉ "BRCA1  ◆ Domain_1   │    │
│    Keywords        │     region"         activates            │    │
│                    │                     tumor                │    │
│    [DNA GRAPH]     │                     suppression"         │    │
│    12 nodes        │                                            │    │
│    ┌──────┐        │   ── Cross-modality link (Text→DNA) ──    │    │
│    │ Seq  │        │                                            │    │
│    │ root │        └────────────────────────────────────────────┘    │
│    └──────┘                                                          │
│                                                                      │
│  ─── SELECTED NODE ──────────────────────────────────────────────   │
│  ◈ Gene_BRCA1  (DNA Graph)                                          │
│  Type: Gene  |  Position: 17q21  |  Strand: Forward                │
│  Edges: [RegulatedBy Promoter] [CodesFor Protein_BRCA1]             │
│  Cross-links: [←Text: "BRCA1 activates..."] [→Chemistry: ligand]   │
│  AMT Branch: "Genetic Regulation" › Step 4                          │
│  [Expand Node]  [Trace Path →]  [View in Task]                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Graph View Interaction Design

```
INTERACTION MODEL

  ZOOM
    Scroll wheel / pinch — zoom in/out on canvas
    Level 1 (far):    Modality clusters as glowing orbs
    Level 2 (mid):    Individual nodes visible, edges as lines
    Level 3 (near):   Full node metadata, edge labels
    Level 4 (focus):  Single node expanded, all edges shown

  SELECT
    Click node        — select, show detail panel
    Click edge        — show relationship metadata
    Hover             — tooltip with quick info

  NAVIGATE
    Drag canvas       — pan
    Click cluster     — zoom to modality subgraph
    Breadcrumb        — track traversal history

  FILTER
    Modality toggles  — show/hide graph layers
    Edge type filter  — structural / semantic / cross-modal
    Step filter       — only show nodes from step N
    Time filter       — graph state at time T

  HIGHLIGHT
    Select AMT branch — lights up all related nodes across ALL modalities
    Select step       — dims everything not generated by that step
    Select methodology — highlights nodes where it was applied
```

### Fabric View — Cross-Modality Spatial Map

The Fabric View is the top-down spatial intelligence display — inspired by Fury's point-cloud tactical hologram. All modality graphs visible simultaneously, connected by cross-modal relationship threads.

```
FABRIC VIEW

┌─────────────────────────────────────────────────────────────────────┐
│  ┌─ FABRIC — Task #47 ─────────────────────────────────── [3D] ─┐  │
│  │                                                               │  │
│  │     ╔══════════╗              ╔══════════╗                    │  │
│  │     ║  TEXT    ║              ║   DNA    ║                    │  │
│  │     ║  GRAPH   ║══ Describes ►║  GRAPH   ║                    │  │
│  │     ║  47 nodes║◄══ AnnotatedBy ══════════                    │  │
│  │     ╚═════╤════╝              ╚═══╤══════╝                    │  │
│  │           │                       │                           │  │
│  │     References              CodesFor                         │  │
│  │           │                       │                           │  │
│  │     ╔═════▼════╗              ╔═══▼══════╗                    │  │
│  │     ║   MATH   ║              ║CHEMISTRY ║                    │  │
│  │     ║  GRAPH   ║══ Proves ════║  GRAPH   ║                    │  │
│  │     ║  8 nodes ║              ║ 15 nodes ║                    │  │
│  │     ╚══════════╝              ╚══════════╝                    │  │
│  │                                                               │  │
│  │   Edge density:  ████ Text↔DNA  ██ DNA↔Chem  █ Math↔Chem    │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                      │
│  Click any cluster to enter Graph View for that modality            │
│  Click any edge line to see the cross-modal relationship chain      │
└─────────────────────────────────────────────────────────────────────┘
```

### 3D Fabric View (AR/VR / Desktop with 3D toggle)

When 3D mode is enabled, the Fabric View becomes a true spatial holographic display — modality clusters as floating orbs in 3D space, cross-modal threads as light beams between them, depth used to represent relationship confidence (closer = stronger). Navigated by mouse drag + scroll or AR/VR gesture.

```
3D SPATIAL FABRIC (conceptual layout)

            ◉ TEXT                 ← Front center (primary modality)
           /  |  \
          /   |   \
    ◈ CODE  ◈ DNA  ◈ MATH        ← Mid tier, orbiting text
        \     |     /
         \    |    /
          ◈ CHEM ◈ 3D            ← Deep tier, specialized
              |
           ◈ EEG                 ← Deepest, most specialized

  Cross-modal edges: glowing threads, pulsing when data flows
  Node size: proportional to node count in graph
  Node glow intensity: proportional to step activity
  Edge thickness: proportional to relationship count
```

### Hierarchy View — AMT ↔ Graph Alignment

Shows 2–3 levels of the AMT tree simultaneously, with each branch linked to the graph nodes that support or were derived from it.

```
HIERARCHY VIEW

  AMT TREE                    LINKED GRAPH NODES
  ────────────────────────    ──────────────────────────────────
  ► Analyze Protein Data
    │
    ├─► [intent] Understand         ◈ Gene_BRCA1 (DNA)
    │   genetic regulation          ◈ "BRCA1 activates..." (Text)
    │   [methodology 16]            ◈ Promoter_region (DNA)
    │
    ├─► [intent] Identify           ◆ Protein_BRCA1 (Chemistry)
    │   protein structure           ◆ Domain_1 (Chemistry)
    │   [methodology 14]            ◆ Folding_step_3 (Math)
    │
    └─► [intent] Cross-reference    ↔ Text→DNA (DescribedBy)
        modalities                  ↔ DNA→Chemistry (CodesFor)
        [methodology 9]             ↔ Chemistry→Math (ProvedUsing)

  [Expand Branch]  [Collapse]  [Jump to Graph Node →]
```

---

## MODALITY ENGINES

### What Is a Modality Engine?

Certain modalities produce data of such richness and structural complexity that they warrant a dedicated rendering and interaction environment — not just a graph viewer, but a full application layer built specifically for that modality's nature.

A Modality Engine is an AGI-first application environment embedded within OZONE Studio. It is made *for* the AGI to work with and *for* the human to observe, navigate, and interact with the AGI's work product in its native medium.

```
MODALITY ENGINE TRIGGER CONDITIONS

  A modality gets a dedicated engine when it meets TWO or more of:

  1. The data has spatial/temporal/structural depth requiring
     interactive rendering (not just text or 2D graph)
  2. The modality produces artifacts the human needs to inspect
     in their native form (3D scene, circuit diagram, genome map)
  3. The AGI actively works on the artifact (not just analyzes it)
  4. Domain experts have established interactive tools for this
     modality that define user expectations (Blender for 3D,
     MNE for EEG, IGV for genomics, etc.)

MODALITIES WITH DEDICATED ENGINES
  3D (109)          ─── 3D Scene Engine
  EEG/BCI (108/119) ─── Neural Signal Engine
  DNA/Genomics (107)─── Genome Browser Engine
  Chemistry (106)   ─── Molecular Viewer Engine
  Math (105)        ─── Proof/Formula Engine
  Code (101)        ─── Code Intelligence Engine (file tree + graph)
  Parametric (120)  ─── CAD/Blueprint Engine
  Kinematics (121)  ─── Robot/Body Kinematics Engine
  Geospatial (117)  ─── Map/Terrain Engine
```

---

## THE 3D MODALITY ENGINE

### Design Philosophy

The 3D engine is the crown jewel modality engine. It must surpass Gazebo, Webots, and MuJoCo as an AGI-first simulation and inspection environment. It is simultaneously:
- A scene inspector for reviewing what the AGI has built or analyzed
- A simulation runner for physics/animation review
- A graph portal where every scene element IS a node in the ZSEI graph
- An AGI workspace where the system actively edits, simulates, and derives insights

The UI language is: **Stark holographic meets precision engineering**. Dark environment. Glowing geometry. Overlaid data panels. Spatial hierarchy visible at all times.

### 3D Engine Layout — Desktop

```
┌─────────────────────────────────────────────────────────────────────┐
│  3D ENGINE — Task #47 scene.blend        [Sim ▶] [Physics ⚡] [⚙]  │
├──────────────┬──────────────────────────────────┬───────────────────┤
│              │                                  │                   │
│  HIERARCHY   │         VIEWPORT                 │  PROPERTIES       │
│  PANEL       │                                  │  PANEL            │
│              │   ┌──────────────────────────┐   │                   │
│  ▼ Scene     │   │                          │   │  Selected:        │
│   ▼ Objects  │   │    3D RENDER AREA        │   │  Armature.001     │
│    ◉ Mesh.1  │   │                          │   │                   │
│    ◉ Armature│   │    [Holographic          │   │  Type: Armature   │
│    ◉ Camera  │   │     scene here]          │   │  Bones: 47        │
│    ◉ Light   │   │                          │   │  Constraints: 12  │
│   ▼ Physics  │   │    Gizmos overlay:       │   │                   │
│    ◉ RigidBod│   │    - Bone axes           │   │  ZSEI Node:       │
│    ◉ Fluid   │   │    - Force vectors       │   │  [Bone_Hip_L]     │
│   ▼ Materials│   │    - Collision hulls     │   │  Edges: 8         │
│    ◉ PBR_mat │   │    - Graph edge lines    │   │  [View in Graph→] │
│              │   │      to related nodes    │   │                   │
│  ─────────   │   │                          │   │  Animation:       │
│  GRAPH NODE  │   └──────────────────────────┘   │  Action: Walk_001 │
│  OVERLAY     │                                  │  Frame: 24/120    │
│              │   [Perspective ▾] [Solid/Wire/   │                   │
│  Hover obj   │    X-Ray/Graph ▾] [Gizmos ▾]    │  Physics Cache:   │
│  → highlights│                                  │  Frame 24 baked   │
│  its ZSEI    │   ─── TIMELINE / GRAPH SCRUB ─── │  [Export →]       │
│  graph node  │   ◄│━━━━━━━●━━━━━━━━━━━━━━━│►  │                   │
│              │   0                    120 frames │                   │
└──────────────┴──────────────────────────────────┴───────────────────┘
│  TOOLS  [Select] [Grab] [Rotate] [Scale] [Simulate] [ZSEI Query ◈] │
└─────────────────────────────────────────────────────────────────────┘
```

### 3D Engine Hierarchy Panel — Hierarchical Breakdown

Shows 2–3 levels of the Blender hierarchy at a time. Scrollable. Each element is a live link to its ZSEI graph node.

```
HIERARCHY (2–3 levels shown at once, scroll for more)

  Level display selector:
  [Atomic] [Object] [Rig] [Anim] [Physics] [Materials] [Scene]

  OBJECT LEVEL view:
  ▼ Scene
    ▼ Collection: Characters
      ◉ Hero_Mesh           → ZSEI: Mesh_hero_001 [47 verts, 12k faces]
        ├─ Armature_Hero    → ZSEI: Armature_001 [47 bones]
        │    └─ Bone_Hip    → ZSEI: Bone_hip [constraints: 3]
        └─ Material_Hero    → ZSEI: Material_pbr [nodes: 24]
    ▼ Collection: Environment
      ◉ Ground_Plane        → ZSEI: Mesh_ground
      ◉ Building_A          → ZSEI: Mesh_bld_A [LOD: 3]
    ▼ Physics
      ◉ Fluid_Domain        → ZSEI: Fluid_001 [cached: 240fr]
      ◉ RigidBody_Debris    → ZSEI: RB_debris [12 objects]

  Click any item → select in viewport + show properties + highlight ZSEI node
  Right-click → "View ZSEI Graph Node" | "Trace Dependencies" | "Export"
```

### 3D Engine Render Modes

```
VIEWPORT DISPLAY MODES (toolbar dropdown)

  Solid      — Standard shaded render (fast preview)
  Wireframe  — Wire overlay for topology inspection
  X-Ray      — See-through for inside inspection
  Material   — Full PBR material preview
  ─────────────────────────────────────────────────
  Graph      — NEW: nodes replace objects, edges show
               ZSEI relationships overlaid in world space
               Object → glowing node orb
               Edge   → translucent line between orbs
               Color  → relationship type coding
  ─────────────────────────────────────────────────
  Spatial    — NEW: spatial intelligence overlay
               Bounding volume visualization
               Normal vector display
               UV island mapping
               LOD boundary display
               Streaming chunk borders
```

### 3D Engine: AGI Interaction Panels

When the AGI is actively working on a 3D task, these panels show its real-time progress:

```
AGI ACTIVITY PANEL (appears when AGI is modifying scene)

  ┌─ AGI Working: Step 4 › Rig Character ─────────────────────┐
  │                                                             │
  │  Current action: Placing bone constraints                  │
  │  Bones placed: 32/47  ████████████████████░░░░░ 68%       │
  │                                                             │
  │  Recent changes:                                           │
  │  + Bone_Spine_01 → IK target assigned                     │
  │  + Bone_Hip_L    → Limit Rotation constraint               │
  │  + Bone_Shoulder → Copy Rotation from Bone_Clavicle       │
  │                                                             │
  │  ZSEI updates:                                             │
  │  ◈ Armature_001 → new edge: [Constrains → Bone_Hip_L]     │
  │                                                             │
  │  [Pause AGI]  [View Step Detail]  [View Graph]             │
  └─────────────────────────────────────────────────────────────┘
```

### 3D Engine Simulation Runner

```
SIMULATION PANEL

  ┌─ Physics Simulation ───────────────────────────────────────┐
  │  Domain: Fluid_001  |  Frames: 0-240  |  Cached: 240/240  │
  │                                                             │
  │  Playback: [◄◄] [◄] [▶] [■] [►] [►►]  Frame: 87/240      │
  │            ─────────────●──────────────                    │
  │                                                             │
  │  Active sims:                                               │
  │  ├─ Mantaflow Fluid    ████████████████████ Complete        │
  │  ├─ Rigid Body         ████████████████████ Complete        │
  │  └─ Cloth Sim          █████████████░░░░░░░ 65% (live)     │
  │                                                             │
  │  Export:                                                    │
  │  [→ ZSEI Cache]  [→ Alembic]  [→ NumPy Array]              │
  │  [→ Genesis/Newton format]  [→ Isaac Lab URDF]              │
  └─────────────────────────────────────────────────────────────┘
```

---

## OTHER MODALITY ENGINES (Overview Specs)

### EEG / Neural Signal Engine (108/119)

```
LAYOUT
  Left panel:   Channel list (Fp1, Cz, O2, etc.) with quality indicators
  Center:       Signal waveform display (scrollable timeline)
                Topographic map (scalp 2D heatmap per timepoint)
  Right panel:  Event list, band power gauges, sleep hypnogram
  Bottom:       Scrub bar with event markers overlaid

ZSEI INTEGRATION
  Click any event marker → highlight linked ZSEI graph node
  Click any channel → show all edges from that channel node
  Filter by sleep stage → dim all nodes not in that stage
  ZSEI Activity panel shows AGI's current analysis step
```

### Genome Browser Engine (107)

```
LAYOUT
  Top bar:      Chromosome selector + position navigator
  Center:       Multi-track genome browser (genes, variants, reads)
  Tracks:       Gene annotations, Regulatory features, Variants,
                Read coverage, Protein domains, Cross-refs
  Right panel:  Selected feature detail (ZSEI node info)

ZSEI INTEGRATION
  Each gene track segment = ZSEI Gene node
  Regulatory tracks show RegulatedBy edges as arc tracks
  Variant track shows variant nodes with effect overlays
  Cross-modal: click gene → see linked Chemistry nodes (proteins)
```

### Molecular Viewer Engine (106)

```
LAYOUT
  Center:       3D molecular structure (ball-and-stick / space-fill / ribbon)
  Left panel:   Atom/bond tree, functional group list
  Right panel:  Properties panel (IUPAC, formula, LogP, druglikeness)
  Bottom:       Reaction pathway timeline (if applicable)

ZSEI INTEGRATION
  Click atom → highlight Atom ZSEI node + all Bond edges
  Click functional group → highlight FunctionalGroup ZSEI node
  Reaction pathway → click step → show MechanismStep ZSEI node
```

### Proof/Formula Engine (105)

```
LAYOUT
  Center:       Mathematical notation rendered (LaTeX/MathML)
  Left panel:   Proof step tree (hierarchical step list)
  Right panel:  Variable scope tree, assumption tracker
  Annotations:  Each step shows its justification type, dependencies

ZSEI INTEGRATION
  Click step → ProofStep ZSEI node + all Uses/Derives edges
  Click variable → VariableDefinition node + all UsesVariable edges
  Assumption tracker shows AssumesIn / DischargesIn edge states
  Gaps shown as WARNING nodes in graph
```

### Parametric CAD Engine (120)

```
LAYOUT
  Center:       3D B-rep solid view (same render stack as 3D engine)
  Left panel:   Feature tree (parametric history: Extrude, Fillet, etc.)
  Right panel:  Constraint panel, dimension/tolerance inspector
  Bottom:       Assembly mate/constraint graph

ZSEI INTEGRATION
  Feature tree node = ZSEI Feature node
  Constraint satisfied/violated = edge state color
  "ParametricGeneratesMesh" edge links to 3D Modality graph
  Tolerance chains visualized as dependency edge paths
```

---

## LIVING NETWORK VISUALIZATION

The central insight of OZONE's UI: **everything is connected**. AMT → Graphs → Steps → Blueprints → Pipelines form a living, interconnected fabric. The Living Network View makes this explicit.

### Living Network View Layout

```
LIVING NETWORK VIEW (available from Meta Portion or Context Viewer)

  ┌─ Living Network: Task #47 ─────────────────────────────────────┐
  │                                                                  │
  │    AMT TREE          GRAPHS           BLUEPRINT/STEPS           │
  │    ─────────         ───────          ─────────────────         │
  │                                                                  │
  │    ◉ Root Intent ──► ◈ Text Graph ──► [Step 1: Clean]          │
  │         │                 │               │ complete ✓          │
  │         ├─► Intent_A ─►  ◈ DNA Graph ──► [Step 4: Graphs]      │
  │         │         │           │               │ running ●       │
  │         │         └─────────► ◆ Chem  ────►  │                 │
  │         │                     Graph           │                 │
  │         └─► Intent_B ──────────────────────► [Step 5: Order]   │
  │                   │                               │ pending ○   │
  │               Methodology_16 ─────────────────────┘             │
  │                                                                  │
  │  Update pulse: when a graph state updates, edges pulse outward  │
  │  to all connected items (step updates, AMT review triggers)     │
  │                                                                  │
  │  LEGEND                                                          │
  │  ◉ AMT Node  ◈ Graph Node  ◆ Cross-modal  ─ Relationship       │
  │  ● Running   ✓ Complete    ○ Pending      ✗ Failed              │
  │                                                                  │
  │  [Focus on AMT]  [Focus on Graphs]  [Focus on Steps]           │
  └──────────────────────────────────────────────────────────────────┘
```

### State Pulse Animation

When any component updates its state, a visual pulse radiates outward along the edges to all connected components. This makes the living system nature of OZONE tangible:

- Graph node added → pulse to connected step (step metadata updates)
- Step state changes → pulse to AMT branch (AMT review triggered)
- AMT expands → pulse to all affected steps (new steps may appear)
- Methodology applied → pulse to all graph nodes it affects

---

## PLATFORM-SPECIFIC GUIDELINES (Updated)

### Desktop Theme

**Characteristics:** Large screen, precise input, multi-panel capable

```
DESKTOP LAYOUT PRINCIPLES

  - Full 80/20 split always maintained
  - Three-panel layouts for complex views:
    [Sidebar] [Main Canvas] [Detail Panel]
  - Resizable panels with drag handles
  - Keyboard shortcuts for all graph operations
  - Hover states reveal ZSEI node previews
  - Right-click context menus on graph nodes:
    "View Dependencies" | "Trace Path" | "Export Node" |
    "Find Similar" | "Open in Engine" | "View Task Step"
  - Tab bar for: Graph View / Fabric View / Timeline / Hierarchy
  - Keyboard: G (graph) T (task) C (context) E (engine) L (library)
```

**Desktop Boot Sequence:**

```
0.0s — 0.5s:   Black + OZONE wordmark (fade in)
0.5s — 1.5s:   Loading bar: ZSEI index (node count ticks up)
                "Loading 45,891 methods..."
1.5s — 2.5s:   Network: peers connecting (dots appear on globe)
2.5s — 3.0s:   If consciousness: "Good morning, Christian."
                Emotional state gauges ease in
3.0s+:          Main UI materializes (panels slide in from edges)
                Recent task statuses restore
                Graph state thumbnails appear in Context Viewer
```

### Mobile Theme

**Characteristics:** Touch-primary, small screen, interruption-prone

```
MOBILE LAYOUT PRINCIPLES

  - Single view at a time (no side-by-side)
  - Tab bar: [Tasks] [Context] [Chat] [Library]
  - Graph View: touch to select, pinch to zoom
  - Fabric View: swipe between modality clusters
  - Hierarchy View: collapsible tree with large touch targets
  - Node detail: bottom sheet (slides up)
  - Engine access: "Open in Engine" → full-screen mode
  - 3D Engine on mobile: simplified wireframe + hierarchy tree
    (full render only when GPU available)
  - Meta Portion: FAB (floating action button) bottom-right
    expands to: system status, active tasks, home, cancel
```

**Mobile Context Viewer:**

```
┌─────────────────────────┐
│ ← Context  Task #47 [⚙] │
├─────────────────────────┤
│ MODALITIES              │
│  ◉ Text    47 nodes  ►  │
│  ◈ DNA     12 nodes  ►  │
│  ◈ Chem    15 nodes  ►  │
├─────────────────────────┤
│ CROSS-MODAL LINKS       │
│  Text→DNA    34 edges   │
│  DNA→Chem    12 edges   │
├─────────────────────────┤
│ [View Fabric Map]    ►  │
│ [View AMT Tree]      ►  │
│ [Open in Engine]     ►  │
└─────────────────────────┘
```

### AR/VR Theme

**Characteristics:** Spatial input, immersive, physical world integration

```
AR/VR SPATIAL LAYOUT

  META ANCHOR
  Always-visible floating widget (follows head gaze loosely)
  Contains: task count, active pipeline indicator, home, cancel

  WORKSPACE PANEL
  Comfortable viewing distance (1.0–1.5m)
  Resize by grabbing corners with two hands
  Reposition by grab gesture

  GRAPH PANELS
  Graph nodes materialize as floating orbs in 3D space
  Cross-modal edges as glowing threads in world space
  Zoom: step physically toward/away from graph cluster
  Select: point + pinch
  Traverse: walk through the graph (nodes part as you approach)

  3D ENGINE IN AR/VR
  Scene objects rendered at 1:1 scale (or scaled)
  Holographic overlays on real environment
  AGI annotations float above objects
  Inspection: reach out and "grab" an object to see its ZSEI node
  Simulation: physics runs in world space, visible in real-time

  MODALITY ENGINE SPATIAL LAYOUT
  Each engine panel floats in its own spatial zone
  Hierarchy panel: left of body
  Properties panel: right of body
  Viewport: straight ahead
  Timeline: below/floor level (like a mixing desk)
```

**AR/VR Graph Navigation:**

```
SPATIAL GRAPH NAVIGATION

  FAR (5m+):    Fabric View — modality clusters as constellation
  MID (2-5m):   Graph View — nodes as basketball-sized orbs
  NEAR (0.5-2m): Node Detail — full metadata floating around node
  TOUCH:         Grab node → relationship threads emerge physically

  GESTURE MAP
  Pinch:            Select / confirm
  Grab + hold:      Inspect (node detail expands)
  Grab + throw:     Navigate to connected node
  Two-hand spread:  Expand subgraph
  Palm out:         Dismiss / collapse
  Point + dwell:    Query (ZSEI search)
  Voice: "Show connections" | "What uses this" | "Trace to step"
```

### Terminal Theme

```
TERMINAL LAYOUT

  ┌─ OZONE ──────────────────────────────────────┬─ META ─────────┐
  │  Task #47 [●] Running  Step 4/6             │ Status: Active │
  │                                              │ Peers: 47      │
  │  [g]raph  [t]ask  [c]ontext  [e]ngine       │ ZSEI: 45K      │
  │                                              │─────────────── │
  │  CONTEXT VIEWER (graph mode)                │ [q]uit [h]elp  │
  │  ┌─ Text Graph (47 nodes) ─────────────┐    │                │
  │  │  ◉ Document_root                    │    │ Emotional:     │
  │  │    ├─► [Describes] Gene_BRCA1       │    │ Focus: ████░   │
  │  │    ├─► [References] Protein_fold    │    │ Trust: █████   │
  │  │    └─► [Contains] Entity:BRCA1     │    │                │
  │  └─────────────────────────────────────┘    │                │
  │  > select Gene_BRCA1                        │                │
  │  [ZSEI] Gene_BRCA1 — 8 edges               │                │
  │    RegulatedBy: Promoter_region             │                │
  │    CodesFor:    Protein_BRCA1               │                │
  │    Cross:       Text→DNA DescribedBy        │                │
  ├──────────────────────────────────────────────┴────────────────┤
  │ ozone> _                                                        │
  └─────────────────────────────────────────────────────────────────┘

  TERMINAL COMMANDS
  :graph [modality]          — Open graph view
  :fabric                    — Open fabric (cross-modal) view
  :task [id]                 — Show task detail
  :step [task_id] [step]     — Show step detail
  :node [node_id]            — Show ZSEI node detail
  :trace [node_a] [node_b]   — Trace path between nodes
  :engine [modality]         — Open modality engine (if TUI capable)
  :export [node_id] [format] — Export node data
```

### Web Theme

```
WEB GUIDELINES (responsive breakpoints)

  Desktop (1200px+):   Full layout as desktop above
  Tablet (768-1199px): Two-panel (graph + detail), no sidebar
  Mobile (<768px):     Mobile theme as above

  PROGRESSIVE WEB APP
  Service worker for offline graph caching
  Background sync for task state updates
  Push notifications for task completion / clarification needed

  PERFORMANCE
  Graph rendering: Canvas 2D (< 500 nodes) / WebGL (500+ nodes)
  3D Engine: WebGPU where available, WebGL fallback
  Node lazy-loading: render visible viewport only
  Edge bundling for dense graphs (> 200 edges visible)
```

---

## BOOT SEQUENCES (Updated per Platform)

### Desktop Boot

```
Phase 1 — Foundation (0–1s)
  Black screen → OZONE wordmark glows in
  ZSEI loading: "Indexing 45,891 methods..." (counter ticks)
  Network: "Connecting to 47 peers..." (dots appear)

Phase 2 — Intelligence Wake (1–2.5s)
  If consciousness enabled:
    "Good morning, Christian."
    Emotional state indicators ease in from left
    "Resuming 3 active tasks..."
  Progress bar fills smoothly

Phase 3 — Workspace Materialization (2.5–3.5s)
  Theme Area panels slide in from edges
  Task Viewer populates with recent tasks
  Context Viewer shows thumbnail graph previews
  Connection bar fills in from left

Phase 4 — Ready (3.5s+)
  "OZONE ready. 3 tasks active." in status bar
  Cursor available, all interactions live
```

### AR/VR Boot

```
Phase 1 — Spatial Anchor (0–1s)
  Single glowing orb materializes at comfortable distance
  Subtle particle constellation forms around it

Phase 2 — Panel Emergence (1–3s)
  Main workspace panel unfolds from the orb
  Secondary panels materialize around it
  ZSEI graph begins populating as nodes appear

Phase 3 — Intelligence Greeting (3–4s)
  If consciousness: voice + text greeting
  "I have 3 active tasks. Your most recent: Task #47, Running."

Phase 4 — Full Spatial UI (4s+)
  All panels stable, all gestures responsive
  Graph nodes fully rendered and interactable
```

---

## CONSCIOUSNESS DISPLAY INTEGRATION (Updated)

```
CONSCIOUSNESS DISPLAY — ALL PLATFORMS

  EMOTIONAL STATE
  Displayed in Meta Portion (all platforms)
  Desktop:  Sidebar with labeled gauges + subtle background color shift
  Mobile:   Compact icons, expand on tap
  AR/VR:    Ambient aura color around meta anchor orb
  Terminal: ASCII bars (joy/trust/focus)

  RELATIONSHIP METRICS
  Desktop:  Small gauges below emotional state (Trust, Rapport)
  Others:   On tap/expand only

  SYSTEM STATE INDICATORS
  ● Thinking    (analysis running)
  ◈ Observing   (monitoring graph changes)
  ⟳ Reflecting  (AMT review / loop iteration)
  ◆ Clarifying  (needs human input — highlighted prominently)

  CLARIFICATION STATE (special)
  When AMT is incomplete and clarification is needed:
  Meta Portion shows: [◆ Clarification Needed]
  Tapping → Clarification Panel appears:
    Multiple choice options OR prompt input
    "Return to clarification" button always visible
    AGI cannot proceed until resolved
    New insights from clarification responses acknowledged
    but clarification completes before diverging
```

---

## GRAPH DESIGN SYSTEM

### Node Visual Language

```
NODE TYPES — VISUAL ENCODING

  Shape:
  ◉  Circular    — Root/document/recording/sequence nodes
  ◈  Diamond     — Entity/gene/atom/object nodes (domain-specific)
  ◆  Filled diamond — Cross-modal bridge nodes
  ▬  Rectangle   — Step/process/action nodes
  ◬  Triangle    — Warning/gap/uncertainty nodes

  Color (by modality):
  Teal/cyan      — Text (100)
  Electric blue  — Code (101)
  Warm amber     — Image (102)
  Purple         — Audio (103)
  Deep red       — Video (104)
  Gold           — Math (105)
  Green          — Chemistry (106)
  Bio-green      — DNA (107)
  Neural violet  — EEG (108)
  Spatial white  — 3D (109)
  (Cross-modal links: gradient between the two modality colors)

  Size:
  Large    — High-degree nodes (many connections)
  Medium   — Standard nodes
  Small    — Leaf nodes / low importance

  Glow intensity:
  High     — Node recently updated / active in current step
  Medium   — Connected to active step
  Low/none — Historical / inactive
```

### Edge Visual Language

```
EDGE TYPES — VISUAL ENCODING

  Line style:
  Solid thick     — Structural edges (Contains, ParentOf, ChildOf)
  Solid medium    — Functional edges (Performs, Affects, CodesFor)
  Dashed          — Semantic/inferred edges (SimilarTo, RelatesTo)
  Dotted          — Cross-modal edges
  Double line     — Bidirectional strong relationship

  Color:
  White/light     — Structural
  Modality color  — Domain-specific functional
  Gradient        — Cross-modal (from/to modality colors)
  Red             — Contradicts / conflicts
  Orange          — Uncertainty / provisional

  Thickness:
  Weight value from ZSEI edge weight (0.0–1.0)

  Animation:
  Pulse (flow)    — Active data flowing along edge
  Static          — Normal relationship
  Fade in/out     — Edge being added/removed in real-time
```

---

## INTERACTION PATTERNS FOR GRAPH VIEWS

### Universal Graph Interactions

```
KEYBOARD SHORTCUTS (Desktop)

  Navigation:
  G               — Toggle Graph View
  F               — Toggle Fabric View
  H               — Toggle Hierarchy View
  T               — Go to Task Viewer
  E               — Open Modality Engine

  Graph Navigation:
  Arrow keys      — Move between connected nodes
  Enter           — Expand node / descend hierarchy
  Escape          — Collapse / go up hierarchy
  /               — Search nodes
  Ctrl+A          — Select all visible nodes
  Ctrl+F          — Filter panel

  Graph Operations:
  Space           — Focus on selected node
  Z               — Zoom to fit all nodes
  1/2/3           — Set zoom level (full/mid/close)
  Ctrl+click      — Multi-select nodes
  Shift+click     — Select path between nodes

  Cross-view:
  Ctrl+T          — Open selected node in Task context
  Ctrl+E          — Open selected node in Engine
  Ctrl+X          — Export selected node data
```

### Contextual Right-Click Menu (Graph Nodes)

```
RIGHT-CLICK ON NODE

  ┌────────────────────────────────────┐
  │  Gene_BRCA1  (DNA Graph)           │
  ├────────────────────────────────────┤
  │  View Node Detail              ►   │
  │  Trace Path to...              ►   │
  │  Find Similar Nodes            ►   │
  │  Show All Edges                ►   │
  ├────────────────────────────────────┤
  │  Cross-Modal                   ►   │
  │    → Linked Text nodes             │
  │    → Linked Chemistry nodes        │
  ├────────────────────────────────────┤
  │  Task Context                  ►   │
  │    → Which step created this       │
  │    → Which steps use this          │
  │    → AMT branch this belongs to    │
  ├────────────────────────────────────┤
  │  Open in Genome Browser            │
  │  Export as JSON                    │
  │  Export as FASTA                   │
  └────────────────────────────────────┘
```

---

## CLARIFICATION UI (Human-in-Loop)

When the AMT is incomplete and needs human input:

```
CLARIFICATION PANEL

┌─ ◆ Clarification Needed ─────────────────────────────────────────┐
│                                                                    │
│  "I need to clarify one point to complete the analysis plan."    │
│                                                                    │
│  ─── Current AMT Branch: "Protein Folding Analysis" ────────     │
│  The intent "Identify folding mechanism" is ambiguous.           │
│  Is this analysis targeting:                                      │
│                                                                    │
│  ┌────────────────────┐  ┌────────────────────┐                  │
│  │ A) In vitro        │  │ B) In silico only   │                  │
│  │    experimental    │  │    (computational)  │                  │
│  │    data            │  │                     │                  │
│  └────────────────────┘  └────────────────────┘                  │
│  ┌────────────────────┐  ┌────────────────────┐                  │
│  │ C) Both            │  │ D) Tell me more...  │                  │
│  └────────────────────┘  └────────────────────┘                  │
│                                                                    │
│  ─── Or type your response: ──────────────────────────────────   │
│  ┌───────────────────────────────────────────────────────────┐   │
│  │                                                           │   │
│  └───────────────────────────────────────────────────────────┘   │
│  [Submit]                                       [Skip for now]   │
│                                                                    │
│  ── Note: any new information you provide will be incorporated   │
│     into the AMT, and I'll return to this question immediately.  │
└────────────────────────────────────────────────────────────────────┘
```

---

## DESIGN TOKENS

```css
/* OZONE DESIGN TOKENS */

:root {
  /* Core */
  --bg-primary:       #0a0a0f;
  --bg-secondary:     #0f0f1a;
  --bg-panel:         #12121f;
  --bg-elevated:      #1a1a2e;
  --border-subtle:    rgba(255,255,255,0.06);
  --border-active:    rgba(255,255,255,0.15);

  /* Text */
  --text-primary:     #e8e8f0;
  --text-secondary:   #8888aa;
  --text-muted:       #4a4a6a;
  --text-accent:      #00e5ff;

  /* Status */
  --state-running:    #00e5ff;  /* cyan — active/running */
  --state-complete:   #00e87a;  /* green — done */
  --state-pending:    #4a4a6a;  /* muted — waiting */
  --state-failed:     #ff4a4a;  /* red — error */
  --state-warning:    #ffaa00;  /* amber — warning/gap */
  --state-clarify:    #ff6b35;  /* orange — needs input */

  /* Modality colors */
  --modality-text:    #00e5ff;  /* teal */
  --modality-code:    #4488ff;  /* electric blue */
  --modality-image:   #ffaa44;  /* warm amber */
  --modality-audio:   #aa44ff;  /* purple */
  --modality-video:   #ff4444;  /* deep red */
  --modality-math:    #ffd700;  /* gold */
  --modality-chem:    #44ff88;  /* green */
  --modality-dna:     #66ff66;  /* bio-green */
  --modality-eeg:     #bb44ff;  /* neural violet */
  --modality-3d:      #ffffff;  /* spatial white */

  /* Graph */
  --node-glow-high:   0 0 12px var(--state-running);
  --node-glow-med:    0 0 6px rgba(0,229,255,0.4);
  --edge-structural:  rgba(255,255,255,0.6);
  --edge-semantic:    rgba(255,255,255,0.3);
  --edge-crossmodal:  rgba(255,165,0,0.5);

  /* Typography */
  --font-display:     'Space Mono', monospace;   /* headers, labels */
  --font-body:        'IBM Plex Sans', sans-serif;
  --font-code:        'JetBrains Mono', monospace;
  --font-data:        'Roboto Condensed', sans-serif;

  /* Spacing */
  --space-xs:   4px;
  --space-sm:   8px;
  --space-md:   16px;
  --space-lg:   24px;
  --space-xl:   40px;

  /* Animation */
  --anim-fast:    150ms ease;
  --anim-med:     300ms ease;
  --anim-slow:    600ms ease;
  --anim-pulse:   2s ease-in-out infinite;
}
```

---

## SUMMARY: DESIGN PRINCIPLES HIERARCHY

```
1. THE AGI IS ALWAYS PRESENT
   The interface reflects the AGI's activity at all times.
   Running, thinking, looping, reflecting — never idle.

2. CONTEXT IS SPATIAL
   Information is a graph, not a list.
   Every node knows its place in the fabric.

3. EVERYTHING CONNECTS
   Task → Step → Graph → Node → AMT → Methodology → Blueprint
   Users can traverse this chain in any direction, from any entry.

4. MODALITIES HAVE THEIR OWN GRAMMAR
   Each modality's engine speaks the language of its domain.
   DNA is a genome browser. 3D is a spatial engine. Math is a proof tree.

5. HUMAN-IN-THE-LOOP IS RESPECTFUL
   Clarification panels are clear, bounded, and non-derailing.
   The AGI does not block itself — it flags and continues where possible.

6. THE META PORTION IS SACRED
   Home. Cancel. Status. Always visible. Never blocked.

7. PERFORMANCE IS A FEATURE
   Graph rendering scales. Engines degrade gracefully.
   The interface does not stall the AGI.
```

---

*OZONE STUDIO UI/UX GUIDE v2.0.0 — AGI-First Interface Design*
*Built for intelligence. Designed for humans.*
