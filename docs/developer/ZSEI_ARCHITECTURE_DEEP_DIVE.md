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
nodes and edges. These graphs need to be persisted as ZSEI containers.

### Current State and Required Integration Steps

The traversal engine, hook processor, and container storage are correctly designed.
The gap is in wiring modality graph output into the ZSEI container hierarchy.

**Four steps to complete alignment:**

**Step 1: Bootstrap creates modality root containers**
```
On first run, bootstrap creates:
  /Modality/Text           (ContainerType::Modality, pipeline_id: 100)
  /Modality/Code           (ContainerType::Modality, pipeline_id: 101)
  /Modality/Image          (ContainerType::Modality, pipeline_id: 102)
  ... (all 27 modalities)
  /External/Packages       (ContainerType::Category)
  /External/URLs           (ContainerType::Category)
Also creates IndexReference containers for existing pipeline/methodology/blueprint JSONs.
```

**Step 2: Modality pipelines persist their graphs as ZSEI containers**
```rust
// After text pipeline creates a TextGraph:
for node in &text_graph.nodes {
    let container = Container::from_text_node(node, project_id);
    container.parent_id = text_modality_root_id;
    zsei.store_container(container).await?;
}
// Create edges as Relationship entries in the parent container
```
Add `store_modality_graph(graph, modality_root_id)` to ZSEI API.

**Step 3: Add materialized_path to Container**
```rust
// Optional field on LocalState::Metadata
pub materialized_path: Option<String>,
// e.g. "/Modality/Code/rust/async"
// Enables O(log n) get_by_path lookups
```

**Step 4: Trigger semantic hooks after graph persistence**
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
