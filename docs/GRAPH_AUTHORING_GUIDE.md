# Graph Authoring Guide

How tools and pipelines put real content into the ZSEI graph, what each modality can and cannot produce today, and how to add a new one. Status is as verified on 2026-10-05; anything not checked is marked as such.

## 1. The model

- **Containment** is native: a container's `parent_id` and `child_ids`. Structural traversal follows it.
- **Typed relations** live in `local_state.context.relationships` as `Relation { target_id, relation_type, confidence, discovered_via }`. Structural traversal follows these too.
- **Entity attributes** sit in a content file, referenced from `storage.object_store_path`. This is the content-pointer pattern, so containers stay small.
- **Traversal** is the localized read: start at a container, bound it by `max_depth` and `budget`, and read what is reachable. An operation scoped this way can only touch the subtree it names.

Relation names the host accepts (`src/types/container.rs`, `RelationType`, name map in `src/mcp_graph.rs`): `Contains`, `PartOf`, `RelatedTo`, `References`, `ImportsFrom`, `CallsTo`, `Above`, `Below`, `NearTo`, `InFrontOf`, `Overlaps`. Discovery: `ToolOutput` marks an edge that came from a tool's graph block.

## 2. The graph block contract

An MCP tool adds a `graph` object to its output, either at `output.graph` or `output.output.graph`:

```json
{
  "nodes": [
    {"key": "image", "kind": "Image", "label": "image", "attributes": {"width": 1280, "height": 720}},
    {"key": "cell-1-1", "kind": "GridCell", "label": "center", "parent": "image", "attributes": {}},
    {"key": "obj-0", "kind": "Object", "label": "person", "parent": "cell-1-1", "attributes": {"confidence": 0.84}}
  ],
  "edges": [
    {"from": "obj-0", "to": "obj-1", "relation": "NearTo", "confidence": 0.84}
  ]
}
```

Rules, enforced by `src/mcp_graph.rs` before anything is written:

- Keys are `[A-Za-z0-9_-]`, at most 80 characters, unique.
- `parent` names another node's key. Parent chains must end at the root and must not cycle.
- Edge `from` and `to` must be node keys. `relation` must be from the list above.
- A block that fails any rule writes **nothing**. The reason appears in `review.captured` as "graph persistence failed: ...".
- Nodes with no `parent` hang under a new `McpResult` root container created for the call. The response carries `persisted_graph: {root_id, entities, relations}`.
- Attributes are stored as-is. Only put in them what the tool actually computed.

Containers are created as `McpResult` (root) and `McpEntity` (every node). The modality is `Image` for the image tool family and `Unknown` for the rest, until the modality enum grows.

## 3. Modality status

| Modality | Tool(s) | Graph block | What it actually produces | Gaps |
|---|---|---|---|---|
| Image | `scene_graph` (visual-mcp) | Yes, verified (5 entities, 2 relations) | Real YOLO objects in 3×3 image-thirds cells, edges from box geometry (Overlaps, Above, NearTo). Thresholds stated in output. | OCR needs the tesseract binary (not installed). Faces need a model (no Haar cascades in this OpenCV build). |
| Image | `depth_graph`, `yolo_graph`, `visual_graph` | No, older path: writes a ModalityGraph via the Rust pipeline | Real detections and depth | Depth relations (InFrontOf) are returned, not persisted as edges |
| Image | `pose_detect`, `yolo_segment`, `shape_detect` | Built, not yet verified (person → keypoints; objects with masks; lines and shapes) | Real keypoints, masks, lines and shapes | Verify on the final build |
| Chemistry | `chem_analyze` | Yes, verified (30 entities, 30 relations for caffeine; traversal reached 31) | Real RDKit atoms and bonds; bonds are nodes referencing their atoms | `chem_lookup` (PubChem) has no block |
| DNA | `dna_find_gene` | Yes, verified (3 entities) | Real GenBank genome → gene → CDS containment | `dna_translate` uses the standard code; Mycoplasma reads TGA as Trp, so TGA-containing genes translate wrong |
| Geospatial | `geo_features_near` | Yes, verified (51 entities) | Real OSM features contained by the queried location | Location-to-feature distances are not stored as edges |
| Proteomics | `protein_function` | Built, not yet verified (protein → GO terms and pathways) | Real UniProt annotations | `protein_lookup` has no block. Name lookup resolves "insulin human" to a Conus peptide; needs an organism or reviewed filter. |
| 3D / CAD mesh | `mesh_analyze`, `mesh_convert` | No | Real trimesh metrics (volume, area, watertightness) | A mesh is one node with metrics as attributes; bodies could be nodes |
| CAD STEP | `cad_step_info` | No | Real product list | Assembly hierarchy is available from `python-step-parser`'s `get_product_hierarchy`, but **untested**: no real STEP file on this machine, and the parser needs one |
| CAD pipeline (Rust) | `assets/pipelines/modalities/CAD` | No | Analyze action exists | The graph container it builds has `object_store_path: "graphs/cad_placeholder.json"` and a zeroed integrity hash. It is a stub. Do not build on it. |
| Code | Code modality (Rust, `assets/pipelines/modalities/code`) | No | Regex-based structure extraction, not an AST | **No pipeline emits `CallsTo` or `ImportsFrom`**, and the code modality does not use the `Code*` container types. A real call graph does not exist today. |
| Text | Text modality (Rust) | No | Keyword and topic extraction with stated fallbacks | Document and section containers exist as types; emission is not verified |
| Other 20+ modality pipelines | — | No | Not audited individually | Several contain placeholder markers (e.g. `math`, `text`). Treat as unverified until audited. |

