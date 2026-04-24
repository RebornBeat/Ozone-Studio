# OZONE STUDIO — Text Modality Processing Guide

## The Processing Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    TEXT MODALITY PROCESSING FLOW                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  PHASE 1: PER-CHUNK PROCESSING                                              │
│  ─────────────────────────────                                              │
│                                                                             │
│  For each chunk:                                                            │
│    1. CleanChunk (grammar correction, normalization via zero-shot LLM)      │
│    2. DetectParagraphs (identify paragraph breaks → byte offsets)           │
│    3. DetectSentences (within paragraphs → byte offsets)                    │
│    4. SectionStateTracking (cross-chunk state machine, format-agnostic)     │
│    5. Create ParagraphNodes + SentenceNodes → push to text modality graph   │
│    6. Store byte positions on every node (start_byte, end_byte, line)       │
│    7. Create ChunkGraph (persistent historical position record)             │
│                                                                             │
│  State tracked across chunks:                                               │
│    - Current section context (is a section open? what level?)               │
│    - Detected formatting pattern (discovered by zero-shot, any format)      │
│    - Current document boundary state                                        │
│    - Current document ID (if inside a detected document)                    │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  PHASE 2: CROSS-CHUNK AGGREGATION                                           │
│  ─────────────────────────────────                                          │
│                                                                             │
│  After all chunks processed:                                                │
│    1. Gather all ParagraphNodes from all chunk graphs                       │
│    2. Gather all SentenceNodes from all chunk graphs                        │
│    3. Link SentenceNodes to their ParagraphNodes                            │
│    4. Link ParagraphNodes to their Section/Document parents                 │
│    5. Resolve cross-chunk paragraph continuations                           │
│    6. Finalize DocumentNode → SectionNode → ParagraphNode → SentenceNode   │
│       hierarchy in the text modality graph                                  │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  PHASE 3: GRAMMAR EXTRACTION (on full corpus, graph-native)                 │
│  ──────────────────────────────────────────────────────────                 │
│                                                                             │
│    1. For each SentenceNode:                                                │
│       a. ExtractGrammarRelationships(sentence) → grammar edges + nodes      │
│       b. Link grammar nodes to SentenceNode                                 │
│                                                                             │
│    2. Pairwise cross-sentence comparison (build relationship depth):         │
│       a. Sentence × Sentence → relationship edges                           │
│       b. Coreference chains (John → He → the manager)                       │
│       c. Causal chains (because → therefore → consequently)                 │
│       d. Temporal chains (yesterday → then → afterwards)                    │
│       e. Discourse relations (Elaborates, Contradicts, Enables, etc.)       │
│                                                                             │
│    3. GRAPH TRAVERSAL for entity/keyword/topic derivation:                  │
│       a. Traverse SubjectNodes → derive Person/Org/Concept entities         │
│       b. Traverse ObjectNodes → derive Thing/Place/Event entities           │
│       c. Traverse VerbNodes → derive action keywords                        │
│       d. Traverse RelationshipEdges → derive topics                         │
│       (NOT separate extraction — derived FROM the relationship graph)       │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  PHASE 4: MODALITY DETECTION                                                │
│  ───────────────────────────                                                │
│                                                                             │
│    1. For each SentenceNode:                                                │
│       a. Detect embedded modalities (code blocks, math, chemistry, etc.)    │
│       b. Mark spans as "true_text" or detected modality name                │
│       c. 5x consecutive stable loop (detect_modalities_stable)              │
│    2. Create ModalityReferenceNode for non-text spans                       │
│    3. Store span_start / span_end (byte offsets, NO content_snippet)        │
│    4. Contribute to root_modality_list for orchestrator aggregation         │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Critical principle:** The text modality graph is the working structure. ChunkGraphs are
historical position records anchoring byte offsets for the Text Engine. Once chunking
produces a ChunkGraph, the system always works on the graph — never reconstructs back to
chunks for processing. Reconstruction to text is only needed for LLM context window
feeding (cross-model capability) or external file export.

---

## The Chunking Boundary

Chunking happens ONCE on initial intake. After chunks are processed and nodes are created,
all subsequent work is graph-based. Chunks exist only to:

1. Fit text within any LLM context window (cross-model capability)
2. Enable parallel processing of large documents
3. Anchor byte positions for the Text Engine

The ChunkGraph for each processed chunk preserves the byte offset mapping so that any
node in the text modality graph can be found in O(1) via position lookup — enabling the
Text Engine to perform precise byte-level editing without scanning file contents.

---

## Section & Document Detection

### Why Section Detection Matters

