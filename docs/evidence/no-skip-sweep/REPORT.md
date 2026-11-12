# No-skip sweep + step_index + acceptance == readability (2026-11-11)

Operator rulings applied (edit only; NOTHING compiled or run — rustfmt parse only; EU untouched):
1. "why would a step index ever repeat … not a fail or retry … just change the step index … we don't record failures and move on, all failures are to be fixed … or paused and retried … there is no skip."
2. "there should be NO unreadable files, because only accepted modalities/extensions link at all … acceptance == readability … a malformed file of an accepted type is rejected at LINK time with the exact reason."

## A. step_index — assigned centrally, never fails/replans
| Path | Old | New | file:line |
|---|---|---|---|
| `stages.rs::stage_3_validated` | duplicate index → replan from scratch → fail+pause | `normalize_step_indices` assigns `step_index = position` once, order preserved; `depends_on` remapped (unique index → that step; repeated index → carriers BEFORE the dependent step; dangling → dropped); an event `step_index_assigned` records what changed | stages.rs `normalize_step_indices`, `stage_3_validated` |
| `stages.rs::reconcile_tool_bindings` | remap by a map that collapsed duplicates (last wins) | indices normalized first (step 0) so the remap is never ambiguous | top of fn |
| stage-3 per-step salvage | a step that failed strict deserialize was **skipped** | rebuilt leniently from its own fields (text kept, defaults for the rest) — never dropped | `salvaged_steps` |
| `walk_expansion.rs`, capability steps (`next_step_index`) | max+1 | unchanged: unique by construction on a normalized plan |  |
`duplicate_step_indices` and the `plan_invalid_duplicate_step_index` event are removed (superseded by the above).

