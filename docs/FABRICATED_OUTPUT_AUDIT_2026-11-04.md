# Fabricated / stub output audit — 2026-11-04 (read-only; nothing in `src/ assets/ tools/ ui/` was edited)

Rule (AGENTS.md "No fabricated metrics anywhere"; docs/CONTRACTS.md §5.1 "Absence is None/false/0.0 — never an invented default"; failures are data):
an analysis/result presented as REAL must come from the input; a stub must FAIL honestly ("not implemented — no analysis performed"), never return
synthetic numbers or a success.

**Reference patterns (already honest — copy these):** `audio/main.rs:2018-2050` (`absent_reason: Some(ABSENT_ASR)`, zero fields), `video/main.rs:2117`
(`track_objects` → `Err("object tracking is absent: …")`, D16), `3D/main.rs:2308` (`glue::analyze_scene` real readers; unsupported source = honest error),
`hyperspectral/main.rs` `AtmosphericCorrection` ("REQUESTED — NOT applied"), and the EEG conversion made 2026-11-04 (`EEG_ANALYSIS_NOT_IMPLEMENTED`).

Classes: **F** fabricated/synthetic output presented as real · **S** simplified but REAL on the real input · **N** unimplemented action returns success/empty ·
**G** explicit simulator/synthetic-data feature, labelled · **T** test-only.

## 0. Headline

- The dominant defect is **not** a handful of stubs: **16 of the 27 modality pipelines never had the honesty pass.** Their `Analyze` action builds a
  `source_description` string from the request and returns `success: true` with `..Default::default()` — an EMPTY analysis (class N) — and `file_link` then
  `CreateGraph`s a root-only graph from it. That is exactly why the fan-out coverage run found graphs with zero edges for control, electromagnetic, haptic,
  hyperspectral, network, proteomics, radar, sonar, sound, thermal.
- Beyond `Analyze`: **16 modalities' `ExportProduct` returns a `/tmp/…` path with `success: true` and writes nothing** (class F), ~25 headless/"operation"
  arms push a `VersionNote` claiming work that was never done ("Fused N frames", "Normals computed", "Re-decoded all motor imagery epochs", "FK computed for all
  chains"…), and several real-sounding analytics return literals (DNA ORF/gene/alignment/translation/variants, proteomics alignment + TM-score 0.85, BCI
  decoder accuracy, depth ICP/mesh, 3D physics cache, kinematics gait, geospatial CRS transform that leaves coordinates unchanged).
- Honest already: **audio, video, image, text, code, math, chemistry (mostly), 3D `AnalyzeScene`/`AnalyzeMesh`**, all `tools/*` MCP servers (mesh, dna-mcp,
  proteomics-mcp, document, visual, browser, android… state "never placeholder/never faked" and the scan found no counter-example), the document/uidocument path.
- Counts (distinct findings, grouped rows below): **F 37 · N 20 · S 12 · G 4 · T 2.** Consumers: `file_link` analysis → modality graph creation → span-out/fan-out,
  Marketplace "Relations", cross-modal links; the UI shows export paths and version-note text verbatim.

## 1. Findings by pipeline