Section detection is the only mechanism that allows the system to distinguish:
- Raw text (user's prompt, conversational input, free-form request)
- Embedded documents (guides, articles, reports, specifications)
- Mixed content (user request containing an attached or inline document)

Intent detection belongs to the AMT. Section detection belongs to the text modality.
The text modality identifies WHAT the content is structurally; the AMT determines
WHY the user provided it and what to do with it.

### Section State Machine

State tracking runs across chunk boundaries. The state is carried from chunk N to
chunk N+1 to handle sections that span multiple chunks.

```
SECTION TRACKING STATE MACHINE

State: IDLE
  Trigger: Any structured heading pattern detected
           (regardless of format — zero-shot determines the format)
  Transition → IN_SECTION

State: IN_SECTION
  Trigger: Another heading of same or higher level detected
  Transition → NEW_SECTION (close current, open new)

  Trigger: Sub-heading detected (lower level than current)
  Transition → IN_SUBSECTION (nest under current)

  Trigger: Formatting pattern breaks (N paragraphs with no heading,
           style shift detected by zero-shot, narrative voice changes)
  Transition → DOCUMENT_BREAK or IDLE

State: DOCUMENT_BREAK
  Trigger: New structural pattern begins
  Transition → IN_SECTION (new document context)

CROSS-CHUNK CONTINUATION:

  Chunk N ends mid-section:
    State carries:
      - section_id (persistent across chunks)
      - section_level (1-6 or detected equivalent)
      - formatting_pattern (discovered by zero-shot — any format)
      - current_document_id

  Chunk N+1 receives continuation context:
    - Inherits open section from chunk N
    - May close it or extend it
    - May detect a break → new document
```

**Format-agnostic detection:** The system does not hardcode markdown or any specific
format. Zero-shot LLM determines the formatting pattern from the content itself.
Patterns include (but are not limited to): markdown headers, numbered sections,
all-caps headings, underlined headings, Roman numerals, bold lead lines,
indented outlines, any consistent structural pattern the LLM identifies.

### Zero-Shot Section Detection Prompt

```text
You are analyzing text to detect document structure and section boundaries.
The formatting style is UNKNOWN — do not assume any specific format (not just
markdown). Detect whatever structural patterns actually appear in the text.

CHUNK CONTEXT:
- This is chunk {chunk_index} of {total_chunks}
- Previous section state: {previous_section_state}
- Formatting pattern being tracked: {formatting_pattern or "not yet detected"}

CHUNK TEXT:
{chunk_text}

TASK: Identify sections and document boundaries.

A "section" is any coherent block that begins with a recognizable heading or
structural marker — regardless of format. This includes but is not limited to:
- Markdown headers (# ## ###)
- Numbered sections (1. 2. 2.1)
- ALL-CAPS headings (INTRODUCTION, METHODS)
- Underlined or decorated headings (===, ---)
- Bold lead lines followed by body text
- Any other consistent structural marker you detect

A "document break" occurs when:
- The formatting pattern changes significantly
- The writing style or voice shifts substantially
- Structural markers stop appearing after a consistent run

Return ONLY valid JSON:
{
  "section_events": [
    {
      "type": "section_start|section_continue|section_end|document_break",
      "position": <character_offset_in_chunk>,
      "section_title": "title if section_start, null otherwise",
      "section_level": <1-6 or null>,
      "formatting_pattern": "description of detected pattern or null"
    }
  ],
  "current_section_context": {
    "section_id": "unique identifier for tracking across chunks",
    "level": <1-6 or null>,
    "title": "current section title or null",
    "formatting_pattern": "detected pattern description"
  },
  "document_boundary_detected": false,
  "document_boundary_reason": "reason or null"
}
```

### Zero-Shot Document Identification Prompt

Called when a section boundary or document break is detected, to classify
whether the content constitutes a standalone document.

```text
You are identifying whether a block of text constitutes a standalone document
within a larger corpus.

A "document" is a coherent, self-contained unit characterized by:
- Unified topic or theme throughout
- Consistent formatting pattern (whatever format)
- Clear structural organization (sections, paragraphs, or both)
- Neutral or third-person narrative voice (for reference/informational documents)
- OR: a sustained narrative voice (for stories, guides, essays)

NOT a document:
- First-person conversational requests ("I want", "Please help me")
- Imperative instructions addressed to the AI ("Fix this", "Create a")
- Questions without context ("How do I", "What is")
- Short mixed-intent fragments

IMPORTANT: Documents sometimes contain user intent mixed in. The presence of
some conversational sentences does not disqualify a document — look for the
dominant structure and voice.

TEXT TO ANALYZE:
{text_span}

Return ONLY valid JSON:
{
  "is_document": true|false,
  "document_type": "article|guide|story|report|specification|reference|code_doc|data|unknown|null",
  "title": "extracted title if detectable, null otherwise",
  "confidence": 0.0-1.0,
  "dominant_structure": "description of what makes this a document or not",
  "mixed_intent_detected": true|false,
  "mixed_intent_description": "what conversational/intent elements appear, or null"
}
```

### Document vs Raw Text Classification

After section state detection, content falls into two categories:

**Document** — minimum structure: one section OR one paragraph with consistent
formatting and unified topic. Documents are broken down into their own
Document → Section → Paragraph → Sentence node hierarchy.

**Raw text** — minimum structure: one sentence (or even a fragment). Raw text
from the prompt itself is still formalized to complete sentences during
cleaning (CleanChunk), but it does not require section or document structure.
It links directly to the root of the text modality graph.

**Mixed content** — both exist simultaneously. The section state machine handles
this by detecting document starts and stops within the same chunk stream. A
user request followed by an inline document results in:
- Raw text nodes for the user's words (linked to prompt root)
- A DocumentNode + full hierarchy for the document portion

Intent identification (what the user wants, what's primary vs supplementary)
is NOT handled here — that belongs to the AMT building phase in the
orchestrator, via zero-shot role detection and the FileGraphRole classification.

---

## The Text Engine

### Definition

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            TEXT ENGINE                                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  The Text Engine is a bidirectional document manipulation system that       │
│  operates directly on the text modality graph. It is the AGI's native       │
│  interface for reading, writing, and modifying text-based artifacts.        │
│                                                                             │
│  It is NOT a text editor with graph features added.                         │
│  It IS a graph engine that can render, generate, and edit text.             │
│                                                                             │
│  Primary storage: ZSEI graph (always)                                       │
│  File sync: OPTIONAL — only when working with external local files          │
│  Document state: always lives in the graph, not in a file                  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

For the AGI operating within Ozone Studio, documents are always in their graph
state via ZSEI. There is no "source of truth" file that the graph mirrors — the
graph IS the source of truth. File export and file sync are available when the
use case requires updating an external local file, but they are not the default
operating mode.

### Why the Text Engine Outperforms Grep and Nano

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    SPEED AND CAPABILITY COMPARISON                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  GREP APPROACH:                                                             │
│  ──────────────                                                             │
│  Task: Find sentence containing "John submitted"                            │
│                                                                             │
│  1. Load file into memory (or stream)                                       │
│  2. Compile regex pattern                                                   │
│  3. Scan every line: O(n) where n = file size                               │
│  4. Return raw match — no structural context                                │
│  Accuracy: approximate — misses semantic variants                           │
│  Context: none — does not know which paragraph or section                  │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  TEXT ENGINE APPROACH:                                                      │
│  ─────────────────────                                                      │
│  Task: Find sentence containing "John submitted"                            │
│                                                                             │
│  1. Query graph: SentenceNode WHERE content LIKE "%John submitted%"         │
│  2. Index lookup: O(log n) or O(1) with hash index                          │
│  3. Return node with byte_offset, paragraph context, document context       │
│  Time: sub-millisecond                                                      │
│  Accuracy: exact — graph contains the precise cleaned text                  │
│  Context: full — knows parent paragraph, section, document, relationships   │
│                                                                             │
│  ADDITIONAL CAPABILITIES (grep cannot do these):                            │
│  - Find all sentences where John is the grammatical subject                 │
│  - Follow coreference chain: John → He → the manager → the new hire        │
│  - Find sentences with similar meaning (submitted → turned in → delivered)  │
│  - Find sentences in the same paragraph as target                          │
│  - Find sentences related by causality (John submitted → report reviewed)   │
│  - Find all sentences in the same document section                          │
│  - Distinguish "John" the person from "john" the plumbing fixture          │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  NANO/TEXT EDITOR APPROACH:                                                 │
│  ─────────────────────────────                                              │
│  Task: Edit a specific sentence                                             │
│                                                                             │
│  1. Open file, scroll to location                                           │
│  2. Find text visually                                                      │
│  3. Make edit manually                                                      │
│  4. Save file                                                               │
│  Cognitive load: high (reading, searching, positioning)                     │
│  Error rate: significant (wrong line, wrong position)                       │
│  Speed: human-dependent (seconds to minutes)                                │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  TEXT ENGINE APPROACH:                                                      │
│  ─────────────────────                                                      │
│  Task: Edit a specific sentence                                             │
│                                                                             │
│  1. Graph traversal finds SentenceNode                                      │
│  2. Node contains byte_range: {start_byte: 4523, end_byte: 4558}           │
│  3. Generate EditOperation: replace bytes 4523–4558                         │
│  4. Apply single write operation to file (if file sync enabled)             │
│  5. Update graph node content                                               │
│  6. Propagate position shifts to all subsequent nodes                       │
│  Time: O(1) for edit, O(k) for position propagation                         │
│  Cognitive load: zero (AGI-driven)                                          │
│  Error rate: zero (precise byte targeting)                                  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Text Engine Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    TEXT ENGINE ARCHITECTURE                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ GRAPH LAYER (ZSEI-Native)                                           │   │
│  │                                                                      │   │
│  │ Structural nodes:                                                    │   │
│  │   DocumentNode → SectionNode → ParagraphNode → SentenceNode        │   │
│  │                                                                      │   │
│  │ Grammar nodes (per sentence):                                        │   │
│  │   Subject, Verb, Object, Modifier                                   │   │
│  │                                                                      │   │
│  │ Derived nodes (from graph traversal):                                │   │
│  │   EntityNode (Person, Org, Location, Concept, etc.)                  │   │
│  │   CoreferenceChain (John → He → the manager)                         │   │
│  │                                                                      │   │
│  │ Reference nodes:                                                     │   │
│  │   ModalityReferenceNode (embedded code, math, etc.)                  │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│                                    ▼                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ POSITION LAYER (byte-offset mapping)                                 │   │
│  │                                                                      │   │
│  │ NodePositions:                                                       │   │
│  │   node_id → {start_byte, end_byte, start_line, end_line}            │   │
│  │   Enables precise edit targeting — no file scanning needed           │   │
│  │                                                                      │   │
│  │ LineMappingTable:                                                    │   │
│  │   line_number → {start_byte, primary_node_id}                       │   │
│  │   O(1) lookup: "what node is at line N" → byte offset               │   │
│  │                                                                      │   │
│  │ ChunkAnchors (stored in ChunkGraph):                                 │   │
│  │   node_id → {chunk_id, chunk_offset}                                │   │
│  │   Enables cross-model reconstruction: any LLM context limit          │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│                                    ▼                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ OPERATION LAYER                                                      │   │
│  │                                                                      │   │
│  │ Read Operations:                                                     │   │
│  │   GetSentence(node_id)  → text, position, grammar relationships     │   │
│  │   GetParagraph(node_id) → text, child sentences, position           │   │
│  │   GetSection(node_id)   → text, paragraphs, subsections             │   │
│  │   GetDocument(node_id)  → text, sections, metadata                  │   │
│  │   FindSentences(query)  → [SentenceNode] (semantic or exact)        │   │
│  │   TraverseCoreferences(entity) → [EntityMention in order]           │   │
│  │   FindByGrammarRole(role, value) → [SentenceNode]                   │   │
│  │                                                                      │   │
│  │ Write Operations (graph-native, file sync optional):                 │   │
│  │   InsertSentence(after_node, content)   → SentenceNode              │   │
│  │   ModifySentence(node_id, new_content)  → UpdatedNode               │   │
│  │   DeleteSentence(node_id)               → UpdatedGraph              │   │
│  │   InsertParagraph(section_id, position) → ParagraphNode             │   │
│  │   ReorderParagraphs(section_id, order)  → UpdatedGraph              │   │
│  │   InsertSection(document_id, after, content) → SectionNode          │   │
│  │   GenerateParagraph(topic, style)       → ParagraphNode             │   │
│  │   ExpandOutline(outline)                → FullDocumentGraph         │   │
│  │   SummarizeSection(section_id)          → SummaryNode               │   │
│  │                                                                      │   │
│  │ Synthesis Operations (AGI document generation):                      │   │
│  │   CreateDocument(title, type)     → DocumentNode (blank)            │   │
│  │   GenerateFromAMT(amt_branch)     → DocumentGraph                   │   │
│  │   AdaptStyle(content, target)     → StyledDocumentGraph             │   │
│  │   TranslateContent(content, lang) → TranslatedDocumentGraph         │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│                                    ▼                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ RENDER LAYER (text output from graph)                                │   │
│  │                                                                      │   │
│  │   RenderDocument(node_id)   → formatted text (any target format)   │   │
│  │   RenderSection(node_id)    → section text                          │   │
│  │   RenderParagraph(node_id)  → paragraph text                        │   │
│  │   RenderWithAnnotations(node_id, highlights) → annotated output     │   │
│  │   ExportToFormat(doc_id, format) → markdown/html/docx/txt          │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Bidirectional Sync (External File Mode Only)

File sync is optional and only relevant when the AGI is working on a local file
that exists outside Ozone Studio and needs to be updated. For all AGI-generated
or AGI-processed content, the graph is the source of truth and no file sync
is needed.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    BIDIRECTIONAL SYNC (EXTERNAL FILE MODE)                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  GRAPH → FILE SYNC (AGI modifies graph, file needs update)                  │
│  ──────────────────────────────────────────────────────────                 │
│                                                                             │
│  1. AGI modifies SentenceNode.content in graph                              │
│  2. CalculatePositionDiff(old_content, new_content)                        │
│  3. Generate EditOperation:                                                 │
│     {                                                                       │
│       type: "replace",                                                      │
│       start_byte: 4523,                                                     │
│       end_byte: 4558,                                                       │
│       old_text: "John submitted the report yesterday.",                     │
│       new_text: "John submitted the final report yesterday."                │
│     }                                                                       │
│  4. ApplyToFile(file_path, edit_operation) — single write, no scan         │
│  5. UpdatePositionMappings() — propagate byte shifts to subsequent nodes   │
│  6. Version note added to affected nodes                                    │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  FILE → GRAPH SYNC (external file changed, graph needs update)              │
│  ──────────────────────────────────────────────────────────────             │
│                                                                             │
│  1. Detect file change (watcher or version control signal)                 │
│  2. DiffDetection: compare file to cached version                          │
│  3. For each changed region:                                                │
│     a. FindAffectedNodes(byte_range) → [SentenceNode, ParagraphNode]       │
│     b. ReParse changed content                                              │
│     c. UpdateGrammarNodes(changed sentences)                               │
│     d. PropagateCoreferenceChanges()                                        │
│     e. RevalidateDocumentStructure()                                        │
│  4. Store new version state                                                 │
│                                                                             │
│  NOTE: This sync path is only needed for external file workflows.           │
│  AGI-native document work never requires syncing — the graph IS the doc.   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Position Tracking in Nodes

Every node in the text modality graph that corresponds to a text span carries
byte-level position data. This is what enables O(1) edit targeting.

```
SentenceNode {
  node_id: 12345,
  node_type: Sentence,
  content: "John submitted the report yesterday.",

  // POSITION (populated during Phase 1 chunk processing)
  position: {
    start_byte: 4523,
    end_byte: 4558,
    start_line: 23,
    end_line: 23
  },

  // CHUNK ANCHOR (stored in ChunkGraph — enables cross-model reconstruction)
  chunk_id: 3,
  chunk_offset: 145,

  // VERSION
  version: 1,
  last_modified_step: None
}

EDIT OPERATION FLOW:
  Graph traversal finds: SentenceNode[12345] needs modification
  Lookup: start_byte = 4523 (O(1) — no file scan)
  Generate EditOperation { start_byte: 4523, end_byte: 4558, new_text: "..." }
  Apply: single file write if external file sync enabled
  Propagate: shift all byte offsets for nodes with start_byte > 4558
```

### Line Mapping Table

```
LINE MAPPING TABLE (lightweight in-memory structure)

File: document.md
┌───────┬────────────┬───────────────────────┐
│ Line  │ Start Byte │ Primary Node          │
├───────┼────────────┼───────────────────────┤
│ 1     │ 0          │ DocumentNode[1]       │
│ 2     │ 24         │ (blank)               │
│ 3     │ 25         │ SectionNode[2]        │
│ 4     │ 45         │ ParagraphNode[3]      │
│ 5     │ 203        │ ParagraphNode[3]      │
│ ...   │ ...        │ ...                   │
│ 23    │ 4523       │ SentenceNode[12345]   │
└───────┴────────────┴───────────────────────┘

QUERY: "Find node at line 23"
→ O(1) lookup: line 23 → byte 4523 → SentenceNode[12345]
No content scan. No regex. No grep. Just math.
```

---

## What We Capture in the Text Graph

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                TEXT GRAPH CONTENT (What We Actually Store)                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  STRUCTURAL NODES (TextNodeType in code)                                    │
│  ──────────────────────────────────────                                     │
│                                                                             │
│  DocumentNode                                                               │
│    node_type: Document                                                      │
│    content: summary (word count, type, etc.)                               │
│    properties: { document_type, word_count, sentence_count, paragraph_count,│
│                  readability_score, title, source, role }                   │
│    position: { start_byte, end_byte }                                       │
│                                                                             │
│  SectionNode                                                                │
│    node_type: Section                                                       │
│    content: section title                                                   │
│    properties: { level (1-6), formatting_pattern, parent_section }          │
│    position: { start_byte, end_byte, start_line }                           │
│                                                                             │
│  ParagraphNode                                                              │
│    node_type: Paragraph                                                     │
│    properties: { sentence_count, parent_section, document_id }             │
│    position: { start_byte, end_byte }                                       │
│                                                                             │
│  SentenceNode                                                               │
│    node_type: Sentence                                                      │
│    content: the actual sentence text                                        │
│    properties: { sentence_type (declarative/interrogative/imperative/       │
│                  fragment), parent_paragraph }                              │
│    position: { start_byte, end_byte, start_line, end_line }                 │
│    chunk_anchor: { chunk_id, chunk_offset }                                 │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  GRAMMAR NODES (per sentence, derived via zero-shot)                        │
│  ────────────────────────────────────────────────────                       │
│                                                                             │
│  Subject                                                                    │
│    text: the subject noun phrase                                            │
│    position_in_sentence: { start, end }                                     │
│    entity_type_hint: Person|Organization|Location|Concept|Unknown           │
│                                                                             │
│  Verb                                                                       │
│    text: the main verb                                                      │
│    verb_type: action|linking|helping (VerbType enum in code)                │
│    tense: past|present|future                                               │
│    negated: bool                                                            │
│                                                                             │
│  Object                                                                     │
│    text: the object noun phrase                                             │
│    position_in_sentence: { start, end }                                     │
│    entity_type_hint                                                         │
│                                                                             │
│  Modifier                                                                   │
│    text: the modifier text                                                  │
│    modifier_type: temporal|adjectival|adverbial|prepositional               │
│    modifies: subject|verb|object                                            │
│                                                                             │
│  ChunkGrammarRelationship (code struct) captures all of the above:          │
│    from_text, to_text, edge_type, tense, negated, verb, verb_type,          │
│    subject, object, source_sentence_start, source_sentence_end              │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  DERIVED NODES (from graph traversal after Phase 3)                         │
│  ──────────────────────────────────────────────────                         │
│                                                                             │
│  EntityNode                                                                 │
│    node_type: Entity                                                        │
│    text: canonical entity text                                              │
│    entity_type: Person|Organization|Location|Date|Product|Event|           │
│                 Technology|Concept|Custom (EntityType enum in code)         │
│    confidence: f32                                                          │
│    mentions: [grammar_node_ids across sentences]                            │
│    coreference_chain_id: Option<u64>                                        │
│                                                                             │
│  CoreferenceChain                                                           │
│    chain_id                                                                 │
│    mentions: [node_ids in order of appearance]                              │
│    canonical_form: e.g. "John Smith"                                        │
│                                                                             │
│  TopicNode                                                                  │
│    node_type: Topic                                                         │
│    text: topic name                                                         │
│    relevance: f32 (derived from relationship traversal coverage)            │
│                                                                             │
│  KeywordNode                                                                │
│    node_type: Keyword                                                       │
│    text: keyword or phrase                                                  │
│    relevance: f32                                                           │
│    (derived from verb/object traversal, NOT separate extraction pass)       │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  MODALITY REFERENCE NODES                                                   │
│  ─────────────────────────                                                  │
│                                                                             │
│  ModalityReferenceNode (ChunkModalityDetection in code)                     │
│    modality: the detected modality name (from pipeline registry)            │
│    span_start: char offset within sentence/chunk                            │
│    span_end: char offset within sentence/chunk                              │
│    intent_reference: how this modality is referenced (describes/contains)   │
│    chunk_index: which chunk this was detected in                            │
│    NOTE: no content_snippet — retrieve dynamically: &text[span_start..end] │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  HISTORICAL RECORDS (ChunkGraph in code)                                    │
│  ─────────────────────────────────────────                                  │
│                                                                             │
│  ChunkGraph (one per processed chunk — position anchor, not working struct) │
│    graph_id                                                                 │
│    chunk_index                                                              │
│    prompt_start_char, prompt_end_char (absolute position in original)       │
│    sentence_boundaries: [{start, end, sentence_type}]                       │
│    paragraph_breaks: [char_offsets]                                         │
│    cleaned_text: the cleaned chunk text                                     │
│    grammar_relationships: [ChunkGrammarRelationship]                        │
│    modality_detections: [ChunkModalityDetection]                            │
│    root_modality_list_contribution: [modality_names]                        │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### What We Do NOT Store Redundantly

- No content_snippet on modality detections (span offsets → retrieve dynamically)
- No duplicate extracted keywords/topics (derived from graph traversal)
- No raw chunk text in the working graph (ChunkGraph holds it for position only)
- No intent labels in text modality (that is AMT's responsibility)
- No "primary/supplementary/raw_data" role classification in text modality
  (that is FileGraphRole, classified by the orchestrator post-graph-creation)

---

## Edges in the Text Modality Graph

```
STRUCTURAL EDGES (TextEdgeType in code)
  HasSection     DocumentNode → SectionNode
  HasParagraph   SectionNode → ParagraphNode  (or DocumentNode → ParagraphNode)
  HasSentence    ParagraphNode → SentenceNode
  Contains       Document/Section/Paragraph → any child
  ContainedBy    inverse of Contains
  Follows        SentenceNode → next SentenceNode (sequential order)
  Precedes       SentenceNode → previous SentenceNode

GRAMMAR EDGES (per sentence, from ChunkGrammarRelationship)
  SubjectOf      SubjectNode → VerbNode
  ObjectOf       ObjectNode → VerbNode
  ModifiedBy     AnyGrammarNode → ModifierNode
  Edge types from grammar extraction (edge_type field in code):
    Performs, Affects, Implies, Contradicts, Elaborates, Summarizes,
    Supports, TemporalPrecedes, TemporalFollows, CausedBy, Enables,
    Prevents, PartOf, HasPart, FunctionalRole, InstanceOf, HasInstance,
    SimilarTo, DerivedFrom, VersionOf

CROSS-SENTENCE EDGES (from pairwise comparison in Phase 3)
  Coreferences   GrammarNode → GrammarNode (in different sentence)
  Elaborates     SentenceNode → SentenceNode
  Causes         SentenceNode → SentenceNode
  Enables        SentenceNode → SentenceNode
  Prevents       SentenceNode → SentenceNode
  Contradicts    SentenceNode → SentenceNode
  Exemplifies    SentenceNode → SentenceNode
  Summarizes     SentenceNode → SentenceNode
  TemporalPrecedes SentenceNode → SentenceNode

CROSS-DOCUMENT EDGES (TextEdgeType in code)
  References     DocumentNode → DocumentNode
  RelatesTo      SentenceNode → SentenceNode (cross-document)
  SimilarTo      any node → any node (semantic similarity)
  Supports       SentenceNode → SentenceNode
  Contradicts    SentenceNode → SentenceNode

CROSS-MODALITY EDGES (TextEdgeType in code)
  DescribesCode   TextNode → CodeGraph node
  DescribesImage  TextNode → ImageGraph node
  DescribesAudio  TextNode → AudioGraph node
  DescribesVideo  TextNode → VideoGraph node
  TranscribedFrom TextNode ← AudioGraph node
```

---

## Processing Details

### Phase 1: Per-Chunk Processing

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    PER-CHUNK PROCESSING STEPS                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  INPUT: RawChunk { index, text, start_char, end_char, token_count }         │
│  OUTPUT: ChunkGraph { sentence_boundaries, paragraph_breaks, positions,     │
│                       grammar_relationships, modality_detections }          │
│          + ParagraphNodes and SentenceNodes pushed to text modality graph   │
│                                                                             │
│  STEP 1: CLEAN CHUNK (zero-shot LLM)                                        │
│    - Grammar correction and normalization                                   │
│    - Consistent punctuation and quotation marks                             │
│    - Preserve structure: paragraph breaks, sentence boundaries              │
│    - Returns cleaned_text                                                   │
│                                                                             │
│  STEP 2: DETECT PARAGRAPHS                                                  │
│    - Split on double line breaks and other paragraph markers               │
│    - Track paragraph start/end byte offsets                                │
│    - Create ParagraphNode for each paragraph                               │
│    - Store in chunk graph as paragraph_breaks                               │
│                                                                             │
│  STEP 3: DETECT SENTENCES (within each paragraph)                           │
│    - Split on sentence boundaries (. ! ? and context-aware)                │
│    - Handle: abbreviations (Dr. Mr. etc.), decimals, URLs, ellipses        │
│    - Calculate byte offsets for each sentence                               │
│    - Determine SentenceType: declarative/interrogative/imperative/fragment  │
│    - Create SentenceNode for each sentence                                  │
│    - Store in chunk graph as sentence_boundaries                            │
│                                                                             │
│  STEP 4: SECTION STATE TRACKING                                             │
│    - Check for heading patterns via zero-shot LLM (format-agnostic)        │
│    - Update section state machine                                           │
│    - Detect document boundaries                                             │
│    - Carry state to next chunk                                              │
│                                                                             │
│  STEP 5: PUSH NODES TO TEXT MODALITY GRAPH                                  │
│    - ParagraphNode and SentenceNode linked to current section/document      │
│    - Grammar relationships and modality detections stored in ChunkGraph     │
│    - Byte positions stored on every node                                    │
│                                                                             │
│  STEP 6: CREATE CHUNKGRAPH (historical position record)                     │
│    - Stores everything needed to reconstruct context at any token limit     │
│    - Chunk anchors link graph nodes back to their chunk                     │
│                                                                             │
│  STATE CARRIED TO NEXT CHUNK:                                               │
│    current_section_id, current_section_level,                              │
│    formatting_pattern, current_document_id                                  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Phase 3: Grammar Extraction

Grammar extraction runs on the full graph after all chunks are processed and
the complete sentence/paragraph/section hierarchy is assembled. This ensures
cross-sentence relationships can form across any sentence in the corpus,
not just within a single chunk.

#### Per-Sentence Grammar Extraction Prompt

```text
You are extracting grammatical structure from a single sentence.

SENTENCE: "{sentence_text}"

TASK: Identify the grammatical components and their relationships.

Return ONLY valid JSON:
{
  "subject": {
    "text": "the subject noun phrase",
    "position_start": 0,
    "position_end": 10,
    "entity_type_hint": "Person|Organization|Location|Document|Time|Concept|Unknown"
  },
  "verb": {
    "text": "the main verb",
    "position_start": 11,
    "position_end": 20,
    "verb_type": "action|linking|helping",
    "tense": "past|present|future|unknown",
    "negated": false
  },
  "object": {
    "text": "the object noun phrase or null",
    "position_start": 21,
    "position_end": 35,
    "entity_type_hint": "..."
  },
  "modifiers": [
    {
      "text": "modifier text",
      "position_start": 36,
      "position_end": 45,
      "modifier_type": "temporal|adjectival|adverbial|prepositional",
      "modifies": "subject|verb|object"
    }
  ],
  "sentence_type": "declarative|interrogative|imperative|fragment",
  "edge_type": "Performs|Affects|Implies|CausedBy|Enables|PartOf|SimilarTo|..."
}
```

#### Cross-Sentence Relationship Prompt (Pairwise Comparison)

Run after all per-sentence grammar extraction. The pairwise comparison produces
the relationship depth of the graph — this is where the most cross-sentence
connections form.

```text
You are analyzing relationships between sentences in a text corpus.

SENTENCES:
{sentence_list_with_indices_and_grammar_summaries}

TASK: Identify all meaningful relationships between these sentences.

Relationship types:
- Elaborates:        S2 provides more detail about S1
- Causes:            S1's action or state leads to S2
- Enables:           S1 makes S2 possible
- Prevents:          S1 prevents S2
- Contradicts:       S2 contradicts or negates S1
- Exemplifies:       S2 is a concrete example of S1
- Summarizes:        S2 is a summary of S1 (or vice versa)
- TemporalPrecedes:  S1 happens before S2 in described time
- Coreference:       S2 contains a pronoun or reference to an entity in S1
- PartOf:            S2 is a component of the whole described in S1
- SimilarTo:         S1 and S2 describe closely related concepts

Return ONLY valid JSON:
{
  "relationships": [
    {
      "from_sentence_index": 0,
      "to_sentence_index": 1,
      "relationship_type": "Elaborates|Causes|Enables|...",
      "evidence": "brief quote showing the relationship"
    }
  ],
  "coreference_chains": [
    {
      "canonical_entity": "John Smith",
      "mentions": [
        {"sentence_index": 0, "text": "John Smith", "grammar_role": "subject"},
        {"sentence_index": 2, "text": "He", "grammar_role": "subject"},
        {"sentence_index": 5, "text": "the manager", "grammar_role": "object"}
      ]
    }
  ]
}
```

### Phase 4: Modality Detection (5x Stable Loop)

```text
You are identifying all modality content present in or referenced by this text.

Available modalities to detect: {dynamic_modality_list_from_pipeline_registry}

Also detect:
- "true_text": prose/text that is genuinely text content (not embedded other modalities)
- "unknown": content that does not fit any listed modality

IMPORTANT: span_start and span_end are character offsets within the provided text.
Do NOT include content_snippet — positions are sufficient for retrieval.

Text:
{sentence_or_chunk_text}

Return ONLY valid JSON array:
[{
  "modality": "modality_name|true_text|unknown",
  "span_start": 0,
  "span_end": 100,
  "intent_reference": "describes|contains|references|mentions the modality"
}]
```

The 5x stable loop (detect_modalities_stable in code) runs this prompt
repeatedly until 5 consecutive passes find no new modality detections.
Deduplication key: modality + span_start + span_end.

The modality list is loaded dynamically from the pipeline registry at runtime.
It is never hardcoded.

---

## Summary

### The Text Modality Captures

1. **Structure**: Documents → Sections → Paragraphs → Sentences with byte positions
2. **Grammar**: Subject, Verb, Object, Modifier per sentence (zero-shot)
3. **Relationships**: Pairwise cross-sentence comparison → full relationship graph
4. **Coreferences**: Entity chains across the entire corpus
5. **Derived entities**: Person, Organization, Location, Concept — from traversal
6. **Derived topics & keywords**: From verb/object/relationship traversal (NOT separate extraction)
7. **Modality spans**: Embedded code, math, etc. with byte positions (5x stable)
8. **Position tracking**: Byte offsets on every node (enables Text Engine O(1) editing)
9. **Historical records**: ChunkGraphs anchor positions for cross-model reconstruction

### The Text Engine Provides

1. **O(1) lookup** by position via line mapping table
2. **Semantic search** — find by meaning, grammar role, or coreference chain
3. **Precise editing** — byte-level targeting, no file scanning
4. **Graph-native generation** — AGI creates/modifies documents in graph space
5. **Optional file sync** — for external local files only; graph is source of truth in Ozone Studio
6. **Cross-model reconstruction** — reconstruct clean text from chunk graphs at any token limit

### Processing Order

```
Phase 1: Per-chunk → clean + sentences/paragraphs + section state + ChunkGraph
Phase 2: Cross-chunk aggregation → full hierarchy in text modality graph
Phase 3: Grammar extraction per sentence → pairwise comparison → graph traversal
          for entity/keyword/topic derivation
Phase 4: Modality detection (5x stable) → ModalityReferenceNodes → root_modality_list
```

Grammar extraction waits for the full corpus to be assembled because cross-sentence
relationships are richer when all sentences are available — a coreference chain
spanning chunk boundaries can only be discovered after both chunks are processed.
Keywords, topics, and entities wait for grammar extraction to complete because they
are derived BY TRAVERSING the relationship graph, not by independent extraction passes.
This produces more accurate and contextually grounded derivations.