## 4. Authoring: create, edit, remove

- **Create**: a tool returns a graph block (above), or a caller uses `CreateContainer` through `/zsei/query`. Verified through the block path.
- **Edit relations**: `UpdateContainer` with the full `context` (it replaces the context). The block path already does this for edges. Editing attributes means rewriting the content file; not built.
- **Remove a leaf**: `DeleteContainer`. Verified: traversal dropped 6 → 5.
- **Remove a parent**: **not safe yet.** Deleting a parent orphans its subtree; the children stay stored but unreachable. A cascade delete is needed in `src/zsei/mod.rs`, which currently holds an uncommitted lock fix from ZCode.
- **Writes outside the MCP path**: `/zsei/query` accepts writes. As of the current code, every write is audited to `model_calls/zsei_writes.jsonl`, and `OZONE_ZSEI_REQUIRE_SESSION=1` refuses writes without a valid session. The code is written and compiles, but it is **not yet verified live**, because the host build is blocked by in-progress changes in the orchestrator.
- **Rollback, Link and Unlink**: listed as write variants in the query set, but `process()` has no handler for them and returns "Unsupported query type". They cannot write today, and the write audit records each attempt as a failure. `GetVersionHistory` is read-only and is not verified.

## 5. Use cases

- **Code**: a call graph with `CallsTo` and `ImportsFrom` edges would make impact analysis a traversal. Needs a real parser (tree-sitter or `syn`) first. Not built.
- **Text**: sections under documents, with topic relations. Not verified.
- **3D CAD**: an assembly as a containment tree, mates as typed relations. Editing one part means regenerating only its subtree's export. Needs a real STEP file (or CadQuery to produce one) and a graph block from `mesh-mcp`. Not built.
- **Images**: the scene graph works now. Pose and segmentation can use the same cell and geometry approach.
- **Chemistry, DNA, geospatial**: already real. Their graphs support questions like "which features are contained by this genome region" by traversal.
- **Cross-modal**: an image object, a CAD part and a measured value can be linked with typed relations, once they share a root.

## 6. Adding a tool that writes to the graph

1. Compute every node and edge from real data. If a relation depends on a threshold, put the threshold in the output.
2. Return the `graph` block at `output.graph`.
3. Register the tool: `POST /mcp/tools/register` with `{name, transport: "http", endpoint, capabilities, server_version}`. Register only tools that are safe to expose.
4. Call it through `/mcp/call`. Check `persisted_graph` in the response and `review.captured`.
5. Traverse from `root_id`. Expect `entities + 1` containers reached (the root plus every node).
6. Test a bad block (unknown parent, cycle, bad relation). Expect a refusal and no new root.

## 7. Safety properties

- Validation runs before any write, so a malformed block changes nothing.
- The MCP route runs the jurisdiction gate, usage ledger and S13 capture. The S13 row stores the input preview, not the output. Outputs live in the graph.
- Attributes are content files, so a large result does not bloat a container.
- Traversal bounds every read and every operation scoped to a subtree.

## 8. Test hygiene used for these results

- Verified on a second host instance (port 50052, its own data directory, P2P off). Production host 50051 was not restarted.
- Test MCP servers were stopped after each run.
- The test data is disposable.

## 9. Zero-shot calls and semantic relations

Zero-shot calls are LLM prompts that extract entities, topics and relations from content without task-specific training. They are not limited to text.

**Where they run today (survey, 2026-10-05):**
- `llm_zero_shot` helper calls in 18 modality pipelines: 3D (4), BCI (5), biology (6), CAD (4), control (6), depth (5), electromagnetic (4), geospatial (6), haptic (5), hyperspectral (6), IMU (3), kinematics (3), network (4), proteomics (5), radar (5), sonar (5), sound (6), thermal (5).
- Text pipeline: 23 references (the largest user).
- Orchestrator stage 4 (`stage_4_zero_shot_simulation`) and `general/zero_shot_simulation`.
- Every call is logged to `pipeline_zero_shot_calls.jsonl` (`assets/pipelines/shared/capture.rs`).

**What is not captured today:** zero-shot output does not become typed relations. `DiscoveryMethod::ZeroShot` exists in the type system and nothing creates it. Relations are either absent or only recorded as keywords and topics.

**The validator (`assets/pipelines/shared/semantic_relations.rs`):** a model proposes `{"relations": [{"from", "to", "relation", "evidence"}]}`. A proposal is accepted only if:
- `from` and `to` are entities the same extraction produced,
- `relation` is a ZSEI relation name,
- the endpoints differ,
- `evidence` appears verbatim in the source.

Rejections are returned with a reason, not dropped. An accepted relation is stored with `discovered_via: ZeroShot` and `confidence: 1.0`, which means "evidence verified", not a semantic probability (the model supplies none).

**Not done yet:** wiring the validator into the 18 modality pipelines. Each has its own prompts, so each needs its prompt changed to request relations and its consumer changed to call the validator. That changes model behavior and spends LLM calls, so it needs an approved budget and a test per modality. The validator's unit tests run only with `cargo test` on a pipeline that includes it.

**Recipe for wiring a pipeline:**
1. Include the module: `#[path = "../../shared/semantic_relations.rs"] mod semantic_relations;`
2. Ask the zero-shot prompt for the relation JSON above, with the allowed relation names listed.
3. Call `semantic_relations::validate(&proposal, &entity_labels, &source_text)`. Persist the accepted relations through `/zsei/query` `UpdateContainer`, using `zsei_relation_json(target_id, relation)`.
4. Capture the rejected list too, so a model that invents evidence is visible.
