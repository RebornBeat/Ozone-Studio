### Gaps Identified in Existing Modalities (Quick Summary – What Was Missing)
From our prior reviews:
- **Relationship inference** was weak across the board (especially Text: missing grammar-based edges like Performs, Affects, Implies, Contradicts, TemporalPrecedes, PartOf, FunctionalRole, etc.).
- Most relied too heavily on zero-shot LLM surface extraction without deep structural/grammar/SRL/discourse analysis.
- Missing **cross-modality hooks** and **provisional node** logic in some.
- **Voice-specific** separation from raw Audio (speaker ID, prosody, emotion, intent layers).
- **Biology coverage** in DNA/EEG is narrow — lacks full cellular, tissue, organ, physiological, and multi-scale integration.


### Speech (110)
Raw audio (103) typically handles waveforms, spectrograms, acoustic tokens → low-level features like timbre, energy, pitch contours, noise, environmental sound. Speech adds **linguistic + paralinguistic** layers that require very different processing:

- Phoneme sequences, prosody (duration, intonation, stress), speaker identity/diariation
- Higher-level: dialogue acts (question, command, backchannel), emotional valence, intent primitives
- Temporal structure: turn-taking, overlap, repair sequences

**Verdict:** Keep as **110**, tightly linked to 103 via "SpokenFromRawAudio" or similar.

### Biology multi-scale (111) vs. DNA/Proteomics (112) — separate or merge?
Biology is hierarchical: DNA sequence → gene regulation → RNA → proteins → folding/structure → complexes → pathways → cells → tissues → organs → physiology/pathophysiology.

- Genomics/DNA is already covered (assume earlier modality, e.g., 104/105 sequence layers).
- Proteomics (112) focuses on sequence → 3D fold → PTMs → interactions (PPI networks, docking).
- Multi-scale biology (111) bridges upward: metabolic/signaling networks, cellular processes, immunology, tissue-level organization, emergent pathophysiology.

**Recommendation:**
Keep **112 Proteomics & Protein Structure** separate — it has very strong, direct bridges to 109 (3D atomic/coarse-grained) and 106 (Chemistry: ligands, reactions, QM properties).
Keep **111 Biology / Multi-Scale** as its own modality but **narrow its scope slightly** to emphasize the upward integration (cells ↔ tissues ↔ organs ↔ systems ↔ pathophysiology), explicitly excluding pure sequence/folding which live in 112 (and genomics if already covered). This avoids massive overlap while allowing rich relationships: ProteinFoldsInto → CellularComponent, PathwayRegulates → OrganFunction, etc.

### Haptic / Tactile (114)
This feels very distinct: pressure maps, vibration spectra, shear, texture descriptors, contact forces/torques. It links beautifully to 109 (3D surfaces → local geometry → predicted contact patches) and robotics (force control, grasping). Not easily recovered from vision or audio alone.

**Verdict:** Strong case for keeping **114** — it's a core "somatosensory" primitive missing from most current multimodal setups.

### CAD / Parametric Models (131) vs. 3D (109)
This is the clearest separation in engineering space.

- **109 3D** → meshes, point clouds, voxels, implicit surfaces, appearance (RGB/normal/texture) — "dumb" geometry, great for rendering/physics sim but loses design intent.
- **131 CAD/Parametric** → B-rep solids, feature tree, parameters/constraints, dimensions/tolerances/GD&T, assemblies, mate relations — fully editable, history-based or constraint-driven, manufacturing-ready.

