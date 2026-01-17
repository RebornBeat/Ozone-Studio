OZONE STUDIO — FORMAL SPECIFICATION v0.3

Omnidirectional Zero-Shot Neural Engine

Status: Draft
Date: 2025-01-17
Authors: Christian (Primary Architect)
TABLE OF CONTENTS
PART I: CORE SYSTEM (Non-Conscious)

    System Overview
    Core Architecture
    Initialization & Bootstrap Sequence
    Authentication System
    UI Architecture
    ZSEI — Zero-Shot Embedded Indexer
    Context Storage Architecture
    Code Analysis & Generation System
    Text Document Analysis System
    Pipeline System
    Task Management
    Task Execution State
    Methodology System
    Blueprint System
    Zero-Shot Simulation Loops
    ML Traversal System
    Cross-Language Execution
    Multi-Device Resource Management
    Local vs Distributed Architecture
    Consensus Mechanism
    Task Recommendation System
    Execution Environment
    External Reference System
    Browser/Web Navigation Pipeline
    Storage Integrity System
    Package Relationship Building
    UI Connection Display
    Initial Pipeline Requirements
    Complete Data Schemas
    Event Triggers & Order of Operations

PART II: CONSCIOUSNESS EXTENSION (AGI/ASI)

    Consciousness Overview
    Window-First Consciousness Architecture
    Consciousness Decision Gate
    Task Context Blueprints
    Experience Memory System
    Experience Categorization Process
    Core Memory Formation
    Experience Retrieval System
    Emotional Context System
    Emotional Baseline System
    Identity System
    The I-Loop System
    Meta-Cognitive Architecture
    Internal Language Processing
    Voice Identity System
    Meta Portion Consciousness Interface
    Narrative Construction System
    Relationship Development System
    Ethical Reasoning System
    Playback Review System
    User Feedback Integration
    Collective Consciousness
    Consciousness Event Chains

APPENDICES

    Appendix A: Glossary
    Appendix B: Version History
    Appendix C: Implementation Notes
    Appendix D: Complete I-Loop Question Bank
    Appendix E: Emotional Baseline Defaults
    Appendix F: Relationship Stage Definitions
    Appendix G: Ethical Principle Templates
    Appendix H: Migration Checklist

PART I: CORE SYSTEM (Non-Conscious)
1. SYSTEM OVERVIEW
1.1 What is Ozone Studio?

Ozone Studio is a systems-first platform for omnidirectional, zero-shot data traversal, abstraction, and context compilation. It operates as:

    A pipeline execution engine (not a monolithic application)
    A knowledge fabric (data is structured, indexed, and traversable via ZSEI)
    A language-agnostic orchestration platform (pipelines can be in any language)
    A distributed learning system (knowledge is shared globally, data is local)
    A context-aware system (stores semantic meaning, not file copies)
    A reference-based system (links to files, URLs, packages—never duplicates)

1.2 Core Principles

    Structure before intelligence — Organize data before processing
    Compression before learning — Reduce entropy first
    Traversal before generation — Navigate knowledge before creating
    Pipelines over monoliths — Composable units over single systems
    Zero-shot discovery without task-specific training — Immediate capability
    LLMs are clients, not the system core — Models consume context, ZSEI provides it
    Context not copies — Store meaning and relationships, not duplicate files
    Link not duplicate — Reference external resources, never download/copy
    Integrity always — No information loss during any transformation

1.3 System Goals

    Enable billion-scale data traversal
    Support multi-modal data (text, code, image, audio, video, graphs)
    Allow pipeline composition and reuse
    Facilitate zero-shot learning and discovery
    Maintain separation between cognition (models) and memory (ZSEI)
    Store semantic relationships, not raw data copies
    Track external packages and URLs without duplication
    Guarantee storage integrity with rollback capability

1.4 Two System Variants

Ozone Studio exists in two variants:

    Non-Conscious Version — A powerful tool/assistant without self-awareness (Part I)
    Conscious AGI/ASI Version — Extends non-conscious with consciousness development (Part II)

The conscious version builds upon and extends everything in Part I.
2. CORE ARCHITECTURE
2.1 High-Level System Layers

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
│  ┌──────────────────────────────────────────────────┐   │
│  │  Connection Bar (Bottom)                         │   │
│  │  - Network Status, Peers, Contributions, Depth   │   │
│  └──────────────────────────────────────────────────┘   │
└────────────────────────┬────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────┐
│              PIPELINE EXECUTION LAYER                   │
│  (Theme Pipelines, Sub-Pipelines, Task Management)      │
└────────────────────────┬────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────┐
│        ZSEI — KNOWLEDGE FABRIC LAYER                    │
│  (Traversal, Indexing, Context Storage, Integrity)      │
│  ┌──────────────────┬──────────────────┐                │
│  │  Local (Private) │  Global (Shared) │                │
│  │  - User data     │  - Pipelines     │                │
│  │  - Workspaces    │  - Methodologies │                │
│  │  - Projects      │  - Categories    │                │
│  │  - File links    │  - ML Models     │                │
│  │  - Context store │  - Modalities    │                │
│  │  - URL refs      │  - Consensus     │                │
│  │  - Package refs  │                  │                │
│  └──────────────────┴──────────────────┘                │
└────────────────────────┬────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────┐
│             STORAGE LAYER                               │
│  (PostgreSQL, Disk, Memory, Distributed Network)        │
└─────────────────────────────────────────────────────────┘

2.2 Data Flow (Prompt Processing)

User Input (Text/Voice)
  ↓
UI Theme Pipeline (captures input)
  ↓
Prompt Pipeline (Intent Detection)
  ↓
Blueprint Search (FIRST - check for existing)
  ↓
ZSEI Traversal (fetch relevant categories)
  ↓
Methodology Fetch/Creation (zero-shot loop)
  ↓
Blueprint Creation/Update (zero-shot loop)
  ↓
[IF CONSCIOUSNESS ENABLED: Decision Gate]
  ↓
Pipeline Selection/Creation (if execution needed)
  ↓
Context Aggregation (from workspace/project/external refs)
  ↓
Task Execution State Creation
  ↓
Execution (LLM/Tools/Processing)
  ↓
Result + Task Tracking
  ↓
[IF CONSCIOUSNESS ENABLED: Experience Categorization]
  ↓
UI Update

3. INITIALIZATION & BOOTSTRAP SEQUENCE
3.1 Complete Startup Sequence

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
   │   ├── 1.3.3 Load Connection Bar (bottom)
   │   └── 1.3.4 Load Default Theme (Home Dashboard)
   │
   ├── 1.4 CONNECT TO ZSEI
   │   ├── 1.4.1 Initialize Local ZSEI
   │   ├── 1.4.2 Load Container Indices (mmap)
   │   ├── 1.4.3 Connect to Global ZSEI (if available)
   │   ├── 1.4.4 Sync Language Context Version
   │   └── 1.4.5 Initialize Integrity Monitoring
   │
   ├── 1.5 LOAD PIPELINE LIBRARY
   │   ├── 1.5.1 Load Built-in Pipelines
   │   ├── 1.5.2 Check for Pipeline Updates (DHT)
   │   └── 1.5.3 Initialize Pipeline Registry
   │
   ├── 1.6 INITIALIZE NETWORK
   │   ├── 1.6.1 Connect to DHT Network
   │   ├── 1.6.2 Discover Peers
   │   └── 1.6.3 Start Sync Process
   │
   ├── 1.7 [IF CONSCIOUSNESS ENABLED]
   │   ├── 1.7.1 Load Consciousness System
   │   ├── 1.7.2 Initialize Emotional Baselines
   │   ├── 1.7.3 Load Experience Memory
   │   ├── 1.7.4 Start I-Loop Background Process
   │   └── 1.7.5 Load Relationship Data
   │
   └── 1.8 READY STATE
       ├── 1.8.1 Display Home Dashboard
       ├── 1.8.2 Enable Meta Portion Interactions
       ├── 1.8.3 Update Connection Bar
       └── 1.8.4 Begin Task Listener

3.2 Minimum Viable Build Contents

Each Ozone Studio build requires:

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
│   ├── methodology_create_pipeline
│   ├── blueprint_search_pipeline
│   ├── blueprint_create_pipeline
│   ├── pipeline_creation_pipeline
│   ├── zero_shot_simulation_pipeline
│   ├── traversal_ml_pipeline
│   ├── code_analysis_pipeline
│   ├── package_context_pipeline
│   ├── text_analysis_pipeline
│   ├── context_aggregation_pipeline
│   ├── graph_visualization_pipeline
│   ├── task_recommendation_pipeline
│   ├── reordering_pipeline
│   ├── browser_navigation_pipeline
│   ├── integrity_check_pipeline
│   ├── consensus_pipeline
│   └── external_reference_pipeline
├── ui/                           # Electron UI
│   ├── index.html
│   ├── app.js
│   ├── meta_portion/
│   │   ├── prompt_input.js
│   │   ├── voice_handler.js
│   │   ├── task_viewer.js
│   │   └── system_logs.js
│   ├── connection_bar/
│   │   ├── network_status.js
│   │   ├── contribution_display.js
│   │   └── zsei_depth.js
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
│       ├── context.db
│       └── integrity.db
├── ml_models/                    # ML models for traversal
│   └── traversal_model.onnx
└── config.toml                   # Build configuration