### 1.1 DNA — `assets/pipelines/modalities/dna/main.rs` (severity HIGH: `.fasta` ingest, `dna-mcp` is the honest alternative)
| # | file:line | Snippet / what the caller sees | Class |
|---|---|---|---|
| D1 | 1051-1075 `analyze_sequence` | `SequenceData::_ => "ATGCATGCATGC"` — for any non-Raw/non-FASTA source (e.g. `FilePath`) the "sequence" IS a literal; composition/GC are then computed on it | **F** |
| D2 | 1107-1121 | `find_orfs` → ONE ORF `orf_1`, `start 1`, `end min(len,300)`, `protein "MXXX..."` regardless of content | **F** |
| D3 | 1103-1105, 1087 | molecular weight `len*330`, melting-temp formula — real arithmetic on real `seq`, labelled "Approximate" | S |
| D4 | 1142-1208 `annotate_sequence` | `ExampleGene/EXG1` 100–500, 2 exons, promoter, `confidence 0.85/0.80`; ignores `_feature_types`, never reads the sequence | **F** |
| D5 | 1212-1243 `align_sequences` | every sequence "aligned" to `"ATGC--ATGC"`, score 85, identity 80, consensus `ATGCNATGC`, conservation literal | **F** |
| D6 | 1245-1272 `find_motifs` | ignores `_sequence`, returns a literal motif (score 0.85) | **F** |
| D7 | 1273-1298 `predict_structure` | ignores `_sequence`, literal structure (confidence 0.75) | **F** |
| D8 | 1300-1320 `translate_sequence` | protein `"MMMM"` for any input; non-Raw input → `"ATGATGATGATG"`; `stop_codons: []` | **F** |
| D9 | 1323-1360 `find_variants` | ignores query+reference, returns one invented SNP (position 100, A→G, `Gene1`, `A100G`) | **F** |
| D10 | 1948-1960 `test_analyze_sequence` | asserts only `result.is_ok()` — passes on the fabricated ORF | **T** |

### 1.2 EEG — `eeg/main.rs`
| # | file:line | | Class |
|---|---|---|---|
| E1 | 1207-1236 `extract_bands` | **NOT converted yet** (the 2026-11-04 conversion covered 5 functions, not this one): hard-coded channels `Fp1/Fp2/Cz`, `absolute_power 15+rand_float()*20` (clock-nanosecond noise), ignores `_eeg_data/_window_seconds/_overlap` | **F** |
| E2 | 1362-1388 `extract_epochs` | windows around CALLER-SUPPLIED event times (real arithmetic) but `data: HashMap::new()`, `rejected_epochs: 0` — no signal was read | N |
| E3 | 1151-1155 `rand_float` | still defined; after E1 is fixed it should be deleted | — |
| E4 | five converted fns | now `Err(EEG_ANALYSIS_NOT_IMPLEMENTED)` (reference pattern) | done |

### 1.3 Depth — `depth/main.rs` (severity HIGH: registration/mesh feed cross-modal links)
| # | file:line | | Class |
|---|---|---|---|
| P1 | 1052-1062 `icp_point_to_point` + 1690-1702 `Register` | returns `Transform4x4::identity()` and `rmse = 0.005; // 5mm placeholder`; action reports `success: true` + the identity as the registration | **F** |
| P2 | 1658-1673 `ReconstructMesh` | `vertex_count 0, face_count 0, has_normals true, is_watertight = matches!(method, TSDF\|Poisson)` — a mesh object with no mesh and flags derived from the method NAME | **F** |
| P3 | 1675-1682 `FuseFrames` | bumps `graph.version`, pushes note "Fused N frames using M", fuses nothing | **F** |
| P4 | 1704-1711 `ComputeNormals` | note "Normals computed: radius… orient…" — nothing computed | **F** |
| P5 | 1684-1689 `Segment`, 1713-1717 `ComputeOccupancy` | `success: true`, empty `clusters` / no payload ("In production: …") | N |
| P6 | 1927-1930 headless `ExportMesh` | empty arm; 1882-1896 `ExportProduct` returns `/tmp/depth_export_…` and writes nothing | **F** |
| P7 | 1606-1627 `Analyze` | description only (class N-wide, §2) | N |
| P8 | 1719-1750 `ExtractPlanes` (RANSAC on centroid proxy points), 1117-1150 `euclidean_cluster` (real O(n²) BFS), `ransac_plane` | real algorithms on the supplied points; limitation = proxy points, document it | S |

