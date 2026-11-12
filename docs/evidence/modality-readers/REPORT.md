> **SUPERSEDED IN PART 2026-11-11 (operator: "there should be NO unreadable files, because only accepted modalities/extensions link at all")** — `Prepared::Unreadable` and the worker statuses `unreadable` / `not_text_readable` / `file_unreadable` are gone. The one extension table (`assets/pipelines/shared/modality_ext.rs`) now holds ONLY extensions a reader exists for; everything else is rejected at LINK time with the reason (`modality_ext::accept`). See docs/evidence/no-skip-sweep/REPORT.md for the per-extension table.

# Modality file readers — 2026-11-11

Operator ruling: readers for ALL modalities with the extensions we have for each; follow how text is handled; review the GitHub for existing 3D/image traversal; no over-engineering. Edit only: NOT compiled, NOT tested (rustfmt / `ts.transpileModule` parse checks only). EU untouched.

## 1. What existed
- **One extension table** (`assets/pipelines/shared/modality_ext.rs`, included by `src/types/modality.rs`): maps every extension to a modality or one of four metadata-only buckets (`data`, `config`, `archive`, `document`). Reused as is.
- **Text path**: the walk and `graph_on_link.rs` both did `read_to_string` and sent `{"action":{"type":"Analyze","text"|"code":…}}` (math: `ParseExpression`). Chunking to the reading model's window happens downstream, never at the read.
- **The gap**: for every other modality the walk sent the generic text-shaped `Analyze` (the old comment in `graphs.rs::process_modality` says so) and `graph_on_link` recorded `not_text_readable`. The 26 other pipelines do NOT take text: reading their `main.rs` shows two input shapes and per-modality action vocabularies (`AnalyzeScene`, `AnalyzeFile`, `AnalyzeChain`, `AnalyzeMolecule`, `AnalyzeSequence`, `Analyze` with a typed `data` source…). Their data sources mostly name a **file path**; the pipeline opens and streams the file itself.
- **Existing traversal inside the pipelines**: 3D already has real readers (`3D/readers.rs`, `glue.rs`: glTF/GLB/OBJ/STL/PLY); image decodes PNG/JPEG/GIF/BMP via the `image` crate; chemistry has `molparse.rs`; IMU has `real.rs`. Nothing needed to be written in the host for pixel/mesh/sample traversal — the reader's job is to hand each pipeline the input it takes.

## 2. What the operator's GitHub had (83 repos; `api.github.com`, read-only)
| Repo | Content | Verdict |
|---|---|---|
| `DeepTracking-ZeroShotBoltedEmbedding-Indexer-Legacy-` | `src/indexing/base/{image,audio,video}.rs`, embeddings, stores | Struct sketches whose methods return `Ok(Vec::new())`/placeholders (`detect_objects`, `detect_scenes`), `confidence` fields, `max_objects` caps, ML feature extraction. **Outdated, stubbed, contradicts doctrine — not used.** |
| `ZSEI` | 3D methodology docs (`docs/methodologies/3d/*`, `3d-framework.md`) | Docs only, no reader code. Not used. |
| `BlendTagger` | Python Blender plugin (annotation/animation export) | Authoring tool, no file traversal. Not used. |
| `HyperGCode-4D` | `slicer/src/core/mesh_loader.rs` (STL ascii/binary, OBJ, 3MF) | A competent loader, but the 3D pipeline here already reads STL/OBJ/PLY/glTF natively; 3MF is not in the extension table. Not needed. |
| `OMNI-SENSE-…` | sensor traits/physics, README only | Nothing to read. |
| others (blockchain, OS, defence/sensing frameworks, CIBOS…) | no modality file readers | n/a |

Conclusion: nothing on the GitHub is worth importing. The right design is the smallest one that follows what Ozone already does.

## 3. Design (`src/orchestrator/file_reader.rs`, new; ~330 lines, no new dependency)
One module, used by BOTH the walk and `graph_on_link.rs`:
- `prepare(path, pipeline_id) -> Prepared { Text | Action | Unreadable(reason) }`
- `text_analyze_input`, `create_graph_input`, `graph_id_of`, `shape_of` — the payload/shape differences made explicit (`Shape::{TextLike, Math, Wrapped, Bare}`; table in the module doc):
  - **Wrapped** (`{"action":{…}}` in, `result` out, graph id at `result.graph_id`): image 102, audio 103, video 104, chemistry 106, dna 107, eeg 108 (+ math 105 with its own actions).
  - **Bare** (the action IS the input, `analysis`/`graph_id` at top level): 109–127.
  - `link_to_existing` exists only on the text / code / math `CreateGraph`; it is no longer sent to the others (they would ignore it).
- Chunking/size: the pipelines stream by path, so the host never loads or caps a binary file. Text-shaped files are read whole as before. No size refusal anywhere.
- Unreadable = recorded reason (ledger status `unreadable`, shown in the UI), never an invented analysis and never made-up parameters.