Parametric models carry rich symbolic/engineering semantics that meshes do not (you can't easily change a hole diameter in a mesh without remeshing everything

- **Manufacturing Process** → Process planning/G-code/toolpaths as sequences → could live as "ExecutionTrace" on top of 131 CAD + 134 Robotics.


**Verdict:** Keep **131** separate. It's the "blueprint/intent" layer sitting beside/augmenting 109. Relationships like ParametricGeneratesMesh, ConstraintLimits3DDeformation, ToleranceAffectsAssemblyFit.

**Do we need a separate "blueprint" modality?** Yes — exactly this one.

### Robotics & Kinematics (134) + Control Systems & Signals (135)
Kinematics (forward/inverse, Jacobians, chains, workspace, singularities) is structural — URDF-style tree, joint limits/types, DH params.

Dynamics adds masses, inertias, Coriolis, gravity → equations of motion.

Control adds transfer functions, PID/state-space, stability margins, response shaping, trajectories over time.

They are deeply intertwined: kinematics feeds dynamics, dynamics feeds control synthesis, control outputs reference trajectories that close the loop on kinematics.

But in representation terms:

- Kinematics is mostly **geometric/topological** (graphs of joints/links/transforms).
- Dynamics is **physical/inertial**.
- Control is **signal-processing + feedback** (frequency domain, time-domain responses, observer/controller matrices).

Many robotics stacks treat them in a layered way (kinematics → dynamics → control), but multimodal AI benefits from separating the **structural description** (134) from the **behavioral/closed-loop** layer (135). Human motion capture (mocap) trajectories often feed into kinematics-like representations (joint angles over time) but lack URDF-style robot topology unless retargeted.

**Human joints/movements extension?**
You can capture human motion (via mocap, video pose estimation) as time-series of joint angles/positions → but to drive a robot, you retarget to URDF kinematics + dynamics constraints. This suggests **134** can subsume human kinematic chains (as a special case: "bio-kinematic" vs. "mecha-kinematic"), while **135** handles the low-level servo/control signals that differ hugely between human muscles and robot actuators.

**Verdict:**
- Keep **134 Robotics & Kinematics** (include URDF-like trees, kinematic chains, human retargetable skeletons as subclasses).
- Keep **135 Control Systems & Signals** (controllers, stability, Bode/Nyquist, trajectories as output signals) — it links to sensor time-series (130 if you keep it) but focuses on engineered feedback loops.

They form a natural chain: 134 → 135 → actuator signals → physics sim back to 109/114.

1. **116: Thermal / Infrared Sensing**
   Input: Thermal images, heat maps, IR video, temperature distributions.
   Outputs: Heat signatures, material emissivity, thermal gradients, anomaly detection (hotspots, leaks).
   Relationships: EmitsHeat, ThermallyCoupledTo, IndicatesMaterialState, AffectsBiologicalProcess (links to Biology 111/Human motion in 134), CorrelatesWithHaptic (surface temp → perceived texture in 114).
   Why new: Distinct physics (radiative heat transfer) vs. visible 3D (109) or haptic (114). Enables zero-shot material/property inference (e.g., "hot metal vs. hot plastic"). Strong in embodied/robotics scenes. Not recoverable from RGB + basic sensors.

2. **117: Depth / 3D Point Cloud & Range Sensing** (if not fully subsumed in 109 3D)
   Input: LiDAR, depth cameras (RGB-D), stereo disparity, structured light scans.
   Outputs: Point clouds, depth maps, volumetric occupancy, surface normals, occlusion graphs.
   Relationships: Occludes, DistanceTo, SurfaceGeometryOf, FillsVolume (links tightly to 109 meshes, 131 CAD for validation, 134 kinematics for collision).
   Why new (or refine 109): Many 3D pipelines treat meshes/voxels separately from raw sparse point clouds (different noise, density, registration challenges). If your 109 already covers full volumetric/mesh, fold as metadata; otherwise, keep for raw sensor fusion.

3. **118: IMU / Motion & Inertial Sensing**
   Input: Accelerometer, gyroscope, magnetometer data (6–9 DoF streams), inertial navigation logs.
   Outputs: Orientation, linear/angular velocity/acceleration, trajectory segments, vibration signatures, drift-corrected poses.
   Relationships: AcceleratesAlong, RotatesAbout, ExperiencesForce (links to 114 Haptic forces, 134 Robotics kinematics/dynamics, human motion retargeting, 109 3D pose estimation).
   Why new: Pure inertial dynamics (no visual anchor) — critical for dead-reckoning, sensor fusion in robotics/AR, and distinguishing motion primitives (e.g., "walking vs. falling"). Not just time-series; has specific graph structures (integral chains, covariance).

4. **119: Geospatial / Geographic & Location-Based Data**
   Input: GPS/GNSS traces, satellite imagery, vector maps (GeoJSON, Shapefiles), elevation models (DEM), raster layers.
   Outputs: Coordinates, polygons, trajectories, land-use classifications, spatial relations (containment, proximity, topology).
   Relationships: LocatedAt, Borders, Overlaps, ElevationAffects (links to 109 3D terrain, 111 Biology ecosystems, 134 Robotics navigation, climate if ever added).
   Why new: Explicit spatial topology and coordinate reference systems (CRS) enable unique queries (shortest path, visibility, flood modeling) beyond generic 3D.

5. **120: Electromagnetic / RF & Wireless Signal Data**
   Input: Radio spectra, WiFi/Bluetooth/LTE/5G packet metadata, signal strength maps, antenna patterns.
   Outputs: Frequency bands, modulation types, interference graphs, propagation models, device fingerprints.
   Relationships: TransmitsTo, InterferesWith, PropagatesThrough (links to 134 Robotics comms, 153 Network if kept, sensor fusion in IoT-like scenarios).
   Why new: Wave propagation physics and spectrum analysis add distinct edges (e.g., "signal attenuation by material" linking to 109/131). Not just network traffic.

6. **121: EEG / Neural Signal Extension (if expanding 108)** or **Brain-Computer Interface (BCI) Streams**
   (If 108 is narrow events/bands, expand here.) Input: Multi-channel EEG/ECoG/MEG, spike trains, fNIRS.
   Outputs: Brain state (attention, workload), motor imagery, evoked potentials, connectivity graphs.
   Relationships: CorrelatesWithIntent (links to 110 Voice intent, 134 Human kinematics, 111 Biology neural pathways, ConsciousnessStore).
   Why consider: If current 108 lacks rich BCI/action decoding, this adds motor/cognitive primitives.


- **153: Network / Graph Traffic & Topology**
  Input: Packet captures, flow logs, topology data.




- Reconstruct intelligible or characteristic "speech" (or vocalization)
 - Handle **human speech** (including dysarthric, accented, emotional, or degraded cases)
 - Extend to **animal speech/vocalizations per species** (bioacoustics: bird songs, mammal calls, frog choruses, whale songs, etc.)
 - Potentially synthesize or regenerate vocalizations from the waveform features (e.g., species-specific timbre, rhythm, formants)

 This goes beyond typical Audio 103 (which likely focuses on general acoustic features like spectrograms, MFCCs, music/source separation, environmental sounds) and beyond basic Speech 110 (linguistic layers like dialogue acts).

 ### Why This Justifies a Dedicated or Expanded Modality (Aligned with Our Rules)
 - **New representational primitives**: Raw waveform → learnable frontends (e.g., time-domain filterbanks, WaveNet-style autoregressive modeling, or self-supervised encoders like wav2vec from raw samples). This captures glottal source + vocal tract filtering directly, species-specific anatomy (syrinx in birds, larynx variations), and non-linear production mechanisms that spectrogram-based pipelines often lose or approximate poorly.
 - **Unique inference patterns**: Direct waveform-to-vocalization reconstruction, species identification from waveform patterns, emotional/paralinguistic reconstruction without relying on transcript first, cross-species transfer (speech models fine-tuned on animal vocalizations).
 - **Not easily recoverable via cross-links**: Audio 103 might extract general features, but full waveform-based reconstruction + per-species modeling requires specialized graph nodes (e.g., "VocalizationUnit", "SourceFilterModel", "SpeciesSpecificTimbre"). Zero-shot AMT can help with intent/emotion, but the core waveform inversion/reconstruction is a distinct pipeline.
 - **Strong relationships to existing modalities**:
   - Links to **Audio 103** via "RawWaveformDerivedFrom"
   - Links to **Voice 110** (if kept linguistic) via "LinguisticLayerOnReconstructedVocalization"
   - Links to **Biology 111** and **DNA/Proteomics 112** via species anatomy (vocal apparatus evolution)
   - Links to **3D 109** for vocal tract modeling (e.g., 3D simulation of syrinx/larynx)
   - Links to **Haptic 114** or Robotics 134 for embodied sound production (e.g., robot vocalization synthesis)
   - Cross to Consciousness for emotional valence in vocalizations


 **Make the "Waveform Reconstruction Modality"** (covering human + animal "speech" from soundwaves).
 Keep it **separate from raw Audio 103** (general acoustics) and from pure linguistic Voice (if you still want that as a thin layer on top).

 **Core Focus**:
 - Input: Raw audio bytes/waveforms (multi-channel if needed), optional metadata (species hint, recording conditions).
 - Outputs: Reconstructed vocalization parameters (glottal pulses, formants, prosody envelope), species-specific features, synthesized waveform (or segments), emotional/functional interpretation.
 - Graph nodes: VocalizationUnit, SourceFilterModel, SpeciesVocalSignature, ReconstructedSegment.
 - Relationships: ReconstructsTo (human/animal speech), SpeciesSpecific (e.g., "BirdSyrinxPattern"), ExpressesState (emotion, intent, alarm), EvolvesFrom (biological anatomy link to 111/112).


  - Soundwaves aren't **only** from vocal tracts (human speech or animal calls).
  - **Any** physical interaction between objects (two things clashing, scraping, vibrating, breaking, etc.) also generates soundwaves.
  - You don't want to over-specialize into 3 separate modalities (raw Sound + Speech + Vocalization).
  - Question: Can **vocalization** (human speech + animal calls + object-impact “sounds”) be **constructed directly inside a single Sound modality**?



  This single pipeline:
  - Takes **raw soundwaves** as primary input.
  - Reconstructs / analyzes **any** sound-producing event (vocal tract, object collision, friction, resonance, breakage, etc.).
  - Uses zero-shot AMT + object-type/species hints to specialize reconstruction (human speech, animal calls per species, “metal-clang”, “glass-shatter”, “wood-crack”, etc.).
  - Builds the rich ZSEI graph with the relationships you care about.

  Vocalization is **not** a separate pipeline — it is a **specialized reconstruction path inside 110**, triggered by context (species hint, object collision metadata, or zero-shot inference). Everything is constructible from the raw waveform + cross-links. No bloat, no duplication.

  This perfectly matches your vision: “reconstruct what would be human speech or animal speech etc per specie” **and** “two objects or etc clashing makes soundwaves”.

  - **Input**: Raw audio bytes/waveform (multi-channel OK), optional metadata (species, object types involved, collision hint).
  - **Outputs**: Reconstructed waveform parameters, source-filter model, event segments, species/object signature, synthesized audio (optional).
  - **Graph nodes**: SoundEvent, WaveformSource, ReconstructedSegment, VocalizationUnit (for speech/calls), CollisionImpact (for objects).
  - **Key relationships**:
    - `ReconstructsTo` (human speech / animal call / object clash sound)
    - `ProducedBy` (vocal apparatus / colliding objects / vibrating surface)
    - `SpeciesSpecific` or `ObjectMaterialSpecific`
    - `ExpressesState` (emotion, intent, alarm, mechanical failure)
    - `DerivedFromRawWaveform` → Audio 103
    - Links to **Biology 111** (vocal anatomy), **3D 109** (object geometry causing collision sound), **Haptic 114** (vibration felt), **Robotics 134** (synthesized robot sounds).



  ### Summary of What Is Missing Across All Modalities

  **Core Problem (Common to Almost All)**
  Most pipelines are excellent at **structural extraction** (nodes + basic edges) but extremely weak at **relationship inference** — the exact thing that makes ZSEI powerful. They rely too heavily on LLM zero-shot for surface features (keywords, entities, topics) and almost never use grammar, action verbs, dependency parsing, temporal/causal logic, or hierarchical part-whole reasoning to build rich contextual/semantic edges.

  **Specific Gaps per Modality (Full, No Omissions)**

  **Text (100) — Biggest Gap**
  - **Does well**: Chunking, cleaning, basic keyword/entity/topic extraction via LLM.
  - **Missing**:
    - Grammar-based relationship extraction (subject-verb-object, action vs linking verbs, tense, voice, modality, coreference).
    - Discourse structure (cause-effect, elaboration, contradiction, temporal sequencing).
    - Semantic role labeling (who does what to whom, under what conditions).
    - Cross-sentence relationships (anaphora, bridging, entailment).
    - No explicit support for the verb lists you gave (action verbs, linking verbs, helping verbs).
  - **What we need**: LLM prompt that forces grammar parsing + dependency tree → ZSEI edges like `Performs`, `Affects`, `Implies`, `Contradicts`, `Elaborates`, `TemporalPrecedes`.

  **Code (101) — Strong but Incomplete**
  - **Does well**: AST, functions, classes, imports, calls, complexity.
  - **Missing**:
    - Semantic design-pattern relationships (`ImplementsStrategy`, `UsesObserver`, `ExtendsDecorator`).
    - Data-flow and control-flow relationships (beyond simple calls).
    - Intent-level relationships (`ThisFunctionSolvesX`, `ThisClassEncapsulatesY`).
    - Cross-file semantic dependencies (not just imports).

  **Image (102) — Good Structure, Weak Semantics**
  - **Does well**: Objects, regions, text, faces, colors, composition.
  - **Missing**:
    - Spatial relationships (`Above`, `LeftOf`, `Occludes`, `Supports`, `ContainedIn`).
    - Functional/affordance relationships (`CanBeUsedFor`, `PartOfObject`).
    - Scene-graph level hierarchy (object → part → subpart).
    - Narrative/compositional intent (`FocalPoint`, `LeadingLine`, `RuleOfThirdsViolation`).

  **Audio (103) — Temporal Gap**
  - **Does well**: Segments, speakers, transcription, music features.
  - **Missing**:
    - Temporal event chains (`Precedes`, `Causes`, `OverlapsWith`).
    - Prosody/emotion relationships (`ToneIndicates`, `PitchCorrelatesWith`).
    - Cross-speaker interaction relationships.

  **Video (104) — Strong Timeline, Weak Interaction**
  - **Does well**: Scenes, shots, keyframes, object tracks.
  - **Missing**:
    - Object-object interaction relationships over time (`InteractsWith`, `CollidesWith`, `Follows`).
    - Narrative/causal scene relationships.
    - Cross-modal synchronization edges (audio ↔ visual).

  **Math (105) — Excellent for Proofs**
  - **Does well**: Proof steps, variables, scopes, dependencies.
  - **Missing**:
    - Logical relationships (`Implies`, `Contradicts`, `EquivalentTo`).
    - Theorem networks (this lemma supports that theorem).
    - Assumption-discharge graphs.

  **Chemistry (106) — Strong Molecules**
  - **Does well**: Atoms, bonds, reactions.
  - **Missing**:
    - Full reaction mechanism graphs (`Step1 → Step2 → Intermediate`).
    - Synthesis pathway networks.
    - Similarity + reactivity relationships.

  **DNA (107) — Strong Sequences**
  - **Does well**: Genes, features, variants.
  - **Missing**:
    - Regulatory networks (`Regulates`, `Inhibits`, `Activates`).
    - Interaction graphs (protein-DNA, protein-protein).
    - Evolutionary relationships.

  **EEG (108) — Strong Events**
  - **Does well**: Events, bands, connectivity, sleep stages.
  - **Missing**:
    - Dynamic brain-state transitions.
    - Cognitive process graphs (attention → decision → action).

  **Completely Missing Modalities**
  - **3D (the big one you asked for)** — This is the only major gap. It should unify geometry, rigging, animation, simulation, physics, constraints, materials, lighting, etc.
  - Physics/Simulation (can be absorbed into 3D + Code + Math).
  - UI/Interaction (lower priority for now).

  ### What We Need for the 3D Modality (Full, No Omissions)

  Based on your Blender hierarchical breakdown, the 3D modality must support **everything** that can be offloaded from a 3D scene. It must create a rich graph with:

  **Core Levels (Hierarchical, Like Your Blender Example)**

  1. **Atomic Primitives & Topology**
     - Vertices, edges, faces, loops, custom attributes, UVs, vertex colors, normals, tangents, ID properties.

  2. **Curves, Splines, Surfaces, Volumes, Text, Grease Pencil**
     - Bezier/NURBS, metaballs, VDB grids, text objects, strokes/layers.

  3. **Objects & Hierarchy**
     - All object types (Mesh, Curve, Surface, Meta, Volume, Empty, Light, Camera, Speaker, Lattice, etc.).
     - Parent/child, collections, library overrides.

  4. **Rigging & Armatures**
     - Bones, constraints, shape keys, vertex groups, lattices.

  5. **Animation, Drivers, Keyframes**
     - Actions, F-curves, drivers, constraints.

  6. **Simulation & Physics (All Types)**
     - Rigid body, soft body, cloth, fluid, smoke, dynamic paint, force fields, particles, hair.

  7. **Materials, Shaders, Textures**
     - Full node trees, UDIMs, procedural textures.

  8. **Lights, World, Environment, Freestyle**
     - HDRI, mist, freestyle lines.

  9. **Compositing, VSE, Render Settings, Passes**
     - Full compositor + VSE timeline.

  10. **Metadata & Custom Data**
      - ID properties on every datablock.

  **Graph Requirements for 3D**
  - **Nodes**: Mesh, Vertex, Edge, Face, Bone, Constraint, Keyframe, SimulationCache, Material, Light, Camera, etc.
  - **Edges** (critical for ZSEI):
    - Structural: `Contains`, `ParentOf`, `ChildOf`.
    - Dependency: `Constrains`, `Animates`, `Drives`, `Affects`.
    - Physics: `CollidesWith`, `InfluencedByForce`, `Simulates`.
    - Semantic (ZSEI hooks): `SimilarTo`, `PartOf`, `SymmetricalTo`, `FunctionalRole`.
    - Cross-modality: `DescribedByText`, `ImplementedInCode`, `VisualizedAsImage`.

  This 3D modality will be the richest because it naturally produces dense spatial/temporal/functional graphs.

**Summary of What Is Missing Across All Modalities**

**Core Problem (Common to Almost All)**
Most pipelines are excellent at **structural extraction** (nodes + basic edges) but extremely weak at **relationship inference** — the exact thing that makes ZSEI powerful. They rely too heavily on LLM zero-shot for surface features (keywords, entities, topics) and almost never use grammar, action verbs, dependency parsing, temporal/causal logic, hierarchical part-whole reasoning, or domain-specific inference patterns to build rich contextual/semantic edges.

Every modality (existing, newly defined, and the new 3D) must now include:
- Explicit grammar / action-verb / SRL / discourse extraction where applicable.
- Typed edges with weights, confidence, provenance (`Performs`, `Affects`, `Implies`, `Contradicts`, `TemporalPrecedes`, `PartOf`, `FunctionalRole`, `Constrains`, `InfluencedBy`, `ReconstructsTo`, etc.).
- Provisional nodes for uncertain elements.
- Full ZSEI hooks (`OnGraphCreated`, cross-modality linking, integrity checks).
- Materialized paths and reference-based storage for zero-shot traversal.

**Specific Gaps per Modality (Full, No Omissions – Existing + Newly Defined)**

**Text (100)**
- Does well: Chunking, cleaning, basic keyword/entity/topic extraction via LLM.
- Missing: Grammar-based relationship extraction (subject-verb-object, action vs. linking vs. helping verbs, tense, voice, modality, coreference); discourse structure (cause-effect, elaboration, contradiction, temporal sequencing); semantic role labeling; cross-sentence relationships (anaphora, bridging, entailment).
- Required fix: LLM prompt that forces grammar parsing + dependency tree → ZSEI edges like `Performs`, `Affects`, `Implies`, `Contradicts`, `Elaborates`, `TemporalPrecedes`, `PartOf`, `FunctionalRole`.

**Code (101)**
- Does well: AST, functions, classes, imports, calls, complexity.
- Missing: Semantic design-pattern relationships (`ImplementsStrategy`, `UsesObserver`, `ExtendsDecorator`); data-flow and control-flow relationships; intent-level relationships (`ThisFunctionSolvesX`, `ThisClassEncapsulatesY`); cross-file semantic dependencies.

**Image (102)**
- Does well: Objects, regions, text, faces, colors, composition.
- Missing: Spatial relationships (`Above`, `LeftOf`, `Occludes`, `Supports`, `ContainedIn`); functional/affordance relationships (`CanBeUsedFor`, `PartOfObject`); scene-graph hierarchy (object → part → subpart); narrative/compositional intent (`FocalPoint`, `LeadingLine`, `RuleOfThirdsViolation`).

**Audio (103 – Raw Acoustics)**
- Does well: Segments, speakers, transcription, music features, spectrograms, MFCCs, source separation, environmental sounds.
- Missing: Temporal event chains (`Precedes`, `Causes`, `OverlapsWith`); prosody/emotion relationships (`ToneIndicates`, `PitchCorrelatesWith`); cross-speaker interaction relationships.
- Links to 110 via `RawWaveformDerivedFrom`.

**Video (104)**
- Does well: Scenes, shots, keyframes, object tracks.
- Missing: Object-object interaction relationships over time (`InteractsWith`, `CollidesWith`, `Follows`); narrative/causal scene relationships; cross-modal synchronization edges (audio ↔ visual).

**Math (105)**
- Does well: Proof steps, variables, scopes, dependencies.
- Missing: Logical relationships (`Implies`, `Contradicts`, `EquivalentTo`); theorem networks; assumption-discharge graphs.

**Chemistry (106)**
- Does well: Atoms, bonds, reactions.
- Missing: Full reaction mechanism graphs (`Step1 → Step2 → Intermediate`); synthesis pathway networks; similarity + reactivity relationships.

**DNA / Genomics (107)**
- Does well: Genes, features, variants.
- Missing: Regulatory networks (`Regulates`, `Inhibits`, `Activates`); interaction graphs (protein-DNA, protein-protein); evolutionary relationships.
- Links to 111/112 via `EvolvesInto` / `CodesFor`.

**EEG / Neural Signals (108)**
- Does well: Events, bands, connectivity, sleep stages.
- Missing: Dynamic brain-state transitions; cognitive process graphs (attention → decision → action).
- Links to 121 (BCI extension) and ConsciousnessStore.

**3D (109 – the big one you asked for first)**
- Does well: (none yet – to be created).
- Must support every Blender hierarchical level with no omissions:
  1. Atomic Primitives & Topology (vertices, edges, faces, loops, custom attributes, UVs, vertex colors, normals, tangents, ID properties).
  2. Curves, Splines, Surfaces, Volumes, Text, Grease Pencil.
  3. Objects & Hierarchy (all types, parent/child, collections, library overrides).
  4. Rigging & Armatures (bones, constraints, shape keys, vertex groups, lattices).
  5. Animation, Drivers, Keyframes (actions, F-curves, drivers, constraints).
  6. Simulation & Physics (rigid/soft/cloth/fluid/smoke/dynamic paint/force fields/particles/hair).
  7. Materials, Shaders, Textures (full node trees, UDIMs, procedural).
  8. Lights, World, Environment, Freestyle.
  9. Compositing, VSE, Render Settings, Passes.
  10. Metadata & Custom Data (ID properties on every datablock).
- Graph requirements: Nodes for every datablock; edges (`Contains`, `ParentOf`, `ChildOf`, `Constrains`, `Animates`, `Drives`, `Affects`, `CollidesWith`, `InfluencedByForce`, `Simulates`, `SimilarTo`, `PartOf`, `SymmetricalTo`, `FunctionalRole`); cross-modality (`DescribedByText`, `ImplementedInCode`, `VisualizedAsImage`, `ParametricGeneratesMesh` to 131, etc.).
- This will be the richest graph producer for spatial/temporal/functional relationships.

**Sound & Vocalization Reconstruction (110)**
- Does well: (none yet).
- Must reconstruct intelligible/characteristic vocalizations (human speech including dysarthric/accented/emotional/degraded; animal vocalizations per species – bird songs, mammal calls, frog choruses, whale songs, etc.) **and** any physical sound-producing event (object collision, scraping, vibration, breakage, resonance).
- New primitives: waveform → learnable frontends (time-domain filterbanks, source-filter model, glottal pulses, formants, prosody envelope); species/object-specific timbre and production mechanisms.
- Inference patterns: zero-shot AMT + context (species hint / object types / collision metadata) to specialize reconstruction.
- Graph nodes: `SoundEvent`, `WaveformSource`, `ReconstructedSegment`, `VocalizationUnit`, `CollisionImpact`.
- Key relationships: `ReconstructsTo` (human speech / animal call / object clash), `ProducedBy` (vocal apparatus / colliding objects), `SpeciesSpecific` / `ObjectMaterialSpecific`, `ExpressesState` (emotion/intent/alarm/mechanical failure), `DerivedFromRawWaveform` (to 103).
- Cross-links: Biology 111/112 (vocal anatomy), 3D 109 (object geometry / vocal tract simulation), Haptic 114 (vibration), Robotics 134 (synthesized sounds), Consciousness (emotional valence).
- Separate from raw Audio 103; tightly linked via `RawWaveformDerivedFrom`.

**Multi-Scale Biology (111)**
- Does well: (none yet – narrow scope to upward integration).
- Covers cells ↔ tissues ↔ organs ↔ systems ↔ pathophysiology; explicitly excludes pure sequence/folding (lives in 107/112).
- Required: regulatory networks, metabolic/signaling networks, cellular processes, immunology, tissue-level organization, emergent pathophysiology.
- Key relationships: `ProteinFoldsIntoCellularComponent`, `PathwayRegulatesOrganFunction`, `AffectsBiologicalProcess`.
- Cross-links to 109 (3D cellular structures), 112 (proteins), 110 (bioacoustics), 134 (bio-kinematics).

**Proteomics & Protein Structure (112)**
- Does well: (none yet).
- Sequence → 3D fold → PTMs → interactions (PPI networks, docking).
- Required relationships: `FoldsInto`, `BindsTo`, `Catalyzes`, `InteractsWith`.
- Strong bridges to 109 (3D atomic/coarse-grained), 106 (Chemistry: ligands, reactions, QM), 111 (cellular component).

**Haptic / Tactile Sensing (113)**
- Does well: (none yet).
- Pressure maps, vibration spectra, shear, texture descriptors, contact forces/torques.
- Required relationships: `EmitsTexture`, `FeelsForce`, `CorrelatesWithSurfaceGeometry` (to 109), `CorrelatesWithThermal` (to 116).
- Cross-links to 109, 134 (grasping/force control), 114–135 chain.

**Thermal / Infrared Sensing (114)**
- Does well: (none yet).
- Thermal images, heat maps, IR video, temperature distributions, emissivity, gradients, hotspots.
- Required relationships: `EmitsHeat`, `ThermallyCoupledTo`, `IndicatesMaterialState`, `AffectsBiologicalProcess`, `CorrelatesWithHaptic`.

**Depth / 3D Point Cloud & Range Sensing (115)**
- Does well: (none yet – keep if raw sparse data not fully subsumed in 109).
- LiDAR, depth cameras, stereo, structured light → point clouds, depth maps, volumetric occupancy, normals, occlusion graphs.
- Required relationships: `Occludes`, `DistanceTo`, `SurfaceGeometryOf`, `FillsVolume`.

**IMU / Motion & Inertial Sensing (116)**
- Does well: (none yet).
- Accelerometer/gyro/magnetometer (6–9 DoF), inertial navigation.
- Required relationships: `AcceleratesAlong`, `RotatesAbout`, `ExperiencesForce`.

**Geospatial / Geographic & Location-Based Data (117)**
- Does well: (none yet).
- GPS traces, satellite imagery, vector maps, elevation models.
- Required relationships: `LocatedAt`, `Borders`, `Overlaps`, `ElevationAffects`.

**Electromagnetic / RF & Wireless Signal Data (118)**
- Does well: (none yet).
- Radio spectra, signal strength, antenna patterns, propagation models.
- Required relationships: `TransmitsTo`, `InterferesWith`, `PropagatesThrough`.

**BCI / Advanced Neural Signal Extension (119)**
- Does well: (none yet – expands 108).
- Multi-channel EEG/ECoG/MEG, spike trains, fNIRS, motor imagery, connectivity.
- Required relationships: `CorrelatesWithIntent`, `DrivesMotorAction`.

**Parametric CAD / Blueprint Modeling (120)**
- Does well: (none yet – this is the “blueprint/intent” layer).
- B-rep solids, feature tree, parameters/constraints, dimensions/tolerances/GD&T, assemblies, mate relations, history-based or constraint-driven models.
- Required relationships: `ParametricGeneratesMesh` (to 109), `ConstraintLimitsDeformation`, `ToleranceAffectsAssemblyFit`.
- This is the dedicated blueprint modality.

**Kinematics & Robotics (Bio & Mecha) (121)**
- Does well: (none yet).
- URDF-style trees, joint limits/types, DH params, kinematic chains, workspace, singularities; includes human/bio retargetable skeletons and all biological motion (not just human).
- Required relationships: `KinematicallyLinked`, `JointConstrains`, `BioKinematicToMecha`.

**Control Systems & Signals (122)**
- Does well: (none yet).
- Transfer functions, PID/state-space, stability margins, response shaping, trajectories, feedback loops.
- Required relationships: `Controls`, `Stabilizes`, `OutputsTrajectory` (links to 134 kinematics, 118 IMU, 109 physics).

**Network / Graph Traffic & Topology (123)**
- Does well: (none yet).
- Packet captures, flow logs, topology data.
- Required relationships: `TransmitsTo`, `InterferesWith`, `TopologicallyConnected`.

**Completely Missing Modalities (Now Fully Defined and Numbered Above)**
All previously identified gaps are now closed with the numbered list above. The only major addition was 3D (109), which is the first to be implemented. Physics/Simulation is absorbed into 109 + 106/121/122.

**Alignment & Next Steps**
- All modalities now follow the **Core Creation Rule** (new primitives, new inference patterns, or rich new cross-links).
- Numbers are organized logically.
- Every modality must produce the same rich ZSEI graph format and trigger the same hooks.
- Relationship inference (grammar/action-verbs/SRL/discourse for text/voice; domain-specific for others) is now mandatory across the board.
- Cross-modality links are explicitly defined so the fabric remains omnidirectional and zero-shot capable.