### 1.4 BCI — `BCI/main.rs`
| # | file:line | | Class |
|---|---|---|---|
| B1 | 2310-2390 `CalibrateDecoder` | "Stratified accuracy estimate … Simulate 70-85% accuracy based on feature SNR": `cv_accuracy`, `kappa`, confusion matrix and `feature_importance {beta 0.35, alpha 0.25, high_gamma 0.20, csp 0.15, ersp 0.05}` are formula/literals — **no decoder is trained** | **F (severe)** |
| B2 | 2631, 2703-2710 headless | notes "Re-decoded all motor imagery epochs", "Decoder updated with N new calibration trials"; ROS-bag export empty | **F** |
| B3 | 2109, 2197 | channel contributions "simplified: equal weight"; phase lag "simplified estimate" — computed from real input | S |
| B4 | 193-200, 1833 `BCIDataSource::Synthetic` | simulator input, labelled "Synthetic {paradigm}…" in `source_description` | G |
| B5 | 1820-1850 `Analyze` | description + declared electrode_count/sample_rate only | N |

### 1.5 3D / CAD / kinematics / control
| # | file:line | | Class |
|---|---|---|---|
| T1 | 3D 2560-2590 `RunSimulation`, 2589 `BakePhysics` | adds a `PhysicsCache` node "Simulation ran: N frames", returns `simulation_frames`; no physics backend is called ("In production: dispatch to physics backend") | **F** |
| T2 | 3D 2790-2800 headless `ExportScene`/IK solve | empty arms | N |
| T3 | CAD 1836-1860 `AnalyzeFile/Part/Assembly/Drawing` | `is_assembly` GUESSED from the extension (`F3D/SLDASM/IAM/CATPRODUCT` = true), nothing parsed; use `mesh-mcp cad_step_info` for real STEP | **F-lite → N** |
| T4 | CAD 1956 interference check, 2199 BOM CSV, 2085/2094 export paths | "In production: …"; exports return `/tmp/cad_export_…` unwritten | **F** |
| K1 | kinematics 1962-2010 `AnalyzeGait` | for BVH/C3D/IMU sources: `speed_ms 1.2, step_length 0.6, stride 1.2, cadence 60, symmetry_index 1.0` + canned 7 gait phases + 4 step events — only the `Synthetic` variant is honest | **F** |
| K2 | kinematics 2028-2060 `AnalyzeBiomechanics` | OpenSim/other sources default to 70 kg / 1.75 m | **F** |
| K3 | kinematics 2696-2716 `ImportFromCAD/3D` | "In production: load CAD graph…" builds an analysis labelled "Imported from CAD graph N" with no joints read | **F** |
| K4 | kinematics 2720-2725 `ExportToControl`, 2782-2808 headless `ComputeAllFK/DetectAllSingularities/ExportURDF/ExportMJCF` | path `/tmp/kin_control_export_…` unwritten; notes "FK computed for all chains", "Singularity detection completed"; URDF/MJCF arms empty | **F** |
| K5 | kinematics 1049-1075 FK ("simplified: accumulate position only"), 1132 singularity via trace proxy, 2421 sweep joint 0 only | real math, documented simplifications | S |
| K6 | kinematics 106/113/1968-2033 `GaitDataSource::Synthetic`, `BiomechanicsDataSource::Synthetic` | labelled "Synthetic …" | G |
| C1 | control 2269-2290 `Analyze` | description only | N |