### Per-modality readers
| Modality (pipeline) | Extensions → input |
|---|---|
| text 100, code 101, math 105 | whole UTF-8 text (unchanged). A binary document (pdf/docx…) fails the UTF-8 read → `unreadable` (needs its text extractor; the walk had the same limit). |
| image 102 | every image extension → `{"FilePath"}` (decoders: pipeline's `image` crate; heic/psd/raw/etc. get the pipeline's own honest error) |
| audio 103 / video 104 / eeg 108 | every extension → `{"FilePath"}` |
| dna 107 | fasta fa fna ffn faa frn fastq fq gbk gb genbank → `{"FilePath"}`; vcf/sam/bam/bed/gff/… have no sequence-analysis input → `unreadable` |
| chemistry 106 | mol sdf→`MolFile`, pdb→`PDB`, xyz→`XYZ`, smi smiles→`SMILES`, inchi→`InChI` (whole text); cif mol2 cml cdx cdxml rxn ct mae → `unreadable` |
| 3d 109 | gltf glb obj stl ply → `AnalyzeScene` (STL unit = the pipeline's documented default Meters); fbx usd* blend dae world 3ds c4d ma mb → `unreadable` (`glue.rs` itself lists only the five) |
| cad 120 | step stp iges igs brep occ f3d fcstd sldprt sldasm x_t x_b sat → `AnalyzeFile` |
| kinematics 121 | urdf mjcf sdf bvh c3d osim → `AnalyzeChain`; kin dh → `unreadable` |
| control 122 | slx mdl simulink → `Simulink`; fmu/others need a model name → `unreadable` |
| proteomics 112 | mmcif → `MMCIF`; mass-spec / map formats need format+experiment → `unreadable` |
| geospatial 117 | geojson kml kmz gpx shp osm pbf → typed source; dbf shx prj (sidecars), dem asc grd nc (datum/CRS/variable) → `unreadable` |
| thermal 114 | rjpeg fff → `FLIRRadiometric`; seq csq ir2 (fps), tif (bit depth) → `unreadable` |
| depth 115 | las laz → flags read from the LAS header (point format byte); pcd → `binary`/`normals`/`rgb` read from the PCD header; pts e57 zdf → `unreadable` |
| sound 110 | `AudioFile` channel Mono (the pipeline's default); only reachable by explicit modality — no path hint produces it |
| imu 116, haptic 113, radar 124, sonar 125, electromagnetic 118, hyperspectral 126, bci 119, biology 111 | their file sources carry sensor PARAMETERS (sample rate, band count, species, marker channel…) the file does not state → `unreadable` with the exact list; a caller that knows them sends its own `Analyze` |
| network 123 | graph files (graphml gexf dot gml net pajek cyjs) have no data source (nmap/pcap/netflow/…) → `unreadable` |
| uidocument 127 | library crate with no pipeline binary → `unreadable` |

## 4. Wiring
- `graph_on_link.rs`: its local `analyze_input` / `create_graph_input` / `graph_id_of` and the `read_to_string` gate are replaced by `file_reader`; `not_text_readable` is superseded by `unreadable` (old rows still honoured as terminal). Tests updated.
- Walk: `graphs.rs::graph_attached_file` (new) + a call-site swap in `mod.rs` STEP 0 (the previous `process_modality` + inline `CreateGraph` + graph-id block, ~95 lines, removed). Text / code / math behave as before, including the earlier minted-id fallback for those three only (kept deliberately: the directive says text stays unchanged; it is a known weakness, listed below). Other modalities error on a missing graph id instead of minting one.
- UI: `FileGraphsView.tsx` status text for `unreadable`; `ozoneClient.ts` status comment.
- `mod.rs` was edited after the walk-expansion fork committed (aea7afb) and released it; claims were checked first.

## 5. Gaps left (not hidden)
1. **Sensor modalities (8) have no reader** — by design (parameters), not by omission. If the operator wants them, the answer is a small parameter sidecar (`<file>.ozone.json`) or a UI form that supplies them; not inventing.
2. **Pipelines' own support is narrower than their enums**: CAD/kinematics/proteomics/geospatial/control/thermal pipelines may refuse a source honestly; that surfaces as `failed_analyze` (retried on the next trigger), not `unreadable`. Only 3D's supported set (5 formats) was verified in code and is encoded in the reader.
3. **Walk text path keeps `unwrap_or(generate_id_static())`** — a failed text CreateGraph is masked by a minted id (pre-existing; recommend fixing in the same change as an operator-approved behaviour change).
4. **Walk logs a warn for binary attachments** ("Could not read attached file from disk…") because the old UTF-8 read still runs first; harmless, can be skipped per modality later.
5. **No `FileReference → graph` edge** (unchanged from the graph-on-link report); the ledger holds the mapping.
6. Folder links still create one `FileReference` per direct child only.

## 6. Unverified (nothing compiled or run)
- `file_reader.rs`, `graphs.rs::graph_attached_file`, the `mod.rs` swap and the `graph_on_link.rs` edits parse (rustfmt); borrow shapes unchecked (`file_spec` is a reference into the cloned attachment vec; `state.request.project_id` read while `state` is `&mut`).
- JSON shapes were derived from the pipelines' serde enums (`{"FilePath": …}` external tags, struct variants, unit variants as strings). A first real run per modality is the proof; the first failure will show a serde message naming the field.
- Unit tests added in `file_reader.rs` (3) and kept in `graph_on_link.rs` (4) have not run.
- Depth header probes (LAS byte 104, PCD `FIELDS`/`DATA`) are from the formats' specs, untested on real files.
- First checks after the build: link one `.png`, one `.obj`, one `.step`, one `.las`, one `.iq` (expect `unreadable`) and watch `/callgraph/files` and the Call graphs tab.