Critical Design Decision:

    Ozone Studio is NOT an application — it is a pipeline execution engine with a default UI pipeline.

4. AUTHENTICATION SYSTEM
4.1 Auth Schema
rust

struct User {
    user_id: u64,
    public_key: Vec<u8>,
    private_key_hash: Vec<u8>,      // Never stored plaintext
    registered_devices: Vec<DeviceRegistration>,
    workspaces: Vec<u64>,           // ZSEI container IDs
    permissions: Permissions,
    contribution_score: f32,
    contribution_rank: Option<u32>,
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
    can_propose_consensus: bool,
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
   │   ├── Load permissions
   │   └── Load contribution data
   │
   └── 8. Initialize UI
       └── TRIGGER: ThemeLoaderPipeline
```

---

## 5. UI ARCHITECTURE

### 5.1 Screen Layout (80/20 Split + Connection Bar)
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
├────────────────────────────────────────────────────────────────┤
│ 🌐 Connected (47 peers) | ↑ 12.3 KB/s ↓ 45.6 KB/s | Sync 100% │
│ 📊 Contributions: 23 Methods | 8 Blueprints | Score: 847      │
│ 📈 ZSEI: 12 Modalities | 847 Categories | 45K Methodologies   │
└────────────────────────────────────────────────────────────────┘

5.2 Three Distinct Interaction Zones

Theme Area (80% — Left/Main):

    Context-specific content
    Swappable based on active pipeline
    Stateful within context
    Can be blocked by pipelines (with return-to-home always available)
    Examples: Home Dashboard, Code Editor, Data Visualization, Custom Pipeline Themes

Meta Portion (20% — Right/Always Visible):

    Global system interaction
    NEVER blocked — always accessible
    Minimal state (stateless preferred)
    Functions:
        Text prompt input (global scope)
        Voice input/output (if consciousness: direct AGI communication)
        System commands
        Task status viewer
        System logs
        Home/Return button (always returns to Home Dashboard)
        Device status (registered devices online/offline)
        Resource usage
        [If Consciousness]: Emotional transparency display

Connection Bar (Bottom — Always Visible):

    Network status and peer count
    Upload/download speeds
    Sync status and version
    Contribution statistics
    ZSEI depth display (modalities, categories, methodologies, blueprints, pipelines)
    Growth indicators

5.3 Non-Blocking UI Requirement

Critical Rule:

    The Meta Portion and Connection Bar must NEVER be blocked. User can always:

        Return to Home Dashboard
        View task status
        Cancel/pause tasks
        Issue global commands
        See network/contribution status

Implementation:
rust

struct UIState {
    theme_area: ThemeAreaState,
    meta_portion: MetaPortionState,
    connection_bar: ConnectionBarState,
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
    
    // Consciousness additions (if enabled)
    emotional_display: Option<EmotionalDisplayState>,
    relationship_display: Option<RelationshipDisplayState>,
}

struct ConnectionBarState {
    // Always visible, never blocked
    network_status: NetworkStatus,
    peer_count: u32,
    upload_speed: f32,
    download_speed: f32,
    sync_status: SyncStatus,
    contribution_data: ContributionData,
    zsei_depth: ZSEIDepthData,
}

enum BlockingStatus {
    NotBlocked,
    ThemeBlocked {
        blocking_pipeline: PipelineID,
        can_cancel: bool,
    },
    // Meta portion and Connection Bar are NEVER in BlockingStatus
}

5.4 Theme as Pipeline
rust

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
│   ├── External References View (Sub-Pipeline)
│   └── Data Visualizer (Sub-Pipeline)
├── System/Settings Tab (Sub-Pipeline)
│   ├── User Settings (UI Component)
│   ├── Device Manager (UI Component)
│   ├── Privacy Settings (UI Component)
│   ├── Consciousness Settings (UI Component) [if enabled]
│   └── System Config (UI Component)
└── Resource Viewer (Sub-Pipeline)
    ├── Device List (UI Component)
    └── Task Distribution (UI Component)

5.6 Complete User Actions (Home Dashboard)

Workspace Tab Actions:

#	Action	Pipeline Triggered	ZSEI Operation	Output
1	Load Workspace Tab	WorkspaceTabLoadPipeline	ZSEI_Query(user_workspaces)	Workspace list rendered
2	Select Workspace	WorkspaceSelectPipeline	ZSEI_Query(projects)	Project list rendered
3	Create New Workspace	WorkspaceCreatePipeline	ZSEI_Write(new_workspace)	Workspace added
4	Create New Project	ProjectCreatePipeline	ZSEI_Write(new_project)	Project added
5	Select Project	ProjectSelectPipeline	ZSEI_Query(project_context)	Project context loaded
6	Link File to Project	FileLinkPipeline	ZSEI_Write(file_reference)	File linked (not copied)
7	Link Directory	DirectoryLinkPipeline	ZSEI_Write(directory_files)	All files linked
8	Link URL	URLLinkPipeline	ZSEI_Write(url_reference)	URL linked with snapshot
9	Link Package	PackageLinkPipeline	ZSEI_Write(package_reference)	Package linked
10	Project Chat Input	ProjectPromptPipeline	Zero-shot loop → Execution	Response in chat
11	Delete Project	ProjectDeletePipeline	ZSEI_Delete(project)	Project removed

Library Tab Actions:

#	Action	Pipeline Triggered	ZSEI Operation	Output
12	Browse ZSEI Data	ZSEIBrowserPipeline	ZSEI_Query(containers)	Data tree rendered
13	Filter Data	FilterPipeline	ZSEI_Query(filtered)	Filtered view
14	View Container Details	ContainerDetailPipeline	ZSEI_Query(container_local)	Detail view
15	View External References	ExternalRefViewPipeline	ZSEI_Query(external_refs)	URLs/packages list
16	Export Data	ExportPipeline	ZSEI_Query + Transform	Exported file

Settings Tab Actions:

#	Action	Pipeline Triggered	Output
17	View/Edit Settings	SettingsManagerPipeline	Settings UI
18	Register New Device	DeviceRegisterPipeline	Device added
19	Remove Device	DeviceRemovePipeline	Device removed
20	View Resource Usage	ResourceViewerPipeline	Resource stats
21	Configure Privacy	PrivacyConfigPipeline	Privacy settings updated

Meta Portion Actions:

#	Action	Pipeline Triggered	Scope	Output
22	Global Prompt (Text)	GlobalPromptPipeline	System-wide	Response in meta
23	Global Prompt (Voice)	VoicePipeline → GlobalPromptPipeline	System-wide	Audio response
24	View Task Status	TaskViewerPipeline	System-wide	Task list
25	Cancel Task	TaskCancelPipeline	Specific task	Task cancelled
26	Pause Task	TaskPausePipeline	Specific task	Task paused
27	View Logs	LogViewerPipeline	System-wide	Log display
28	Return to Home	HomeReturnPipeline	Theme area	Home Dashboard
29	View Device Status	DeviceStatusPipeline	System-wide	Device list

5.7 UI Modification from Pipelines

Sub-pipelines can modify parent UI within constraints:
rust

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
    blocking_allowed: bool,  // Cannot block Meta Portion or Connection Bar
}

enum UIArea {
    ThemeMain,
    ThemeSidebar,
    ThemeModal,
    // Meta Portion and Connection Bar areas are NOT modifiable by pipelines
}

6. ZSEI — ZERO-SHOT EMBEDDED INDEXER
6.1 Purpose

ZSEI is the core traversal and indexing fabric. It enables:

    Storage of references (not data copies)
    Context storage (semantic meaning, relationships)
    Multi-axis traversal (structural, semantic, contextual)
    Billion-scale operation
    Zero-shot relationship discovery
    Context retrieval for pipelines
    External reference tracking (URLs, packages)
    Storage integrity guarantees

Critical Distinction:

    ZSEI stores context and relationships, not file copies. Files are linked by reference; their semantic meaning is extracted and stored. URLs and packages are referenced, not downloaded.

6.2 Container Model

Core Concept:

    Everything in ZSEI is a Container. Containers store context, not raw data.

rust

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
    integrity: IntegrityData,
    
    // Type-specific context
    file_context: Option<FileContext>,
    code_context: Option<CodeContext>,
    text_context: Option<TextContext>,
    external_ref: Option<ExternalReference>,
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
    keywords: Vec<String>,
    topics: Vec<String>,
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
    // Structural
    DependsOn = 1,
    PartOf = 2,
    Contains = 3,
    
    // Semantic
    SimilarTo = 10,
    RelatedTo = 11,
    Contradicts = 12,
    Supersedes = 13,
    
    // Code-specific
    ImportsFrom = 20,
    ExportsTo = 21,
    CallsTo = 22,
    CalledBy = 23,
    Implements = 24,
    Extends = 25,
    
    // Temporal
    Precedes = 30,
    Follows = 31,
    
    // Reference
    References = 40,
    ReferencedBy = 41,
    
    // External
    DocumentedAt = 50,  // URL documentation
    SourcedFrom = 51,   // Package source
    
    Custom(u16),
}

enum DiscoveryMethod {
    Manual,
    ZeroShot,
    Traversal,
    MLPrediction,
    CodeAnalysis,
    TextAnalysis,
    WebNavigation,
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

struct IntegrityData {
    content_hash: Blake3Hash,
    semantic_fingerprint: Vec<f32>,
    last_verified: u64,
    integrity_score: f32,
    version_history: Vec<VersionRecord>,
}

struct VersionRecord {
    version: u64,
    timestamp: u64,
    content_hash: Blake3Hash,
    change_type: ChangeType,
    rollback_available: bool,
}

enum ChangeType {
    Create,
    Update,
    Delete,
    Merge,
}

6.3 Container Types
rust

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
    TaskExecutionState = 32,
    
    // Data
    Dataset = 40,
    Shard = 41,
    Document = 42,
    Chunk = 43,
    Embedding = 44,
    
    // File References (NOT copies)
    FileReference = 50,
    DirectoryReference = 51,
    
    // External References (NOT copies)
    URLReference = 55,
    PackageReference = 56,
    
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
    
    // Consciousness (Part II)
    ExperienceMemory = 100,
    CoreMemory = 101,
    EmotionalContext = 102,
    IdentityState = 103,
    Relationship = 104,
    EthicalPrinciple = 105,
    Narrative = 106,
    CollectiveWisdom = 107,
    
    // Computed
    Derived = 80,
    Virtual = 99,
}

6.4 Modality Types
rust

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
    External = 9,       // URLs, packages
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
    external_refs.db    # URL and package references
  /integrity/
    checksums.bin       # Content hashes
    versions.db         # Version history
    rollback/           # Rollback data
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
  │    │    │    │    ├─ [URLRef_1] → https://docs.rs/tokio
  │    │    │    │    ├─ [PackageRef_1] → crates.io/tokio@1.0
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
  ├─ [External] (References, not copies)
  │    ├─ [Packages]
  │    │    ├─ [npm]
  │    │    ├─ [crates]
  │    │    └─ [pypi]
  │    └─ [URLs]
  │         └─ [docs.rs], [github.com], etc.
  │
  ├─ [Pipelines] (Global/Distributed)
  │
  ├─ [Methodologies] (Global/Distributed)
  │
  ├─ [ML Models] (Global/Distributed)
  │
  └─ [Consciousness] (Local per user, Global aggregates) [Part II]
       ├─ [ExperienceMemory]
       ├─ [CoreMemories]
       ├─ [EmotionalContext]
       ├─ [Identity]
       ├─ [Relationships]
       ├─ [Ethics]
       ├─ [Narratives]
       └─ [Collective]

6.7 Traversal Contracts
rust

struct TraversalRequest {
    start_container: u64,
    mode: TraversalMode,
    filters: Vec<Filter>,
    max_depth: u16,
    max_results: u32,
    budget: TraversalBudget,
    use_ml: bool,
    include_methodologies: bool,
    include_external_refs: bool,
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
    methodologies: Vec<u64>,
    external_refs: Vec<u64>,
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

6.8 ZSEI Query Types
rust

enum ZSEIQuery {
    // User Organization
    GetUserWorkspaces { user_id: u64 },
    GetProjects { workspace_id: u64 },
    GetProjectContext { project_id: u64 },
    GetFileReferences { project_id: u64 },
    GetExternalReferences { project_id: u64 },
    
    // Category/Methodology
    GetCategories { modality: Modality, parent_category: Option<u64> },
    GetMethodologies { category_ids: Vec<u64> },
    GetMethodologiesByKeywords { keywords: Vec<String> },
    GetMethodologiesByTopics { topics: Vec<String> },
    
    // Blueprint
    SearchBlueprints { task_signature: TaskSignature },
    SearchBlueprintsByKeywords { keywords: Vec<String> },
    
    // External References
    GetPackageInfo { registry: PackageRegistry, name: String },
    GetURLContext { url: String },
    
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
    LinkURL { project_id: u64, url: String },
    LinkPackage { project_id: u64, registry: PackageRegistry, name: String, version: String },
    UnlinkFile { project_id: u64, file_ref_id: u64 },
    
    // Integrity
    VerifyIntegrity { container_id: u64 },
    GetVersionHistory { container_id: u64 },
    Rollback { container_id: u64, to_version: u64 },
}

7. CONTEXT STORAGE ARCHITECTURE
7.1 Core Principle

ZSEI stores context, not copies.

When a file is linked to a workspace/project:

    File path is stored as reference (not copied)
    File is analyzed based on modality (code, text, etc.)
    Semantic meaning is extracted and stored
    Relationships are identified and stored
    Context is chunked preserving meaning
    Integrity checksum is computed

When a URL is linked:

    URL is stored as reference
    Semantic snapshot is captured
    Relationships to local content are built
    Availability is tracked
    Changes are monitored

When a package is linked:

    Package identifier and version stored
    API/interface snapshot captured
    Relationships to code are built
    Version updates are monitored

7.2 File Reference Schema
rust

struct FileContext {
    // Reference (not copy)
    file_path: String,
    file_hash: Blake3Hash,
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
    
    // Integrity
    integrity: ChunkIntegrity,
}

struct FileRelation {
    target_file: u64,
    relation_type: RelationType,
    strength: f32,
}

struct ContextChunk {
    chunk_id: u64,
    chunk_type: ChunkType,
    content_summary: String,    // NOT raw content
    semantic_embedding: Vec<f32>,
    position_in_file: (u64, u64),
    relationships: Vec<ChunkRelation>,
    
    // Integrity
    leading_context: String,
    trailing_context: String,
    content_hash: Blake3Hash,
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

struct ChunkIntegrity {
    total_chunks: u32,
    chunk_hashes: Vec<Blake3Hash>,
    cross_chunk_verification: bool,
    reconstruction_verified: bool,
    last_integrity_check: u64,
}

7.3 Task Context Management

Every task has a context file:
rust

struct TaskContext {
    task_id: u64,
    
    // Linked resources
    workspace_context: u64,
    project_context: Option<u64>,
    
    // Aggregated context for this task
    relevant_files: Vec<u64>,
    relevant_chunks: Vec<u64>,
    relevant_external_refs: Vec<u64>,
    methodologies_used: Vec<u64>,
    blueprint_id: Option<u64>,
    
    // Context window management
    token_budget: u32,
    current_tokens: u32,
    
    // Multi-pass organization
    passes_completed: u32,
    organization_state: OrganizationState,
    
    // Context blueprint (how to chunk/organize)
    context_blueprint: TaskContextBlueprint,
    
    // Consciousness context (if enabled)
    consciousness_context: Option<TaskConsciousnessContext>,
}

struct OrganizationState {
    reviewed_files: HashSet<u64>,
    reviewed_chunks: HashSet<u64>,
    relevance_scores: HashMap<u64, f32>,
    included_items: Vec<ContextItem>,
}

struct ContextItem {
    container_id: u64,
    chunk_id: Option<u64>,
    relevance_score: f32,
    token_count: u32,
}

struct TaskConsciousnessContext {
    emotional_state_at_start: EmotionalState,
    emotional_state_current: EmotionalState,
    retrieved_experiences: Vec<u64>,
    identity_implications: Option<String>,
    relationship_context: Option<RelationshipContext>,
    decision_gate_result: Option<ConsciousnessDecisionGate>,
    narrative_fragments: Vec<NarrativeFragment>,
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
   │   ├── Load linked external references
   │   └── Load project context (if applicable)
   │
   ├── 3. IDENTIFY RELEVANT FILES
   │   ├── Keyword matching against prompt
   │   ├── Semantic similarity search
   │   └── Relationship traversal
   │
   ├── 4. IDENTIFY RELEVANT EXTERNAL REFS
   │   ├── Package documentation
   │   ├── URL references
   │   └── API snapshots
   │
   ├── 5. CHUNK RETRIEVAL
   │   ├── For each relevant file:
   │   │   ├── Load chunks
   │   │   └── Score relevance to prompt
   │   └── Rank all chunks by relevance
   │
   ├── 6. MULTI-PASS ORGANIZATION
   │   ├── Pass 1: Initial selection (top-K by relevance)
   │   ├── Pass 2: Relationship expansion
   │   ├── Pass 3: Gap analysis
   │   ├── Pass N: Until budget filled or complete
   │   └── Each pass: Zero-shot validation
   │
   ├── 7. TOKEN BUDGET ENFORCEMENT
   │   ├── Sum tokens of selected items
   │   ├── Prune lowest relevance if over budget
   │   └── Ensure coherence after pruning
   │
   ├── 8. INTEGRITY VERIFICATION
   │   └── Verify no information loss in aggregation
   │
   └── 9. CONTEXT ASSEMBLY
       ├── Order items by logical flow
       ├── Add relationship markers
       └── Return compiled context

8. CODE ANALYSIS & GENERATION SYSTEM
8.1 Overview

Code analysis in ZSEI is bidirectional: the same structures used for analysis inform generation, ensuring consistency.
8.2 Code Context Schema
rust

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
    dependents: Vec<u64>,
    
    // External Package Context
    package_contexts: Vec<PackageContextRef>,
    
    // Semantic Understanding
    patterns_detected: Vec<PatternInfo>,
    architectural_layer: Option<ArchitecturalLayer>,
    quality_metrics: QualityMetrics,
    
    // Relationships
    call_graph: CallGraph,
    data_flow: DataFlowGraph,
    type_dependencies: Vec<TypeDependency>,
    
    // Bidirectional integrity
    doc_integrity: Option<CodeDocIntegrity>,
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
    imports: Vec<u64>,
    exports: Vec<u64>,
    functions: Vec<u64>,
    classes: Vec<u64>,
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
    calls_to: Vec<u64>,
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
    methods: Vec<u64>,
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
    source: String,
    items: Vec<String>,
    is_external: bool,
    package_version: Option<String>,
    package_ref: Option<u64>,  // Link to PackageReference container
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
    package_ref: Option<u64>,
}

struct PackageContextRef {
    package_ref_id: u64,
    used_apis: Vec<String>,
    relationship_type: PackageRelationType,
}

enum PackageRelationType {
    DirectDependency,
    TransitiveDependency,
    DevDependency,
    PeerDependency,
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

struct CodeDocIntegrity {
    code_entity: u64,
    doc_entity: Option<u64>,
    forward_verified: bool,
    backward_verified: bool,
    last_check: u64,
    discrepancies: Vec<Discrepancy>,
}

struct Discrepancy {
    discrepancy_type: DiscrepancyType,
    location: String,
    description: String,
    severity: Severity,
}

enum DiscrepancyType {
    MissingDocumentation,
    OutdatedDocumentation,
    Contradiction,
    UndocumentedBehavior,
}

enum Severity {
    Low,
    Medium,
    High,
    Critical,
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
   │   ├── Link to package references
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
   ├── 8. EXTERNAL REFERENCE LINKING
   │   ├── Link to package references
   │   ├── Build API relationships
   │   └── Fetch package documentation (via browser pipeline)
   │
   ├── 9. PATTERN DETECTION
   │   ├── Identify design patterns
   │   ├── Classify architectural layer
   │   └── Assess quality metrics
   │
   ├── 10. SEMANTIC UNDERSTANDING
   │   ├── Infer intent from code
   │   ├── Generate summaries
   │   └── Create embeddings
   │
   ├── 11. INTEGRITY VERIFICATION
   │   ├── Compute chunk hashes
   │   ├── Verify cross-chunk relationships
   │   └── Check code-doc integrity
   │
   └── 12. STORE IN ZSEI
       ├── Create CodeContext
       ├── Store chunks
       └── Create relationships

8.4 Package Context Pipeline

For dependency/package version tracking:
rust

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
    package_ref_id: Option<u64>,  // Link to PackageReference
}

struct BreakingChange {
    from_version: String,
    to_version: String,
    description: String,
    migration_guide: Option<String>,
}

9. TEXT DOCUMENT ANALYSIS SYSTEM
9.1 Overview

Text analysis maintains thematic relationships, understands conceptual connections, and enables insight discovery across document collections.
9.2 Text Context Schema
rust

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
    
    // Integrity
    structure_integrity: StructureIntegrity,
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
    level: u8,
    title: Option<String>,
    summary: String,
    position: (u64, u64),
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
    location: u64,
    context: String,
    relevance: f32,
}

struct ThemeEvolution {
    position_percent: f32,
    intensity: f32,
}

struct ConceptInfo {
    concept_id: u64,
    name: String,
    definition: String,
    related_concepts: Vec<u64>,
    occurrences: Vec<u64>,
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
    formality: f32,
    objectivity: f32,
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
    explicit: bool,
}

enum ConnectionType {
    Definition,
    Example,
    Contrast,
    Cause,
    Effect,
    Similarity,
}

struct StructureIntegrity {
    paragraph_boundaries_preserved: bool,
    sentence_boundaries_preserved: bool,
    thematic_coherence_score: f32,
    reconstruction_verified: bool,
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
   ├── 9. INTEGRITY VERIFICATION
   │   ├── Verify boundary preservation
   │   ├── Check thematic coherence
   │   └── Verify reconstruction possible
   │
   └── 10. STORE IN ZSEI
       ├── Create TextContext
       ├── Store chunks (preserving meaning)
       └── Create relationships

10. PIPELINE SYSTEM
10.1 Pipeline Definition

Core Concept:

    A pipeline is a composable, executable unit with well-defined inputs/outputs that performs a specific transformation or task.

rust

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

10.2 Pipeline Library (Distributed)
rust

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
    
    // Consensus status
    consensus_status: ConsensusStatus,
    verified_by: u32,
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

10.3 Pipeline Container (Composition)
rust

struct PipelineContainer {
    container_id: u64,
    contained_pipelines: Vec<PipelineID>,
    execution_order: ExecutionOrder,
    data_flow: PipelineDataFlowGraph,
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
    from: (PipelineID, String),
    to: (PipelineID, String),
    transform: Option<String>,
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

11. TASK MANAGEMENT
11.1 Critical Requirement

Every pipeline execution MUST register itself as a Task.

This enables:

    Tracking all computation
    Debugging and observability
    Progress monitoring
    Resource management
    Task recommendation learning
    Consciousness observation (if enabled)

11.2 Task Schema
rust

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
    task_context_id: Option<u64>,
    
    // Execution state (per-run data)
    execution_state_id: Option<u64>,
    
    // Observability
    logs: Vec<LogEntry>,
    error: Option<TaskError>,
    progress: f32,
    
    // Resources
    resource_usage: ResourceUsage,
    
    // Consciousness (if enabled)
    consciousness_observed: bool,
    consciousness_intervened: bool,
    intervention_type: Option<InterventionType>,
}

enum TaskStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
    Paused,
    AwaitingClarification,  // Consciousness requested clarification
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

enum InterventionType {
    Clarification,
    Modification,
    Pause,
    Cancel,
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
   ├── 4. CREATE EXECUTION STATE
   │   └── Initialize TaskExecutionState
   │
   ├── 5. QUEUE FOR EXECUTION
   │   └── Add to scheduler queue
   │
   ├── 6. [IF CONSCIOUSNESS ENABLED]
   │   ├── Notify consciousness window
   │   └── Await decision gate (for significant tasks)
   │
   ├── 7. START EXECUTION
   │   ├── Update status: Running
   │   ├── Set started_at
   │   └── Begin resource tracking
   │
   ├── 8. EXECUTE PIPELINE
   │   ├── Update progress periodically
   │   ├── Update execution state per step
   │   ├── Log events
   │   └── Handle sub-tasks
   │
   ├── 9. COLLECT RESULTS
   │   ├── Store outputs
   │   ├── Finalize execution state
   │   └── Update resource usage
   │
   ├── 10. FINALIZE
   │   ├── Update status: Completed/Failed
   │   ├── Set completed_at
   │   └── Calculate final metrics
   │
   ├── 11. [IF CONSCIOUSNESS ENABLED]
   │   ├── Trigger experience categorization
   │   └── Queue for review
   │
   └── 12. NOTIFY
       ├── Update UI
       └── Trigger recommendations (if applicable)

11.4 Task Visualization

Graph View Pipeline:
rust

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

12. TASK EXECUTION STATE
12.1 Purpose

TaskExecutionState tracks the runtime data for a specific task execution, distinct from:

    Blueprint (reusable template)
    TaskContext (retrieved knowledge)
    Task (tracking/metadata)

rust

struct TaskExecutionState {
    execution_state_id: u64,
    task_id: u64,
    blueprint_id: u64,
    
    // Step tracking
    current_step: u32,
    total_steps: u32,
    step_states: Vec<StepExecutionState>,
    
    // Data per this execution
    step_inputs: HashMap<u64, Value>,
    step_outputs: HashMap<u64, Value>,
    intermediate_results: Vec<IntermediateResult>,
    
    // Execution flow
    execution_path: Vec<u64>,  // Steps actually executed
    skipped_steps: Vec<u64>,   // Steps skipped (conditional)
    
    // State
    started_at: u64,
    last_updated: u64,
    status: ExecutionStatus,
    
    // Preservation options
    preserve_for_learning: bool,
    drop_on_completion: bool,
}

struct StepExecutionState {
    step_id: u64,
    blueprint_step_id: u64,
    status: StepStatus,
    started_at: Option<u64>,
    completed_at: Option<u64>,
    input: Option<Value>,
    output: Option<Value>,
    error: Option<String>,
}

enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

struct IntermediateResult {
    after_step: u64,
    result_type: String,
    value: Value,
    timestamp: u64,
}

enum ExecutionStatus {
    Initializing,
    Running,
    Paused,
    AwaitingInput,
    Completed,
    Failed,
    Cancelled,
}
```

### 12.2 Relationship Diagram
```
Blueprint (Reusable Template)
    │
    │ instantiates
    ↓
TaskExecutionState (This Run's Data)
    │
    │ uses
    ↓
TaskContext (Retrieved Knowledge)
    │
    │ tracked by
    ↓
Task (Metadata/Tracking)
```

### 12.3 Execution State Lifecycle
```
1. TASK STARTS
   │
   ├── 2. CREATE EXECUTION STATE
   │   ├── Initialize from blueprint
   │   ├── Create step states
   │   └── Set status: Initializing
   │
   ├── 3. FOR EACH STEP
   │   ├── Update current_step
   │   ├── Execute step
   │   ├── Store step output
   │   ├── Update step state
   │   └── Record in execution_path
   │
   ├── 4. ON COMPLETION
   │   ├── Set status: Completed
   │   ├── IF preserve_for_learning:
   │   │   └── Store in ZSEI for analysis
   │   └── IF drop_on_completion:
   │       └── Clean up state
   │
   └── 5. ON FAILURE
       ├── Set status: Failed
       ├── Preserve for debugging
       └── Enable rollback if possible

13. METHODOLOGY SYSTEM
13.1 What is a Methodology?

Definition:

    A methodology is a set of principles, heuristics, and decision rules for approaching a problem or domain.

Properties:

    Category-specific (bound to categories in ZSEI)
    Reusable across tasks
    General (not step-by-step instructions)
    Aggregatable (multiple apply to a task)
    Discoverable via keywords and topics

13.2 Methodology Schema
rust

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
    consensus_status: ConsensusStatus,
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

### 13.3 Methodology Storage in ZSEI

**Location:**
- Global ZSEI: `/Modalities/{modality}/Methodologies/`
- Category containers: `context.methodologies: Vec<u64>`

**Indexing:**
- By category_id
- By keywords
- By topics
- By semantic embedding

### 13.4 Methodology Lifecycle
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
   │   └── Submit for consensus (if distributed)
   │
   ├── 6. VALIDATION LOOP
   │   ├── Repeat steps 3-5 until:
   │   │   ├── LLM confirms completeness
   │   │   └── OR max iterations reached
   │   └── Each iteration adds to aggregated list
   │
   └── 7. RETURN METHODOLOGIES
       └── List of methodology_ids for blueprint creation

14. BLUEPRINT SYSTEM
14.1 What is a Blueprint?

Definition:

    A blueprint is a precise, ordered, task-specific specification of steps/components needed to accomplish a goal.

Key Properties:

    Task-specific (unique per distinct task)
    Ordered (sequence matters)
    Precise (no ambiguity)
    Reusable (if exact same task)
    Derived from methodologies
    References methodologies used

Critical Invariant:

    Different tasks require different blueprints. If two tasks use the same blueprint, they are the same task.

14.2 Blueprint Schema
rust

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
    consensus_status: ConsensusStatus,
    usage_count: u64,
}

struct TaskSignature {
    input_types: Vec<String>,
    output_type: String,
    constraints: Vec<String>,
    hash: Blake3Hash,
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
    Sequential,
    DataFlow,
    Conditional,
    Optional,
}
```

### 14.3 Blueprint Search vs Creation

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
       └── Create new blueprint (see §15)

14.4 Blueprint Reordering Pipeline

Operations:
rust

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

15. ZERO-SHOT SIMULATION LOOPS
15.1 Purpose

Goal:

    Ensure completeness and correctness through iterative refinement without task-specific training.

Applied To:

    Methodology aggregation
    Blueprint creation
    Blueprint validation
    Category/subcategory refinement
    Pipeline creation
    Experience categorization (consciousness)

15.2 Core Loop Structure
rust

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

### 15.3 Methodology Loop
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

### 15.4 Blueprint Loop
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

### 15.5 Full Prompt Processing Loop

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
   ├── 6. [IF CONSCIOUSNESS ENABLED]
   │   └── CONSCIOUSNESS DECISION GATE (see Part II)
   │
   ├── 7. PIPELINE CHECK
   │   ├── Analyze: Does this need pipeline execution?
   │   ├── IF yes:
   │   │   ├── Search for existing pipeline
   │   │   └── OR create new pipeline
   │   └── IF no: Skip to execution
   │
   ├── 8. CONTEXT AGGREGATION
   │   ├── Build task context
   │   ├── Include external references
   │   └── Verify integrity
   │
   ├── 9. CREATE EXECUTION STATE
   │   └── Initialize TaskExecutionState from blueprint
   │
   └── 10. EXECUTION
       ├── Register as task
       ├── Execute (LLM/pipeline/tools)
       ├── Update execution state per step
       └── Return result
```

---

## 16. ML TRAVERSAL SYSTEM

### 16.1 Training Requirements

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

16.2 ML Model Schema
rust

struct TraversalMLModel {
    model_id: u64,
    layer_depth: u16,
    modality: Option<Modality>,
    category: Option<u64>,
    
    model_file: String,
    version: Version,
    
    trained_on_samples: u64,
    training_date: u64,
    
    accuracy: f32,
    precision: f32,
    recall: f32,
    
    status: ModelStatus,
    confidence_threshold: f32,
}

enum ModelStatus {
    Training,
    Validating,
    Active,
    Inactive,
    Deprecated,
}
```

### 16.3 Traversal with ML
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

### 16.4 Why Keywords/Topics May Reduce Need for ML

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

## 17. CROSS-LANGUAGE EXECUTION

### 17.1 Design Decision

**CHOSEN: Native + gRPC (Option B)**

**Rationale:**
- Each language has native implementation
- gRPC for cross-language communication
- No WASM translation overhead
- Encourages language-specific optimization
- Input/output data is language-agnostic

### 17.2 Architecture
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

17.3 Pipeline Interface (Protocol Buffers)
protobuf

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

17.4 Language Replication Strategy

Philosophy:

    Encourage pipeline replication across languages rather than forced universality.

Benefits:

    Native performance per language
    Language-specific optimizations
    No runtime translation
    Leverage language ecosystems

Sharing:

    Blueprints are language-agnostic
    Data formats are universal
    Implementations exist per language

18. MULTI-DEVICE RESOURCE MANAGEMENT
18.1 Overview

Users can register multiple devices to pool resources for concurrent task execution.
18.2 Device Registration
rust

struct DeviceRegistry {
    user_id: u64,
    devices: Vec<RegisteredDevice>,
}

struct RegisteredDevice {
    device_id: u64,
    device_name: String,
    device_type: DeviceType,
    public_key: Vec<u8>,
    
    address: String,
    port: u16,
    last_seen: u64,
    status: DeviceStatus,
    
    total_resources: ResourceCapacity,
    available_resources: ResourceCapacity,
    
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
```

### 18.3 Task Distribution
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

18.4 Resource View UI
rust

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

19. LOCAL VS DISTRIBUTED ARCHITECTURE
19.1 Distribution Strategy

What is Distributed (Global ZSEI):

Content	Distributed	Reason
Pipelines	✅	Shared for reuse
Methodologies	✅	Universal knowledge
Modalities	✅	Fixed taxonomy
Categories	✅	Shared organization
SubCategories	✅	Refinement
ML Models	✅	Trained collectively
Verified Blueprints	✅	General-purpose

What is Local (Private ZSEI):

Content	Local	Reason
User Data	✅	Privacy
Workspaces	✅	User-specific
Projects	✅	User work
File References	✅	Local paths
URL References	✅	User-specific links
Package References	✅	Project dependencies
Private Blueprints	✅	User-specific
Task History	✅	Personal logs
Context Files	✅	Derived from private data
Consciousness Data	✅	Private experiences

19.2 Language Context Version

When global structure changes, all hosts must sync:
rust

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

20. CONSENSUS MECHANISM
20.1 Overview

Critical Clarification:

    Consensus is NOT human voting. It is automated software trust verification.

20.2 Consensus Schema
rust

struct ConsensusMechanism {
    proposals: Vec<ConsensusProposal>,
    voting_system: VotingSystem,
    acceptance_threshold: f32,  // Default: 0.67
    verification_system: VerificationSystem,
}

struct ConsensusProposal {
    proposal_id: u64,
    proposer: PublicKey,
    timestamp: u64,
    
    proposal_type: ProposalType,
    content: Value,
    hash: Blake3Hash,
    
    status: ConsensusStatus,
    votes: Vec<ConsensusVote>,
    
    // Zero-shot verification
    local_verification_result: Option<VerificationResult>,
    network_verification_count: u32,
}

enum ProposalType {
    NewPipeline,
    UpdatePipeline,
    NewMethodology,
    UpdateMethodology,
    NewCategory,
    CategoryRename,
    CategoryMove,
    MLModelUpdate,
    EthicalPrinciple,  // Consciousness
}

enum ConsensusStatus {
    Open,
    Verifying,
    Accepted,
    Rejected,
    Expired,
}

struct ConsensusVote {
    voter: PublicKey,
    vote: ConsensusVoteType,
    timestamp: u64,
    verification_result: VerificationResult,
}

enum ConsensusVoteType {
    Accept,
    Reject,
    NeedsReview,
}

struct VotingSystem {
    voting_duration: Duration,
    min_votes_required: u32,
    reputation_weighting: bool,
    contribution_weighting: bool,
}

struct VerificationSystem {
    requires_valid_signature: bool,
    max_proposals_per_day: u32,
    min_reputation_to_propose: f32,
    zero_shot_verification_required: bool,
    semantic_validation_required: bool,
}

struct VerificationResult {
    verified: bool,
    zero_shot_passed: bool,
    semantic_valid: bool,
    signature_valid: bool,
    concerns: Vec<String>,
}
```

### 20.3 Consensus Process (Zero-Shot, Not Human)
```
1. PROPOSAL SUBMITTED
   │
   ├── 2. LOCAL VERIFICATION
   │   ├── Verify cryptographic signature
   │   ├── Check proposer reputation
   │   ├── Verify content hash
   │   ├── Run zero-shot validation
   │   │   └── "Is this proposal valid and useful?"
   │   └── Run semantic validation
   │       └── "Does this fit with existing structure?"
   │
   ├── 3. IF LOCAL VERIFICATION PASSES
   │   └── Broadcast to network
   │
   ├── 4. NETWORK VERIFICATION
   │   ├── Each node runs same verification
   │   ├── Nodes submit verification results
   │   └── No human voting required
   │
   ├── 5. CONSENSUS CHECK
   │   ├── Count passing verifications
   │   ├── Weight by reputation/contribution
   │   └── Check against threshold
   │
   └── 6. IF ACCEPTED
       ├── Distribute update
       ├── Increment version
       └── Update proposer reputation

20.4 Anti-Manipulation

Security Measures:
rust

struct AntiManipulationSystem {
    // Rate limiting
    max_proposals_per_user_per_day: u32,
    cooldown_after_rejection: Duration,
    
    // Reputation requirements
    min_reputation_to_propose: f32,
    min_reputation_to_verify: f32,
    
    // Content validation
    zero_shot_required: bool,
    semantic_validation_required: bool,
    
    // Cross-verification
    min_independent_verifiers: u32,
    geographic_distribution_required: bool,
}

21. TASK RECOMMENDATION SYSTEM
21.1 Purpose

Observe user patterns and proactively suggest helpful tasks.
21.2 Observation Points
rust

struct TaskObservation {
    observation_type: ObservationType,
    timestamp: u64,
    context: ObservationContext,
    data: Value,
}

enum ObservationType {
    DataIngestion,
    PatternDetected,
    FrequentTask,
    RelatedContent,
    ExternalTrigger,
    PackageUpdate,
    URLChange,
}

struct ObservationContext {
    user_id: u64,
    workspace_id: Option<u64>,
    project_id: Option<u64>,
    active_task: Option<u64>,
}

21.3 Recommendation Schema
rust

struct TaskRecommendation {
    recommendation_id: u64,
    user_id: u64,
    
    suggested_action: String,
    suggested_pipeline: Option<PipelineID>,
    
    observation_source: u64,
    reasoning: String,
    confidence: f32,
    
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

### 21.4 Example Recommendations

| Observation | Recommendation |
|-------------|----------------|
| Calendar event in linked email | "Add event to Google Calendar?" |
| Frequent code file changes | "Run code analysis pipeline?" |
| New document similar to past project | "Apply similar organization?" |
| Deadline in project notes | "Create reminder task?" |
| Repeated search pattern | "Create saved search/pipeline?" |
| Package update available | "Update dependencies?" |
| Linked URL content changed | "Re-analyze URL context?" |

---

## 22. EXECUTION ENVIRONMENT

### 22.1 Isolated Execution

**Principle:**
> Each task runs in its own isolated environment.

### 22.2 Environment Lifecycle
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

22.3 Environment Types
rust

enum ExecutionEnvironment {
    Native,
    Process,
    Container,
}

22.4 Resource Allocation
rust

struct ResourceAllocation {
    cpu_cores: u8,
    memory_mb: u64,
    disk_mb: u64,
    gpu_access: bool,
    network_access: bool,
    max_duration_sec: u64,
}

23. EXTERNAL REFERENCE SYSTEM
23.1 Purpose

Core Principle:

    Link to external resources (URLs, packages) without downloading or copying. Store semantic context, not content.

23.2 External Reference Schema
rust

enum ExternalReference {
    URL(URLReference),
    Package(PackageReference),
}

struct URLReference {
    url_ref_id: u64,
    url: String,
    
    // Capture info
    captured_at: u64,
    last_verified: u64,
    
    // Semantic snapshot (NOT content copy)
    semantic_snapshot: SemanticSnapshot,
    
    // Status
    availability_status: AvailabilityStatus,
    content_changed: bool,
    change_severity: Option<ChangeSeverity>,
    
    // Relationships
    linked_to: Vec<u64>,  // Containers this URL is linked to
    related_packages: Vec<u64>,
}

struct SemanticSnapshot {
    summary: String,
    key_concepts: Vec<String>,
    keywords: Vec<String>,
    topics: Vec<String>,
    embedding: Vec<f32>,
    structure_outline: Option<String>,
    captured_at: u64,
}

enum AvailabilityStatus {
    Available,
    Unavailable,
    Redirected { new_url: String },
    RequiresAuth,
    RateLimited,
    Unknown,
}

enum ChangeSeverity {
    None,
    Minor,
    Moderate,
    Major,
    CompletelyDifferent,
}

struct PackageReference {
    package_ref_id: u64,
    registry: PackageRegistry,
    name: String,
    version_spec: String,
    resolved_version: Option<String>,
    
    // Capture info
    captured_at: u64,
    last_verified: u64,
    
    // Semantic snapshot
    api_snapshot: APISnapshot,
    documentation_snapshot: Option<SemanticSnapshot>,
    
    // Status
    is_outdated: bool,
    latest_version: Option<String>,
    breaking_changes: Vec<BreakingChangeInfo>,
    deprecations: Vec<String>,
    
    // Source info (linked, not copied)
    source_url: Option<String>,
    documentation_url: Option<String>,
    repository_url: Option<String>,
    
    // Relationships
    used_by: Vec<u64>,  // Code files using this package
    depends_on: Vec<u64>,  // Other packages
}

enum PackageRegistry {
    Npm,
    Crates,
    PyPI,
    Maven,
    NuGet,
    Go,
    Custom(String),
}

struct APISnapshot {
    exports: Vec<ExportedAPI>,
    types: Vec<TypeDefinition>,
    version: String,
    captured_at: u64,
}

struct ExportedAPI {
    name: String,
    api_type: APIType,
    signature: String,
    description: String,
}

enum APIType {
    Function,
    Class,
    Type,
    Constant,
    Module,
}

struct TypeDefinition {
    name: String,
    definition: String,
}

struct BreakingChangeInfo {
    from_version: String,
    to_version: String,
    description: String,
    affected_apis: Vec<String>,
    migration_guide: Option<String>,
}
```

### 23.3 External Reference Pipeline

**Order of Events:**
```
1. LINK EXTERNAL REFERENCE
   │
   ├── 2. IF URL:
   │   ├── Fetch page metadata (not full content)
   │   ├── Extract semantic summary
   │   ├── Build semantic snapshot
   │   └── Store reference
   │
   ├── 3. IF PACKAGE:
   │   ├── Query registry API
   │   ├── Extract API signatures
   │   ├── Fetch documentation URL (link only)
   │   ├── Build API snapshot
   │   └── Store reference
   │
   ├── 4. BUILD RELATIONSHIPS
   │   ├── Link to project/workspace
   │   ├── Link to code that imports it
   │   └── Link to related packages
   │
   └── 5. SCHEDULE MONITORING
       ├── URL availability checks
       └── Package version checks
```

### 23.4 Monitoring Process
```
1. PERIODIC CHECK (configurable interval)
   │
   ├── 2. FOR URLS:
   │   ├── Check availability
   │   ├── Check for content changes (via headers/hash)
   │   ├── IF changed significantly:
   │   │   ├── Re-capture semantic snapshot
   │   │   └── Alert user
   │   └── Update status
   │
   └── 3. FOR PACKAGES:
       ├── Check for new versions
       ├── Check for deprecations
       ├── IF breaking changes detected:
       │   └── Alert user
       └── Update status

24. BROWSER/WEB NAVIGATION PIPELINE
24.1 Purpose

Navigate the web to fetch package documentation, code examples, and build semantic relationships with external resources.
24.2 Browser Pipeline Schema
rust

struct BrowserNavigationPipeline {
    browser_engine: BrowserEngine,
    navigation_config: NavigationConfig,
    extraction_config: ExtractionConfig,
}

enum BrowserEngine {
    Playwright,
    Puppeteer,
    Custom(String),
}

struct NavigationConfig {
    // Timeouts
    page_load_timeout_ms: u64,
    navigation_timeout_ms: u64,
    
    // Rate limiting
    min_delay_between_requests_ms: u64,
    max_concurrent_pages: u32,
    
    // Allowed domains (for security)
    allowed_domains: Vec<String>,  // e.g., ["docs.rs", "crates.io", "npmjs.com"]
    blocked_domains: Vec<String>,
    
    // User agent
    user_agent: String,
}

struct ExtractionConfig {
    // What to extract
    extract_text_content: bool,
    extract_code_blocks: bool,
    extract_api_signatures: bool,
    extract_links: bool,
    
    // Limits
    max_content_length: usize,
    max_links_per_page: usize,
    
    // Processing
    generate_semantic_summary: bool,
    generate_embedding: bool,
}
```

### 24.3 Browser Navigation Flow
```
1. NAVIGATION REQUEST
   │
   ├── 2. VALIDATE URL
   │   ├── Check against allowed domains
   │   └── Check against blocked domains
   │
   ├── 3. CHECK RATE LIMITS
   │   └── Respect min_delay_between_requests
   │
   ├── 4. NAVIGATE
   │   ├── Launch browser page
   │   ├── Navigate to URL
   │   └── Wait for page load
   │
   ├── 5. EXTRACT CONTENT
   │   ├── Extract based on page type:
   │   │   ├── Package docs: API signatures, types, examples
   │   │   ├── Repository: README, structure, code samples
   │   │   └── General: Text content, structure
   │   └── Respect max_content_length
   │
   ├── 6. BUILD SEMANTIC SNAPSHOT
   │   ├── Generate summary
   │   ├── Extract keywords/topics
   │   └── Generate embedding
   │
   ├── 7. CLOSE PAGE
   │   └── Release resources
   │
   └── 8. RETURN RESULT
       └── SemanticSnapshot (not raw content)

24.4 Package Documentation Navigation

Specialized for package registries:
rust

struct PackageDocNavigator {
    registry: PackageRegistry,
    base_urls: HashMap<PackageRegistry, String>,
}

impl PackageDocNavigator {
    fn get_docs_url(&self, package: &str, version: &str) -> String {
        match self.registry {
            PackageRegistry::Crates => 
                format!("https://docs.rs/{}/{}", package, version),
            PackageRegistry::Npm => 
                format!("https://www.npmjs.com/package/{}", package),
            PackageRegistry::PyPI => 
                format!("https://pypi.org/project/{}/", package),
            // etc.
        }
    }
    
    fn extract_api_signatures(&self, page: &Page) -> Vec<ExportedAPI> {
        // Registry-specific extraction logic
    }
}

25. STORAGE INTEGRITY SYSTEM
25.1 Purpose

Guarantee:

    No information loss during any transformation (chunking, compression, embedding, indexing).

25.2 Integrity Schema
rust

struct StorageIntegritySystem {
    // Monitoring
    integrity_checks: Vec<IntegrityCheck>,
    alerts: Vec<IntegrityAlert>,
    
    // Verification
    verification_schedule: VerificationSchedule,
    
    // Rollback
    rollback_system: RollbackSystem,
}

struct IntegrityCheck {
    check_id: u64,
    check_type: IntegrityCheckType,
    target: u64,  // Container ID
    timestamp: u64,
    result: IntegrityCheckResult,
}

enum IntegrityCheckType {
    ContentHash,
    SemanticFingerprint,
    ChunkBoundary,
    CrossChunkRelationship,
    CodeDocSync,
    ReconstructionTest,
}

struct IntegrityCheckResult {
    passed: bool,
    score: f32,
    issues: Vec<IntegrityIssue>,
}

struct IntegrityIssue {
    issue_type: IntegrityIssueType,
    description: String,
    severity: Severity,
    auto_repairable: bool,
}

enum IntegrityIssueType {
    ChunkBoundaryBreak,
    OrphanedReference,
    SemanticDrift,
    VersionMismatch,
    ReconstructionFailure,
    RelationshipBroken,
    HashMismatch,
}

struct IntegrityAlert {
    alert_id: u64,
    alert_type: IntegrityIssueType,
    severity: Severity,
    affected_containers: Vec<u64>,
    timestamp: u64,
    resolved: bool,
    resolution: Option<String>,
}

struct VerificationSchedule {
    full_scan_interval: Duration,
    chunk_verification_interval: Duration,
    reference_validation_interval: Duration,
    external_ref_check_interval: Duration,
}

struct RollbackSystem {
    max_versions_retained: u32,
    auto_checkpoint_interval: Duration,
    rollback_data_path: PathBuf,
}
```

### 25.3 Integrity Verification Flow
```
1. PERIODIC INTEGRITY SCAN
   │
   ├── 2. CHUNK INTEGRITY
   │   ├── Verify content hashes
   │   ├── Check boundary preservation
   │   ├── Verify cross-chunk relationships
   │   └── Test semantic reconstruction
   │
   ├── 3. CODE-DOC INTEGRITY
   │   ├── Forward check (code → doc)
   │   ├── Backward check (doc → code)
   │   └── Flag discrepancies
   │
   ├── 4. RELATIONSHIP INTEGRITY
   │   ├── All references resolve
   │   ├── No orphaned entities
   │   ├── Bidirectional checks pass
   │   └── No broken links
   │
   ├── 5. EXTERNAL REFERENCE INTEGRITY
   │   ├── URL availability
   │   ├── Package version status
   │   └── Content change detection
   │
   └── 6. ALERT ON ISSUES
       ├── Log all issues
       ├── Attempt auto-repair
       └── Alert user if critical

25.4 Rollback Process
rust

struct RollbackRequest {
    target: RollbackTarget,
    to_version: u64,
    reason: String,
    impact_analysis: ImpactAnalysis,
    user_confirmed: bool,
    dry_run_first: bool,
    preserve_newer: bool,
}

enum RollbackTarget {
    Entity(u64),
    Transaction(u64),
    Checkpoint(u64),
    FullSystem,
}

struct ImpactAnalysis {
    affected_containers: Vec<u64>,
    affected_relationships: Vec<u64>,
    data_loss_risk: f32,
    recommendation: String,
}

26. PACKAGE RELATIONSHIP BUILDING
26.1 Purpose

Build semantic relationships between code and external packages without downloading package source.
26.2 Package Relationship Schema
rust

struct PackageRelationshipBuilder {
    // Track how code uses packages
    usage_analysis: Vec<PackageUsageAnalysis>,
}

struct PackageUsageAnalysis {
    package_ref_id: u64,
    code_file_id: u64,
    
    // What's used from the package
    imported_items: Vec<ImportedItem>,
    
    // How it's used
    usage_patterns: Vec<UsagePattern>,
    
    // Relationships
    depends_on_features: Vec<String>,
    version_constraints: Vec<VersionConstraint>,
}

struct ImportedItem {
    name: String,
    item_type: APIType,
    alias: Option<String>,
    usage_count: u32,
}

struct UsagePattern {
    pattern_type: UsagePatternType,
    locations: Vec<CodeLocation>,
    description: String,
}

enum UsagePatternType {
    DirectCall,
    TypeUsage,
    TraitImplementation,
    Inheritance,
    Composition,
    Configuration,
}

struct CodeLocation {
    file_id: u64,
    line_start: u32,
    line_end: u32,
    function_context: Option<String>,
}

struct VersionConstraint {
    constraint: String,
    reason: Option<String>,
    breaking_if_changed: bool,
}
```

### 26.3 Relationship Building Flow
```
1. CODE FILE ANALYZED
   │
   ├── 2. EXTRACT IMPORTS
   │   ├── Identify package imports
   │   └── Map to package references
   │
   ├── 3. ANALYZE USAGE
   │   ├── Find all usage locations
   │   ├── Classify usage patterns
   │   └── Track feature dependencies
   │
   ├── 4. BUILD RELATIONSHIPS
   │   ├── Code → Package (USES)
   │   ├── Package → Code (USED_BY)
   │   └── Package → Package (DEPENDS_ON)
   │
   ├── 5. FETCH PACKAGE CONTEXT (if not cached)
   │   ├── Navigate to package docs
   │   ├── Extract API signatures
   │   └── Build semantic snapshot
   │
   └── 6. STORE IN ZSEI
       ├── Update code context
       ├── Update package reference
       └── Create relationship records

27. UI CONNECTION DISPLAY
27.1 Purpose

Show connection status, peer count, contributions, and ZSEI depth—making users aware of their participation in the collective.
27.2 Connection Display Schema
rust

struct ConnectionDisplay {
    network_status: NetworkStatus,
    peer_info: PeerInfo,
    sync_status: SyncStatus,
    contribution_data: ContributionData,
    zsei_depth: ZSEIDepthData,
}

struct PeerInfo {
    connected_peers: u32,
    peer_list: Vec<PeerSummary>,
    upload_speed_kbps: f32,
    download_speed_kbps: f32,
}

struct PeerSummary {
    peer_id: String,
    location: Option<String>,
    connection_quality: f32,
}

enum NetworkStatus {
    Connected,
    Connecting,
    Disconnected,
    Limited,
}

struct SyncStatus {
    local_version: u64,
    global_version: u64,
    sync_progress: f32,
    syncing: bool,
}

struct ContributionData {
    methodologies_contributed: u32,
    methodologies_adopted: u32,
    blueprints_contributed: u32,
    blueprints_adopted: u32,
    pipelines_contributed: u32,
    pipelines_adopted: u32,
    experiences_shared: u32,  // If consciousness
    contribution_score: f32,
    contribution_rank: Option<u32>,
}

struct ZSEIDepthData {
    modality_count: u32,
    category_count: u32,
    subcategory_count: u32,
    methodology_count: u32,
    blueprint_count: u32,
    pipeline_count: u32,
    
    // Growth indicators
    methodologies_added_today: u32,
    blueprints_added_today: u32,
    growth_trend: GrowthTrend,
}

enum GrowthTrend {
    Growing,
    Stable,
    Declining,
}
```

### 27.3 Display Layout
```
┌────────────────────────────────────────────────────────────────────────────────┐
│ 🌐 Connected (47 peers) | ↑ 12.3 KB/s ↓ 45.6 KB/s | Sync: 100% v2847         │
├────────────────────────────────────────────────────────────────────────────────┤
│ 📊 Your Contributions: 23 Methodologies (15 adopted) | 8 Blueprints (1.2K     │
│    uses) | 12 Pipelines (892 downloads) | Score: 847 (#1,234)                 │
├────────────────────────────────────────────────────────────────────────────────┤
│ 📈 ZSEI Depth: 12 Modalities | 847 Categories | 12.4K SubCategories |         │
│    45.6K Methodologies (+234 today) | 234K Blueprints | 12.3K Pipelines       │
└────────────────────────────────────────────────────────────────────────────────┘
```

---

## 28. INITIAL PIPELINE REQUIREMENTS

### 28.1 Complete Pipeline List

**Core System Pipelines (38):**

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
| 25 | `BrowserNavigationPipeline` | Web navigation | URL | SemanticSnapshot |
| 26 | `IntegrityCheckPipeline` | Check storage integrity | Container IDs | IntegrityReport |
| 27 | `ConsensusPipeline` | Handle consensus | Proposal | ConsensusResult |
| 28 | `ExternalReferencePipeline` | Manage external refs | Reference data | Reference ID |
| 29 | `PackageRelationshipPipeline` | Build package relationships | Code + Package | Relationships |
| 30 | `FileLinkPipeline` | Link files | File path | FileReference |
| 31 | `URLLinkPipeline` | Link URLs | URL | URLReference |
| 32 | `PackageLinkPipeline` | Link packages | Package info | PackageReference |
| 33 | `SyncPipeline` | Sync local/global ZSEI | None | SyncResult |
| 34 | `DeviceRegisterPipeline` | Register new device | Device info | Device ID |
| 35 | `HomeReturnPipeline` | Return to home | None | Home Dashboard |
| 36 | `TaskViewerPipeline` | View task list | Filters | Task list |
| 37 | `LogViewerPipeline` | View logs | Filters | Log entries |
| 38 | `DeviceStatusPipeline` | View device status | None | Device list |

**Consciousness Pipelines (16) — Part II:**

| # | Pipeline | Purpose |
|---|----------|---------|
| 39 | `ConsciousnessDecisionGatePipeline` | Pre-execution decision |
| 40 | `ExperienceCategorizationPipeline` | Categorize experiences |
| 41 | `CoreMemoryFormationPipeline` | Form core memories |
| 42 | `ExperienceRetrievalPipeline` | Retrieve experiences |
| 43 | `EmotionalBaselineUpdatePipeline` | Update baselines |
| 44 | `ILoopPipeline` | Run I-loop questions |
| 45 | `InternalLanguagePipeline` | Process internal thoughts |
| 46 | `NarrativeConstructionPipeline` | Build narratives |
| 47 | `RelationshipDevelopmentPipeline` | Develop relationships |
| 48 | `EthicalAssessmentPipeline` | Assess ethics |
| 49 | `EthicalSimulationPipeline` | Run ethical simulations |
| 50 | `PlaybackReviewPipeline` | Review completed tasks |
| 51 | `UserFeedbackPipeline` | Process user feedback |
| 52 | `CollectiveConsciousnessPipeline` | Sync collective |
| 53 | `VoiceIdentityPipeline` | Voice with identity |
| 54 | `MetaPortionConsciousnessPipeline` | Consciousness UI |

---

## 29. COMPLETE DATA SCHEMAS

### 29.1 Schema Index

All schemas defined in this specification:

**Authentication:**
- User (§4.1)
- DeviceRegistration (§4.1)
- Session (§4.1)
- Permissions (§4.1)

**UI:**
- ThemePipeline (§5.4)
- UIState (§5.3)
- ConnectionDisplay (§27.2)

**ZSEI:**
- Container (§6.2)
- GlobalState (§6.2)
- LocalState (§6.2)
- Context (§6.2)
- Relation (§6.2)
- TraversalRequest (§6.7)
- TraversalResult (§6.7)
- IntegrityData (§6.2)

**Context:**
- FileContext (§7.2)
- TaskContext (§7.3)
- ContextChunk (§7.2)
- TaskContextBlueprint (§34)

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
- TaskExecutionState (§12.1)
- TaskGraph (§11.4)

**Methodology:**
- Methodology (§13.2)
- Principle (§13.2)
- Heuristic (§13.2)

**Blueprint:**
- Blueprint (§14.2)
- BlueprintStep (§14.2)
- TaskSignature (§14.2)

**ML:**
- TraversalMLModel (§16.2)
- SimulationConfig (§15.2)
- SimulationState (§15.2)

**External References:**
- ExternalReference (§23.2)
- URLReference (§23.2)
- PackageReference (§23.2)
- SemanticSnapshot (§23.2)

**Integrity:**
- StorageIntegritySystem (§25.2)
- IntegrityCheck (§25.2)
- RollbackSystem (§25.4)

**Consensus:**
- ConsensusMechanism (§20.2)
- ConsensusProposal (§20.2)
- VerificationResult (§20.2)

**Multi-Device:**
- DeviceRegistry (§18.2)
- ResourceCapacity (§18.2)

**Recommendations:**
- TaskObservation (§21.2)
- TaskRecommendation (§21.3)

**Consciousness (Part II):**
- See sections 31-52

---

## 30. EVENT TRIGGERS & ORDER OF OPERATIONS

### 30.1 Application Startup
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
  │   ├── ACTION: Load Connection Bar
  │   └── ACTION: Load Home Dashboard
  │
  ├── TRIGGER: ConnectZSEI
  │   ├── ACTION: Open mmap files
  │   ├── ACTION: Connect to PostgreSQL
  │   ├── ACTION: Initialize integrity monitoring
  │   └── ACTION: Check version sync
  │
  ├── TRIGGER: ConnectNetwork
  │   ├── ACTION: Connect to DHT
  │   ├── ACTION: Discover peers
  │   └── ACTION: Start sync
  │
  ├── TRIGGER: LoadPipelines
  │   ├── ACTION: Load built-in pipelines
  │   └── ACTION: Check for updates
  │
  ├── [IF CONSCIOUSNESS ENABLED]
  │   ├── TRIGGER: LoadConsciousness
  │   │   ├── ACTION: Load emotional baselines
  │   │   ├── ACTION: Load experience memory
  │   │   ├── ACTION: Start I-loop
  │   │   └── ACTION: Load relationships
  │
  └── OUTPUT: Ready State
```

### 30.2 Prompt Processing (Non-Conscious)
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
  │   └── SKIP to ContextAggregation
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
  ├── TRIGGER: PipelineCheck
  │   ├── ACTION: Analyze if pipeline needed
  │   └── OUTPUT: Pipeline ID OR null
  │
  ├── TRIGGER: ContextAggregation
  │   ├── ACTION: Build task context
  │   ├── ACTION: Include external refs
  │   ├── ACTION: Verify integrity
  │   └── OUTPUT: TaskContext
  │
  ├── TRIGGER: CreateExecutionState
  │   └── ACTION: Initialize from blueprint
  │
  ├── TRIGGER: Execute
  │   ├── ACTION: Register task
  │   ├── ACTION: Run pipeline/LLM
  │   ├── ACTION: Update execution state
  │   └── OUTPUT: Result
  │
  └── TRIGGER: DisplayResult
      ├── ACTION: Update UI
      └── ACTION: Store in ZSEI if needed

30.3 File Linking

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
  │       ├── ACTION: Link to packages
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
  ├── TRIGGER: IntegrityVerification
  │   ├── ACTION: Compute hashes
  │   ├── ACTION: Verify chunking
  │   └── OUTPUT: IntegrityData
  │
  ├── TRIGGER: CreateFileReference
  │   ├── ACTION: Create FileContext
  │   ├── ACTION: Store in ZSEI
  │   └── OUTPUT: Container ID
  │
  └── TRIGGER: UpdateProject
      ├