### 1.6 Proteomics / geospatial / network / EM / biology / sound / haptic / IMU
| # | file:line | | Class |
|---|---|---|---|
| R1 | proteomics 2280-2305 `AlignSequences` | identity = 100·(1 − length difference/max length) (a LENGTH ratio, not alignment), `similarity = identity+5`, `coverage 95`, `e_value 1e-50`, `bit_score 500` — "In production: call ClustalOmega/MUSCLE/MAFFT" | **F** |
| R2 | proteomics 2308-2322 `StructuralAlignment` | literal `tm_score 0.85, rmsd_angstrom 1.4, aligned_residues 200` — "In production: run TM-align / DALI" | **F (severe)** |
| G1 | geospatial 2046-2054 `TransformCoordinates` | "identity transform with metadata": coordinates UNCHANGED, `crs` field rewritten to `to_crs` — wrong data labelled as transformed | **F (severe)** |
| G2 | geospatial 2131 `BuildSpatialIndex`, 2298 `ExportLayer` | success/no-op; note "Exported … to {path}" with nothing written | N/F |
| G3 | geospatial 1782-1806 `Analyze` | description + `crs: "EPSG:4326"` invented default | N |
| NW1 | network 2013-2027 `DiscoverNetwork` | no scan; "hosts" = the caller's own `host_list`, `last_seen = generate_id()/1e9` (a fabricated timestamp), `is_internal: true` | **F** |
| NW2 | network 2029-2036 `AnalyzeCapture` | pcap path echoed in `source_description`, empty analysis, `success: true` ("In production: libpcap/scapy") | N |
| NW3 | network 2548-2550 `StreamToUI`, 2570/2638/2655 notes "Path index recomputed / Segmentation analysis run / Resilience metrics computed" | claims, no compute | **F** |
| EM1 | electromagnetic 2257-2262 headless `ReRunCoverage` | "In production: …" bumps `graph.version`, recomputes nothing | **F** |
| BI1 | biology 2405-2413 `ReclusterCellTypes`/`InferSignalingPaths`, 2445 export | notes "Cell type clustering re-run", "Signaling path inference run"; SBML/BioPAX export empty | **F** |
| BI2 | biology 2031-2060 `Analyze` | description + `organism` only | N |
| SO1 | sound 2026-2050 `Analyze` | invents `sample_rate_hz 44100, channel_count 1, recording_duration_sec 0.0`; 2461 note "Re-ran species ID" | **F** |
| HA1 | haptic 2044-2068 `Analyze` | `sample_rate_hz: 1000.0` for every source (not read from the file) | **F-lite** |
| IM1 | IMU 1978-2000 `Analyze` | default sample rates `100.0` (CSV) / `200.0` (ROSBag, benchmark) when none declared | **F-lite** |
| TH1 | thermal 1639-1662 `Analyze`, 1956 note "Heat balance computed" | description only; claim note | N/F |
| SN1 | sonar 1684-1720 `Analyze` | description only | N |
| HS1 | hyperspectral 2024-2080 `Analyze` | derived from the REQUEST's declared band_count/wavelength range (caller-supplied metadata, not read from the cube) — honest if labelled "declared" | N |
| HS2 | hyperspectral 2126-2160 `UnmixPixel` | named FCLS but computes spectral-angle similarity, normalised to sum 1 — a real proxy on real spectra, mis-named | S (rename) |
| RA1 | radar 2150-2205 `analyze_radar_data` | `let _ = (extract_targets, extract_weather, …)` — every extraction flag ignored; description-only analysis | N |

### 1.7 Cross-cutting
| # | where | | Class |
|---|---|---|---|
| X1 | `ExportProduct` in **BCI 2606-2617, 3D 2620, biology 2378, CAD 2085/2094, haptic 2335, control 3014, depth 1882-1896, hyperspectral 2413, geospatial 2246, kinematics 2724, IMU 2739, thermal 1921, sonar 1971, proteomics 2460, network 2544, sound 2448** | returns `export_path: Some("/tmp/<mod>_export_…")` with `success: true`; no file is written (verified near the return in 14 of 16; `control` and `network` show a write nearby — re-verify) | **F** |
| X2 | `weight: 0.85/0.9/0.95` and `hotness_score: 0.75–0.9` literals in the graph builders of **3D, BCI, biology, CAD, control, depth, EM, geospatial, haptic, hyperspectral, IMU, kinematics, network, proteomics, radar, sonar, sound, thermal** (≈ 330 edge-weight and ≈ 270 hotness literals) | invented scores on edges/nodes; same family as the retired `confidence` (CONTRACTS §5a) — structural edges should be 1.0, hotness derived from use | **F-lite** |
| X3 | `Analyze` that returns an empty analysis (N-wide): geospatial, biology, proteomics, electromagnetic, network, BCI, control, radar, depth, thermal, sonar, sound, haptic, IMU, hyperspectral, CAD | `file_link` → `CreateGraph` → root-only graph; the graph looks analysed | N |