## B. Acceptance == readability (per-extension table)
One source: `assets/pipelines/shared/modality_ext.rs` (host via include!, file_link via #[path]). `modality_for_extension` holds only extensions `file_reader.rs` can prepare; `accept(path)` = table lookup + streamed well-formedness check (UTF-8 for text-shaped, LASF for LAS/LAZ, DATA line for PCD); `rejection_reason(ext)` gives the exact reason. `ModalityType::from_extension/from_path` no longer fall back to the per-modality `file_extensions()` lists (they would re-admit rejected extensions); path hints no longer pick a modality (every hinted modality needs parameters the file does not state). Link points: `FileLink` Link / LinkMultiple / LinkFolder (`rejected: [{path, reason}]` in the output) and `/orchestrate attached_files` (400-style refusal with the reason). A unit test (file_reader.rs) asserts every accepted extension has a reader.

| Modality / extensions | Decision | Why |
|---|---|---|
| text: txt md markdown rst adoc org | accepted, read whole | text pipeline |
| text: pdf doc docx rtf odt ods odp pptx ppt | accepted: `Prepared::Text("")` + `file_path` → text pipeline extracts via the document tool | a missing tool is a real failure → pause + retry |
| text (NEW fallback): csv tsv env gitignore dockerignore editorconfig; cif mol2 cml cdxml rxn ct; gff gff3 gtf vcf sam bed wig nex nexus phy phylip aln; graphml gexf dot gml net pajek cyjs; sbml owl obo biopax rxnml hmdb kegg | accepted AS TEXT | plain-text formats with no dedicated reader; structure-agnostic, nothing invented |
| code: all code + json jsonl yaml yml toml ini conf cfg xml; html htm xhtml css scss sass less svg | accepted, read whole | uidocument (127) has no pipeline binary; read as source text |
| image / audio / video (all listed) | accepted, by path | pipeline opens the file |
| math: tex latex nb m mpl maple mw wxm wxmx sage ipynb rmd qmd | accepted | text |
| chemistry: mol sdf pdb xyz smi smiles inchi | accepted | the 5 content variants |
| dna: fasta fa fna ffn faa frn fastq fq gbk gb genbank | accepted | sequence by path |
| eeg: edf bdf gdf set fif vhdr vmrk eeg cnt avg mff ncs nev ns2–ns6 plx | accepted, by path | pipeline sniffs |
| 3d: obj glb gltf stl ply | accepted | the 5 formats the 3D pipeline reads |
| cad: step stp iges igs brep occ f3d fcstd sat x_t x_b sldprt sldasm | accepted | typed sources |
| kinematics: urdf mjcf bvh c3d osim (+ sdf by path rule) | accepted | typed sources |
| control: slx mdl simulink | accepted | self-describing |
| thermal: rjpeg fff | accepted | radiometric, self-describing |
| depth: pcd las laz | accepted + header validated | flags read from the header |
| geospatial: geojson kml kmz gpx shp osm pbf | accepted | typed sources |
| proteomics: mmcif | accepted | self-describing |
| REJECTED (reason shown at link): zip tar gz rar 7z bz2 xz; xls xlsx; bam cram bigwig bw 2bit; cdx mae maegz; xdf bcilab; blend fbx dae usd usda usdc usdz 3ds c4d ma mb world; kin dh; fmu fmi; seq csq ir2; pts e57 zdf; dbf shx prj; dem asc grd nc; mrc map dx ccp4 xplor mzml mzxml wiff; imu bag mcap; iq rf sigmf cf32 cs16; cphd sicd nitf ntf rs2; xtf jsf s7k gsf all wcd xse; hdr bil bip bsq h4 hdf4 img; hdf5 h5 haptic force mat | **stop accepting** | no reader without inventing parameters the file does not state (or no unpacker/extractor) |

Option NOT taken (follow-up, per modality when wanted): have the sensor pipelines (imu haptic radar sonar em hyperspectral bci biology) accept raw input with parameters recorded as UNSPECIFIED (structure-only graph) — needs each pipeline to change (uncompiled), so it was not done blind; until then those modalities have no accepted extension. Also: a FileReference linked BEFORE this rule whose extension is no longer accepted shows `not_accepted` (reason + "unlink it") in /callgraph/files — it is not guessed at; this is an operator-pending state (exception below).

## C. No skip — every place that recorded a failure and moved on
| Path | Old | New | Where |
|---|---|---|---|
| walk attached-file graph creation | `graph_failures` recorded, walk continued from chunked text | returns Err → walk PAUSES, resume retries (graph-on-link may build it meanwhile; ledger reuse) | mod.rs STEP 0 |
| walk attached text-shaped file unreadable | silently fell back to a 512-byte preview | Err → pause + retry (binary modalities still use the preview label — they open by path) | mod.rs STEP 0 |
| prompt modality graphs (Analyze / ParseExpression / CreateGraph) | `unwrap_or_default()` / logged-and-`continue` | `?` / Err → pause + retry; no graph from an empty analysis | graphs.rs `create_initial_modality_graphs` |
| `graph_attached_file_build` CreateGraph | `unwrap_or_default()` | `?` | graphs.rs |
| `graph_failures` machinery (`record_graph_failure`, checkpoint field, `wo_graph_retry` items, link worker 2b poll, ticks) | record + retry item | REMOVED — superseded by pause + retry (no remaining producer) | graphs.rs, mod.rs, amt.rs, run_checkpoint.rs, work_order.rs, graph_on_link.rs |
| graph_on_link job failure | "recorded; the next trigger resumes it" | re-queued after 5 s doubling (held 60 s), per-file, until it succeeds; one failing file never blocks others | graph_on_link.rs worker |
| graph_on_link `unreadable` / `not_text_readable` / `file_unreadable` | recorded, never retried | gone. Read errors → `failed_read` + Err (retried). Missing file → `file_missing` (resumes by itself on the beacon's created/updated event). Legacy ref with a no-longer-accepted extension → `not_accepted` | graph_on_link.rs `process_job` |
| capability closure — methodology draft | "could not be drafted this walk" and moved on | Err → pause + retry (`closure_attempted` entry removed; a durable `methodology_drafted` status prevents a duplicate draft on resume) | stages.rs `close_capability_gaps` |
| tool creation (draft / parse / write / registry) | "nothing created … next walk retries" | Err → pause + retry; operator switch off and "a tool of that name exists" stay `Ok(None)` (by design) | stages.rs `create_tool_for_capability` |
| expansion rounds | closure result ignored | `?` | walk_expansion.rs |
| boot: system call graphs | one-shot best-effort | retried 5 s doubling (held 60 s) until every graph is ensured and the asset found; boot is not blocked | lib.rs |
| MCP agent steps | await the agent's report, no timeout, no fail-open | unchanged (already doctrine-conformant) | agent_requests.rs |

## D. Real doctrine exceptions (NOT retried / not forced)
1. Operator-gated decisions (marketplace accept/decide, retry-vs-cancel of a parked child, unlinking a `not_accepted` legacy reference).
2. Kill switches the operator set to off (`OZONE_TOOL_CREATION`, `OZONE_GRAPH_ON_LINK`, refinement switches …) — `Ok(None)`.
3. Privacy withholding of a created tool's publication: `publish_withheld` with the recorded reason; the tool stays created locally; pending operator decision.
4. Refinement/removal gates: "no evidence = no action" (asymmetric gates) — `continue` there is the doctrine, and the refinement pass re-runs every meta pass (retry by design).
5. A linked file that no longer exists (`file_missing`): nothing a retry can change; resumes by itself when the file reappears.
6. Observability bookkeeping appends (hit/judgment/event jsonl, listing/inbox writes: ~40 sites labelled "best-effort") — a failed LOG write is not skipped WORK; NOT rewritten here. Candidates for a later sweep where a durable record matters (e.g. paired-peers store, `network/mod.rs`, marketplace listing write).
7. Local binary-modality `content_preview` label in STEP 0 (not a skip: the pipeline reads the file by path).

## E. Unverified (nothing compiled)
- All Rust/TS/JS edits are parse-checked only. At the first `cargo check` look at: `close_capability_gaps` / `create_tool_for_capability` brace + type shape; `graph_on_link.rs` worker (`attempts` map, `retry_tx`, `job` moved into the retry task after `&job` borrow); `process_job` use of `prev`/`hash`; `file_reader.rs` tests (`pipeline_id_for_modality` is `pub(crate)`); `modality_ext::validate_file` inside `include!`; file_link `RejectedFile`/`rejected` literals (every `FileLinkOutput {..}` has the new field); grpc `attached_files` check.
- Behaviour change to confirm on a real walk: a text-graph failure now PAUSES the walk instead of answering from the chunked text.
- `from_extension/from_path` no longer fall back to `file_extensions()`: any host code that relied on e.g. `.dae` → ThreeD now sees Unknown.
