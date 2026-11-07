# Modality status — every modality, every action (2026-11-05)

Generated from the code (`assets/pipelines/modalities/*/main.rs`) after the doctrine + fabricated-output audits. **All edits are UNCOMPILED / UNTESTED** (rustfmt-parse only). Classifier: an action is **FAILS-HONESTLY** if this wave put an honesty guard on it, **STUB/SIMPLIFIED** if its body still says "In production / placeholder / simplified", otherwise **unmarked** (NOT the same as verified-real — only audio, video, image, text, code, math, chemistry, 3D AnalyzeScene, and the DNA real parts were verified by the audit).

| modality | actions | unmarked | fails honestly | stub / simplified | unit tests | status note |
|---|---:|---:|---:|---:|---:|---|
| 3D | 24 | 18 | 2 | 4 | 0 | AnalyzeScene honest; RunSimulation/BakePhysics now fail (no physics backend); headless sub-ops still write claiming notes. |
| BCI | 18 | 12 | 3 | 3 | 0 | Analyze, CalibrateDecoder, ExportProduct fail. |
| CAD | 19 | 16 | 1 | 2 | 0 | Analyze now fails (was guessing is_assembly); BOM/export paths unchanged (open). |
| IMU | 18 | 14 | 2 | 2 | 0 | Analyze + ExportProduct fail. |
| audio | 12 | 12 | 0 | 0 | 17 | Honest (audit): real analysis + the pattern the other modalities are being brought to. |
| biology | 15 | 11 | 3 | 1 | 0 | Analyze, BuildRegulatoryNetwork, ExportProduct fail. |
| chemistry | 11 | 11 | 0 | 0 | 8 | Honest (audit). |
| code | 13 | 13 | 0 | 0 | 11 | Honest (audit): real call graph (this wave: Python block ends, cross-file calls). |
| control | 21 | 18 | 2 | 1 | 14 | Analyze + ExportProduct fail; LQR/MPC math beyond Analyze NOT verified. |
| depth | 19 | 9 | 6 | 4 | 0 | Analyze, Register (identity ICP stub), ReconstructMesh, FuseFrames, ComputeNormals, ExportProduct fail; RANSAC plane + clustering are simplified-but-real. |
| dna | 13 | 12 | 0 | 1 | 4 | REAL: six-frame ORF scan, exact motif search, standard-code translation, composition stats. FAILS: annotation, alignment, structure, variants (canned). |
| eeg | 13 | 12 | 1 | 0 | 2 | FIVE analysis functions + extract_bands converted to honest errors (they were synthetic/random); only graph structure helpers remain. |
| electromagnetic | 16 | 13 | 2 | 1 | 0 | Analyze + ExportProduct fail. |
| geospatial | 20 | 14 | 3 | 3 | 0 | Analyze, TransformCoordinates, ExportProduct fail. |
| haptic | 16 | 14 | 2 | 0 | 0 | Analyze + ExportProduct fail. |
| hyperspectral | 17 | 13 | 2 | 2 | 0 | Analyze + ExportProduct fail; UnmixPixel is a spectral-angle proxy (rename open); DetectAnomalies simplified. |
| image | 9 | 9 | 0 | 0 | 15 | Honest (audit). |
| kinematics | 21 | 12 | 5 | 4 | 0 | Synthetic gait/biomechanics generator kept (labelled simulator); recorded sources + imports/export now fail; URDF/MJCF headless export still a stub. |
| math | 15 | 15 | 0 | 0 | 9 | Honest (audit): real mexpr parser; ProofStep reference prompt no longer requests confidence. |
| network | 17 | 10 | 4 | 3 | 0 | Analyze, DiscoverNetwork, AnalyzeCapture, ExportProduct fail. |
| proteomics | 17 | 13 | 4 | 0 | 0 | Analyze, AlignSequences, StructuralAlignment, ExportProduct fail. |
| radar | 15 | 14 | 1 | 0 | 0 | Analyze (helper) + ExportProduct fail. |
| sonar | 16 | 13 | 2 | 1 | 0 | Analyze + ExportProduct fail. |
| sound | 17 | 16 | 1 | 0 | 0 | Analyze fails (invented sample rates removed). |
| text | 16 | 16 | 0 | 0 | 5 | Honest (audit): real extraction; document MCP client. |
| thermal | 15 | 13 | 2 | 0 | 0 | Analyze + ExportProduct fail; FuseWithVisible kept as an ASSOCIATION record (relabelled honestly). |
| video | 11 | 11 | 0 | 0 | 13 | Honest (audit). |
| **total** | 434 | 354 | 48 | 32 | | |