### 1.8 General / consciousness / host (`src/`)
| # | file:line | | Class |
|---|---|---|---|
| H1 | `general/sync/main.rs:145` `load_or_generate_peer_id` | "simplified — would use libp2p in production": generates `12D3KooW` + clock-nanosecond hex and persists it as the peer id; the REAL identity is `<data_dir>/p2p_identity.key` (network/mod.rs) — this is a second, fake identity | **F** |
| H2 | `consciousness/collective_consciousness/main.rs:556` `extract_pattern` (used at :125) | returns `"[N word pattern about general topic]"` — placeholder text stored as the "Pattern" of a contribution | **F** |
| H3 | `src/integrity/mod.rs:298` | `data_loss_risk = if estimated_data_loss > 0 { 0.5 } else { 0.0 }` — an invented risk number shown to the operator | **F** |
| H4 | `src/zsei/hooks.rs:434-441, 462-469` `process_single_hook`/`process_graph_hooks` | "For now, return placeholder": `NoAction` / `Ok(())` — the queued-hook path does nothing (the direct `process_*` hook fns are real LLM work) | N |
| H5 | `src/task/mod.rs:2087-2145` `run_modality_refinement`, `run_deduplication`, splitting recs | `// TODO` bodies that log and return `Ok(())` (daemon is not started — `start_refinement_daemon` has no caller outside docs) | N |
| H6 | `src/zsei/traversal.rs:479-505` ML traversal | "simplified — in production would use actual ML inference": constant 0.6/0.4 "ML-learned weights" when a model file exists | S (mislabelled "ML") |
| H7 | `general/auth/main.rs:24` session store | in-pipeline store, documented | S |
| H8 | `general/traversal_ml/main.rs:394` | keyword-match results get `distance: 0.0` | S |
| H9 | `general/methodology_create` Propose | already an honest `Err(not implemented …)` | ok |

### 1.9 Tools (`tools/**`)
Clean: the scan (`Math.random`, placeholder, mock, fake, canned, stub, TODO) found only test/selftest files (`browser-farm-mcp/selftest.mjs` — contract test vs a MOCK farm, labelled in its own output), id generation (`android-mcp` `Math.random` for a tool-call id) and honesty comments. **No fabricated result in any MCP server.** `threed-mcp clock_generate`/`cad_clock_generate` call real Blender/CadQuery (G: generators by design).

## 2. Unit tests that must change
1. `dna/main.rs:1948 test_analyze_sequence` — assert the analysis carries no fabricated ORF for a sequence with none, and that a non-Raw source errors instead of analysing `"ATGCATGCATGC"`.
2. `eeg/main.rs test_analyze_eeg` — already updated 2026-11-04 (asserts Err); add the same for `extract_bands` once converted.
3. New: one "unimplemented analysis must error" test per converted modality (`Analyze` on a nonexistent path must NOT return `success: true`); `ExportProduct` must not return a path that does not exist on disk.
4. `video/main.rs:2290`, `audio/main.rs:3060` (nonexistent inputs) already assert errors — the reference tests.
5. Only `control` (14), `chemistry`, `math`, `image`, `text`, `audio`, `video`, `code` have tests; **BCI, biology, depth, EM, geospatial, haptic, hyperspectral, IMU, kinematics, network, proteomics, radar, sonar, sound, thermal, 3D, CAD, dna(1) have none** — which is how all of the above shipped.

