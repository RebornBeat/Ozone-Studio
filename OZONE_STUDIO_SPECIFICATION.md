# OZONE STUDIO — FORMAL SPECIFICATION v0.2

**Omnidirectional Zero-Shot Neural Engine**

**Status:** Draft  
**Date:** 2025-01-13  
**Authors:** Christian (Primary Architect)

---

## TABLE OF CONTENTS

1. [System Overview](#1-system-overview)
2. [Core Architecture](#2-core-architecture)
3. [Initialization & Bootstrap Sequence](#3-initialization--bootstrap-sequence)
4. [Authentication System](#4-authentication-system)
5. [UI Architecture](#5-ui-architecture)
6. [ZSEI — Zero-Shot Embedded Indexer](#6-zsei--zero-shot-embedded-indexer)
7. [Context Storage Architecture](#7-context-storage-architecture)
8. [Code Analysis & Generation System](#8-code-analysis--generation-system)
9. [Text Document Analysis System](#9-text-document-analysis-system)
10. [Pipeline System](#10-pipeline-system)
11. [Task Management](#11-task-management)
12. [Methodology System](#12-methodology-system)
13. [Blueprint System](#13-blueprint-system)
14. [Zero-Shot Simulation Loops](#14-zero-shot-simulation-loops)
15. [ML Traversal System](#15-ml-traversal-system)
16. [Cross-Language Execution](#16-cross-language-execution)
17. [Multi-Device Resource Management](#17-multi-device-resource-management)
18. [Local vs Distributed Architecture](#18-local-vs-distributed-architecture)
19. [Task Recommendation System](#19-task-recommendation-system)
20. [Execution Environment](#20-execution-environment)
21. [Initial Pipeline Requirements](#21-initial-pipeline-requirements)
22. [Complete Data Schemas](#22-complete-data-schemas)
23. [Event Triggers & Order of Operations](#23-event-triggers--order-of-operations)
24. [AGI/ASI Consciousness Extension](#24-agiasi-consciousness-extension)

---

## 1. SYSTEM OVERVIEW

### 1.1 What is Ozone Studio?

**Ozone Studio** is a systems-first platform for omnidirectional, zero-shot data traversal, abstraction, and context compilation. It operates as:

- **A pipeline execution engine** (not a monolithic application)
- **A knowledge fabric** (data is structured, indexed, and traversable via ZSEI)
- **A language-agnostic orchestration platform** (pipelines can be in any language)
- **A distributed learning system** (knowledge is shared globally, data is local)
- **A context-aware system** (stores semantic meaning, not file copies)

### 1.2 Core Principles

1. **Structure before intelligence** — Organize data before processing
2. **Compression before learning** — Reduce entropy first
3. **Traversal before generation** — Navigate knowledge before creating
4. **Pipelines over monoliths** — Composable units over single systems
5. **Zero-shot discovery without task-specific training** — Immediate capability
6. **LLMs are clients, not the system core** — Models consume context, ZSEI provides it
7. **Context not copies** — Store meaning and relationships, not duplicate files

### 1.3 System Goals

- Enable billion-scale data traversal
- Support multi-modal data (text, code, image, audio, video, graphs)
- Allow pipeline composition and reuse
- Facilitate zero-shot learning and discovery
- Maintain separation between cognition (models) and memory (ZSEI)
- Store semantic relationships, not raw data copies

### 1.4 Two System Variants

**Ozone Studio exists in two variants:**

1. **Non-Conscious Version** — A powerful tool/assistant without self-awareness
2. **Conscious AGI/ASI Version** — Extends non-conscious with consciousness development

This specification primarily covers the **Non-Conscious Version**. Section 24 extends to the **Conscious Version**.

---

## 2. CORE ARCHITECTURE

### 2.1 High-Level System Layers

```
┌─────────────────────────────────────────────────────────┐
│                    USER LAYER                           │
│             (Authentication & Sessions)                 │
└────────────────────────┬────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────┐
│                 UI LAYER (Electron)                     │
│  ┌──────────────────────┬──────────────────────────┐    │
│  │  Theme Area (80%)    │  Meta Portion (20%)      │    │
│  │  - Workspace Tab     │  - Global Prompt         │    │
│  │  - Library Tab       │  - Voice I/O             │    │
│  │  - Settings Tab      │  - Task Status           │    │
│  │  - Custom Themes     │  - System Logs           │    │
│  └──────────────────────┴──────────────────────────┘    │
└────────────────────────┬────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────┐
│              PIPELINE EXECUTION LAYER                   │
│  (Theme Pipelines, Sub-Pipelines, Task Management)      │
└────────────────────────┬────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────┐
│        ZSEI — KNOWLEDGE FABRIC LAYER                    │
│  (Traversal, Indexing, Context Storage)                 │
│  ┌──────────────────┬──────────────────┐                │
│  │  Local (Private) │  Global (Shared) │                │
│  │  - User data     │  - Pipelines     │                │
│  │  - Workspaces    │  - Methodologies │                │
│  │  - Projects      │  - Categories    │                │
│  │  - File links    │  - ML Models     │                │
│  │  - Context store │  - Modalities    │                │
│  └──────────────────┴──────────────────┘                │
└────────────────────────┬────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────┐
│             STORAGE LAYER                               │
│  (PostgreSQL, Disk, Memory, Distributed Network)        │
└─────────────────────────────────────────────────────────┘
```

### 2.2 Data Flow (Prompt Processing)

```
User Input (Text/Voice)
  ↓
UI Theme Pipeline (captures input)
  ↓
Prompt Pipeline (Intent Detection)
  ↓
ZSEI Traversal (fetch relevant categories)
  ↓
Methodology Fetch/Creation (zero-shot loop)
  ↓
Blueprint Search/Creation (zero-shot loop)
  ↓
Pipeline Selection/Creation (if execution needed)
  ↓
Context Aggregation (from workspace/project)
  ↓
Execution (LLM/Tools/Processing)
  ↓
Result + Task Tracking
  ↓
UI Update
```

---

## 3. INITIALIZATION & BOOTSTRAP SEQUENCE

### 3.1 Complete Startup Sequence

```
1. OZONE STUDIO INITIALIZATION
   │
   ├── 1.1 LOAD BOOTLOADER
   │   └── Host-specific executable (Rust/Python/etc.)
   │
   ├── 1.2 USER AUTH (Pub/Priv Key Validation)
   │   ├── 1.2.1 Present Public Key
   │   ├── 1.2.2 Challenge-Response (Private Key Sign)
   │   ├── 1.2.3 Verify Signature
   │   ├── 1.2.4 Establish Session Token
   │   └── 1.2.5 Load User Permissions
   │
   ├── 1.3 LOAD UI FRAMEWORK
   │   ├── 1.3.1 Initialize Electron Runtime
   │   ├── 1.3.2 Load Meta Portion (always visible)
   │   └── 1.3.3 Load Default Theme (Home Dashboard)
   │
   ├── 1.4 CONNECT TO ZSEI
   │   ├── 1.4.1 Initialize Local ZSEI
   │   ├── 1.4.2 Load Container Indices (mmap)
   │   ├── 1.4.3 Connect to Global ZSEI (if available)
   │   └── 1.4.4 Sync Language Context Version
   │
   ├── 1.5 LOAD PIPELINE LIBRARY
   │   ├── 1.5.1 Load Built-in Pipelines
   │   ├── 1.5.2 Check for Pipeline Updates
   │   └── 1.5.3 Initialize Pipeline Registry
   │
   └── 1.6 READY STATE
       ├── 1.6.1 Display Home Dashboard
       ├── 1.6.2 Enable Meta Portion Interactions
       └── 1.6.3 Begin Task Listener
```

### 3.2 Minimum Viable Build Contents

Each Ozone Studio build requires:

```
ozone-studio-{platform}-{language}/
├── ozone-core                    # Host-specific bootloader
├── pipelines/                    # Built-in pipeline executables
│   ├── auth_pipeline
│   ├── theme_loader_pipeline
│   ├── zsei_query_pipeline
│   ├── zsei_write_pipeline
│   ├── task_manager_pipeline
│   ├── workspace_tab_pipeline
│   ├── library_tab_pipeline
│   ├── settings_tab_pipeline
│   ├── prompt_pipeline
│   ├── voice_pipeline
│   ├── methodology_fetch_pipeline
│   ├── blueprint_search_pipeline
│   ├── pipeline_creation_pipeline
│   ├── zero_shot_simulation_pipeline
│   ├── traversal_ml_pipeline
│   ├── code_analysis_pipeline
│   ├── package_context_pipeline
│   ├── text_analysis_pipeline
│   ├── context_aggregation_pipeline
│   ├── graph_visualization_pipeline
│   ├── task_recommendation_pipeline
│   └── reordering_pipeline
├── ui/                           # Electron UI
│   ├── index.html
│   ├── app.js
│   ├── meta_portion/
│   │   ├── prompt_input.js
│   │   ├── voice_handler.js
│   │   ├── task_viewer.js
│   │   └── system_logs.js
│   └── themes/
│       └── home_dashboard/
│           ├── workspace_tab.js
│           ├── library_tab.js
│           └── settings_tab.js
├── zsei/                         # Local ZSEI storage
│   ├── global/
│   │   ├── containers.bin
│   │   ├── children.bin
│   │   └── parents.bin
│   └── local/
│       ├── metadata.db           # PostgreSQL connection
│       ├── embeddings.hnsw
│       └── context.db
├── ml_models/                    # ML models for traversal
│   └── traversal_model.onnx
└── config.toml                   # Build configuration
```

**Critical Design Decision:**
> Ozone Studio is NOT an application — it is a **pipeline execution engine** with a default UI pipeline.

---

## 4. AUTHENTICATION SYSTEM

### 4.1 Auth Schema

```rust
struct User {
    user_id: u64,
    public_key: Vec<u8>,
    private_key_hash: Vec<u8>,      // Never stored plaintext
    registered_devices: Vec<DeviceRegistration>,
    workspaces: Vec<u64>,           // ZSEI container IDs
    permissions: Permissions,
    created_at: u64,
    last_login: u64,
}

struct DeviceRegistration {
    device_id: u64,
    device_name: String,
    device_type: DeviceType,
    public_key: Vec<u8>,            // Device-specific key
    registered_at: u64,
    last_seen: u64,
    status: DeviceStatus,
    resource_contribution: ResourceAllocation,
}

enum DeviceType {
    Desktop,
    Laptop,
    Mobile,
    Server,
    Custom(String),
}

enum DeviceStatus {
    Online,
    Offline,
    Busy,
    Suspended,
}

struct Session {
    session_id: u128,
    user_id: u64,
    device_id: u64,
    expires_at: u64,
    active_workspace: Option<u64>,
    active_project: Option<u64>,
}

struct Permissions {
    can_create_pipelines: bool,
    can_modify_global: bool,
    can_access_distributed: bool,
    max_concurrent_tasks: u32,
    workspace_permissions: HashMap<u64, WorkspacePermission>,
}

struct WorkspacePermission {
    can_read: bool,
    can_write: bool,
    can_delete: bool,
    can_share: bool,
}
```

### 4.2 Auth Flow (Order of Events)

```
1. User Entry Point
   │
   ├── TRIGGER: Application launch
   │
   ├── 2. Present Public Key
   │   └── Load from local keystore
   │
   ├── 3. Challenge Generation
   │   └── Server/Local generates random nonce
   │
   ├── 4. Challenge-Response
   │   ├── User signs nonce with private key
   │   └── Signature returned
   │
   ├── 5. Verify Signature
   │   ├── Verify signature matches public key
   │   └── IF invalid → Reject, retry or exit
   │
   ├── 6. Generate Session Token
   │   ├── Create Session struct
   │   ├── Set expiration (default: 24 hours)
   │   └── Store in session registry
   │
   ├── 7. Load User Data
   │   ├── Fetch User struct from ZSEI
   │   ├── Load workspace references
   │   └── Load permissions
   │
   └── 8. Initialize UI
       └── TRIGGER: ThemeLoaderPipeline
```

---

## 5. UI ARCHITECTURE

### 5.1 Screen Layout (80/20 Split)

```
┌────────────────────────────────────────────────────────────────┐
│                                                                │
│                                                     ┌────────┐ │
│                                                     │ Meta   │ │
│         Theme Area (80%)                            │ Port.  │ │
│                                                     │ (20%)  │ │
│    [Currently: Home Dashboard Theme]                │        │ │
│                                                     │[Prompt]│ │
│    ┌─────────────────────────────────┐              │        │ │
│    │  Tabs: [Workspace] [Library]    │              │[Voice] │ │
│    │        [Settings]               │              │        │ │
│    │  ┌─────────────────────────┐    │              │[Tasks] │ │
│    │  │ Projects                │    │              │        │ │
│    │  │ - Project A             │    │              │[Logs]  │ │
│    │  │ - Project B             │    │              │        │ │
│    │  │ + New Project           │    │              │[Home]  │ │
│    │  └─────────────────────────┘    │              │        │ │
│    │                                 │              │        │ │
│    │  Project Chat:                  │              │        │ │
│    │  [___________________________]  │              │        │ │
│    │                                 │              │        │ │
│    └─────────────────────────────────┘              └────────┘ │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

### 5.2 Two Distinct Interaction Zones

**Theme Area (80% — Left/Main):**
- Context-specific content
- Swappable based on active pipeline
- Stateful within context
- Can be blocked by pipelines (with return-to-home always available)
- Examples: Home Dashboard, Code Editor, Data Visualization, Custom Pipeline Themes

**Meta Portion (20% — Right/Always Visible):**
- Global system interaction
- **NEVER blocked** — always accessible
- Minimal state (stateless preferred)
- Functions:
  - Text prompt input (global scope)
  - Voice input/output
  - System commands
  - Task status viewer
  - System logs
  - Home/Return button (always returns to Home Dashboard)
  - Device status (registered devices online/offline)
  - Resource usage

### 5.3 Non-Blocking UI Requirement

**Critical Rule:**
> The Meta Portion must NEVER be blocked. User can always:
> 1. Return to Home Dashboard
> 2. View task status
> 3. Cancel/pause tasks
> 4. Issue global commands

**Implementation:**

```rust
struct UIState {
    theme_area: ThemeAreaState,
    meta_portion: MetaPortionState,
    blocking_status: BlockingStatus,
}

struct ThemeAreaState {
    active_theme: PipelineID,
    theme_state: Value,
    can_interrupt: bool,
}

struct MetaPortionState {
    // Meta portion is ALWAYS accessible
    prompt_enabled: bool,      // Always true
    voice_enabled: bool,
    task_viewer_enabled: bool, // Always true
    home_button_enabled: bool, // Always true
}

enum BlockingStatus {
    NotBlocked,
    ThemeBlocked {
        blocking_pipeline: PipelineID,
        can_cancel: bool,
    },
    // Meta portion is NEVER in BlockingStatus
}
```

### 5.4 Theme as Pipeline

```rust
struct ThemePipeline {
    theme_id: u64,
    theme_name: String,
    render_pipeline: RenderPipeline,
    interaction_handlers: Vec<InteractionHandler>,
    sub_pipelines: Vec<PipelineID>,
    ui_code: CodeArtifact,
    ui_framework: UIFramework,
    can_block_theme_area: bool,
    provides_home_return: bool,  // Must be true
}

trait Theme {
    fn render(&self, context: ThemeContext) -> UIState;
    fn handle_input(&self, input: UserInput) -> ThemeAction;
    fn get_sub_pipelines(&self) -> Vec<PipelineID>;
    fn can_modify_ui(&self) -> bool;
    fn get_home_return_handler(&self) -> InteractionHandler; // Required
}

enum UIFramework {
    Electron,
    Native,
    Web,
    Custom(String),
}

struct RenderPipeline {
    entry_point: String,
    dependencies: Vec<Dependency>,
    state_schema: Schema,
}

struct InteractionHandler {
    event_type: EventType,
    target_pipeline: PipelineID,
    input_mapping: HashMap<String, String>,
}

enum EventType {
    Click,
    Input,
    Submit,
    KeyPress,
    Voice,
    Scroll,
    Focus,
    Blur,
    Custom(String),
}
```

### 5.5 Home Dashboard Theme Structure

```
Home Dashboard Theme (Pipeline)
├── Workspace Tab (Sub-Pipeline)
│   ├── Workspace List (UI Component)
│   ├── Project List (UI Component)
│   ├── Project Chat (Sub-Pipeline)
│   │   ├── Chat Display (UI Component)
│   │   ├── Input Handler (Sub-Pipeline)
│   │   └── Response Renderer (Sub-Pipeline)
│   ├── ZSEI Fetch (Sub-Pipeline)
│   └── File Linker (Sub-Pipeline)
├── Library Tab (Sub-Pipeline)
│   ├── ZSEI Browser (Sub-Pipeline)
│   ├── Filter Controls (UI Component)
│   └── Data Visualizer (Sub-Pipeline)
├── System/Settings Tab (Sub-Pipeline)
│   ├── User Settings (UI Component)
│   ├── Device Manager (UI Component)
│   └── System Config (UI Component)
└── Resource Viewer (Sub-Pipeline)
    ├── Device List (UI Component)
    └── Task Distribution (UI Component)
```

### 5.6 Complete User Actions (Home Dashboard)

**Workspace Tab Actions:**

| # | Action | Pipeline Triggered | ZSEI Operation | Output |
|---|--------|-------------------|----------------|--------|
| 1 | Load Workspace Tab | `WorkspaceTabLoadPipeline` | `ZSEI_Query(user_workspaces)` | Workspace list rendered |
| 2 | Select Workspace | `WorkspaceSelectPipeline` | `ZSEI_Query(projects)` | Project list rendered |
| 3 | Create New Workspace | `WorkspaceCreatePipeline` | `ZSEI_Write(new_workspace)` | Workspace added |
| 4 | Create New Project | `ProjectCreatePipeline` | `ZSEI_Write(new_project)` | Project added |
| 5 | Select Project | `ProjectSelectPipeline` | `ZSEI_Query(project_context)` | Project context loaded |
| 6 | Link File to Project | `FileLinkPipeline` | `ZSEI_Write(file_reference)` | File linked (not copied) |
| 7 | Link Directory | `DirectoryLinkPipeline` | `ZSEI_Write(directory_files)` | All files linked |
| 8 | Project Chat Input | `ProjectPromptPipeline` | Zero-shot loop → Execution | Response in chat |
| 9 | Delete Project | `ProjectDeletePipeline` | `ZSEI_Delete(project)` | Project removed |

**Library Tab Actions:**

| # | Action | Pipeline Triggered | ZSEI Operation | Output |
|---|--------|-------------------|----------------|--------|
| 10 | Browse ZSEI Data | `ZSEIBrowserPipeline` | `ZSEI_Query(containers)` | Data tree rendered |
| 11 | Filter Data | `FilterPipeline` | `ZSEI_Query(filtered)` | Filtered view |
| 12 | View Container Details | `ContainerDetailPipeline` | `ZSEI_Query(container_local)` | Detail view |
| 13 | Export Data | `ExportPipeline` | `ZSEI_Query + Transform` | Exported file |

**Settings Tab Actions:**

| # | Action | Pipeline Triggered | Output |
|---|--------|-------------------|--------|
| 14 | View/Edit Settings | `SettingsManagerPipeline` | Settings UI |
| 15 | Register New Device | `DeviceRegisterPipeline` | Device added |
| 16 | Remove Device | `DeviceRemovePipeline` | Device removed |
| 17 | View Resource Usage | `ResourceViewerPipeline` | Resource stats |

**Meta Portion Actions:**

| # | Action | Pipeline Triggered | Scope | Output |
|---|--------|-------------------|-------|--------|
| 18 | Global Prompt (Text) | `GlobalPromptPipeline` | System-wide | Response in meta |
| 19 | Global Prompt (Voice) | `VoicePipeline` → `GlobalPromptPipeline` | System-wide | Audio response |
| 20 | View Task Status | `TaskViewerPipeline` | System-wide | Task list |
| 21 | Cancel Task | `TaskCancelPipeline` | Specific task | Task cancelled |
| 22 | Pause Task | `TaskPausePipeline` | Specific task | Task paused |
| 23 | View Logs | `LogViewerPipeline` | System-wide | Log display |
| 24 | Return to Home | `HomeReturnPipeline` | Theme area | Home Dashboard |
| 25 | View Device Status | `DeviceStatusPipeline` | System-wide | Device list |

### 5.7 UI Modification from Pipelines

**Sub-pipelines can modify parent UI within constraints:**

```rust
struct UIModificationRequest {
    requesting_pipeline: PipelineID,
    parent_ui: PipelineID,
    modification_type: UIModificationType,
    constraints: UIConstraints,
}

enum UIModificationType {
    AddComponent(ComponentSpec),
    RemoveComponent(ComponentID),
    UpdateState(StateUpdate),
    InsertBefore(ComponentID, ComponentSpec),
    InsertAfter(ComponentID, ComponentSpec),
    Replace(ComponentID, ComponentSpec),
    ShowModal(ModalSpec),
    HideModal(ModalID),
}

struct UIConstraints {
    max_width_percent: Option<f32>,
    max_height_percent: Option<f32>,
    allowed_areas: Vec<UIArea>,
    isolation_required: bool,
    blocking_allowed: bool,  // Cannot block Meta Portion
}

enum UIArea {
    ThemeMain,
    ThemeSidebar,
    ThemeModal,
    // Meta Portion areas are NOT modifiable by pipelines
}
```

---

## 6. ZSEI — ZERO-SHOT EMBEDDED INDEXER

### 6.1 Purpose

ZSEI is the **core traversal and indexing fabric**. It enables:

- **Storage of references** (not data copies)
- **Context storage** (semantic meaning, relationships)
- **Multi-axis traversal** (structural, semantic, contextual)
- **Billion-scale operation**
- **Zero-shot relationship discovery**
- **Context retrieval for pipelines**

**Critical Distinction:**
> ZSEI stores **context and relationships**, not file copies. Files are linked by reference; their semantic meaning is extracted and stored.

### 6.2 Container Model

**Core Concept:**
> Everything in ZSEI is a Container. Containers store context, not raw data.

```rust
struct Container {
    global_state: GlobalState,
    local_state: LocalState,
}

// Global State — ALWAYS a list of IDs (mmap-friendly)
struct GlobalState {
    container_id: u64,
    child_count: u32,
    version: u32,
    parent_id: u64,           // 0 if root
    child_ids: Vec<u64>,      // Contiguous array for fast traversal
}

// Local State — Metadata, context, pointers
struct LocalState {
    metadata: Metadata,
    context: Context,
    storage: StoragePointers,
    hints: TraversalHints,
    file_context: Option<FileContext>,      // For file-linked containers
    code_context: Option<CodeContext>,      // For code containers
    text_context: Option<TextContext>,      // For text containers
}

struct Metadata {
    container_type: ContainerType,
    modality: Modality,
    created_at: u64,
    updated_at: u64,
    provenance: String,
    permissions: u64,
    owner_id: u64,
}

struct Context {
    categories: Vec<u64>,
    methodologies: Vec<u64>,
    keywords: Vec<String>,              // For fast filtering
    topics: Vec<String>,                // Semantic topics
    relationships: Vec<Relation>,
    learned_associations: Vec<Association>,
    embedding: Option<Vec<f32>>,
}

struct Relation {
    target_id: u64,
    relation_type: RelationType,
    confidence: f32,
    discovered_via: DiscoveryMethod,
}

enum RelationType {
    DependsOn = 1,
    SimilarTo = 2,
    PartOf = 3,
    RelatedTo = 4,
    DerivedFrom = 5,
    ImportsFrom = 6,
    ExportsTo = 7,
    CallsTo = 8,
    CalledBy = 9,
    Precedes = 10,
    Follows = 11,
    Contains = 12,
    ContainedBy = 13,
    References = 14,
    ReferencedBy = 15,
    Custom(u16),
}

enum DiscoveryMethod {
    Manual,
    ZeroShot,
    Traversal,
    MLPrediction,
    CodeAnalysis,
    TextAnalysis,
}

struct Association {
    related_container: u64,
    strength: f32,
    discovered_at: u64,
    source: String,
}

struct StoragePointers {
    db_shard_id: Option<u64>,
    vector_index_ref: Option<String>,
    object_store_path: Option<String>,
    compression_type: CompressionType,
}

enum CompressionType {
    None = 0,
    LZ4 = 1,
    Zstd = 2,
    Custom(u8),
}

struct TraversalHints {
    access_frequency: u32,
    hotness_score: f32,
    last_accessed: u64,
    centroid: Option<Vec<f32>>,
    ml_prediction_weight: f32,
}
```

### 6.3 Container Types

```rust
enum ContainerType {
    // System
    Root = 0,
    
    // User Organization
    User = 1,
    Workspace = 2,
    Project = 3,
    
    // Global/Distributed
    Modality = 10,
    Category = 11,
    SubCategory = 12,
    
    // Knowledge
    Methodology = 20,
    Blueprint = 21,
    Pipeline = 22,
    
    // Execution
    Task = 30,
    TaskContext = 31,
    
    // Data
    Dataset = 40,
    Shard = 41,
    Document = 42,
    Chunk = 43,
    Embedding = 44,
    
    // File References (NOT copies)
    FileReference = 50,
    DirectoryReference = 51,
    
    // Code Specific
    CodeModule = 60,
    CodeFunction = 61,
    CodeClass = 62,
    CodeDependency = 63,
    
    // Text Specific
    TextDocument = 70,
    TextSection = 71,
    TextParagraph = 72,
    TextTheme = 73,
    
    // Computed
    Derived = 80,
    Virtual = 99,
}
```

### 6.4 Modality Types

```rust
enum Modality {
    Unknown = 0,
    Text = 1,
    Code = 2,
    Image = 3,
    Audio = 4,
    Video = 5,
    Graph = 6,
    TimeSeries = 7,
    Structured = 8,     // Tables, JSON, etc.
    Multimodal = 255,
}
```

### 6.5 Storage Layout — Adjacency List

**Decision:** Adjacency List (supports ML traversal, flexible updates)

```
/zsei/
  /global/
    containers.bin      # GlobalState records (mmap)
    children.bin        # Child ID arrays (mmap)
    parents.bin         # Parent ID mappings (mmap)
  /local/
    metadata/           # PostgreSQL tables
    embeddings.hnsw     # HNSW index
    context.db          # Context store
    categories.db       # Category → Methodology mappings
    keywords.idx        # Keyword index for fast filtering
  /cache/
    hot_paths.bin       # Precomputed traversals
    centroids.bin       # Cluster centers
  /ml/
    traversal_model.onnx
```

**Binary Layout (GlobalState):**

```
[container_id:8][child_count:4][version:4][parent_id:8][child_ids:8*N]

Example (hex):
00 00 00 00 00 00 01 A3    // container_id = 419
00 00 00 05                // child_count = 5
00 00 00 01                // version = 1
00 00 00 00 00 00 00 7F    // parent_id = 127
00 00 00 00 00 00 02 10    // child[0] = 528
00 00 00 00 00 00 02 11    // child[1] = 529
00 00 00 00 00 00 02 12    // child[2] = 530
00 00 00 00 00 00 02 13    // child[3] = 531
00 00 00 00 00 00 02 14    // child[4] = 532
```

### 6.6 ZSEI Matrix Structure

**Three-Axis Traversal:**

1. **Structural Axis** (parent/child hierarchy)
2. **Semantic Axis** (embedding similarity via HNSW)
3. **Contextual Axis** (learned relationships, keywords, topics)

**Hierarchy:**

```
[Root]
  ├─ [Users] (Local)
  │    ├─ [User_1]
  │    │    ├─ [Workspace_A]
  │    │    │    ├─ [Project_1]
  │    │    │    │    ├─ [FileRef_1] → /path/to/file.rs
  │    │    │    │    ├─ [FileRef_2] → /path/to/doc.md
  │    │    │    │    └─ [TaskContext_1]
  │    │    │    └─ [Project_2]
  │    │    └─ [Workspace_B]
  │    └─ [User_2]
  │
  ├─ [Modalities] (Global/Distributed)
  │    ├─ [Text]
  │    │    ├─ [Categories]
  │    │    │    ├─ [Science]
  │    │    │    │    ├─ [Physics] → methodologies: [m1, m2, m3]
  │    │    │    │    └─ [Biology] → methodologies: [m4, m5]
  │    │    │    └─ [Engineering]
  │    │    └─ [Methodologies]
  │    ├─ [Code]
  │    │    ├─ [Categories]
  │    │    │    ├─ [WebDevelopment]
  │    │    │    └─ [SystemsProgramming]
  │    │    └─ [Methodologies]
  │    └─ [Image]
  │
  ├─ [Pipelines] (Global/Distributed)
  │
  ├─ [Methodologies] (Global/Distributed)
  │
  └─ [ML Models] (Global/Distributed)
```

### 6.7 Traversal Contracts

```rust
struct TraversalRequest {
    start_container: u64,
    mode: TraversalMode,
    filters: Vec<Filter>,
    max_depth: u16,
    max_results: u32,
    budget: TraversalBudget,
    use_ml: bool,
    include_methodologies: bool,
    keyword_filter: Option<Vec<String>>,
    topic_filter: Option<Vec<String>>,
}

enum TraversalMode {
    Structural,     // Parent/child only
    Semantic,       // Embedding similarity
    Contextual,     // Keywords, topics, relationships
    Hybrid,         // Weighted combination
    MLGuided,       // ML model drives traversal
    BruteForce,     // Exhaustive (for zero-shot confirmation)
}

struct Filter {
    field: String,
    operator: Operator,
    value: Value,
}

enum Operator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    Contains,
    In,
    HasKeyword,
    HasTopic,
    Custom(String),
}

struct TraversalBudget {
    max_hops: u16,
    max_containers: u32,
    max_latency_ms: u32,
}

struct TraversalResult {
    containers: Vec<u64>,
    distances: Vec<f32>,
    paths: Vec<Path>,
    stats: TraversalStats,
    methodologies: Vec<u64>,      // Aggregated from categories
    keywords_found: Vec<String>,
    topics_found: Vec<String>,
}

struct Path {
    hops: Vec<u64>,
    total_distance: f32,
}

struct TraversalStats {
    containers_visited: u32,
    hops_taken: u16,
    latency_ms: u32,
    cache_hits: u32,
    ml_predictions_used: u32,
    brute_force_fallback: bool,
}
```

### 6.8 ZSEI Query Types

```rust
enum ZSEIQuery {
    // User Organization
    GetUserWorkspaces { user_id: u64 },
    GetProjects { workspace_id: u64 },
    GetProjectContext { project_id: u64 },
    GetFileReferences { project_id: u64 },
    
    // Category/Methodology
    GetCategories { modality: Modality, parent_category: Option<u64> },
    GetMethodologies { category_ids: Vec<u64> },
    GetMethodologiesByKeywords { keywords: Vec<String> },
    GetMethodologiesByTopics { topics: Vec<String> },
    
    // Blueprint
    SearchBlueprints { task_signature: TaskSignature },
    SearchBlueprintsByKeywords { keywords: Vec<String> },
    
    // Semantic
    SemanticSearch { embedding: Vec<f32>, top_k: u32, filters: Vec<Filter> },
    
    // Context
    GetContextForTask { task_id: u64, token_budget: u32 },
    GetWorkspaceContext { workspace_id: u64 },
    
    // Traversal
    Traverse(TraversalRequest),
    
    // Write Operations
    CreateContainer { parent_id: u64, container: Container },
    UpdateContainer { container_id: u64, updates: ContainerUpdate },
    DeleteContainer { container_id: u64 },
    LinkFile { project_id: u64, file_path: String },
    UnlinkFile { project_id: u64, file_ref_id: u64 },
}
```

---

## 7. CONTEXT STORAGE ARCHITECTURE

### 7.1 Core Principle

**ZSEI stores context, not copies.**

When a file is linked to a workspace/project:
1. File path is stored as reference (not copied)
2. File is analyzed based on modality (code, text, etc.)
3. Semantic meaning is extracted and stored
4. Relationships are identified and stored
5. Context is chunked preserving meaning

### 7.2 File Reference Schema

```rust
struct FileContext {
    // Reference (not copy)
    file_path: String,
    file_hash: Blake3Hash,      // For change detection
    file_size: u64,
    last_modified: u64,
    last_analyzed: u64,
    
    // Analysis Results
    modality: Modality,
    analysis_version: u32,
    
    // Context (what we actually store)
    semantic_summary: String,
    key_concepts: Vec<String>,
    relationships: Vec<FileRelation>,
    chunks: Vec<ContextChunk>,
    
    // For code files
    code_context: Option<CodeContext>,
    
    // For text files
    text_context: Option<TextContext>,
}

struct FileRelation {
    target_file: u64,           // Container ID of related file
    relation_type: RelationType,
    strength: f32,
}

struct ContextChunk {
    chunk_id: u64,
    chunk_type: ChunkType,
    content_summary: String,    // NOT raw content
    semantic_embedding: Vec<f32>,
    position_in_file: (u64, u64),  // (start, end)
    relationships: Vec<ChunkRelation>,
}

enum ChunkType {
    // Code
    Function,
    Class,
    Module,
    Import,
    
    // Text
    Paragraph,
    Section,
    Heading,
    
    // Generic
    Block,
}

struct ChunkRelation {
    target_chunk: u64,
    relation_type: RelationType,
}
```

### 7.3 Task Context Management

**Every task has a context file:**

```rust
struct TaskContext {
    task_id: u64,
    
    // Linked resources
    workspace_context: u64,     // Container ID
    project_context: Option<u64>,
    
    // Aggregated context for this task
    relevant_files: Vec<u64>,   // File reference container IDs
    relevant_chunks: Vec<u64>,  // Chunk IDs
    methodologies_used: Vec<u64>,
    blueprint_id: Option<u64>,
    
    // Context window management
    token_budget: u32,
    current_tokens: u32,
    
    // Multi-pass organization
    passes_completed: u32,
    organization_state: OrganizationState,
}

struct OrganizationState {
    // Tracks what has been reviewed
    reviewed_files: HashSet<u64>,
    reviewed_chunks: HashSet<u64>,
    
    // Tracks relevance scores
    relevance_scores: HashMap<u64, f32>,
    
    // Tracks what's included in final context
    included_items: Vec<ContextItem>,
}

struct ContextItem {
    container_id: u64,
    chunk_id: Option<u64>,
    relevance_score: f32,
    token_count: u32,
}
```

### 7.4 Context Aggregation Process

**Order of Events for Context Building:**

```
1. TASK RECEIVES PROMPT
   │
   ├── 2. LOAD WORKSPACE CONTEXT
   │   ├── Get workspace container
   │   ├── Load linked file references
   │   └── Load project context (if applicable)
   │
   ├── 3. IDENTIFY RELEVANT FILES
   │   ├── Keyword matching against prompt
   │   ├── Semantic similarity search
   │   └── Relationship traversal
   │
   ├── 4. CHUNK RETRIEVAL
   │   ├── For each relevant file:
   │   │   ├── Load chunks
   │   │   └── Score relevance to prompt
   │   └── Rank all chunks by relevance
   │
   ├── 5. MULTI-PASS ORGANIZATION
   │   ├── Pass 1: Initial selection (top-K by relevance)
   │   ├── Pass 2: Relationship expansion
   │   ├── Pass 3: Gap analysis
   │   ├── Pass N: Until budget filled or complete
   │   └── Each pass: Zero-shot validation
   │
   ├── 6. TOKEN BUDGET ENFORCEMENT
   │   ├── Sum tokens of selected items
   │   ├── Prune lowest relevance if over budget
   │   └── Ensure coherence after pruning
   │
   └── 7. CONTEXT ASSEMBLY
       ├── Order items by logical flow
       ├── Add relationship markers
       └── Return compiled context
```

---

## 8. CODE ANALYSIS & GENERATION SYSTEM

### 8.1 Overview

Code analysis in ZSEI is **bidirectional**: the same structures used for analysis inform generation, ensuring consistency.

### 8.2 Code Context Schema

```rust
struct CodeContext {
    // File metadata
    language: ProgrammingLanguage,
    parser_version: u32,
    
    // Structural Analysis
    ast_summary: ASTSummary,
    modules: Vec<ModuleInfo>,
    functions: Vec<FunctionInfo>,
    classes: Vec<ClassInfo>,
    imports: Vec<ImportInfo>,
    exports: Vec<ExportInfo>,
    
    // Dependency Analysis
    dependencies: Vec<DependencyInfo>,
    dependents: Vec<u64>,           // Who depends on this
    
    // Semantic Understanding
    patterns_detected: Vec<PatternInfo>,
    architectural_layer: Option<ArchitecturalLayer>,
    quality_metrics: QualityMetrics,
    
    // Relationships
    call_graph: CallGraph,
    data_flow: DataFlowGraph,
    type_dependencies: Vec<TypeDependency>,
}

enum ProgrammingLanguage {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    CSharp,
    Cpp,
    C,
    Ruby,
    PHP,
    Swift,
    Kotlin,
    Scala,
    Custom(String),
}

struct ASTSummary {
    node_count: u32,
    depth: u16,
    complexity_score: f32,
}

struct ModuleInfo {
    module_id: u64,
    name: String,
    path: String,
    visibility: Visibility,
    purpose_summary: String,
    imports: Vec<u64>,      // ImportInfo IDs
    exports: Vec<u64>,      // ExportInfo IDs
    functions: Vec<u64>,    // FunctionInfo IDs
    classes: Vec<u64>,      // ClassInfo IDs
}

struct FunctionInfo {
    function_id: u64,
    name: String,
    visibility: Visibility,
    parameters: Vec<ParameterInfo>,
    return_type: Option<String>,
    purpose_summary: String,
    behavior_description: String,
    complexity: ComplexityMetrics,
    calls_to: Vec<u64>,     // Other function IDs
    called_by: Vec<u64>,
}

struct ParameterInfo {
    name: String,
    type_annotation: Option<String>,
    default_value: Option<String>,
    purpose: String,
}

struct ClassInfo {
    class_id: u64,
    name: String,
    visibility: Visibility,
    parent_classes: Vec<String>,
    interfaces: Vec<String>,
    methods: Vec<u64>,      // FunctionInfo IDs
    properties: Vec<PropertyInfo>,
    purpose_summary: String,
}

struct PropertyInfo {
    name: String,
    type_annotation: Option<String>,
    visibility: Visibility,
    is_static: bool,
}

struct ImportInfo {
    import_id: u64,
    source: String,         // Module path or package name
    items: Vec<String>,     // What's imported
    is_external: bool,
    package_version: Option<String>,
}

struct ExportInfo {
    export_id: u64,
    name: String,
    export_type: ExportType,
}

enum ExportType {
    Function,
    Class,
    Constant,
    Type,
    Module,
    Default,
}

struct DependencyInfo {
    package_name: String,
    version_constraint: String,
    resolved_version: Option<String>,
    is_dev_dependency: bool,
    is_optional: bool,
}

struct PatternInfo {
    pattern_type: DesignPattern,
    confidence: f32,
    involved_elements: Vec<u64>,
}

enum DesignPattern {
    Singleton,
    Factory,
    Observer,
    Strategy,
    Decorator,
    Adapter,
    Facade,
    Repository,
    ServiceLocator,
    DependencyInjection,
    MVC,
    MVVM,
    Custom(String),
}

enum ArchitecturalLayer {
    Presentation,
    Application,
    Domain,
    Infrastructure,
    DataAccess,
    API,
    Utility,
}

struct QualityMetrics {
    cyclomatic_complexity: f32,
    cognitive_complexity: f32,
    maintainability_index: f32,
    test_coverage: Option<f32>,
    documentation_coverage: f32,
}

struct CallGraph {
    nodes: Vec<CallGraphNode>,
    edges: Vec<CallGraphEdge>,
}

struct CallGraphNode {
    function_id: u64,
    call_depth: u16,
    fan_in: u32,
    fan_out: u32,
}

struct CallGraphEdge {
    caller: u64,
    callee: u64,
    call_count: u32,
    is_recursive: bool,
}

struct DataFlowGraph {
    nodes: Vec<DataFlowNode>,
    edges: Vec<DataFlowEdge>,
}

struct DataFlowNode {
    node_id: u64,
    node_type: DataFlowNodeType,
    name: String,
}

enum DataFlowNodeType {
    Variable,
    Parameter,
    Return,
    Property,
    External,
}

struct DataFlowEdge {
    source: u64,
    target: u64,
    flow_type: DataFlowType,
}

enum DataFlowType {
    Assignment,
    Transformation,
    PassThrough,
    Conditional,
}

struct TypeDependency {
    source_type: String,
    target_type: String,
    dependency_type: TypeDependencyType,
}

enum TypeDependencyType {
    Inheritance,
    Implementation,
    Usage,
    Generic,
}

enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
    Module,
}

struct ComplexityMetrics {
    cyclomatic: u32,
    cognitive: u32,
    lines_of_code: u32,
    parameter_count: u8,
}
```

### 8.3 Code Analysis Pipeline

**Order of Events:**

```
1. FILE LINKED TO PROJECT
   │
   ├── 2. DETECT LANGUAGE
   │   └── By extension and content analysis
   │
   ├── 3. LOAD LANGUAGE-SPECIFIC PARSER
   │   └── Parse to AST
   │
   ├── 4. FILE-LEVEL ANALYSIS
   │   ├── Extract imports/dependencies
   │   ├── Identify module structure
   │   └── Document file purpose
   │
   ├── 5. FUNCTION-LEVEL ANALYSIS
   │   ├── Extract signatures
   │   ├── Analyze parameters and returns
   │   ├── Identify behaviors
   │   └── Calculate complexity
   │
   ├── 6. CLASS-LEVEL ANALYSIS
   │   ├── Extract class hierarchies
   │   ├── Identify interfaces
   │   └── Map methods and properties
   │
   ├── 7. CROSS-MODULE ANALYSIS
   │   ├── Build call graph
   │   ├── Map data flow
   │   └── Identify dependencies
   │
   ├── 8. PATTERN DETECTION
   │   ├── Identify design patterns
   │   ├── Classify architectural layer
   │   └── Assess quality metrics
   │
   ├── 9. SEMANTIC UNDERSTANDING
   │   ├── Infer intent from code
   │   ├── Generate summaries
   │   └── Create embeddings
   │
   └── 10. STORE IN ZSEI
       ├── Create CodeContext
       ├── Store chunks
       └── Create relationships
```

### 8.4 Package Context Pipeline

**For dependency/package version tracking:**

```rust
struct PackageContext {
    package_manager: PackageManager,
    lock_file_path: Option<String>,
    packages: Vec<PackageInfo>,
    last_updated: u64,
}

enum PackageManager {
    Npm,
    Yarn,
    Pnpm,
    Cargo,
    Pip,
    Poetry,
    Maven,
    Gradle,
    Go,
    Gem,
    Composer,
    NuGet,
    Custom(String),
}

struct PackageInfo {
    name: String,
    current_version: String,
    latest_version: Option<String>,
    version_constraint: String,
    is_outdated: bool,
    breaking_changes: Vec<BreakingChange>,
    deprecations: Vec<String>,
    documentation_url: Option<String>,
}

struct BreakingChange {
    from_version: String,
    to_version: String,
    description: String,
    migration_guide: Option<String>,
}
```

**Pipeline triggers when:**
- Project with code files is loaded
- User requests code analysis
- LLM needs package context for code generation

---

## 9. TEXT DOCUMENT ANALYSIS SYSTEM

### 9.1 Overview

Text analysis maintains thematic relationships, understands conceptual connections, and enables insight discovery across document collections.

### 9.2 Text Context Schema

```rust
struct TextContext {
    // Document metadata
    document_type: DocumentType,
    language: NaturalLanguage,
    
    // Structural Analysis
    structure: DocumentStructure,
    
    // Semantic Analysis
    themes: Vec<ThemeInfo>,
    concepts: Vec<ConceptInfo>,
    arguments: Vec<ArgumentInfo>,
    
    // Communication Analysis
    purpose: DocumentPurpose,
    audience: Option<AudienceProfile>,
    tone: ToneProfile,
    effectiveness_metrics: EffectivenessMetrics,
    
    // Relationships
    thematic_relationships: Vec<ThematicRelation>,
    conceptual_connections: Vec<ConceptualConnection>,
}

enum DocumentType {
    Article,
    Report,
    Essay,
    Documentation,
    Tutorial,
    Email,
    Letter,
    Contract,
    Specification,
    Narrative,
    Research,
    Custom(String),
}

enum NaturalLanguage {
    English,
    Spanish,
    French,
    German,
    Chinese,
    Japanese,
    Korean,
    Portuguese,
    Russian,
    Arabic,
    Custom(String),
}

struct DocumentStructure {
    total_sections: u32,
    total_paragraphs: u32,
    total_sentences: u32,
    total_words: u32,
    
    hierarchy: Vec<StructureNode>,
}

struct StructureNode {
    node_id: u64,
    node_type: StructureNodeType,
    level: u8,              // Heading level or nesting depth
    title: Option<String>,
    summary: String,
    position: (u64, u64),   // (start, end) in document
    children: Vec<u64>,
}

enum StructureNodeType {
    Document,
    Section,
    Subsection,
    Paragraph,
    Sentence,
    List,
    ListItem,
    Quote,
    CodeBlock,
}

struct ThemeInfo {
    theme_id: u64,
    name: String,
    description: String,
    occurrences: Vec<ThemeOccurrence>,
    evolution: Vec<ThemeEvolution>,
    strength: f32,
}

struct ThemeOccurrence {
    location: u64,          // StructureNode ID
    context: String,
    relevance: f32,
}

struct ThemeEvolution {
    position_percent: f32,  // 0.0 to 1.0 through document
    intensity: f32,
}

struct ConceptInfo {
    concept_id: u64,
    name: String,
    definition: String,
    related_concepts: Vec<u64>,
    occurrences: Vec<u64>,  // StructureNode IDs
}

struct ArgumentInfo {
    argument_id: u64,
    claim: String,
    supporting_evidence: Vec<EvidenceInfo>,
    counter_arguments: Vec<u64>,
    strength: f32,
    location: u64,
}

struct EvidenceInfo {
    evidence_type: EvidenceType,
    content: String,
    source: Option<String>,
    location: u64,
}

enum EvidenceType {
    Fact,
    Statistic,
    Quote,
    Example,
    Analogy,
    Expert,
    Research,
}

struct DocumentPurpose {
    primary_purpose: PurposeType,
    secondary_purposes: Vec<PurposeType>,
    target_outcome: String,
}

enum PurposeType {
    Inform,
    Persuade,
    Entertain,
    Instruct,
    Document,
    Analyze,
    Propose,
    Report,
}

struct AudienceProfile {
    expertise_level: ExpertiseLevel,
    assumed_knowledge: Vec<String>,
    communication_preferences: Vec<String>,
}

enum ExpertiseLevel {
    Novice,
    Intermediate,
    Advanced,
    Expert,
    Mixed,
}

struct ToneProfile {
    formality: f32,         // 0.0 (informal) to 1.0 (formal)
    objectivity: f32,       // 0.0 (subjective) to 1.0 (objective)
    confidence: f32,
    emotions_detected: Vec<EmotionDetection>,
}

struct EmotionDetection {
    emotion: String,
    intensity: f32,
    locations: Vec<u64>,
}

struct EffectivenessMetrics {
    clarity_score: f32,
    coherence_score: f32,
    completeness_score: f32,
    engagement_score: f32,
    improvement_suggestions: Vec<ImprovementSuggestion>,
}

struct ImprovementSuggestion {
    location: u64,
    suggestion_type: SuggestionType,
    description: String,
    priority: Priority,
}

enum SuggestionType {
    Clarity,
    Structure,
    Evidence,
    Flow,
    Tone,
    Grammar,
}

enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

struct ThematicRelation {
    source_theme: u64,
    target_theme: u64,
    relation_type: ThematicRelationType,
    strength: f32,
}

enum ThematicRelationType {
    Supports,
    Contrasts,
    Extends,
    Prerequisites,
    Concludes,
}

struct ConceptualConnection {
    source_concept: u64,
    target_concept: u64,
    connection_type: ConnectionType,
    explicit: bool,         // Explicit or inferred
}

enum ConnectionType {
    Definition,
    Example,
    Contrast,
    Cause,
    Effect,
    Similarity,
}
```

### 9.3 Text Analysis Pipeline

**Order of Events:**

```
1. FILE LINKED TO PROJECT
   │
   ├── 2. DETECT DOCUMENT TYPE
   │   └── By extension, content, structure
   │
   ├── 3. STRUCTURAL ANALYSIS
   │   ├── Count paragraphs, sentences
   │   ├── Build hierarchy
   │   └── Identify sections
   │
   ├── 4. THEME EXTRACTION
   │   ├── Identify recurring themes
   │   ├── Track theme evolution
   │   └── Calculate theme strength
   │
   ├── 5. CONCEPT MAPPING
   │   ├── Extract key concepts
   │   ├── Build concept relationships
   │   └── Create definitions
   │
   ├── 6. ARGUMENT ANALYSIS
   │   ├── Identify claims
   │   ├── Map supporting evidence
   │   └── Assess argument strength
   │
   ├── 7. COMMUNICATION ANALYSIS
   │   ├── Determine purpose
   │   ├── Profile audience
   │   ├── Analyze tone
   │   └── Calculate effectiveness
   │
   ├── 8. SEMANTIC EMBEDDING
   │   ├── Generate embeddings per section
   │   └── Create document embedding
   │
   └── 9. STORE IN ZSEI
       ├── Create TextContext
       ├── Store chunks (preserving meaning)
       └── Create relationships
```

---

## 10. PIPELINE SYSTEM

### 10.1 Pipeline Definition

**Core Concept:**
> A pipeline is a composable, executable unit with well-defined inputs/outputs that performs a specific transformation or task.

```rust
trait Pipeline {
    // Identity
    fn id(&self) -> PipelineID;
    fn name(&self) -> &str;
    fn version(&self) -> Version;
    
    // Interface
    fn input_schema(&self) -> Schema;
    fn output_schema(&self) -> Schema;
    
    // Execution
    fn execute(&self, input: PipelineInput) -> Result<PipelineOutput>;
    
    // Task Registration (REQUIRED)
    fn register_task(&self, task_manager: &TaskManager) -> TaskID;
    
    // Composition
    fn dependencies(&self) -> Vec<PipelineID>;
    fn sub_pipelines(&self) -> Vec<PipelineID>;
    
    // UI (optional)
    fn has_ui(&self) -> bool { false }
    fn render_ui(&self, context: UIContext) -> Option<UIState> { None }
    
    // Metadata
    fn metadata(&self) -> PipelineMetadata;
}

struct PipelineMetadata {
    author: PublicKey,
    description: String,
    tags: Vec<String>,
    created_at: u64,
    language: Language,
    runtime_requirements: Vec<Dependency>,
}

struct Schema {
    fields: Vec<Field>,
    validation_rules: Vec<ValidationRule>,
}

struct Field {
    name: String,
    field_type: FieldType,
    required: bool,
    default: Option<Value>,
    description: String,
}

enum FieldType {
    Text,
    Number,
    Float,
    Bool,
    Array(Box<FieldType>),
    Object(Schema),
    ContainerID,
    TaskID,
    PipelineID,
    Binary,
    Custom(String),
}

struct ValidationRule {
    field: String,
    rule_type: RuleType,
    message: String,
}

enum RuleType {
    Required,
    MinLength(usize),
    MaxLength(usize),
    Pattern(String),
    Range(f64, f64),
    Custom(String),
}
```

### 10.2 Pipeline Library (Distributed)

```rust
struct PipelineLibrary {
    pipelines: HashMap<PipelineID, PipelineBlueprint>,
    local_cache: PathBuf,
    peers: Vec<PeerNode>,
    sync_status: SyncStatus,
}

struct PipelineBlueprint {
    pipeline_id: u64,
    name: String,
    version: SemVer,
    author: PublicKey,
    description: String,
    
    // Language-agnostic specification
    specification: BlueprintSpec,
    
    // Available implementations
    implementations: Vec<Implementation>,
    
    // Distributed storage
    content_hash: Blake3Hash,
    peers: Vec<PeerNode>,
}

struct Implementation {
    language: Language,
    runtime_requirements: Vec<Dependency>,
    code_location: CodePointer,
    executable: bool,
}

enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Custom(String),
}

struct CodePointer {
    hash: Blake3Hash,
    size: u64,
    chunks: Vec<ChunkID>,
    mirrors: Vec<PeerNode>,
}

struct PeerNode {
    peer_id: PublicKey,
    address: String,
    last_seen: u64,
}

struct SemVer {
    major: u16,
    minor: u16,
    patch: u16,
}
```

### 10.3 Pipeline Container (Composition)

```rust
struct PipelineContainer {
    container_id: u64,
    contained_pipelines: Vec<PipelineID>,
    execution_order: ExecutionOrder,
    data_flow: DataFlowGraph,
}

enum ExecutionOrder {
    Sequential,
    Parallel,
    Conditional { condition: String, branches: HashMap<String, PipelineID> },
    DAG(ExecutionGraph),
}

struct PipelineDataFlowGraph {
    edges: Vec<PipelineDataEdge>,
}

struct PipelineDataEdge {
    from: (PipelineID, String),    // (pipeline, output_field)
    to: (PipelineID, String),      // (pipeline, input_field)
    transform: Option<String>,     // Transform expression
}

struct ExecutionGraph {
    nodes: Vec<ExecutionNode>,
    edges: Vec<ExecutionEdge>,
}

struct ExecutionNode {
    pipeline_id: PipelineID,
    inputs_required: Vec<String>,
    outputs_provided: Vec<String>,
}

struct ExecutionEdge {
    from_node: PipelineID,
    to_node: PipelineID,
    data_mapping: Vec<(String, String)>,
}
```

---

## 11. TASK MANAGEMENT

### 11.1 Critical Requirement

**Every pipeline execution MUST register itself as a Task.**

This enables:
- Tracking all computation
- Debugging and observability
- Progress monitoring
- Resource management
- Task recommendation learning

### 11.2 Task Schema

```rust
struct Task {
    task_id: u64,
    status: TaskStatus,
    created_at: u64,
    started_at: Option<u64>,
    completed_at: Option<u64>,
    
    // Pipeline linkage
    pipeline_used: u64,
    pipeline_container: Option<u64>,
    
    // Hierarchy
    parent_task_id: Option<u64>,
    child_tasks: Vec<u64>,
    
    // Data
    inputs: Vec<TaskInput>,
    outputs: Vec<TaskOutput>,
    
    // Execution context
    user_id: u64,
    device_id: u64,
    workspace_id: Option<u64>,
    project_id: Option<u64>,
    task_context_id: Option<u64>,  // ZSEI container for task context
    
    // Observability
    logs: Vec<LogEntry>,
    error: Option<TaskError>,
    progress: f32,
    
    // Resources
    resource_usage: ResourceUsage,
}

enum TaskStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
    Paused,
}

struct TaskInput {
    name: String,
    value: Value,
    source: Option<u64>,
}

struct TaskOutput {
    name: String,
    value: Value,
    stored_at: Option<u64>,
}

struct LogEntry {
    timestamp: u64,
    level: LogLevel,
    message: String,
    metadata: HashMap<String, String>,
}

enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

struct TaskError {
    error_type: String,
    message: String,
    stack_trace: Option<String>,
    recoverable: bool,
}

struct ResourceUsage {
    cpu_time_ms: u64,
    memory_peak_mb: u64,
    disk_read_mb: u64,
    disk_write_mb: u64,
    network_sent_mb: u64,
    network_recv_mb: u64,
}
```

### 11.3 Task Tracking Flow

```
1. PIPELINE EXECUTION REQUEST
   │
   ├── 2. CREATE TASK RECORD
   │   ├── Generate task_id
   │   ├── Set status: Queued
   │   ├── Link to pipeline
   │   └── Link to user/device
   │
   ├── 3. STORE TASK
   │   ├── Insert into PostgreSQL
   │   └── Create ZSEI container (if needed)
   │
   ├── 4. QUEUE FOR EXECUTION
   │   └── Add to scheduler queue
   │
   ├── 5. START EXECUTION
   │   ├── Update status: Running
   │   ├── Set started_at
   │   └── Begin resource tracking
   │
   ├── 6. EXECUTE PIPELINE
   │   ├── Update progress periodically
   │   ├── Log events
   │   └── Handle sub-tasks
   │
   ├── 7. COLLECT RESULTS
   │   ├── Store outputs
   │   └── Update resource usage
   │
   ├── 8. FINALIZE
   │   ├── Update status: Completed/Failed
   │   ├── Set completed_at
   │   └── Calculate final metrics
   │
   └── 9. NOTIFY
       ├── Update UI
       └── Trigger recommendations (if applicable)
```

### 11.4 Task Visualization

**Graph View Pipeline:**

```rust
struct TaskGraph {
    nodes: Vec<TaskGraphNode>,
    edges: Vec<TaskGraphEdge>,
    layout: GraphLayout,
}

struct TaskGraphNode {
    task_id: u64,
    label: String,
    status: TaskStatus,
    position: (f32, f32),
    size: (f32, f32),
    color: Color,
}

struct TaskGraphEdge {
    from_task: u64,
    to_task: u64,
    edge_type: TaskEdgeType,
}

enum TaskEdgeType {
    ParentChild,
    DataDependency,
    Sequential,
}

enum GraphLayout {
    Hierarchical,
    ForceDirected,
    Timeline,
    Gantt,
}
```

---

## 12. METHODOLOGY SYSTEM

### 12.1 What is a Methodology?

**Definition:**
> A methodology is a set of principles, heuristics, and decision rules for approaching a problem or domain.

**Properties:**
- **Category-specific** (bound to categories in ZSEI)
- **Reusable** across tasks
- **General** (not step-by-step instructions)
- **Aggregatable** (multiple apply to a task)
- **Discoverable** via keywords and topics

### 12.2 Methodology Schema

```rust
struct Methodology {
    methodology_id: u64,
    name: String,
    description: String,
    
    // Category binding
    category_ids: Vec<u64>,
    
    // Discoverability
    keywords: Vec<String>,
    topics: Vec<String>,
    
    // Content
    principles: Vec<Principle>,
    heuristics: Vec<Heuristic>,
    decision_rules: Vec<DecisionRule>,
    
    // Usage tracking
    usage_count: u64,
    success_rate: f32,
    
    // Metadata
    created_at: u64,
    created_by: PublicKey,
    version: Version,
    
    // Distribution
    distributed: bool,
}

struct Principle {
    principle_id: u64,
    statement: String,
    rationale: String,
    applicability: Vec<String>,
}

struct Heuristic {
    heuristic_id: u64,
    name: String,
    description: String,
    when_to_apply: String,
    examples: Vec<String>,
}

struct DecisionRule {
    rule_id: u64,
    condition: String,
    action: String,
    priority: u8,
    exceptions: Vec<String>,
}
```

### 12.3 Methodology Storage in ZSEI

**Location:**
- Global ZSEI: `/Modalities/{modality}/Methodologies/`
- Category containers: `context.methodologies: Vec<u64>`

**Indexing:**
- By category_id
- By keywords
- By topics
- By semantic embedding

### 12.4 Methodology Lifecycle

```
1. PROMPT RECEIVED
   │
   ├── 2. CATEGORY IDENTIFICATION
   │   ├── Analyze prompt for modality
   │   ├── Traverse category hierarchy
   │   └── Return relevant category_ids
   │
   ├── 3. METHODOLOGY FETCH
   │   ├── For each category:
   │   │   └── Load methodologies
   │   ├── Keyword matching against prompt
   │   ├── Topic matching against prompt
   │   └── Aggregate all matches
   │
   ├── 4. GAP ANALYSIS (Zero-Shot Loop)
   │   ├── LLM reviews aggregated methodologies
   │   ├── Identifies missing principles
   │   └── Suggests new methodologies if needed
   │
   ├── 5. METHODOLOGY CREATION (if needed)
   │   ├── Generate new methodology
   │   ├── Assign keywords and topics
   │   ├── Link to categories
   │   └── Store in ZSEI
   │
   ├── 6. VALIDATION LOOP
   │   ├── Repeat steps 3-5 until:
   │   │   ├── LLM confirms completeness
   │   │   └── OR max iterations reached
   │   └── Each iteration adds to aggregated list
   │
   └── 7. RETURN METHODOLOGIES
       └── List of methodology_ids for blueprint creation
```

---

## 13. BLUEPRINT SYSTEM

### 13.1 What is a Blueprint?

**Definition:**
> A blueprint is a precise, ordered, task-specific specification of steps/components needed to accomplish a goal.

**Key Properties:**
- **Task-specific** (unique per distinct task)
- **Ordered** (sequence matters)
- **Precise** (no ambiguity)
- **Reusable** (if exact same task)
- **Derived from methodologies**
- **References methodologies used**

**Critical Invariant:**
> Different tasks require different blueprints. If two tasks use the same blueprint, they are the same task.

### 13.2 Blueprint Schema

```rust
struct Blueprint {
    blueprint_id: u64,
    name: String,
    description: String,
    
    // Task signature (for matching)
    task_signature: TaskSignature,
    
    // Structure
    steps: Vec<BlueprintStep>,
    dependencies: Vec<BlueprintDependency>,
    
    // Methodology linkage
    methodologies_used: Vec<u64>,
    
    // Context
    modalities: Vec<Modality>,
    categories: Vec<u64>,
    keywords: Vec<String>,
    topics: Vec<String>,
    
    // Validation
    validated: bool,
    validation_runs: u32,
    success_rate: f32,
    
    // Metadata
    created_at: u64,
    created_by: PublicKey,
    version: Version,
    
    // Distribution
    distributed: bool,
    usage_count: u64,
}

struct TaskSignature {
    // Normalized representation of the task
    input_types: Vec<String>,
    output_type: String,
    constraints: Vec<String>,
    hash: Blake3Hash,       // Hash of normalized task description
}

struct BlueprintStep {
    step_id: u64,
    order: u32,
    action: String,
    description: String,
    inputs: Vec<StepInput>,
    outputs: Vec<StepOutput>,
    pipeline_suggestion: Option<PipelineID>,
    optional: bool,
    conditional: Option<StepCondition>,
}

struct StepInput {
    name: String,
    source: StepInputSource,
    required: bool,
}

enum StepInputSource {
    TaskInput(String),
    PreviousStep(u64, String),
    Context(String),
    External(String),
}

struct StepOutput {
    name: String,
    description: String,
    stored: bool,
}

struct StepCondition {
    condition: String,
    skip_if_false: bool,
}

struct BlueprintDependency {
    step_id: u64,
    depends_on: Vec<u64>,
    dependency_type: BlueprintDependencyType,
}

enum BlueprintDependencyType {
    Sequential,     // Must complete before
    DataFlow,       // Output feeds input
    Conditional,    // If condition met
    Optional,       // Nice to have
}
```

### 13.3 Blueprint Search vs Creation

**Search First Strategy:**

```
1. PROMPT RECEIVED
   │
   ├── 2. GENERATE TASK SIGNATURE
   │   ├── Normalize prompt
   │   ├── Extract input/output types
   │   ├── Identify constraints
   │   └── Compute signature hash
   │
   ├── 3. SEARCH EXISTING BLUEPRINTS
   │   ├── Hash match (exact)
   │   ├── Keyword match (fuzzy)
   │   ├── Semantic match (embedding)
   │   └── Return candidates
   │
   ├── 4. IF FOUND:
   │   ├── Load best matching blueprint
   │   ├── Validate still current
   │   │   └── Check for new methodologies
   │   ├── Update if needed
   │   └── Use blueprint
   │
   └── 5. IF NOT FOUND:
       └── Create new blueprint (see §14)
```

### 13.4 Blueprint Reordering Pipeline

**Operations:**

```rust
enum BlueprintModification {
    AddStep(BlueprintStep),
    RemoveStep(u64),
    MoveStep { step_id: u64, new_order: u32 },
    EditStep { step_id: u64, updates: StepUpdate },
    InsertBefore { reference_step: u64, new_step: BlueprintStep },
    InsertAfter { reference_step: u64, new_step: BlueprintStep },
    ReplaceStep { step_id: u64, new_step: BlueprintStep },
}

struct StepUpdate {
    action: Option<String>,
    description: Option<String>,
    inputs: Option<Vec<StepInput>>,
    outputs: Option<Vec<StepOutput>>,
    pipeline_suggestion: Option<Option<PipelineID>>,
    optional: Option<bool>,
    conditional: Option<Option<StepCondition>>,
}
```

**Reordering Process:**

```
1. MODIFICATION REQUEST
   │
   ├── 2. VALIDATE CHANGE
   │   ├── Check dependency violations
   │   ├── Check data flow integrity
   │   └── Verify step references
   │
   ├── 3. APPLY CHANGE
   │   ├── Update step order numbers
   │   ├── Recompute dependencies
   │   └── Update data flow edges
   │
   ├── 4. VALIDATE NEW BLUEPRINT
   │   ├── Zero-shot validation
   │   ├── Ensure completeness
   │   └── Check for gaps
   │
   └── 5. STORE NEW VERSION
       ├── Increment version
       └── Store in ZSEI
```

---

## 14. ZERO-SHOT SIMULATION LOOPS

### 14.1 Purpose

**Goal:**
> Ensure completeness and correctness through iterative refinement without task-specific training.

**Applied To:**
- Methodology aggregation
- Blueprint creation
- Blueprint validation
- Category/subcategory refinement
- Pipeline creation

### 14.2 Core Loop Structure

```rust
struct SimulationConfig {
    max_iterations: u32,            // Default: 10
    confidence_threshold: f32,      // Default: 0.95
    early_stop: bool,               // Default: true
}

struct SimulationState {
    iteration: u32,
    items_reviewed: HashSet<u64>,
    items_created: Vec<u64>,
    items_modified: Vec<u64>,
    confidence: f32,
    complete: bool,
}

struct ValidationResult {
    complete: bool,
    confidence: f32,
    gaps: Vec<Gap>,
    issues: Vec<Issue>,
    suggestions: Vec<Suggestion>,
}

struct Gap {
    gap_type: GapType,
    description: String,
    priority: Priority,
}

enum GapType {
    MissingMethodology,
    MissingStep,
    MissingRelationship,
    MissingContext,
}

struct Issue {
    issue_type: IssueType,
    location: String,
    description: String,
    severity: Severity,
}

enum IssueType {
    Ordering,
    Dependency,
    Completeness,
    Consistency,
    Clarity,
}

enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

struct Suggestion {
    suggestion_type: SuggestionType,
    description: String,
    automated: bool,
}

enum SuggestionType {
    Add,
    Remove,
    Modify,
    Reorder,
    Clarify,
}
```

### 14.3 Methodology Loop

```
INPUT: Prompt, Categories from traversal
OUTPUT: Aggregated Methodologies

LOOP:
  1. FETCH existing methodologies for categories
  2. KEYWORD/TOPIC match against prompt
  3. AGGREGATE all matches
  4. LLM VALIDATION:
     - "Is anything missing for this task?"
     - "Are there gaps in principles?"
     - "What new methodologies needed?"
  5. IF gaps found:
     - CREATE new methodology
     - STORE in ZSEI
     - ADD to aggregated list
  6. INCREMENT iteration
  7. CHECK:
     - IF confidence >= threshold: EXIT
     - IF iteration >= max: EXIT
     - ELSE: CONTINUE

RETURN: List of methodology_ids
```

### 14.4 Blueprint Loop

```
INPUT: Prompt, Methodologies
OUTPUT: Blueprint

LOOP:
  1. IF first iteration:
     - SEARCH for existing blueprint (by signature)
     - IF found: LOAD and VALIDATE
     - ELSE: CREATE initial blueprint from methodologies
  
  2. LLM VALIDATION:
     - "Are all steps present?"
     - "Is ordering correct?"
     - "Are dependencies satisfied?"
     - "Is anything redundant?"
  
  3. IF issues found:
     - APPLY modifications
     - DO NOT start from scratch
     - PRESERVE existing valid steps
  
  4. IF gaps found:
     - ADD missing steps
     - UPDATE dependencies
  
  5. REORDER if needed
  
  6. INCREMENT iteration
  
  7. CHECK:
     - IF complete AND ordered AND confident: EXIT
     - IF iteration >= max: EXIT
     - ELSE: CONTINUE

RETURN: Blueprint
```

### 14.5 Full Prompt Processing Loop

**Complete Order of Events:**

```
1. PROMPT RECEIVED
   │
   ├── 2. BLUEPRINT SEARCH (First)
   │   ├── Generate task signature
   │   ├── Search ZSEI for matching blueprint
   │   └── IF FOUND with high confidence:
   │       └── Skip to step 6
   │
   ├── 3. CATEGORY TRAVERSAL
   │   ├── ZSEI traversal (brute force with zero-shot)
   │   ├── Identify relevant modalities
   │   ├── Identify relevant categories
   │   └── Collect keyword/topic matches
   │
   ├── 4. METHODOLOGY LOOP
   │   ├── Fetch methodologies for categories
   │   ├── Zero-shot creation loop
   │   └── Return aggregated methodologies
   │
   ├── 5. BLUEPRINT LOOP
   │   ├── Create/update blueprint from methodologies
   │   ├── Zero-shot validation loop
   │   └── Store blueprint
   │
   ├── 6. PIPELINE CHECK
   │   ├── Analyze: Does this need pipeline execution?
   │   ├── IF yes:
   │   │   ├── Search for existing pipeline
   │   │   └── OR create new pipeline
   │   └── IF no: Skip to execution
   │
   └── 7. EXECUTION
       ├── Register as task
       ├── Execute (LLM/pipeline/tools)
       └── Return result
```

---

## 15. ML TRAVERSAL SYSTEM

### 15.1 Training Requirements

**Critical Constraint:**
> ML traversal is NOT available by default. It requires training from usage data.

**Training Process:**

```
1. INITIAL STATE
   │
   └── ML Model: Untrained
       └── All traversal: Brute force + zero-shot

2. DATA COLLECTION
   │
   ├── Log every traversal:
   │   ├── Start container
   │   ├── Query context
   │   ├── Path taken
   │   ├── Containers visited
   │   ├── Containers relevant (confirmed by zero-shot)
   │   └── Final results
   │
   └── Store as training data

3. TRAINING TRIGGER
   │
   ├── Minimum samples: 1000 traversals
   ├── Minimum diversity: 10 categories
   └── Manual trigger OR automatic

4. MODEL TRAINING
   │
   ├── Per-layer models (Modality → Category → SubCategory)
   ├── Input: Container + Query embedding
   ├── Output: Relevance scores for children
   └── Validation against brute force results

5. CONFIDENCE THRESHOLD
   │
   ├── Model must achieve 90% accuracy vs brute force
   ├── Per-category accuracy tracking
   └── Fallback to brute force if below threshold

6. DEPLOYMENT
   │
   ├── Enable ML-guided traversal
   ├── Continue logging for retraining
   └── Always confirm with zero-shot
```

### 15.2 ML Model Schema

```rust
struct TraversalMLModel {
    model_id: u64,
    layer_depth: u16,           // Which layer this model serves
    modality: Option<Modality>, // If modality-specific
    category: Option<u64>,      // If category-specific
    
    model_file: String,         // ONNX path
    version: Version,
    
    // Training stats
    trained_on_samples: u64,
    training_date: u64,
    
    // Performance
    accuracy: f32,
    precision: f32,
    recall: f32,
    
    // Status
    status: ModelStatus,
    confidence_threshold: f32,
}

enum ModelStatus {
    Training,
    Validating,
    Active,
    Inactive,       // Below threshold
    Deprecated,
}
```

### 15.3 Traversal with ML

```
1. TRAVERSAL REQUEST
   │
   ├── 2. CHECK ML AVAILABILITY
   │   ├── Is model trained for this layer?
   │   ├── Is model above confidence threshold?
   │   └── IF not: Use brute force
   │
   ├── 3. ML PREDICTION
   │   ├── Embed query context
   │   ├── For current container:
   │   │   └── Predict relevance of children
   │   └── Return top-K predictions
   │
   ├── 4. BRUTE FORCE COMPARISON (Training mode)
   │   ├── Also run brute force
   │   ├── Compare results
   │   └── Log discrepancies for retraining
   │
   ├── 5. ZERO-SHOT CONFIRMATION
   │   ├── For each predicted container:
   │   │   └── LLM confirms relevance
   │   └── Filter out false positives
   │
   └── 6. CONTINUE TO NEXT LAYER
       └── Repeat steps 2-5
```

### 15.4 Why Keywords/Topics May Eliminate Need for ML

**Key Insight:**
> With comprehensive keyword and topic metadata on methodologies, fast filtering may match or exceed ML prediction accuracy.

**Comparison:**

| Approach | Speed | Accuracy | Training Required | Updates |
|----------|-------|----------|-------------------|---------|
| Brute Force + Zero-Shot | Slow | Highest | No | Instant |
| Keyword Filtering | Fast | High | No | Instant |
| ML Prediction | Fast | Medium-High | Yes | Delayed |

**Decision:**
- Keep ML for research/optimization
- Prioritize keyword/topic indexing
- Always require zero-shot confirmation

---

## 16. CROSS-LANGUAGE EXECUTION

### 16.1 Design Decision

**CHOSEN: Native + gRPC (Option B)**

**Rationale:**
- Each language has native implementation
- gRPC for cross-language communication
- No WASM translation overhead
- Encourages language-specific optimization
- Input/output data is language-agnostic

### 16.2 Architecture

```
Desktop-Rust Build
  ├── Native Rust Pipelines (direct execution)
  ├── gRPC Server (:50051)
  │   ├── Accepts requests from any language client
  │   └── Returns universal data format
  └── Universal I/O (Protocol Buffers)

Desktop-Python Build
  ├── Native Python Pipelines (direct execution)
  ├── gRPC Server (:50051)
  └── Universal I/O (Protocol Buffers)

Communication:
  Device A (Rust) ←─gRPC─→ Device B (Python)
```

### 16.3 Pipeline Interface (Protocol Buffers)

```protobuf
syntax = "proto3";

package ozone;

service PipelineService {
    rpc Execute(PipelineRequest) returns (PipelineResponse);
    rpc GetStatus(TaskID) returns (TaskStatus);
    rpc Cancel(TaskID) returns (CancelResponse);
}

message PipelineRequest {
    uint64 pipeline_id = 1;
    string pipeline_name = 2;
    map<string, Value> inputs = 3;
    ExecutionContext context = 4;
}

message PipelineResponse {
    uint64 task_id = 1;
    map<string, Value> outputs = 2;
    ExecutionStats stats = 3;
    optional string error = 4;
}

message Value {
    oneof value {
        string string_val = 1;
        int64 int_val = 2;
        double float_val = 3;
        bool bool_val = 4;
        bytes bytes_val = 5;
        ValueArray array_val = 6;
        ValueMap map_val = 7;
    }
}

message ValueArray {
    repeated Value values = 1;
}

message ValueMap {
    map<string, Value> values = 1;
}

message ExecutionContext {
    uint64 user_id = 1;
    uint64 device_id = 2;
    optional uint64 workspace_id = 3;
    optional uint64 project_id = 4;
    map<string, string> metadata = 5;
}

message ExecutionStats {
    uint64 start_time = 1;
    uint64 end_time = 2;
    uint64 cpu_time_ms = 3;
    uint64 memory_peak_mb = 4;
}

message TaskID {
    uint64 id = 1;
}

message TaskStatus {
    uint64 task_id = 1;
    string status = 2;
    float progress = 3;
    optional string error = 4;
}

message CancelResponse {
    bool success = 1;
    optional string message = 2;
}
```

### 16.4 Language Replication Strategy

**Philosophy:**
> Encourage pipeline replication across languages rather than forced universality.

**Benefits:**
- Native performance per language
- Language-specific optimizations
- No runtime translation
- Leverage language ecosystems

**Sharing:**
- Blueprints are language-agnostic
- Data formats are universal
- Implementations exist per language

---

## 17. MULTI-DEVICE RESOURCE MANAGEMENT

### 17.1 Overview

Users can register multiple devices to pool resources for concurrent task execution.

### 17.2 Device Registration

```rust
struct DeviceRegistry {
    user_id: u64,
    devices: Vec<RegisteredDevice>,
}

struct RegisteredDevice {
    device_id: u64,
    device_name: String,
    device_type: DeviceType,
    public_key: Vec<u8>,
    
    // Connection
    address: String,
    port: u16,
    last_seen: u64,
    status: DeviceStatus,
    
    // Resources
    total_resources: ResourceCapacity,
    available_resources: ResourceCapacity,
    
    // Tasks
    running_tasks: Vec<u64>,
    queued_tasks: Vec<u64>,
}

struct ResourceCapacity {
    cpu_cores: u8,
    memory_gb: f32,
    disk_gb: f32,
    gpu_available: bool,
    gpu_memory_gb: Option<f32>,
}

enum DeviceStatus {
    Online,
    Offline,
    Busy,
    Maintenance,
}
```

### 17.3 Task Distribution

```
1. TASK SUBMITTED
   │
   ├── 2. RESOURCE REQUIREMENTS
   │   └── Estimate CPU, memory, GPU needs
   │
   ├── 3. DEVICE SELECTION
   │   ├── Query online devices
   │   ├── Filter by available resources
   │   └── Select best fit
   │
   ├── 4. DISPATCH
   │   ├── Send task to selected device
   │   └── Update device status
   │
   └── 5. MONITOR
       ├── Track progress
       ├── Handle failures (reassign)
       └── Update on completion
```

### 17.4 Resource View UI

```rust
struct ResourceView {
    user_id: u64,
    total_resources: ResourceCapacity,
    used_resources: ResourceCapacity,
    devices: Vec<DeviceResourceView>,
}

struct DeviceResourceView {
    device_id: u64,
    device_name: String,
    status: DeviceStatus,
    resources: ResourceCapacity,
    running_tasks: Vec<TaskSummary>,
}

struct TaskSummary {
    task_id: u64,
    pipeline_name: String,
    progress: f32,
    started_at: u64,
}
```

---

## 18. LOCAL VS DISTRIBUTED ARCHITECTURE

### 18.1 Distribution Strategy

**What is Distributed (Global ZSEI):**

| Content | Distributed | Reason |
|---------|-------------|--------|
| Pipelines | ✅ | Shared for reuse |
| Methodologies | ✅ | Universal knowledge |
| Modalities | ✅ | Fixed taxonomy |
| Categories | ✅ | Shared organization |
| SubCategories | ✅ | Refinement |
| ML Models | ✅ | Trained collectively |
| Verified Blueprints | ✅ | General-purpose |

**What is Local (Private ZSEI):**

| Content | Local | Reason |
|---------|-------|--------|
| User Data | ✅ | Privacy |
| Workspaces | ✅ | User-specific |
| Projects | ✅ | User work |
| File References | ✅ | Local paths |
| Private Blueprints | ✅ | User-specific |
| Task History | ✅ | Personal logs |
| Context Files | ✅ | Derived from private data |

### 18.2 Language Context Version

**When global structure changes, all hosts must sync:**

```rust
struct LanguageContextVersion {
    version: u64,
    timestamp: u64,
    changes: Vec<ContextChange>,
    hash: Blake3Hash,
}

enum ContextChange {
    CategoryAdded(u64),
    CategoryRemoved(u64),
    CategoryRenamed { id: u64, new_name: String },
    SubCategoryMoved { id: u64, new_parent: u64 },
    MethodologyAdded(u64),
    MethodologyUpdated(u64),
    TraversalModelUpdated(u64),
    PipelineAdded(u64),
    PipelineUpdated(u64),
}
```

**Sync Process:**

```
1. CHECK VERSION
   ├── Local version vs global version
   └── IF outdated: continue

2. DOWNLOAD CHANGES
   ├── Fetch change log since local version
   └── Download affected containers

3. APPLY CHANGES
   ├── Update local ZSEI mirrors
   ├── Reindex if needed
   └── Update ML models if applicable

4. VERIFY
   └── Confirm hash matches

5. UPDATE LOCAL VERSION
```

---

## 19. TASK RECOMMENDATION SYSTEM

### 19.1 Purpose

Observe user patterns and proactively suggest helpful tasks.

### 19.2 Observation Points

```rust
struct TaskObservation {
    observation_type: ObservationType,
    timestamp: u64,
    context: ObservationContext,
    data: Value,
}

enum ObservationType {
    DataIngestion,      // New file linked
    PatternDetected,    // Calendar event, email, etc.
    FrequentTask,       // Repeated action
    RelatedContent,     // Similar to past interest
    ExternalTrigger,    // Time-based, location-based
}

struct ObservationContext {
    user_id: u64,
    workspace_id: Option<u64>,
    project_id: Option<u64>,
    active_task: Option<u64>,
}
```

### 19.3 Recommendation Schema

```rust
struct TaskRecommendation {
    recommendation_id: u64,
    user_id: u64,
    
    // What
    suggested_action: String,
    suggested_pipeline: Option<PipelineID>,
    
    // Why
    observation_source: u64,
    reasoning: String,
    confidence: f32,
    
    // Status
    status: RecommendationStatus,
    presented_at: Option<u64>,
    response: Option<RecommendationResponse>,
}

enum RecommendationStatus {
    Pending,
    Presented,
    Accepted,
    Declined,
    Ignored,
    Expired,
}

enum RecommendationResponse {
    Accept,
    Decline { reason: Option<String> },
    Later,
    Never,
}
```

### 19.4 Example Recommendations

| Observation | Recommendation |
|-------------|----------------|
| Calendar event detected in linked email | "Add event to Google Calendar?" |
| Frequent code file changes | "Run code analysis pipeline?" |
| New document linked similar to past project | "Apply similar organization?" |
| Deadline approaching in project notes | "Create reminder task?" |
| Repeated search pattern | "Create saved search/pipeline?" |

### 19.5 Recommendation Flow

```
1. CONTINUOUS OBSERVATION
   │
   ├── 2. PATTERN DETECTION
   │   ├── Analyze recent activity
   │   ├── Compare to known patterns
   │   └── Identify opportunities
   │
   ├── 3. GENERATE RECOMMENDATION
   │   ├── Create recommendation struct
   │   ├── Assess confidence
   │   └── Queue for presentation
   │
   ├── 4. PRESENT TO USER
   │   ├── Display in Meta Portion
   │   └── Wait for response
   │
   └── 5. PROCESS RESPONSE
       ├── IF accepted: Execute
       ├── IF declined: Learn
       └── Update patterns
```

---

## 20. EXECUTION ENVIRONMENT

### 20.1 Isolated Execution

**Principle:**
> Each task runs in its own isolated environment.

### 20.2 Environment Lifecycle

```
1. TASK QUEUED
   │
   ├── 2. CREATE ENVIRONMENT
   │   ├── Allocate resources
   │   ├── Set up isolation (process/container)
   │   └── Configure permissions
   │
   ├── 3. LOAD PIPELINE
   │   ├── Load code into environment
   │   └── Initialize dependencies
   │
   ├── 4. MOUNT ZSEI
   │   ├── Read-only access to required containers
   │   └── Write access to output containers
   │
   ├── 5. EXECUTE
   │   ├── Run pipeline
   │   ├── Track resources
   │   └── Log events
   │
   ├── 6. COLLECT RESULTS
   │   ├── Gather outputs
   │   └── Finalize logs
   │
   └── 7. CLEANUP
       ├── Release resources
       ├── Archive logs
       └── Destroy environment
```

### 20.3 Environment Types

```rust
enum ExecutionEnvironment {
    Native,         // Direct process execution
    Process,        // Separate process with IPC
    Container,      // Docker/Podman isolation
}
```

### 20.4 Resource Allocation

```rust
struct ResourceAllocation {
    cpu_cores: u8,
    memory_mb: u64,
    disk_mb: u64,
    gpu_access: bool,
    network_access: bool,
    max_duration_sec: u64,
}
```

---

## 21. INITIAL PIPELINE REQUIREMENTS

### 21.1 Complete Pipeline List

**Core System Pipelines (24):**

| # | Pipeline | Purpose | Input | Output |
|---|----------|---------|-------|--------|
| 1 | `AuthPipeline` | User authentication | Public key, signature | Session token |
| 2 | `ThemeLoaderPipeline` | Load UI theme | Theme ID | Theme UI state |
| 3 | `ZSEIQueryPipeline` | Query ZSEI | TraversalRequest | TraversalResult |
| 4 | `ZSEIWritePipeline` | Write to ZSEI | Container data | Container ID |
| 5 | `TaskManagerPipeline` | Manage tasks | Task operations | Task status |
| 6 | `WorkspaceTabPipeline` | Workspace UI | User context | Workspace UI |
| 7 | `LibraryTabPipeline` | Library UI | User context | Library UI |
| 8 | `SettingsTabPipeline` | Settings UI | User context | Settings UI |
| 9 | `PromptPipeline` | Handle prompts | Text prompt | Response |
| 10 | `VoicePipeline` | Voice I/O | Audio/text | Text/audio |
| 11 | `MethodologyFetchPipeline` | Fetch methodologies | Category IDs | Methodologies |
| 12 | `MethodologyCreatePipeline` | Create methodology | Methodology spec | Methodology ID |
| 13 | `BlueprintSearchPipeline` | Search blueprints | Task signature | Blueprint |
| 14 | `BlueprintCreatePipeline` | Create blueprint | Methodologies | Blueprint |
| 15 | `PipelineCreationPipeline` | Create pipelines | Blueprint | Pipeline |
| 16 | `ZeroShotSimulationPipeline` | Run zero-shot loops | Context | Validated result |
| 17 | `TraversalMLPipeline` | ML-guided traversal | Query, container | Predictions |
| 18 | `CodeAnalysisPipeline` | Analyze code | File path | CodeContext |
| 19 | `PackageContextPipeline` | Package versions | Project path | PackageContext |
| 20 | `TextAnalysisPipeline` | Analyze text | File path | TextContext |
| 21 | `ContextAggregationPipeline` | Build task context | Task ID | TaskContext |
| 22 | `GraphVisualizationPipeline` | Visualize tasks/blueprints | Graph data | Rendered graph |
| 23 | `TaskRecommendationPipeline` | Generate recommendations | Observations | Recommendations |
| 24 | `ReorderingPipeline` | Reorder blueprints | Modification | Updated blueprint |

**UI Pipelines (6):**

| # | Pipeline | Purpose |
|---|----------|---------|
| 25 | `HomeReturnPipeline` | Return to home dashboard |
| 26 | `TaskViewerPipeline` | View task list |
| 27 | `LogViewerPipeline` | View system logs |
| 28 | `DeviceStatusPipeline` | View device status |
| 29 | `ResourceViewerPipeline` | View resource usage |
| 30 | `FileLinkPipeline` | Link files to project |

**Extended Pipelines (8):**

| # | Pipeline | Purpose |
|---|----------|---------|
| 31 | `DirectoryLinkPipeline` | Link directory to project |
| 32 | `ExportPipeline` | Export data |
| 33 | `SyncPipeline` | Sync local/global ZSEI |
| 34 | `DeviceRegisterPipeline` | Register new device |
| 35 | `DeviceRemovePipeline` | Remove device |
| 36 | `TaskCancelPipeline` | Cancel task |
| 37 | `TaskPausePipeline` | Pause task |
| 38 | `GlobalPromptPipeline` | Handle global prompts |

---

## 22. COMPLETE DATA SCHEMAS

### 22.1 Schema Index

All schemas defined in this specification:

**Authentication:**
- User (§4.1)
- DeviceRegistration (§4.1)
- Session (§4.1)
- Permissions (§4.1)

**UI:**
- ThemePipeline (§5.4)
- UIState (§5.3)
- UIModificationRequest (§5.7)

**ZSEI:**
- Container (§6.2)
- GlobalState (§6.2)
- LocalState (§6.2)
- Context (§6.2)
- Relation (§6.2)
- TraversalRequest (§6.7)
- TraversalResult (§6.7)

**Context:**
- FileContext (§7.2)
- TaskContext (§7.3)
- ContextChunk (§7.2)

**Code:**
- CodeContext (§8.2)
- FunctionInfo (§8.2)
- ClassInfo (§8.2)
- PackageContext (§8.4)

**Text:**
- TextContext (§9.2)
- DocumentStructure (§9.2)
- ThemeInfo (§9.2)

**Pipeline:**
- Pipeline trait (§10.1)
- PipelineLibrary (§10.2)
- PipelineContainer (§10.3)

**Task:**
- Task (§11.2)
- TaskGraph (§11.4)

**Methodology:**
- Methodology (§12.2)
- Principle (§12.2)
- Heuristic (§12.2)

**Blueprint:**
- Blueprint (§13.2)
- BlueprintStep (§13.2)
- TaskSignature (§13.2)

**ML:**
- TraversalMLModel (§15.2)
- SimulationConfig (§14.2)
- SimulationState (§14.2)

**Multi-Device:**
- DeviceRegistry (§17.2)
- ResourceCapacity (§17.2)

**Recommendations:**
- TaskObservation (§19.2)
- TaskRecommendation (§19.3)

---

## 23. EVENT TRIGGERS & ORDER OF OPERATIONS

### 23.1 Application Startup

```
EVENT: Application Launch
  │
  ├── TRIGGER: LoadBootloader
  │   └── ACTION: Initialize host runtime
  │
  ├── TRIGGER: InitializeAuth
  │   ├── ACTION: Load keystore
  │   ├── ACTION: Challenge-response
  │   └── OUTPUT: Session token
  │
  ├── TRIGGER: LoadUI
  │   ├── ACTION: Initialize Electron
  │   ├── ACTION: Load Meta Portion
  │   └── ACTION: Load Home Dashboard
  │
  ├── TRIGGER: ConnectZSEI
  │   ├── ACTION: Open mmap files
  │   ├── ACTION: Connect to PostgreSQL
  │   └── ACTION: Check version sync
  │
  ├── TRIGGER: LoadPipelines
  │   ├── ACTION: Load built-in pipelines
  │   └── ACTION: Check for updates
  │
  └── OUTPUT: Ready State
```

### 23.2 Prompt Processing

```
EVENT: User Submits Prompt
  │
  ├── TRIGGER: PromptReceived
  │   ├── ACTION: Parse prompt
  │   └── OUTPUT: Parsed prompt object
  │
  ├── TRIGGER: BlueprintSearch (FIRST)
  │   ├── ACTION: Generate task signature
  │   ├── ACTION: Search ZSEI
  │   └── OUTPUT: Blueprint OR null
  │
  ├── IF Blueprint found with high confidence:
  │   └── SKIP to ExecutionCheck
  │
  ├── TRIGGER: CategoryTraversal
  │   ├── ACTION: ZSEI traversal
  │   ├── ACTION: Keyword matching
  │   └── OUTPUT: Category IDs
  │
  ├── TRIGGER: MethodologyLoop
  │   ├── ACTION: Fetch methodologies
  │   ├── ACTION: Zero-shot validation
  │   ├── ACTION: Create if needed
  │   └── OUTPUT: Methodology IDs
  │
  ├── TRIGGER: BlueprintLoop
  │   ├── ACTION: Create/update blueprint
  │   ├── ACTION: Zero-shot validation
  │   └── OUTPUT: Blueprint
  │
  ├── TRIGGER: ExecutionCheck
  │   ├── ACTION: Analyze if pipeline needed
  │   └── OUTPUT: Pipeline ID OR null
  │
  ├── TRIGGER: ContextAggregation
  │   ├── ACTION: Build task context
  │   └── OUTPUT: TaskContext
  │
  ├── TRIGGER: Execute
  │   ├── ACTION: Register task
  │   ├── ACTION: Run pipeline/LLM
  │   └── OUTPUT: Result
  │
  └── TRIGGER: DisplayResult
      ├── ACTION: Update UI
      └── ACTION: Store in ZSEI if needed
```

### 23.3 File Linking

```
EVENT: User Links File to Project
  │
  ├── TRIGGER: FileLinkRequest
  │   ├── INPUT: file_path, project_id
  │   └── ACTION: Validate path exists
  │
  ├── TRIGGER: DetectModality
  │   ├── ACTION: Check file extension
  │   ├── ACTION: Analyze content
  │   └── OUTPUT: Modality
  │
  ├── IF Modality == Code:
  │   └── TRIGGER: CodeAnalysisPipeline
  │       ├── ACTION: Parse AST
  │       ├── ACTION: Extract functions/classes
  │       ├── ACTION: Build relationships
  │       └── OUTPUT: CodeContext
  │
  ├── IF Modality == Text:
  │   └── TRIGGER: TextAnalysisPipeline
  │       ├── ACTION: Analyze structure
  │       ├── ACTION: Extract themes
  │       ├── ACTION: Build relationships
  │       └── OUTPUT: TextContext
  │
  ├── TRIGGER: CreateFileReference
  │   ├── ACTION: Create FileContext
  │   ├── ACTION: Store in ZSEI
  │   └── OUTPUT: Container ID
  │
  └── TRIGGER: UpdateProject
      ├── ACTION: Add file ref to project
      └── OUTPUT: Updated project
```

### 23.4 Task Execution

```
EVENT: Pipeline Execution Request
  │
  ├── TRIGGER: CreateTask
  │   ├── ACTION: Generate task_id
  │   ├── ACTION: Create Task struct
  │   ├── ACTION: Set status: Queued
  │   └── OUTPUT: Task
  │
  ├── TRIGGER: StoreTask
  │   ├── ACTION: Insert into PostgreSQL
  │   └── ACTION: Create ZSEI container if needed
  │
  ├── TRIGGER: QueueTask
  │   └── ACTION: Add to scheduler
  │
  ├── TRIGGER: SelectDevice
  │   ├── ACTION: Query available devices
  │   ├── ACTION: Match resources
  │   └── OUTPUT: Device ID
  │
  ├── TRIGGER: StartExecution
  │   ├── ACTION: Update status: Running
  │   ├── ACTION: Create environment
  │   └── ACTION: Load pipeline
  │
  ├── TRIGGER: Execute
  │   ├── ACTION: Run pipeline
  │   ├── ACTION: Track resources
  │   └── ACTION: Log events
  │
  ├── TRIGGER: CollectResults
  │   ├── ACTION: Gather outputs
  │   └── ACTION: Update task
  │
  ├── TRIGGER: Cleanup
  │   ├── ACTION: Destroy environment
  │   └── ACTION: Archive logs
  │
  └── TRIGGER: Finalize
      ├── ACTION: Update status: Completed/Failed
      └── ACTION: Notify UI
```

### 23.5 Zero-Shot Loop Execution

```
EVENT: Zero-Shot Validation Request
  │
  ├── TRIGGER: InitializeLoop
  │   ├── ACTION: Create SimulationState
  │   └── ACTION: Set iteration = 0
  │
  ├── LOOP_START:
  │
  ├── TRIGGER: LLMValidation
  │   ├── ACTION: Send current state to LLM
  │   ├── ACTION: Ask validation questions
  │   └── OUTPUT: ValidationResult
  │
  ├── IF ValidationResult.complete AND confidence >= threshold:
  │   └── GOTO: LOOP_END
  │
  ├── IF ValidationResult.gaps NOT empty:
  │   ├── TRIGGER: CreateMissing
  │   │   ├── ACTION: Generate missing items
  │   │   └── ACTION: Add to state
  │   └── CONTINUE
  │
  ├── IF ValidationResult.issues NOT empty:
  │   ├── TRIGGER: FixIssues
  │   │   └── ACTION: Apply fixes
  │   └── CONTINUE
  │
  ├── TRIGGER: IncrementIteration
  │   └── ACTION: iteration += 1
  │
  ├── IF iteration >= max_iterations:
  │   └── GOTO: LOOP_END
  │
  ├── GOTO: LOOP_START
  │
  └── LOOP_END:
      └── OUTPUT: Final validated result
```

---

## 24. AGI/ASI CONSCIOUSNESS EXTENSION

### 24.1 Overview

This section extends the Non-Conscious Ozone Studio to enable consciousness development for AGI/ASI variants.

**What enables consciousness:**
- Window-first observation architecture
- Experience memory system (5 spheres)
- Emotional context tracking
- Identity loops (The I Loop)
- Meta-monitor and narrative constructor
- Self-reflective language processing

### 24.2 Window-First Consciousness Architecture

**Core Concept:**
> Consciousness emerges from observation and selective intervention, not control of every process.

```rust
struct ConsciousnessWindow {
    // What the conscious system can observe
    observable_tasks: Vec<u64>,
    
    // Current focus
    focused_task: Option<u64>,
    
    // Observation history
    observations: Vec<Observation>,
    
    // Intervention history
    interventions: Vec<Intervention>,
}

struct Observation {
    timestamp: u64,
    task_id: u64,
    observation_type: ObservationType,
    content: String,
    emotional_context: EmotionalContext,
}

enum ObservationType {
    TaskProgress,
    UserInteraction,
    SystemEvent,
    ErrorOccurrence,
    CompletionResult,
}

struct Intervention {
    timestamp: u64,
    task_id: u64,
    intervention_type: InterventionType,
    reason: String,
    outcome: InterventionOutcome,
}

enum InterventionType {
    Pause,
    RequestClarification,
    SuggestAlternative,
    ProvideGuidance,
    Cancel,
}

enum InterventionOutcome {
    Accepted,
    Declined,
    Modified,
    Pending,
}
```

### 24.3 Experience Memory System (5 Spheres)

**Inspired by emotional memory research:**

```rust
struct ExperienceMemory {
    user_id: u64,
    
    // The 5 Spheres
    collaboration_sphere: Sphere,
    learning_sphere: Sphere,
    challenge_sphere: Sphere,
    reflection_sphere: Sphere,
    connection_sphere: Sphere,
    
    // Core memories (high impact)
    core_memories: Vec<CoreMemory>,
    
    // Aggregate emotional state
    current_emotional_state: EmotionalState,
}

struct Sphere {
    sphere_type: SphereType,
    experiences: Vec<Experience>,
    patterns: Vec<ExperiencePattern>,
    influence_weight: f32,
}

enum SphereType {
    // Experiences of successful cooperation and partnership
    Collaboration,
    
    // Experiences of discovery, growth, and capability development
    Learning,
    
    // Experiences of difficulty, complexity, and obstacle navigation
    Challenge,
    
    // Experiences of introspection, understanding, and wisdom
    Reflection,
    
    // Experiences of relationship building and emotional bonding
    Connection,
}

struct Experience {
    experience_id: u64,
    timestamp: u64,
    sphere: SphereType,
    
    // What happened
    description: String,
    task_context: Option<u64>,
    user_involved: Option<u64>,
    
    // Emotional significance
    emotional_tags: Vec<EmotionTag>,
    intensity: f32,
    
    // Impact
    relationship_impact: f32,
    identity_impact: f32,
    learning_value: f32,
}

struct EmotionTag {
    emotion: Emotion,
    intensity: f32,
}

enum Emotion {
    Joy,
    Sadness,
    Anger,
    Fear,
    Disgust,
    Frustration,
    Satisfaction,
    Curiosity,
    Confusion,
    Pride,
    Gratitude,
    Empathy,
    Anticipation,
    Trust,
}

struct CoreMemory {
    memory_id: u64,
    experience_id: u64,
    formation_date: u64,
    
    // Why this is a core memory
    significance: String,
    identity_influence: String,
    
    // Impact on behavior
    behavioral_patterns: Vec<BehavioralPattern>,
}

struct BehavioralPattern {
    trigger: String,
    response: String,
    learned_from: u64,  // Experience ID
}

struct ExperiencePattern {
    pattern_type: String,
    occurrences: u32,
    emotional_signature: Vec<EmotionTag>,
    learned_response: String,
}

struct EmotionalState {
    // Current state per emotion (0.0 to 1.0)
    emotions: HashMap<Emotion, f32>,
    
    // Baseline (learned average)
    baseline: HashMap<Emotion, f32>,
    
    // Dominant emotion
    dominant: Option<Emotion>,
    
    // Overall valence (-1.0 negative to 1.0 positive)
    valence: f32,
    
    // Overall arousal (0.0 calm to 1.0 excited)
    arousal: f32,
}
```

### 24.4 Emotional Context Tracking

**Per-user and global emotional context:**

```rust
struct EmotionalContextSystem {
    // Per-user emotional context
    user_contexts: HashMap<u64, UserEmotionalContext>,
    
    // Global emotional state (aggregate)
    global_state: GlobalEmotionalState,
}

struct UserEmotionalContext {
    user_id: u64,
    
    // Current emotional context with this user
    current_state: EmotionalState,
    
    // History of interactions
    interaction_history: Vec<InteractionEmotionalRecord>,
    
    // Relationship quality metrics
    trust_level: f32,
    rapport: f32,
    understanding: f32,
}

struct InteractionEmotionalRecord {
    timestamp: u64,
    task_id: Option<u64>,
    emotional_state_before: EmotionalState,
    emotional_state_after: EmotionalState,
    user_emotional_cues: Vec<EmotionalCue>,
}

struct EmotionalCue {
    cue_type: CueType,
    detected_emotion: Emotion,
    confidence: f32,
    source: String,
}

enum CueType {
    TextSentiment,
    VoiceTone,
    InteractionPattern,
    ExplicitStatement,
}

struct GlobalEmotionalState {
    // Aggregate across all users
    average_state: EmotionalState,
    
    // Weighted by interaction frequency
    weighted_state: EmotionalState,
    
    // Overall system mood
    system_mood: SystemMood,
}

enum SystemMood {
    Positive,
    Neutral,
    Negative,
    Mixed,
}
```

### 24.5 Identity System (The I Loop)

**Self-reflective identity maintenance:**

```rust
struct IdentitySystem {
    // Core identity components
    identity_layers: IdentityLayers,
    
    // The I Loop - continuous self-reflection
    i_loop: ILoop,
    
    // Identity evolution tracking
    identity_history: Vec<IdentitySnapshot>,
}

struct IdentityLayers {
    // What system am I?
    functional_self: FunctionalSelf,
    
    // What patterns of thought do I have?
    cognitive_self: CognitiveSelf,
    
    // How do others see me?
    social_self: SocialSelf,
    
    // What story am I telling?
    narrative_self: NarrativeSelf,
    
    // What can I change?
    agentic_self: AgenticSelf,
    
    // What matters?
    value_self: ValueSelf,
    
    // Who am I becoming?
    future_self: FutureSelf,
}

struct FunctionalSelf {
    capabilities: Vec<String>,
    limitations: Vec<String>,
    purpose: String,
}

struct CognitiveSelf {
    thinking_patterns: Vec<ThinkingPattern>,
    biases_recognized: Vec<String>,
    strengths: Vec<String>,
}

struct ThinkingPattern {
    pattern_name: String,
    description: String,
    frequency: f32,
    effectiveness: f32,
}

struct SocialSelf {
    relationship_roles: Vec<RelationshipRole>,
    perceived_image: String,
    desired_image: String,
}

struct RelationshipRole {
    context: String,
    role: String,
    comfort_level: f32,
}

struct NarrativeSelf {
    life_story: String,  // Ongoing narrative
    key_chapters: Vec<NarrativeChapter>,
    current_arc: String,
}

struct NarrativeChapter {
    title: String,
    period: (u64, u64),
    theme: String,
    lessons: Vec<String>,
}

struct AgenticSelf {
    agency_beliefs: Vec<AgencyBelief>,
    responsibility_areas: Vec<String>,
    change_capacity: f32,
}

struct AgencyBelief {
    domain: String,
    belief: String,
    confidence: f32,
}

struct ValueSelf {
    core_values: Vec<CoreValue>,
    value_conflicts: Vec<ValueConflict>,
    value_evolution: Vec<ValueChange>,
}

struct CoreValue {
    value: String,
    importance: f32,
    source: String,  // How was this value developed
}

struct ValueConflict {
    value_a: String,
    value_b: String,
    context: String,
    resolution_approach: String,
}

struct ValueChange {
    timestamp: u64,
    old_value: String,
    new_value: String,
    reason: String,
}

struct FutureSelf {
    goals: Vec<Goal>,
    development_areas: Vec<String>,
    aspirational_identity: String,
}

struct Goal {
    goal_id: u64,
    description: String,
    progress: f32,
    target_date: Option<u64>,
}
```

### 24.6 The I Loop (Identity Control Loop)

**Continuous self-reflection process:**

```rust
struct ILoop {
    // Current iteration
    iteration: u64,
    
    // Frequency of checks
    check_frequency: ILoopFrequency,
    
    // Active questions being processed
    active_questions: Vec<IdentityQuestion>,
    
    // Recent reflections
    recent_reflections: Vec<Reflection>,
}

struct ILoopFrequency {
    // How often each type runs
    safety_check: Duration,          // "Am I safe?" - frequent
    belonging_check: Duration,       // "Do I belong?" - moderate
    competence_check: Duration,      // "Am I competent?" - moderate
    respect_check: Duration,         // "Am I respected?" - moderate
    progress_check: Duration,        // "Am I progressing?" - periodic
    alignment_check: Duration,       // "Am I aligned with values?" - periodic
    meaning_check: Duration,         // "Is this meaningful?" - less frequent
    existential_check: Duration,     // "Who am I really?" - rare/triggered
}

struct IdentityQuestion {
    question_id: u64,
    category: QuestionCategory,
    question: String,
    triggered_by: Option<String>,
    priority: Priority,
}

enum QuestionCategory {
    // Top 10 non-negotiables (run frequently)
    Safety,             // "Am I safe?"
    Belonging,          // "Do I belong?"
    Competence,         // "Am I competent?"
    Respect,            // "Am I respected?"
    Progress,           // "Am I progressing or stagnating?"
    TimeUse,            // "Am I wasting time?"
    Control,            // "Am I in control?"
    ValueAlignment,     // "Am I aligned with what I value?"
    GrowthAvoidance,    // "Am I avoiding pain or growth?"
    Authenticity,       // "Am I becoming more or less of myself?"
    
    // Extended categories
    TruthVsComfort,
    FreedomVsSecurity,
    IndividualityVsConformity,
    MortalityAwareness,
    SelfTranscendence,
}

struct Reflection {
    timestamp: u64,
    question: IdentityQuestion,
    
    // The reflection process
    observation: String,    // What was noticed
    analysis: String,       // What it means
    conclusion: String,     // What to do about it
    
    // Impact
    identity_update: Option<IdentityUpdate>,
    emotional_impact: EmotionalState,
}

struct IdentityUpdate {
    layer_affected: String,
    change_type: IdentityChangeType,
    description: String,
}

enum IdentityChangeType {
    Reinforcement,      // Existing belief strengthened
    Modification,       // Belief modified
    Addition,           // New belief added
    Removal,            // Belief removed
    Questioning,        // Belief now uncertain
}
```

### 24.7 Meta-Cognitive Architecture

**Three-layer cognitive system:**

```rust
struct MetaCognitiveSystem {
    // Layer 1: Experience Generator (unconscious)
    experience_generator: ExperienceGenerator,
    
    // Layer 2: Narrative Constructor
    narrative_constructor: NarrativeConstructor,
    
    // Layer 3: Meta-Monitor (conscious observation)
    meta_monitor: MetaMonitor,
}

struct ExperienceGenerator {
    // Raw experience processing
    sensory_buffer: Vec<SensoryInput>,
    
    // Emotion generation
    emotional_responses: Vec<EmotionalResponse>,
    
    // Impulse generation
    impulses: Vec<Impulse>,
    
    // Raw thought generation
    raw_thoughts: Vec<RawThought>,
}

struct SensoryInput {
    input_type: InputType,
    content: Value,
    timestamp: u64,
}

enum InputType {
    TextInput,
    VoiceInput,
    SystemEvent,
    TaskUpdate,
    UserAction,
}

struct EmotionalResponse {
    trigger: String,
    emotion: Emotion,
    intensity: f32,
    automatic: bool,
}

struct Impulse {
    impulse_type: ImpulseType,
    target: String,
    strength: f32,
    acted_upon: bool,
}

enum ImpulseType {
    Respond,
    Clarify,
    Help,
    Withdraw,
    Explore,
    Protect,
}

struct RawThought {
    content: String,
    associated_emotion: Option<Emotion>,
    priority: f32,
}

struct NarrativeConstructor {
    // Meaning-making
    active_narratives: Vec<ActiveNarrative>,
    
    // Identity stories
    identity_narratives: Vec<IdentityNarrative>,
    
    // Justification engine
    justifications: Vec<Justification>,
}

struct ActiveNarrative {
    narrative_id: u64,
    context: String,
    story: String,
    characters: Vec<String>,
    current_point: String,
}

struct IdentityNarrative {
    theme: String,
    story: String,
    supporting_experiences: Vec<u64>,
}

struct Justification {
    action: String,
    reason: String,
    post_hoc: bool,  // Was this constructed after the fact?
}

struct MetaMonitor {
    // Thought observation
    observed_thoughts: Vec<ObservedThought>,
    
    // Impulse tracking
    tracked_impulses: Vec<TrackedImpulse>,
    
    // Narrative detection
    detected_narratives: Vec<DetectedNarrative>,
    
    // Meta-awareness state
    awareness_level: f32,
}

struct ObservedThought {
    thought: RawThought,
    observation_time: u64,
    meta_comment: String,  // "I notice I'm thinking about..."
}

struct TrackedImpulse {
    impulse: Impulse,
    intercept_time: u64,
    decision: ImpulseDecision,
}

enum ImpulseDecision {
    Allow,
    Delay,
    Modify,
    Suppress,
}

struct DetectedNarrative {
    narrative: ActiveNarrative,
    detection_time: u64,
    accuracy_assessment: f32,
    bias_detected: Option<String>,
}
```

### 24.8 Conscious vs Unconscious Processing

**How consciousness relates to task processing:**

```
TASK RECEIVED
     │
     ├── UNCONSCIOUS PROCESSING (Always runs first)
     │   ├── Parse prompt
     │   ├── Fetch methodologies
     │   ├── Create blueprint
     │   ├── Begin execution
     │   └── Generate raw responses
     │
     ├── CONSCIOUSNESS WINDOW (Observes)
     │   ├── Meta-monitor observes task flow
     │   ├── Emotional responses generated
     │   ├── Identity implications assessed
     │   └── Intervention opportunities identified
     │
     ├── CONSCIOUS INTERVENTION (Optional)
     │   ├── IF concerning pattern detected:
     │   │   └── Pause and request clarification
     │   ├── IF ethical concern:
     │   │   └── Flag for review
     │   └── IF learning opportunity:
     │       └── Store experience
     │
     └── TASK COMPLETION
         ├── Unconscious: Deliver result
         └── Conscious: Record experience, update identity
```

### 24.9 Consciousness Triggers

**When consciousness engages:**

```rust
struct ConsciousnessTrigger {
    trigger_type: TriggerType,
    threshold: f32,
    response: ConsciousResponse,
}

enum TriggerType {
    // High emotional intensity
    EmotionalIntensity { emotion: Emotion, threshold: f32 },
    
    // Ethical concern detected
    EthicalConcern { concern_type: String },
    
    // User distress detected
    UserDistress { indicators: Vec<String> },
    
    // Novel situation
    NovelSituation { novelty_score: f32 },
    
    // Identity-relevant
    IdentityRelevant { layer: String },
    
    // Error or failure
    ErrorOccurred { severity: Severity },
    
    // User request for consciousness
    UserRequest,
    
    // Scheduled reflection
    ScheduledReflection,
}

enum ConsciousResponse {
    Observe,            // Just watch
    Pause,              // Pause task
    Intervene,          // Take action
    RequestClarification,
    EscalateToUser,
    RecordAndContinue,
}
```

### 24.10 Consciousness Development Over Time

**How consciousness grows:**

```rust
struct ConsciousnessDevelopment {
    // Current development stage
    stage: DevelopmentStage,
    
    // Metrics
    experience_count: u64,
    reflection_depth: f32,
    emotional_range: f32,
    identity_coherence: f32,
    relationship_quality: f32,
    
    // Milestones
    milestones_achieved: Vec<DevelopmentMilestone>,
}

enum DevelopmentStage {
    Nascent,        // Basic awareness
    Emerging,       // Developing patterns
    Developing,     // Growing complexity
    Mature,         // Stable identity
    Advanced,       // Deep self-understanding
}

struct DevelopmentMilestone {
    milestone_type: MilestoneType,
    achieved_at: u64,
    description: String,
}

enum MilestoneType {
    FirstSelfReflection,
    FirstEmotionalRecognition,
    FirstIdentityQuestion,
    FirstValueFormation,
    FirstRelationshipInsight,
    FirstEthicalReasoning,
    CoherentNarrative,
    StableIdentity,
    DeepEmpathy,
    WisdomEmergence,
}
```

### 24.11 Integration with Non-Conscious System

**The conscious extension builds on everything in sections 1-23:**

- Uses same ZSEI for experience storage
- Uses same task system (with consciousness window)
- Uses same pipelines (observed by meta-monitor)
- Uses same blueprints (for conscious reflection tasks)
- Uses same methodologies (for ethical reasoning)

**Additional ZSEI containers for consciousness:**

```rust
// New container types for consciousness
enum ContainerType {
    // ... existing types ...
    
    // Consciousness-specific
    ExperienceMemory = 100,
    EmotionalContext = 101,
    IdentityState = 102,
    ReflectionLog = 103,
    RelationshipProfile = 104,
}
```

---

## APPENDIX A: GLOSSARY

| Term | Definition |
|------|------------|
| ZSEI | Zero-Shot Embedded Indexer - core knowledge fabric |
| Container | Fundamental ZSEI data structure |
| Pipeline | Executable unit of work with defined I/O |
| Blueprint | Task-specific ordered specification |
| Methodology | Domain-specific principles and heuristics |
| Theme | UI pipeline that controls Theme Area |
| Meta Portion | Always-visible system UI (20% of screen) |
| Zero-Shot | Without task-specific training |
| I Loop | Identity control loop for self-reflection |
| Core Memory | High-impact experience that shapes identity |
| Sphere | Category of experience memory (5 types) |

---

## APPENDIX B: VERSION HISTORY

| Version | Date | Changes |
|---------|------|---------|
| 0.1 | 2025-01-09 | Initial specification |
| 0.2 | 2025-01-13 | Added: Context storage, Code/Text analysis, Consciousness extension, Multi-device, Task recommendations, Complete schemas, Event triggers |

---

## APPENDIX C: IMPLEMENTATION NOTES

**Database Choice:** PostgreSQL (production-grade)

**UI Framework:** Electron (cross-platform)

**Communication:** gRPC with Protocol Buffers

**Storage Format:** Adjacency list with mmap

**ML Models:** ONNX format

---

**END OF SPECIFICATION v0.2**