## Actions that FAIL honestly now (were fabricated / fake-success)

- **3D**: RunSimulation, BakePhysics
- **BCI**: Analyze, CalibrateDecoder, ExportProduct
- **CAD**: AnalyzeFile
- **IMU**: Analyze, ExportProduct
- **biology**: Analyze, BuildRegulatoryNetwork, ExportProduct
- **control**: Analyze, ExportProduct
- **depth**: Analyze, ReconstructMesh, FuseFrames, Register, ComputeNormals, ExportProduct
- **eeg**: TriggerSemanticHook
- **electromagnetic**: Analyze, ExportProduct
- **geospatial**: Analyze, TransformCoordinates, ExportProduct
- **haptic**: Analyze, ExportProduct
- **hyperspectral**: Analyze, ExportProduct
- **kinematics**: AnalyzeGait, AnalyzeBiomechanics, ImportFromCAD, ImportFrom3D, ExportToControl
- **network**: Analyze, DiscoverNetwork, AnalyzeCapture, ExportProduct
- **proteomics**: Analyze, AlignSequences, StructuralAlignment, ExportProduct
- **radar**: ExportProduct
- **sonar**: Analyze, ExportProduct
- **sound**: Analyze
- **thermal**: Analyze, ExportProduct

## Actions still STUB / SIMPLIFIED (open — decide: implement for real, fail honestly, or keep as documented simplification)

- **3D**: AnalyzeRig, AnalyzeAnimation, AnalyzeMaterials, HeadlessProcess
- **BCI**: ExtractP300, ExtractSSVEP, HeadlessProcess
- **CAD**: CheckInterference, HeadlessProcess
- **IMU**: RunAHRS, Calibrate
- **biology**: HeadlessProcess
- **control**: Verify
- **depth**: Segment, ComputeOccupancy, ExtractPlanes, HeadlessProcess
- **dna**: TriggerSemanticHook
- **electromagnetic**: EstimateDoA
- **geospatial**: CoverageAnalysis, BuildSpatialIndex, HeadlessProcess
- **hyperspectral**: UnmixPixel, DetectAnomalies
- **kinematics**: ForwardKinematics, PlanTrajectory, ComputeWorkspace, HeadlessProcess
- **network**: PathAnalysis, SegmentNetwork, StreamToUI
- **sonar**: ModelPropagation

## Cross-cutting numbers (from the code)
- **Literal graded scores still typed into modality code:** 521 edge `weight:` literals (226 are 1.0 = "present"; 295 are 0.6–0.95, e.g. 0.8 ×110, 0.9 ×97), 272 node `hotness_score:` literals (0.5–0.8 typical), 15 `confidence:` literals. Not measured from data → same false-precision class as model confidence. **NOT removed** — only consumer logic found: `biology` filters nodes by `hotness_score > 0.8`; the UI shows edge Weight. Proposed (needs your decision): structural edge weight = 1.0, `hotness_score` = 0.0 unless derived from real use, fix the biology filter first.
- **Unit tests:** 16 of 27 modalities have none (3D, BCI, CAD, IMU, biology, depth, EM, geospatial, haptic, hyperspectral, kinematics, network, proteomics, radar, sonar, sound, thermal) — this is how fabricated output shipped. A test that asserts only `is_ok` passes on fabrication (the DNA/EEG tests were the examples; both rewritten).
- **Headless sub-ops** (`HeadlessProcess`/`ExportToControl` URDF/MJCF variants) in BCI, biology, depth, kinematics, EM, network, sound, thermal, geospatial, 3D: ~25 still push version notes claiming work that did not run (EDIT_MANIFEST item 28).
- **Previously failing-honestly modalities now lose their placeholder graphs:** `file_link` analyze:true on those 16 returns `analyzed:false` with the reason instead of a root-only graph (the E2E modalities suite uses analyze:false — unaffected).

## What I verified I did NOT drop
Compared every arm I made fail against its original (git HEAD): depth `Register` (ICP stub returning identity + a 5 mm mock error), BCI `CalibrateDecoder` ("Simulate 70-85% accuracy" + literal feature importance), depth `ReconstructMesh` (zero-count mesh flagged watertight), kinematics `ExportToControl` (unwritten /tmp path), network/proteomics/geospatial/the 16 `Analyze` arms (description-only, no parsing) — all fabricated, nothing real removed. ONE over-reach found and reverted: thermal `FuseWithVisible` records a real cross-modal reference node/edge; it was only mislabelled "fused" → restored, relabelled "association only (no pixel fusion)".