## 3. FIX LIST (F and N items only, ordered by blast radius; each is mechanical)
1. **X3 — 16 `Analyze` actions** (geospatial 1782, biology 2031, proteomics 2157, EM 1525, network 1859, BCI 1820, control 2269, radar 2205, depth 1606, thermal 1639, sonar 1684, sound 2026, haptic 2044, IMU 1978, hyperspectral 2024, CAD 1836): replace the description-only `Ok(success: true, analysis: …Default)` with the audio pattern — `success: false`, `error: "<modality> analysis is not implemented: no analysis was performed on <path> (use <provider> MCP)"`; keep `CreateGraph` for caller-supplied analyses only. Removes the root-only "analysed" graphs.
2. **X1 — `ExportProduct` ×16** (list in §1.7): return `Err("export not implemented: nothing was written")`, or actually write the file (JSON/CSV is trivial) and return the real path; never return an unwritten `/tmp` path with `success: true`.
3. **DNA D1–D9** (`dna/main.rs` 1062, 1107, 1142, 1212, 1245, 1273, 1300, 1323): D1 — error on non-Raw/FASTA sources; D2 — real ORF scan (6-frame ATG…stop, easy) or empty list + `absent`; D4–D9 → `Err("… not implemented; use dna-mcp")`; fix test D10.
4. **Version-note claims** (depth 1675/1704, BCI 2631/2703-2710, kinematics 2782/2796/2804-2808, EM 2257, biology 2405-2413/2445, network 2570/2638/2655, sound 2461, thermal 1956, geospatial 2298, 3D 2790-2800): delete the note or make the arm `Err("not implemented: nothing was done")`; a version bump must reflect a real change.
5. **B1 BCI `CalibrateDecoder`** (2310-2390): remove accuracy/kappa/confusion/feature_importance; return `Err` until a real LDA/SVM exists, or compute real LDA on `training_data` and report measured leave-one-out accuracy.
6. **R1/R2 proteomics** (2280-2322): `Err(not implemented: use proteomics-mcp)`; remove literals `0.85`, `1.4`, `1e-50`, `500`, `95`.
7. **P1–P4, P6 depth** (1052, 1658, 1675, 1704, 1927, 1882): ICP → `Err` (or implement point-to-point ICP with a real KD-tree/SVD); `ReconstructMesh` → `Err`; set `has_normals`/`is_watertight` only from real data.
8. **G1 geospatial `TransformCoordinates`** (2046): `Err("CRS transform not implemented (no PROJ binding)")` — never relabel unchanged coordinates; likewise drop invented `crs: "EPSG:4326"` default (1782): use `None`.
9. **T1 3D `RunSimulation`/`BakePhysics`** (2560-2590): `Err("no physics backend")`; delete the fabricated `PhysicsCache` node.
10. **K1–K4 kinematics** (1962-2060, 2696-2725, 2782-2808): non-Synthetic gait/biomechanics → `Err`; remove 70 kg/1.75 m and 1.2 m/s defaults; `ExportToControl`/URDF/MJCF → real serialisation or `Err`.
11. **E1 EEG `extract_bands`** (1207): same `EEG_ANALYSIS_NOT_IMPLEMENTED` conversion; then delete `rand_float` (1151) and E2's empty `data`/`rejected_epochs: 0` claim.
12. **NW1–NW3 network** (2013, 2029, 2548-2655): `DiscoverNetwork` must not return hosts it did not discover (drop fabricated `last_seen`/`is_internal`); `AnalyzeCapture` → `Err`.
13. **SO1/HA1/IM1** invented sample rates/channel counts (sound 2026-2050, haptic 2044, IMU 1978-2000): `None`/absent unless read from the file or declared by the caller (and then label "declared").
14. **H1** `general/sync/main.rs:145`: delete the fake peer-id generator; read the real identity (`p2p_identity.key`) or fail.
15. **H3** `src/integrity/mod.rs:298`: remove `data_loss_risk` (or derive from counted dependents) — never 0.5.
16. **H2** `collective_consciousness extract_pattern` (556): return the content or `None`, not `"[N word pattern about general topic]"`.
17. **T3/T4 CAD** (1836-1860, 1956, 2085/2094, 2199): do not guess `is_assembly`; parse via `mesh-mcp cad_step_info` for STEP or error; no unwritten export paths.
18. **H4/H5** (`zsei/hooks.rs:434/468`, `task/mod.rs:2087-2145`): `Err`/`NoAction{reason:"not implemented"}` instead of `Ok(())`; or delete the dead daemon bodies.
19. **X2** invented edge weights/hotness (≈ 600 literals, 18 modalities): structural edges = 1.0; remove `hotness_score` literals or derive from real use — pair this with the `confidence` retirement (EDIT_MANIFEST_2026-11-04 item 23).
20. **HS2/H6** (S, rename only): `UnmixPixel` → "spectral-angle proxy" in the method name; ML traversal → "weighted hybrid traversal".

## 4. Not verified / out of scope (stated plainly)
`control/main.rs` beyond `Analyze` (3,597 lines, real LQR/MPC math suspected — unverified), `chemistry`, `radar` actions other than `Analyze`, `sonar/sound/thermal/hyperspectral/haptic/IMU` non-`Analyze` actions (only sampled), `electromagnetic` beyond the headless arm, `uidocument/src/*` (separately reviewed in docs/UIDOCUMENT_ENGINE_REVIEW_2026-10-31.md), `code_analysis` tree-sitter paths, `src/` modules not matched by the keyword scan, `tools/bridges` TypeScript internals, and the Python workers (`tools/*/*.py` scan found nothing).


---

## 5. FIX STATUS (2026-11-05, edits applied under "no build, no test" — every edited `.rs` parses under `rustfmt --check`; NOT compiled, NOT run)

| # | Status | What was done |
|---|---|---|
| 1 X3 `Analyze` ×16 | ✅ | geospatial, biology, proteomics, EM, network, BCI, control, radar (helper), depth, thermal, sonar, sound, haptic, IMU, hyperspectral, CAD → `success: false` + honest error (no more description-only "success"; `file_link` no longer builds a root-only graph from nothing) |
| 2 X1 `ExportProduct` | ✅ ×14 | the 14 modalities that have the arm now fail (nothing is written); the audit's other 2 not located — verify at compile |
| 3 DNA D1–D10 | ✅ | placeholder sequence → error; REAL six-frame ORF scan; annotation / alignment / structure / variants → honest errors; REAL exact motif search (both strands, real positions); REAL standard-code translation; tests replaced (`real_orf_scan_and_translation_use_the_input`, …) |
| 4 version-note claims | ◻ partial | explicit arms fixed (depth FuseFrames + ComputeNormals, thermal FuseWithVisible, biology BuildRegulatoryNetwork, 3D RunSimulation + BakePhysics); the `HeadlessProcess` sub-ops in BCI/biology/depth/kinematics/EM/network/sound/thermal/geospatial/3D still push claiming version notes — list in EDIT_MANIFEST |
| 5 BCI `CalibrateDecoder` | ✅ | fails (no simulated accuracy/kappa/feature table) |
| 6 proteomics | ✅ | AlignSequences + StructuralAlignment fail (no TM 0.85 / RMSD 1.4) |
| 7 depth | ✅ | ReconstructMesh + Register (ICP) fail; FuseFrames/ComputeNormals fail |
| 8 geospatial | ✅ | TransformCoordinates fails (never relabels unchanged coordinates) |
| 9 3D physics | ✅ | RunSimulation + BakePhysics fail (no fabricated PhysicsCache) |
| 10 kinematics | ✅ | recorded-source gait/biomechanics fail (only the labelled Synthetic generator runs, no invented 70 kg / 1.2 m/s); ImportFromCAD / ImportFrom3D / ExportToControl fail; URDF/MJCF headless export still open |
| 11 EEG | ✅ | `extract_bands` converted too; `rand_float` marked do-not-use |
| 12 network | ✅ | DiscoverNetwork + AnalyzeCapture fail |
| 13 invented sample rates | ✅ | covered by the Analyze failures (sound / haptic / IMU) |
| 14 sync peer id | ✅ | fake generator deleted; reads the host's real id, which `network/mod.rs` now writes to `<data_dir>/p2p/peer_id` |
| 15 integrity `data_loss_risk` | ✅ | derived ratio, 0.0 when nothing is affected — never 0.5 |
| 16 collective `extract_pattern` | ✅ | returns the content's own summary, never "[N word pattern…]" |
| 17 CAD | ◻ partial | Analyze fails (no guessed `is_assembly`); unwritten export/BOM paths not changed |
| 18 hooks / daemon | ◻ partial | `process_batch` now `Err` (not a fake `Ok`); refinement-daemon TODO bodies untouched (the daemon is never started) |
| 19 invented weights/hotness (~600) | ◻ | pair with confidence retirement — not done |
| 20 renames | ◻ | not done |
